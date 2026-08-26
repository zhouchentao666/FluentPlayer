use super::helpers::*;
use crate::music_sdk::client::ResponseExt;
use super::crypto::signature_params;
use super::helpers::decode_html;

/// Get newest comments for a song
pub async fn get_comment(args: serde_json::Value) -> Result<serde_json::Value, String> {
    let song_info = args.get("songInfo").cloned().unwrap_or(serde_json::json!({}));
    let hash = song_info.get("hash").and_then(|v| v.as_str()).unwrap_or("");
    let page = get_u64(&args, "page", 1);
    let limit = get_u64(&args, "limit", 20);

    let timestamp = chrono::Utc::now().timestamp();
    let params = format!(
        "dfid=0&mid=16249512204336365674023395779019&clienttime={}&uuid=0&extdata={}&appid=1005&code=fc4be23b4e972707f36b8a828a93ba8a&schash={}&clientver=11409&p={}&clienttoken=&pagesize={}&ver=10&kugouid=0",
        timestamp, hash, hash, page, limit
    );
    let sig = signature_params(&params, "android", "");

    let url = format!(
        "http://m.comment.service.kugou.com/r/v1/rank/newest?{}&signature={}",
        params, sig
    );

    let resp: serde_json::Value = get_http().get(&url)
        .header("User-Agent", "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/107.0.0.0 Safari/537.36 Edg/107.0.1418.24")
        .send().await.map_err(|e| e.to_string())?
        .json_sanitized().await?;

    let err_code = resp.get("err_code").and_then(|v| v.as_i64()).unwrap_or(-1);
    if err_code != 0 {
        return Err("KuGou comment API error".into());
    }

    let total = resp.get("count").and_then(|v| v.as_i64()).unwrap_or(0);
    let raw_list = resp.get("list").and_then(|v| v.as_array()).cloned().unwrap_or_default();
    let comments = filter_comments(&raw_list);

    Ok(serde_json::json!({
        "source": "kg",
        "comments": comments,
        "total": total,
        "page": page,
        "limit": limit,
        "maxPage": ((total as f64 / limit as f64).ceil() as i64).max(1)
    }))
}

/// Get hot/top-liked comments for a song
pub async fn get_hot_comment(args: serde_json::Value) -> Result<serde_json::Value, String> {
    let song_info = args.get("songInfo").cloned().unwrap_or(serde_json::json!({}));
    let hash = song_info.get("hash").and_then(|v| v.as_str()).unwrap_or("");
    let page = get_u64(&args, "page", 1);
    let limit = get_u64(&args, "limit", 20);

    let timestamp = chrono::Utc::now().timestamp();
    let params = format!(
        "dfid=0&mid=16249512204336365674023395779019&clienttime={}&uuid=0&extdata={}&appid=1005&code=fc4be23b4e972707f36b8a828a93ba8a&schash={}&clientver=11409&p={}&clienttoken=&pagesize={}&ver=10&kugouid=0",
        timestamp, hash, hash, page, limit
    );
    let sig = signature_params(&params, "android", "");

    let url = format!(
        "http://m.comment.service.kugou.com/r/v1/rank/topliked?{}&signature={}",
        params, sig
    );

    let resp: serde_json::Value = get_http().get(&url)
        .header("User-Agent", "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/107.0.0.0 Safari/537.36 Edg/107.0.1418.24")
        .send().await.map_err(|e| e.to_string())?
        .json_sanitized().await?;

    let err_code = resp.get("err_code").and_then(|v| v.as_i64()).unwrap_or(-1);
    if err_code != 0 {
        return Err("KuGou hot comment API error".into());
    }

    let total = resp.get("count").and_then(|v| v.as_i64()).unwrap_or(0);
    let raw_list = resp.get("list").and_then(|v| v.as_array()).cloned().unwrap_or_default();
    let comments = filter_comments(&raw_list);

    Ok(serde_json::json!({
        "source": "kg",
        "comments": comments,
        "total": total,
        "page": page,
        "limit": limit,
        "maxPage": ((total as f64 / limit as f64).ceil() as i64).max(1)
    }))
}

fn filter_comments(raw_list: &[serde_json::Value]) -> Vec<serde_json::Value> {
    raw_list.iter().map(|item| {
        let content = item.get("content").and_then(|v| v.as_str()).unwrap_or("");
        let atlist = item.get("atlist").and_then(|v| v.as_array());
        let text = if let Some(at_arr) = atlist {
            let mut t = content.to_string();
            for atobj in at_arr {
                let at_id = atobj.get("id").and_then(|v| v.as_str()).unwrap_or("");
                let at_name = atobj.get("name").and_then(|v| v.as_str()).unwrap_or("");
                t = t.replace(&format!("[at={}]", at_id), &format!("@{} ", at_name));
            }
            decode_html(&t)
        } else {
            decode_html(content)
        };

        let images: Vec<serde_json::Value> = item.get("images")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|i| i.get("url").map(|u| serde_json::json!(u))).collect())
            .unwrap_or_default();

        let like_num = item.get("like").and_then(|l| l.get("likenum")).cloned().unwrap_or(serde_json::json!(0));
        let addtime = item.get("addtime").cloned().unwrap_or(serde_json::Value::Null);

        let data = serde_json::json!({
            "id": item.get("id"),
            "text": text,
            "images": images,
            "time": addtime,
            "timeStr": format_time_str(item.get("addtime").and_then(|v| v.as_i64()).unwrap_or(0)),
            "userName": item.get("user_name"),
            "avatar": item.get("user_pic"),
            "userId": item.get("user_id"),
            "likedCount": like_num,
            "replyNum": item.get("reply_num"),
            "reply": []
        });

        // If this is a reply (has pcontent), nest it
        if let Some(pcontent) = item.get("pcontent").and_then(|v| v.as_str()) {
            if !pcontent.is_empty() {
                return serde_json::json!({
                    "id": item.get("id"),
                    "text": decode_html(pcontent),
                    "time": null,
                    "userName": item.get("puser"),
                    "avatar": null,
                    "userId": item.get("puser_id"),
                    "likedCount": null,
                    "replyNum": null,
                    "reply": [data]
                });
            }
        }
        data
    }).collect()
}

fn format_time_str(ts: i64) -> String {
    if ts == 0 { return String::new(); }
    let dt = chrono::DateTime::from_timestamp(ts, 0);
    match dt {
        Some(d) => d.format("%Y-%m-%d %H:%M").to_string(),
        None => String::new(),
    }
}
