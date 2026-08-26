pub mod config_commands;
pub mod directory_commands;
pub mod hotkey_commands;
pub mod music_commands;
pub mod playlist_commands;
pub mod power_save;
pub mod s3_commands;

use crate::db::AppDb;
use crate::network::{clamp_proxy_timeout, decode_response_text, HttpPolicy, RestrictedHttpClient};
use base64::Engine;
use serde::Serialize;
use serde_json::Value;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager, State, WebviewUrl, WebviewWindowBuilder, WindowEvent};

pub type DbState<'a> = State<'a, AppDb>;

const AUDIO_PROXY_MAX_BYTES: usize = 64 * 1024 * 1024;
const HTTP_PROXY_MAX_BYTES: usize = 64 * 1024 * 1024;

/// Shared state for the desktop lyric window
pub struct DesktopLyricState {
    pub is_open: Mutex<bool>,
    pub is_locked: Mutex<bool>,
}

/// Open or close the desktop lyric window.
///
/// Called via `ipcSend('change-desktop-lyric', bool)` which maps to
/// `invoke('change_desktop_lyric', { args: [bool] })`.
fn lyric_window_state_path() -> std::path::PathBuf {
    crate::db::get_app_data_dir().join("desktop_lyric_window.json")
}

fn save_lyric_window_state(is_open: bool, x: i32, y: i32) {
    let path = lyric_window_state_path();
    let json = serde_json::json!({"is_open": is_open, "x": x, "y": y});
    let _ = std::fs::write(
        &path,
        serde_json::to_string_pretty(&json).unwrap_or_default(),
    );
}

/// Create the desktop lyric window with saved or default position.
pub fn create_desktop_lyric_window(app: &AppHandle) -> Result<(), String> {
    let win_width = 600.0_f64;
    let win_height = 180.0_f64;

    let mut builder = WebviewWindowBuilder::new(
        app,
        "desktop-lyric",
        WebviewUrl::App("#/desktop-lyric".into()),
    )
    .title("Desktop Lyric")
    .inner_size(win_width, win_height);

    #[cfg(desktop)]
    {
        builder = builder
            .always_on_top(true)
            .decorations(false)
            .transparent(true)
            .skip_taskbar(true)
            .resizable(false)
            .shadow(false);
    }

    // Try to restore saved position
    let mut has_saved_pos = false;
    let state_path = lyric_window_state_path();
    if let Ok(data) = std::fs::read_to_string(&state_path) {
        if let Ok(val) = serde_json::from_str::<Value>(&data) {
            if let (Some(px), Some(py)) = (
                val.get("x").and_then(|v| v.as_i64()),
                val.get("y").and_then(|v| v.as_i64()),
            ) {
                let scale = app
                    .primary_monitor()
                    .ok()
                    .flatten()
                    .map(|m| m.scale_factor())
                    .unwrap_or(1.0);
                builder = builder.position(px as f64 / scale, py as f64 / scale);
                has_saved_pos = true;
            }
        }
    }

    // Default position: bottom-center
    if !has_saved_pos {
        if let Ok(monitor) = app.primary_monitor() {
            if let Some(m) = monitor {
                let scale = m.scale_factor();
                let screen_w = m.size().width as f64 / scale;
                let screen_h = m.size().height as f64 / scale;
                let x = (screen_w - win_width) / 2.0;
                let y = screen_h - win_height - 100.0;
                builder = builder.position(x, y);
            }
        }
    }

    let window = builder
        .build()
        .map_err(|e| format!("创建桌面歌词窗口失败: {}", e))?;

    #[cfg(debug_assertions)]
    {
        let _ = window.close_devtools();
    }

    // Save position on every move
    let save_path = lyric_window_state_path();
    window.on_window_event(move |event| {
        if let WindowEvent::Moved(pos) = event {
            let json = serde_json::json!({"is_open": true, "x": pos.x, "y": pos.y});
            let _ = std::fs::write(
                &save_path,
                serde_json::to_string_pretty(&json).unwrap_or_default(),
            );
        }
    });

    Ok(())
}

#[tauri::command]
pub async fn change_desktop_lyric(
    app: AppHandle,
    state: State<'_, DesktopLyricState>,
    args: Vec<bool>,
) -> Result<(), String> {
    let open = args.first().copied().unwrap_or(false);

    if open {
        // If window already exists, just show/focus it
        if let Some(existing) = app.get_webview_window("desktop-lyric") {
            let _ = existing.show();
            let _ = existing.set_focus();
            *state.is_open.lock().map_err(|e| e.to_string())? = true;
            return Ok(());
        }

        create_desktop_lyric_window(&app)?;
        *state.is_open.lock().map_err(|e| e.to_string())? = true;
    } else {
        // Close the desktop lyric window and save state
        if let Some(window) = app.get_webview_window("desktop-lyric") {
            if let Ok(pos) = window.outer_position() {
                save_lyric_window_state(false, pos.x, pos.y);
            }
            window
                .close()
                .map_err(|e| format!("关闭桌面歌词窗口失败: {}", e))?;
        }
        *state.is_open.lock().map_err(|e| e.to_string())? = false;
    }

    Ok(())
}

/// Toggle the desktop lyric window's cursor-events-ignore (lock) state.
///
/// When locked the window ignores mouse events (click-through);
/// when unlocked it accepts pointer events normally.
///
/// Called via `ipcSend('toogleDesktopLyricLock', bool)`.
#[tauri::command]
pub async fn toogle_desktop_lyric_lock(
    app: AppHandle,
    state: State<'_, DesktopLyricState>,
    args: Vec<bool>,
) -> Result<(), String> {
    let locked = args.first().copied().unwrap_or(false);
    let _ = &app;

    #[cfg(desktop)]
    if let Some(window) = app.get_webview_window("desktop-lyric") {
        window
            .set_ignore_cursor_events(locked)
            .map_err(|e| format!("设置桌面歌词锁定状态失败: {}", e))?;
    }

    *state.is_locked.lock().map_err(|e| e.to_string())? = locked;
    Ok(())
}

/// Return whether the desktop lyric window is currently open.
#[tauri::command]
pub async fn get_lyric_open_state(state: State<'_, DesktopLyricState>) -> Result<bool, String> {
    let is_open = state.is_open.lock().map_err(|e| e.to_string())?;
    Ok(*is_open)
}

/// Return whether the desktop lyric window is currently locked (ignoring cursor events).
#[tauri::command]
pub async fn get_lyric_lock_state(state: State<'_, DesktopLyricState>) -> Result<bool, String> {
    let is_locked = state.is_locked.lock().map_err(|e| e.to_string())?;
    Ok(*is_locked)
}

/// 获取系统已安装字体列表
#[tauri::command]
pub async fn get_font_list() -> Result<Vec<String>, String> {
    let mut fonts: Vec<String> = Vec::new();

    // macOS 系统字体目录
    let mut dirs: Vec<std::path::PathBuf> = Vec::new();
    dirs.push(std::path::PathBuf::from("/System/Library/Fonts"));
    dirs.push(std::path::PathBuf::from("/Library/Fonts"));
    if let Some(home) = dirs::home_dir() {
        dirs.push(home.join("Library/Fonts"));
    }

    let font_exts = ["ttf", "otf", "ttc", "TTF", "OTF", "TTC"];
    for dir in &dirs {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    if font_exts.contains(&ext) {
                        if let Some(name) = path.file_stem().and_then(|n| n.to_str()) {
                            let name = name.to_string();
                            if !fonts.contains(&name) {
                                fonts.push(name);
                            }
                        }
                    }
                }
            }
        }
    }

    fonts.sort();
    Ok(fonts)
}

/// 桌面歌词选项 — 从 app_data_dir/desktop_lyric_option.json 读取
#[tauri::command]
pub async fn get_desktop_lyric_option() -> Result<Option<Value>, String> {
    let dir = crate::db::get_app_data_dir();
    let path = dir.join("desktop_lyric_option.json");
    if !path.exists() {
        return Ok(None);
    }
    let data =
        std::fs::read_to_string(&path).map_err(|e| format!("读取桌面歌词配置失败: {}", e))?;
    let val: Value =
        serde_json::from_str(&data).map_err(|e| format!("解析桌面歌词配置失败: {}", e))?;
    Ok(Some(val))
}

/// 桌面歌词选项 — 保存到 app_data_dir/desktop_lyric_option.json 并广播事件到桌面歌词窗口
#[tauri::command]
pub async fn set_desktop_lyric_option(app: AppHandle, args: Vec<Value>) -> Result<(), String> {
    let options = args.first().ok_or("缺少配置参数")?.clone();
    let dir = crate::db::get_app_data_dir();
    std::fs::create_dir_all(&dir).map_err(|e| format!("创建目录失败: {}", e))?;
    let path = dir.join("desktop_lyric_option.json");
    let data =
        serde_json::to_string_pretty(&options).map_err(|e| format!("序列化配置失败: {}", e))?;
    std::fs::write(&path, data).map_err(|e| format!("写入桌面歌词配置失败: {}", e))?;
    // 广播到桌面歌词窗口使其实时更新
    app.emit("desktop-lyric-style-change", options)
        .map_err(|e| format!("广播样式事件失败: {}", e))?;
    Ok(())
}

/// Audio proxy — fetches a remote audio URL via Rust backend (bypassing CORS)
/// and returns a `data:` URI that the WebView `<audio>` element can play.
/// The 64 MiB response limit leaves headroom for the base64-encoded data URI.
/// Follows up to 10 redirects.
#[tauri::command]
pub async fn audio_proxy(url: String) -> Result<String, String> {
    let response = RestrictedHttpClient::new(HttpPolicy::public_proxy())
        .fetch_bytes(&url, AUDIO_PROXY_MAX_BYTES, &["audio/*"])
        .await
        .map_err(|error| format!("音频请求失败: {error}"))?;
    let content_type = response
        .headers
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("audio/mpeg")
        .to_string();
    let mut data_uri = format!("data:{content_type};base64,");
    base64::engine::general_purpose::STANDARD.encode_string(&response.bytes, &mut data_uri);
    Ok(data_uri)
}

/// HTTP proxy command — bypasses CORS by making requests from the Rust backend.
/// Used by plugins in the WebView that need to call cross-origin APIs.
///
/// Supports `raw` mode: when `raw: true`, returns the response body as a base64 string
/// (useful for binary data like audio files). The response format changes:
///   { "statusCode": N, "headers": {}, "body": "<base64>", "isBase64": true }
///
/// By default (raw: false), follows redirects and returns JSON-wrapped response:
///   { "statusCode": N, "headers": {}, "body": <parsed JSON or string> }
/// Response bodies are limited to 64 MiB to retain headroom when raw mode base64 encodes them.
#[tauri::command]
pub async fn http_proxy(args: Value) -> Result<Value, String> {
    let url = args
        .get("url")
        .and_then(|v| v.as_str())
        .ok_or("缺少 url 参数")?;
    let method = args
        .get("method")
        .and_then(|v| v.as_str())
        .unwrap_or("GET")
        .parse::<reqwest::Method>()
        .map_err(|error| format!("HTTP 方法无效: {error}"))?;
    let headers: std::collections::HashMap<String, String> = args
        .get("headers")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();
    let body = args
        .get("body")
        .and_then(|v| v.as_str())
        .map(|value| value.as_bytes().to_vec());
    let timeout = clamp_proxy_timeout(
        args.get("timeout")
            .and_then(|value| value.as_u64())
            .unwrap_or(15_000),
    );
    let raw = args.get("raw").and_then(|v| v.as_bool()).unwrap_or(false);
    let response = RestrictedHttpClient::new(HttpPolicy::public_proxy())
        .proxy_request_bytes(method, url, headers, body, HTTP_PROXY_MAX_BYTES, timeout)
        .await?;
    let status = response.status.as_u16();
    let response_headers: std::collections::HashMap<String, String> = response
        .headers
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
        .collect();

    if raw {
        let body_base64 = base64::engine::general_purpose::STANDARD.encode(&response.bytes);
        return Ok(serde_json::json!({
            "statusCode": status,
            "headers": response_headers,
            "body": body_base64,
            "isBase64": true
        }));
    }

    let text = decode_response_text(&response.headers, &response.bytes);
    let body_value: Value = serde_json::from_str(&text).unwrap_or(Value::String(text));

    Ok(serde_json::json!({
        "statusCode": status,
        "headers": response_headers,
        "body": body_value
    }))
}

#[cfg(test)]
mod http_proxy_tests {
    use crate::network::{
        is_blocked_network_ip, strip_cross_origin_credentials, validate_public_http_url,
    };
    use std::collections::HashMap;
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

    #[test]
    fn blocks_non_public_ip_ranges() {
        let blocked = [
            IpAddr::V4(Ipv4Addr::LOCALHOST),
            IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)),
            IpAddr::V4(Ipv4Addr::new(169, 254, 1, 1)),
            IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)),
            IpAddr::V6(Ipv6Addr::LOCALHOST),
            "fc00::1".parse().unwrap(),
            "fe80::1".parse().unwrap(),
        ];
        assert!(blocked.into_iter().all(is_blocked_network_ip));
        assert!(!is_blocked_network_ip(IpAddr::V4(Ipv4Addr::new(
            1, 1, 1, 1
        ))));
    }

    #[tokio::test]
    async fn rejects_local_and_non_http_urls() {
        assert!(validate_public_http_url("file:///etc/passwd")
            .await
            .is_err());
        assert!(validate_public_http_url("http://127.0.0.1:8080")
            .await
            .is_err());
        assert!(validate_public_http_url("http://[::1]/").await.is_err());
        assert!(validate_public_http_url("http://localhost/").await.is_err());
        assert!(validate_public_http_url("http://user:pass@example.com/")
            .await
            .is_err());
    }

    #[tokio::test]
    async fn accepts_public_ip_url() {
        let target = validate_public_http_url("https://1.1.1.1/path")
            .await
            .unwrap();
        assert_eq!("1.1.1.1", target.url.host_str().unwrap());
    }

    #[test]
    fn strips_credentials_before_cross_origin_redirect() {
        let mut headers = HashMap::from([
            ("Authorization".to_string(), "Bearer secret".to_string()),
            ("Cookie".to_string(), "session=secret".to_string()),
            ("Accept".to_string(), "application/json".to_string()),
        ]);
        strip_cross_origin_credentials(&mut headers);
        assert_eq!(Some(&"application/json".to_string()), headers.get("Accept"));
        assert!(!headers.contains_key("Authorization"));
        assert!(!headers.contains_key("Cookie"));
    }
}

#[derive(Serialize)]
pub struct MemorySnapshot {
    pub rss_mb: Option<f64>,
}

#[allow(non_snake_case)]
#[tauri::command]
pub async fn performance__memory() -> Result<MemorySnapshot, String> {
    #[cfg(target_os = "macos")]
    {
        use std::process::Command;
        let pid = std::process::id().to_string();
        let output = Command::new("ps")
            .args(["-o", "rss=", "-p", &pid])
            .output()
            .map_err(|e| format!("读取内存失败: {}", e))?;

        if !output.status.success() {
            return Ok(MemorySnapshot { rss_mb: None });
        }

        let text = String::from_utf8_lossy(&output.stdout);
        let kb = text.trim().parse::<f64>().ok();
        let rss_mb = kb.map(|v| v / 1024.0);
        return Ok(MemorySnapshot { rss_mb });
    }

    #[cfg(not(target_os = "macos"))]
    {
        Ok(MemorySnapshot { rss_mb: None })
    }
}
