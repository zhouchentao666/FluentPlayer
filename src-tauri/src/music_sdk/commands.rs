#![allow(non_snake_case)]

use crate::music_sdk::client;

#[tauri::command]
pub async fn service_music_sdk_request(
    method: String,
    args: serde_json::Value,
) -> Result<serde_json::Value, String> {
    client::handle_request(&method, args).await
}

#[tauri::command]
pub async fn service_music_tip_search(
    source: String,
    keyword: String,
) -> Result<serde_json::Value, String> {
    client::handle_request("tipSearch", serde_json::json!({ "source": source, "keyword": keyword })).await
}

#[tauri::command]
pub async fn service_music_search_music(
    name: String,
    singer: String,
    source: String,
    limit: Option<u64>,
) -> Result<serde_json::Value, String> {
    client::search_music(serde_json::json!({
        "name": name, "singer": singer, "source": source,
        "limit": limit.unwrap_or(25)
    })).await
}

#[tauri::command]
pub async fn service_music_find_music(
    name: String,
    singer: String,
    album_name: Option<String>,
    interval: Option<String>,
    source: String,
) -> Result<serde_json::Value, String> {
    client::find_music(serde_json::json!({
        "name": name, "singer": singer,
        "albumName": album_name.unwrap_or_default(),
        "interval": interval.unwrap_or_default(),
        "source": source
    })).await
}
