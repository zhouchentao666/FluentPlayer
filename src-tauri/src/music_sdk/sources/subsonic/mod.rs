use crate::music_sdk::client::{MusicItem, PlaylistDetailResult, PlaylistItem, PlaylistResult, SearchResult};
use md5::{Digest, Md5};
use rand::{distributions::Alphanumeric, Rng};
use serde_json::{json, Value};

const SOURCE: &str = "subsonic";
const DEFAULT_API_VERSION: &str = "1.16.1";
const DEFAULT_CLIENT_NAME: &str = "Mio";

#[derive(Debug, Clone)]
struct Config {
    base_url: String,
    username: String,
    password: String,
    api_version: String,
    client_name: String,
}

pub async fn handle(method: &str, args: Value) -> Result<Value, String> {
    match method {
        "ping" | "testConnection" => test_connection(args).await,
        "search" => search(args).await,
        "tipSearch" | "hotSearch" => Ok(json!({ "list": [] })),
        "getMusicUrl" => get_music_url(args).await,
        "getPic" => get_pic(args).await,
        "getLyric" => get_lyric(args).await,
        "getHotSonglist" | "getHotPlaylists" => get_hot_songlist(args).await,
        "getPlaylistTags" | "getSongboardTags" => Ok(json!({
            "tags": [
                { "id": "recent", "name": "最近添加" },
                { "id": "newest", "name": "最新专辑" },
                { "id": "random", "name": "随机专辑" },
                { "id": "starred", "name": "收藏专辑" },
                { "id": "alphabeticalByName", "name": "按名称排序" }
            ],
            "hotTag": []
        })),
        "getCategoryPlaylists" => get_category_playlists(args).await,
        "getPlaylistDetail" | "getPlaylistDetailById" => get_playlist_detail(args).await,
        "searchPlaylist" => search_playlist(args).await,
        "getLeaderboards" => Ok(json!({ "list": [], "source": SOURCE })),
        "getLeaderboardDetail" => Ok(json!(empty_playlist_detail())),
        "getComment" | "getHotComment" => Ok(json!({ "comments": [], "total": 0, "source": SOURCE })),
        "recognize" => Ok(json!({ "list": [] })),
        "getSingerInfo" => Ok(json!({})),
        "getSingerSongList" | "getSingerAlbumList" => Ok(json!(SearchResult {
            list: vec![],
            all_page: 0,
            limit: 30,
            total: 0,
            source: SOURCE.into(),
        })),
        "getAlbumInfo" => Ok(json!({ "list": [], "info": {} })),
        "getArtistList" => Ok(json!({ "list": [], "total": 0, "allPage": 0, "limit": 60, "source": SOURCE })),
        _ => Err(format!("Unknown SDK method for subsonic: {method}")),
    }
}

async fn test_connection(args: Value) -> Result<Value, String> {
    let config = read_config(&args)?;
    call_api(&config, "ping.view", &[]).await?;
    Ok(json!({ "success": true, "message": "连接成功", "source": SOURCE }))
}

async fn search(args: Value) -> Result<Value, String> {
    let config = read_config(&args)?;
    let keyword = get_str(&args, "keyword");
    let page = get_u64(&args, "page", 1).max(1);
    let limit = get_u64(&args, "limit", 30).max(1);
    if keyword.trim().is_empty() {
        return Ok(json!(SearchResult {
            list: vec![],
            all_page: 0,
            limit: limit as i64,
            total: 0,
            source: SOURCE.into(),
        }));
    }

    let offset = (page - 1) * limit;
    let song_count = limit.to_string();
    let song_offset = offset.to_string();
    let resp = call_api(
        &config,
        "search3.view",
        &[
            ("query", keyword),
            ("songCount", &song_count),
            ("songOffset", &song_offset),
            ("artistCount", "0"),
            ("albumCount", "0"),
        ],
    )
    .await?;

    let songs = resp
        .get("searchResult3")
        .and_then(|v| v.get("song"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let list: Vec<MusicItem> = songs.iter().map(|song| map_song(song, &config)).collect();
    let total = if list.len() as u64 == limit { (page * limit) as i64 } else { offset as i64 + list.len() as i64 };

    Ok(json!(SearchResult {
        list,
        all_page: if limit == 0 { 0 } else { ((total as f64) / (limit as f64)).ceil() as i64 },
        limit: limit as i64,
        total,
        source: SOURCE.into(),
    }))
}

async fn get_music_url(args: Value) -> Result<Value, String> {
    let config = read_config(&args)?;
    let songmid = get_songmid(&args);
    if songmid.is_empty() {
        return Ok(json!({ "url": "", "type": get_str(&args, "quality") }));
    }

    let quality = get_str(&args, "quality");
    let mut params = vec![("id", songmid.as_str())];
    let max_bit_rate = quality_to_max_bit_rate(quality);
    if let Some(bit_rate) = max_bit_rate.as_deref() {
        params.push(("maxBitRate", bit_rate));
    }
    let url = build_url(&config, "stream.view", &params)?;
    Ok(json!({ "url": url, "type": quality }))
}

async fn get_pic(args: Value) -> Result<Value, String> {
    let config = read_config(&args)?;
    let song_info = args.get("songInfo").unwrap_or(&Value::Null);
    let id = song_info
        .get("coverArt")
        .and_then(Value::as_str)
        .map(str::to_string)
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| get_songmid(&args));
    if id.is_empty() {
        return Ok(json!({ "url": "" }));
    }
    let url = build_url(&config, "getCoverArt.view", &[("id", id.as_str()), ("size", "600")])?;
    Ok(json!({ "url": url }))
}

async fn get_lyric(args: Value) -> Result<Value, String> {
    let config = read_config(&args)?;
    let song_info = args.get("songInfo").unwrap_or(&Value::Null);
    let artist = song_info.get("singer").and_then(Value::as_str).unwrap_or("");
    let title = song_info.get("name").and_then(Value::as_str).unwrap_or("");
    if artist.is_empty() && title.is_empty() {
        return Ok(empty_lyric());
    }

    let resp = match call_api(&config, "getLyrics.view", &[("artist", artist), ("title", title)]).await {
        Ok(resp) => resp,
        Err(_) => return Ok(empty_lyric()),
    };
    let lyric = resp
        .get("lyrics")
        .and_then(|v| v.get("value"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    Ok(json!({ "lyric": lyric, "tlyric": "", "crlyric": "", "source": SOURCE }))
}

async fn get_hot_songlist(args: Value) -> Result<Value, String> {
    let mut args = args;
    if !args.get("sortId").is_some_and(|v| !v.is_null()) {
        args["sortId"] = json!("recent");
    }
    get_category_playlists(args).await
}

async fn get_category_playlists(args: Value) -> Result<Value, String> {
    let config = read_config(&args)?;
    let page = get_u64(&args, "page", 1).max(1);
    let limit = get_u64(&args, "limit", 30).max(1);
    let offset = ((page - 1) * limit).to_string();
    let size = limit.to_string();
    let list_type = album_list_type(get_str(&args, "sortId"));
    let resp = call_api(
        &config,
        "getAlbumList2.view",
        &[("type", list_type), ("size", &size), ("offset", &offset)],
    )
    .await?;

    let albums = resp
        .get("albumList2")
        .and_then(|v| v.get("album"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let list: Vec<PlaylistItem> = albums.iter().map(|album| map_album(album, &config)).collect();
    let total = if list.len() as u64 == limit { (page * limit) as i64 } else { ((page - 1) * limit) as i64 + list.len() as i64 };

    Ok(json!(PlaylistResult {
        list,
        all_page: ((total as f64) / (limit as f64)).ceil() as i64,
        limit: limit as i64,
        total,
        source: SOURCE.into(),
    }))
}

async fn search_playlist(args: Value) -> Result<Value, String> {
    let config = read_config(&args)?;
    let keyword = get_str(&args, "keyword");
    let page = get_u64(&args, "page", 1).max(1);
    let limit = get_u64(&args, "limit", 30).max(1);
    if keyword.trim().is_empty() {
        return Ok(json!(PlaylistResult { list: vec![], all_page: 0, limit: limit as i64, total: 0, source: SOURCE.into() }));
    }

    let offset = (page - 1) * limit;
    let album_count = limit.to_string();
    let album_offset = offset.to_string();
    let resp = call_api(
        &config,
        "search3.view",
        &[
            ("query", keyword),
            ("songCount", "0"),
            ("artistCount", "0"),
            ("albumCount", &album_count),
            ("albumOffset", &album_offset),
        ],
    )
    .await?;

    let albums = resp
        .get("searchResult3")
        .and_then(|v| v.get("album"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let list: Vec<PlaylistItem> = albums.iter().map(|album| map_album(album, &config)).collect();
    let total = if list.len() as u64 == limit { (page * limit) as i64 } else { offset as i64 + list.len() as i64 };

    Ok(json!(PlaylistResult {
        list,
        all_page: ((total as f64) / (limit as f64)).ceil() as i64,
        limit: limit as i64,
        total,
        source: SOURCE.into(),
    }))
}

async fn get_playlist_detail(args: Value) -> Result<Value, String> {
    let config = read_config(&args)?;
    let id = args
        .get("id")
        .map(value_to_string)
        .filter(|s| !s.is_empty())
        .or_else(|| args.get("songlistId").map(value_to_string).filter(|s| !s.is_empty()))
        .unwrap_or_default();
    if id.is_empty() {
        return Ok(json!(empty_playlist_detail()));
    }

    let resp = match call_api(&config, "getAlbum.view", &[("id", id.as_str())]).await {
        Ok(resp) => resp,
        Err(_) => return Ok(json!(empty_playlist_detail())),
    };
    let album = resp.get("album").cloned().unwrap_or(Value::Null);
    let songs = album
        .get("song")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let list: Vec<MusicItem> = songs.iter().map(|song| map_song(song, &config)).collect();
    let total = list.len() as i64;
    let limit = get_u64(&args, "limit", total.max(30) as u64) as i64;

    Ok(json!(PlaylistDetailResult {
        list,
        info: json!({
            "id": id,
            "name": album.get("name").and_then(Value::as_str).unwrap_or(""),
            "img": cover_url(&config, album.get("coverArt").and_then(Value::as_str).unwrap_or("")),
            "source": SOURCE,
            "desc": album.get("artist").and_then(Value::as_str).unwrap_or(""),
            "author": album.get("artist").and_then(Value::as_str).unwrap_or(""),
            "total": total,
        }),
        all_page: 1,
        limit,
        total,
        source: SOURCE.into(),
    }))
}

async fn call_api(config: &Config, method: &str, params: &[(&str, &str)]) -> Result<Value, String> {
    let url = build_url(config, method, params)?;
    let resp: Value = crate::music_sdk::client::get_client()
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Subsonic 请求失败: {e}"))?
        .json()
        .await
        .map_err(|e| format!("Subsonic 响应解析失败: {e}"))?;
    parse_response(resp)
}

fn parse_response(resp: Value) -> Result<Value, String> {
    let data = resp
        .get("subsonic-response")
        .ok_or_else(|| "Subsonic 响应缺少 subsonic-response".to_string())?;
    if data.get("status").and_then(Value::as_str).unwrap_or("") == "failed" {
        let message = data
            .get("error")
            .and_then(|e| e.get("message"))
            .and_then(Value::as_str)
            .unwrap_or("Subsonic 请求失败");
        return Err(message.to_string());
    }
    Ok(data.clone())
}

fn build_url(config: &Config, method: &str, params: &[(&str, &str)]) -> Result<String, String> {
    let mut url = reqwest::Url::parse(&format!("{}/rest/{method}", config.base_url))
        .map_err(|e| format!("Subsonic 地址无效: {e}"))?;
    {
        let mut pairs = url.query_pairs_mut();
        pairs
            .append_pair("u", &config.username)
            .append_pair("v", &config.api_version)
            .append_pair("c", &config.client_name)
            .append_pair("f", "json");
        let salt = random_salt();
        pairs
            .append_pair("s", &salt)
            .append_pair("t", &token(&config.password, &salt));
        for (key, value) in params {
            if !value.is_empty() {
                pairs.append_pair(key, value);
            }
        }
    }
    Ok(url.to_string())
}

fn read_config(args: &Value) -> Result<Config, String> {
    let config = args.get("subsonicConfig").unwrap_or(&Value::Null);
    let base_url = get_config_str(config, "baseUrl");
    let username = get_config_str(config, "username");
    let password = get_config_str(config, "password");
    if base_url.is_empty() || username.is_empty() || password.is_empty() {
        return Err("请先配置 Subsonic 服务器地址、用户名和密码".into());
    }
    Ok(Config {
        base_url: normalize_base_url(base_url),
        username: username.to_string(),
        password: password.to_string(),
        api_version: get_config_str(config, "apiVersion")
            .trim()
            .to_string()
            .if_empty(DEFAULT_API_VERSION),
        client_name: get_config_str(config, "clientName")
            .trim()
            .to_string()
            .if_empty(DEFAULT_CLIENT_NAME),
    })
}

fn normalize_base_url(base_url: &str) -> String {
    base_url.trim().trim_end_matches('/').trim_end_matches("/rest").to_string()
}

fn get_config_str<'a>(config: &'a Value, key: &str) -> &'a str {
    config.get(key).and_then(Value::as_str).unwrap_or("")
}

fn get_str<'a>(args: &'a Value, key: &str) -> &'a str {
    args.get(key).and_then(Value::as_str).unwrap_or("")
}

fn get_u64(args: &Value, key: &str, default: u64) -> u64 {
    args.get(key).and_then(Value::as_u64).unwrap_or(default)
}

fn get_songmid(args: &Value) -> String {
    let info = args.get("songInfo").unwrap_or(&Value::Null);
    info.get("songmid").map(value_to_string).unwrap_or_default()
}

fn value_to_string(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        _ => String::new(),
    }
}

fn map_song(song: &Value, config: &Config) -> MusicItem {
    let songmid = song.get("id").map(value_to_string).unwrap_or_default();
    let cover_art = song.get("coverArt").and_then(Value::as_str).unwrap_or("");
    MusicItem {
        songmid: json!(songmid),
        singer: song.get("artist").and_then(Value::as_str).unwrap_or("").to_string(),
        name: song.get("title").and_then(Value::as_str).unwrap_or("").to_string(),
        album_name: song.get("album").and_then(Value::as_str).unwrap_or("").to_string(),
        album_id: song.get("albumId").cloned().unwrap_or(Value::Null),
        source: SOURCE.into(),
        interval: format_duration(song.get("duration").and_then(Value::as_u64).unwrap_or(0)),
        img: cover_url(config, cover_art),
        lrc: None,
        types: Some(quality_types(song)),
        types_map: None,
        type_url: None,
        hash: None,
        singer_id: song.get("artistId").and_then(Value::as_str).map(str::to_string),
        song_id: Some(song.get("id").cloned().unwrap_or_else(|| json!(songmid))),
        str_media_mid: None,
        album_mid: None,
        copyright_id: None,
        lrc_url: None,
        mrc_url: None,
        trc_url: None,
    }
}

fn map_album(album: &Value, config: &Config) -> PlaylistItem {
    let id = album.get("id").cloned().unwrap_or(Value::Null);
    let song_count = album.get("songCount").cloned().unwrap_or(Value::Null);
    PlaylistItem {
        id,
        name: album.get("name").and_then(Value::as_str).unwrap_or("").to_string(),
        img: cover_url(config, album.get("coverArt").and_then(Value::as_str).unwrap_or("")),
        source: SOURCE.into(),
        desc: album.get("artist").and_then(Value::as_str).unwrap_or("").to_string(),
        play_count: song_count.clone(),
        author: album.get("artist").and_then(Value::as_str).unwrap_or("").to_string(),
        total: song_count,
    }
}

fn empty_playlist_detail() -> PlaylistDetailResult {
    PlaylistDetailResult {
        list: vec![],
        info: json!({}),
        all_page: 0,
        limit: 30,
        total: 0,
        source: SOURCE.into(),
    }
}

fn empty_lyric() -> Value {
    json!({ "lyric": "", "tlyric": "", "crlyric": "", "source": SOURCE })
}

fn cover_url(config: &Config, cover_art: &str) -> String {
    if cover_art.is_empty() {
        String::new()
    } else {
        build_url(config, "getCoverArt.view", &[("id", cover_art), ("size", "600")]).unwrap_or_default()
    }
}

fn quality_types(song: &Value) -> Vec<String> {
    let suffix = song.get("suffix").and_then(Value::as_str).unwrap_or("").to_lowercase();
    let bit_rate = song.get("bitRate").and_then(Value::as_u64).unwrap_or(0);
    let mut types = Vec::new();
    if matches!(suffix.as_str(), "flac" | "alac" | "wav" | "aiff") {
        types.push("flac".to_string());
    }
    if bit_rate >= 320 {
        types.push("320k".to_string());
    }
    types.push("128k".to_string());
    types.dedup();
    types
}

fn quality_to_max_bit_rate(quality: &str) -> Option<String> {
    match quality {
        "128k" => Some("128".into()),
        "320k" => Some("320".into()),
        _ => None,
    }
}

fn album_list_type(sort_id: &str) -> &str {
    match sort_id {
        "newest" | "recent" | "random" | "starred" | "alphabeticalByName" | "alphabeticalByArtist" => sort_id,
        _ => "recent",
    }
}

fn format_duration(seconds: u64) -> String {
    let minutes = seconds / 60;
    let secs = seconds % 60;
    format!("{minutes:02}:{secs:02}")
}

fn random_salt() -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(12)
        .map(char::from)
        .collect()
}

fn token(password: &str, salt: &str) -> String {
    let mut hasher = Md5::new();
    hasher.update(password.as_bytes());
    hasher.update(salt.as_bytes());
    format!("{:x}", hasher.finalize())
}

trait IfEmpty {
    fn if_empty(self, default: &str) -> String;
}

impl IfEmpty for String {
    fn if_empty(self, default: &str) -> String {
        if self.is_empty() { default.to_string() } else { self }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_base_url() {
        assert_eq!(normalize_base_url("https://demo.example/rest/"), "https://demo.example");
        assert_eq!(normalize_base_url("https://demo.example/"), "https://demo.example");
    }

    #[test]
    fn builds_md5_token() {
        assert_eq!(token("sesame", "c19b2d"), "26719a1196d2a940705a59634eb18eab");
    }

    #[test]
    fn formats_duration() {
        assert_eq!(format_duration(65), "01:05");
    }
}
