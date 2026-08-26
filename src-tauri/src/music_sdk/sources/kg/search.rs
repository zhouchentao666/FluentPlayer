use super::helpers::*;
use crate::music_sdk::client::ResponseExt;
use crate::music_sdk::client::{MusicItem, PlaylistItem, PlaylistResult, SearchResult};
use super::playback::get_batch_quality_info;

/// Song search using songsearch.kugou.com (matches JS musicSearch.js)
pub async fn search(args: serde_json::Value) -> Result<serde_json::Value, String> {
    let keyword = get_str(&args, "keyword");
    let page = get_u64(&args, "page", 1);
    let limit = get_u64(&args, "limit", 30);
    if keyword.is_empty() {
        return Ok(serde_json::to_value(SearchResult {
            list: vec![], all_page: 0, limit: limit as i64, total: 0, source: "kg".into(),
        }).unwrap());
    }

    let url = format!(
        "https://songsearch.kugou.com/song_search_v2?keyword={}&page={}&pagesize={}&userid=0&clientver=&platform=WebFilter&filter=2&iscorrection=1&privilege_filter=0&area_code=1",
        urlencoding::encode(keyword), page, limit
    );

    let resp: serde_json::Value = get_http().get(&url)
        .send().await.map_err(|e| e.to_string())?
        .json_sanitized().await?;

    let errcode = resp.get("errcode").and_then(|v| v.as_i64()).unwrap_or(0);
    let status = resp.get("status").and_then(|v| v.as_i64()).unwrap_or(1);
    if errcode != 0 && status != 1 {
        return Err("KuGou song search API error".into());
    }

    let data = resp.get("data").cloned().unwrap_or(serde_json::json!({}));
    let total = data.get("total").and_then(|v| v.as_i64()).unwrap_or(0);
    let raw_list = data.get("lists").and_then(|v| v.as_array()).cloned().unwrap_or_default();

    // Deduplicate by Audioid + FileHash
    let mut seen = std::collections::HashSet::new();
    let mut items: Vec<&serde_json::Value> = Vec::new();
    for item in &raw_list {
        let key = format!(
            "{}{}",
            item.get("Audioid").and_then(|v| v.as_i64()).unwrap_or(0),
            item.get("FileHash").and_then(|v| v.as_str()).unwrap_or("")
        );
        if seen.insert(key) {
            items.push(item);
            // Also include grouped items
            if let Some(grp) = item.get("Grp").and_then(|v| v.as_array()) {
                for child in grp {
                    let child_key = format!(
                        "{}{}",
                        child.get("Audioid").and_then(|v| v.as_i64()).unwrap_or(0),
                        child.get("FileHash").and_then(|v| v.as_str()).unwrap_or("")
                    );
                    if seen.insert(child_key) {
                        items.push(child);
                    }
                }
            }
        }
    }

    // Batch fetch quality info
    let hashes: Vec<String> = items.iter()
        .filter_map(|item| item.get("FileHash").and_then(|v| v.as_str()).map(|s| s.to_string()))
        .collect();
    let quality_map = get_batch_quality_info(&hashes).await.unwrap_or_default();

    let list: Vec<MusicItem> = items.iter().filter_map(|item| {
        let hash = item.get("FileHash")?.as_str()?.to_string();
        let songmid = item.get("Audioid").and_then(|v| v.as_i64()).unwrap_or(0).to_string();
        let name = decode_html(item.get("SongName").and_then(|v| v.as_str()).unwrap_or(""));
        let singer = item.get("Singers").and_then(|v| v.as_array())
            .map(|arr| arr.iter()
                .filter_map(|s| s.get("name").and_then(|n| n.as_str()))
                .collect::<Vec<_>>().join("、"))
            .unwrap_or_default();
        let album_name = item.get("AlbumName").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let album_id = item.get("AlbumID").cloned().unwrap_or(serde_json::Value::Null);
        let duration = item.get("Duration").and_then(|v| v.as_i64()).unwrap_or(0);
        let interval = format_play_time(duration);

        let quality_info = quality_map.get(&hash);
        let types: Vec<String> = quality_info
            .map(|q| q.0.clone())
            .unwrap_or_default();
        let types_map = quality_info.map(|q| q.1.clone());

        Some(MusicItem {
            songmid: serde_json::Value::String(songmid), singer, name, album_name, album_id,
            source: "kg".into(), interval, img: String::new(), lrc: None,
            types: Some(types), types_map, type_url: Some(serde_json::json!({})),
            hash: Some(hash),
            song_id: None, str_media_mid: None, album_mid: None,
        copyright_id: None, lrc_url: None, mrc_url: None, trc_url: None,
        singer_id: None,
        })
    }).collect();

    Ok(serde_json::to_value(SearchResult {
        list, all_page: (total as f64 / limit as f64).ceil() as i64,
        limit: limit as i64, total, source: "kg".into(),
    }).unwrap())
}

fn decode_html(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
}

/// Tip search (autocomplete)
pub async fn tip_search(args: serde_json::Value) -> Result<serde_json::Value, String> {
    let keyword = get_str(&args, "keyword");
    if keyword.is_empty() {
        return Ok(serde_json::json!({ "list": [] }));
    }

    let url = format!(
        "https://searchtip.kugou.com/getSearchTip?MusicTipCount=10&keyword={}",
        urlencoding::encode(keyword)
    );

    let resp: serde_json::Value = get_http().get(&url)
        .header("Referer", "https://www.kugou.com/")
        .send().await.map_err(|e| e.to_string())?
        .json_sanitized().await?;

    let data = resp.as_array().cloned().unwrap_or_default();

    let mut order = Vec::new();
    let mut songs = Vec::new();
    let mut albums = Vec::new();

    if let Some(song_data) = data.first() {
        let count = song_data.get("RecordCount").and_then(|v| v.as_i64()).unwrap_or(0);
        if count > 0 {
            order.push("songs".to_string());
            if let Some(records) = song_data.get("RecordDatas").and_then(|v| v.as_array()) {
                for info in records {
                    if let Some(hint) = info.get("HintInfo").and_then(|v| v.as_str()) {
                        songs.push(serde_json::json!({ "name": hint }));
                    }
                }
            }
        }
    }

    if data.len() > 2 {
        if let Some(album_data) = data.get(2) {
            let count = album_data.get("RecordCount").and_then(|v| v.as_i64()).unwrap_or(0);
            if count > 0 {
                order.push("albums".to_string());
                if let Some(records) = album_data.get("RecordDatas").and_then(|v| v.as_array()) {
                    for info in records {
                        if let Some(hint) = info.get("HintInfo").and_then(|v| v.as_str()) {
                            albums.push(serde_json::json!({ "name": hint }));
                        }
                    }
                }
            }
        }
    }

    Ok(serde_json::json!({ "order": order, "songs": songs, "albums": albums }))
}

/// Hot search keywords
pub async fn hot_search(_args: serde_json::Value) -> Result<serde_json::Value, String> {
    let url = "http://gateway.kugou.com/api/v3/search/hot_tab?signature=ee44edb9d7155821412d220bcaf509dd&appid=1005&clientver=10026&plat=0";

    let resp: serde_json::Value = get_http().get(url)
        .header("dfid", "1ssiv93oVqMp27cirf2CvoF1")
        .header("mid", "156798703528610303473757548878786007104")
        .header("clienttime", "1584257267")
        .header("x-router", "msearch.kugou.com")
        .header("User-Agent", "Android9-AndroidPhone-10020-130-0-searchrecommendprotocol-wifi")
        .header("kg-rc", "1")
        .send().await.map_err(|e| e.to_string())?
        .json_sanitized().await?;

    let errcode = resp.get("errcode").and_then(|v| v.as_i64()).unwrap_or(-1);
    if errcode != 0 {
        return Err("KuGou hot search API error".into());
    }

    let raw_list = resp.get("data").and_then(|d| d.get("list"))
        .and_then(|v| v.as_array()).cloned().unwrap_or_default();

    let mut keywords: Vec<serde_json::Value> = Vec::new();
    for item in &raw_list {
        if let Some(kw_list) = item.get("keywords").and_then(|v| v.as_array()) {
            for kw in kw_list {
                if let Some(keyword) = kw.get("keyword").and_then(|v| v.as_str()) {
                    keywords.push(serde_json::json!(keyword));
                }
            }
        }
    }

    Ok(serde_json::json!({ "source": "kg", "list": keywords }))
}

/// Playlist search (special search)
pub async fn search_playlist(args: serde_json::Value) -> Result<serde_json::Value, String> {
    let keyword = get_str(&args, "keyword");
    let page = get_u64(&args, "page", 1);
    let limit = get_u64(&args, "limit", 20);
    if keyword.is_empty() {
        return Ok(serde_json::to_value(PlaylistResult {
            list: vec![], all_page: 0, limit: limit as i64, total: 0, source: "kg".into(),
        }).unwrap());
    }

    let url = format!(
        "http://msearchretry.kugou.com/api/v3/search/special?keyword={}&page={}&pagesize={}&showtype=10&filter=0&version=7910&sver=2",
        urlencoding::encode(keyword), page, limit
    );

    let resp: serde_json::Value = get_http().get(&url)
        .send().await.map_err(|e| e.to_string())?
        .json_sanitized().await?;

    let errcode = resp.get("errcode").and_then(|v| v.as_i64()).unwrap_or(-1);
    if errcode != 0 {
        return Err("KuGou search API error".into());
    }

    let data = resp.get("data").cloned().unwrap_or(serde_json::json!({}));
    let total = data.get("total").and_then(|v| v.as_i64()).unwrap_or(0);
    let raw_list = data.get("info").and_then(|v| v.as_array()).cloned().unwrap_or_default();

    let list: Vec<PlaylistItem> = raw_list.iter().map(|item| {
        let special_id = item.get("specialid").cloned().unwrap_or(serde_json::Value::Null);
        PlaylistItem {
            id: serde_json::json!(format!("id_{}", special_id.as_i64().unwrap_or(0))),
            name: item.get("specialname").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            img: item.get("imgurl").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            source: "kg".into(),
            desc: item.get("intro").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            play_count: item.get("playcount").cloned().unwrap_or(serde_json::Value::Null),
            author: item.get("nickname").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            total: serde_json::Value::Null,
        }
    }).collect();

    Ok(serde_json::to_value(PlaylistResult {
        list, all_page: (total as f64 / limit as f64).ceil() as i64,
        limit: limit as i64, total, source: "kg".into(),
    }).unwrap())
}
