use super::helpers::*;

pub async fn get_comment(args: serde_json::Value) -> Result<serde_json::Value, String> {
    let songmid = get_songmid(&args);
    let page = get_u64(&args, "page", 1);
    let limit = get_u64(&args, "limit", 20);

    let start = limit * (page - 1);
    let url = format!("http://ncomment.kuwo.cn/com.s?f=web&type=get_comment&aapiver=1&prod=kwplayer_ar_10.5.2.0&digest=15&sid={}&start={}&msgflag=1&count={}&newver=3&uid=0", songmid, start, limit);
    fetch_comments(url, page, limit).await
}

pub async fn get_hot_comment(args: serde_json::Value) -> Result<serde_json::Value, String> {
    let songmid = get_songmid(&args);
    let page = get_u64(&args, "page", 1);
    let limit = get_u64(&args, "limit", 100);

    let start = limit * (page - 1);
    let url = format!("http://ncomment.kuwo.cn/com.s?f=web&type=get_rec_comment&aapiver=1&prod=kwplayer_ar_10.5.2.0&digest=15&sid={}&start={}&msgflag=1&count={}&newver=3&uid=0", songmid, start, limit);
    fetch_comments(url, page, limit).await
}

async fn fetch_comments(url: String, page: u64, limit: u64) -> Result<serde_json::Value, String> {
    let resp: serde_json::Value = get_http()
        .get(&url)
        .header("User-Agent", "Dalvik/2.1.0 (Linux; U; Android 9;)")
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;

    let total = resp
        .get("comments_counts")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);
    let hot_total = resp
        .get("hot_comments_counts")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);
    let actual_total = if total > 0 { total } else { hot_total };

    let raw = resp
        .get("comments")
        .or(resp.get("hot_comments"))
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let comments: Vec<serde_json::Value> = raw.iter().map(|item| {
        let child = item.get("child_comments").and_then(|v| v.as_array()).cloned().unwrap_or_default();
        let reply: Vec<serde_json::Value> = child.iter().map(|c| {
            serde_json::json!({
                "id": c.get("id"), "text": c.get("msg"), "time": c.get("time"),
                "userName": c.get("u_name"), "avatar": c.get("u_pic"), "userId": c.get("u_id"),
                "likedCount": c.get("like_num"), "images": c.get("mpic").map(|m| vec![m]).unwrap_or_default()
            })
        }).collect();
        serde_json::json!({
            "id": item.get("id"), "text": item.get("msg"), "time": item.get("time"),
            "userName": item.get("u_name"), "avatar": item.get("u_pic"), "userId": item.get("u_id"),
            "likedCount": item.get("like_num"),
            "images": item.get("mpic").and_then(|m| m.as_str()).map(|m| vec![serde_json::json!(m)]).unwrap_or_default(),
            "reply": reply
        })
    }).collect();

    Ok(serde_json::json!({
        "source": "kw", "comments": comments, "total": actual_total,
        "page": page, "limit": limit, "maxPage": ((actual_total as f64 / limit as f64).ceil() as i64).max(1)
    }))
}
