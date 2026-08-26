use super::helpers::*;
use crate::music_sdk::client::ResponseExt;
use super::crypto::{linuxapi_form, eapi_form};
use crate::music_sdk::client::{MusicItem, PlaylistItem, PlaylistResult};

pub async fn get_playlist_tags(_args: serde_json::Value) -> Result<serde_json::Value, String> {
    let (tags_result, hot_result) = tokio::join!(
        async {
            let body = linuxapi_form(&serde_json::json!({
                "method": "POST",
                "url": "https://music.163.com/api/playlist/catalogue",
                "params": {}
            }));
            let resp: serde_json::Value = get_http()
                .post("https://music.163.com/api/linux/forward")
                .headers(reqwest::header::HeaderMap::from_iter(
                    wy_headers().into_iter().map(|(k, v)| (k.parse().unwrap(), v.parse().unwrap())),
                ))
                .header("Content-Type", "application/x-www-form-urlencoded")
                .body(body)
                .send().await.map_err(|e| e.to_string())?
                .json_sanitized().await?;

            let code = resp.get("code").and_then(|v| v.as_i64()).unwrap_or(0);
            if code != 200 { return Err(format!("WY tag API error: code={}", code)); }

            let sub = resp.get("sub").and_then(|v| v.as_array()).cloned().unwrap_or_default();
            let categories_raw = resp.get("categories").cloned().unwrap_or(serde_json::json!({}));

            let mut cat_tags: std::collections::HashMap<String, Vec<serde_json::Value>> = std::collections::HashMap::new();
            for item in &sub {
                let cat_idx = item.get("category").and_then(|v| v.as_i64()).unwrap_or(0).to_string();
                let tag_name = item.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
                cat_tags.entry(cat_idx).or_default().push(serde_json::json!({
                    "id": tag_name, "name": tag_name, "source": "wy"
                }));
            }

            let list: Vec<serde_json::Value> = categories_raw.as_object().map(|cats| {
                cats.iter().map(|(key, val)| {
                    let cat_name = val.as_str().unwrap_or("");
                    serde_json::json!({ "name": cat_name, "list": cat_tags.get(key).cloned().unwrap_or_default(), "source": "wy" })
                }).collect::<Vec<_>>()
            }).unwrap_or_default();

            Ok::<_, String>(list)
        },
        async {
            let body = linuxapi_form(&serde_json::json!({
                "method": "POST",
                "url": "https://music.163.com/api/playlist/hottags",
                "params": {}
            }));
            let resp: serde_json::Value = get_http()
                .post("https://music.163.com/api/linux/forward")
                .headers(reqwest::header::HeaderMap::from_iter(
                    wy_headers().into_iter().map(|(k, v)| (k.parse().unwrap(), v.parse().unwrap())),
                ))
                .header("Content-Type", "application/x-www-form-urlencoded")
                .body(body)
                .send().await.map_err(|e| e.to_string())?
                .json_sanitized().await?;

            let code = resp.get("code").and_then(|v| v.as_i64()).unwrap_or(0);
            if code != 200 { return Err(format!("WY hot tag API error: code={}", code)); }

            let tags = resp.get("tags").and_then(|v| v.as_array()).cloned().unwrap_or_default();
            let hot: Vec<serde_json::Value> = tags.iter().filter_map(|item| {
                item.get("playlistTag").and_then(|pt| {
                    let name = pt.get("name")?.as_str()?.to_string();
                    Some(serde_json::json!({ "id": name.clone(), "name": name, "source": "wy" }))
                })
            }).collect();
            Ok::<_, String>(hot)
        }
    );

    let tags = tags_result.unwrap_or_default();
    let hot_tag = hot_result.unwrap_or_default();
    Ok(serde_json::json!({ "tags": tags, "hotTag": hot_tag, "source": "wy" }))
}

pub async fn get_category_playlists(args: serde_json::Value) -> Result<serde_json::Value, String> {
    let tag_id = get_str(&args, "tagId").to_string();
    let page = get_u64(&args, "page", 1);
    let limit = get_u64(&args, "limit", 30);

    let cat = if tag_id.is_empty() { "全部".to_string() } else { tag_id };
    let body = format!(
        "cat={}&order=hot&limit={}&offset={}&total=true",
        urlencoding::encode(&cat), limit, limit * (page - 1)
    );

    let resp: serde_json::Value = get_http()
        .post("https://music.163.com/api/playlist/list")
        .headers(reqwest::header::HeaderMap::from_iter(
            wy_headers().into_iter().map(|(k, v)| (k.parse().unwrap(), v.parse().unwrap())),
        ))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send().await.map_err(|e| e.to_string())?
        .json_sanitized().await?;

    let code = resp.get("code").and_then(|v| v.as_i64()).unwrap_or(0);
    if code != 200 { return Err(format!("WY playlist list API error: code={}", code)); }

    let total = resp.get("total").and_then(|v| v.as_i64()).unwrap_or(0);
    let raw_list = resp.get("playlists").and_then(|v| v.as_array()).cloned().unwrap_or_default();
    let list = wy_filter_playlist(&raw_list);

    Ok(serde_json::to_value(PlaylistResult {
        list, all_page: (total as f64 / limit as f64).ceil() as i64,
        limit: limit as i64, total, source: "wy".into(),
    }).unwrap())
}

pub async fn get_hot_songlist(args: serde_json::Value) -> Result<serde_json::Value, String> {
    get_category_playlists(args).await
}

pub async fn get_leaderboards(_args: serde_json::Value) -> Result<serde_json::Value, String> {
    let body = linuxapi_form(&serde_json::json!({
        "method": "POST",
        "url": "https://music.163.com/api/toplist",
        "params": {}
    }));

    let resp: serde_json::Value = get_http()
        .post("https://music.163.com/api/linux/forward")
        .headers(reqwest::header::HeaderMap::from_iter(
            wy_headers().into_iter().map(|(k, v)| (k.parse().unwrap(), v.parse().unwrap())),
        ))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send().await.map_err(|e| e.to_string())?
        .json_sanitized().await?;

    let code = resp.get("code").and_then(|v| v.as_i64()).unwrap_or(0);
    if code != 200 { return Err(format!("WY toplist API error: code={}", code)); }

    let raw_list = resp.get("list").and_then(|v| v.as_array()).cloned().unwrap_or_default();
    let list: Vec<serde_json::Value> = raw_list.iter().map(|item| {
        serde_json::json!({
            "id": format!("wy__{}", item.get("id").and_then(|v| v.as_i64()).unwrap_or(0)),
            "name": item.get("name").and_then(|v| v.as_str()).unwrap_or(""),
            "img": item.get("coverImgUrl").and_then(|v| v.as_str()).unwrap_or(""),
            "pic": item.get("coverImgUrl").and_then(|v| v.as_str()).unwrap_or(""),
            "listen": item.get("playCount").cloned().unwrap_or(serde_json::Value::Null),
            "update_frequency": item.get("updateFrequency").and_then(|v| v.as_str()).unwrap_or(""),
            "source": "wy"
        })
    }).collect();

    Ok(serde_json::json!({ "list": list, "source": "wy" }))
}

pub async fn get_playlist_detail(args: serde_json::Value) -> Result<serde_json::Value, String> {
    let raw_id = args.get("id").map(|v| match v {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Number(n) => n.to_string(),
        _ => String::new(),
    }).unwrap_or_default();
    let page = get_u64(&args, "page", 1);
    let limit: i64 = 1000;
    let start = (limit * (page as i64 - 1)) as usize;

    let id = parse_wy_id(&raw_id);

    let body = linuxapi_form(&serde_json::json!({
        "method": "POST",
        "url": "https://music.163.com/api/v3/playlist/detail",
        "params": { "id": id, "n": 100000, "s": 8 }
    }));

    let resp: serde_json::Value = get_http()
        .post("https://music.163.com/api/linux/forward")
        .headers(reqwest::header::HeaderMap::from_iter(
            wy_headers().into_iter().map(|(k, v)| (k.parse().unwrap(), v.parse().unwrap())),
        ))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send().await.map_err(|e| e.to_string())?
        .json_sanitized().await?;

    let code = resp.get("code").and_then(|v| v.as_i64()).unwrap_or(0);
    if code != 200 { return Err(format!("WY playlist detail API error: code={}", code)); }

    let playlist = resp.get("playlist").cloned().unwrap_or(serde_json::json!({}));
    let track_ids: Vec<serde_json::Value> = playlist.get("trackIds").and_then(|v| v.as_array()).cloned().unwrap_or_default();
    let total = track_ids.len() as i64;
    let privileges = resp.get("privileges").and_then(|v| v.as_array()).cloned().unwrap_or_default();
    let tracks = playlist.get("tracks").and_then(|v| v.as_array()).cloned().unwrap_or_default();

    let list: Vec<MusicItem> = if tracks.len() == privileges.len() {
        parse_wy_tracks(&tracks, &privileges)
    } else if !track_ids.is_empty() {
        let song_ids: Vec<i64> = track_ids.iter().skip(start).take(limit as usize)
            .filter_map(|tid| tid.get("id").and_then(|v| v.as_i64()))
            .collect();
        fetch_wy_song_details(&song_ids).await.unwrap_or_default()
    } else {
        vec![]
    };

    Ok(serde_json::json!({
        "list": list, "info": {
            "play_count": playlist.get("playCount").cloned().unwrap_or(serde_json::Value::Null),
            "name": playlist.get("name").and_then(|v| v.as_str()).unwrap_or(""),
            "img": playlist.get("coverImgUrl").and_then(|v| v.as_str()).unwrap_or(""),
            "pic": playlist.get("coverImgUrl").and_then(|v| v.as_str()).unwrap_or(""),
            "desc": playlist.get("description").and_then(|v| v.as_str()).unwrap_or(""),
            "author": playlist.get("creator").and_then(|c| c.get("nickname")).and_then(|v| v.as_str()).unwrap_or("")
        },
        "allPage": (total as f64 / limit as f64).ceil() as i64, "limit": limit, "total": total, "source": "wy"
    }))
}

pub async fn get_leaderboard_detail(args: serde_json::Value) -> Result<serde_json::Value, String> {
    get_playlist_detail(args).await
}

pub async fn search_playlist(args: serde_json::Value) -> Result<serde_json::Value, String> {
    let keyword = get_str(&args, "keyword");
    let page = get_u64(&args, "page", 1);
    let limit = get_u64(&args, "limit", 30);

    if keyword.is_empty() {
        return Ok(serde_json::json!({ "list": [], "allPage": 0, "limit": limit, "total": 0, "source": "wy" }));
    }

    let body = eapi_form("/api/cloudsearch/pc", &serde_json::json!({
        "s": keyword, "type": 1000, "limit": limit,
        "total": page == 1, "offset": limit * (page - 1)
    }));

    let resp: serde_json::Value = get_http()
        .post("http://interface3.music.163.com/eapi/cloudsearch/pc")
        .headers(reqwest::header::HeaderMap::from_iter(
            wy_headers().into_iter().map(|(k, v)| (k.parse().unwrap(), v.parse().unwrap())),
        ))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send().await.map_err(|e| e.to_string())?
        .json_sanitized().await?;

    let code = resp.get("code").and_then(|v| v.as_i64()).unwrap_or(0);
    if code != 200 { return Err(format!("WY search playlist API error: code={}", code)); }

    let result = resp.get("result").cloned().unwrap_or(serde_json::json!({}));
    let total = result.get("playlistCount").and_then(|v| v.as_i64()).unwrap_or(0);
    let raw_list = result.get("playlists").and_then(|v| v.as_array()).cloned().unwrap_or_default();
    let list = wy_filter_playlist(&raw_list);

    Ok(serde_json::to_value(PlaylistResult {
        list, all_page: (total as f64 / limit as f64).ceil() as i64,
        limit: limit as i64, total, source: "wy".into(),
    }).unwrap())
}

fn wy_filter_playlist(raw: &[serde_json::Value]) -> Vec<PlaylistItem> {
    raw.iter().map(|item| PlaylistItem {
        id: item.get("id").cloned().unwrap_or(serde_json::Value::Null),
        name: item.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string(),
        img: item.get("coverImgUrl").and_then(|v| v.as_str()).unwrap_or("").to_string(),
        source: "wy".into(),
        desc: item.get("description").and_then(|v| v.as_str()).unwrap_or("").to_string(),
        play_count: item.get("playCount").cloned().unwrap_or(serde_json::Value::Null),
        author: item.get("creator").and_then(|c| c.get("nickname")).and_then(|v| v.as_str()).unwrap_or("").to_string(),
        total: serde_json::Value::Null,
    }).collect()
}

fn parse_wy_tracks(tracks: &[serde_json::Value], privileges: &[serde_json::Value]) -> Vec<MusicItem> {
    tracks.iter().enumerate().map(|(idx, item)| {
        let privilege = privileges.get(idx)
            .or_else(|| privileges.iter().find(|p| p.get("id") == item.get("id")))
            .cloned().unwrap_or(serde_json::json!({}));

        let id = item.get("id").cloned().unwrap_or(serde_json::Value::Null);
        let name = item.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let singers = item.get("ar").and_then(|v| v.as_array()).cloned().unwrap_or_default();
        let singer = format_singer(&singers);
        let album_name = item.get("al").and_then(|a| a.get("name")).and_then(|v| v.as_str()).unwrap_or("").to_string();
        let album_id = item.get("al").and_then(|a| a.get("id")).cloned().unwrap_or(serde_json::Value::Null);
        let img = item.get("al").and_then(|a| a.get("picUrl")).and_then(|v| v.as_str()).unwrap_or("").to_string();
        let dt = item.get("dt").and_then(|v| v.as_i64()).unwrap_or(0);

        let types = parse_quality_types(&privilege);

        MusicItem {
            songmid: id, singer, name, album_name, album_id,
            source: "wy".into(), interval: format_play_time(dt / 1000), img, lrc: None,
            types: Some(types), types_map: None, type_url: Some(serde_json::json!({})), hash: None,
            song_id: None, str_media_mid: None, album_mid: None,
        copyright_id: None, lrc_url: None, mrc_url: None, trc_url: None,
        singer_id: None,
        }
    }).collect()
}

async fn fetch_wy_song_details(song_ids: &[i64]) -> Result<Vec<MusicItem>, String> {
    if song_ids.is_empty() { return Ok(vec![]); }

    let body = eapi_form("/api/v3/song/detail", &serde_json::json!({ "ids": song_ids }));

    let resp: serde_json::Value = get_http()
        .post("http://interface3.music.163.com/eapi/v3/song/detail")
        .headers(reqwest::header::HeaderMap::from_iter(
            wy_headers().into_iter().map(|(k, v)| (k.parse().unwrap(), v.parse().unwrap())),
        ))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send().await.map_err(|e| e.to_string())?
        .json_sanitized().await?;

    let songs = resp.get("songs").and_then(|v| v.as_array()).cloned().unwrap_or_default();
    let privileges = resp.get("privileges").and_then(|v| v.as_array()).cloned().unwrap_or_default();

    Ok(parse_wy_tracks(&songs, &privileges))
}
