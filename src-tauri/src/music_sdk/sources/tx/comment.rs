use super::helpers::*;
use crate::music_sdk::client::ResponseExt;

/// Get songId from songmid via musicInfo API
async fn get_song_id(song_info: &serde_json::Value) -> Result<i64, String> {
    if let Some(id) = song_info.get("songId").or_else(|| song_info.get("song_id")).and_then(|v| v.as_i64()) {
        return Ok(id);
    }

    let songmid = song_info.get("songmid").and_then(|v| v.as_str()).unwrap_or("");
    if songmid.is_empty() {
        return Err("No songmid provided".into());
    }

    let body = serde_json::json!({
        "comm": { "ct": "19", "cv": "1859", "uin": "0" },
        "req": {
            "module": "music.pf_song_detail_svr",
            "method": "get_song_detail_yqq",
            "param": { "song_type": 0, "song_mid": songmid }
        }
    });

    let resp: serde_json::Value = get_http()
        .post("https://u.y.qq.com/cgi-bin/musicu.fcg")
        .header("User-Agent", "Mozilla/5.0 (compatible; MSIE 9.0; Windows NT 6.1; WOW64; Trident/5.0)")
        .json(&body)
        .send().await.map_err(|e| e.to_string())?
        .json_sanitized().await?;

    let code = resp.get("code").and_then(|v| v.as_i64()).unwrap_or(-1);
    let req_code = resp.get("req").and_then(|r| r.get("code")).and_then(|v| v.as_i64()).unwrap_or(-1);
    if code != 0 || req_code != 0 {
        return Err("TX get songId failed".into());
    }

    resp.get("req").and_then(|r| r.get("data"))
        .and_then(|d| d.get("track_info"))
        .and_then(|t| t.get("id"))
        .and_then(|v| v.as_i64())
        .ok_or_else(|| "No songId found".into())
}

/// Get newest comments for a song
pub async fn get_comment(args: serde_json::Value) -> Result<serde_json::Value, String> {
    let song_info = args.get("songInfo").cloned().unwrap_or(serde_json::json!({}));
    let page = get_u64(&args, "page", 1);
    let limit = get_u64(&args, "limit", 20);

    let song_id = get_song_id(&song_info).await?;

    let form = [
        ("uin", "0"),
        ("format", "json"),
        ("cid", "205360772"),
        ("reqtype", "2"),
        ("biztype", "1"),
        ("topid", &song_id.to_string()),
        ("cmd", "8"),
        ("needmusiccrit", "1"),
        ("pagenum", &(page.saturating_sub(1)).to_string()),
        ("pagesize", &limit.to_string()),
    ];

    let resp: serde_json::Value = get_http()
        .post("http://c.y.qq.com/base/fcgi-bin/fcg_global_comment_h5.fcg")
        .header("User-Agent", "Mozilla/5.0 (compatible; MSIE 9.0; Windows NT 6.1; WOW64; Trident/5.0)")
        .form(&form)
        .send().await.map_err(|e| e.to_string())?
        .json_sanitized().await?;

    let code = resp.get("code").and_then(|v| v.as_i64()).unwrap_or(-1);
    if code != 0 {
        return Err("TX comment API error".into());
    }

    let comment = resp.get("comment").cloned().unwrap_or(serde_json::json!({}));
    let total = comment.get("commenttotal").and_then(|v| v.as_i64()).unwrap_or(0);
    let raw_list = comment.get("commentlist").and_then(|v| v.as_array()).cloned().unwrap_or_default();
    let comments = filter_new_comment(&raw_list);

    Ok(serde_json::json!({
        "source": "tx", "comments": comments, "total": total,
        "page": page, "limit": limit,
        "maxPage": ((total as f64 / limit as f64).ceil() as i64).max(1)
    }))
}

/// Get hot/top-liked comments for a song
pub async fn get_hot_comment(args: serde_json::Value) -> Result<serde_json::Value, String> {
    let song_info = args.get("songInfo").cloned().unwrap_or(serde_json::json!({}));
    let page = get_u64(&args, "page", 1);
    let limit = get_u64(&args, "limit", 20);

    let song_id = get_song_id(&song_info).await?;

    let body = serde_json::json!({
        "comm": {
            "cv": 4747474, "ct": 24, "format": "json",
            "inCharset": "utf-8", "outCharset": "utf-8",
            "notice": 0, "platform": "yqq.json", "needNewCode": 1, "uin": 0
        },
        "req": {
            "module": "music.globalComment.CommentRead",
            "method": "GetHotCommentList",
            "param": {
                "BizType": 1, "BizId": song_id.to_string(),
                "LastCommentSeqNo": "", "PageSize": limit,
                "PageNum": page.saturating_sub(1), "HotType": 1,
                "WithAirborne": 0, "PicEnable": 1
            }
        }
    });

    let resp: serde_json::Value = get_http()
        .post("https://u.y.qq.com/cgi-bin/musicu.fcg")
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/113.0.0.0 Safari/537.36 Edg/113.0.0.0")
        .header("Referer", "https://y.qq.com/")
        .header("Origin", "https://y.qq.com")
        .json(&body)
        .send().await.map_err(|e| e.to_string())?
        .json_sanitized().await?;

    let code = resp.get("code").and_then(|v| v.as_i64()).unwrap_or(-1);
    let req_code = resp.get("req").and_then(|r| r.get("code")).and_then(|v| v.as_i64()).unwrap_or(-1);
    if code != 0 || req_code != 0 {
        return Err("TX hot comment API error".into());
    }

    let comment_list = resp.get("req").and_then(|r| r.get("data"))
        .and_then(|d| d.get("CommentList")).cloned()
        .unwrap_or(serde_json::json!({}));
    let total = comment_list.get("Total").and_then(|v| v.as_i64()).unwrap_or(0);
    let raw_comments = comment_list.get("Comments").and_then(|v| v.as_array()).cloned().unwrap_or_default();
    let comments = filter_hot_comment(&raw_comments);

    Ok(serde_json::json!({
        "source": "tx", "comments": comments, "total": total,
        "page": page, "limit": limit,
        "maxPage": ((total as f64 / limit as f64).ceil() as i64).max(1)
    }))
}

fn format_time_to_ts(time: &serde_json::Value) -> Option<i64> {
    let ts = time.as_i64().or_else(|| time.as_str().and_then(|s| s.parse::<i64>().ok()))?;
    if ts.to_string().len() < 10 { return None; }
    Some(ts * 1000)
}

fn filter_new_comment(raw_list: &[serde_json::Value]) -> Vec<serde_json::Value> {
    raw_list.iter().map(|item| {
        let time_ts = format_time_to_ts(&item.get("time").cloned().unwrap_or(serde_json::Value::Null));
        let time_str = time_ts.map(|t| format_time_str(t / 1000));

        let root_content = item.get("rootcommentcontent").and_then(|v| v.as_str()).unwrap_or("");
        let text = replace_emoji(root_content).replace("\\n", "\n");

        let root_id = item.get("rootcommentid").and_then(|v| v.as_i64()).unwrap_or(0);
        let comment_id = item.get("commentid").and_then(|v| v.as_i64()).unwrap_or(0);
        let is_root = root_id == comment_id;

        let data = serde_json::json!({
            "id": format!("{}_{}", root_id, comment_id),
            "rootId": root_id,
            "text": text,
            "time": if is_root { time_ts } else { None },
            "timeStr": if is_root { time_str.clone() } else { None },
            "userName": item.get("rootcommentnick").and_then(|v| v.as_str())
                .map(|s| &s[1..]).unwrap_or(""),
            "avatar": item.get("avatarurl"),
            "userId": item.get("encrypt_rootcommentuin"),
            "likedCount": item.get("praisenum"),
            "reply": []
        });

        // Handle middlecommentcontent (replies)
        if let Some(middle) = item.get("middlecommentcontent").and_then(|v| v.as_array()) {
            let replies: Vec<serde_json::Value> = middle.iter().map(|c| {
                let sub_id = c.get("subcommentid").and_then(|v| v.as_i64()).unwrap_or(0);
                let sub_text = replace_emoji(c.get("subcommentcontent").and_then(|v| v.as_str()).unwrap_or(""))
                    .replace("\\n", "\n");
                let is_sub = sub_id == comment_id;
                serde_json::json!({
                    "id": format!("sub_{}_{}", root_id, sub_id),
                    "text": sub_text,
                    "time": if is_sub { time_ts } else { None },
                    "timeStr": if is_sub { time_str.clone() } else { None },
                    "userName": c.get("replynick").and_then(|v| v.as_str())
                        .map(|s| &s[1..]).unwrap_or(""),
                    "avatar": c.get("avatarurl"),
                    "userId": c.get("encrypt_replyuin"),
                    "likedCount": c.get("praisenum")
                })
            }).collect();

            let mut result = data.clone();
            result["reply"] = serde_json::json!(replies);
            // Use avatar/likedCount from parent for the root comment
            if let Some(first) = middle.first() {
                result["avatar"] = first.get("avatarurl").cloned().unwrap_or(serde_json::Value::Null);
                result["likedCount"] = first.get("praisenum").cloned().unwrap_or(serde_json::Value::Null);
            }
            result
        } else {
            data
        }
    }).collect()
}

fn filter_hot_comment(raw_list: &[serde_json::Value]) -> Vec<serde_json::Value> {
    raw_list.iter().map(|item| {
        let seq_no = item.get("SeqNo").and_then(|v| v.as_i64()).unwrap_or(0);
        let cm_id = item.get("CmId").and_then(|v| v.as_i64()).unwrap_or(0);
        let content = item.get("Content").and_then(|v| v.as_str()).unwrap_or("");
        let text = replace_emoji(content).replace("\\n", "\n");

        let pub_time = item.get("PubTime").and_then(|v| v.as_str())
            .and_then(|s| s.parse::<i64>().ok())
            .filter(|t| t.to_string().len() >= 10)
            .map(|t| t * 1000);
        let time_str = pub_time.map(|t| format_time_str(t / 1000));

        let pic = item.get("Pic").and_then(|v| v.as_str()).unwrap_or("");
        let images = if !pic.is_empty() {
            vec![serde_json::json!(pic)]
        } else {
            vec![]
        };

        let sub_comments: Vec<serde_json::Value> = item.get("SubComments").and_then(|v| v.as_array())
            .cloned().unwrap_or_default().iter().map(|c| {
                let c_seq = c.get("SeqNo").and_then(|v| v.as_i64()).unwrap_or(0);
                let c_cm = c.get("CmId").and_then(|v| v.as_i64()).unwrap_or(0);
                let c_content = c.get("Content").and_then(|v| v.as_str()).unwrap_or("");
                let c_text = replace_emoji(c_content).replace("\\n", "\n");
                let c_pub = c.get("PubTime").and_then(|v| v.as_str())
                    .and_then(|s| s.parse::<i64>().ok())
                    .filter(|t| t.to_string().len() >= 10)
                    .map(|t| t * 1000);
                let c_time_str = c_pub.map(|t| format_time_str(t / 1000));
                let c_pic = c.get("Pic").and_then(|v| v.as_str()).unwrap_or("");
                let c_images = if !c_pic.is_empty() { vec![serde_json::json!(c_pic)] } else { vec![] };

                serde_json::json!({
                    "id": format!("sub_{}_{}", c_seq, c_cm),
                    "text": c_text, "time": c_pub, "timeStr": c_time_str,
                    "userName": c.get("Nick").and_then(|v| v.as_str()).unwrap_or(""),
                    "avatar": c.get("Avatar"),
                    "images": c_images,
                    "userId": c.get("EncryptUin"),
                    "likedCount": c.get("PraiseNum")
                })
            }).collect();

        serde_json::json!({
            "id": format!("{}_{}", seq_no, cm_id),
            "rootId": seq_no,
            "text": text, "time": pub_time, "timeStr": time_str,
            "userName": item.get("Nick").and_then(|v| v.as_str()).unwrap_or(""),
            "images": images,
            "avatar": item.get("Avatar"),
            "location": item.get("Location").and_then(|v| v.as_str()).unwrap_or(""),
            "userId": item.get("EncryptUin"),
            "likedCount": item.get("PraiseNum"),
            "reply": sub_comments
        })
    }).collect()
}
