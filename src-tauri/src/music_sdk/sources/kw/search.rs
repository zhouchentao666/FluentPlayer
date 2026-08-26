use super::helpers::*;
use crate::music_sdk::client::SearchResult;

pub async fn search(args: serde_json::Value) -> Result<serde_json::Value, String> {
    let keyword = get_str(&args, "keyword");
    let page = get_u64(&args, "page", 1);
    let limit = get_u64(&args, "limit", 30);
    if keyword.is_empty() {
        return empty_search("kw");
    }

    let url = format!(
        "http://search.kuwo.cn/r.s?client=kt&all={}&pn={}&rn={}&uid=794762570&ver=kwplayer_ar_9.2.2.1&vipver=1&show_copyright_off=1&newver=1&ft=music&cluster=0&strategy=2012&encoding=utf8&rformat=json&vermerge=1&mobi=1&issubtitle=1",
        urlencoding::encode(keyword), page - 1, limit
    );
    let resp = get_http()
        .get(&url)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let resp: serde_json::Value = super::helpers::parse_kuwo_response(resp).await?;

    let total: i64 = resp
        .get("TOTAL")
        .and_then(|v| v.as_str())
        .unwrap_or("0")
        .parse()
        .unwrap_or(0);
    let abslist = resp
        .get("abslist")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    let list: Vec<crate::music_sdk::client::MusicItem> = abslist
        .iter()
        .filter_map(|info| {
            let singer_id = info.get("ARTISTID")
                .or_else(|| info.get("artistid"))
                .and_then(|v| match v {
                    serde_json::Value::String(s) => Some(s.clone()),
                    serde_json::Value::Number(n) => Some(n.to_string()),
                    _ => None,
                });
            let mut item = parse_music_item(info)?;
            item.singer_id = singer_id;
            Some(item)
        })
        .collect();

    Ok(serde_json::to_value(SearchResult {
        list,
        all_page: (total as f64 / limit as f64).ceil() as i64,
        limit: limit as i64,
        total,
        source: "kw".into(),
    })
    .unwrap())
}

pub async fn tip_search(args: serde_json::Value) -> Result<serde_json::Value, String> {
    let keyword = get_str(&args, "keyword");
    if keyword.is_empty() {
        return Ok(serde_json::json!({ "list": [] }));
    }

    let url = format!("https://tips.kuwo.cn/t.s?corp=kuwo&newver=3&p2p=1&notrace=0&c=mbox&w={}&encoding=utf8&rformat=json", urlencoding::encode(keyword));
    let resp: serde_json::Value = get_http()
        .get(&url)
        .header("Referer", "http://www.kuwo.cn/")
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;

    let items = resp
        .get("WORDITEMS")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let songs: Vec<serde_json::Value> = items.iter().map(|item| {
        serde_json::json!({ "name": item.get("RELWORD").and_then(|v| v.as_str()).unwrap_or("") })
    }).collect();

    Ok(serde_json::json!({ "order": ["songs"], "songs": songs }))
}

pub async fn hot_search(_args: serde_json::Value) -> Result<serde_json::Value, String> {
    let url = "http://hotword.kuwo.cn/hotword.s?prod=kwplayer_ar_9.3.0.1&corp=kuwo&newver=2&vipver=9.3.0.1&source=kwplayer_ar_9.3.0.1_40.apk&p2p=1&notrace=0&uid=0&plat=kwplayer_ar&rformat=json&encoding=utf8&tabid=1";
    let resp: serde_json::Value = get_http()
        .get(url)
        .header("User-Agent", "Dalvik/2.1.0 (Linux; U; Android 9;)")
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;

    let list = resp
        .get("tagvalue")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let keywords: Vec<serde_json::Value> = list
        .iter()
        .filter_map(|item| {
            item.get("key")
                .and_then(|v| v.as_str())
                .map(|s| serde_json::json!(s))
        })
        .collect();

    Ok(serde_json::json!({ "source": "kw", "list": keywords }))
}
