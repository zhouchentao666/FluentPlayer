use super::helpers::*;
use crate::music_sdk::client::ResponseExt;
use super::crypto::{eapi_form, weapi_form};

pub async fn search_music(args: serde_json::Value) -> Result<serde_json::Value, String> {
    let keyword = get_str(&args, "keyword");
    let page = get_u64(&args, "page", 1);
    let limit = get_u64(&args, "limit", 30);
    if keyword.is_empty() {
        return Ok(serde_json::json!({ "list": [], "allPage": 0, "limit": limit, "total": 0, "source": "wy" }));
    }

    let body = eapi_form("/api/search/song/list/page", &serde_json::json!({
        "keyword": keyword, "needCorrect": "1", "channel": "typing",
        "offset": limit * (page - 1), "scene": "normal", "total": page == 1, "limit": limit
    }));

    let resp: serde_json::Value = get_http()
        .post("http://interface3.music.163.com/eapi/search/song/list/page")
        .header("User-Agent", "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/60.0.3112.90 Safari/537.36")
        .header("Origin", "https://music.163.com")
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send().await.map_err(|e| e.to_string())?
        .json_sanitized().await?;

    let code = resp.get("code").and_then(|v| v.as_i64()).unwrap_or(0);
    if code != 200 {
        return Err(format!("WY music search API error: code={}", code));
    }

    let data = resp.get("data").cloned().unwrap_or(serde_json::json!({}));
    let total = data.get("totalCount").and_then(|v| v.as_i64()).unwrap_or(0);
    let resources = data.get("resources").and_then(|v| v.as_array()).cloned().unwrap_or_default();

    let mut list = Vec::new();
    for res in &resources {
        let item = res.get("baseInfo").and_then(|b| b.get("simpleSongData")).cloned().unwrap_or(serde_json::json!({}));

        let id = item.get("id").and_then(|v| v.as_i64()).unwrap_or(0);
        let name = item.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let singers = item.get("ar").and_then(|v| v.as_array()).cloned().unwrap_or_default();
        let singer = format_singer(&singers);
        let album_name = item.get("al").and_then(|a| a.get("name")).and_then(|v| v.as_str()).unwrap_or("").to_string();
        let album_id = item.get("al").and_then(|a| a.get("id")).and_then(|v| v.as_i64()).unwrap_or(0);
        let img = item.get("al").and_then(|a| a.get("picUrl")).and_then(|v| v.as_str()).unwrap_or("").to_string();
        let dt = item.get("dt").and_then(|v| v.as_i64()).unwrap_or(0);

        // Fetch quality detail
        let detail_body = get_http()
            .get(format!("https://music.163.com/api/song/music/detail/get?songId={}", id))
            .header("User-Agent", "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/60.0.3112.90 Safari/537.36")
            .header("Origin", "https://music.163.com")
            .send().await;

        let (types, types_map) = if let Ok(resp) = detail_body {
            if let Ok(json) = resp.json::<serde_json::Value>().await {
                if json.get("code").and_then(|v| v.as_i64()).unwrap_or(0) == 200 {
                    let song_data = json.get("data").cloned().unwrap_or(serde_json::json!({}));
                    parse_quality_types_from_detail(&song_data)
                } else {
                    (Vec::new(), std::collections::HashMap::new())
                }
            } else {
                (Vec::new(), std::collections::HashMap::new())
            }
        } else {
            (Vec::new(), std::collections::HashMap::new())
        };

        let singer_id = singers.first()
            .and_then(|s| s.get("id"))
            .and_then(|v| v.as_i64())
            .map(|id| id.to_string());

        list.push(serde_json::json!({
            "songmid": id, "singer": singer, "name": name,
            "albumName": album_name, "albumId": album_id,
            "source": "wy", "interval": format_play_time(dt / 1000),
            "img": img, "types": types, "_types": types_map, "typeUrl": {},
            "singerId": singer_id,
        }));
    }

    Ok(serde_json::json!({
        "list": list,
        "allPage": (total as f64 / limit as f64).ceil() as i64,
        "limit": limit, "total": total, "source": "wy"
    }))
}

pub async fn tip_search(args: serde_json::Value) -> Result<serde_json::Value, String> {
    let keyword = get_str(&args, "keyword");
    if keyword.is_empty() {
        return Ok(serde_json::json!({ "list": [] }));
    }

    let body = weapi_form(&serde_json::json!({ "s": keyword }));

    let resp: serde_json::Value = get_http()
        .post("https://music.163.com/weapi/search/suggest/web")
        .header("Referer", "https://music.163.com/")
        .header("Origin", "https://music.163.com/")
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send().await.map_err(|e| e.to_string())?
        .json_sanitized().await?;

    let code = resp.get("code").and_then(|v| v.as_i64()).unwrap_or(0);
    if code != 200 {
        return Err("WY tip search API error".into());
    }

    let result = resp.get("result").and_then(|r| r.get("songs")).and_then(|v| v.as_array()).cloned().unwrap_or_default();
    let list: Vec<String> = result.iter().map(|info| {
        let name = info.get("name").and_then(|v| v.as_str()).unwrap_or("");
        let artists = info.get("artists").and_then(|v| v.as_array()).cloned().unwrap_or_default();
        let singer_names: Vec<&str> = artists.iter().filter_map(|a| a.get("name").and_then(|v| v.as_str())).collect();
        format!("{} - {}", name, singer_names.join("、"))
    }).collect();

    Ok(serde_json::json!({ "list": list, "source": "wy" }))
}

pub async fn hot_search(_args: serde_json::Value) -> Result<serde_json::Value, String> {
    let body = eapi_form("/api/search/chart/detail", &serde_json::json!({ "id": "HOT_SEARCH_SONG#@#" }));

    let resp: serde_json::Value = get_http()
        .post("http://interface3.music.163.com/eapi/search/chart/detail")
        .header("User-Agent", "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/60.0.3112.90 Safari/537.36")
        .header("Origin", "https://music.163.com")
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send().await.map_err(|e| e.to_string())?
        .json_sanitized().await?;

    let code = resp.get("code").and_then(|v| v.as_i64()).unwrap_or(0);
    if code != 200 {
        return Err("WY hot search API error".into());
    }

    let items = resp.get("data").and_then(|d| d.get("itemList")).and_then(|v| v.as_array()).cloned().unwrap_or_default();
    let list: Vec<String> = items.iter()
        .filter_map(|item| item.get("searchWord").and_then(|v| v.as_str()).map(|s| s.to_string()))
        .collect();

    Ok(serde_json::json!({ "source": "wy", "list": list }))
}
