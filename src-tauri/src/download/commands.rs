#![allow(non_snake_case)]

use crate::download::manager::DownloadManager;
use serde_json::Value;
use tauri::State;

type ApiResult = Result<serde_json::Value, String>;

fn ok(data: Value) -> Result<Value, String> {
    Ok(serde_json::json!({ "success": true, "data": data }))
}

fn ok_null() -> Result<Value, String> {
    Ok(serde_json::json!({ "success": true, "data": null }))
}

#[tauri::command]
pub async fn download__add_task(
    dm: State<'_, DownloadManager>,
    song_info: Value,
    url: String,
    file_path: String,
    plugin_id: Option<String>,
    quality: Option<String>,
    priority: Option<u32>,
) -> ApiResult {
    let task = dm
        .add_task(
            song_info,
            url,
            file_path,
            plugin_id,
            quality,
            priority.unwrap_or(0),
        )
        .await?;
    ok(serde_json::to_value(task).unwrap_or_default())
}

#[tauri::command]
pub async fn download__get_tasks(dm: State<'_, DownloadManager>) -> ApiResult {
    let tasks = dm.get_tasks().await;
    ok(serde_json::to_value(tasks).unwrap_or_default())
}

#[tauri::command]
pub async fn download__pause_task(dm: State<'_, DownloadManager>, task_id: String) -> ApiResult {
    dm.pause_task(&task_id).await?;
    ok_null()
}

#[tauri::command]
pub async fn download__resume_task(dm: State<'_, DownloadManager>, task_id: String) -> ApiResult {
    dm.resume_task(&task_id).await?;
    ok_null()
}

#[tauri::command]
pub async fn download__cancel_task(dm: State<'_, DownloadManager>, task_id: String) -> ApiResult {
    dm.cancel_task(&task_id).await?;
    ok_null()
}

#[tauri::command]
pub async fn download__delete_task(
    dm: State<'_, DownloadManager>,
    task_id: String,
    delete_file: Option<bool>,
) -> ApiResult {
    dm.delete_task(&task_id, delete_file.unwrap_or(false))
        .await?;
    ok_null()
}

#[tauri::command]
pub async fn download__retry_task(dm: State<'_, DownloadManager>, task_id: String) -> ApiResult {
    dm.retry_task(&task_id).await?;
    ok_null()
}

#[tauri::command]
pub async fn download__pause_all_tasks(dm: State<'_, DownloadManager>) -> ApiResult {
    dm.pause_all_tasks().await;
    ok_null()
}

#[tauri::command]
pub async fn download__resume_all_tasks(dm: State<'_, DownloadManager>) -> ApiResult {
    dm.resume_all_tasks().await;
    ok_null()
}

#[tauri::command]
pub async fn download__set_max_concurrent(dm: State<'_, DownloadManager>, max: usize) -> ApiResult {
    dm.set_max_concurrent(max).await;
    ok_null()
}

#[tauri::command]
pub async fn download__get_max_concurrent(dm: State<'_, DownloadManager>) -> ApiResult {
    let max = dm.get_max_concurrent().await;
    ok(serde_json::json!(max))
}

#[tauri::command]
pub async fn download__clear_tasks(dm: State<'_, DownloadManager>, task_type: String) -> ApiResult {
    dm.clear_tasks(&task_type).await;
    ok_null()
}

#[tauri::command]
pub async fn download__validate_files(dm: State<'_, DownloadManager>) -> ApiResult {
    dm.validate_files().await;
    let tasks = dm.get_tasks().await;
    ok(serde_json::to_value(tasks).unwrap_or_default())
}

#[tauri::command]
pub async fn download__open_file_location(dm: State<'_, DownloadManager>, file_path: String) -> ApiResult {
    dm.open_file_location(&file_path).await?;
    ok_null()
}
