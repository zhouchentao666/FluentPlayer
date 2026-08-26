use super::helpers::*;
use crate::music_sdk::client::MusicItem;

pub async fn get_artist_list(args: serde_json::Value) -> Result<serde_json::Value, String> {
    let category = get_u64(&args, "category", 0);
    let page = get_u64(&args, "page", 1);
    let limit = get_u64(&args, "limit", 60);

    let url = format!(
        "https://kuwo.cn/api/www/artist/artistInfo?category={}&pn={}&rn={}&httpsStatus=1",
        category, page, limit
    );

    let resp: serde_json::Value = get_http()
        .get(&url)
        .header("Referer", "https://kuwo.cn/singers")
        .header("User-Agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/139.0.0.0 Safari/537.36")
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;

    let code = resp.get("code").and_then(|v| v.as_i64()).unwrap_or(0);
    if code != 200 {
        return Err(format!("KW artist list API error: code={}", code));
    }

    let data = resp.get("data").cloned().unwrap_or(serde_json::json!({}));
    let total: i64 = data
        .get("total")
        .and_then(|v| v.as_str().and_then(|s| s.parse().ok()).or_else(|| v.as_i64()))
        .unwrap_or(0);
    let raw_list = data
        .get("artistList")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    let list: Vec<serde_json::Value> = raw_list
        .iter()
        .map(|item| {
            let avatar = item
                .get("pic300")
                .and_then(|v| v.as_str())
                .or_else(|| item.get("pic").and_then(|v| v.as_str()))
                .unwrap_or("")
                .to_string();
            serde_json::json!({
                "id": item.get("id"),
                "name": item.get("name").and_then(|v| v.as_str()).unwrap_or(""),
                "aartist": item.get("aartist").and_then(|v| v.as_str()).unwrap_or(""),
                "avatar": avatar,
                "fans": item.get("artistFans"),
                "albumNum": item.get("albumNum"),
                "musicNum": item.get("musicNum"),
                "source": "kw"
            })
        })
        .collect();

    Ok(serde_json::json!({
        "list": list,
        "total": total,
        "allPage": (total as f64 / limit as f64).ceil() as i64,
        "limit": limit as i64,
        "source": "kw"
    }))
}

pub async fn get_singer_song_list(args: serde_json::Value) -> Result<serde_json::Value, String> {
    let id = args
        .get("id")
        .map(|v| match v {
            serde_json::Value::String(s) => s.clone(),
            serde_json::Value::Number(n) => n.to_string(),
            _ => String::new(),
        })
        .unwrap_or_default();
    if id.is_empty() {
        return Err("KW singer song list: missing id".into());
    }

    let page = get_u64(&args, "page", 1);
    let limit = get_u64(&args, "limit", 30);

    let url = format!(
        "https://kuwo.cn/api/www/artist/artistMusic?artistid={}&pn={}&rn={}&httpsStatus=1",
        id, page, limit
    );

    let resp: serde_json::Value = get_http()
        .get(&url)
        .header("Referer", "https://kuwo.cn/singers")
        .header("User-Agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/139.0.0.0 Safari/537.36")
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;

    let code = resp.get("code").and_then(|v| v.as_i64()).unwrap_or(0);
    if code != 200 {
        return Err(format!("KW singer song list API error: code={}", code));
    }

    let data = resp.get("data").cloned().unwrap_or(serde_json::json!({}));
    let total: i64 = data
        .get("total")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);
    let raw_list = data
        .get("list")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    let singer_id_val = id.clone();
    let list: Vec<MusicItem> = raw_list
        .iter()
        .filter_map(|item| {
            let mut music = parse_music_item(item)?;
            music.singer_id = Some(singer_id_val.clone());
            Some(music)
        })
        .collect();

    Ok(serde_json::json!({
        "list": list,
        "allPage": (total as f64 / limit as f64).ceil() as i64,
        "limit": limit as i64,
        "total": total,
        "source": "kw"
    }))
}
