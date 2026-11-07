//! Modrinth browsing and compatible per-instance mod management.
use crate::{instances, net, storage};
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, path::{Path, PathBuf}};
use tauri::AppHandle;

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModProject {
    #[serde(rename(deserialize = "project_id", serialize = "projectId"))]
    pub project_id: String,
    pub title: String,
    pub description: String,
    #[serde(rename(deserialize = "icon_url", serialize = "iconUrl"))]
    pub icon_url: Option<String>,
}
#[derive(Deserialize)]
struct SearchResult { hits: Vec<ModProject> }
#[derive(Deserialize)]
struct ModVersion { files: Vec<ModArtifact>, dependencies: Vec<ModDependency> }
#[derive(Deserialize)]
struct ModArtifact { url: String, filename: String, primary: Option<bool>, hashes: std::collections::HashMap<String, String> }
#[derive(Deserialize)]
struct ModDependency { project_id: Option<String>, dependency_type: String }
#[derive(Serialize)]
#[serde(rename_all="camelCase")]
pub struct LocalMod { pub name: String, pub enabled: bool }

fn ensure_modded(inst: &instances::Instance) -> Result<(), String> {
    if inst.loader == "vanilla" { Err("Choose and install a mod loader for this instance first".into()) } else { Ok(()) }
}

#[tauri::command]
pub async fn find_mods(app: AppHandle, query: String, instance_id: String) -> Result<Vec<ModProject>, String> {
    let inst = instances::find_instance(&app, &instance_id)?;
    ensure_modded(&inst)?;
    let query = query.trim();
    if query.len() > 90 { return Err("Search phrase is too long".into()); }
    let facets = serde_json::json!([["project_type:mod"], [format!("versions:{}", inst.version_id)], [format!("categories:{}", inst.loader)]]).to_string();
    let mut url = reqwest::Url::parse("https://api.modrinth.com/v2/search").map_err(|e| e.to_string())?;
    url.query_pairs_mut().append_pair("query", query).append_pair("facets", &facets).append_pair("limit", "24");
    let result: SearchResult = net::get_json(&net::client()?, url.as_str()).await?;
    Ok(result.hits)
}

fn game_mods(app: &AppHandle, id: &str) -> Result<PathBuf, String> {
    let inst = instances::find_instance(app, id)?;
    Ok(instances::game_directory(app, id, &inst.version_id)?.join("mods"))
}

fn safe_filename(name: &str) -> Result<&str, String> {
    if name.is_empty() || name.len() > 180 || name.starts_with('.') || name.contains('/') || name.contains('\\') || name.contains("..") ||
        name.chars().any(char::is_control) || !name.to_lowercase().ends_with(".jar") {
        return Err("Invalid mod JAR filename".into());
    }
    Ok(name)
}

#[tauri::command]
pub async fn install_modrinth_mod(app: AppHandle, instance_id: String, project_id: String) -> Result<Vec<LocalMod>, String> {
    let inst = instances::find_instance(&app, &instance_id)?;
    ensure_modded(&inst)?;
    if project_id.is_empty() || project_id.len() > 48 || !project_id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-') {
        return Err("Invalid Modrinth project identifier".into());
    }
    let folder = game_mods(&app, &instance_id)?;
    tokio::fs::create_dir_all(&folder).await.map_err(|e| e.to_string())?;
    let client = net::client()?;
    let mut pending = vec![project_id];
    let mut visited = HashSet::new();
    while let Some(project) = pending.pop() {
        if !visited.insert(project.clone()) { continue; }
        if visited.len() > 40 { return Err("Too many mod dependencies; install cancelled".into()); }
        let url = format!("https://api.modrinth.com/v2/project/{}/version?loaders={}&game_versions={}",
            urlencoding::encode(&project), urlencoding::encode(&serde_json::json!([inst.loader]).to_string()),
            urlencoding::encode(&serde_json::json!([inst.version_id]).to_string()));
        let versions: Vec<ModVersion> = net::get_json(&client, &url).await?;
        let version = versions.first().ok_or_else(|| format!("No compatible Modrinth version for {project}"))?;
        let file = version.files.iter().find(|f| f.primary.unwrap_or(false)).or_else(|| version.files.first())
            .ok_or("Modrinth mod has no downloadable file")?;
        let file_name = safe_filename(&file.filename)?;
        let dest = folder.join(file_name);
        let expected = file.hashes.get("sha1").ok_or("Modrinth version does not provide a SHA-1 checksum")?;
        net::ensure_download(&client, &file.url, &dest, Some(expected)).await?;
        for dependency in &version.dependencies {
            if dependency.dependency_type == "required" {
                if let Some(id) = &dependency.project_id {
                    if id.len() <= 48 && id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-') { pending.push(id.clone()); }
                }
            }
        }
    }
    list_local_mods(app, instance_id)
}

#[tauri::command]
pub fn list_local_mods(app: AppHandle, instance_id: String) -> Result<Vec<LocalMod>, String> {
    let dir = game_mods(&app, &instance_id)?;
    if !dir.exists() { return Ok(Vec::new()); }
    let mut mods = Vec::new();
    for entry in std::fs::read_dir(dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        if !entry.path().is_file() { continue; }
        let file_name = entry.file_name().to_string_lossy().into_owned();
        let (name, enabled) = if file_name.to_lowercase().ends_with(".jar.disabled") {
            (&file_name[..file_name.len() - ".disabled".len()], false)
        } else if file_name.to_lowercase().ends_with(".jar") { (file_name.as_str(), true) }
        else { continue; };
        if safe_filename(name).is_ok() { mods.push(LocalMod { name: name.to_string(), enabled }); }
    }
    mods.sort_by(|a,b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(mods)
}

#[tauri::command]
pub fn toggle_local_mod(app: AppHandle, instance_id: String, filename: String, enabled: bool) -> Result<Vec<LocalMod>, String> {
    safe_filename(&filename)?;
    let dir = game_mods(&app, &instance_id)?;
    let active = dir.join(&filename);
    let inactive = dir.join(format!("{filename}.disabled"));
    let (from, to) = if enabled { (&inactive, &active) } else { (&active, &inactive) };
    if !from.is_file() || to.exists() { return Err("Mod not found or target filename already exists".into()); }
    std::fs::rename(from, to).map_err(|e| e.to_string())?;
    list_local_mods(app, instance_id)
}

#[tauri::command]
pub fn remove_local_mod(app: AppHandle, instance_id: String, filename: String) -> Result<Vec<LocalMod>, String> {
    safe_filename(&filename)?;
    let dir = game_mods(&app, &instance_id)?;
    let enabled = dir.join(&filename);
    let disabled = dir.join(format!("{filename}.disabled"));
    let path: &Path = if enabled.is_file() { &enabled } else if disabled.is_file() { &disabled } else { return Err("Mod file not found".into()); };
    std::fs::remove_file(path).map_err(|e| e.to_string())?;
    list_local_mods(app, instance_id)
}

#[cfg(test)] mod tests {
    use super::*;
    #[test] fn validates_names() {
        assert!(safe_filename("sodium-1.0.jar").is_ok());
        assert!(safe_filename("../../bad.jar").is_err());
        assert!(safe_filename("a.jar.disabled").is_err());
    }
}
