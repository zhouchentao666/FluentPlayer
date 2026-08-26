use crate::music_sdk::client::{self, MusicItem, PlaylistItem, PlaylistResult, SearchResult};
use crate::music_sdk::client::ResponseExt;

fn get_http() -> &'static reqwest::Client {
    client::get_client()
}

fn get_str<'a>(args: &'a serde_json::Value, key: &str) -> &'a str {
    args.get(key).and_then(|v| v.as_str()).unwrap_or("")
}

fn get_u64(args: &serde_json::Value, key: &str, default: u64) -> u64 {
    args.get(key).and_then(|v| v.as_u64()).unwrap_or(default)
}

fn gen_req_id() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    (0..8).map(|_| format!("{:04x}", rng.gen::<u32>() % 65536)).collect::<Vec<_>>().join("")
}

fn format_play_time(seconds: i64) -> String {
    let m = seconds / 60;
    let s = seconds % 60;
    format!("{:02}:{:02}", m, s)
}

// --- Playlist Detail ---

async fn get_playlist_detail(args: serde_json::Value) -> Result<serde_json::Value, String> {
    let raw_id = args.get("id").map(|v| match v {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Number(n) => n.to_string(),
        _ => String::new(),
    }).unwrap_or_default();
    let page = get_u64(&args, "page", 1);
    let limit = 100;

    if raw_id.is_empty() {
        return Ok(serde_json::json!({
            "list": [], "info": {}, "allPage": 0, "limit": limit as i64, "total": 0, "source": "bd"
        }));
    }

    let req_id = gen_req_id();

    // Fetch playlist info
    let info_url = format!(
        "https://bd-api.kuwo.cn/api/service/playlist/info/{}?reqId={}&source=5",
        raw_id, req_id
    );
    let info_resp: serde_json::Value = get_http().get(&info_url)
        .header("User-Agent", "Mozilla/5.0 (iPhone; CPU iPhone OS 18_5 like Mac OS X) AppleWebKit/605.1.15")
        .header("plat", "h5")
        .send().await.map_err(|e| e.to_string())?
        .json_sanitized().await?;

    let code = info_resp.get("code").and_then(|v| v.as_i64()).unwrap_or(0);
    if code != 200 {
        return Err("BD playlist info API error".into());
    }

    let info_data = info_resp.get("data").cloned().unwrap_or(serde_json::json!({}));
    let name = info_data.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let img = info_data.get("pic").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let desc = info_data.get("description").and_then(|v| v.as_str()).unwrap_or("").to_string();

    // Fetch playlist songs
    let list_url = format!(
        "https://bd-api.kuwo.cn/api/service/playlist/{}/musicList?reqId={}&source=5&pn={}&rn=100",
        raw_id, req_id, page
    );
    let list_resp: serde_json::Value = get_http().get(&list_url)
        .header("User-Agent", "Mozilla/5.0 (iPhone; CPU iPhone OS 18_5 like Mac OS X) AppleWebKit/605.1.15")
        .header("plat", "h5")
        .send().await.map_err(|e| e.to_string())?
        .json_sanitized().await?;

    let code = list_resp.get("code").and_then(|v| v.as_i64()).unwrap_or(0);
    if code != 200 {
        return Err("BD playlist list API error".into());
    }

    let list_data = list_resp.get("data").cloned().unwrap_or(serde_json::json!({}));
    let total = list_data.get("total").and_then(|v| v.as_i64()).unwrap_or(0);
    let raw_list = list_data.get("list").and_then(|v| v.as_array()).cloned().unwrap_or_default();

    let list: Vec<MusicItem> = raw_list.iter().filter_map(|item| {
        let songmid = item.get("id")?.as_i64()?.to_string();
        let name = item.get("name")?.as_str()?.to_string();
        let singer = item.get("artists").and_then(|a| a.as_array()).map(|arr| {
            arr.iter().filter_map(|s| s.get("name").and_then(|n| n.as_str())).collect::<Vec<_>>().join("、")
        }).unwrap_or_default();
        let album_name = item.get("album").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let album_id = item.get("albumId").cloned().unwrap_or(serde_json::Value::Null);
        let img = item.get("albumPic").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let duration = item.get("duration")?.as_i64()?;
        let interval = format_play_time(duration);

        // Parse quality types from audios array
        let mut types = Vec::new();
        if let Some(audios) = item.get("audios").and_then(|a| a.as_array()) {
            for audio in audios {
                let bitrate = audio.get("bitrate").and_then(|v| v.as_str()).unwrap_or("");
                match bitrate {
                    "4000" => types.push("flac24bit".to_string()),
                    "2000" => types.push("flac".to_string()),
                    "320" => types.push("320k".to_string()),
                    "128" => types.push("128k".to_string()),
                    _ => {}
                }
            }
        }
        types.reverse();
        if types.is_empty() { types.push("128k".to_string()); }

        Some(MusicItem {
            songmid: serde_json::json!(songmid), singer, name, album_name, album_id,
            source: "bd".into(), interval, img, lrc: None,
            types: Some(types), types_map: None, type_url: Some(serde_json::json!({})),
            hash: None,
            song_id: None, str_media_mid: None, album_mid: None,
        copyright_id: None, lrc_url: None, mrc_url: None, trc_url: None,
        singer_id: None,
        })
    }).collect();

    Ok(serde_json::json!({
        "list": list,
        "info": { "name": name, "img": img, "desc": desc },
        "allPage": (total as f64 / limit as f64).ceil() as i64,
        "limit": limit as i64, "total": total, "source": "bd"
    }))
}

// --- Search (uses kw search API with bd source tag) ---

async fn search(args: serde_json::Value) -> Result<serde_json::Value, String> {
    let keyword = get_str(&args, "keyword");
    let page = get_u64(&args, "page", 1);
    let limit = get_u64(&args, "limit", 20);
    if keyword.is_empty() {
        return Ok(serde_json::to_value(SearchResult {
            list: vec![], all_page: 0, limit: limit as i64, total: 0, source: "bd".into(),
        }).unwrap());
    }

    // BD shares kuwo's search infrastructure
    let url = format!(
        "http://www.kuwo.cn/api/www/search/searchMusicBykeyWord?key={}&pn={}&rn={}&httpsStatus=1",
        urlencoding::encode(keyword), page, limit
    );

    let resp: serde_json::Value = get_http().get(&url)
        .header("User-Agent", "Mozilla/5.0 (iPhone; CPU iPhone OS 18_5 like Mac OS X)")
        .header("Referer", "http://www.kuwo.cn/search/list")
        .header("Cookie", "kw_token=12345")
        .send().await.map_err(|e| e.to_string())?
        .json_sanitized().await?;

    let total = resp.get("data").and_then(|d| d.get("total")).and_then(|v| v.as_i64()).unwrap_or(0);
    let raw_list = resp.get("data").and_then(|d| d.get("list")).and_then(|v| v.as_array()).cloned().unwrap_or_default();

    let list: Vec<MusicItem> = raw_list.iter().filter_map(|item| {
        let songmid = item.get("rid")?.as_i64()?.to_string();
        let name = item.get("name")?.as_str()?.to_string();
        let singer = item.get("artist")?.as_str().unwrap_or("").to_string();
        let album_name = item.get("album")?.as_str().unwrap_or("").to_string();
        let album_id = item.get("albid").cloned().unwrap_or(serde_json::Value::Null);
        let duration = item.get("duration")?.as_i64()?;
        let interval = format_play_time(duration);

        let mut types = Vec::new();
        if item.get("hasMp3").and_then(|v| v.as_bool()).unwrap_or(false) { types.push("128k".to_string()); }
        if item.get("hasLossless").and_then(|v| v.as_bool()).unwrap_or(false) { types.push("flac".to_string()); }
        if item.get("hasSq").and_then(|v| v.as_bool()).unwrap_or(false) { types.push("320k".to_string()); }
        types.reverse();

        Some(MusicItem {
            songmid: serde_json::json!(songmid), singer, name, album_name, album_id,
            source: "bd".into(), interval,
            img: item.get("albpic").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            lrc: None, types: Some(types), types_map: None, type_url: Some(serde_json::json!({})),
            hash: None,
            song_id: None, str_media_mid: None, album_mid: None,
        copyright_id: None, lrc_url: None, mrc_url: None, trc_url: None,
        singer_id: None,
        })
    }).collect();

    Ok(serde_json::to_value(SearchResult {
        list, all_page: (total as f64 / limit as f64).ceil() as i64,
        limit: limit as i64, total, source: "bd".into(),
    }).unwrap())
}

async fn search_playlist(args: serde_json::Value) -> Result<serde_json::Value, String> {
    let keyword = get_str(&args, "keyword");
    let page = get_u64(&args, "page", 1);
    let limit = get_u64(&args, "limit", 30);
    if keyword.is_empty() {
        return Ok(serde_json::to_value(PlaylistResult {
            list: vec![], all_page: 0, limit: limit as i64, total: 0, source: "bd".into(),
        }).unwrap());
    }

    let url = format!(
        "http://www.kuwo.cn/api/www/search/searchPlayList?key={}&pn={}&rn={}",
        urlencoding::encode(keyword), page, limit
    );

    let resp: serde_json::Value = get_http().get(&url)
        .header("User-Agent", "Mozilla/5.0 (iPhone; CPU iPhone OS 18_5 like Mac OS X)")
        .header("Referer", "http://www.kuwo.cn/search/list")
        .header("Cookie", "kw_token=12345")
        .send().await.map_err(|e| e.to_string())?
        .json_sanitized().await?;

    let total = resp.get("data").and_then(|d| d.get("total")).and_then(|v| v.as_i64()).unwrap_or(0);
    let raw_list = resp.get("data").and_then(|d| d.get("list")).and_then(|v| v.as_array()).cloned().unwrap_or_default();

    let list: Vec<PlaylistItem> = raw_list.iter().filter_map(|item| {
        Some(PlaylistItem {
            id: item.get("id")?.clone(),
            name: item.get("name")?.as_str()?.to_string(),
            img: item.get("img")?.as_str()?.to_string(),
            source: "bd".into(),
            desc: item.get("desc").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            play_count: item.get("playCount").cloned().unwrap_or(serde_json::Value::Null),
            author: String::new(),
            total: serde_json::Value::Null,
        })
    }).collect();

    Ok(serde_json::to_value(PlaylistResult {
        list, all_page: (total as f64 / limit as f64).ceil() as i64,
        limit: limit as i64, total, source: "bd".into(),
    }).unwrap())
}

// --- Router ---

pub async fn handle(method: &str, args: serde_json::Value) -> Result<serde_json::Value, String> {
    match method {
        "search" => search(args).await,
        "tipSearch" | "hotSearch" => Ok(serde_json::json!({ "list": [] })),
        "getMusicUrl" => Ok(serde_json::json!({ "url": "" })),
        "getPic" => Ok(serde_json::json!({ "url": "" })),
        "getLyric" => Ok(serde_json::json!({ "lrc": "" })),
        "getComment" | "getHotComment" => Ok(serde_json::json!({ "comments": [], "total": 0 })),
        "getHotSonglist" | "getHotPlaylists" => Ok(serde_json::to_value(PlaylistResult {
            list: vec![], all_page: 0, limit: 30, total: 0, source: "bd".into(),
        }).unwrap()),
        "getPlaylistTags" | "getSongboardTags" => Ok(serde_json::json!({ "tags": [], "hotTag": [], "source": "bd" })),
        "getCategoryPlaylists" => Ok(serde_json::to_value(PlaylistResult {
            list: vec![], all_page: 0, limit: 30, total: 0, source: "bd".into(),
        }).unwrap()),
        "getLeaderboards" => Ok(serde_json::json!({ "list": [], "source": "bd" })),
        "getPlaylistDetail" | "getPlaylistDetailById" => get_playlist_detail(args).await,
        "getLeaderboardDetail" => Ok(serde_json::json!({
            "list": [], "info": {}, "allPage": 0, "limit": 30, "total": 0, "source": "bd"
        })),
        "searchPlaylist" => search_playlist(args).await,
        _ => Err(format!("Unknown SDK method for bd: {}", method)),
    }
}
