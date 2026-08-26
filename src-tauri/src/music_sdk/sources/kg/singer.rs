use super::helpers::*;
use crate::music_sdk::client::ResponseExt;
use super::playback::get_batch_quality_info;
use crate::music_sdk::client::{MusicItem, SearchResult, SingerInfo, SingerDetail, SingerCount, SingerAlbumItem, AlbumBrief, SingerAlbumListResult};

pub async fn get_singer_info(args: serde_json::Value) -> Result<serde_json::Value, String> {
    let id = get_str(&args, "id");
    if id.is_empty() {
        return Err("KG singer info: missing id".into());
    }
    if id == "0" {
        return Err("KG singer info: singer not found (id=0)".into());
    }

    let url = format!("http://mobiles.kugou.com/api/v5/singer/info?singerid={}", id);
    let resp: serde_json::Value = get_http().get(&url)
        .send().await.map_err(|e| e.to_string())?
        .json_sanitized().await?;

    if resp.is_null() {
        return Err("KG singer info: empty response".into());
    }

    let info = SingerInfo {
        id: serde_json::json!(id),
        source: "kg".into(),
        info: SingerDetail {
            name: resp.get("singername").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            desc: resp.get("intro").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            avatar: resp.get("imgurl").and_then(|v| v.as_str()).unwrap_or("").replace("{size}", "480"),
            gender: match resp.get("grade").and_then(|v| v.as_i64()).unwrap_or(0) {
                1 => Some("man".into()),
                2 => Some("woman".into()),
                _ => None,
            },
        },
        count: SingerCount {
            music: resp.get("songcount").and_then(|v| v.as_i64()).unwrap_or(0),
            album: resp.get("albumcount").and_then(|v| v.as_i64()).unwrap_or(0),
        },
    };

    Ok(serde_json::to_value(info).unwrap())
}

pub async fn get_singer_song_list(args: serde_json::Value) -> Result<serde_json::Value, String> {
    let id = get_str(&args, "id");
    let page = get_u64(&args, "page", 1);
    let limit = get_u64(&args, "limit", 30);

    if id.is_empty() {
        return Err("KG singer song list: missing id".into());
    }
    if id == "0" {
        return Err("KG singer song list: singer not found (id=0)".into());
    }

    let url = format!(
        "http://mobiles.kugou.com/api/v5/singer/song?singerid={}&page={}&pagesize={}",
        id, page, limit
    );
    let resp: serde_json::Value = get_http().get(&url)
        .send().await.map_err(|e| e.to_string())?
        .json_sanitized().await?;

    let info = resp.get("info").and_then(|v| v.as_array()).cloned().unwrap_or_default();
    let total = resp.get("total").and_then(|v| v.as_i64()).unwrap_or(0);

    let hashes: Vec<String> = info.iter().filter_map(|item| {
        item.get("hash").and_then(|v| v.as_str()).map(|s| s.to_string())
    }).collect();

    let quality_map = get_batch_quality_info(&hashes).await.unwrap_or_default();

    let list: Vec<MusicItem> = info.iter().filter_map(|item| {
        let hash = item.get("hash").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let (types, types_map) = quality_map.get(&hash).cloned().unwrap_or((vec![], std::collections::HashMap::new()));

        Some(MusicItem {
            songmid: serde_json::json!(hash),
            singer: item.get("singername").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            name: decode_html(item.get("songname").and_then(|v| v.as_str()).unwrap_or("")),
            album_name: decode_html(item.get("album_name").and_then(|v| v.as_str()).unwrap_or("")),
            album_id: item.get("album_id").cloned().unwrap_or(serde_json::Value::Null),
            source: "kg".into(),
            interval: format_play_time(item.get("duration").and_then(|v| v.as_i64()).unwrap_or(0)),
            img: String::new(),
            lrc: None,
            types: if types.is_empty() { None } else { Some(types) },
            types_map: if types_map.is_empty() { None } else { Some(types_map) },
            type_url: Some(serde_json::json!({})),
            hash: Some(hash),
            song_id: None,
            str_media_mid: None,
            album_mid: None,
            copyright_id: None,
            lrc_url: None,
            mrc_url: None,
            trc_url: None,
            singer_id: None,
        })
    }).collect();

    Ok(serde_json::to_value(SearchResult {
        list, all_page: (total as f64 / limit as f64).ceil() as i64,
        limit: limit as i64, total, source: "kg".into(),
    }).unwrap())
}

pub async fn get_singer_album_list(args: serde_json::Value) -> Result<serde_json::Value, String> {
    let id = get_str(&args, "id");
    let page = get_u64(&args, "page", 1);
    let limit = get_u64(&args, "limit", 30);

    if id.is_empty() {
        return Err("KG singer album list: missing id".into());
    }
    if id == "0" {
        return Err("KG singer album list: singer not found (id=0)".into());
    }

    let url = format!(
        "http://mobiles.kugou.com/api/v5/singer/album?singerid={}&page={}&pagesize={}",
        id, page, limit
    );
    let resp: serde_json::Value = get_http().get(&url)
        .send().await.map_err(|e| e.to_string())?
        .json_sanitized().await?;

    let info = resp.get("info").and_then(|v| v.as_array()).cloned().unwrap_or_default();
    let total = resp.get("total").and_then(|v| v.as_i64()).unwrap_or(0);

    let list: Vec<SingerAlbumItem> = info.iter().filter_map(|item| {
        Some(SingerAlbumItem {
            id: item.get("albumid").cloned().unwrap_or(serde_json::Value::Null),
            count: item.get("songcount").and_then(|v| v.as_i64()).unwrap_or(0),
            info: AlbumBrief {
                name: decode_html(item.get("albumname").and_then(|v| v.as_str()).unwrap_or("")),
                author: item.get("singername").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                img: item.get("imgurl").and_then(|v| v.as_str()).unwrap_or("").replace("{size}", "480"),
                desc: item.get("intro").and_then(|v| v.as_str()).map(|s| s.to_string()),
            },
        })
    }).collect();

    Ok(serde_json::to_value(SingerAlbumListResult {
        list, all_page: (total as f64 / limit as f64).ceil() as i64,
        limit: limit as i64, total, source: "kg".into(),
    }).unwrap())
}
