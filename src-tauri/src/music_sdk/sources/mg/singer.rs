use super::helpers::*;
use crate::music_sdk::client::MusicItem;

pub async fn get_artist_list(args: serde_json::Value) -> Result<serde_json::Value, String> {
    let tab = get_str(&args, "tab");
    let tab = if tab.is_empty() { "huayu-nan" } else { tab };

    let url = format!(
        "https://app.c.nf.migu.cn/pc/bmw/singer-index/list/v1.0?tab={}",
        tab
    );

    let resp: serde_json::Value = get_http()
        .get(&url)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;

    let code = resp.get("code").and_then(|v| v.as_str()).unwrap_or("");
    if code != "000000" {
        return Err(format!("MG artist list API error: code={}", code));
    }

    let data = resp.get("data").cloned().unwrap_or(serde_json::json!({}));
    let raw_list = data
        .get("contents")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    let list: Vec<serde_json::Value> = raw_list
        .iter()
        .filter(|item| {
            item.get("resType").and_then(|v| v.as_str()).unwrap_or("") == "2002"
        })
        .map(|item| {
            let id = item.get("resId").and_then(|v| v.as_str()).unwrap_or("");
            let img = item.get("img").and_then(|v| v.as_str()).unwrap_or("");
            // replace webp with jpg for broader compatibility
            let avatar = if img.ends_with(".webp") {
                format!("{}.jpg", &img[..img.len() - 5])
            } else {
                img.to_string()
            };
            serde_json::json!({
                "id": id,
                "name": item.get("txt").and_then(|v| v.as_str()).unwrap_or(""),
                "avatar": avatar,
                "source": "mg"
            })
        })
        .collect();

    Ok(serde_json::json!({
        "list": list,
        "total": list.len() as i64,
        "allPage": 1,
        "limit": list.len() as i64,
        "source": "mg"
    }))
}

pub async fn get_singer_song_list(args: serde_json::Value) -> Result<serde_json::Value, String> {
    let id = get_str(&args, "id");
    if id.is_empty() {
        return Err("MG singer song list: missing id".into());
    }

    let page = get_u64(&args, "page", 1);
    let limit = get_u64(&args, "limit", 30);

    let url = format!(
        "https://app.c.nf.migu.cn/pc/bmw/singer/song/v1.0?pageNo={}&singerId={}&type=1",
        page, id
    );

    let resp: serde_json::Value = get_http()
        .get(&url)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;

    let code = resp.get("code").and_then(|v| v.as_str()).unwrap_or("");
    if code != "000000" {
        return Err(format!("MG singer song list API error: code={}", code));
    }

    let data = resp.get("data").cloned().unwrap_or(serde_json::json!({}));
    let raw_list = data
        .get("contents")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    let singer_id_val = id.to_string();
    let mut all_songs: Vec<MusicItem> = Vec::new();

    for container in &raw_list {
        // Handle nested contents (ZJ-Singer-Song-Scroll wrapping items)
        if let Some(inner) = container.get("contents").and_then(|v| v.as_array()) {
            for item in inner {
                let song_item = item.get("songItem").cloned().unwrap_or(serde_json::json!({}));
                if let Some(mut music) = mg_parse_music_item(&song_item) {
                    music.singer_id = Some(singer_id_val.clone());
                    all_songs.push(music);
                }
            }
        } else {
            // Direct item without nesting
            let song_item = container.get("songItem").cloned().unwrap_or(serde_json::json!({}));
            if let Some(mut music) = mg_parse_music_item(&song_item) {
                music.singer_id = Some(singer_id_val.clone());
                all_songs.push(music);
            }
        }
    }

    let total = all_songs.len() as i64;
    Ok(serde_json::json!({
        "list": all_songs,
        "allPage": (total as f64 / limit as f64).ceil() as i64,
        "limit": limit as i64,
        "total": total,
        "source": "mg"
    }))
}
