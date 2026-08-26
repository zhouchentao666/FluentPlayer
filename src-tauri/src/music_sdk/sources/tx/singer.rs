use super::helpers::*;
use crate::music_sdk::client::ResponseExt;
use crate::music_sdk::client::{MusicItem, SearchResult, SingerInfo, SingerDetail, SingerCount, SingerAlbumItem, AlbumBrief, SingerAlbumListResult};

pub async fn get_singer_info(args: serde_json::Value) -> Result<serde_json::Value, String> {
    let id = get_str(&args, "id");
    if id.is_empty() {
        return Err("TX singer info: missing id".into());
    }

    let body = serde_json::json!({
        "comm": { "cv": 4747474, "ct": 24, "format": "json", "inCharset": "utf-8", "outCharset": "utf-8", "uin": 0 },
        "req_1": {
            "module": "music.musichallSinger.SingerInfoInter",
            "method": "GetSingerDetail",
            "param": {
                "singer_mid": [id], "ex_singer": 1, "wiki_singer": 1, "group_singer": 0, "pic": 1, "photos": 0
            }
        },
        "req_2": {
            "module": "music.musichallAlbum.AlbumListServer",
            "method": "GetAlbumList",
            "param": { "singerMid": id, "order": 0, "begin": 0, "num": 1, "songNumTag": 0, "singerID": 0 }
        },
        "req_3": {
            "module": "musichall.song_list_server",
            "method": "GetSingerSongList",
            "param": { "singerMid": id, "order": 1, "begin": 0, "num": 1 }
        }
    });

    let resp: serde_json::Value = get_http()
        .post("https://u.y.qq.com/cgi-bin/musicu.fcg")
        .header("User-Agent", "Mozilla/5.0 (compatible; MSIE 9.0; Windows NT 6.1; WOW64; Trident/5.0)")
        .json(&body)
        .send().await.map_err(|e| e.to_string())?
        .json_sanitized().await?;

    let req1_code = resp.get("req_1").and_then(|r| r.get("code")).and_then(|v| v.as_i64()).unwrap_or(-1);
    if req1_code != 0 {
        return Err(format!("TX singer info API error: req_1.code={}", req1_code));
    }

    let singer_list = resp.get("req_1").and_then(|r| r.get("data")).and_then(|d| d.get("singer_list"))
        .and_then(|v| v.as_array()).and_then(|a| a.first()).cloned().unwrap_or(serde_json::json!({}));
    let basic = singer_list.get("basic_info").cloned().unwrap_or(serde_json::json!({}));
    let ex = singer_list.get("ex_info").cloned().unwrap_or(serde_json::json!({}));
    let pic = singer_list.get("pic").cloned().unwrap_or(serde_json::json!({}));

    let music_total = resp.get("req_3").and_then(|r| r.get("data")).and_then(|d| d.get("totalNum")).and_then(|v| v.as_i64()).unwrap_or(0);
    let album_total = resp.get("req_2").and_then(|r| r.get("data")).and_then(|d| d.get("total")).and_then(|v| v.as_i64()).unwrap_or(0);

    let info = SingerInfo {
        id: serde_json::json!(id),
        source: "tx".into(),
        info: SingerDetail {
            name: basic.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            desc: ex.get("desc").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            avatar: pic.get("pic").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            gender: match ex.get("genre").and_then(|v| v.as_i64()).unwrap_or(0) {
                1 => Some("man".into()),
                2 => Some("woman".into()),
                _ => None,
            },
        },
        count: SingerCount {
            music: music_total,
            album: album_total,
        },
    };

    Ok(serde_json::to_value(info).unwrap())
}

pub async fn get_singer_song_list(args: serde_json::Value) -> Result<serde_json::Value, String> {
    let id = get_str(&args, "id");
    let page = get_u64(&args, "page", 1);
    let limit = get_u64(&args, "limit", 30);

    if id.is_empty() {
        return Err("TX singer song list: missing id".into());
    }

    let begin = limit * (page - 1);
    let body = serde_json::json!({
        "comm": { "cv": 4747474, "ct": 24, "format": "json", "inCharset": "utf-8", "outCharset": "utf-8", "uin": 0 },
        "req": {
            "module": "musichall.song_list_server",
            "method": "GetSingerSongList",
            "param": { "singerMid": id, "order": 1, "begin": begin, "num": limit }
        }
    });

    let resp: serde_json::Value = get_http()
        .post("https://u.y.qq.com/cgi-bin/musicu.fcg")
        .header("User-Agent", "Mozilla/5.0 (compatible; MSIE 9.0; Windows NT 6.1; WOW64; Trident/5.0)")
        .json(&body)
        .send().await.map_err(|e| e.to_string())?
        .json_sanitized().await?;

    let req_code = resp.get("req").and_then(|r| r.get("code")).and_then(|v| v.as_i64()).unwrap_or(-1);
    if req_code != 0 {
        return Err(format!("TX singer song list API error: req.code={}", req_code));
    }

    let total = resp.get("req").and_then(|r| r.get("data")).and_then(|d| d.get("totalNum")).and_then(|v| v.as_i64()).unwrap_or(0);
    let raw_list = resp.get("req").and_then(|r| r.get("data")).and_then(|d| d.get("songList"))
        .and_then(|v| v.as_array()).cloned().unwrap_or_default();

    let list: Vec<MusicItem> = raw_list.iter().filter_map(|item| {
        let song_info = item.get("songInfo")?;
        let file = song_info.get("file")?;
        let media_mid = file.get("media_mid").and_then(|v| v.as_str()).unwrap_or("");
        if media_mid.is_empty() { return None; }

        let songmid = song_info.get("mid").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let song_id = song_info.get("id").and_then(|v| v.as_i64()).unwrap_or(0);
        let singer_list = song_info.get("singer").and_then(|v| v.as_array()).cloned().unwrap_or_default();
        let album = song_info.get("album");
        let album_mid = album.and_then(|a| a.get("mid")).and_then(|v| v.as_str()).unwrap_or("").to_string();
        let interval = song_info.get("interval").and_then(|v| v.as_i64()).unwrap_or(0);

        let (types, types_map) = parse_quality_types(file);

        Some(MusicItem {
            songmid: serde_json::json!(songmid),
            singer: format_singer(&singer_list),
            name: song_info.get("title").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            album_name: album.and_then(|a| a.get("name")).and_then(|v| v.as_str()).unwrap_or("").to_string(),
            album_id: serde_json::json!(album.and_then(|a| a.get("id")).and_then(|v| v.as_i64()).unwrap_or(0)),
            source: "tx".into(),
            interval: format_play_time(interval),
            img: get_song_img(song_info, &album_mid),
            lrc: None,
            types: if types.is_empty() { None } else { Some(types) },
            types_map: if types_map.is_empty() { None } else { Some(types_map) },
            type_url: Some(serde_json::json!({})),
            hash: None,
            song_id: Some(serde_json::json!(song_id)),
            str_media_mid: Some(media_mid.to_string()),
            album_mid: Some(album_mid),
            copyright_id: None,
            lrc_url: None,
            mrc_url: None,
            trc_url: None,
            singer_id: None,
        })
    }).collect();

    Ok(serde_json::to_value(SearchResult {
        list, all_page: (total as f64 / limit as f64).ceil() as i64,
        limit: limit as i64, total, source: "tx".into(),
    }).unwrap())
}

pub async fn get_singer_album_list(args: serde_json::Value) -> Result<serde_json::Value, String> {
    let id = get_str(&args, "id");
    let page = get_u64(&args, "page", 1);
    let limit = get_u64(&args, "limit", 30);

    if id.is_empty() {
        return Err("TX singer album list: missing id".into());
    }

    let begin = limit * (page - 1);
    let body = serde_json::json!({
        "comm": { "cv": 4747474, "ct": 24, "format": "json", "inCharset": "utf-8", "outCharset": "utf-8", "uin": 0 },
        "req": {
            "module": "music.musichallAlbum.AlbumListServer",
            "method": "GetAlbumList",
            "param": { "singerMid": id, "order": 0, "begin": begin, "num": limit, "songNumTag": 0, "singerID": 0 }
        }
    });

    let resp: serde_json::Value = get_http()
        .post("https://u.y.qq.com/cgi-bin/musicu.fcg")
        .header("User-Agent", "Mozilla/5.0 (compatible; MSIE 9.0; Windows NT 6.1; WOW64; Trident/5.0)")
        .json(&body)
        .send().await.map_err(|e| e.to_string())?
        .json_sanitized().await?;

    let req_code = resp.get("req").and_then(|r| r.get("code")).and_then(|v| v.as_i64()).unwrap_or(-1);
    if req_code != 0 {
        return Err(format!("TX singer album list API error: req.code={}", req_code));
    }

    let total = resp.get("req").and_then(|r| r.get("data")).and_then(|d| d.get("total")).and_then(|v| v.as_i64()).unwrap_or(0);
    let raw_list = resp.get("req").and_then(|r| r.get("data")).and_then(|d| d.get("albumList"))
        .and_then(|v| v.as_array()).cloned().unwrap_or_default();

    let list: Vec<SingerAlbumItem> = raw_list.iter().filter_map(|item| {
        let album_mid = item.get("albumMid").and_then(|v| v.as_str()).unwrap_or("");
        Some(SingerAlbumItem {
            id: item.get("albumID").cloned().unwrap_or(serde_json::Value::Null),
            count: item.get("totalNum").and_then(|v| v.as_i64()).unwrap_or(0),
            info: AlbumBrief {
                name: item.get("albumName").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                author: item.get("singerName").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                img: if album_mid.is_empty() {
                    String::new()
                } else {
                    format!("https://y.gtimg.cn/music/photo_new/T002R500x500M000{}.jpg", album_mid)
                },
                desc: None,
            },
        })
    }).collect();

    Ok(serde_json::to_value(SingerAlbumListResult {
        list, all_page: (total as f64 / limit as f64).ceil() as i64,
        limit: limit as i64, total, source: "tx".into(),
    }).unwrap())
}
