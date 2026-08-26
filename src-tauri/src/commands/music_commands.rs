#![allow(non_snake_case)]

use crate::commands::DbState;
use crate::db::music_db;
use serde::Serialize;

#[derive(Serialize)]
struct ApiResponse<T: Serialize> {
    success: bool,
    data: Option<T>,
    error: Option<String>,
}

impl<T: Serialize> ApiResponse<T> {
    fn ok(data: T) -> Self {
        Self { success: true, data: Some(data), error: None }
    }
    fn err(msg: impl ToString) -> Self {
        Self { success: false, data: None, error: Some(msg.to_string()) }
    }
}

#[tauri::command]
pub fn track__get_all(state: DbState<'_>) -> Result<serde_json::Value, String> {
    let conn = state.music.lock().map_err(|e| e.to_string())?;
    match music_db::get_all_tracks(&conn) {
        Ok(tracks) => Ok(serde_json::to_value(ApiResponse::ok(tracks)).unwrap()),
        Err(e) => Ok(serde_json::to_value(ApiResponse::<Vec<music_db::TrackRow>>::err(e.to_string())).unwrap()),
    }
}

#[tauri::command]
pub fn track__get_by_id(state: DbState<'_>, songmid: String) -> Result<serde_json::Value, String> {
    let conn = state.music.lock().map_err(|e| e.to_string())?;
    match music_db::get_track_by_id(&conn, &songmid) {
        Ok(track) => Ok(serde_json::to_value(ApiResponse::ok(track)).unwrap()),
        Err(e) => Ok(serde_json::to_value(ApiResponse::<Option<music_db::TrackRow>>::err(e.to_string())).unwrap()),
    }
}

#[tauri::command]
pub fn track__get_by_path(state: DbState<'_>, path: String) -> Result<serde_json::Value, String> {
    let conn = state.music.lock().map_err(|e| e.to_string())?;
    match music_db::get_track_by_path(&conn, &path) {
        Ok(track) => Ok(serde_json::to_value(ApiResponse::ok(track)).unwrap()),
        Err(e) => Ok(serde_json::to_value(ApiResponse::<Option<music_db::TrackRow>>::err(e.to_string())).unwrap()),
    }
}

#[tauri::command]
pub fn track__upsert(state: DbState<'_>, track: music_db::TrackRow) -> Result<serde_json::Value, String> {
    let conn = state.music.lock().map_err(|e| e.to_string())?;
    match music_db::upsert_track(&conn, &track) {
        Ok(_) => Ok(serde_json::to_value(ApiResponse::ok(true)).unwrap()),
        Err(e) => Ok(serde_json::to_value(ApiResponse::<bool>::err(e.to_string())).unwrap()),
    }
}

#[tauri::command]
pub fn track__upsert_batch(state: DbState<'_>, tracks: Vec<music_db::TrackRow>) -> Result<serde_json::Value, String> {
    let conn = state.music.lock().map_err(|e| e.to_string())?;
    match music_db::upsert_tracks(&conn, &tracks) {
        Ok(_) => Ok(serde_json::to_value(ApiResponse::ok(tracks.len())).unwrap()),
        Err(e) => Ok(serde_json::to_value(ApiResponse::<usize>::err(e.to_string())).unwrap()),
    }
}

#[tauri::command]
pub fn track__delete_by_path(state: DbState<'_>, path: String) -> Result<serde_json::Value, String> {
    let conn = state.music.lock().map_err(|e| e.to_string())?;
    match music_db::delete_by_path(&conn, &path) {
        Ok(n) => Ok(serde_json::to_value(ApiResponse::ok(n)).unwrap()),
        Err(e) => Ok(serde_json::to_value(ApiResponse::<usize>::err(e.to_string())).unwrap()),
    }
}

#[tauri::command]
pub fn track__delete_by_songmid(state: DbState<'_>, songmid: String) -> Result<serde_json::Value, String> {
    let conn = state.music.lock().map_err(|e| e.to_string())?;
    match music_db::delete_by_songmid(&conn, &songmid) {
        Ok(n) => Ok(serde_json::to_value(ApiResponse::ok(n)).unwrap()),
        Err(e) => Ok(serde_json::to_value(ApiResponse::<usize>::err(e.to_string())).unwrap()),
    }
}

#[tauri::command]
pub fn track__clear(state: DbState<'_>) -> Result<serde_json::Value, String> {
    let conn = state.music.lock().map_err(|e| e.to_string())?;
    match music_db::clear_tracks(&conn) {
        Ok(n) => Ok(serde_json::to_value(ApiResponse::ok(n)).unwrap()),
        Err(e) => Ok(serde_json::to_value(ApiResponse::<usize>::err(e.to_string())).unwrap()),
    }
}

#[tauri::command]
pub fn track__get_stat(state: DbState<'_>, path: String) -> Result<serde_json::Value, String> {
    let conn = state.music.lock().map_err(|e| e.to_string())?;
    match music_db::get_stat_by_path(&conn, &path) {
        Ok(stat) => Ok(serde_json::to_value(ApiResponse::ok(stat)).unwrap()),
        Err(e) => Ok(serde_json::to_value(ApiResponse::<Option<music_db::TrackStat>>::err(e.to_string())).unwrap()),
    }
}

#[tauri::command]
pub fn track__get_all_stats(state: DbState<'_>) -> Result<serde_json::Value, String> {
    let conn = state.music.lock().map_err(|e| e.to_string())?;
    match music_db::get_all_stats(&conn) {
        Ok(stats) => Ok(serde_json::to_value(ApiResponse::ok(stats)).unwrap()),
        Err(e) => Ok(serde_json::to_value(ApiResponse::<Vec<music_db::TrackStat>>::err(e.to_string())).unwrap()),
    }
}

#[tauri::command]
pub fn track__prune_outside(state: DbState<'_>, keep_paths: Vec<String>) -> Result<serde_json::Value, String> {
    let conn = state.music.lock().map_err(|e| e.to_string())?;
    match music_db::prune_outside_keep(&conn, &keep_paths) {
        Ok(n) => Ok(serde_json::to_value(ApiResponse::ok(n)).unwrap()),
        Err(e) => Ok(serde_json::to_value(ApiResponse::<usize>::err(e.to_string())).unwrap()),
    }
}

#[tauri::command]
pub fn dir__get_all(state: DbState<'_>) -> Result<serde_json::Value, String> {
    let conn = state.music.lock().map_err(|e| e.to_string())?;
    match music_db::get_dirs(&conn) {
        Ok(dirs) => Ok(serde_json::to_value(ApiResponse::ok(dirs)).unwrap()),
        Err(e) => Ok(serde_json::to_value(ApiResponse::<Vec<String>>::err(e.to_string())).unwrap()),
    }
}

#[tauri::command]
pub fn dir__set(state: DbState<'_>, dirs: Vec<String>) -> Result<serde_json::Value, String> {
    let conn = state.music.lock().map_err(|e| e.to_string())?;
    match music_db::set_dirs(&conn, &dirs) {
        Ok(_) => Ok(serde_json::to_value(ApiResponse::ok(true)).unwrap()),
        Err(e) => Ok(serde_json::to_value(ApiResponse::<bool>::err(e.to_string())).unwrap()),
    }
}
