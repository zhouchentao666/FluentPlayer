use std::process::Child;
#[cfg(target_os = "macos")]
use std::process::{Command, Stdio};
use std::sync::Mutex;
use tauri::State;

pub struct PowerBlocker(Option<Child>);

pub fn power_save_blocker_state() -> PowerBlocker {
    PowerBlocker(None)
}

/// Start a caffeinate child process to prevent display sleep
#[allow(non_snake_case)]
#[tauri::command]
pub async fn power_save_blocker__start(state: State<'_, Mutex<PowerBlocker>>) -> Result<bool, String> {
    let mut guard = state.lock().map_err(|e| e.to_string())?;
    // Already running
    if let Some(ref mut child) = guard.0 {
        match child.try_wait() {
            Ok(Some(_)) => { /* process exited, will restart below */ }
            Ok(None) => return Ok(true), // still running
            Err(_) => { /* stale, will restart below */ }
        }
    }

    #[cfg(target_os = "macos")]
    {
        let child = Command::new("caffeinate")
            .args(["-d"]) // prevent display sleep
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("启动 caffeinate 失败: {}", e))?;
        guard.0 = Some(child);
    }
    #[cfg(not(target_os = "macos"))]
    {
        // No-op on other platforms
    }
    Ok(true)
}

/// Stop the caffeinate child process
#[allow(non_snake_case)]
#[tauri::command]
pub async fn power_save_blocker__stop(state: State<'_, Mutex<PowerBlocker>>) -> Result<bool, String> {
    let mut guard = state.lock().map_err(|e| e.to_string())?;
    if let Some(mut child) = guard.0.take() {
        let _ = child.kill();
        let _ = child.wait();
    }
    Ok(true)
}
