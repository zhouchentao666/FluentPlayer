use serde::Serialize;
use tauri::{Manager, WebviewUrl, WebviewWindowBuilder, WindowConfig, WindowEvent};

#[derive(Clone, Serialize)]
struct LyricPayload {
    title: String,
    artist: String,
    album: String,
    cover: String,
}

#[tauri::command]
pub fn toggle_desktop_lyric(
    app: tauri::AppHandle,
    payload: LyricPayload,
) -> Result<bool, String> {
    const LABEL: &str = "desktop-lyric";

    // 如果已存在桌面歌词窗口，则关闭它
    if let Some(existing) = app.get_webview_window(LABEL) {
        existing.close().map_err(|e| e.to_string())?;
        return Ok(false);
    }

    // 透明效果通过 WindowConfig 传入，避免调用受 macos-private-api cfg 限制的
    // WebviewWindowBuilder::transparent 方法（旧版 Tauri / 未启用该 feature 时编译失败）。
    let config = WindowConfig {
        label: LABEL.to_string(),
        url: WebviewUrl::App("/index.html?desktopLyric=1".into()),
        title: "桌面歌词".to_string(),
        width: 800.0,
        height: 180.0,
        decorations: false,
        transparent: true,
        shadow: false,
        always_on_top: true,
        resizable: false,
        visible: false,
        drag_drop_enabled: false,
        ..Default::default()
    };

    let window = WebviewWindowBuilder::from_config(&app, &config)
        .map_err(|e| e.to_string())?
        .build()
        .map_err(|e| e.to_string())?;

    window.on_window_event(move |event| {
        if let WindowEvent::CloseRequested { .. } = event {
            // 关闭事件无需额外处理，下次调用会重新创建
        }
    });

    // 注入歌词数据
    let script = format!(
        "window.__lyricData = {}; window.dispatchEvent(new CustomEvent('lyric-update', {{ detail: {} }}));",
        serde_json::to_string(&payload).unwrap_or_else(|_| "{}".to_string()),
        serde_json::to_string(&payload).unwrap_or_else(|_| "{}".to_string())
    );
    let _ = window.eval(&script);

    window.show().map_err(|e| e.to_string())?;
    window.set_focus().ok();

    Ok(true)
}

#[tauri::command]
pub fn close_desktop_lyric(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("desktop-lyric") {
        window.close().map_err(|e| e.to_string())?;
    }
    Ok(())
}
