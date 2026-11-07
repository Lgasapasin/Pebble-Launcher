//! Local instance management. Versions are shared; each instance has its own game directory.
use crate::storage;
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};
use tauri::AppHandle;

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Instance {
    pub id: String,
    pub name: String,
    pub version_id: String,
    #[serde(default = "default_loader")]
    pub loader: String,
    #[serde(default)]
    pub loader_version: Option<String>,
}

fn default_loader() -> String { "vanilla".into() }

fn instance_root(app: &AppHandle, id: &str) -> Result<PathBuf, String> {
    storage::safe_id(id)?;
    if !id.starts_with("custom-") { return Err("Invalid instance identifier".into()); }
    Ok(storage::root(app)?.join("instances").join(id))
}

pub fn find_instance(app: &AppHandle, id: &str) -> Result<Instance, String> {
    let path = instance_root(app, id)?.join("instance.json");
    let data = fs::read(path).map_err(|_| "Instance not found".to_string())?;
    let instance: Instance = serde_json::from_slice(&data).map_err(|_| "Instance metadata invalid".to_string())?;
    if instance.id != id { return Err("Instance metadata does not match folder".into()); }
    storage::safe_id(&instance.version_id)?;
    Ok(instance)
}

#[tauri::command]
pub fn list_instances(app: AppHandle) -> Result<Vec<Instance>, String> {
    let folder = storage::root(&app)?.join("instances");
    if !folder.exists() { return Ok(Vec::new()); }
    let mut result = Vec::new();
    for entry in fs::read_dir(folder).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        if !entry.path().is_dir() { continue; }
        if let Some(id) = entry.file_name().to_str() {
            if let Ok(instance) = find_instance(&app, id) { result.push(instance); }
        }
    }
    result.sort_by(|a,b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(result)
}

#[tauri::command]
pub fn create_instance(app: AppHandle, name: String, version_id: String) -> Result<Instance, String> {
    storage::safe_id(&version_id)?;
    let name = name.trim();
    if name.is_empty() || name.len() > 64 || name.chars().any(char::is_control) {
        return Err("Instance name must be 1–64 printable characters".into());
    }
    let slug: String = name.to_ascii_lowercase().chars().filter_map(|c| {
        if c.is_ascii_alphanumeric() { Some(c) }
        else if c == ' ' || c == '_' || c == '-' { Some('-') }
        else { None }
    }).take(36).collect();
    let slug = slug.trim_matches('-');
    if slug.is_empty() { return Err("Instance name must contain Latin letters or numbers".into()); }
    let mut id = format!("custom-{slug}");
    let mut next = 2;
    while instance_root(&app, &id)?.exists() {
        id = format!("custom-{slug}-{next}");
        next += 1;
        if next > 10000 { return Err("Too many instances with this name".into()); }
    }
    let folder = instance_root(&app, &id)?;
    fs::create_dir_all(folder.join("game")).map_err(|e| e.to_string())?;
    let instance = Instance { id, name: name.into(), version_id, loader: default_loader(), loader_version: None };
    fs::write(folder.join("instance.json"), serde_json::to_vec_pretty(&instance).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    Ok(instance)
}

pub fn set_loader(app: &AppHandle, instance_id: &str, loader: &str, version: Option<String>) -> Result<Instance, String> {
    let mut instance = find_instance(app, instance_id)?;
    instance.loader = loader.into();
    instance.loader_version = version;
    let metadata = instance_root(app, instance_id)?.join("instance.json");
    let temp = metadata.with_extension("json.tmp");
    fs::write(&temp, serde_json::to_vec_pretty(&instance).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    #[cfg(windows)] if metadata.exists() { fs::remove_file(&metadata).map_err(|e| e.to_string())?; }
    fs::rename(temp, metadata).map_err(|e| e.to_string())?;
    Ok(instance)
}

#[tauri::command]
pub fn remove_instance(app: AppHandle, instance_id: String) -> Result<(), String> {
    let _ = find_instance(&app, &instance_id)?;
    // This operation is destructive; the UI must request confirmation first.
    fs::remove_dir_all(instance_root(&app, &instance_id)?).map_err(|e| e.to_string())
}

pub fn game_directory(app: &AppHandle, instance_id: &str, version_id: &str) -> Result<PathBuf, String> {
    let instance = find_instance(app, instance_id)?;
    if instance.version_id != version_id { return Err("Instance version mismatch".into()); }
    Ok(instance_root(app, instance_id)?.join("game"))
}

#[tauri::command]
pub fn list_instance_mods(app: AppHandle, instance_id: String) -> Result<Vec<String>, String> {
    let instance = find_instance(&app, &instance_id)?;
    let folder = game_directory(&app, &instance_id, &instance.version_id)?.join("mods");
    list_directory(&folder, true)
}

#[tauri::command]
pub fn list_instance_worlds(app: AppHandle, instance_id: String) -> Result<Vec<String>, String> {
    let instance = find_instance(&app, &instance_id)?;
    let folder = game_directory(&app, &instance_id, &instance.version_id)?.join("saves");
    list_directory(&folder, false)
}

fn list_directory(dir: &std::path::Path, files: bool) -> Result<Vec<String>, String> {
    if !dir.exists() { return Ok(Vec::new()); }
    let mut items = Vec::new();
    for entry in fs::read_dir(dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let is_match = if files {
            entry.path().is_file() && entry.path().extension().and_then(|s| s.to_str()) == Some("jar")
        } else { entry.path().is_dir() };
        if is_match { if let Some(name) = entry.file_name().to_str() { items.push(name.into()); } }
    }
    items.sort();
    Ok(items)
}

#[tauri::command]
pub fn open_instance_folder(app: AppHandle, instance_id: String, subdirectory: String) -> Result<(), String> {
    if !matches!(subdirectory.as_str(), "game" | "mods" | "saves" | "resourcepacks" | "shaderpacks") {
        return Err("Unsupported folder".into());
    }
    let instance = find_instance(&app, &instance_id)?;
    let base = game_directory(&app, &instance_id, &instance.version_id)?;
    let folder = if subdirectory == "game" { base } else { base.join(subdirectory) };
    fs::create_dir_all(&folder).map_err(|e| e.to_string())?;
    #[cfg(target_os = "linux")]
    let mut command = std::process::Command::new("xdg-open");
    #[cfg(target_os = "windows")]
    let mut command = std::process::Command::new("explorer");
    #[cfg(target_os = "macos")]
    let mut command = std::process::Command::new("open");
    command.arg(folder).spawn().map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn rejects_traversal() { assert!(storage::safe_id("../../etc").is_err()); }
}
