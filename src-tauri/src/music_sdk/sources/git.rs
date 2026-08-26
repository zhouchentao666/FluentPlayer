use crate::music_sdk::client::ResponseExt;
use crate::music_sdk::client::{self, MusicItem, PlaylistItem, PlaylistResult, SearchResult};

fn get_http() -> &'static reqwest::Client {
    client::get_client()
}

fn get_str<'a>(args: &'a serde_json::Value, key: &str) -> &'a str {
    args.get(key).and_then(|v| v.as_str()).unwrap_or("")
}

fn get_u64(args: &serde_json::Value, key: &str, default: u64) -> u64 {
    args.get(key).and_then(|v| v.as_u64()).unwrap_or(default)
}

const GITCODE_API: &str =
    "https://api.gitcode.com/api/v5/repos/ikun_0014/music/raw/audio_database.json";
const PIC_URL: &str = "data:image/png;base64,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";

// Cached music database
static DB_CACHE: once_cell::sync::Lazy<tokio::sync::RwLock<Option<Vec<serde_json::Value>>>> =
    once_cell::sync::Lazy::new(|| tokio::sync::RwLock::new(None));

async fn fetch_git_db() -> Result<Vec<serde_json::Value>, String> {
    {
        let cache = DB_CACHE.read().await;
        if let Some(ref data) = *cache {
            return Ok(data.clone());
        }
    }

    let resp: serde_json::Value = get_http()
        .get(GITCODE_API)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json_sanitized()
        .await?;

    let list = if resp.is_array() {
        resp.as_array().cloned().unwrap_or_default()
    } else {
        vec![]
    };

    {
        let mut cache = DB_CACHE.write().await;
        *cache = Some(list.clone());
    }

    Ok(list)
}

fn format_music_item(item: &serde_json::Value) -> Option<MusicItem> {
    let title = item
        .get("title")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let filename = item.get("filename").and_then(|v| v.as_str()).unwrap_or("");
    let name = if title.is_empty() {
        filename
            .strip_suffix(|c: char| c != '.')
            .unwrap_or(filename)
            .to_string()
    } else {
        title
    };

    let singer = item
        .get("artist")
        .and_then(|v| v.as_str())
        .unwrap_or("未知歌手")
        .to_string();
    let album_name = item
        .get("album")
        .and_then(|v| v.as_str())
        .unwrap_or("未知专辑")
        .to_string();
    let songmid = item
        .get("relative_path")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let img = item
        .get("img")
        .and_then(|v| v.as_str())
        .unwrap_or(PIC_URL)
        .to_string();
    let lrc = item
        .get("lyrics")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let format = item
        .get("format")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_lowercase();
    let types = match format.as_str() {
        "flac" => vec!["flac".to_string()],
        "mp3" => vec!["320k".to_string()],
        _ => vec!["128k".to_string()],
    };

    Some(MusicItem {
        songmid: serde_json::json!(songmid),
        singer,
        name,
        album_name,
        album_id: serde_json::Value::String(String::new()),
        source: "git".into(),
        interval: String::new(),
        img,
        lrc,
        types: Some(types),
        types_map: None,
        type_url: Some(serde_json::json!({})),
        hash: None,
        song_id: None,
        str_media_mid: None,
        album_mid: None,
        copyright_id: None,
        lrc_url: None,
        mrc_url: None,
        trc_url: None,
        singer_id: None,
    })
}

// --- Get Playlist Tags (empty) ---

async fn get_playlist_tags(_args: serde_json::Value) -> Result<serde_json::Value, String> {
    Ok(serde_json::json!({ "tags": [], "hotTag": [], "source": "git" }))
}

// --- Get Category Playlists (static single playlist) ---

async fn get_category_playlists(_args: serde_json::Value) -> Result<serde_json::Value, String> {
    let list = vec![PlaylistItem {
        id: serde_json::json!("git_main"),
        name: "下架歌曲收录".to_string(),
        img: PIC_URL.to_string(),
        source: "git".into(),
        desc: "来自 GitCode 仓库的音乐收藏".to_string(),
        play_count: serde_json::Value::Null,
        author: "GitCode".to_string(),
        total: serde_json::Value::Null,
    }];

    Ok(serde_json::to_value(PlaylistResult {
        list,
        all_page: 1,
        limit: 30,
        total: 1,
        source: "git".into(),
    })
    .unwrap())
}

// --- Get Playlist Detail (fetch from GitCode) ---

async fn get_playlist_detail(args: serde_json::Value) -> Result<serde_json::Value, String> {
    let page = get_u64(&args, "page", 1);
    let _limit = get_u64(&args, "limit", 30);

    let db = fetch_git_db().await?;
    let total = db.len() as i64;

    // Paginate
    let limit = 10000i64;
    let start = ((page - 1) * limit as u64) as usize;
    let end = (start + limit as usize).min(db.len());

    let list: Vec<MusicItem> = if start < db.len() {
        db[start..end]
            .iter()
            .filter_map(format_music_item)
            .collect()
    } else {
        vec![]
    };

    Ok(serde_json::json!({
        "list": list,
        "info": { "name": "下架歌曲收录", "img": PIC_URL, "desc": "来自 GitCode 仓库的音乐收藏" },
        "allPage": 1, "limit": limit, "total": total, "source": "git"
    }))
}

// --- Search (filter in memory) ---

async fn search(args: serde_json::Value) -> Result<serde_json::Value, String> {
    let keyword = get_str(&args, "keyword");
    let page = get_u64(&args, "page", 1);
    let limit = get_u64(&args, "limit", 20);
    if keyword.is_empty() {
        return Ok(serde_json::to_value(SearchResult {
            list: vec![],
            all_page: 0,
            limit: limit as i64,
            total: 0,
            source: "git".into(),
        })
        .unwrap());
    }

    let db = fetch_git_db().await?;

    // Build regex pattern: allow any char between keyword chars
    let pattern: String = keyword
        .chars()
        .map(|c| regex_lite::escape(&c.to_string()))
        .collect::<Vec<_>>()
        .join(".*");

    let re = regex_lite::Regex::new(&format!("(?i){}", pattern))
        .unwrap_or_else(|_| regex_lite::Regex::new("").unwrap());

    let filtered: Vec<MusicItem> = db
        .iter()
        .filter_map(|item| {
            let name = item.get("title").and_then(|v| v.as_str()).unwrap_or("");
            if re.is_match(name) {
                format_music_item(item)
            } else {
                None
            }
        })
        .collect();

    let total = filtered.len() as i64;
    let start = ((page - 1) * limit) as usize;
    let end = (start + limit as usize).min(filtered.len());
    let list = if start < filtered.len() {
        filtered[start..end].to_vec()
    } else {
        vec![]
    };

    Ok(serde_json::to_value(SearchResult {
        list,
        all_page: (total as f64 / limit as f64).ceil() as i64,
        limit: limit as i64,
        total,
        source: "git".into(),
    })
    .unwrap())
}

// --- Search Playlist (return main playlist if keyword matches) ---

async fn search_playlist(args: serde_json::Value) -> Result<serde_json::Value, String> {
    let keyword = get_str(&args, "keyword");
    let db = fetch_git_db().await?;

    let pattern: String = keyword
        .chars()
        .map(|c| regex_lite::escape(&c.to_string()))
        .collect::<Vec<_>>()
        .join(".*");
    let re = regex_lite::Regex::new(&format!("(?i){}", pattern))
        .unwrap_or_else(|_| regex_lite::Regex::new("").unwrap());

    let match_count = db
        .iter()
        .filter(|item| {
            let name = item.get("title").and_then(|v| v.as_str()).unwrap_or("");
            re.is_match(name)
        })
        .count();

    let list = vec![PlaylistItem {
        id: serde_json::json!("git_main"),
        name: "下架歌曲收录".to_string(),
        img: PIC_URL.to_string(),
        source: "git".into(),
        desc: "来自 GitCode 仓库的音乐收藏".to_string(),
        play_count: serde_json::Value::Null,
        author: "GitCode".to_string(),
        total: serde_json::Value::Null,
    }];

    Ok(serde_json::to_value(PlaylistResult {
        list,
        all_page: 1,
        limit: 30,
        total: match_count as i64,
        source: "git".into(),
    })
    .unwrap())
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
        "getHotSonglist" | "getHotPlaylists" => get_category_playlists(args).await,
        "getPlaylistTags" | "getSongboardTags" => get_playlist_tags(args).await,
        "getCategoryPlaylists" => get_category_playlists(args).await,
        "getLeaderboards" => Ok(serde_json::json!({ "list": [], "source": "git" })),
        "getPlaylistDetail" | "getPlaylistDetailById" => get_playlist_detail(args).await,
        "getLeaderboardDetail" => Ok(serde_json::json!({
            "list": [], "info": {}, "allPage": 0, "limit": 30, "total": 0, "source": "git"
        })),
        "searchPlaylist" => search_playlist(args).await,
        _ => Err(format!("Unknown SDK method for git: {}", method)),
    }
}
