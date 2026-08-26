#![allow(non_snake_case)]

use crate::commands::DbState;
use crate::db::playlist_db;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::OnceLock;
use tauri::AppHandle;
#[cfg(desktop)]
use tauri::{Emitter, Manager};

// ---------------------------------------------------------------------------
// Types (mirror frontend types/hotkeys.ts)
// ---------------------------------------------------------------------------

pub type HotkeyBindings = HashMap<String, String>;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct HotkeyConfig {
    pub enabled: bool,
    pub bindings: HotkeyBindings,
}

#[derive(Serialize)]
pub struct HotkeyStatus {
    pub failed_actions: Vec<String>,
}

const KV_KEY: &str = "hotkeys";

// ---------------------------------------------------------------------------
// App handle storage (for re-registration from commands)
// ---------------------------------------------------------------------------

static APP_HANDLE: OnceLock<AppHandle> = OnceLock::new();

pub fn set_app_handle(handle: AppHandle) {
    let _ = APP_HANDLE.set(handle);
}

fn default_config() -> HotkeyConfig {
    let mut bindings = HotkeyBindings::new();
    bindings.insert("toggle".into(), "CommandOrControl+Alt+P".into());
    bindings.insert("playPrev".into(), "CommandOrControl+Alt+Left".into());
    bindings.insert("playNext".into(), "CommandOrControl+Alt+Right".into());
    bindings.insert("seekBackward".into(), "CommandOrControl+Alt+J".into());
    bindings.insert("seekForward".into(), "CommandOrControl+Alt+L".into());
    bindings.insert("volumeDown".into(), "CommandOrControl+Alt+Down".into());
    bindings.insert("volumeUp".into(), "CommandOrControl+Alt+Up".into());
    bindings.insert("toggleDesktopLyric".into(), "CommandOrControl+Alt+D".into());
    bindings.insert("setPlayModeSequence".into(), "CommandOrControl+Alt+4".into());
    bindings.insert("setPlayModeRandom".into(), "CommandOrControl+Alt+5".into());
    bindings.insert("togglePlayModeSingle".into(), "CommandOrControl+Alt+6".into());
    HotkeyConfig { enabled: true, bindings }
}

pub fn load_config_from_db(conn: &rusqlite::Connection) -> HotkeyConfig {
    playlist_db::ensure_kv_table(conn).ok();
    match playlist_db::kv_get(conn, KV_KEY) {
        Ok(Some(json_str)) => {
            serde_json::from_str::<HotkeyConfig>(&json_str).unwrap_or_else(|_| default_config())
        }
        _ => default_config(),
    }
}

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn hotkeys__get(state: DbState<'_>) -> Result<serde_json::Value, String> {
    let conn = state.playlist.lock().map_err(|e| e.to_string())?;
    let config = load_config_from_db(&conn);
    let status = HotkeyStatus { failed_actions: Vec::new() };
    Ok(serde_json::json!({
        "success": true,
        "data": config,
        "status": status,
    }))
}

#[tauri::command]
pub fn hotkeys__set(state: DbState<'_>, args: serde_json::Value) -> Result<serde_json::Value, String> {
    let payload = args.get("args").cloned().unwrap_or(args);
    let enabled = payload.get("enabled").and_then(|v| v.as_bool()).unwrap_or(true);
    let bindings: HotkeyBindings = payload
        .get("bindings")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();

    let config = HotkeyConfig { enabled, bindings };

    // Persist to DB
    {
        let conn = state.playlist.lock().map_err(|e| e.to_string())?;
        playlist_db::ensure_kv_table(&conn).map_err(|e| e.to_string())?;
        let json_str = serde_json::to_string(&config).map_err(|e| e.to_string())?;
        playlist_db::kv_set(&conn, KV_KEY, &json_str).map_err(|e| e.to_string())?;
    }

    // Re-register OS-level shortcuts
    if let Some(handle) = APP_HANDLE.get() {
        re_register_shortcuts(handle, &config);
    }

    let status = HotkeyStatus { failed_actions: Vec::new() };
    Ok(serde_json::json!({
        "success": true,
        "data": config,
        "status": status,
    }))
}

// ---------------------------------------------------------------------------
// Shortcut re-registration (called from commands and setup)
// ---------------------------------------------------------------------------

#[allow(unused_variables)]
pub fn re_register_shortcuts(app_handle: &AppHandle, config: &HotkeyConfig) {
    #[cfg(desktop)]
    {
        use tauri_plugin_global_shortcut::{GlobalShortcut, ShortcutState};

    let shortcut_plugin = app_handle.state::<GlobalShortcut<tauri::Wry>>();

    let _ = shortcut_plugin.unregister_all();

    if !config.enabled {
        return;
    }

    for (action, accelerator) in &config.bindings {
        if accelerator.is_empty() {
            continue;
        }
        let action_clone = action.clone();
        let handle_clone = app_handle.clone();
        let acc = accelerator.clone();

        let _ = shortcut_plugin.on_shortcut(acc.as_str(), move |_app, _shortcut, event| {
            if event.state == ShortcutState::Pressed {
                let _ = handle_clone.emit("hotkey-triggered", action_clone.clone());
            }
        });
    }
    }
}
