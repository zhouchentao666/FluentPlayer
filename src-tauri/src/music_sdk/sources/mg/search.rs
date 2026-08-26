use super::helpers::*;
use crate::music_sdk::client::ResponseExt;
use crate::music_sdk::client::{MusicItem, PlaylistItem, PlaylistResult, SearchResult};

pub async fn search_music(args: serde_json::Value) -> Result<serde_json::Value, String> {
    let keyword = get_str(&args, "keyword");
    let page = get_u64(&args, "page", 1);
    let limit = get_u64(&args, "limit", 20);
    if keyword.is_empty() {
        return Ok(serde_json::to_value(SearchResult {
            list: vec![], all_page: 0, limit: limit as i64, total: 0, source: "mg".into(),
        }).unwrap());
    }

    let time = chrono::Utc::now().timestamp_millis().to_string();
    let (sign, device_id) = create_signature(&time, keyword);

    let url = format!(
        "https://jadeite.migu.cn/music_search/v3/search/searchAll?isCorrect=0&isCopyright=1&searchSwitch=%7B%22song%22%3A1%2C%22album%22%3A0%2C%22singer%22%3A0%2C%22tagSong%22%3A1%2C%22mvSong%22%3A0%2C%22bestShow%22%3A1%2C%22songlist%22%3A0%2C%22lyricSong%22%3A0%7D&pageSize={}&text={}&pageNo={}&sort=0&sid=USS",
        limit, urlencoding::encode(keyword), page
    );

    let resp: serde_json::Value = get_http().get(&url)
        .header("uiVersion", "A_music_3.6.1")
        .header("deviceId", &device_id)
        .header("timestamp", &time)
        .header("sign", &sign)
        .header("channel", "0146921")
        .header("User-Agent", "Mozilla/5.0 (Linux; U; Android 11.0.0; zh-cn; MI 11 Build/OPR1.170623.032) AppleWebKit/534.30 (KHTML, like Gecko) Version/4.0 Mobile Safari/534.30")
        .send().await.map_err(|e| e.to_string())?
        .json_sanitized().await?;

    let code = resp.get("code").and_then(|v| v.as_str()).unwrap_or("");
    if code != "000000" {
        return Err(format!("MG search API error: code={}", code));
    }

    let song_data = resp.get("songResultData").cloned().unwrap_or(serde_json::json!({
        "resultList": [], "totalCount": 0
    }));
    let total = song_data.get("totalCount").and_then(|v| v.as_i64()).unwrap_or(0);
    let result_list = song_data.get("resultList").and_then(|v| v.as_array()).cloned().unwrap_or_default();

    let mut list = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for group in &result_list {
        let items = match group.as_array() {
            Some(arr) => arr,
            None => continue,
        };
        for data in items {
            let song_id = data.get("songId").and_then(|v| v.as_str()).unwrap_or("");
            let copyright_id = data.get("copyrightId").and_then(|v| v.as_str()).unwrap_or("");
            if song_id.is_empty() || copyright_id.is_empty() || seen.contains(copyright_id) { continue; }
            seen.insert(copyright_id.to_string());

            let (types, types_map) = parse_quality(data);

            let img = get_song_img(data);

            list.push(MusicItem {
                songmid: serde_json::json!(song_id),
                singer: data.get("singerList").and_then(|v| v.as_array())
                    .map(|arr| format_singer_from_list(arr, "name"))
                    .unwrap_or_default(),
                name: data.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                album_name: data.get("album").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                album_id: data.get("albumId").cloned().unwrap_or(serde_json::Value::Null),
                source: "mg".into(),
                interval: data.get("duration").and_then(|v| v.as_i64()).map(format_play_time).unwrap_or_default(),
                img, lrc: None,
                types: Some(types), types_map: Some(types_map), type_url: Some(serde_json::json!({})),
                hash: None, song_id: None, str_media_mid: None, album_mid: None,
        copyright_id: None, lrc_url: None, mrc_url: None, trc_url: None,
        singer_id: None,
            });
        }
    }

    Ok(serde_json::to_value(SearchResult {
        list, all_page: (total as f64 / limit as f64).ceil() as i64,
        limit: limit as i64, total, source: "mg".into(),
    }).unwrap())
}

pub async fn tip_search(args: serde_json::Value) -> Result<serde_json::Value, String> {
    let keyword = get_str(&args, "keyword");
    if keyword.is_empty() {
        return Ok(serde_json::json!({ "order": [], "songs": [], "artists": [] }));
    }

    let url = format!(
        "https://app.u.nf.migu.cn/pc/resource/content/tone_search_suggest/v1.0?text={}",
        urlencoding::encode(keyword)
    );

    let resp: serde_json::Value = get_http().get(&url)
        .header("Referer", "https://music.migu.cn/v3")
        .send().await.map_err(|e| e.to_string())?
        .json_sanitized().await?;

    let mut order = Vec::new();
    let songs: Vec<serde_json::Value> = resp.get("songList")
        .and_then(|v| v.as_array())
        .cloned().unwrap_or_default()
        .iter().filter_map(|info| {
            let name = info.get("songName")?.as_str()?;
            Some(serde_json::json!({ "name": name }))
        }).collect();
    if !songs.is_empty() { order.push("songs"); }

    let artists: Vec<serde_json::Value> = resp.get("singerList")
        .and_then(|v| v.as_array())
        .cloned().unwrap_or_default()
        .iter().filter_map(|info| {
            let name = info.get("singerName")?.as_str()?;
            Some(serde_json::json!({ "name": name }))
        }).collect();
    if !artists.is_empty() { order.push("artists"); }

    Ok(serde_json::json!({ "order": order, "songs": songs, "artists": artists }))
}

pub async fn hot_search(_args: serde_json::Value) -> Result<serde_json::Value, String> {
    let resp: serde_json::Value = get_http()
        .get("http://jadeite.migu.cn:7090/music_search/v3/search/hotword")
        .send().await.map_err(|e| e.to_string())?
        .json_sanitized().await?;

    let code = resp.get("code").and_then(|v| v.as_str()).unwrap_or("");
    if code != "000000" {
        return Err("MG hot search API error".into());
    }

    let list: Vec<serde_json::Value> = resp.get("data")
        .and_then(|d| d.get("hotwords"))
        .and_then(|h| h.as_array())
        .and_then(|a| a.first())
        .and_then(|item| item.get("hotwordList"))
        .and_then(|v| v.as_array())
        .cloned().unwrap_or_default()
        .iter().filter(|item| {
            item.get("resourceType").and_then(|v| v.as_str()).unwrap_or("") == "song"
        })
        .filter_map(|item| {
            let word = item.get("word")?.as_str()?;
            Some(serde_json::json!(word))
        }).collect();

    Ok(serde_json::json!({ "source": "mg", "list": list }))
}

pub async fn search_playlist(args: serde_json::Value) -> Result<serde_json::Value, String> {
    let keyword = get_str(&args, "keyword");
    let page = get_u64(&args, "page", 1);
    let limit = get_u64(&args, "limit", 30);

    let url = format!(
        "https://app.u.nf.migu.cn/pc/v1.0/content/search_all.do?text={}&pageNo={}&pageSize={}&searchSwitch=%7B%22songlist%22%3A1%7D",
        urlencoding::encode(keyword), page, limit
    );

    let resp: serde_json::Value = get_http().get(&url)
        .headers(mg_headers())
        .send().await.map_err(|e| e.to_string())?
        .json_sanitized().await?;

    let code = resp.get("code").and_then(|v| v.as_str()).unwrap_or("");
    if code != "000000" {
        return Err(format!("MG search playlist API error: code={}", code));
    }

    let data = resp.get("songListResultData").cloned().unwrap_or(serde_json::json!({}));
    let total: i64 = data.get("totalCount").and_then(|v| v.as_str()).unwrap_or("0").parse().unwrap_or(0);
    let raw_list = data.get("result").and_then(|v| v.as_array()).cloned().unwrap_or_default();

    let list: Vec<PlaylistItem> = raw_list.iter().map(|item| {
        PlaylistItem {
            id: item.get("id").cloned().unwrap_or(serde_json::Value::Null),
            name: item.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            img: item.get("musicListPicUrl").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            source: "mg".into(),
            desc: String::new(),
            play_count: item.get("playNum").cloned().unwrap_or(serde_json::Value::Null),
            author: String::new(),
            total: item.get("musicNum").cloned().unwrap_or(serde_json::Value::Null),
        }
    }).collect();

    Ok(serde_json::to_value(PlaylistResult {
        list, all_page: (total as f64 / limit as f64).ceil() as i64,
        limit: limit as i64, total, source: "mg".into(),
    }).unwrap())
}
