use super::helpers::*;
use crate::music_sdk::client::ResponseExt;
use super::crypto::{qrc_decrypt, parse_qrc_lyrics};
use crate::music_sdk::client::QualityInfo;
use std::collections::HashMap;

/// Get song info by songmid (to get songId and strMediaMid)
async fn get_music_info(songmid: &str) -> Result<serde_json::Value, String> {
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
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; WOW64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/86.0.4240.198 Safari/537.36")
        .header("Referer", "https://y.qq.com")
        .json(&body)
        .send().await.map_err(|e| e.to_string())?
        .json_sanitized().await?;

    let code = resp.get("code").and_then(|v| v.as_i64()).unwrap_or(-1);
    let req_code = resp.get("req").and_then(|r| r.get("code")).and_then(|v| v.as_i64()).unwrap_or(-1);
    if code != 0 || req_code != 0 {
        return Err(format!("TX get music info failed: code={}, req_code={}", code, req_code));
    }

    let track_info = resp.get("req").and_then(|r| r.get("data"))
        .and_then(|d| d.get("track_info")).cloned()
        .unwrap_or(serde_json::json!({}));

    Ok(track_info)
}

/// Get batch quality info for QQ Music songs (by song IDs)
pub async fn get_batch_quality_info(song_ids: &[i64]) -> Result<HashMap<i64, (Vec<String>, HashMap<String, QualityInfo>)>, String> {
    if song_ids.is_empty() {
        return Ok(HashMap::new());
    }

    let types_arr: Vec<i64> = song_ids.iter().map(|_| 1).collect();
    let body = serde_json::json!({
        "comm": { "ct": "19", "cv": "1859", "uin": "0" },
        "req": {
            "module": "music.trackInfo.UniformRuleCtrl",
            "method": "CgiGetTrackInfo",
            "param": {
                "types": types_arr, "ids": song_ids, "ctx": 0
            }
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
        return Err("TX quality detail API error".into());
    }

    let tracks = resp.get("req").and_then(|r| r.get("data"))
        .and_then(|d| d.get("tracks"))
        .and_then(|v| v.as_array()).cloned().unwrap_or_default();

    let mut map = HashMap::new();
    for track in &tracks {
        let song_id = track.get("id").and_then(|v| v.as_i64()).unwrap_or(0);
        let file = track.get("file").cloned().unwrap_or(serde_json::json!({}));
        let (types, types_map) = parse_quality_types(&file);
        map.insert(song_id, (types, types_map));
    }

    Ok(map)
}

/// Get music play URL
pub async fn get_music_url(args: serde_json::Value) -> Result<serde_json::Value, String> {
    let song_info = args.get("songInfo").cloned().unwrap_or(serde_json::json!({}));
    let songmid = song_info.get("songmid").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let quality = get_str(&args, "quality");

    // Get strMediaMid for URL generation
    let str_media_mid = song_info.get("strMediaMid")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string());

    let file_mid = if let Some(mid) = str_media_mid {
        mid
    } else {
        // Fetch from musicInfo API
        let info = get_music_info(&songmid).await?;
        info.get("file").and_then(|f| f.get("media_mid"))
            .and_then(|v| v.as_str()).unwrap_or("").to_string()
    };

    if file_mid.is_empty() {
        return Ok(serde_json::json!({ "url": "", "type": quality }));
    }

    let type_map = match quality {
        "128k" => ("M500", "mp3", 128),
        "320k" => ("M800", "mp3", 320),
        "flac" => ("F000", "flac", 0),
        "hires" => ("RS02", "flac", 0),
        "master" => ("Q000", "flac", 0),
        _ => ("M500", "mp3", 128),
    };

    let filename = format!("{}{}.{}", type_map.0, file_mid, type_map.1);

    let vkey_body = serde_json::json!({
        "comm": { "ct": "6", "cv": 565 },
        "req_0": {
            "module": "vkey.GetVkeyServer",
            "method": "CgiGetVkey",
            "param": {
                "filename": [filename],
                "guid": "10000",
                "songmid": [songmid],
                "songtype": [0],
                "uin": "0",
                "loginflag": 1,
                "platform": "20"
            }
        }
    });

    let resp: serde_json::Value = get_http()
        .post("https://u.y.qq.com/cgi-bin/musicu.fcg")
        .header("User-Agent", "Mozilla/5.0 (compatible; MSIE 9.0; Windows NT 6.1; WOW64; Trident/5.0)")
        .json(&vkey_body)
        .send().await.map_err(|e| e.to_string())?
        .json_sanitized().await?;

    let req_data = resp.get("req_0").and_then(|r| r.get("data")).cloned().unwrap_or(serde_json::json!({}));
    let midurlinfo = req_data.get("midurlinfo").and_then(|v| v.as_array()).cloned().unwrap_or_default();
    let purl = midurlinfo.first()
        .and_then(|m| m.get("purl"))
        .and_then(|v| v.as_str()).unwrap_or("");

    let play_url = if purl.is_empty() {
        String::new()
    } else {
        let sips = req_data.get("sips").and_then(|v| v.as_array()).cloned().unwrap_or_default();
        let sip = sips.first().and_then(|v| v.as_str()).unwrap_or("https://dl.stream.qqmusic.qq.com/");
        format!("{}{}", sip, purl)
    };

    Ok(serde_json::json!({ "url": play_url, "type": quality }))
}

/// Get album art URL
pub async fn get_pic(args: serde_json::Value) -> Result<serde_json::Value, String> {
    let song_info = args.get("songInfo").cloned().unwrap_or(serde_json::json!({}));

    let album_id = song_info.get("albumId")
        .or_else(|| song_info.get("albumMid"))
        .or_else(|| song_info.get("album_mid"))
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let img = if !album_id.is_empty() && album_id != "空" {
        build_album_img(album_id)
    } else {
        let singer_list = song_info.get("singer").and_then(|v| v.as_array());
        if let Some(singers) = singer_list {
            if let Some(mid) = singers.first().and_then(|s| s.get("mid")).and_then(|v| v.as_str()) {
                build_singer_img(mid)
            } else {
                String::new()
            }
        } else {
            song_info.get("img").and_then(|v| v.as_str()).unwrap_or("").to_string()
        }
    };

    Ok(serde_json::json!({ "url": img }))
}

/// Get lyrics (with QRC decryption)
pub async fn get_lyric(args: serde_json::Value) -> Result<serde_json::Value, String> {
    let empty_result = || serde_json::json!({ "lyric": "", "tlyric": "", "crlyric": "", "rlyric": "", "source": "tx" });

    let song_info = args.get("songInfo").cloned().unwrap_or(serde_json::json!({}));
    let songmid = song_info.get("songmid").and_then(|v| v.as_str()).unwrap_or("");
    let song_id = song_info.get("songId")
        .or_else(|| song_info.get("song_id"))
        .and_then(|v| v.as_i64());

    // Get songId if not available
    let resolved_id = if let Some(id) = song_id {
        id
    } else {
        match get_music_info(songmid).await {
            Ok(info) => info.get("id").and_then(|v| v.as_i64()).unwrap_or(0),
            Err(e) => {
                println!("[TX Lyrics] get_music_info failed: {}", e);
                0
            }
        }
    };

    if resolved_id == 0 {
        return Ok(empty_result());
    }

    // Retry up to 3 times (matching JS reference behavior)
    let max_retries = 3;
    for attempt in 0..max_retries {
        let body = serde_json::json!({
            "comm": { "ct": "19", "cv": "1859", "uin": "0" },
            "req": {
                "method": "GetPlayLyricInfo",
                "module": "music.musichallSong.PlayLyricInfo",
                "param": {
                    "format": "json", "crypt": 1, "ct": 19, "cv": 1873,
                    "interval": 0, "lrc_t": 0, "qrc": 1, "qrc_t": 0,
                    "roma": 1, "roma_t": 0, "songID": resolved_id,
                    "trans": 1, "trans_t": 0, "type": -1
                }
            }
        });

        let resp_result: Result<serde_json::Value, String> = get_http()
            .post("https://u.y.qq.com/cgi-bin/musicu.fcg")
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; WOW64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/86.0.4240.198 Safari/537.36")
            .header("Referer", "https://y.qq.com")
            .json(&body)
            .send().await.map_err(|e| e.to_string())?
            .json().await.map_err(|e| e.to_string());

        let resp = match resp_result {
            Ok(r) => r,
            Err(e) => {
                println!("[TX Lyrics] API request failed (attempt {}/{}): {}", attempt + 1, max_retries, e);
                if attempt < max_retries - 1 { continue; }
                return Ok(empty_result());
            }
        };

        let code = resp.get("code").and_then(|v| v.as_i64()).unwrap_or(-1);
        let req_code = resp.get("req").and_then(|r| r.get("code")).and_then(|v| v.as_i64()).unwrap_or(-1);
        if code != 0 || req_code != 0 {
            println!("[TX Lyrics] API error (attempt {}/{}): code={}, req_code={}", attempt + 1, max_retries, code, req_code);
            if attempt < max_retries - 1 { continue; }
            return Ok(empty_result());
        }

        let data = resp.get("req").and_then(|r| r.get("data")).cloned().unwrap_or(serde_json::json!({}));

        let lrc_enc = data.get("lyric").and_then(|v| v.as_str()).unwrap_or("");
        let trans_enc = data.get("trans").and_then(|v| v.as_str()).unwrap_or("");
        let roma_enc = data.get("roma").and_then(|v| v.as_str()).unwrap_or("");

        let lrc = qrc_decrypt(lrc_enc).unwrap_or_default();
        let tlrc = qrc_decrypt(trans_enc).unwrap_or_default();
        let rlrc = qrc_decrypt(roma_enc).unwrap_or_default();

        return Ok(parse_qrc_lyrics(&lrc, &tlrc, &rlrc));
    }

    Ok(empty_result())
}
