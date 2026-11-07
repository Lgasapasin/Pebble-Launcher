//! Per-instance Minecraft mod-loader profiles.
//! Fabric & Quilt use their launcher-profile APIs. Forge/NeoForge use their official installer
//! and are experimental until platform-specific launch testing is completed.
use crate::{instances, java, net, storage};
use serde::Serialize;
use serde_json::Value;
use std::{path::{Path, PathBuf}, collections::HashSet};
use tauri::AppHandle;
use tokio::process::Command;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoaderVersion { pub version: String, pub stable: bool }

fn kind_name(kind: &str) -> Result<&'static str, String> {
    match kind.to_lowercase().as_str() {
        "vanilla" => Ok("vanilla"), "fabric" => Ok("fabric"), "quilt" => Ok("quilt"),
        "forge" => Ok("forge"), "neoforge" => Ok("neoforge"),
        _ => Err("Unsupported Minecraft mod loader".into()),
    }
}

fn meta_base(kind: &str) -> Result<&'static str, String> {
    match kind {
        "fabric" => Ok("https://meta.fabricmc.net/v2"),
        "quilt" => Ok("https://meta.quiltmc.org/v3"),
        _ => Err("This loader uses the installer workflow".into()),
    }
}

#[tauri::command]
pub async fn list_loader_versions(version_id: String, loader: String) -> Result<Vec<LoaderVersion>, String> {
    storage::safe_id(&version_id)?;
    let kind = kind_name(&loader)?;
    if kind == "vanilla" { return Ok(Vec::new()); }
    if matches!(kind, "forge" | "neoforge") {
        // Do not invent a Forge/NeoForge build number. Use the exact build from its official download page.
        return Ok(Vec::new());
    }
    let url = format!("{}/versions/loader/{}", meta_base(kind)?, urlencoding::encode(&version_id));
    let values: Vec<Value> = net::get_json(&net::client()?, &url).await?;
    let mut versions = Vec::new();
    for value in values {
        let loader = &value["loader"];
        if let Some(version) = loader["version"].as_str() {
            versions.push(LoaderVersion { version: version.into(), stable: loader["stable"].as_bool().unwrap_or(true) });
        }
    }
    Ok(versions)
}

pub fn loader_profile_path(app: &AppHandle, instance_id: &str) -> Result<PathBuf, String> {
    let inst = instances::find_instance(app, instance_id)?;
    Ok(storage::root(app)?.join("instances").join(inst.id).join("loader-profile.json"))
}

#[tauri::command]
pub async fn configure_instance_loader(
    app: AppHandle, instance_id: String, loader: String, loader_version: Option<String>, java_path: Option<String>,
) -> Result<instances::Instance, String> {
    let instance = instances::find_instance(&app, &instance_id)?;
    let kind = kind_name(&loader)?;
    let profile_path = loader_profile_path(&app, &instance_id)?;
    if kind == "vanilla" {
        let _ = tokio::fs::remove_file(&profile_path).await;
        return instances::set_loader(&app, &instance_id, kind, None);
    }
    let version = if let Some(v) = loader_version.filter(|v| !v.trim().is_empty()) {
        if v.len() > 100 || !v.bytes().all(|b| b.is_ascii_alphanumeric() || b".-_+".contains(&b)) {
            return Err("Invalid mod-loader build version".into());
        }
        v
    } else if matches!(kind, "fabric" | "quilt") {
        let all = list_loader_versions(instance.version_id.clone(), kind.to_string()).await?;
        all.iter().find(|v| v.stable).or_else(|| all.first()).ok_or("No loader release supports this Minecraft version")?.version.clone()
    } else {
        return Err("Enter the exact Forge/NeoForge build version from its official releases".into());
    };
    let value = if matches!(kind, "fabric" | "quilt") {
        let url = format!("{}/versions/loader/{}/{}/profile/json", meta_base(kind)?, urlencoding::encode(&instance.version_id), urlencoding::encode(&version));
        net::get_json::<Value>(&net::client()?, &url).await?
    } else {
        official_installer(&app, kind, &instance.version_id, &version, java_path.as_deref().unwrap_or("auto")).await?
    };
    let main = value["mainClass"].as_str().ok_or("Loader profile has no Minecraft client main class")?;
    if main.is_empty() { return Err("Loader main class is empty".into()); }
    // Save only validated installer/profile metadata. Never replace the original Vanilla JSON.
    let output = serde_json::to_vec_pretty(&value).map_err(|e| e.to_string())?;
    tokio::fs::write(&profile_path, output).await.map_err(|e| e.to_string())?;
    instances::set_loader(&app, &instance_id, kind, Some(version))
}

async fn official_installer(app: &AppHandle, kind: &str, mc: &str, build: &str, java_override: &str) -> Result<Value, String> {
    // Installer launch metadata is version-dependent. Treat these two paths as experimental.
    let root = storage::root(app)?;
    let coordinate = if kind == "forge" {
        if !build.starts_with(&format!("{mc}-")) { return Err("Forge version must be MinecraftVersion-ForgeVersion (example: 1.21.11-61.2.5)".into()); }
        format!("net/minecraftforge/forge/{build}/forge-{build}-installer.jar")
    } else {
        format!("net/neoforged/neoforge/{build}/neoforge-{build}-installer.jar")
    };
    let base = if kind == "forge" { "https://maven.minecraftforge.net/" } else { "https://maven.neoforged.net/releases/" };
    let url = format!("{base}{coordinate}");
    let client = net::client()?;
    let installer = root.join("installers").join(format!("{}-{}.jar", kind, build));
    net::ensure_download(&client, &url, &installer, None).await?;
    // Explicitly use Java matching the Mojang base version to run the installer.
    let base_meta: crate::models::VersionMeta = serde_json::from_slice(&tokio::fs::read(storage::version_json(&root, mc)).await.map_err(|_| "Install Vanilla Minecraft before configuring Forge/NeoForge")?).map_err(|e| e.to_string())?;
    let major = base_meta.java_version.map(|v| v.major_version).unwrap_or(21);
    let java = java::resolve(app, java_override, major).await?;
    let out = Command::new(&java).arg("-jar").arg(&installer).arg("--installClient").arg(&root)
        .output().await.map_err(|e| format!("Could not run {kind} installer: {e}"))?;
    if !out.status.success() { return Err(format!("{kind} installer failed: {}", String::from_utf8_lossy(&out.stderr).chars().take(400).collect::<String>())); }
    let versions = root.join("versions");
    for entry in std::fs::read_dir(&versions).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        if !entry.path().is_dir() { continue; }
        let name = entry.file_name().to_string_lossy().to_lowercase();
        if !name.contains(kind) || !name.contains(mc) { continue; }
        let json = entry.path().join(format!("{}.json", entry.file_name().to_string_lossy()));
        if !json.is_file() { continue; }
        if let Ok(data) = std::fs::read(&json) {
            if let Ok(profile) = serde_json::from_slice::<Value>(&data) {
                if profile["inheritsFrom"].as_str() == Some(mc) && profile["mainClass"].as_str().is_some() { return Ok(profile); }
            }
        }
    }
    Err("Installer finished but no supported client profile was generated; this Forge/NeoForge version may require additional integration".into())
}

#[derive(Default)]
pub struct PreparedLoader { pub main_class: Option<String>, pub libraries: Vec<PathBuf>, pub jvm: Vec<Value>, pub game: Vec<Value>, pub profile_id: Option<String> }

pub async fn prepare(app: &AppHandle, instance_id: &str) -> Result<PreparedLoader, String> {
    let inst = instances::find_instance(app, instance_id)?;
    if inst.loader == "vanilla" { return Ok(PreparedLoader::default()); }
    let path = loader_profile_path(app, instance_id)?;
    let bytes = tokio::fs::read(path).await.map_err(|_| "Install the selected mod loader in Instances before launching".to_string())?;
    let profile: Value = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    if let Some(inherited) = profile["inheritsFrom"].as_str() {
        if inherited != inst.version_id { return Err("Loader profile targets a different Minecraft version".into()); }
    }
    let main = profile["mainClass"].as_str().ok_or("Loader main class missing")?.to_string();
    let mut output = PreparedLoader { main_class: Some(main), profile_id: profile["id"].as_str().map(str::to_owned), ..Default::default() };
    let client = net::client()?;
    let root = storage::root(app)?;
    let mut paths = HashSet::new();
    for library in profile["libraries"].as_array().ok_or("Loader profile has no libraries")? {
        let name = library["name"].as_str().ok_or("Malformed loader Maven library name")?;
        let (path, url, checksum) = if let Some(artifact) = library["downloads"]["artifact"].as_object() {
            let rel = artifact.get("path").and_then(Value::as_str).ok_or("Loader library missing path")?;
            let url = artifact.get("url").and_then(Value::as_str).ok_or("Loader library missing URL")?.to_string();
            (storage::checked_relative(rel)?.to_path_buf(), url, artifact.get("sha1").and_then(Value::as_str).map(str::to_owned))
        } else {
            let relative = maven_relative(name)?;
            let base = library["url"].as_str().unwrap_or_else(|| if inst.loader == "quilt" { "https://maven.quiltmc.org/repository/release/" } else if inst.loader == "neoforge" { "https://maven.neoforged.net/releases/" } else if inst.loader == "forge" { "https://maven.minecraftforge.net/" } else { "https://maven.fabricmc.net/" });
            (relative.clone(), format!("{}/{}", base.trim_end_matches('/'), relative.to_string_lossy().replace('\\', "/")), None)
        };
        if !paths.insert(path.clone()) { continue; }
        let dest = root.join("libraries").join(path);
        net::ensure_download(&client, &url, &dest, checksum.as_deref()).await?;
        output.libraries.push(dest);
    }
    if let Some(args) = profile["arguments"]["jvm"].as_array() { output.jvm = args.clone(); }
    if let Some(args) = profile["arguments"]["game"].as_array() { output.game = args.clone(); }
    Ok(output)
}

fn maven_relative(coordinate: &str) -> Result<PathBuf, String> {
    let fields: Vec<&str> = coordinate.split(':').collect();
    if fields.len() < 3 || fields.len() > 4 { return Err(format!("Unsupported Maven coordinate: {coordinate}")); }
    for part in &fields {
        if part.is_empty() || !part.chars().all(|c| c.is_ascii_alphanumeric() || "._-+".contains(c)) || *part == "." || *part == ".." {
            return Err("Invalid Maven coordinate in loader metadata".into());
        }
    }
    let suffix = if fields.len() == 4 { format!("-{}", fields[3]) } else { String::new() };
    let path = format!("{}/{}/{}/{}-{}{}.jar", fields[0].replace('.', "/"), fields[1], fields[2], fields[1], fields[2], suffix);
    Ok(PathBuf::from(path))
}

#[cfg(test)] mod tests {
    use super::*;
    #[test] fn maven_coordinates() {
        assert_eq!(maven_relative("net.fabricmc:fabric-loader:0.17.3").unwrap().to_string_lossy().replace('\\', "/"), "net/fabricmc/fabric-loader/0.17.3/fabric-loader-0.17.3.jar");
        assert!(maven_relative("a:b:../../etc").is_err());
    }
}
