use super::helpers::*;
use crate::music_sdk::client::ResponseExt;
use crate::music_sdk::client::{MusicItem, PlaylistItem, PlaylistResult};

// --- Playlist Tags ---

pub async fn get_playlist_tags(_args: serde_json::Value) -> Result<serde_json::Value, String> {
    let tags_url = "https://u.y.qq.com/cgi-bin/musicu.fcg?loginUin=0&hostUin=0&format=json&inCharset=utf-8&outCharset=utf-8&notice=0&platform=wk_v15.json&needNewCode=0&data=%7B%22tags%22%3A%7B%22method%22%3A%22get_all_categories%22%2C%22param%22%3A%7B%22qq%22%3A%22%22%7D%2C%22module%22%3A%22playlist.PlaylistAllCategoriesServer%22%7D%7D";
    let hot_tag_url = "https://c.y.qq.com/node/pc/wk_v15/category_playlist.html";

    let (tags_result, hot_result) = tokio::join!(
        async {
            let resp: serde_json::Value = get_http().get(tags_url)
                .header("User-Agent", "Mozilla/5.0 (compatible; MSIE 9.0; Windows NT 6.1; WOW64; Trident/5.0)")
                .send().await.map_err(|e| e.to_string())?
                .json_sanitized().await?;

            let code = resp.get("code").and_then(|v| v.as_i64()).unwrap_or(-1);
            if code != 0 {
                return Err(format!("TX tag API error: code={}", code));
            }

            let v_group = resp.get("tags").and_then(|t| t.get("data")).and_then(|d| d.get("v_group"))
                .and_then(|v| v.as_array()).cloned().unwrap_or_default();

            let tags: Vec<serde_json::Value> = v_group.iter().map(|group| {
                let name = group.get("group_name").and_then(|v| v.as_str()).unwrap_or("");
                let list: Vec<serde_json::Value> = group.get("v_item").and_then(|v| v.as_array())
                    .cloned().unwrap_or_default()
                    .iter().map(|item| {
                        let id = item.get("id").cloned().unwrap_or(serde_json::Value::Null);
                        let name = item.get("name").and_then(|v| v.as_str()).unwrap_or("");
                        serde_json::json!({ "id": id, "name": name, "source": "tx" })
                    }).collect();
                serde_json::json!({ "name": name, "list": list, "source": "tx" })
            }).collect();

            Ok::<_, String>(tags)
        },
        async {
            let resp = get_http().get(hot_tag_url)
                .header("User-Agent", "Mozilla/5.0 (compatible; MSIE 9.0; Windows NT 6.1; WOW64; Trident/5.0)")
                .send().await.map_err(|e| e.to_string())?;
            let html = resp.text().await.map_err(|e| e.to_string())?;

            let re = regex_lite::Regex::new(r#"data-id="(\w+)">(.+?)</a>"#).map_err(|e| e.to_string())?;
            let hot: Vec<serde_json::Value> = re.captures_iter(&html).filter_map(|caps| {
                let id = caps.get(1)?.as_str();
                let name = caps.get(2)?.as_str();
                Some(serde_json::json!({ "id": id, "name": name, "source": "tx" }))
            }).collect();

            Ok::<_, String>(hot)
        }
    );

    let tags = tags_result.unwrap_or_default();
    let hot_tag = hot_result.unwrap_or_default();
    Ok(serde_json::json!({ "tags": tags, "hotTag": hot_tag, "source": "tx" }))
}

// --- Category Playlists ---

fn build_tx_playlist_url(sort_id: &str, tag_id: &str, page: u64) -> String {
    let base = "https://u.y.qq.com/cgi-bin/musicu.fcg?loginUin=0&hostUin=0&format=json&inCharset=utf-8&outCharset=utf-8&notice=0&platform=wk_v15.json&needNewCode=0";

    let data = if !tag_id.is_empty() {
        let id: i64 = tag_id.parse().unwrap_or(0);
        serde_json::json!({
            "comm": { "cv": 1602, "ct": 20 },
            "playlist": {
                "method": "get_category_content",
                "param": {
                    "titleid": id, "caller": "0", "category_id": id,
                    "size": 36, "page": (page as i64) - 1, "use_page": 1
                },
                "module": "playlist.PlayListCategoryServer"
            }
        })
    } else {
        let order = if sort_id.is_empty() || sort_id == "hot" { "5" } else { sort_id };
        serde_json::json!({
            "comm": { "cv": 1602, "ct": 20 },
            "playlist": {
                "method": "get_playlist_by_tag",
                "param": {
                    "id": 10000000_i64,
                    "sin": (36_i64) * ((page as i64) - 1),
                    "size": 36_i64,
                    "order": order.parse::<i64>().unwrap_or(5),
                    "cur_page": page as i64
                },
                "module": "playlist.PlayListPlazaServer"
            }
        })
    };

    format!("{}&data={}", base, urlencoding::encode(&serde_json::to_string(&data).unwrap_or_default()))
}

pub async fn get_category_playlists(args: serde_json::Value) -> Result<serde_json::Value, String> {
    let tag_id = get_str(&args, "tagId").to_string();
    let sort_id = get_str(&args, "sortId").to_string();
    let page = get_u64(&args, "page", 1);
    let limit = get_u64(&args, "limit", 36);

    let url = build_tx_playlist_url(&sort_id, &tag_id, page);
    let resp: serde_json::Value = get_http().get(&url)
        .header("User-Agent", "Mozilla/5.0 (compatible; MSIE 9.0; Windows NT 6.1; WOW64; Trident/5.0)")
        .send().await.map_err(|e| e.to_string())?
        .json_sanitized().await?;

    let code = resp.get("code").and_then(|v| v.as_i64()).unwrap_or(-1);
    if code != 0 {
        return Err(format!("TX playlist API error: code={}", code));
    }

    let playlist_data = resp.get("playlist").and_then(|p| p.get("data")).cloned().unwrap_or(serde_json::json!({}));

    let (list, total) = if tag_id.is_empty() {
        let raw_list = playlist_data.get("v_playlist").and_then(|v| v.as_array()).cloned().unwrap_or_default();
        let total = playlist_data.get("total").and_then(|v| v.as_i64()).unwrap_or(0);
        let list: Vec<PlaylistItem> = raw_list.iter().map(|item| {
            let tid = item.get("tid").cloned().unwrap_or(serde_json::Value::Null);
            PlaylistItem {
                id: tid,
                name: item.get("title").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                img: item.get("cover_url_medium").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                source: "tx".into(),
                desc: item.get("desc").and_then(|v| v.as_str()).unwrap_or("").replace("<br>", "\n"),
                play_count: item.get("access_num").cloned().unwrap_or(serde_json::Value::Null),
                author: item.get("creator_info").and_then(|c| c.get("nick")).and_then(|v| v.as_str()).unwrap_or("").to_string(),
                total: serde_json::Value::Null,
            }
        }).collect();
        (list, total)
    } else {
        let content = playlist_data.get("content").cloned().unwrap_or(serde_json::json!({}));
        let raw_list = content.get("v_item").and_then(|v| v.as_array()).cloned().unwrap_or_default();
        let total = content.get("total_cnt").and_then(|v| v.as_i64()).unwrap_or(0);
        let list: Vec<PlaylistItem> = raw_list.iter().filter_map(|item| {
            let basic = item.get("basic")?;
            let tid = basic.get("tid").cloned().unwrap_or(serde_json::Value::Null);
            Some(PlaylistItem {
                id: tid,
                name: basic.get("title").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                img: basic.get("cover").and_then(|c| c.get("medium_url").or(c.get("default_url"))).and_then(|v| v.as_str()).unwrap_or("").to_string(),
                source: "tx".into(),
                desc: basic.get("desc").and_then(|v| v.as_str()).unwrap_or("").replace("<br>", "\n"),
                play_count: basic.get("play_cnt").cloned().unwrap_or(serde_json::Value::Null),
                author: basic.get("creator").and_then(|c| c.get("nick")).and_then(|v| v.as_str()).unwrap_or("").to_string(),
                total: serde_json::Value::Null,
            })
        }).collect();
        (list, total)
    };

    Ok(serde_json::to_value(PlaylistResult {
        list, all_page: (total as f64 / limit as f64).ceil() as i64,
        limit: limit as i64, total, source: "tx".into(),
    }).unwrap())
}

pub async fn get_hot_songlist(args: serde_json::Value) -> Result<serde_json::Value, String> {
    get_category_playlists(args).await
}

// --- Leaderboards ---

pub async fn get_leaderboards(_args: serde_json::Value) -> Result<serde_json::Value, String> {
    let url = "https://c.y.qq.com/v8/fcg-bin/fcg_myqq_toplist.fcg?g_tk=1928093487&inCharset=utf-8&outCharset=utf-8&notice=0&format=json&uin=0&needNewCode=1&platform=h5";
    let resp: serde_json::Value = get_http().get(url)
        .header("User-Agent", "Mozilla/5.0 (compatible; MSIE 9.0; Windows NT 6.1; WOW64; Trident/5.0)")
        .send().await.map_err(|e| e.to_string())?
        .json_sanitized().await?;

    let code = resp.get("code").and_then(|v| v.as_i64()).unwrap_or(-1);
    if code != 0 {
        return Err(format!("TX toplist API error: code={}", code));
    }

    let raw_list = resp.get("data").and_then(|d| d.get("topList")).and_then(|v| v.as_array()).cloned().unwrap_or_default();
    let list: Vec<serde_json::Value> = raw_list.iter().filter_map(|board| {
        let id = board.get("id")?.as_i64()?;
        if id == 201 { return None; }

        let mut name = board.get("topTitle")?.as_str()?.to_string();
        if name.starts_with("巅峰榜·") {
            name = name.replace("巅峰榜·", "");
        }
        if !name.ends_with('榜') {
            name.push('榜');
        }

        let pic = board.get("picUrl").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let listen = board.get("listenCount").cloned().unwrap_or(serde_json::Value::Null);

        Some(serde_json::json!({
            "id": format!("tx__{}", id), "name": name,
            "bangid": id.to_string(), "img": pic, "pic": pic,
            "listen": listen, "source": "tx"
        }))
    }).collect();

    Ok(serde_json::json!({ "list": list, "source": "tx" }))
}

// --- Leaderboard Detail ---

pub async fn get_leaderboard_detail(args: serde_json::Value) -> Result<serde_json::Value, String> {
    let raw_id = args.get("id").map(|v| match v {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Number(n) => n.to_string(),
        _ => String::new(),
    }).unwrap_or_default();
    let limit = get_u64(&args, "limit", 300);

    let bang_id: i64 = if raw_id.starts_with("tx__") {
        raw_id.replace("tx__", "").parse().unwrap_or(0)
    } else {
        raw_id.parse().unwrap_or(0)
    };

    let body = serde_json::json!({
        "toplist": {
            "module": "musicToplist.ToplistInfoServer",
            "method": "GetDetail",
            "param": { "topid": bang_id, "num": limit, "period": "" }
        },
        "comm": { "uin": 0, "format": "json", "ct": 20, "cv": 1859 }
    });

    let resp: serde_json::Value = get_http()
        .post("https://u.y.qq.com/cgi-bin/musicu.fcg")
        .header("User-Agent", "Mozilla/5.0 (compatible; MSIE 9.0; Windows NT 6.1; WOW64; Trident/5.0)")
        .json(&body)
        .send().await.map_err(|e| e.to_string())?
        .json_sanitized().await?;

    let code = resp.get("code").and_then(|v| v.as_i64()).unwrap_or(-1);
    if code != 0 {
        return Err(format!("TX toplist detail API error: code={}", code));
    }

    let toplist_data = resp.get("toplist").and_then(|t| t.get("data")).cloned().unwrap_or(serde_json::json!({}));
    let song_info_list = toplist_data.get("songInfoList").and_then(|v| v.as_array()).cloned().unwrap_or_default();
    let total = song_info_list.len() as i64;

    let list: Vec<MusicItem> = song_info_list.iter().map(|item| {
        tx_parse_music_item(item)
    }).collect();

    Ok(serde_json::json!({
        "list": list, "info": serde_json::json!({}),
        "allPage": 1, "limit": limit as i64, "total": total, "source": "tx"
    }))
}

// --- Playlist Detail ---

pub async fn get_playlist_detail(args: serde_json::Value) -> Result<serde_json::Value, String> {
    let raw_id = args.get("id").map(|v| match v {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Number(n) => n.to_string(),
        _ => String::new(),
    }).unwrap_or_default();
    let limit = get_u64(&args, "limit", 300);

    let id = raw_id.replace("tx_", "").trim().to_string();

    let url = format!(
        "https://c.y.qq.com/qzone/fcg-bin/fcg_ucc_getcdinfo_byids_cp.fcg?type=1&json=1&utf8=1&onlysong=0&new_format=1&disstid={}&loginUin=0&hostUin=0&format=json&inCharset=utf8&outCharset=utf-8&notice=0&platform=yqq.json&needNewCode=0",
        id
    );

    let resp: serde_json::Value = get_http().get(&url)
        .header("Origin", "https://y.qq.com")
        .header("Referer", format!("https://y.qq.com/n/yqq/playsquare/{}.html", id))
        .header("User-Agent", "Mozilla/5.0 (compatible; MSIE 9.0; Windows NT 6.1; WOW64; Trident/5.0)")
        .send().await.map_err(|e| e.to_string())?
        .json_sanitized().await?;

    let code = resp.get("code").and_then(|v| v.as_i64()).unwrap_or(-1);
    if code != 0 {
        return Err(format!("TX playlist detail API error: code={}", code));
    }

    let cdlist = resp.get("cdlist").and_then(|v| v.as_array()).and_then(|a| a.first()).cloned().unwrap_or(serde_json::json!({}));
    let songlist = cdlist.get("songlist").and_then(|v| v.as_array()).cloned().unwrap_or_default();
    let total = songlist.len() as i64;

    // Fetch batch quality info
    let song_ids: Vec<i64> = songlist.iter()
        .filter_map(|item| item.get("id").and_then(|v| v.as_i64()))
        .collect();
    let quality_map = super::playback::get_batch_quality_info(&song_ids).await.unwrap_or_default();

    let list: Vec<MusicItem> = songlist.iter().map(|item| {
        let song_id = item.get("id").and_then(|v| v.as_i64()).unwrap_or(0);
        let singer_list = item.get("singer").and_then(|v| v.as_array()).cloned().unwrap_or_default();
        let singer = format_singer(&singer_list);
        let name = item.get("title").and_then(|v| v.as_str()).unwrap_or("");
        let album_name = item.get("album").and_then(|a| a.get("name")).and_then(|v| v.as_str()).unwrap_or("").to_string();
        let album_mid = item.get("album").and_then(|a| a.get("mid")).and_then(|v| v.as_str()).unwrap_or("").to_string();
        let songmid = item.get("mid").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let interval = item.get("interval").and_then(|v| v.as_i64()).unwrap_or(0);
        let img = get_song_img(item, &album_mid);

        let (types, types_map) = if let Some((t, m)) = quality_map.get(&song_id) {
            (t.clone(), m.clone())
        } else {
            let file = item.get("file").cloned().unwrap_or(serde_json::json!({}));
            parse_quality_types(&file)
        };

        MusicItem {
            songmid: serde_json::Value::String(songmid.clone()),
            singer, name: name.to_string(), album_name,
            album_id: serde_json::Value::String(album_mid.clone()),
            source: "tx".into(), interval: format_play_time(interval), img, lrc: None,
            types: Some(types), types_map: Some(types_map), type_url: Some(serde_json::json!({})),
            hash: None,
            song_id: Some(serde_json::json!(song_id)),
            str_media_mid: Some(item.get("file").and_then(|f| f.get("media_mid")).and_then(|v| v.as_str()).unwrap_or("").to_string()),
            album_mid: Some(album_mid),
        copyright_id: None, lrc_url: None, mrc_url: None, trc_url: None,
        singer_id: None,
        }
    }).collect();

    Ok(serde_json::json!({
        "list": list,
        "info": {
            "name": cdlist.get("dissname").and_then(|v| v.as_str()).unwrap_or(""),
            "img": cdlist.get("logo").and_then(|v| v.as_str()).unwrap_or(""),
            "desc": cdlist.get("desc").and_then(|v| v.as_str()).unwrap_or("").replace("<br>", "\n"),
            "author": cdlist.get("nickname").and_then(|v| v.as_str()).unwrap_or("")
        },
        "allPage": 1, "limit": limit as i64, "total": total, "source": "tx"
    }))
}

// --- Shared Music Item parser ---

fn tx_parse_music_item(item: &serde_json::Value) -> MusicItem {
    let singer_list = item.get("singer").and_then(|v| v.as_array()).cloned().unwrap_or_default();
    let singer = format_singer(&singer_list);
    let name = item.get("title").and_then(|v| v.as_str()).unwrap_or("");
    let album_name = item.get("album").and_then(|a| a.get("name")).and_then(|v| v.as_str()).unwrap_or("").to_string();
    let album_mid = item.get("album").and_then(|a| a.get("mid")).and_then(|v| v.as_str()).unwrap_or("").to_string();
    let songmid = item.get("mid").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let interval = item.get("interval").and_then(|v| v.as_i64()).unwrap_or(0);
    let img = get_song_img(item, &album_mid);

    let file = item.get("file").cloned().unwrap_or(serde_json::json!({}));
    let (types, types_map) = parse_quality_types(&file);

    MusicItem {
        songmid: serde_json::Value::String(songmid.clone()),
        singer, name: name.to_string(), album_name,
        album_id: serde_json::Value::String(album_mid.clone()),
        source: "tx".into(), interval: format_play_time(interval), img, lrc: None,
        types: Some(types), types_map: Some(types_map), type_url: Some(serde_json::json!({})),
        hash: None,
        song_id: Some(serde_json::json!(item.get("id").and_then(|v| v.as_i64()).unwrap_or(0))),
        str_media_mid: Some(item.get("file").and_then(|f| f.get("media_mid")).and_then(|v| v.as_str()).unwrap_or("").to_string()),
        album_mid: Some(album_mid),
        copyright_id: None, lrc_url: None, mrc_url: None, trc_url: None,
        singer_id: None,
    }
}
