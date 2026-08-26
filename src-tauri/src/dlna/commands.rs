#![allow(non_snake_case)]

use crate::dlna::DlnaManager;
use serde_json::{json, Value};
use tauri::State;

type ApiResult = Result<Value, String>;

fn ok(data: Value) -> ApiResult {
    Ok(json!({ "success": true, "data": data }))
}

fn err(error: String) -> ApiResult {
    Ok(json!({ "success": false, "error": error }))
}

#[tauri::command]
pub async fn dlna__start_search(manager: State<'_, DlnaManager>) -> ApiResult {
    match manager.start_search().await {
        Ok(devices) => ok(json!(devices)),
        Err(error) => err(error),
    }
}

#[tauri::command]
pub async fn dlna__stop_search(manager: State<'_, DlnaManager>) -> ApiResult {
    manager.stop_search();
    ok(Value::Null)
}

#[tauri::command]
pub async fn dlna__get_devices(manager: State<'_, DlnaManager>) -> ApiResult {
    ok(json!(manager.devices().await))
}

#[tauri::command]
pub async fn dlna__play(
    manager: State<'_, DlnaManager>,
    url: String,
    location: String,
    title: String,
) -> ApiResult {
    match manager.play(&location, &url, &title).await {
        Ok(()) => ok(Value::Null),
        Err(error) => err(error),
    }
}

#[tauri::command]
pub async fn dlna__pause(manager: State<'_, DlnaManager>) -> ApiResult {
    control_result(manager.pause().await)
}

#[tauri::command]
pub async fn dlna__resume(manager: State<'_, DlnaManager>) -> ApiResult {
    control_result(manager.resume().await)
}

#[tauri::command]
pub async fn dlna__stop(manager: State<'_, DlnaManager>) -> ApiResult {
    control_result(manager.stop().await)
}

#[tauri::command]
pub async fn dlna__seek(manager: State<'_, DlnaManager>, seconds: f64) -> ApiResult {
    control_result(manager.seek(seconds).await)
}

#[tauri::command]
pub async fn dlna__set_volume(manager: State<'_, DlnaManager>, volume: f64) -> ApiResult {
    control_result(manager.set_volume(volume).await)
}

#[tauri::command]
pub async fn dlna__get_position(manager: State<'_, DlnaManager>) -> ApiResult {
    match manager.position().await {
        Ok(position) => ok(json!(position)),
        Err(error) => err(error),
    }
}

fn control_result(result: Result<(), String>) -> ApiResult {
    match result {
        Ok(()) => ok(Value::Null),
        Err(error) => err(error),
    }
}

#[cfg(test)]
mod tests {
    use crate::dlna::DlnaManager;

    #[tokio::test]
    async fn rejects_control_of_an_uncached_location() {
        let result = DlnaManager::new()
            .play(
                "http://192.168.1.20/untrusted-control.xml",
                "https://media.example/song.mp3",
                "Song",
            )
            .await;
        assert!(result.is_err());
    }
}
