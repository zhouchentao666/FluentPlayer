#[cfg(target_os = "macos")]
mod macos;

use serde::Serialize;
use tauri::AppHandle;

#[derive(Debug, Clone, Serialize)]
pub struct AudioDeviceInfo {
    pub id: u32,
    pub name: String,
    pub is_default: bool,
    pub sample_rate: f64,
    pub channels: u32,
    pub volume: f32,
    pub volume_supported: bool,
}

// ---------------------------------------------------------------------------
// Tauri commands
// ---------------------------------------------------------------------------

#[allow(non_snake_case)]
#[tauri::command]
pub fn audio__enumerate_devices() -> Result<Vec<AudioDeviceInfo>, String> {
    #[cfg(target_os = "macos")]
    {
        macos::enumerate_devices()
            .map(|devices| {
                devices
                    .into_iter()
                    .map(|d| AudioDeviceInfo {
                        id: d.id,
                        name: d.name,
                        is_default: d.is_default,
                        sample_rate: d.sample_rate,
                        channels: d.channels,
                        volume: d.volume,
                        volume_supported: d.volume_supported,
                    })
                    .collect()
            })
    }
    #[cfg(not(target_os = "macos"))]
    {
        Err("Not supported on this platform".into())
    }
}

#[allow(non_snake_case)]
#[tauri::command]
pub fn audio__set_output_device(device_id: u32) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        macos::set_output_device(device_id)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = device_id;
        Err("Not supported on this platform".into())
    }
}

#[allow(non_snake_case)]
#[tauri::command]
pub fn audio__get_device_volume(device_id: u32) -> Result<f32, String> {
    #[cfg(target_os = "macos")]
    {
        macos::get_device_volume_pub(device_id)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = device_id;
        Err("Not supported on this platform".into())
    }
}

#[allow(non_snake_case)]
#[tauri::command]
pub fn audio__set_device_volume(device_id: u32, volume: f32) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        macos::set_device_volume(device_id, volume)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = device_id;
        let _ = volume;
        Err("Not supported on this platform".into())
    }
}

pub fn start_device_listener(app_handle: AppHandle) {
    #[cfg(target_os = "macos")]
    {
        macos::start_listening(app_handle);
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = app_handle;
    }
}
