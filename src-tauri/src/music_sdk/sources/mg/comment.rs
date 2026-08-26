use super::helpers::*;
use crate::music_sdk::client::ResponseExt;

/// Get newest comments for a song
pub async fn get_comment(args: serde_json::Value) -> Result<serde_json::Value, String> {
    let song_info = args.get("songInfo").cloned().unwrap_or(serde_json::json!({}));
    let songmid = song_info.get("songmid")
        .map(|v| match v {
            serde_json::Value::String(s) => s.clone(),
            serde_json::Value::Number(n) => n.to_string(),
            _ => String::new(),
        })
        .unwrap_or_default();

    if songmid.is_empty() {
        return Err("No songmid provided".into());
    }

    let url = format!(
        "https://app.c.nf.migu.cn/MIGUM3.0/user/comment/stack/v1.0?pageSize=20&queryType=1&resourceId={}&resourceType=2&commentId=",
        songmid
    );

    let resp: serde_json::Value = get_http().get(&url)
        .header("User-Agent", "Mozilla/5.0 (iPhone; CPU iPhone OS 13_2_3 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/13.0.3 Mobile/15E148 Safari/604.1")
        .send().await.map_err(|e| e.to_string())?
        .json_sanitized().await?;

    let code = resp.get("code").and_then(|v| v.as_str()).unwrap_or("");
    if code != "000000" {
        return Err(format!("MG comment API error: code={}", code));
    }

    let data = resp.get("data").cloned().unwrap_or(serde_json::json!({}));
    let total = data.get("commentNums")
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(0);
    let raw_list = data.get("comments").and_then(|v| v.as_array()).cloned().unwrap_or_default();
    let comments = filter_new_comment(&raw_list);

    Ok(serde_json::json!({
        "source": "mg", "comments": comments, "total": total,
        "page": 1, "limit": 20,
        "maxPage": ((total as f64 / 20.0).ceil() as i64).max(1)
    }))
}

/// Get hot/top-liked comments for a song
pub async fn get_hot_comment(args: serde_json::Value) -> Result<serde_json::Value, String> {
    let song_info = args.get("songInfo").cloned().unwrap_or(serde_json::json!({}));
    let songmid = song_info.get("songmid")
        .map(|v| match v {
            serde_json::Value::String(s) => s.clone(),
            serde_json::Value::Number(n) => n.to_string(),
            _ => String::new(),
        })
        .unwrap_or_default();

    if songmid.is_empty() {
        return Err("No songmid provided".into());
    }

    let url = format!(
        "https://app.c.nf.migu.cn/MIGUM3.0/user/comment/stack/v1.0?pageSize=20&queryType=2&resourceId={}&resourceType=2&hotCommentStart=0",
        songmid
    );

    let resp: serde_json::Value = get_http().get(&url)
        .header("User-Agent", "Mozilla/5.0 (iPhone; CPU iPhone OS 13_2_3 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/13.0.3 Mobile/15E148 Safari/604.1")
        .send().await.map_err(|e| e.to_string())?
        .json_sanitized().await?;

    let code = resp.get("code").and_then(|v| v.as_str()).unwrap_or("");
    if code != "000000" {
        return Err(format!("MG hot comment API error: code={}", code));
    }

    let data = resp.get("data").cloned().unwrap_or(serde_json::json!({}));
    let total = data.get("cfgHotCount")
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(0);
    let raw_list = data.get("hotComments").and_then(|v| v.as_array()).cloned().unwrap_or_default();
    let comments = filter_new_comment(&raw_list);

    Ok(serde_json::json!({
        "source": "mg", "comments": comments, "total": total,
        "page": 1, "limit": 20,
        "maxPage": ((total as f64 / 20.0).ceil() as i64).max(1)
    }))
}

fn filter_new_comment(raw_list: &[serde_json::Value]) -> Vec<serde_json::Value> {
    raw_list.iter().map(|item| {
        let time = item.get("commentTime").and_then(|v| v.as_str()).unwrap_or("");
        let time_ms = chrono::NaiveDateTime::parse_from_str(time, "%Y-%m-%d %H:%M:%S")
            .ok()
            .map(|dt| dt.and_utc().timestamp_millis())
            .unwrap_or(0);
        let time_str = if time_ms > 0 {
            let dt = chrono::DateTime::from_timestamp_millis(time_ms);
            dt.map(|d| d.format("%Y-%m-%d %H:%M").to_string()).unwrap_or_default()
        } else {
            String::new()
        };

        let user = item.get("user").cloned().unwrap_or(serde_json::json!({}));
        let avatar = user.get("middleIcon")
            .or_else(|| user.get("bigIcon"))
            .or_else(|| user.get("smallIcon"))
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let reply_list = item.get("replyComments")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        let reply: Vec<serde_json::Value> = reply_list.iter().map(|c| {
            let reply_user = c.get("user").cloned().unwrap_or(serde_json::json!({}));
            let reply_avatar = reply_user.get("middleIcon")
                .or_else(|| reply_user.get("bigIcon"))
                .or_else(|| reply_user.get("smallIcon"))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            serde_json::json!({
                "id": c.get("replyId"),
                "text": c.get("replyInfo").and_then(|v| v.as_str()).unwrap_or(""),
                "time": c.get("replyTime").and_then(|v| v.as_str()).unwrap_or(""),
                "timeStr": null,
                "userName": reply_user.get("nickName").and_then(|v| v.as_str()).unwrap_or(""),
                "avatar": reply_avatar,
                "userId": reply_user.get("userId"),
                "likedCount": null,
                "replyNum": null
            })
        }).collect();

        serde_json::json!({
            "id": item.get("commentId"),
            "text": item.get("commentInfo").and_then(|v| v.as_str()).unwrap_or(""),
            "time": time,
            "timeStr": time_str,
            "userName": user.get("nickName").and_then(|v| v.as_str()).unwrap_or(""),
            "avatar": avatar,
            "userId": user.get("userId"),
            "likedCount": item.get("opNumItem").and_then(|o| o.get("thumbNum")),
            "replyNum": item.get("replyTotalCount"),
            "reply": reply
        })
    }).collect()
}
