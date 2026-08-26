#![allow(non_snake_case)]

use crate::commands::DbState;
use crate::db::{get_app_data_dir, playlist_db};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Directories {
    cache_dir: String,
    download_dir: String,
}

fn default_dirs() -> Directories {
    let base = get_app_data_dir();
    Directories {
        cache_dir: base.join("cache").to_string_lossy().to_string(),
        download_dir: dirs::download_dir()
            .unwrap_or_else(|| base.join("downloads"))
            .to_string_lossy()
            .to_string(),
    }
}

fn load_dirs_from_db(conn: &rusqlite::Connection) -> Directories {
    playlist_db::ensure_kv_table(conn).ok();
    match playlist_db::kv_get(conn, "directories") {
        Ok(Some(json)) => serde_json::from_str(&json).unwrap_or_else(|_| default_dirs()),
        _ => default_dirs(),
    }
}

fn save_dirs_to_db(conn: &rusqlite::Connection, dirs: &Directories) -> Result<(), String> {
    playlist_db::ensure_kv_table(conn).map_err(|e| e.to_string())?;
    let json = serde_json::to_string(dirs).map_err(|e| e.to_string())?;
    playlist_db::kv_set(conn, "directories", &json).map_err(|e| e.to_string())
}

fn dir_size(path: &str) -> u64 {
    walkdir::WalkDir::new(path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter_map(|e| e.metadata().ok())
        .filter(|m| m.is_file())
        .map(|m| m.len())
        .sum()
}

#[derive(Serialize)]
pub struct CacheInfo {
    count: u64,
    size: u64,
}

fn get_cache_dir(state: &DbState<'_>) -> Result<String, String> {
    let conn = state.playlist.lock().map_err(|e| e.to_string())?;
    Ok(load_dirs_from_db(&conn).cache_dir)
}

#[tauri::command]
pub fn get_directories(state: DbState<'_>) -> Result<Directories, String> {
    let conn = state.playlist.lock().map_err(|e| e.to_string())?;
    Ok(load_dirs_from_db(&conn))
}

#[tauri::command]
pub fn save_directories(
    state: DbState<'_>,
    directories: Directories,
) -> Result<Directories, String> {
    let conn = state.playlist.lock().map_err(|e| e.to_string())?;
    save_dirs_to_db(&conn, &directories)?;
    // Ensure directories exist
    std::fs::create_dir_all(&directories.cache_dir).map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&directories.download_dir).map_err(|e| e.to_string())?;
    Ok(directories)
}

#[tauri::command]
pub fn reset_directories(state: DbState<'_>) -> Result<Directories, String> {
    let defaults = default_dirs();
    let conn = state.playlist.lock().map_err(|e| e.to_string())?;
    save_dirs_to_db(&conn, &defaults)?;
    std::fs::create_dir_all(&defaults.cache_dir).map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&defaults.download_dir).map_err(|e| e.to_string())?;
    Ok(defaults)
}

#[tauri::command]
pub fn get_directory_size(path: String) -> Result<u64, String> {
    if path.is_empty() || !std::path::Path::new(&path).exists() {
        return Ok(0);
    }
    Ok(dir_size(&path))
}

#[tauri::command]
pub fn open_directory(path: String) -> Result<(), String> {
    if path.is_empty() {
        return Err("目录路径为空".to_string());
    }
    #[cfg(target_os = "macos")]
    std::process::Command::new("open")
        .arg(&path)
        .spawn()
        .map_err(|e| e.to_string())?;
    #[cfg(target_os = "windows")]
    std::process::Command::new("explorer")
        .arg(&path)
        .spawn()
        .map_err(|e| e.to_string())?;
    #[cfg(target_os = "linux")]
    std::process::Command::new("xdg-open")
        .arg(&path)
        .spawn()
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn get_cache_info(state: DbState<'_>, _force_refresh: Option<bool>) -> Result<CacheInfo, String> {
    let cache_dir = get_cache_dir(&state)?;
    let path = std::path::Path::new(&cache_dir);
    if !path.exists() {
        return Ok(CacheInfo { count: 0, size: 0 });
    }
    let mut count: u64 = 0;
    let mut size: u64 = 0;
    for entry in walkdir::WalkDir::new(&cache_dir).into_iter().filter_map(|e| e.ok()) {
        if let Ok(meta) = entry.metadata() {
            if meta.is_file() {
                count += 1;
                size += meta.len();
            }
        }
    }
    Ok(CacheInfo { count, size })
}

#[tauri::command]
pub fn clear_cache(state: DbState<'_>) -> Result<(), String> {
    let cache_dir = get_cache_dir(&state)?;
    let path = std::path::Path::new(&cache_dir);
    if !path.exists() {
        return Ok(());
    }
    for entry in std::fs::read_dir(path).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let p = entry.path();
        if p.is_dir() {
            std::fs::remove_dir_all(&p).map_err(|e| e.to_string())?;
        } else {
            std::fs::remove_file(&p).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}
