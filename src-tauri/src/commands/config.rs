use serde_json::Value;
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

fn config_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_config_dir()
        .map_err(|e| format!("无法获取配置目录: {e}"))?;
    fs::create_dir_all(&dir).map_err(|e| format!("无法创建配置目录: {e}"))?;
    Ok(dir.join("config.json"))
}

#[tauri::command]
pub fn save_config(app: AppHandle, config: Value) -> Result<(), String> {
    let path = config_path(&app)?;
    let data = serde_json::to_vec_pretty(&config).map_err(|e| e.to_string())?;
    fs::write(&path, data).map_err(|e| format!("写入配置失败: {e}"))
}

#[tauri::command]
pub fn load_config(app: AppHandle) -> Result<Value, String> {
    let path = config_path(&app)?;
    let data = fs::read_to_string(&path).map_err(|e| format!("读取配置失败: {e}"))?;
    serde_json::from_str(&data).map_err(|e| format!("解析配置失败: {e}"))
}

/// 读取配置中的 settings.desktopLyric 段（供桌面歌词窗口初始化）
pub fn desktop_lyric_value(app: &AppHandle) -> Value {
    load_config(app.clone())
        .ok()
        .and_then(|cfg| {
            cfg.get("settings")
                .and_then(|s| s.get("desktopLyric"))
                .cloned()
        })
        .unwrap_or(Value::Null)
}

#[tauri::command]
pub fn get_desktop_lyric_config(app: AppHandle) -> Result<Value, String> {
    Ok(desktop_lyric_value(&app))
}
