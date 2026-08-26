#![allow(non_snake_case)]

use crate::commands::DbState;
use crate::db::playlist_db;

#[derive(serde::Serialize)]
struct ApiResponse<T: serde::Serialize> {
    success: bool,
    data: Option<T>,
    error: Option<String>,
}

impl<T: serde::Serialize> ApiResponse<T> {
    fn ok(data: T) -> Self {
        Self { success: true, data: Some(data), error: None }
    }
    fn err(msg: impl ToString) -> Self {
        Self { success: false, data: None, error: Some(msg.to_string()) }
    }
}

#[tauri::command]
pub fn config__get(state: DbState<'_>, key: String) -> Result<serde_json::Value, String> {
    let conn = state.playlist.lock().map_err(|e| e.to_string())?;
    playlist_db::ensure_kv_table(&conn).map_err(|e| e.to_string())?;
    match playlist_db::kv_get(&conn, &key) {
        Ok(v) => Ok(serde_json::to_value(ApiResponse::ok(v)).unwrap()),
        Err(e) => Ok(serde_json::to_value(ApiResponse::<Option<String>>::err(e.to_string())).unwrap()),
    }
}

#[tauri::command]
pub fn config__set(state: DbState<'_>, key: String, value: String) -> Result<serde_json::Value, String> {
    let conn = state.playlist.lock().map_err(|e| e.to_string())?;
    playlist_db::ensure_kv_table(&conn).map_err(|e| e.to_string())?;
    match playlist_db::kv_set(&conn, &key, &value) {
        Ok(_) => Ok(serde_json::to_value(ApiResponse::ok(true)).unwrap()),
        Err(e) => Ok(serde_json::to_value(ApiResponse::<bool>::err(e.to_string())).unwrap()),
    }
}

#[tauri::command]
pub fn config__get_all(state: DbState<'_>) -> Result<serde_json::Value, String> {
    let conn = state.playlist.lock().map_err(|e| e.to_string())?;
    playlist_db::ensure_kv_table(&conn).map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare("SELECT key, value FROM kv_store").map_err(|e| e.to_string())?;
    let rows: std::collections::HashMap<String, String> = stmt
        .query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)))
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();
    Ok(serde_json::to_value(ApiResponse::ok(rows)).unwrap())
}

#[tauri::command]
pub fn config__delete(state: DbState<'_>, key: String) -> Result<serde_json::Value, String> {
    let conn = state.playlist.lock().map_err(|e| e.to_string())?;
    match conn.execute("DELETE FROM kv_store WHERE key = ?1", [&key]) {
        Ok(n) => Ok(serde_json::to_value(ApiResponse::ok(n)).unwrap()),
        Err(e) => Ok(serde_json::to_value(ApiResponse::<usize>::err(e.to_string())).unwrap()),
    }
}
