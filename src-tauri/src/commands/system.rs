use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_autostart::ManagerExt;

/// 在资源管理器中定位文件
#[tauri::command]
pub fn open_in_explorer(path: String) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .args(["/select,", &path])
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .args(["-R", &path])
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "linux")]
    {
        let dir = std::path::Path::new(&path)
            .parent()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or(path.clone());
        std::process::Command::new("xdg-open")
            .arg(dir)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[tauri::command]
pub fn quit_app(app: AppHandle) {
    app.exit(0);
}

pub fn show_main(app: &AppHandle) {
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.show();
        let _ = win.unminimize();
        let _ = win.set_focus();
    }
}

#[tauri::command]
pub fn show_main_window(app: AppHandle) {
    show_main(&app);
}

/// 开机自启动开关
#[tauri::command]
pub fn apply_auto_start(app: AppHandle, enabled: bool) -> Result<(), String> {
    let manager = app.autolaunch();
    if enabled {
        manager.enable().map_err(|e| e.to_string())
    } else {
        // 未启用时 disable 可能报错，忽略即可
        let _ = manager.disable();
        Ok(())
    }
}

/// 广播本地元数据变更（歌曲编辑器保存后通知主窗口刷新）
#[tauri::command]
pub fn emit_metadata_changed(app: AppHandle) -> Result<(), String> {
    app.emit("localmetadata:changed", ()).map_err(|e| e.to_string())
}

/// 在主窗口内打开歌曲信息编辑器浮窗（向主窗口发送事件，由前端渲染浮窗）
#[tauri::command]
pub async fn open_song_editor(app: AppHandle, path: String) -> Result<(), String> {
    app.emit_to("main", "open-song-editor", path)
        .map_err(|e| format!("通知主窗口打开编辑器失败: {e}"))?;
    Ok(())
}

/// 关闭到托盘偏好（前端在关闭时自行处理隐藏逻辑，这里仅保留兼容命令）
#[tauri::command]
pub fn set_close_to_tray(_enabled: bool) {}
