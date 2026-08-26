use super::helpers::*;
use crate::music_sdk::client::ResponseExt;
use crate::music_sdk::client::{PlaylistItem, PlaylistResult};
use std::collections::HashSet;

fn filter_list_recursive(
    contents: &[serde_json::Value],
    list: &mut Vec<PlaylistItem>,
    ids: &mut HashSet<String>,
) {
    for item in contents {
        if let Some(sub) = item.get("contents").and_then(|v| v.as_array()) {
            filter_list_recursive(sub, list, ids);
        } else {
            let res_type = item.get("resType").and_then(|v| v.as_str()).unwrap_or("");
            if res_type != "2021" { continue; }
            let id = item.get("resId").and_then(|v| v.as_str()).unwrap_or("").to_string();
            if id.is_empty() || ids.contains(&id) { continue; }
            ids.insert(id.clone());
            let name = item.get("txt").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let img = item.get("img").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let desc = item.get("txt2").and_then(|v| v.as_str()).unwrap_or("").to_string();
            list.push(PlaylistItem {
                id: serde_json::json!(id),
                name,
                img,
                source: "mg".into(),
                desc,
                play_count: serde_json::Value::Null,
                author: String::new(),
                total: serde_json::Value::Null,
            });
        }
    }
}

const BOARD_LIST: &[(&str, &str)] = &[
    ("27553319", "新歌榜"),
    ("27186466", "热歌榜"),
    ("27553408", "原创榜"),
    ("75959118", "音乐风向榜"),
    ("76557036", "彩铃分贝榜"),
    ("76557745", "会员臻爱榜"),
    ("23189800", "港台榜"),
    ("23189399", "内地榜"),
    ("19190036", "欧美榜"),
    ("83176390", "国风金曲榜"),
];

pub async fn get_playlist_tags(_args: serde_json::Value) -> Result<serde_json::Value, String> {
    let url = "https://app.c.nf.migu.cn/pc/v1.0/template/musiclistplaza-taglist/release";
    let resp: serde_json::Value = get_http().get(url)
        .headers(mg_headers())
        .send().await.map_err(|e| e.to_string())?
        .json_sanitized().await?;

    let status = resp.get("code").and_then(|v| v.as_str()).unwrap_or("");
    if status != "000000" {
        return Err("MG tags API error".into());
    }

    let data = resp.get("data").and_then(|v| v.as_array()).cloned().unwrap_or_default();

    // First element: hot tags
    let hot_tag: Vec<serde_json::Value> = data.first()
        .and_then(|item| item.get("content"))
        .and_then(|c| c.as_array()).cloned().unwrap_or_default()
        .iter().filter_map(|tag| {
            let texts = tag.get("texts").and_then(|t| t.as_array())?;
            let name = texts.first()?.as_str()?;
            let id = texts.get(1)?.as_str()?;
            Some(serde_json::json!({
                "id": id.to_string(),
                "name": name.to_string(),
                "source": "mg"
            }))
        }).collect();

    // Remaining elements: category groups
    let tags: Vec<serde_json::Value> = data.iter().skip(1).filter_map(|item| {
        let title = item.get("header")?.get("title")?.as_str()?;
        let content = item.get("content")?.as_array()?;
        let tag_list: Vec<serde_json::Value> = content.iter().filter_map(|tag| {
            let texts = tag.get("texts")?.as_array()?;
            let name = texts.first()?.as_str()?;
            let id = texts.get(1)?.as_str()?;
            Some(serde_json::json!({
                "id": id.to_string(),
                "name": name.to_string(),
                "source": "mg"
            }))
        }).collect();
        Some(serde_json::json!({
            "name": title.to_string(),
            "list": tag_list,
            "source": "mg"
        }))
    }).collect();

    Ok(serde_json::json!({ "tags": tags, "hotTag": hot_tag, "source": "mg" }))
}

pub async fn get_category_playlists(args: serde_json::Value) -> Result<serde_json::Value, String> {
    let tag_id = get_str(&args, "tagId");
    let page = get_u64(&args, "page", 1);
    let limit = get_u64(&args, "limit", 30);

    let resp: serde_json::Value = if tag_id.is_empty() {
        let url = format!(
            "https://app.c.nf.migu.cn/pc/bmw/page-data/playlist-square-recommend/v1.0?templateVersion={}&pageNo={}",
            page, page
        );
        get_http().get(&url)
            .headers(mg_headers())
            .send().await.map_err(|e| e.to_string())?
            .json_sanitized().await?
    } else {
        let url = format!(
            "https://app.c.nf.migu.cn/pc/v1.0/template/musiclistplaza-listbytag/release?pageNumber={}&templateVersion=2&tagId={}",
            page, tag_id
        );
        get_http().get(&url)
            .headers(mg_headers())
            .send().await.map_err(|e| e.to_string())?
            .json_sanitized().await?
    };

    let code = resp.get("code").and_then(|v| v.as_str()).unwrap_or("");
    if code != "000000" {
        return Err("MG playlist list API error".into());
    }

    let data = resp.get("data").cloned().unwrap_or(serde_json::json!({}));
    let mut list: Vec<PlaylistItem> = Vec::new();

    // Format A: data.contents (recursive structure, resType: '2021')
    if let Some(contents) = data.get("contents").and_then(|v| v.as_array()) {
        let mut ids = std::collections::HashSet::new();
        filter_list_recursive(contents, &mut list, &mut ids);
    // Format B: data.contentItemList[1].itemList (flat structure)
    } else if let Some(item_list) = data.get("contentItemList")
        .and_then(|v| v.as_array())
        .and_then(|arr| arr.get(1))
        .and_then(|v| v.get("itemList"))
        .and_then(|v| v.as_array())
    {
        for item in item_list {
            let id = item.get("logEvent")
                .and_then(|l| l.get("contentId"))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let name = item.get("title").and_then(|v| v.as_str()).unwrap_or("");
            let img = item.get("imageUrl").and_then(|v| v.as_str()).unwrap_or("");
            if id.is_empty() { continue; }
            list.push(PlaylistItem {
                id: serde_json::json!(id),
                name: name.to_string(),
                img: img.to_string(),
                source: "mg".into(),
                desc: String::new(),
                play_count: serde_json::Value::Null,
                author: String::new(),
                total: serde_json::Value::Null,
            });
        }
    }

    let total = list.len() as i64 * page as i64;

    Ok(serde_json::to_value(PlaylistResult {
        list, all_page: (total as f64 / limit as f64).ceil() as i64,
        limit: limit as i64, total, source: "mg".into(),
    }).unwrap())
}

pub async fn get_leaderboards(_args: serde_json::Value) -> Result<serde_json::Value, String> {
    let list: Vec<serde_json::Value> = BOARD_LIST.iter().map(|(id, name)| {
        serde_json::json!({
            "id": format!("mg__{}", id),
            "name": name,
            "bangid": id,
            "source": "mg"
        })
    }).collect();

    Ok(serde_json::json!({ "list": list, "source": "mg" }))
}

pub async fn get_leaderboard_detail(args: serde_json::Value) -> Result<serde_json::Value, String> {
    let raw_id = args.get("id").map(|v| match v {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Number(n) => n.to_string(),
        _ => String::new(),
    }).unwrap_or_default();
    let page = get_u64(&args, "page", 1);
    let limit = get_u64(&args, "limit", 100);

    let column_id = if raw_id.starts_with("mg__") {
        raw_id.replace("mg__", "")
    } else {
        raw_id
    };

    let url = format!(
        "https://app.c.nf.migu.cn/MIGUM2.0/v1.0/content/querycontentbyId.do?columnId={}&needAll=0",
        column_id
    );

    let resp: serde_json::Value = get_http().get(&url)
        .headers(mg_headers())
        .send().await.map_err(|e| e.to_string())?
        .json_sanitized().await?;

    let code = resp.get("code").and_then(|v| v.as_str()).unwrap_or("");
    if code != "000000" {
        return Err("MG leaderboard detail API error".into());
    }

    let column_info = resp.get("columnInfo").cloned().unwrap_or(serde_json::json!({}));
    let contents = column_info.get("contents").and_then(|c| c.as_array()).cloned().unwrap_or_default();
    let total = contents.len() as i64;

    let start = ((page - 1) * limit) as usize;
    let end = (start + limit as usize).min(contents.len());
    let page_contents = if start < contents.len() && start < end {
        &contents[start..end]
    } else {
        &[]
    };

    let list: Vec<crate::music_sdk::client::MusicItem> = page_contents.iter()
        .filter_map(|item| item.get("objectInfo").cloned())
        .filter_map(|obj| mg_parse_music_item(&obj))
        .collect();

    Ok(serde_json::json!({
        "list": list, "info": serde_json::json!({}),
        "allPage": (total as f64 / limit as f64).ceil() as i64,
        "limit": limit as i64, "total": total, "source": "mg"
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

    if raw_id.is_empty() {
        return Ok(serde_json::json!({
            "list": [], "info": {}, "allPage": 0, "limit": limit as i64, "total": 0, "source": "mg"
        }));
    }

    // Get playlist info
    let info_url = format!(
        "https://c.musicapp.migu.cn/MIGUM3.0/resource/playlist/v2.0?playlistId={}",
        raw_id
    );
    let info_resp: serde_json::Value = get_http().get(&info_url)
        .headers(mg_headers())
        .send().await.map_err(|e| e.to_string())?
        .json_sanitized().await?;

    let info_data = info_resp.get("data").cloned().unwrap_or(serde_json::json!({}));
    let name = info_data.get("title").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let img = info_data.get("imgItem")
        .and_then(|i| i.get("img"))
        .and_then(|v| v.as_str())
        .unwrap_or("").to_string();
    let desc = info_data.get("summary").and_then(|v| v.as_str()).unwrap_or("").to_string();

    // Get playlist songs
    let url = format!(
        "https://app.c.nf.migu.cn/MIGUM3.0/resource/playlist/song/v2.0?pageNo={}&pageSize={}&playlistId={}",
        page, limit, raw_id
    );

    let resp: serde_json::Value = get_http().get(&url)
        .headers(mg_headers())
        .send().await.map_err(|e| e.to_string())?
        .json_sanitized().await?;

    let code = resp.get("code").and_then(|v| v.as_str()).unwrap_or("");
    if code != "000000" {
        return Err("MG playlist detail API error".into());
    }

    let data = resp.get("data").cloned().unwrap_or(serde_json::json!({}));
    let total = data.get("totalCount").and_then(|v| v.as_i64()).unwrap_or(0);
    let contents = data.get("songList").and_then(|v| v.as_array()).cloned().unwrap_or_default();

    let list: Vec<crate::music_sdk::client::MusicItem> = contents.iter()
        .filter_map(mg_parse_music_item)
        .collect();

    Ok(serde_json::json!({
        "list": list,
        "info": { "name": name, "img": img, "desc": desc },
        "allPage": (total as f64 / limit as f64).ceil() as i64,
        "limit": limit as i64, "total": total, "source": "mg"
    }))
}

pub async fn get_hot_songlist(args: serde_json::Value) -> Result<serde_json::Value, String> {
    get_category_playlists(args).await
}
