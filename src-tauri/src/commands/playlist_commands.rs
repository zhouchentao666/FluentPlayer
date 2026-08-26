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

// --- Playlist commands (songlist__ prefix) ---

#[tauri::command]
pub fn songlist__get_all(state: DbState<'_>) -> Result<serde_json::Value, String> {
    let conn = state.playlist.lock().map_err(|e| e.to_string())?;
    match playlist_db::list_playlists(&conn) {
        Ok(list) => Ok(serde_json::to_value(ApiResponse::ok(list)).unwrap()),
        Err(e) => Ok(serde_json::to_value(ApiResponse::<Vec<playlist_db::PlaylistRow>>::err(e.to_string())).unwrap()),
    }
}

#[tauri::command]
pub fn songlist__get(state: DbState<'_>, id: String) -> Result<serde_json::Value, String> {
    let conn = state.playlist.lock().map_err(|e| e.to_string())?;
    match playlist_db::get_playlist(&conn, &id) {
        Ok(p) => Ok(serde_json::to_value(ApiResponse::ok(p)).unwrap()),
        Err(e) => Ok(serde_json::to_value(ApiResponse::<Option<playlist_db::PlaylistRow>>::err(e.to_string())).unwrap()),
    }
}

#[tauri::command]
pub fn songlist__create(state: DbState<'_>, name: String, description: Option<String>, source: Option<String>, meta: Option<serde_json::Value>) -> Result<serde_json::Value, String> {
    let conn = state.playlist.lock().map_err(|e| e.to_string())?;
    let now = chrono::Utc::now().to_rfc3339();
    let id = format!("pl_{}", chrono::Utc::now().timestamp_millis());
    let p = playlist_db::PlaylistRow {
        id: id.clone(),
        name,
        description: description.unwrap_or_default(),
        cover_img_url: "default-cover".to_string(),
        source: source.unwrap_or_else(|| "local".to_string()),
        meta: meta.map(|m| m.to_string()).unwrap_or_else(|| "{}".to_string()),
        create_time: now.clone(),
        update_time: now,
        song_count: 0,
    };
    match playlist_db::insert_playlist(&conn, &p) {
        Ok(_) => Ok(serde_json::to_value(ApiResponse::ok(p)).unwrap()),
        Err(e) => Ok(serde_json::to_value(ApiResponse::<playlist_db::PlaylistRow>::err(e.to_string())).unwrap()),
    }
}

#[tauri::command]
pub fn songlist__delete(state: DbState<'_>, id: String) -> Result<serde_json::Value, String> {
    let conn = state.playlist.lock().map_err(|e| e.to_string())?;
    match playlist_db::delete_playlist(&conn, &id) {
        Ok(n) => Ok(serde_json::to_value(ApiResponse::ok(n)).unwrap()),
        Err(e) => Ok(serde_json::to_value(ApiResponse::<usize>::err(e.to_string())).unwrap()),
    }
}

#[tauri::command]
pub fn songlist__update(state: DbState<'_>, id: String, name: String, description: String) -> Result<serde_json::Value, String> {
    let conn = state.playlist.lock().map_err(|e| e.to_string())?;
    match playlist_db::update_playlist(&conn, &id, &name, &description) {
        Ok(n) => Ok(serde_json::to_value(ApiResponse::ok(n)).unwrap()),
        Err(e) => Ok(serde_json::to_value(ApiResponse::<usize>::err(e.to_string())).unwrap()),
    }
}

#[tauri::command]
pub fn songlist__update_cover(state: DbState<'_>, id: String, cover_url: String) -> Result<serde_json::Value, String> {
    let conn = state.playlist.lock().map_err(|e| e.to_string())?;
    match playlist_db::update_cover(&conn, &id, &cover_url) {
        Ok(n) => Ok(serde_json::to_value(ApiResponse::ok(n)).unwrap()),
        Err(e) => Ok(serde_json::to_value(ApiResponse::<usize>::err(e.to_string())).unwrap()),
    }
}

#[tauri::command]
pub fn songlist__search(state: DbState<'_>, keyword: String, source: Option<String>) -> Result<serde_json::Value, String> {
    let conn = state.playlist.lock().map_err(|e| e.to_string())?;
    match playlist_db::search_playlists(&conn, &keyword, source.as_deref()) {
        Ok(list) => Ok(serde_json::to_value(ApiResponse::ok(list)).unwrap()),
        Err(e) => Ok(serde_json::to_value(ApiResponse::<Vec<playlist_db::PlaylistRow>>::err(e.to_string())).unwrap()),
    }
}

#[tauri::command]
pub fn songlist__exists(state: DbState<'_>, id: String) -> Result<serde_json::Value, String> {
    let conn = state.playlist.lock().map_err(|e| e.to_string())?;
    match playlist_db::playlist_exists(&conn, &id) {
        Ok(b) => Ok(serde_json::to_value(ApiResponse::ok(b)).unwrap()),
        Err(e) => Ok(serde_json::to_value(ApiResponse::<bool>::err(e.to_string())).unwrap()),
    }
}

// --- Playlist Song commands ---

#[tauri::command]
pub fn songlist__list_songs(state: DbState<'_>, playlist_id: String) -> Result<serde_json::Value, String> {
    let conn = state.playlist.lock().map_err(|e| e.to_string())?;
    match playlist_db::list_songs(&conn, &playlist_id) {
        Ok(songs) => Ok(serde_json::to_value(ApiResponse::ok(songs)).unwrap()),
        Err(e) => Ok(serde_json::to_value(ApiResponse::<Vec<playlist_db::PlaylistSongRow>>::err(e.to_string())).unwrap()),
    }
}

#[tauri::command]
pub fn songlist__count_songs(state: DbState<'_>, playlist_id: String) -> Result<serde_json::Value, String> {
    let conn = state.playlist.lock().map_err(|e| e.to_string())?;
    match playlist_db::count_songs(&conn, &playlist_id) {
        Ok(n) => Ok(serde_json::to_value(ApiResponse::ok(n)).unwrap()),
        Err(e) => Ok(serde_json::to_value(ApiResponse::<i64>::err(e.to_string())).unwrap()),
    }
}

#[tauri::command]
pub fn songlist__has_song(state: DbState<'_>, playlist_id: String, songmid: String) -> Result<serde_json::Value, String> {
    let conn = state.playlist.lock().map_err(|e| e.to_string())?;
    match playlist_db::has_song(&conn, &playlist_id, &songmid) {
        Ok(b) => Ok(serde_json::to_value(ApiResponse::ok(b)).unwrap()),
        Err(e) => Ok(serde_json::to_value(ApiResponse::<bool>::err(e.to_string())).unwrap()),
    }
}

#[tauri::command]
pub fn songlist__add_songs(state: DbState<'_>, playlist_id: String, songs: Vec<playlist_db::PlaylistSongRow>) -> Result<serde_json::Value, String> {
    let conn = state.playlist.lock().map_err(|e| e.to_string())?;
    match playlist_db::add_songs(&conn, &playlist_id, &songs) {
        Ok(_) => Ok(serde_json::to_value(ApiResponse::ok(songs.len())).unwrap()),
        Err(e) => Ok(serde_json::to_value(ApiResponse::<usize>::err(e.to_string())).unwrap()),
    }
}

#[tauri::command]
pub fn songlist__add_head(state: DbState<'_>, playlist_id: String, songs: Vec<playlist_db::PlaylistSongRow>) -> Result<serde_json::Value, String> {
    let conn = state.playlist.lock().map_err(|e| e.to_string())?;
    match playlist_db::add_songs_head(&conn, &playlist_id, &songs) {
        Ok(_) => Ok(serde_json::to_value(ApiResponse::ok(songs.len())).unwrap()),
        Err(e) => Ok(serde_json::to_value(ApiResponse::<usize>::err(e.to_string())).unwrap()),
    }
}

#[tauri::command]
pub fn songlist__remove_song(state: DbState<'_>, playlist_id: String, songmid: String) -> Result<serde_json::Value, String> {
    let conn = state.playlist.lock().map_err(|e| e.to_string())?;
    match playlist_db::remove_song(&conn, &playlist_id, &songmid) {
        Ok(n) => Ok(serde_json::to_value(ApiResponse::ok(n)).unwrap()),
        Err(e) => Ok(serde_json::to_value(ApiResponse::<usize>::err(e.to_string())).unwrap()),
    }
}

#[tauri::command]
pub fn songlist__remove_batch(state: DbState<'_>, playlist_id: String, songmids: Vec<String>) -> Result<serde_json::Value, String> {
    let conn = state.playlist.lock().map_err(|e| e.to_string())?;
    match playlist_db::remove_songs(&conn, &playlist_id, &songmids) {
        Ok(n) => Ok(serde_json::to_value(ApiResponse::ok(n)).unwrap()),
        Err(e) => Ok(serde_json::to_value(ApiResponse::<usize>::err(e.to_string())).unwrap()),
    }
}

#[tauri::command]
pub fn songlist__clear_songs(state: DbState<'_>, playlist_id: String) -> Result<serde_json::Value, String> {
    let conn = state.playlist.lock().map_err(|e| e.to_string())?;
    match playlist_db::clear_songs(&conn, &playlist_id) {
        Ok(n) => Ok(serde_json::to_value(ApiResponse::ok(n)).unwrap()),
        Err(e) => Ok(serde_json::to_value(ApiResponse::<usize>::err(e.to_string())).unwrap()),
    }
}

#[tauri::command]
pub fn songlist__search_songs(state: DbState<'_>, playlist_id: String, keyword: String) -> Result<serde_json::Value, String> {
    let conn = state.playlist.lock().map_err(|e| e.to_string())?;
    match playlist_db::search_songs(&conn, &playlist_id, &keyword) {
        Ok(songs) => Ok(serde_json::to_value(ApiResponse::ok(songs)).unwrap()),
        Err(e) => Ok(serde_json::to_value(ApiResponse::<Vec<playlist_db::PlaylistSongRow>>::err(e.to_string())).unwrap()),
    }
}

// --- Batch delete & reorder ---

#[tauri::command]
pub fn songlist__batch_delete(state: DbState<'_>, ids: Vec<String>) -> Result<serde_json::Value, String> {
    let conn = state.playlist.lock().map_err(|e| e.to_string())?;
    match playlist_db::batch_delete_playlists(&conn, &ids) {
        Ok(n) => Ok(serde_json::to_value(ApiResponse::ok(n)).unwrap()),
        Err(e) => Ok(serde_json::to_value(ApiResponse::<usize>::err(e.to_string())).unwrap()),
    }
}

#[tauri::command]
pub fn songlist__move_song(state: DbState<'_>, playlist_id: String, songmid: String, to_index: i64) -> Result<serde_json::Value, String> {
    let conn = state.playlist.lock().map_err(|e| e.to_string())?;
    match playlist_db::move_song(&conn, &playlist_id, &songmid, to_index) {
        Ok(_) => Ok(serde_json::to_value(ApiResponse::ok(true)).unwrap()),
        Err(e) => Ok(serde_json::to_value(ApiResponse::<bool>::err(e.to_string())).unwrap()),
    }
}

// --- Favorites KV ---

#[tauri::command]
pub fn songlist__get_favorites_id(state: DbState<'_>) -> Result<serde_json::Value, String> {
    let conn = state.playlist.lock().map_err(|e| e.to_string())?;
    playlist_db::ensure_kv_table(&conn).map_err(|e| e.to_string())?;
    match playlist_db::kv_get(&conn, "favoritesId") {
        Ok(v) => Ok(serde_json::to_value(ApiResponse::ok(v)).unwrap()),
        Err(e) => Ok(serde_json::to_value(ApiResponse::<Option<String>>::err(e.to_string())).unwrap()),
    }
}

#[tauri::command]
pub fn songlist__set_favorites_id(state: DbState<'_>, id: String) -> Result<serde_json::Value, String> {
    let conn = state.playlist.lock().map_err(|e| e.to_string())?;
    playlist_db::ensure_kv_table(&conn).map_err(|e| e.to_string())?;
    match playlist_db::kv_set(&conn, "favoritesId", &id) {
        Ok(_) => Ok(serde_json::to_value(ApiResponse::ok(true)).unwrap()),
        Err(e) => Ok(serde_json::to_value(ApiResponse::<bool>::err(e.to_string())).unwrap()),
    }
}
