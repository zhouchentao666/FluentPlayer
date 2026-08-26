use super::helpers::*;
use super::crypto::weapi_form;

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
    let page = get_u64(&args, "page", 1);
    let limit = get_u64(&args, "limit", 20);

    if songmid.is_empty() {
        return Err("No songmid provided".into());
    }

    let rid = format!("R_SO_4_{}", songmid);
    let cursor = chrono::Utc::now().timestamp_millis();

    let body = weapi_form(&serde_json::json!({
        "cursor": cursor,
        "offset": 0,
        "orderType": 1,
        "pageNo": page,
        "pageSize": limit,
        "rid": rid,
        "threadId": rid
    }));

    let resp = match get_http()
        .post("https://music.163.com/weapi/comment/resource/comments/get")
        .header("User-Agent", "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/60.0.3112.90 Safari/537.36")
        .header("Origin", "https://music.163.com")
        .header("Referer", "http://music.163.com/")
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send().await
    {
        Ok(r) => match parse_json_response(r).await {
            Ok(json) => json,
            Err(_) => return Ok(empty_comment_result(page, limit)),
        },
        Err(_) => return Ok(empty_comment_result(page, limit)),
    };

    let code = resp.get("code").and_then(|v| v.as_i64()).unwrap_or(0);
    if code != 200 {
        return Ok(empty_comment_result(page, limit));
    }

    let data = resp.get("data").cloned().unwrap_or(serde_json::json!({}));
    let total = data.get("totalCount").and_then(|v| v.as_i64()).unwrap_or(0);
    let raw_list = data.get("comments").and_then(|v| v.as_array()).cloned().unwrap_or_default();
    let comments = filter_comments(&raw_list);

    Ok(serde_json::json!({
        "source": "wy", "comments": comments, "total": total,
        "page": page, "limit": limit,
        "maxPage": ((total as f64 / limit as f64).ceil() as i64).max(1)
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
    let page = get_u64(&args, "page", 1);
    let limit = get_u64(&args, "limit", 100);

    if songmid.is_empty() {
        return Err("No songmid provided".into());
    }

    let rid = format!("R_SO_4_{}", songmid);
    let before_time = chrono::Utc::now().timestamp_millis().to_string();

    let body = weapi_form(&serde_json::json!({
        "rid": rid, "limit": limit,
        "offset": limit * page.saturating_sub(1),
        "beforeTime": before_time
    }));

    let resp = match get_http()
        .post(format!("https://music.163.com/weapi/v1/resource/hotcomments/{}", rid))
        .header("User-Agent", "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/60.0.3112.90 Safari/537.36")
        .header("Origin", "https://music.163.com")
        .header("Referer", "http://music.163.com/")
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send().await
    {
        Ok(r) => match parse_json_response(r).await {
            Ok(json) => json,
            Err(_) => return Ok(empty_comment_result(page, limit)),
        },
        Err(_) => return Ok(empty_comment_result(page, limit)),
    };

    let code = resp.get("code").and_then(|v| v.as_i64()).unwrap_or(0);
    if code != 200 {
        return Ok(empty_comment_result(page, limit));
    }

    let total = resp.get("total").and_then(|v| v.as_i64()).unwrap_or(0);
    let raw_list = resp.get("hotComments").and_then(|v| v.as_array()).cloned().unwrap_or_default();
    let comments = filter_comments(&raw_list);

    Ok(serde_json::json!({
        "source": "wy", "comments": comments, "total": total,
        "page": page, "limit": limit,
        "maxPage": ((total as f64 / limit as f64).ceil() as i64).max(1)
    }))
}

fn empty_comment_result(page: u64, limit: u64) -> serde_json::Value {
    serde_json::json!({
        "source": "wy", "comments": [], "total": 0,
        "page": page, "limit": limit, "maxPage": 1
    })
}

fn filter_comments(raw_list: &[serde_json::Value]) -> Vec<serde_json::Value> {
    raw_list.iter().map(|item| {
        let content = item.get("content").and_then(|v| v.as_str()).unwrap_or("");
        let text = apply_emoji(content);
        let time = item.get("time").and_then(|v| v.as_i64()).unwrap_or(0);
        let time_str = if time > 0 {
            let dt = chrono::DateTime::from_timestamp_millis(time);
            dt.map(|d| d.format("%Y-%m-%d %H:%M").to_string()).unwrap_or_default()
        } else {
            String::new()
        };

        let user = item.get("user").cloned().unwrap_or(serde_json::json!({}));
        let location = item.get("ipLocation").and_then(|l| l.get("location")).and_then(|v| v.as_str()).unwrap_or("");

        let data = serde_json::json!({
            "id": item.get("commentId"),
            "text": text, "time": time, "timeStr": time_str,
            "location": location,
            "userName": user.get("nickname").and_then(|v| v.as_str()).unwrap_or(""),
            "avatar": user.get("avatarUrl").and_then(|v| v.as_str()).unwrap_or(""),
            "userId": user.get("userId"),
            "likedCount": item.get("likedCount"),
            "reply": []
        });

        // Handle beReplied (quote reply)
        if let Some(be_replied) = item.get("beReplied").and_then(|v| v.as_array()).and_then(|a| a.first()) {
            let reply_content = be_replied.get("content").and_then(|v| v.as_str()).unwrap_or("");
            let reply_text = apply_emoji(reply_content);
            let reply_user = be_replied.get("user").cloned().unwrap_or(serde_json::json!({}));
            let reply_location = be_replied.get("ipLocation").and_then(|l| l.get("location")).and_then(|v| v.as_str()).unwrap_or("");

            return serde_json::json!({
                "id": item.get("commentId"),
                "rootId": be_replied.get("beRepliedCommentId"),
                "text": reply_text, "time": time, "timeStr": null,
                "location": reply_location,
                "userName": reply_user.get("nickname").and_then(|v| v.as_str()).unwrap_or(""),
                "avatar": reply_user.get("avatarUrl").and_then(|v| v.as_str()).unwrap_or(""),
                "userId": reply_user.get("userId"),
                "likedCount": null,
                "reply": [data]
            });
        }

        data
    }).collect()
}
