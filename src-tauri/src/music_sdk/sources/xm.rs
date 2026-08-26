use crate::music_sdk::client::{PlaylistResult, SearchResult};

// 虾米音乐已停止服务，仅保留空壳 stub

pub async fn handle(method: &str, args: serde_json::Value) -> Result<serde_json::Value, String> {
    let src = args.get("source").and_then(|v| v.as_str()).unwrap_or("xm").to_string();
    match method {
        "search" => Ok(serde_json::to_value(SearchResult {
            list: vec![], all_page: 0, limit: 30, total: 0, source: src,
        }).unwrap()),
        "tipSearch" | "hotSearch" => Ok(serde_json::json!({ "list": [] })),
        "getHotSonglist" | "getHotPlaylists" | "searchPlaylist" | "getCategoryPlaylists" =>
            Ok(serde_json::to_value(PlaylistResult {
                list: vec![], all_page: 0, limit: 30, total: 0, source: src,
            }).unwrap()),
        "getPlaylistTags" | "getSongboardTags" => Ok(serde_json::json!({ "list": [] })),
        "getPlaylistDetail" | "getPlaylistDetailById" | "getLeaderboardDetail" =>
            Ok(serde_json::json!({
                "list": [], "info": {}, "allPage": 0, "limit": 30, "total": 0, "source": src
            })),
        "getLeaderboards" => Ok(serde_json::json!({ "list": [], "source": src })),
        "getMusicUrl" => Ok(serde_json::json!({ "url": "" })),
        "getPic" => Ok(serde_json::json!({ "url": "" })),
        "getLyric" => Ok(serde_json::json!({ "lrc": "" })),
        "getHotComment" | "getComment" => Ok(serde_json::json!({ "comments": [], "total": 0 })),
        _ => Err(format!("Unknown SDK method for xm: {}", method)),
    }
}
