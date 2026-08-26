use super::helpers::*;
use crate::music_sdk::client::ResponseExt;
use super::playback::get_batch_quality_info;
use crate::music_sdk::client::{MusicItem, PlaylistItem, PlaylistResult};

pub async fn get_playlist_tags(_args: serde_json::Value) -> Result<serde_json::Value, String> {
    let url = "http://www2.kugou.kugou.com/yueku/v9/special/getSpecial?is_smarty=1";
    let resp: serde_json::Value = get_http().get(url)
        .send().await.map_err(|e| e.to_string())?
        .json_sanitized().await?;

    let status = resp.get("status").and_then(|v| v.as_i64()).unwrap_or(0);
    if status != 1 {
        return Err("KuGou tags API error".into());
    }

    let data = resp.get("data").cloned().unwrap_or(serde_json::json!({}));

    let hot_tag_raw = data.get("hotTag").and_then(|v| v.get("data")).cloned().unwrap_or(serde_json::json!({}));
    let hot_tag: Vec<serde_json::Value> = if let Some(obj) = hot_tag_raw.as_object() {
        obj.values().filter_map(|tag| {
            let id = tag.get("special_id")?.as_str()?;
            let name = tag.get("special_name")?.as_str()?;
            Some(serde_json::json!({ "id": id, "name": name, "source": "kg" }))
        }).collect()
    } else {
        vec![]
    };

    let tagids_raw = data.get("tagids").cloned().unwrap_or(serde_json::json!({}));
    let tags: Vec<serde_json::Value> = if let Some(obj) = tagids_raw.as_object() {
        obj.iter().map(|(name, category_data)| {
            let tag_list: Vec<serde_json::Value> = category_data
                .get("data").and_then(|v| v.as_array()).cloned().unwrap_or_default()
                .iter().filter_map(|tag| {
                    let id = tag.get("id")?.as_i64()?;
                    let tag_name = tag.get("name")?.as_str()?;
                    Some(serde_json::json!({ "id": id.to_string(), "name": tag_name, "source": "kg" }))
                }).collect();
            serde_json::json!({ "name": name, "list": tag_list, "source": "kg" })
        }).collect()
    } else {
        vec![]
    };

    Ok(serde_json::json!({ "tags": tags, "hotTag": hot_tag, "source": "kg" }))
}

pub async fn get_category_playlists(args: serde_json::Value) -> Result<serde_json::Value, String> {
    let tag_id = get_str(&args, "tagId").to_string();
    let sort_id = get_str(&args, "sortId");
    let page = get_u64(&args, "page", 1);
    let limit = get_u64(&args, "limit", 30);

    let sort = if sort_id.is_empty() { "5" } else { sort_id };
    let url = format!(
        "http://www2.kugou.kugou.com/yueku/v9/special/getSpecial?is_ajax=1&cdn=cdn&t={}&c={}&p={}",
        sort, tag_id, page
    );

    let resp: serde_json::Value = get_http().get(&url)
        .send().await.map_err(|e| e.to_string())?
        .json_sanitized().await?;

    let status = resp.get("status").and_then(|v| v.as_i64()).unwrap_or(0);
    if status != 1 {
        return Err("KuGou playlist list API error".into());
    }

    let raw_list = resp.get("special_db").and_then(|v| v.as_array()).cloned().unwrap_or_default();
    let list: Vec<PlaylistItem> = raw_list.iter().map(|item| {
        let special_id = item.get("specialid").cloned().unwrap_or(serde_json::Value::Null);
        let play_count_val = item.get("total_play_count").or(item.get("play_count"))
            .map(parse_kugou_count).unwrap_or(serde_json::Value::Null);
        PlaylistItem {
            id: serde_json::json!(format!("id_{}", special_id.as_i64().unwrap_or(0))),
            name: item.get("specialname").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            img: item.get("img").or(item.get("imgurl")).and_then(|v| v.as_str()).unwrap_or("").to_string(),
            source: "kg".into(),
            desc: item.get("intro").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            play_count: play_count_val,
            author: item.get("nickname").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            total: item.get("song_count").cloned().unwrap_or(serde_json::Value::Null),
        }
    }).collect();

    let total = resp.get("data").and_then(|d| d.get("params")).and_then(|p| p.get("total"))
        .and_then(|v| v.as_i64()).unwrap_or(list.len() as i64);

    Ok(serde_json::to_value(PlaylistResult {
        list, all_page: (total as f64 / limit as f64).ceil() as i64,
        limit: limit as i64, total, source: "kg".into(),
    }).unwrap())
}

pub async fn get_hot_songlist(args: serde_json::Value) -> Result<serde_json::Value, String> {
    get_category_playlists(args).await
}

pub async fn get_leaderboards(_args: serde_json::Value) -> Result<serde_json::Value, String> {
    let url = "http://mobilecdnbj.kugou.com/api/v5/rank/list?version=9108&plat=0&showtype=2&parentid=0&apiver=6&area_code=1&withsong=1";
    let resp: serde_json::Value = get_http().get(url)
        .send().await.map_err(|e| e.to_string())?
        .json_sanitized().await?;

    let errcode = resp.get("errcode").and_then(|v| v.as_i64()).unwrap_or(-1);
    if errcode != 0 {
        return Err("KuGou rank list API error".into());
    }

    let raw_list = resp.get("data").and_then(|d| d.get("info")).and_then(|v| v.as_array()).cloned().unwrap_or_default();
    let list: Vec<serde_json::Value> = raw_list.iter().filter_map(|board| {
        let isvol = board.get("isvol").and_then(|v| v.as_i64()).unwrap_or(0);
        if isvol != 1 { return None; }

        let rankid = board.get("rankid")?.as_i64()?;
        let name = board.get("rankname")?.as_str()?;
        let imgurl = board.get("imgurl").and_then(|v| v.as_str()).unwrap_or("").replace("{size}", "512");
        let play_times = board.get("play_times").and_then(|v| v.as_i64()).unwrap_or(0);
        let update_frequency = board.get("update_frequency").and_then(|v| v.as_str()).unwrap_or("");

        Some(serde_json::json!({
            "id": format!("kg__{}", rankid),
            "name": name, "bangid": rankid.to_string(),
            "img": imgurl, "pic": imgurl,
            "listen": play_times, "update_frequency": update_frequency,
            "source": "kg"
        }))
    }).collect();

    Ok(serde_json::json!({ "list": list, "source": "kg" }))
}

pub async fn get_leaderboard_detail(args: serde_json::Value) -> Result<serde_json::Value, String> {
    let raw_id = args.get("id").map(|v| match v {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Number(n) => n.to_string(),
        _ => String::new(),
    }).unwrap_or_default();
    let page = get_u64(&args, "page", 1);
    let limit = get_u64(&args, "limit", 100);

    let rank_id = if raw_id.starts_with("kg__") {
        raw_id.replace("kg__", "")
    } else {
        raw_id
    };

    let url = format!(
        "http://mobilecdnbj.kugou.com/api/v3/rank/song?version=9108&ranktype=1&plat=0&pagesize={}&area_code=1&page={}&rankid={}&with_res_tag=0&show_portrait_mv=1",
        limit, page, rank_id
    );

    let resp: serde_json::Value = get_http().get(&url)
        .send().await.map_err(|e| e.to_string())?
        .json_sanitized().await?;

    let errcode = resp.get("errcode").and_then(|v| v.as_i64()).unwrap_or(-1);
    if errcode != 0 {
        return Err("KuGou rank song API error".into());
    }

    let data = resp.get("data").cloned().unwrap_or(serde_json::json!({}));
    let total = data.get("total").and_then(|v| v.as_i64()).unwrap_or(0);
    let raw_list = data.get("info").and_then(|v| v.as_array()).cloned().unwrap_or_default();

    let hashes: Vec<String> = raw_list.iter().filter_map(|item| {
        item.get("hash").and_then(|v| v.as_str()).map(|s| s.to_string())
    }).collect();

    let list = fetch_kg_song_details(&hashes).await.unwrap_or_default();

    Ok(serde_json::json!({
        "list": list, "info": serde_json::json!({}),
        "allPage": (total as f64 / limit as f64).ceil() as i64, "limit": limit as i64, "total": total, "source": "kg"
    }))
}

pub async fn get_playlist_detail(args: serde_json::Value) -> Result<serde_json::Value, String> {
    let raw_id = args.get("id").map(|v| match v {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Number(n) => n.to_string(),
        _ => String::new(),
    }).unwrap_or_default();
    let page = get_u64(&args, "page", 1);
    let limit = get_u64(&args, "limit", 30);

    let id = if raw_id.starts_with("id_") {
        raw_id.replace("id_", "")
    } else {
        raw_id
    };

    let url = format!("http://www2.kugou.kugou.com/yueku/v9/special/single/{}-5-9999.html", id);
    let resp = get_http().get(&url)
        .send().await.map_err(|e| e.to_string())?;
    let html = resp.text().await.map_err(|e| e.to_string())?;

    let name = extract_regex(&html, r#"name: "([^"]+)""#).unwrap_or_default();
    let img = extract_regex(&html, r#"pic: "([^"]+)""#).unwrap_or_default();

    let data_str = extract_regex(&html, r"global\.data = (\[.+?\]);").unwrap_or_default();
    let raw_data: Vec<serde_json::Value> = serde_json::from_str(&data_str).unwrap_or_default();

    let hashes: Vec<String> = raw_data.iter().filter_map(|item| {
        item.get("hash").and_then(|v| v.as_str()).map(|s| s.to_string())
    }).collect();

    let total = hashes.len() as i64;
    let start = ((page - 1) * limit) as usize;
    let end = (start + limit as usize).min(hashes.len());
    let page_hashes = if start < hashes.len() { &hashes[start..end] } else { &[] };

    let list = fetch_kg_song_details(page_hashes).await.unwrap_or_default();

    Ok(serde_json::json!({
        "list": list, "info": { "name": name, "img": img, "desc": "" },
        "allPage": (total as f64 / limit as f64).ceil() as i64, "limit": limit as i64, "total": total, "source": "kg"
    }))
}

async fn fetch_kg_song_details(hashes: &[String]) -> Result<Vec<MusicItem>, String> {
    if hashes.is_empty() {
        return Ok(vec![]);
    }

    let data_hashes: Vec<serde_json::Value> = hashes.iter().map(|h| serde_json::json!({ "hash": h })).collect();
    let body = serde_json::json!({
        "appid": 1005, "clientver": 11451, "mid": "1", "dfid": "-",
        "clienttime": chrono::Utc::now().timestamp_millis(),
        "key": "OIlwieks28dk2k092lksi2UIkp",
        "fields": "album_info,author_name,audio_info,ori_audio_name,base,songname",
        "data": data_hashes,
        "show_privilege": 1, "show_album_info": "1", "is_publish": "", "area_code": "1"
    });

    let resp: serde_json::Value = get_http()
        .post("http://gateway.kugou.com/v2/album_audio/audio")
        .header("KG-THash", "13a3164").header("KG-RC", "1")
        .header("KG-Fake", "0").header("KG-RF", "00869891")
        .header("User-Agent", "Android712-AndroidPhone-11451-376-0-FeeCacheUpdate-wifi")
        .header("x-router", "kmr.service.kugou.com")
        .json(&body)
        .send().await.map_err(|e| e.to_string())?
        .json_sanitized().await?;

    let data = resp.get("data").and_then(|v| v.as_array()).cloned().unwrap_or_default();

    let mut hashes: Vec<String> = Vec::new();
    let mut list: Vec<MusicItem> = data.iter().filter_map(|item| {
        let info = item.as_array()?.first()?;
        let audio_info = info.get("audio_info")?;
        let album_info = info.get("album_info");

        let hash = audio_info.get("hash").and_then(|v| v.as_str()).unwrap_or("").to_string();
        if hash.is_empty() { return None; }
        // Use audio_id as songmid (matches TS quality_detail.js filterData), NOT hash
        let songmid = audio_info.get("audio_id")
            .map(|v| match v {
                serde_json::Value::Number(n) => n.to_string(),
                serde_json::Value::String(s) => s.clone(),
                _ => hash.clone(),
            })
            .unwrap_or_else(|| hash.clone());
        let name = info.get("songname").or_else(|| info.get("ori_audio_name"))
            .and_then(|v| v.as_str()).unwrap_or("").to_string();
        let singer = info.get("author_name").and_then(|v| v.as_str()).unwrap_or("").to_string();

        let album_name = album_info.and_then(|a| a.get("album_name")).and_then(|v| v.as_str()).unwrap_or("").to_string();
        let album_id = album_info.and_then(|a| a.get("album_id")).cloned().unwrap_or(serde_json::Value::Null);
        let duration_ms = audio_info.get("timelength").and_then(|v| v.as_i64()).unwrap_or(0);
        let interval = format_play_time(duration_ms / 1000);

        // Try to extract cover from API response
        let img = album_info
            .and_then(|a| a.get("imgurl").or_else(|| a.get("img")).or_else(|| a.get("image")))
            .and_then(|v| v.as_str())
            .or_else(|| audio_info.get("img").and_then(|v| v.as_str()))
            .unwrap_or("")
            .to_string();

        hashes.push(hash.clone());

        Some(MusicItem {
            songmid: serde_json::Value::String(songmid), singer, name, album_name, album_id,
            source: "kg".into(), interval, img, lrc: None,
            types: Some(vec!["128k".into()]), types_map: None, type_url: Some(serde_json::json!({})),
            hash: Some(hash),
            song_id: None, str_media_mid: None, album_mid: None,
        copyright_id: None, lrc_url: None, mrc_url: None, trc_url: None,
        singer_id: None,
        })
    }).collect();

    // Batch fetch quality info
    let quality_map = get_batch_quality_info(&hashes).await.unwrap_or_default();

    // Backfill quality types
    for item in &mut list {
        if let Some(hash) = &item.hash {
            if let Some((types, types_map)) = quality_map.get(hash) {
                if !types.is_empty() {
                    item.types = Some(types.clone());
                }
                item.types_map = Some(types_map.clone());
            }
        }
    }

    Ok(list)
}
