use tauri::{AppHandle, Manager, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindowBuilder};

use super::config::desktop_lyric_value;

const LABEL: &str = "desktop-lyric";

fn num(value: &serde_json::Value, key: &str, fallback: f64) -> f64 {
    value.get(key).and_then(|v| v.as_f64()).unwrap_or(fallback)
}

/// 打开 / 隐藏桌面歌词窗口
#[tauri::command]
pub async fn toggle_desktop_lyric(app: AppHandle, enabled: bool) -> Result<(), String> {
    if !enabled {
        if let Some(win) = app.get_webview_window(LABEL) {
            let _ = win.hide();
        }
        return Ok(());
    }

    if let Some(win) = app.get_webview_window(LABEL) {
        let _ = win.show();
        return Ok(());
    }

    let cfg = desktop_lyric_value(&app);
    let width = num(&cfg, "width", 800.0).max(200.0);
    let height = num(&cfg, "height", 180.0).max(80.0);
    let x = num(&cfg, "x", 0.0);
    let y = num(&cfg, "y", 0.0);
    let is_lock = cfg
        .get("isLock")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let win = WebviewWindowBuilder::new(
        &app,
        LABEL,
        WebviewUrl::App("index.html?desktopLyric=1".into()),
    )
    .title("桌面歌词")
    .decorations(false)
    .shadow(false)
    .always_on_top(true)
    .skip_taskbar(true)
    .resizable(true)
    .maximizable(false)
    .minimizable(false)
    // transparent 受条件编译约束：仅当启用 macos-private-api 或非 macOS 平台才存在该方法。
    // 与 tauri 内部一致，避免在 macOS 未开启该 feature 时编译失败。
    #[cfg(any(feature = "macos-private-api", not(target_os = "macos")))]
    .transparent(true)
    .inner_size(width, height)
    .build()
    .map_err(|e| format!("创建桌面歌词窗口失败: {e}"))?;

    if x != 0.0 || y != 0.0 {
        let _ = win.set_position(PhysicalPosition::new(x as i32, y as i32));
        let _ = win.set_size(PhysicalSize::new(width as u32, height as u32));
    }
    if is_lock {
        let _ = win.set_ignore_cursor_events(true);
    }
    Ok(())
}

/// 关闭桌面歌词窗口
#[tauri::command]
pub fn close_desktop_lyric(app: AppHandle) -> Result<(), String> {
    if let Some(win) = app.get_webview_window(LABEL) {
        let _ = win.destroy();
    }
    Ok(())
}

/// 设置桌面歌词窗口位置与大小（物理像素）
#[tauri::command]
pub fn set_desktop_lyric_bounds(
    app: AppHandle,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
) -> Result<(), String> {
    if let Some(win) = app.get_webview_window(LABEL) {
        let _ = win.set_position(PhysicalPosition::new(x as i32, y as i32));
        let _ = win.set_size(PhysicalSize::new(width.max(80.0) as u32, height.max(40.0) as u32));
    }
    Ok(())
}

/// 锁定时忽略鼠标事件（点击穿透）
#[tauri::command]
pub fn set_desktop_lyric_ignore_mouse_events(app: AppHandle, ignore: bool) -> Result<(), String> {
    if let Some(win) = app.get_webview_window(LABEL) {
        win.set_ignore_cursor_events(ignore)
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}
