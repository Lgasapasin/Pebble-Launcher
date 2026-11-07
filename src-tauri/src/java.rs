//! Resolve a compatible Java runtime automatically; install Temurin in application data when necessary.
use crate::{net, storage};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{collections::HashSet, io::Read, path::{Path, PathBuf}};
use tauri::{AppHandle, Emitter};
use tokio::process::Command;

#[derive(Deserialize)]
struct AdoptiumAsset { binary: AdoptiumBinary }
#[derive(Deserialize)]
struct AdoptiumBinary { package: AdoptiumPackage }
#[derive(Deserialize)]
struct AdoptiumPackage { link: String, checksum: String }

fn java_exec(folder: &Path) -> PathBuf {
    folder.join("bin").join(if cfg!(windows) { "java.exe" } else { "java" })
}

fn major_from_version(output: &str) -> Option<u32> {
    for line in output.lines() {
        if !line.contains("version") && !line.contains("openjdk") { continue; }
        let field = line.split('"').nth(1).unwrap_or_else(|| line.split_whitespace().nth(1).unwrap_or(""));
        let parts: Vec<&str> = field.split(|c: char| !c.is_ascii_digit() && c != '.').collect();
        let numbers: Vec<u32> = parts.first()?.split('.').filter_map(|s| s.parse().ok()).collect();
        if numbers.first() == Some(&1) { return numbers.get(1).copied(); }
        if let Some(major) = numbers.first() { return Some(*major); }
    }
    None
}

async fn major(path: &Path) -> Option<u32> {
    let output = Command::new(path).arg("-version").output().await.ok()?;
    if !output.status.success() { return None; }
    let text = format!("{}\n{}", String::from_utf8_lossy(&output.stderr), String::from_utf8_lossy(&output.stdout));
    major_from_version(&text)
}

fn candidates(root: &Path, manual: &str, expected: u32) -> Vec<PathBuf> {
    let mut values = Vec::new();
    if !manual.is_empty() && manual != "auto" { values.push(PathBuf::from(manual)); }
    values.push(java_exec(&root.join("runtimes").join(format!("temurin-{expected}"))));
    if let Ok(home) = std::env::var("JAVA_HOME") { values.push(java_exec(Path::new(&home))); }
    if cfg!(target_os = "linux") {
        if let Ok(entries) = std::fs::read_dir("/usr/lib/jvm") {
            for entry in entries.flatten() { values.push(java_exec(&entry.path())); }
        }
    }
    if cfg!(target_os = "windows") {
        for key in ["ProgramFiles", "ProgramFiles(x86)"] {
            if let Ok(base) = std::env::var(key) {
                for vendor in ["Eclipse Adoptium", "Java", "Microsoft"] {
                    if let Ok(entries) = std::fs::read_dir(Path::new(&base).join(vendor)) {
                        for entry in entries.flatten() { values.push(java_exec(&entry.path())); }
                    }
                }
            }
        }
    }
    values.push(PathBuf::from(if cfg!(windows) { "java.exe" } else { "java" }));
    let mut seen = HashSet::new();
    values.into_iter().filter(|p| seen.insert(p.clone())).collect()
}

fn matches_runtime(found: u32, required: u32) -> bool {
    // Mojang's javaVersion is the correct runtime major for the version; choose an exact match.
    found == required
}

pub async fn resolve(app: &AppHandle, manual: &str, required: u32) -> Result<PathBuf, String> {
    let root = storage::root(app)?;
    for candidate in candidates(&root, manual, required) {
        if major(&candidate).await.is_some_and(|m| matches_runtime(m, required)) { return Ok(candidate); }
    }
    install_managed(app, &root, required).await
}

async fn install_managed(app: &AppHandle, root: &Path, required: u32) -> Result<PathBuf, String> {
    if !cfg!(target_arch = "x86_64") { return Err(format!("No compatible Java {required} runtime found; automatic download currently supports x86_64 only")); }
    let os = if cfg!(windows) { "windows" } else if cfg!(target_os = "linux") { "linux" } else { return Err("Automatic Java download isn't available for this platform".into()); };
    let client = net::client()?;
    let url = format!("https://api.adoptium.net/v3/assets/latest/{required}/hotspot?architecture=x64&image_type=jre&os={os}&vendor=eclipse");
    let assets: Vec<AdoptiumAsset> = net::get_json(&client, &url).await?;
    let package = &assets.first().ok_or_else(|| format!("No trusted Java {required} runtime available for {os}"))?.binary.package;
    if package.checksum.len() != 64 || !package.checksum.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("Java download metadata has no valid SHA-256 checksum".into());
    }
    // Adoptium archive URLs are release assets on GitHub; validate the origin before fetching.
    let parsed = reqwest::Url::parse(&package.link).map_err(|_| "Invalid Java runtime URL")?;
    if parsed.scheme() != "https" || parsed.host_str() != Some("github.com") || !parsed.path().starts_with("/adoptium/temurin") {
        return Err("Unexpected Java runtime download origin".into());
    }
    let _ = app.emit("game-event", serde_json::json!({"version":"java", "message":format!("Downloading compatible Java {required} runtime…")}));
    let data = client.get(&package.link).send().await.map_err(|e| e.to_string())?
        .error_for_status().map_err(|e| e.to_string())?.bytes().await.map_err(|e| e.to_string())?;
    if format!("{:x}", Sha256::digest(&data)).to_lowercase() != package.checksum.to_lowercase() {
        return Err("Downloaded Java runtime failed SHA-256 validation".into());
    }
    let runtime_root = root.join("runtimes");
    let installation = runtime_root.join(format!("temurin-{required}"));
    let staging = runtime_root.join(format!("temurin-{required}.partial"));
    let _ = tokio::fs::remove_dir_all(&staging).await;
    tokio::fs::create_dir_all(&staging).await.map_err(|e| e.to_string())?;
    let archive = data.to_vec();
    let output = staging.clone();
    tokio::task::spawn_blocking(move || -> Result<(), String> {
        if cfg!(windows) {
            let zip = zip::ZipArchive::new(std::io::Cursor::new(archive)).map_err(|e| e.to_string())?;
            extract_zip(zip, &output)?;
        } else {
            let tar = tar::Archive::new(flate2::read::GzDecoder::new(std::io::Cursor::new(archive)));
            extract_tar(tar, &output)?;
        }
        Ok(())
    }).await.map_err(|e| e.to_string())??;
    // Both distribution formats normally contain one top-level directory.
    let java = locate_java(&staging).ok_or("Downloaded Java archive does not contain bin/java")?;
    let base = java.parent().and_then(|p| p.parent()).ok_or("Invalid Java layout")?;
    let relocated = staging.join("runtime");
    std::fs::rename(base, &relocated).map_err(|e| e.to_string())?;
    if installation.exists() { tokio::fs::remove_dir_all(&installation).await.map_err(|e| e.to_string())?; }
    tokio::fs::rename(&relocated, &installation).await.map_err(|e| e.to_string())?;
    let _ = tokio::fs::remove_dir_all(&staging).await;
    let java = java_exec(&installation);
    #[cfg(unix)] {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&java).map_err(|e| e.to_string())?.permissions();
        perms.set_mode(perms.mode() | 0o100);
        std::fs::set_permissions(&java, perms).map_err(|e| e.to_string())?;
    }
    if major(&java).await != Some(required) { return Err(format!("Installed Java runtime did not pass Java {required} verification")); }
    Ok(java)
}

fn locate_java(dir: &Path) -> Option<PathBuf> {
    let direct = java_exec(dir);
    if direct.exists() { return Some(direct); }
    let mut search = vec![(dir.to_path_buf(), 0usize)];
    while let Some((folder, depth)) = search.pop() {
        if depth > 3 { continue; }
        for entry in std::fs::read_dir(folder).ok()?.flatten() {
            if !entry.path().is_dir() { continue; }
            let path = entry.path();
            if java_exec(&path).exists() { return Some(java_exec(&path)); }
            search.push((path, depth+1));
        }
    }
    None
}

fn extract_zip<R: Read + std::io::Seek>(mut archive: zip::ZipArchive<R>, dest: &Path) -> Result<(), String> {
    for i in 0..archive.len() {
        let mut file = archive.by_index(i).map_err(|e| e.to_string())?;
        let relative = file.enclosed_name().ok_or("Unsafe runtime path in archive")?;
        let path = dest.join(relative);
        if file.is_dir() { std::fs::create_dir_all(&path).map_err(|e| e.to_string())?; continue; }
        if let Some(parent) = path.parent() { std::fs::create_dir_all(parent).map_err(|e| e.to_string())?; }
        let mut output = std::fs::File::create(path).map_err(|e| e.to_string())?;
        std::io::copy(&mut file, &mut output).map_err(|e| e.to_string())?;
    }
    Ok(())
}
fn extract_tar<R: Read>(mut archive: tar::Archive<R>, dest: &Path) -> Result<(), String> {
    // The archive is authenticated with Adoptium's SHA-256 checksum before extraction.
    // unpack_in prevents archive paths from escaping the destination and preserves
    // executable permissions and the internal symlinks used by Java distributions.
    for entry in archive.entries().map_err(|e| e.to_string())? {
        let mut item = entry.map_err(|e| e.to_string())?;
        if !item.unpack_in(dest).map_err(|e| e.to_string())? {
            return Err("Unsafe path in Java runtime archive".into());
        }
    }
    Ok(())
}

#[cfg(test)] mod tests {
    use super::*;
    #[test] fn parse_versions() {
        assert_eq!(major_from_version("openjdk version \"21.0.9\" 2025-10-21"), Some(21));
        assert_eq!(major_from_version("java version \"1.8.0_402\""), Some(8));
        assert_eq!(major_from_version("openjdk 25.0.1 2025-10-21"), Some(25));
    }
    #[test] fn match_major() { assert!(matches_runtime(21,21)); assert!(!matches_runtime(21,25)); }
}
