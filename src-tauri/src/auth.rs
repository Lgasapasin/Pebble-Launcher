//! Pebble account vault. All secrets live in the operating-system credential store.
//! accounts.json contains only profile metadata; no token is written to disk by Pebble.
use crate::{models::Account, net, storage};
use serde::{Deserialize, Serialize};
use std::{fs, time::{SystemTime, UNIX_EPOCH}};
use tauri::{AppHandle, State};
use tokio::sync::Mutex;

const AUTH_BASE: &str = "https://login.microsoftonline.com/consumers/oauth2/v2.0";
const KEYRING_SERVICE: &str = "io.pebble.launcher.accounts";

#[derive(Default)]
pub struct AuthState {
    pending: Mutex<Option<PendingAuth>>,
}

struct PendingAuth {
    client_id: String,
    device_code: String,
    interval: u64,
    expires: u64,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AccountRecord {
    id: String,
    name: String,
    uuid: String,
    kind: String,
}

#[derive(Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AccountStore {
    active_id: Option<String>,
    accounts: Vec<AccountRecord>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountSummary {
    id: String,
    name: String,
    uuid: String,
    kind: String,
    active: bool,
}

#[derive(Serialize, Deserialize)]
struct Secrets {
    access_token: String,
    refresh_token: Option<String>,
    client_id: Option<String>,
    expires_at: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DevicePrompt {
    user_code: String,
    verification_uri: String,
    expires_in: u64,
    message: String,
}

fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs()
}

fn store_path(app: &AppHandle) -> Result<std::path::PathBuf, String> {
    Ok(storage::root(app)?.join("accounts.json"))
}

fn load_store(app: &AppHandle) -> Result<AccountStore, String> {
    let path = store_path(app)?;
    if !path.exists() { return Ok(AccountStore::default()); }
    let bytes = fs::read(path).map_err(|e| e.to_string())?;
    serde_json::from_slice(&bytes).map_err(|e| format!("Account metadata is invalid: {e}"))
}

fn write_store(app: &AppHandle, store: &AccountStore) -> Result<(), String> {
    let path = store_path(app)?;
    let temp = path.with_extension("json.tmp");
    fs::write(&temp, serde_json::to_vec_pretty(store).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    // On Windows a rename does not replace the destination.
    if path.exists() { fs::remove_file(&path).map_err(|e| e.to_string())?; }
    fs::rename(temp, path).map_err(|e| e.to_string())
}

fn credential(record_id: &str) -> Result<keyring::Entry, String> {
    if record_id.is_empty() || record_id.len() > 100 || !record_id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_') {
        return Err("Invalid account identifier".into());
    }
    keyring::Entry::new(KEYRING_SERVICE, record_id).map_err(|e| e.to_string())
}

fn write_secrets(id: &str, secrets: &Secrets) -> Result<(), String> {
    credential(id)?.set_password(&serde_json::to_string(secrets).map_err(|e| e.to_string())?)
        .map_err(|_| "Cannot store account securely. Set up your OS credential store (Secret Service/keyring on Linux).".to_string())
}

fn read_secrets(id: &str) -> Result<Secrets, String> {
    let raw = credential(id)?.get_password()
        .map_err(|_| "Account credentials unavailable. Please sign in or import this session again.".to_string())?;
    serde_json::from_str(&raw).map_err(|_| "Account credentials are invalid; please sign in again".to_string())
}

fn delete_secrets(id: &str) -> Result<(), String> {
    match credential(id)?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(_) => Err("Unable to delete saved credentials from the OS keyring".to_string()),
    }
}

fn summaries(store: &AccountStore) -> Vec<AccountSummary> {
    store.accounts.iter().map(|item| AccountSummary {
        id: item.id.clone(), name: item.name.clone(), uuid: item.uuid.clone(),
        kind: item.kind.clone(), active: store.active_id.as_deref() == Some(item.id.as_str()),
    }).collect()
}

fn valid_uuid(value: &str) -> bool {
    value.len() == 32 && value.bytes().all(|c| c.is_ascii_hexdigit())
}

fn record_from_profile(kind: &str, profile: &Account) -> Result<AccountRecord, String> {
    if !valid_uuid(&profile.id) { return Err("Minecraft returned an invalid profile UUID".into()); }
    Ok(AccountRecord {
        id: format!("{kind}-{}", profile.id.to_ascii_lowercase()),
        name: profile.name.clone(), uuid: profile.id.to_ascii_lowercase(), kind: kind.into(),
    })
}

fn upsert(app: &AppHandle, record: AccountRecord) -> Result<Vec<AccountSummary>, String> {
    let mut store = load_store(app)?;
    if let Some(existing) = store.accounts.iter_mut().find(|a| a.id == record.id) { *existing = record.clone(); }
    else { store.accounts.push(record.clone()); }
    store.active_id = Some(record.id);
    write_store(app, &store)?;
    Ok(summaries(&store))
}

#[tauri::command]
pub fn list_accounts(app: AppHandle) -> Result<Vec<AccountSummary>, String> {
    Ok(summaries(&load_store(&app)?))
}

#[tauri::command]
pub fn select_account(app: AppHandle, account_id: String) -> Result<Vec<AccountSummary>, String> {
    let mut store = load_store(&app)?;
    if !store.accounts.iter().any(|a| a.id == account_id) { return Err("Account not found".into()); }
    store.active_id = Some(account_id);
    write_store(&app, &store)?;
    Ok(summaries(&store))
}

#[tauri::command]
pub fn remove_account(app: AppHandle, account_id: String) -> Result<Vec<AccountSummary>, String> {
    let mut store = load_store(&app)?;
    let record = store.accounts.iter().find(|a| a.id == account_id)
        .ok_or("Account not found")?.clone();
    if record.kind != "offline" { delete_secrets(&record.id)?; }
    store.accounts.retain(|a| a.id != account_id);
    if store.active_id.as_deref() == Some(account_id.as_str()) {
        store.active_id = store.accounts.first().map(|a| a.id.clone());
    }
    write_store(&app, &store)?;
    Ok(summaries(&store))
}

#[tauri::command]
pub fn add_offline_account(app: AppHandle, username: String) -> Result<Vec<AccountSummary>, String> {
    let username = username.trim();
    if username.len() < 3 || username.len() > 16 || !username.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_') {
        return Err("Offline usernames must be 3–16 letters, numbers or underscores".into());
    }
    // Same name-based UUID calculation as Minecraft's offline mode.
    let mut digest = md5::compute(format!("OfflinePlayer:{username}").as_bytes()).0;
    digest[6] = (digest[6] & 0x0f) | 0x30;
    digest[8] = (digest[8] & 0x3f) | 0x80;
    let uuid: String = digest.iter().map(|b| format!("{b:02x}")).collect();
    upsert(&app, AccountRecord {
        id: format!("offline-{uuid}"),
        name: username.to_owned(), uuid, kind: "offline".into(),
    })
}

async fn minecraft_profile(client: &reqwest::Client, token: &str) -> Result<Account, String> {
    client.get("https://api.minecraftservices.com/minecraft/profile")
        .bearer_auth(token).send().await
        .map_err(|_| "Unable to contact the Minecraft profile service".to_string())?
        .error_for_status().map_err(|_| "Session token is invalid, expired, or has no Minecraft Java profile".to_string())?
        .json::<Account>().await.map_err(|_| "Invalid Minecraft profile response".to_string())
}

#[tauri::command]
pub async fn import_session(app: AppHandle, token: String) -> Result<Vec<AccountSummary>, String> {
    let token = token.trim();
    if token.len() < 20 || token.len() > 8192 || !token.is_ascii() || token.bytes().any(|b| b.is_ascii_whitespace()) {
        return Err("Enter a valid Minecraft access token for an account you own".into());
    }
    let client = net::client()?;
    let profile = minecraft_profile(&client, token).await?;
    let record = record_from_profile("session", &profile)?;
    // Imported access tokens have no refresh token: re-import when they expire.
    write_secrets(&record.id, &Secrets {
        access_token: token.to_owned(), refresh_token: None,
        client_id: None, expires_at: 0,
    })?;
    upsert(&app, record)
}

#[tauri::command]
pub async fn start_login(client_id: String, state: State<'_, AuthState>) -> Result<DevicePrompt, String> {
    if client_id.len() > 100 || client_id.trim().is_empty() || !client_id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
        return Err("Enter an approved Microsoft public application client ID in Settings".into());
    }
    let client = net::client()?;
    let response = client.post(format!("{AUTH_BASE}/devicecode"))
        .form(&[("client_id", client_id.as_str()), ("scope", "XboxLive.signin offline_access")])
        .send().await.map_err(|e| e.to_string())?
        .error_for_status().map_err(|_| "Microsoft device login request was rejected".to_string())?
        .json::<serde_json::Value>().await.map_err(|e| e.to_string())?;
    let field = |k: &str| response[k].as_str().unwrap_or_default().to_owned();
    let device_code = field("device_code");
    let prompt = DevicePrompt {
        user_code: field("user_code"), verification_uri: field("verification_uri"),
        expires_in: response["expires_in"].as_u64().unwrap_or(900), message: field("message"),
    };
    if device_code.is_empty() || prompt.user_code.is_empty() { return Err("Microsoft did not return a login code".into()); }
    *state.pending.lock().await = Some(PendingAuth {
        client_id, device_code, interval: response["interval"].as_u64().unwrap_or(5).max(2),
        expires: now() + prompt.expires_in,
    });
    Ok(prompt)
}

#[tauri::command]
pub async fn complete_login(app: AppHandle, state: State<'_, AuthState>) -> Result<Vec<AccountSummary>, String> {
    let pending = state.pending.lock().await.take().ok_or("Start Microsoft sign-in first")?;
    let client = net::client()?;
    let mut interval = pending.interval;
    let microsoft_tokens = loop {
        if now() >= pending.expires { return Err("Device login timed out; start again".into()); }
        tokio::time::sleep(std::time::Duration::from_secs(interval)).await;
        let r: serde_json::Value = client.post(format!("{AUTH_BASE}/token"))
            .form(&[
                ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
                ("client_id", &pending.client_id),
                ("device_code", &pending.device_code),
            ])
            .send().await.map_err(|_| "Microsoft login request failed".to_string())?
            .json().await.map_err(|_| "Invalid Microsoft login response".to_string())?;
        if r.get("access_token").is_some() { break r; }
        match r["error"].as_str().unwrap_or("unknown_error") {
            "authorization_pending" => continue,
            "slow_down" => { interval += 5; continue; }
            "authorization_declined" | "access_denied" => return Err("Sign-in was declined".into()),
            "expired_token" => return Err("Sign-in code has expired".into()),
            _ => return Err("Microsoft sign-in failed; check your app registration".into()),
        }
    };
    let access_token = microsoft_tokens["access_token"].as_str().ok_or("Missing Microsoft access token")?;
    let refresh_token = microsoft_tokens["refresh_token"].as_str().ok_or("Missing Microsoft refresh token")?;
    let (profile, minecraft_token, lifetime) = minecraft_tokens(&client, access_token).await?;
    let record = record_from_profile("microsoft", &profile)?;
    write_secrets(&record.id, &Secrets {
        access_token: minecraft_token,
        refresh_token: Some(refresh_token.to_owned()),
        client_id: Some(pending.client_id), expires_at: now() + lifetime.saturating_sub(120),
    })?;
    upsert(&app, record)
}

async fn refresh_microsoft(record: &AccountRecord, secrets: &mut Secrets) -> Result<(), String> {
    if now() < secrets.expires_at { return Ok(()); }
    let client_id = secrets.client_id.as_deref().ok_or("Microsoft app ID missing")?;
    let refresh = secrets.refresh_token.as_deref().ok_or("Microsoft refresh token missing")?;
    let client = net::client()?;
    let response: serde_json::Value = client.post(format!("{AUTH_BASE}/token"))
        .form(&[
            ("grant_type", "refresh_token"), ("client_id", client_id),
            ("refresh_token", refresh), ("scope", "XboxLive.signin offline_access"),
        ])
        .send().await.map_err(|_| "Microsoft refresh request failed".to_string())?
        .error_for_status().map_err(|_| "Microsoft login expired; sign in again".to_string())?
        .json().await.map_err(|_| "Invalid Microsoft refresh response".to_string())?;
    let ms_token = response["access_token"].as_str().ok_or("Microsoft did not return an access token")?;
    let (profile, mc_token, lifetime) = minecraft_tokens(&client, ms_token).await?;
    if profile.id.to_ascii_lowercase() != record.uuid {
        return Err("Refreshed Microsoft account does not match the saved Minecraft profile".into());
    }
    secrets.access_token = mc_token;
    secrets.expires_at = now() + lifetime.saturating_sub(120);
    if let Some(new_refresh) = response["refresh_token"].as_str() {
        secrets.refresh_token = Some(new_refresh.to_owned());
    }
    write_secrets(&record.id, secrets)
}

async fn access_for_record(record: &AccountRecord) -> Result<String, String> {
    if record.kind == "offline" { return Ok("0".to_string()); }
    let mut secrets = read_secrets(&record.id)?;
    if record.kind == "microsoft" {
        refresh_microsoft(record, &mut secrets).await?;
    } else if record.kind == "session" {
        let profile = minecraft_profile(&net::client()?, &secrets.access_token).await?;
        if profile.id.to_ascii_lowercase() != record.uuid {
            return Err("Session identity changed; import again".into());
        }
    } else { return Err("Unsupported account type".into()); }
    Ok(secrets.access_token)
}

#[tauri::command]
pub async fn validate_account(app: AppHandle, account_id: String) -> Result<AccountSummary, String> {
    let store = load_store(&app)?;
    let record = store.accounts.iter().find(|a| a.id == account_id).ok_or("Account not found")?;
    let _ = access_for_record(record).await?;
    Ok(AccountSummary {
        id: record.id.clone(), name: record.name.clone(), uuid: record.uuid.clone(),
        kind: record.kind.clone(), active: store.active_id.as_deref() == Some(record.id.as_str()),
    })
}

pub async fn access_for_launch(app: &AppHandle) -> Result<(Account, String), String> {
    let store = load_store(app)?;
    let id = store.active_id.as_deref().ok_or("Select a Minecraft account in Accounts first")?;
    let record = store.accounts.iter().find(|a| a.id == id).ok_or("Selected account was removed")?;
    let token = access_for_record(record).await?;
    Ok((Account { name: record.name.clone(), id: record.uuid.clone() }, token))
}

async fn minecraft_tokens(client: &reqwest::Client, ms_access: &str) -> Result<(Account, String, u64), String> {
    let xbox: serde_json::Value = client.post("https://user.auth.xboxlive.com/user/authenticate")
        .json(&serde_json::json!({
            "Properties": {"AuthMethod": "RPS", "SiteName": "user.auth.xboxlive.com", "RpsTicket": format!("d={ms_access}")},
            "RelyingParty": "http://auth.xboxlive.com", "TokenType": "JWT"
        })).send().await.map_err(|_| "Xbox sign-in failed".to_string())?
        .error_for_status().map_err(|_| "Xbox authentication was rejected".to_string())?
        .json().await.map_err(|_| "Xbox returned invalid data".to_string())?;
    let xbox_token = xbox["Token"].as_str().ok_or("Xbox auth token missing")?;
    let xsts: serde_json::Value = client.post("https://xsts.auth.xboxlive.com/xsts/authorize")
        .json(&serde_json::json!({
            "Properties": {"SandboxId": "RETAIL", "UserTokens": [xbox_token]},
            "RelyingParty": "rp://api.minecraftservices.com/", "TokenType": "JWT"
        })).send().await.map_err(|_| "Xbox XSTS request failed".to_string())?
        .error_for_status().map_err(|_| "Xbox XSTS authorization was rejected".to_string())?
        .json().await.map_err(|_| "Xbox XSTS response invalid".to_string())?;
    let xsts_token = xsts["Token"].as_str().ok_or("XSTS token missing")?;
    let uhs = xsts["DisplayClaims"]["xui"][0]["uhs"].as_str().ok_or("Xbox user hash missing")?;
    let minecraft: serde_json::Value = client.post("https://api.minecraftservices.com/authentication/login_with_xbox")
        .json(&serde_json::json!({"identityToken": format!("XBL3.0 x={uhs};{xsts_token}")}))
        .send().await.map_err(|_| "Minecraft token request failed".to_string())?
        .error_for_status().map_err(|_| "Minecraft API access denied; Microsoft app approval may be required".to_string())?
        .json().await.map_err(|_| "Minecraft token response invalid".to_string())?;
    let token = minecraft["access_token"].as_str().ok_or("Minecraft token missing")?;
    let lifetime = minecraft["expires_in"].as_u64().unwrap_or(86_400);
    let profile = minecraft_profile(client, token).await?;
    Ok((profile, token.to_owned(), lifetime))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn rejects_bad_ids() { assert!(valid_uuid("..\\..\\bad") == false); }
    #[test] fn summaries_do_not_contain_secrets() {
        let value = serde_json::to_string(&AccountSummary {
            id: "a".into(), name: "b".into(), uuid: "c".into(), kind: "offline".into(), active: false,
        }).unwrap();
        assert!(!value.contains("access_token"));
    }
}
