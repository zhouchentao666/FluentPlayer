use super::helpers::*;
use crate::music_sdk::client::ResponseExt;
use super::crypto::eapi_form;
use crate::music_sdk::client::{MusicItem, SearchResult, SingerInfo, SingerDetail, SingerCount, SingerAlbumItem, AlbumBrief, SingerAlbumListResult};
use std::collections::HashMap;
use crate::music_sdk::client::QualityInfo;

pub async fn get_singer_info(args: serde_json::Value) -> Result<serde_json::Value, String> {
    let id = get_str(&args, "id");
    if id.is_empty() {
        return Err("WY singer info: missing id".into());
    }

    let body = eapi_form("/api/artist/head/info/get", &serde_json::json!({ "id": id.parse::<i64>().unwrap_or(0) }));

    let resp: serde_json::Value = get_http()
        .post("http://interface3.music.163.com/eapi/artist/head/info/get")
        .headers(reqwest::header::HeaderMap::from_iter(
            wy_headers().into_iter().map(|(k, v)| (k.parse().unwrap(), v.parse().unwrap())),
        ))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send().await.map_err(|e| e.to_string())?
        .json_sanitized().await?;

    let code = resp.get("code").and_then(|v| v.as_i64()).unwrap_or(0);
    if code != 200 {
        return Err(format!("WY singer info API error: code={}", code));
    }

    let artist = resp.get("artist").cloned().unwrap_or(serde_json::json!({}));
    let user = resp.get("user").cloned().unwrap_or(serde_json::json!({}));

    let info = SingerInfo {
        id: serde_json::json!(id),
        source: "wy".into(),
        info: SingerDetail {
            name: artist.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            desc: artist.get("briefDesc").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            avatar: user.get("avatarUrl").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            gender: match user.get("gender").and_then(|v| v.as_i64()).unwrap_or(0) {
                1 => Some("man".into()),
                2 => Some("woman".into()),
                _ => None,
            },
        },
        count: SingerCount {
            music: artist.get("musicSize").and_then(|v| v.as_i64()).unwrap_or(0),
            album: artist.get("albumSize").and_then(|v| v.as_i64()).unwrap_or(0),
        },
    };

    Ok(serde_json::to_value(info).unwrap())
}

pub async fn get_singer_song_list(args: serde_json::Value) -> Result<serde_json::Value, String> {
    let id = get_str(&args, "id");
    let page = get_u64(&args, "page", 1);
    let limit = get_u64(&args, "limit", 30);

    if id.is_empty() {
        return Err("WY singer song list: missing id".into());
    }

    let offset = limit * (page - 1);
    let body = eapi_form("/api/v2/artist/songs", &serde_json::json!({
        "id": id.parse::<i64>().unwrap_or(0),
        "limit": limit,
        "offset": offset
    }));

    let resp: serde_json::Value = get_http()
        .post("http://interface3.music.163.com/eapi/v2/artist/songs")
        .headers(reqwest::header::HeaderMap::from_iter(
            wy_headers().into_iter().map(|(k, v)| (k.parse().unwrap(), v.parse().unwrap())),
        ))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send().await.map_err(|e| e.to_string())?
        .json_sanitized().await?;

    let code = resp.get("code").and_then(|v| v.as_i64()).unwrap_or(0);
    if code != 200 {
        return Err(format!("WY singer song list API error: code={}", code));
    }

    let total = resp.get("total").and_then(|v| v.as_i64()).unwrap_or(0);
    let songs = resp.get("songs").and_then(|v| v.as_array()).cloned().unwrap_or_default();
    let list = filter_song_list(&songs);

    Ok(serde_json::to_value(SearchResult {
        list, all_page: (total as f64 / limit as f64).ceil() as i64,
        limit: limit as i64, total, source: "wy".into(),
    }).unwrap())
}

pub async fn get_singer_album_list(args: serde_json::Value) -> Result<serde_json::Value, String> {
    let id = get_str(&args, "id");
    let page = get_u64(&args, "page", 1);
    let limit = get_u64(&args, "limit", 30);

    if id.is_empty() {
        return Err("WY singer album list: missing id".into());
    }

    let offset = limit * (page - 1);
    let api_path = format!("/api/artist/albums/{}", id);
    let body = eapi_form(&api_path, &serde_json::json!({ "limit": limit, "offset": offset }));

    let resp: serde_json::Value = get_http()
        .post(&format!("http://interface3.music.163.com{}", api_path))
        .headers(reqwest::header::HeaderMap::from_iter(
            wy_headers().into_iter().map(|(k, v)| (k.parse().unwrap(), v.parse().unwrap())),
        ))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send().await.map_err(|e| e.to_string())?
        .json_sanitized().await?;

    let code = resp.get("code").and_then(|v| v.as_i64()).unwrap_or(0);
    if code != 200 {
        return Err(format!("WY singer album list API error: code={}", code));
    }

    let total = resp.get("artist").and_then(|a| a.get("albumSize")).and_then(|v| v.as_i64()).unwrap_or(0);
    let hot_albums = resp.get("hotAlbums").and_then(|v| v.as_array()).cloned().unwrap_or_default();
    let list: Vec<SingerAlbumItem> = hot_albums.iter().filter_map(|item| {
        let album_id = item.get("id")?;
        Some(SingerAlbumItem {
            id: album_id.clone(),
            count: item.get("size").and_then(|v| v.as_i64()).unwrap_or(0),
            info: AlbumBrief {
                name: item.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                author: format_singer(&item.get("artists").and_then(|v| v.as_array()).cloned().unwrap_or_default()),
                img: item.get("picUrl").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                desc: None,
            },
        })
    }).collect();

    Ok(serde_json::to_value(SingerAlbumListResult {
        list, all_page: (total as f64 / limit as f64).ceil() as i64,
        limit: limit as i64, total, source: "wy".into(),
    }).unwrap())
}

fn filter_song_list(raw: &[serde_json::Value]) -> Vec<MusicItem> {
    raw.iter().filter_map(|item| {
        let songmid = item.get("id")?;
        let privilege = item.get("privilege").cloned().unwrap_or(serde_json::json!({}));

        let types = parse_quality_types(&privilege);
        let types_map: HashMap<String, QualityInfo> = types.iter().map(|t| {
            (t.clone(), QualityInfo { size: String::new(), hash: None })
        }).collect();

        Some(MusicItem {
            songmid: songmid.clone(),
            singer: format_singer(&item.get("artists").and_then(|v| v.as_array()).cloned().unwrap_or_default()),
            name: item.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            album_name: item.get("album").and_then(|a| a.get("name")).and_then(|v| v.as_str()).unwrap_or("").to_string(),
            album_id: item.get("album").and_then(|a| a.get("id")).cloned().unwrap_or(serde_json::Value::Null),
            source: "wy".into(),
            interval: format_play_time(item.get("duration").and_then(|v| v.as_i64()).unwrap_or(0) / 1000),
            img: String::new(),
            lrc: None,
            types: Some(types),
            types_map: Some(types_map),
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
    }).collect()
}
