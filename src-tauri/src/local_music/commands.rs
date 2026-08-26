#![allow(non_snake_case)]

use crate::db::music_db;
use crate::local_music::{scanner, cover_cache};
use crate::AppDb;
use tauri::State;
#[cfg(desktop)]
use tauri_plugin_dialog::DialogExt;

#[tauri::command]
pub fn local_music__scan(state: State<'_, AppDb>, dirs: Vec<String>, skip_hidden: Option<bool>) -> Result<serde_json::Value, String> {
    let conn = state.music.lock().map_err(|e| e.to_string())?;
    let result = scanner::scan_directories(&conn, &dirs, skip_hidden.unwrap_or(false));
    Ok(serde_json::to_value(serde_json::json!({
        "success": true,
        "data": { "scanned": result.scanned, "added": result.added, "updated": result.updated, "errors": result.errors }
    })).unwrap())
}

#[tauri::command]
pub fn local_music__get_cover(state: State<'_, AppDb>, track_id: String) -> Result<serde_json::Value, String> {
    let conn = state.music.lock().map_err(|e| e.to_string())?;
    let track = music_db::get_track_by_id(&conn, &track_id)
        .map_err(|e| e.to_string())?;
    match track {
        Some(t) => {
            let cover = cover_cache::get_cover_base64(&t.songmid, &t.path);
            Ok(serde_json::json!({ "success": true, "data": cover }))
        }
        None => Ok(serde_json::json!({ "success": true, "data": null })),
    }
}

#[tauri::command]
pub fn local_music__get_covers(state: State<'_, AppDb>, track_ids: Vec<String>) -> Result<serde_json::Value, String> {
    let conn = state.music.lock().map_err(|e| e.to_string())?;
    let track_paths = music_db::get_track_paths_by_ids(&conn, &track_ids).map_err(|e| e.to_string())?;
    let covers = cover_cache::batch_get_covers(&track_ids, &track_paths);
    Ok(serde_json::json!({ "success": true, "data": covers }))
}

fn normalize_cover_url(url: &str) -> String {
    if let Some(encoded) = url.strip_prefix("imgproxy://localhost/") {
        match urlencoding::decode(encoded) {
            Ok(decoded) => decoded.into_owned(),
            Err(_) => url.to_string(),
        }
    } else {
        url.to_string()
    }
}

#[tauri::command]
pub async fn local_music__write_tags(
    state: State<'_, AppDb>,
    file_path: String,
    song_info: serde_json::Value,
    tag_write_options: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let path = std::path::Path::new(&file_path);

    let actual_path = path.to_path_buf();

    let has_options = tag_write_options.as_object().map(|o| !o.is_empty()).unwrap_or(false);
    if has_options {
        // Fetch cover if needed
        let cover_data = if tag_write_options.get("cover").and_then(|v| v.as_bool()).unwrap_or(false) {
            let raw_img_url = song_info.get("img").and_then(|v| v.as_str()).unwrap_or("");
            let img_url = normalize_cover_url(raw_img_url);
            if !img_url.is_empty() {
                let resp = crate::music_sdk::client::get_client()
                    .get(&img_url)
                    .send().await;
                match resp {
                    Ok(r) => r.bytes().await.ok().map(|b| b.to_vec()),
                    Err(_) => None,
                }
            } else {
                None
            }
        } else {
            None
        };

        let lyrics = if tag_write_options.get("lyrics").and_then(|v| v.as_bool()).unwrap_or(false)
            || tag_write_options.get("downloadLyrics").and_then(|v| v.as_bool()).unwrap_or(false)
        {
            song_info.get("lrc").and_then(|v| v.as_str()).filter(|s| !s.is_empty()).map(|s| s.to_string())
        } else {
            None
        };
        scanner::write_download_tags(
            &actual_path,
            &song_info,
            &tag_write_options,
            cover_data.as_deref(),
            lyrics.as_deref(),
        )?;
    } else {
        let track = music_db::TrackRow {
            songmid: String::new(),
            path: actual_path.to_string_lossy().to_string(),
            url: None,
            singer: song_info["singer"].as_str().unwrap_or("").to_string(),
            name: song_info["name"].as_str().unwrap_or("").to_string(),
            album_name: song_info["albumName"].as_str().unwrap_or("").to_string(),
            album_id: 0,
            source: "local".to_string(),
            interval: String::new(),
            has_cover: 0,
            cover_key: None,
            year: song_info["year"].as_i64().unwrap_or(0),
            lrc: None,
            types: "[]".to_string(),
            _types: "{}".to_string(),
            type_url: "{}".to_string(),
            bitrate: 0,
            sample_rate: 0,
            channels: 0,
            duration: 0.0,
            size: 0,
            mtime_ms: 0,
            hash: None,
            updated_at: 0,
        };
        scanner::write_tags(&actual_path, &track)?;
    }

    if let Some(track) = scanner::read_file_tags(&actual_path) {
        let conn = state.music.lock().map_err(|e| e.to_string())?;
        music_db::upsert_track(&conn, &track).map_err(|e| e.to_string())?;
    }

    Ok(serde_json::json!({ "success": true, "data": { "filePath": actual_path.to_string_lossy().to_string() } }))
}

#[tauri::command]
pub fn local_music__get_tags(state: State<'_, AppDb>, songmid: String, include_lyrics: Option<bool>) -> Result<serde_json::Value, String> {
    let conn = state.music.lock().map_err(|e| e.to_string())?;
    let track = music_db::get_track_by_id(&conn, &songmid)
        .map_err(|e| e.to_string())?;
    match track {
        Some(t) => {
            let mut data = serde_json::to_value(&t).unwrap();
            if !include_lyrics.unwrap_or(true) {
                data["lrc"] = serde_json::Value::Null;
            }
            Ok(serde_json::json!({ "success": true, "data": data }))
        }
        None => Ok(serde_json::json!({ "success": true, "data": null })),
    }
}

#[tauri::command]
pub fn local_music__get_lyric(state: State<'_, AppDb>, songmid: String) -> Result<serde_json::Value, String> {
    let (stored_lrc, file_path) = {
        let conn = state.music.lock().map_err(|e| e.to_string())?;
        let track = music_db::get_track_by_id(&conn, &songmid)
            .map_err(|e| e.to_string())?;
        match track {
            Some(t) => (t.lrc, t.path),
            None => return Ok(serde_json::json!({ "success": true, "data": "" })),
        }
    };

    if let Some(lrc) = stored_lrc {
        if !lrc.is_empty() {
            return Ok(serde_json::json!({ "success": true, "data": lrc }));
        }
    }

    let lrc = scanner::read_lyrics_from_file(std::path::Path::new(&file_path)).unwrap_or_default();
    Ok(serde_json::json!({ "success": true, "data": lrc }))
}

#[tauri::command]
pub fn local_music__clear_index(state: State<'_, AppDb>) -> Result<serde_json::Value, String> {
    let conn = state.music.lock().map_err(|e| e.to_string())?;
    let n = music_db::clear_tracks(&conn).map_err(|e| e.to_string())?;
    cover_cache::clear_cache();
    Ok(serde_json::json!({ "success": true, "data": n }))
}

#[tauri::command]
pub async fn local_music__select_dirs(app: tauri::AppHandle) -> Result<serde_json::Value, String> {
    #[cfg(desktop)]
    {
        let dirs = app.dialog()
            .file()
            .set_title("选择音乐文件夹")
            .blocking_pick_folders();
        match dirs {
            Some(paths) => {
                let dir_strs: Vec<String> = paths.iter()
                    .map(|p| p.to_string())
                    .collect();
                Ok(serde_json::json!({ "success": true, "data": dir_strs }))
            }
            None => Ok(serde_json::json!({ "success": true, "data": [] })),
        }
    }
    #[cfg(not(desktop))]
    {
        let _ = app;
        Ok(serde_json::json!({ "success": true, "data": [] }))
    }
}
