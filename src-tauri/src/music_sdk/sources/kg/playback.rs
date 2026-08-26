use super::helpers::*;
use crate::music_sdk::client::QualityInfo;
use std::collections::HashMap;

/// Get batch quality info for Kugou hashes.
/// Returns a map of hash -> (types_vec, types_map)
pub async fn get_batch_quality_info(
    hashes: &[String],
) -> Result<HashMap<String, (Vec<String>, HashMap<String, QualityInfo>)>, String> {
    if hashes.is_empty() {
        return Ok(HashMap::new());
    }

    let resources: Vec<serde_json::Value> = hashes
        .iter()
        .map(|hash| serde_json::json!({ "id": 0, "type": "audio", "hash": hash }))
        .collect();

    let timestamp = chrono::Utc::now().timestamp_millis();
    let url = format!(
        "https://gateway.kugou.com/goodsmstore/v1/get_res_privilege?appid=1005&clientver=20049&clienttime={}&mid=NeZha",
        timestamp
    );

    let body = serde_json::json!({
        "behavior": "play",
        "clientver": "20049",
        "resource": resources,
        "area_code": "1",
        "quality": "128",
        "qualities": ["128", "320", "flac", "high", "dolby", "viper_atmos", "viper_tape", "viper_clear"]
    });

    let resp: serde_json::Value = get_http()
        .post(&url)
        .header(
            "User-Agent",
            "Android712-AndroidPhone-20049-376-0-FeeCacheUpdate-wifi",
        )
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;

    let errcode = resp
        .get("error_code")
        .and_then(|v| v.as_i64())
        .unwrap_or(-1);
    if errcode != 0 {
        return Err("KuGou quality info API error".into());
    }

    let data = resp
        .get("data")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let mut map = HashMap::new();

    for (idx, song_data) in data.iter().enumerate() {
        let hash = hashes.get(idx).cloned().unwrap_or_default();
        let mut types = Vec::new();
        let mut types_map = HashMap::new();

        let relate_goods = match song_data.get("relate_goods").and_then(|v| v.as_array()) {
            Some(goods) => goods,
            None => continue,
        };

        for quality_data in relate_goods {
            let quality = quality_data
                .get("quality")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let file_size = quality_data
                .get("info")
                .and_then(|i| i.get("filesize"))
                .and_then(|v| v.as_i64())
                .unwrap_or(0);
            let q_hash = quality_data
                .get("hash")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let size_str = format_file_size(file_size);

            let (type_name, rank) = match quality {
                "128" => ("128k", 1),
                "320" => ("320k", 2),
                "flac" => ("flac", 3),
                "high" => ("hires", 4),
                "viper_atmos" => ("atmos", 5),
                "viper_clear" => ("master", 6),
                "viper_tape" => ("atmos_plus", 7),
                _ => continue,
            };

            types.push((
                rank,
                type_name.to_string(),
                size_str.clone(),
                q_hash.clone(),
            ));
            types_map.insert(
                type_name.to_string(),
                QualityInfo {
                    size: size_str,
                    hash: Some(q_hash),
                },
            );
        }

        types.sort_by_key(|t| std::cmp::Reverse(t.0));
        let types_vec: Vec<String> = types.iter().map(|t| t.1.clone()).collect();
        map.insert(hash, (types_vec, types_map));
    }

    Ok(map)
}

/// Get music play URL
pub async fn get_music_url(args: serde_json::Value) -> Result<serde_json::Value, String> {
    let songmid = args
        .get("songInfo")
        .and_then(|s| s.get("songmid"))
        .map(|v| match v {
            serde_json::Value::String(s) => s.clone(),
            serde_json::Value::Number(n) => n.to_string(),
            _ => String::new(),
        })
        .unwrap_or_default();
    let hash = args
        .get("songInfo")
        .and_then(|s| s.get("hash"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let quality = get_str(&args, "quality");

    // Use hash if available, otherwise use songmid

    // Use hash if available, otherwise use songmid
    let target_hash = if !hash.is_empty() {
        hash
    } else {
        songmid.clone()
    };

    let url = format!(
        "https://trackercdn.kugou.com/i/v2/?appid=1005&clientver=20049&cmd=25&pid=1&behavior=play&hash={}",
        target_hash
    );

    let resp: serde_json::Value = get_http()
        .get(&url)
        .header(
            "User-Agent",
            "Android712-AndroidPhone-20049-376-0-FeeCacheUpdate-wifi",
        )
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;

    let urls = resp
        .get("url")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let play_url = urls
        .first()
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    Ok(serde_json::json!({ "url": play_url, "type": quality }))
}

/// Get album art
pub async fn get_pic(args: serde_json::Value) -> Result<serde_json::Value, String> {
    let song_info = args
        .get("songInfo")
        .cloned()
        .unwrap_or(serde_json::json!({}));
    let songmid = song_info
        .get("songmid")
        .map(|v| match v {
            serde_json::Value::String(s) => s.clone(),
            serde_json::Value::Number(n) => n.to_string(),
            _ => String::new(),
        })
        .unwrap_or_default();
    let hash = song_info.get("hash").and_then(|v| v.as_str()).unwrap_or("");
    let album_id = song_info
        .get("albumId")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);
    let singer = song_info
        .get("singer")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let name = song_info.get("name").and_then(|v| v.as_str()).unwrap_or("");

    // If songmid length is 32, use audioId split (matches JS logic)
    let album_audio_id = if songmid.len() == 32 {
        song_info
            .get("audioId")
            .and_then(|v| v.as_str())
            .and_then(|s| s.split('_').next())
            .unwrap_or(&songmid)
            .to_string()
    } else {
        songmid.clone()
    };

    let body = serde_json::json!({
        "appid": 1001,
        "area_code": "1",
        "behavior": "play",
        "clientver": "9020",
        "need_hash_offset": 1,
        "relate": 1,
        "resource": [{
            "album_audio_id": album_audio_id,
            "album_id": album_id,
            "hash": hash,
            "id": 0,
            "name": format!("{} - {}.mp3", singer, name),
            "type": "audio"
        }],
        "token": "",
        "userid": 2626431536_u64,
        "vip": 1
    });

    let resp: serde_json::Value = get_http()
        .post("http://media.store.kugou.com/v1/get_res_privilege")
        .header("KG-RC", "1")
        .header("KG-THash", "expand_search_manager.cpp:852736169:451")
        .header("User-Agent", "KuGou2012-9020-ExpandSearchManager")
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;

    let errcode = resp
        .get("error_code")
        .and_then(|v| v.as_i64())
        .unwrap_or(-1);
    if errcode != 0 {
        return Err("KuGou pic API error".into());
    }

    let data_arr = resp
        .get("data")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let info = data_arr
        .first()
        .and_then(|d| d.get("info"))
        .cloned()
        .unwrap_or(serde_json::json!({}));

    let img = if let Some(imgsize_arr) = info.get("imgsize").and_then(|v| v.as_array()) {
        let size = imgsize_arr
            .first()
            .and_then(|v| v.as_str())
            .unwrap_or("400");
        info.get("image")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .replace("{size}", size)
    } else {
        info.get("image")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string()
    };

    Ok(serde_json::json!({ "url": img }))
}

/// Get lyrics (search + download + decode KRC)
pub async fn get_lyric(args: serde_json::Value) -> Result<serde_json::Value, String> {
    let song_info = args
        .get("songInfo")
        .cloned()
        .unwrap_or(serde_json::json!({}));
    let name = song_info
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let songmid = song_info
        .get("songmid")
        .map(|v| match v {
            serde_json::Value::String(s) => s.clone(),
            serde_json::Value::Number(n) => n.to_string(),
            _ => String::new(),
        })
        .unwrap_or_default();

    let hash = song_info
        .get("hash")
        .and_then(|v| v.as_str())
        .filter(|v| !v.is_empty())
        .map(|v| v.to_string())
        .unwrap_or(songmid);

    if hash.is_empty() {
        return Ok(serde_json::json!({ "lyric": "", "tlyric": "", "source": "kg" }));
    }

    let interval = song_info
        .get("interval")
        .map(|v| match v {
            serde_json::Value::String(s) => s.clone(),
            serde_json::Value::Number(n) => n.to_string(),
            _ => "0".to_string(),
        })
        .unwrap_or_else(|| "0".to_string());

    let timelength = song_info
        .get("_interval")
        .and_then(|v| match v {
            serde_json::Value::Number(n) => n.as_i64(),
            serde_json::Value::String(s) => s.parse::<i64>().ok(),
            _ => None,
        })
        .unwrap_or_else(|| parse_interval_to_seconds(&interval));

    let search_info = match search_lyric(&name, &hash, timelength).await {
        Ok(Some(info)) => Some(info),
        _ => {
            if hash.len() != 32 {
                match search_lyric(&name, "", timelength).await {
                    Ok(Some(info)) => Some(info),
                    _ => None,
                }
            } else {
                None
            }
        }
    };

    let Some(search_info) = search_info else {
        return Ok(serde_json::json!({ "lyric": "", "tlyric": "", "source": "kg" }));
    };

    let download_resp =
        match get_lyric_download(search_info.id, &search_info.access_key, &search_info.fmt).await {
            Ok(Some(resp)) => resp,
            Ok(None) => {
                return Ok(serde_json::json!({ "lyric": "", "tlyric": "", "source": "kg" }))
            }
            Err(_) => return Ok(serde_json::json!({ "lyric": "", "tlyric": "", "source": "kg" })),
        };

    let content = download_resp
        .get("content")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let fmt_result = download_resp
        .get("fmt")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    match fmt_result {
        "krc" => decode_krc(content),
        "lrc" => {
            use base64::Engine;
            let decoded = base64::engine::general_purpose::STANDARD
                .decode(content)
                .map_err(|e| e.to_string())?;
            let lyric = String::from_utf8_lossy(&decoded).to_string();
            Ok(serde_json::json!({ "lyric": lyric, "tlyric": "", "source": "kg" }))
        }
        _ => Ok(serde_json::json!({ "lyric": "", "tlyric": "", "source": "kg" })),
    }
}

struct KgLyricSearchInfo {
    id: i64,
    access_key: String,
    fmt: String,
}

async fn search_lyric(
    name: &str,
    hash: &str,
    timelength: i64,
) -> Result<Option<KgLyricSearchInfo>, String> {
    let search_url = format!(
        "http://lyrics.kugou.com/search?ver=1&man=yes&client=pc&keyword={}&hash={}&timelength={}&lrctxt=1",
        urlencoding::encode(name), hash, timelength
    );

    for _ in 0..=5 {
        let resp = match get_http()
            .get(&search_url)
            .header("KG-RC", "1")
            .header("KG-THash", "expand_search_manager.cpp:852736169:451")
            .header("User-Agent", "KuGou2012-9020-ExpandSearchManager")
            .send()
            .await
        {
            Ok(r) => r,
            Err(_) => continue,
        };

        if !resp.status().is_success() {
            continue;
        }

        let body: serde_json::Value = match resp.json().await {
            Ok(v) => v,
            Err(_) => continue,
        };

        let candidates = body
            .get("candidates")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        if candidates.is_empty() {
            return Ok(None);
        }

        let parse_i64 = |v: Option<&serde_json::Value>| -> i64 {
            match v {
                Some(serde_json::Value::Number(n)) => n.as_i64().unwrap_or(0),
                Some(serde_json::Value::String(s)) => s.parse::<i64>().unwrap_or(0),
                _ => 0,
            }
        };

        for candidate in &candidates {
            let id = parse_i64(candidate.get("id"));
            let access_key = candidate
                .get("accesskey")
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
                .or_else(|| candidate.get("accessKey").and_then(|v| v.as_str()).filter(|s| !s.is_empty()))
                .unwrap_or("")
                .to_string();
            let krctype = parse_i64(candidate.get("krctype"));
            let contenttype = parse_i64(candidate.get("contenttype"));
            let fmt = if krctype == 1 && contenttype != 1 {
                "krc"
            } else {
                "lrc"
            }
            .to_string();

            if id > 0 && !access_key.is_empty() {
                return Ok(Some(KgLyricSearchInfo {
                    id,
                    access_key,
                    fmt,
                }));
            }
        }

        return Ok(None);
    }

    Err("Get lyric failed".into())
}

async fn get_lyric_download(
    id: i64,
    access_key: &str,
    fmt: &str,
) -> Result<Option<serde_json::Value>, String> {
    let download_url = format!(
        "http://lyrics.kugou.com/download?ver=1&client=pc&id={}&accesskey={}&fmt={}&charset=utf8",
        id, access_key, fmt
    );

    for _ in 0..=5 {
        let resp = match get_http()
            .get(&download_url)
            .header("KG-RC", "1")
            .header("KG-THash", "expand_search_manager.cpp:852736169:451")
            .header("User-Agent", "KuGou2012-9020-ExpandSearchManager")
            .send()
            .await
        {
            Ok(r) => r,
            Err(_) => continue,
        };

        if !resp.status().is_success() {
            continue;
        }

        let body: serde_json::Value = match resp.json().await {
            Ok(v) => v,
            Err(_) => continue,
        };

        return Ok(Some(body));
    }

    Err("Get lyric failed".into())
}

fn parse_interval_to_seconds(interval: &str) -> i64 {
    let parts: Vec<&str> = interval.split(':').collect();
    if parts.len() == 2 {
        let m: i64 = parts[0].parse().unwrap_or(0);
        let s: i64 = parts[1].parse().unwrap_or(0);
        m * 60 + s
    } else {
        interval.parse().unwrap_or(0)
    }
}

/// Decode KRC lyric format (XOR + zlib inflate)
fn decode_krc(content: &str) -> Result<serde_json::Value, String> {
    use base64::Engine;
    use flate2::read::ZlibDecoder;
    use std::io::Read;

    let enc_key: [u8; 16] = [
        0x40, 0x47, 0x61, 0x77, 0x5e, 0x32, 0x74, 0x47, 0x51, 0x36, 0x31, 0x2d, 0xce, 0xd2, 0x6e,
        0x69,
    ];

    let raw = base64::engine::general_purpose::STANDARD
        .decode(content)
        .map_err(|e| format!("KRC base64 decode error: {}", e))?;

    if raw.len() < 4 {
        return Err("KRC data too short".into());
    }

    // Skip first 4 bytes (header), XOR with key
    let mut decoded = raw[4..].to_vec();
    for i in 0..decoded.len() {
        decoded[i] ^= enc_key[i % 16];
    }

    // Zlib inflate
    let mut decoder = ZlibDecoder::new(&decoded[..]);
    let mut decompressed = String::new();
    decoder
        .read_to_string(&mut decompressed)
        .map_err(|e| format!("KRC zlib decompress error: {}", e))?;

    // Parse the KRC format into standard LRC
    parse_krc_lyric(&decompressed)
}

fn parse_krc_lyric(str: &str) -> Result<serde_json::Value, String> {
    let mut content = str.replace('\r', "");

    // Remove header line [id:$xxx]
    if let Some(pos) = content.find('\n') {
        if content[..pos].starts_with("[id:$") {
            content = content[pos + 1..].to_string();
        }
    }

    // Extract translated lyrics if present
    let mut tlyric = String::new();
    let mut crlyric_lines: Vec<String> = Vec::new();
    let mut lyric_lines: Vec<String> = Vec::new();

    // Check for [language:...] tag
    let lang_regex = regex_lite::Regex::new(r"\[language:([\w=\\/+]+)\]").ok();
    let lang_content = if let Some(re) = &lang_regex {
        re.captures(&content).map(|caps| caps[1].to_string())
    } else {
        None
    };

    if let Some(lang_data) = &lang_content {
        content = content.replace(&format!("[language:{}]", lang_data), "");
        // The language data is base64-encoded JSON with translated/romanized lyrics
        use base64::Engine;
        if let Ok(json_bytes) = base64::engine::general_purpose::STANDARD.decode(lang_data) {
            if let Ok(json_str) = String::from_utf8(json_bytes) {
                if let Ok(lang_json) = serde_json::from_str::<serde_json::Value>(&json_str) {
                    if let Some(items) = lang_json.get("content").and_then(|v| v.as_array()) {
                        for item in items {
                            let item_type = item.get("type").and_then(|v| v.as_i64()).unwrap_or(-1);
                            let lyric_content = item.get("lyricContent").and_then(|v| v.as_array());
                            if item_type == 1 {
                                // Translation lyrics
                                if let Some(lines) = lyric_content {
                                    tlyric = lines
                                        .iter()
                                        .filter_map(|line| line.as_array())
                                        .map(|words| {
                                            words
                                                .iter()
                                                .filter_map(|w| w.as_str())
                                                .collect::<Vec<_>>()
                                                .join("")
                                        })
                                        .collect::<Vec<_>>()
                                        .join("\n");
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Parse main lyric with [time,duration] word-by-word format
    let word_regex = regex_lite::Regex::new(r"\[(\d+),\d+\](.*)").unwrap();
    let word_detail_regex = regex_lite::Regex::new(r"<(\d+),(\d+),\d+>([^<]*)").unwrap();

    for line in content.lines() {
        if let Some(caps) = word_regex.captures(line) {
            let line_start_ms: i64 = caps[1].parse().unwrap_or(0);
            let time_tag = ms_to_lrc_time(line_start_ms);
            let word_content = &caps[2];

            // Extract plain text (remove word timing tags)
            let plain_text = word_detail_regex.replace_all(word_content, "$3");
            lyric_lines.push(format!("[{}]{}", time_tag, plain_text));

            // Build crlyric with absolute timestamps
            let mut cr_words = String::new();
            for wc in word_detail_regex.captures_iter(word_content) {
                let offset: i64 = wc[1].parse().unwrap_or(0);
                let duration: i64 = wc[2].parse().unwrap_or(0);
                let word_text = &wc[3];
                let abs_time = line_start_ms + offset;
                cr_words.push_str(&format!("({},{},{})", abs_time, duration, word_text));
            }
            crlyric_lines.push(format!("[{}]{}", time_tag, cr_words));
        }
    }

    let lyric = lyric_lines.join("\n");
    let crlyric = crlyric_lines.join("\n");

    let final_lyric = if lyric.trim().is_empty() {
        let standard_lrc_regex = regex_lite::Regex::new(r"^\[\d{1,2}:\d{2}(?:\.\d{1,3})?\].+").unwrap();
        let line_tag_regex = regex_lite::Regex::new(r"^\[\d+,\d+\]").unwrap();
        let word_tag_regex = regex_lite::Regex::new(r"<\d+,\d+,\d+>").unwrap();

        let standard_lrc_lines: Vec<String> = content
            .lines()
            .map(str::trim)
            .filter(|line| standard_lrc_regex.is_match(line))
            .map(|line| line.to_string())
            .collect();

        if !standard_lrc_lines.is_empty() {
            standard_lrc_lines.join("\n")
        } else {
            let fallback_lines: Vec<String> = content
                .lines()
                .map(str::trim)
                .filter(|line| !line.is_empty())
                .filter(|line| !line.starts_with("[ti:") && !line.starts_with("[ar:") && !line.starts_with("[al:") && !line.starts_with("[by:") && !line.starts_with("[offset:"))
                .map(|line| line_tag_regex.replace(line, "").to_string())
                .map(|line| word_tag_regex.replace_all(&line, "").to_string())
                .map(|line| line.trim().to_string())
                .filter(|line| !line.is_empty() && !line.starts_with('['))
                .collect();

            fallback_lines
                .iter()
                .enumerate()
                .map(|(idx, line)| {
                    let time_tag = ms_to_lrc_time((idx as i64) * 2000);
                    format!("[{}]{}", time_tag, line)
                })
                .collect::<Vec<_>>()
                .join("\n")
        }
    } else {
        lyric
    };

    Ok(serde_json::json!({
        "lyric": final_lyric,
        "tlyric": tlyric,
        "crlyric": crlyric,
        "source": "kg"
    }))
}

fn ms_to_lrc_time(ms: i64) -> String {
    let total_secs = ms / 1000;
    let mins = total_secs / 60;
    let secs = total_secs % 60;
    let ms_rem = ms % 1000;
    format!("{:02}:{:02}.{:03}", mins, secs, ms_rem)
}
