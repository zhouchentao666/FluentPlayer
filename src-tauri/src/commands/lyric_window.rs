use tauri::{
    AppHandle, Manager, PhysicalPosition, PhysicalSize, WebviewBuilder, WebviewUrl, WindowBuilder,
};

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

    // 已存在则直接显示并应用保存的布局/锁定状态
    if let Some(win) = app.get_webview_window(LABEL) {
        let cfg = desktop_lyric_value(&app);
        let width = num(&cfg, "width", 800.0).max(200.0);
        let height = num(&cfg, "height", 180.0).max(80.0);
        let x = num(&cfg, "x", 0.0);
        let y = num(&cfg, "y", 0.0);
        let is_lock = cfg
            .get("isLock")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let _ = win.set_size(PhysicalSize::new(width as u32, height as u32));
        if x != 0.0 || y != 0.0 {
            let _ = win.set_position(PhysicalPosition::new(x as i32, y as i32));
        }
        if is_lock {
            let _ = win.set_ignore_cursor_events(true);
        }
        let _ = win.show();
        return Ok(());
    }

    // 按需创建透明窗口：WindowBuilder 提供稳定的 transparent()，
    // 再附加 WebviewBuilder 加载前端页面。
    let cfg = desktop_lyric_value(&app);
    let width = num(&cfg, "width", 800.0).max(200.0);
    let height = num(&cfg, "height", 180.0).max(80.0);
    let x = num(&cfg, "x", 0.0);
    let y = num(&cfg, "y", 0.0);
    let is_lock = cfg
        .get("isLock")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let window = WindowBuilder::new(&app, LABEL)
        .title("桌面歌词")
        .decorations(false)
        .transparent(true)
        .shadow(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(true)
        .maximizable(false)
        .minimizable(false)
        .inner_size(width, height)
        .build()
        .map_err(|e| format!("创建桌面歌词窗口失败: {e}"))?;

    WebviewBuilder::new(
        LABEL,
        WebviewUrl::App("index.html?desktopLyric=1".into()),
    )
    .build(&window)
    .map_err(|e| format!("创建桌面歌词 webview 失败: {e}"))?;

    if x != 0.0 || y != 0.0 {
        let _ = window.set_position(PhysicalPosition::new(x as i32, y as i32));
    }
    if is_lock {
        let _ = window.set_ignore_cursor_events(true);
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
