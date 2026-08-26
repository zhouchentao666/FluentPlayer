use super::helpers::*;
use crate::music_sdk::client::ResponseExt;
use super::crypto::zzc_sign;
use crate::music_sdk::client::{MusicItem, SearchResult};

pub async fn search(args: serde_json::Value) -> Result<serde_json::Value, String> {
    let keyword = get_str(&args, "keyword");
    let page = get_u64(&args, "page", 1);
    let limit = get_u64(&args, "limit", 20);
    if keyword.is_empty() {
        return Ok(serde_json::to_value(SearchResult {
            list: vec![], all_page: 0, limit: limit as i64, total: 0, source: "tx".into(),
        }).unwrap());
    }

    let search_id = get_search_id();
    let body = serde_json::json!({
        "comm": {
            "ct": "11", "cv": "14090508", "v": "14090508",
            "tmeAppID": "qqmusic", "phonetype": "EBG-AN10",
            "deviceScore": "553.47", "devicelevel": "50", "newdevicelevel": "20",
            "rom": "HuaWei/EMOTION/EmotionUI_14.2.0", "os_ver": "12",
            "OpenUDID": "0", "OpenUDID2": "0", "QIMEI36": "0",
            "udid": "0", "chid": "0", "aid": "0", "oaid": "0",
            "taid": "0", "tid": "0", "wid": "0", "uid": "0", "sid": "0",
            "modeSwitch": "6", "teenMode": "0", "ui_mode": "2", "nettype": "1020", "v4ip": ""
        },
        "req": {
            "module": "music.search.SearchCgiService",
            "method": "DoSearchForQQMusicMobile",
            "param": {
                "search_type": 0, "searchid": search_id,
                "query": keyword, "page_num": page, "num_per_page": limit,
                "highlight": 0, "nqc_flag": 0, "multi_zhida": 0,
                "cat": 2, "grp": 1, "sin": 0, "sem": 0
            }
        }
    });

    let body_str = serde_json::to_string(&body).unwrap_or_default();
    let sign = zzc_sign(&body_str);

    let url = format!("https://u.y.qq.com/cgi-bin/musics.fcg?sign={}", sign);
    let resp: serde_json::Value = get_http()
        .post(&url)
        .header("User-Agent", "QQMusic 14090508(android 12)")
        .header("Content-Type", "application/json")
        .body(body_str)
        .send().await.map_err(|e| e.to_string())?
        .json_sanitized().await?;

    let code = resp.get("code").and_then(|v| v.as_i64()).unwrap_or(-1);
    let req_code = resp.get("req").and_then(|r| r.get("code")).and_then(|v| v.as_i64()).unwrap_or(-1);
    if code != 0 || req_code != 0 {
        return Err(format!("TX search API error: code={}/{}", code, req_code));
    }

    let data = resp.get("req").and_then(|r| r.get("data")).cloned().unwrap_or(serde_json::json!({}));
    let meta = data.get("meta").cloned().unwrap_or(serde_json::json!({}));
    let total = meta.get("sum").and_then(|v| v.as_i64()).unwrap_or(0);
    let raw_list = data.get("body").and_then(|b| b.get("item_song"))
        .or_else(|| data.get("song").and_then(|s| s.get("itemlist")))
        .and_then(|v| v.as_array()).cloned().unwrap_or_default();

    let list: Vec<MusicItem> = raw_list.iter().filter_map(|item| {
        let file = item.get("file")?;
        let media_mid = file.get("media_mid")?.as_str()?;
        if media_mid.is_empty() { return None; }

        let singer_list = item.get("singer").and_then(|v| v.as_array()).cloned().unwrap_or_default();
        let singer = format_singer(&singer_list);
        let name_raw = item.get("name").and_then(|v| v.as_str()).unwrap_or("");
        let title_extra = item.get("title_extra").and_then(|v| v.as_str()).unwrap_or("");
        let name = format!("{}{}", name_raw, title_extra);
        let songmid = item.get("mid").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let song_id = item.get("id").and_then(|v| v.as_i64()).unwrap_or(0);
        let album = item.get("album");
        let album_name = album.and_then(|a| a.get("name")).and_then(|v| v.as_str()).unwrap_or("").to_string();
        let album_mid = album.and_then(|a| a.get("mid")).and_then(|v| v.as_str()).unwrap_or("").to_string();
        let interval = item.get("interval").and_then(|v| v.as_i64()).unwrap_or(0);
        let img = get_song_img(item, &album_mid);
        let (types, types_map) = parse_quality_types(file);

        let singer_id = singer_list.first()
            .and_then(|s| s.get("mid"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        Some(MusicItem {
            songmid: serde_json::Value::String(songmid.clone()),
            singer, name, album_name,
            album_id: serde_json::Value::String(album_mid.clone()),
            source: "tx".into(), interval: format_play_time(interval), img, lrc: None,
            types: Some(types), types_map: Some(types_map), type_url: Some(serde_json::json!({})),
            hash: None,
            song_id: Some(serde_json::json!(song_id)),
            str_media_mid: Some(media_mid.to_string()),
            album_mid: Some(album_mid),
        copyright_id: None, lrc_url: None, mrc_url: None, trc_url: None,
        singer_id,
        })
    }).collect();

    Ok(serde_json::json!({
        "list": list,
        "allPage": (total as f64 / limit as f64).ceil() as i64,
        "limit": limit as i64, "total": total, "source": "tx"
    }))
}

pub async fn tip_search(args: serde_json::Value) -> Result<serde_json::Value, String> {
    let keyword = get_str(&args, "keyword");
    if keyword.is_empty() {
        return Ok(serde_json::json!({ "list": { "order": [], "songs": [], "artists": [], "albums": [] } }));
    }

    let url = format!(
        "https://c.y.qq.com/splcloud/fcgi-bin/smartbox_new.fcg?is_xml=0&format=json&key={}&loginUin=0&hostUin=0&format=json&inCharset=utf8&outCharset=utf-8&notice=0&platform=yqq&needNewCode=0",
        urlencoding::encode(keyword)
    );

    let resp: serde_json::Value = get_http().get(&url)
        .header("Referer", "https://y.qq.com/portal/player.html")
        .send().await.map_err(|e| e.to_string())?
        .json_sanitized().await?;

    let code = resp.get("code").and_then(|v| v.as_i64()).unwrap_or(-1);
    if code != 0 {
        return Err("TX tip search API error".into());
    }

    let data = resp.get("data").cloned().unwrap_or(serde_json::json!({}));
    let mut order = Vec::new();

    let songs_count = data.get("song").and_then(|s| s.get("count")).and_then(|v| v.as_i64()).unwrap_or(0);
    let singer_count = data.get("singer").and_then(|s| s.get("count")).and_then(|v| v.as_i64()).unwrap_or(0);
    let album_count = data.get("album").and_then(|a| a.get("count")).and_then(|v| v.as_i64()).unwrap_or(0);

    if songs_count > 0 { order.push("songs"); }
    if singer_count > 0 { order.push("artists"); }
    if album_count > 0 { order.push("albums"); }

    let songs: Vec<serde_json::Value> = data.get("song").and_then(|s| s.get("itemlist")).and_then(|v| v.as_array())
        .cloned().unwrap_or_default().iter().map(|info| {
            serde_json::json!({
                "name": info.get("name").and_then(|v| v.as_str()).unwrap_or(""),
                "artist": { "name": info.get("singer").and_then(|v| v.as_str()).unwrap_or("") }
            })
        }).collect();

    let artists: Vec<serde_json::Value> = data.get("singer").and_then(|s| s.get("itemlist")).and_then(|v| v.as_array())
        .cloned().unwrap_or_default().iter().map(|info| {
            serde_json::json!({ "name": info.get("name").and_then(|v| v.as_str()).unwrap_or("") })
        }).collect();

    let albums: Vec<serde_json::Value> = data.get("album").and_then(|a| a.get("itemlist")).and_then(|v| v.as_array())
        .cloned().unwrap_or_default().iter().map(|info| {
            serde_json::json!({ "name": info.get("name").and_then(|v| v.as_str()).unwrap_or("") })
        }).collect();

    Ok(serde_json::json!({
        "list": { "order": order, "songs": songs, "artists": artists, "albums": albums },
        "source": "tx"
    }))
}

pub async fn hot_search(_args: serde_json::Value) -> Result<serde_json::Value, String> {
    let body = serde_json::json!({
        "comm": {
            "ct": "19", "cv": "1803", "guid": "0", "patch": "118",
            "psrf_access_token_expiresAt": 0, "psrf_qqaccess_token": "",
            "psrf_qqopenid": "", "psrf_qqunionid": "",
            "tmeAppID": "qqmusic", "tmeLoginType": 0, "uin": "0", "wid": "0"
        },
        "hotkey": {
            "method": "GetHotkeyForQQMusicPC",
            "module": "tencent_musicsoso_hotkey.HotkeyService",
            "param": { "search_id": "", "uin": 0 }
        }
    });

    let resp: serde_json::Value = get_http()
        .post("https://u.y.qq.com/cgi-bin/musicu.fcg")
        .header("User-Agent", "Mozilla/5.0 (compatible; MSIE 9.0; Windows NT 6.1; WOW64; Trident/5.0)")
        .header("Referer", "https://y.qq.com/portal/player.html")
        .json(&body)
        .send().await.map_err(|e| e.to_string())?
        .json_sanitized().await?;

    let code = resp.get("code").and_then(|v| v.as_i64()).unwrap_or(-1);
    if code != 0 {
        return Err("TX hot search API error".into());
    }

    let raw_list = resp.get("hotkey").and_then(|h| h.get("data"))
        .and_then(|d| d.get("vec_hotkey"))
        .and_then(|v| v.as_array()).cloned().unwrap_or_default();

    let list: Vec<String> = raw_list.iter()
        .filter_map(|item| item.get("query").and_then(|v| v.as_str()).map(|s| s.to_string()))
        .collect();

    Ok(serde_json::json!({ "source": "tx", "list": list }))
}

pub async fn search_playlist(args: serde_json::Value) -> Result<serde_json::Value, String> {
    use crate::music_sdk::client::PlaylistItem;
    let keyword = get_str(&args, "keyword");
    let page = get_u64(&args, "page", 1);
    let limit = get_u64(&args, "limit", 20);

    if keyword.is_empty() {
        return Ok(serde_json::json!({ "list": [], "allPage": 0, "limit": limit, "total": 0, "source": "tx" }));
    }

    let url = format!(
        "http://c.y.qq.com/soso/fcgi-bin/client_music_search_songlist?page_no={}&num_per_page={}&format=json&query={}&remoteplace=txt.yqq.playlist&inCharset=utf8&outCharset=utf-8",
        page - 1, limit, urlencoding::encode(keyword)
    );

    let resp: serde_json::Value = get_http().get(&url)
        .header("User-Agent", "Mozilla/5.0 (compatible; MSIE 9.0; Windows NT 6.1; WOW64; Trident/5.0)")
        .header("Referer", "http://y.qq.com/portal/search.html")
        .send().await.map_err(|e| e.to_string())?
        .json_sanitized().await?;

    let code = resp.get("code").and_then(|v| v.as_i64()).unwrap_or(-1);
    if code != 0 {
        return Err("TX search playlist API error".into());
    }

    let data = resp.get("data").cloned().unwrap_or(serde_json::json!({}));
    let total = data.get("sum").and_then(|v| v.as_i64()).unwrap_or(0);
    let raw_list = data.get("list").and_then(|v| v.as_array()).cloned().unwrap_or_default();

    let list: Vec<PlaylistItem> = raw_list.iter().map(|item| {
        let dissid = item.get("dissid").cloned().unwrap_or(serde_json::Value::Null);
        PlaylistItem {
            id: dissid,
            name: item.get("dissname").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            img: item.get("imgurl").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            source: "tx".into(),
            desc: item.get("introduction").and_then(|v| v.as_str()).unwrap_or("").replace("<br>", "\n"),
            play_count: item.get("listennum").cloned().unwrap_or(serde_json::Value::Null),
            author: item.get("creator").and_then(|c| c.get("name")).and_then(|v| v.as_str()).unwrap_or("").to_string(),
            total: serde_json::Value::Null,
        }
    }).collect();

    Ok(serde_json::to_value(crate::music_sdk::client::PlaylistResult {
        list, all_page: (total as f64 / limit as f64).ceil() as i64,
        limit: limit as i64, total, source: "tx".into(),
    }).unwrap())
}
