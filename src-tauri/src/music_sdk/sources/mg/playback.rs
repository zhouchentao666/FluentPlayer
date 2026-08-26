use super::helpers::*;
use super::crypto::decrypt_mrc;

/// Get music play URL
pub async fn get_music_url(args: serde_json::Value) -> Result<serde_json::Value, String> {
    let song_info = args.get("songInfo").cloned().unwrap_or(serde_json::json!({}));
    let quality = get_str(&args, "quality");

    let song_id = song_info.get("songmid")
        .map(|v| match v {
            serde_json::Value::String(s) => s.clone(),
            serde_json::Value::Number(n) => n.to_string(),
            _ => String::new(),
        })
        .unwrap_or_default();

    let copyright_id = song_info.get("copyrightId")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let tone_flag = match quality {
        "128k" => "PQ",
        "320k" => "HQ",
        "flac" | "hires" => "SQ",
        _ => "SQ",
    };

    if !song_id.is_empty() && song_id != copyright_id {
        if let Some(url) = get_music_url_by_id(&song_id, tone_flag).await {
            return Ok(serde_json::json!({ "url": url, "type": quality }));
        }
    }

    if !copyright_id.is_empty() {
        if let Some(url) = get_music_url_by_copyright(&copyright_id, tone_flag).await {
            return Ok(serde_json::json!({ "url": url, "type": quality }));
        }
    }

    Ok(serde_json::json!({ "url": "", "type": quality }))
}

async fn get_music_url_by_id(song_id: &str, tone_flag: &str) -> Option<String> {
    let url = format!(
        "https://app.c.nf.migu.cn/MIGUM2.0/v2.0/content/listen-url?netType=01&resourceType=2&songId={}&toneFlag={}",
        song_id, tone_flag
    );

    let resp: serde_json::Value = get_http().get(&url)
        .header("User-Agent", "migu")
        .header("channel", "0146921")
        .send().await.ok()?
        .json().await.ok()?;

    if resp.get("code").and_then(|v| v.as_str()) != Some("000000") {
        return None;
    }

    let play_url = resp.get("data")?
        .get("songPlayUrl")?
        .as_str()?;
    if play_url.is_empty() { None } else { Some(play_url.to_string()) }
}

async fn get_music_url_by_copyright(copyright_id: &str, tone_flag: &str) -> Option<String> {
    let url = format!(
        "https://app.c.nf.migu.cn/MIGUM2.0/v2.0/content/listen-url?copyrightId={}&contentId={}&resourceType=2&netType=01&toneFlag={}",
        copyright_id, copyright_id, tone_flag
    );

    let resp: serde_json::Value = get_http().get(&url)
        .header("User-Agent", "migu")
        .header("channel", "0146921")
        .send().await.ok()?
        .json().await.ok()?;

    if resp.get("code").and_then(|v| v.as_str()) != Some("000000") {
        return None;
    }

    let play_url = resp.get("data")?
        .get("songPlayUrl")?
        .as_str()?;
    if play_url.is_empty() { None } else { Some(play_url.to_string()) }
}

/// Get album art — fetches image via backend to bypass CORS
pub async fn get_pic(args: serde_json::Value) -> Result<serde_json::Value, String> {
    let song_info = args.get("songInfo").cloned().unwrap_or(serde_json::json!({}));

    let img_url = song_info.get("img").and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .unwrap_or_else(|| get_song_img(&song_info));

    if img_url.is_empty() {
        return Ok(serde_json::json!({ "url": "" }));
    }

    // Fetch image through backend to avoid CORS
    let resp = get_http().get(&img_url)
        .headers(mg_headers())
        .send().await;

    match resp {
        Ok(response) => {
            let content_type = response.headers()
                .get("content-type")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("image/jpeg")
                .to_string();
            let bytes = response.bytes().await.map_err(|e| e.to_string())?;
            let b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &bytes);
            Ok(serde_json::json!({ "url": format!("data:{};base64,{}", content_type, b64) }))
        }
        Err(_) => {
            Ok(serde_json::json!({ "url": img_url }))
        }
    }
}

/// Get lyrics
pub async fn get_lyric(args: serde_json::Value) -> Result<serde_json::Value, String> {
    let song_info = args.get("songInfo").cloned().unwrap_or(serde_json::json!({}));
    let songmid = song_info.get("songmid")
        .map(|v| match v {
            serde_json::Value::String(s) => s.clone(),
            serde_json::Value::Number(n) => n.to_string(),
            _ => String::new(),
        })
        .unwrap_or_default();
    let copyright_id = song_info.get("copyrightId")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    // Try MRC (encrypted lyric) first
    let mrc_url = song_info.get("mrcUrl")
        .and_then(|v| v.as_str()).unwrap_or("");
    if !mrc_url.is_empty() {
        if let Ok(text) = get_text_from_url(mrc_url).await {
            let decrypted = decrypt_mrc(&text);
            let parsed = parse_mrc_lyrics(&decrypted);
            let lyric = parsed.get("lyric").and_then(|v| v.as_str()).unwrap_or("");
            let crlyric = parsed.get("crlyric").and_then(|v| v.as_str()).unwrap_or("");
            let tlyric = get_optional_text(song_info.get("trcUrl").and_then(|v| v.as_str()).unwrap_or("")).await;
            return Ok(serde_json::json!({
                "lyric": lyric, "crlyric": crlyric, "tlyric": tlyric, "source": "mg"
            }));
        }
    }

    // Try LRC URL from song info
    let lrc_url = song_info.get("lrcUrl")
        .and_then(|v| v.as_str()).unwrap_or("");
    if !lrc_url.is_empty() {
        if let Ok(text) = get_text_from_url(lrc_url).await {
            let tlyric = get_optional_text(song_info.get("trcUrl").and_then(|v| v.as_str()).unwrap_or("")).await;
            return Ok(serde_json::json!({
                "lyric": text, "crlyric": "", "tlyric": tlyric, "source": "mg"
            }));
        }
    }

    // Try web API with copyrightId (preferred) or songmid
    let lyric_id = if !copyright_id.is_empty() { &copyright_id } else { &songmid };
    if !lyric_id.is_empty() {
        let url = format!(
            "https://music.migu.cn/v3/api/music/audioPlayer/getLyric?copyrightId={}",
            lyric_id
        );
        if let Ok(resp) = get_http().get(&url)
            .header("Referer", "https://music.migu.cn/v3/music/player/audio?from=migu")
            .send().await
        {
            if let Ok(json) = resp.json::<serde_json::Value>().await {
                let return_code = json.get("returnCode").and_then(|v| v.as_str()).unwrap_or("");
                if return_code == "000000" {
                    if let Some(lyric) = json.get("lyric").and_then(|v| v.as_str()) {
                        return Ok(serde_json::json!({
                            "lyric": lyric, "crlyric": "", "tlyric": "", "source": "mg"
                        }));
                    }
                }
            }
        }
    }

    Ok(serde_json::json!({ "lyric": "", "crlyric": "", "tlyric": "", "source": "mg" }))
}

fn parse_mrc_lyrics(str: &str) -> serde_json::Value {
    let line_time_re = regex_lite::Regex::new(r"^\s*\[(\d+),\d+\]").unwrap();
    let word_time_re = regex_lite::Regex::new(r"\(\d+,\d+\)").unwrap();

    let mut lrc_lines = Vec::new();
    let mut lxlrc_lines = Vec::new();

    for line in str.replace('\r', "").lines() {
        if line.len() < 6 { continue; }
        if let Some(caps) = line_time_re.captures(line) {
            let start_time: i64 = caps[1].parse().unwrap_or(0);
            let ms = start_time % 1000;
            let total_secs = start_time / 1000;
            let m = total_secs / 60;
            let s = total_secs % 60;
            let time_str = format!("[{:02}:{:02}.{:03}]", m, s, ms);

            let words = line_time_re.replace(line, "");
            let plain = word_time_re.replace_all(&words, "");
            lrc_lines.push(format!("{}{}", time_str, plain));

            // Build LXLRC (word-by-word with relative timestamps)
            if let Some(times) = word_time_re.find_iter(&words).collect::<Vec<_>>().first().map(|_| {
                word_time_re.find_iter(&words).collect::<Vec<_>>()
            }) {
                let word_arr: Vec<&str> = word_time_re.split(&words).collect();
                let mut lxlrc_parts = Vec::new();
                for (idx, m) in times.iter().enumerate() {
                    let t_str = &words[m.start() + 1..m.end() - 1]; // Remove ( and )
                    let parts: Vec<&str> = t_str.split(',').collect();
                    if parts.len() >= 2 {
                        let offset: i64 = parts[0].parse::<i64>().unwrap_or(0) - start_time;
                        let dur: i64 = parts[1].parse().unwrap_or(0);
                        let text = word_arr.get(idx).unwrap_or(&"");
                        lxlrc_parts.push(format!("<{},{}>{}", offset, dur, text));
                    }
                }
                lxlrc_lines.push(format!("{}{}", time_str, lxlrc_parts.join("")));
            }
        }
    }

    serde_json::json!({
        "lyric": lrc_lines.join("\n"),
        "crlyric": lxlrc_lines.join("\n")
    })
}

async fn get_text_from_url(url: &str) -> Result<String, String> {
    let resp = get_http().get(url)
        .header("Referer", "https://app.c.nf.migu.cn/")
        .header("User-Agent", "Mozilla/5.0 (Linux; Android 5.1.1; Nexus 6 Build/LYZ28E) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/59.0.3071.115 Mobile Safari/537.36")
        .header("channel", "0146921")
        .send().await.map_err(|e| e.to_string())?;

    let status = resp.status();
    if status != 200 {
        return Err(format!("HTTP {}", status));
    }

    resp.text().await.map_err(|e| e.to_string())
}

async fn get_optional_text(url: &str) -> String {
    if url.is_empty() { return String::new(); }
    get_text_from_url(url).await.unwrap_or_default()
}

#[allow(dead_code)]
async fn get_music_info_by_copyright(copyright_id: &str) -> Option<serde_json::Value> {
    let url = "https://c.musicapp.migu.cn/MIGUM2.0/v1.0/content/resourceinfo.do?resourceType=2";
    let body = format!("resourceId={}", copyright_id);

    let resp: serde_json::Value = get_http().post(url)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .header("User-Agent", "migu")
        .header("channel", "0146921")
        .body(body)
        .send().await.ok()?
        .json().await.ok()?;

    let code = resp.get("code").and_then(|v| v.as_str()).unwrap_or("");
    if code != "000000" { return None; }

    resp.get("resource")
        .and_then(|r| r.as_array())
        .and_then(|a| a.first())
        .cloned()
}
