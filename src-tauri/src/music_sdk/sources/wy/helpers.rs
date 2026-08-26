use crate::music_sdk::client;
use crate::music_sdk::client::QualityInfo;
use std::collections::HashMap;

pub fn get_http() -> &'static reqwest::Client {
    client::get_client()
}

/// Read an HTTP response body as bytes, check status, then parse as JSON.
/// On parse failure the raw body (truncated) and diagnostic headers are included in the error.
pub async fn parse_json_response(resp: reqwest::Response) -> Result<serde_json::Value, String> {
    let status = resp.status();
    let url = resp.url().to_string();
    let content_encoding = resp.headers().get("content-encoding").cloned();
    let content_length = resp.headers().get("content-length").cloned();
    let bytes = resp.bytes().await.map_err(|e| format!("WY read body error [{}]: {}", url, e))?;

    if !status.is_success() {
        let preview = String::from_utf8_lossy(&bytes[..bytes.len().min(200)]);
        return Err(format!("WY HTTP error [{}]: status={}, cl={:?}, ce={:?}, body={}", url, status, content_length, content_encoding, preview));
    }

    if bytes.is_empty() {
        return Err(format!("WY empty body [{}]: status={}, cl={:?}, ce={:?}", url, status, content_length, content_encoding));
    }

    // Detect gzip magic bytes (server sent compressed data but client didn't decompress)
    let effective_bytes = if bytes.len() >= 2 && bytes[0] == 0x1f && bytes[1] == 0x8b {
        match flate2::read::GzDecoder::new(std::io::Cursor::new(&bytes)) {
            decoder => {
                use std::io::Read;
                let mut decoded = Vec::new();
                let mut d = decoder;
                match d.read_to_end(&mut decoded) {
                    Ok(_) if !decoded.is_empty() => decoded,
                    _ => bytes.to_vec(),
                }
            }
        }
    } else {
        bytes.to_vec()
    };

    serde_json::from_slice(&effective_bytes).map_err(|e| {
        let preview = String::from_utf8_lossy(&effective_bytes[..effective_bytes.len().min(200)]);
        format!("WY JSON parse error [{}]: {}, body={}", url, e, preview)
    })
}

pub fn get_str<'a>(args: &'a serde_json::Value, key: &str) -> &'a str {
    args.get(key).and_then(|v| v.as_str()).unwrap_or("")
}

pub fn get_u64(args: &serde_json::Value, key: &str, default: u64) -> u64 {
    args.get(key).and_then(|v| v.as_u64()).unwrap_or(default)
}

pub fn format_play_time(seconds: i64) -> String {
    let m = seconds / 60;
    let s = seconds % 60;
    format!("{:02}:{:02}", m, s)
}

pub fn format_file_size(size: i64) -> String {
    if size >= 1_073_741_824 {
        format!("{:.1}GB", size as f64 / 1_073_741_824.0)
    } else if size >= 1_048_576 {
        format!("{:.1}MB", size as f64 / 1_048_576.0)
    } else if size >= 1024 {
        format!("{:.1}KB", size as f64 / 1024.0)
    } else {
        format!("{}B", size)
    }
}

pub fn wy_headers() -> Vec<(&'static str, &'static str)> {
    vec![
        ("User-Agent", "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/60.0.3112.90 Safari/537.36"),
        ("Referer", "https://music.163.com/"),
        ("Origin", "https://music.163.com"),
    ]
}

pub fn parse_wy_id(raw_id: &str) -> String {
    if raw_id.contains("wy__") {
        return raw_id.replace("wy__", "");
    }
    if let Some(pos) = raw_id.rfind('/') {
        return raw_id[pos + 1..].to_string();
    }
    raw_id.to_string()
}

pub fn format_singer(singers: &[serde_json::Value]) -> String {
    singers.iter()
        .filter_map(|s| s.get("name").and_then(|v| v.as_str()))
        .collect::<Vec<_>>()
        .join("、")
}

/// Parse quality types from privilege data
pub fn parse_quality_types(privilege: &serde_json::Value) -> Vec<String> {
    let mut types = Vec::new();
    let maxbr = privilege.get("maxbr").and_then(|v| v.as_i64()).unwrap_or(128000);

    if privilege.get("maxBrLevel").and_then(|v| v.as_str()) == Some("hires") {
        types.push("hires".to_string());
    }
    if maxbr >= 999000 {
        types.push("flac".to_string());
    }
    if maxbr >= 320000 {
        types.push("320k".to_string());
    }
    if maxbr >= 128000 {
        types.push("128k".to_string());
    }
    types.dedup();
    types
}

/// Parse quality types from song detail data (jm, db, hr, sq, h, m, l)
pub fn parse_quality_types_from_detail(data: &serde_json::Value) -> (Vec<String>, HashMap<String, QualityInfo>) {
    let mut types = Vec::new();
    let mut types_map = HashMap::new();

    if let Some(jm) = data.get("jm").filter(|v| v.get("size").and_then(|s| s.as_i64()).unwrap_or(0) > 0) {
        let size = format_file_size(jm.get("size").and_then(|v| v.as_i64()).unwrap_or(0));
        types.push("master".to_string());
        types_map.insert("master".to_string(), QualityInfo { size, hash: None });
    }
    if let Some(db) = data.get("db").filter(|v| v.get("size").and_then(|s| s.as_i64()).unwrap_or(0) > 0) {
        let size = format_file_size(db.get("size").and_then(|v| v.as_i64()).unwrap_or(0));
        types.push("dolby".to_string());
        types_map.insert("dolby".to_string(), QualityInfo { size, hash: None });
    }
    if let Some(hr) = data.get("hr").filter(|v| v.get("size").and_then(|s| s.as_i64()).unwrap_or(0) > 0) {
        let size = format_file_size(hr.get("size").and_then(|v| v.as_i64()).unwrap_or(0));
        types.push("hires".to_string());
        types_map.insert("hires".to_string(), QualityInfo { size, hash: None });
    }
    if let Some(sq) = data.get("sq").filter(|v| v.get("size").and_then(|s| s.as_i64()).unwrap_or(0) > 0) {
        let size = format_file_size(sq.get("size").and_then(|v| v.as_i64()).unwrap_or(0));
        types.push("flac".to_string());
        types_map.insert("flac".to_string(), QualityInfo { size, hash: None });
    }
    if let Some(h) = data.get("h").filter(|v| v.get("size").and_then(|s| s.as_i64()).unwrap_or(0) > 0) {
        let size = format_file_size(h.get("size").and_then(|v| v.as_i64()).unwrap_or(0));
        types.push("320k".to_string());
        types_map.insert("320k".to_string(), QualityInfo { size, hash: None });
    }
    if data.get("m").filter(|v| v.get("size").and_then(|s| s.as_i64()).unwrap_or(0) > 0).is_some() {
        let m = data.get("m").unwrap();
        let size = format_file_size(m.get("size").and_then(|v| v.as_i64()).unwrap_or(0));
        types.push("128k".to_string());
        types_map.insert("128k".to_string(), QualityInfo { size, hash: None });
    } else if data.get("l").filter(|v| v.get("size").and_then(|s| s.as_i64()).unwrap_or(0) > 0).is_some() {
        let l = data.get("l").unwrap();
        let size = format_file_size(l.get("size").and_then(|v| v.as_i64()).unwrap_or(0));
        types.push("128k".to_string());
        types_map.insert("128k".to_string(), QualityInfo { size, hash: None });
    }

    (types, types_map)
}

/// Replace NetEase emoji names with actual emoji characters
pub fn apply_emoji(text: &str) -> String {
    let emojis: &[(&str, &str)] = &[
        ("大笑", "😃"), ("可爱", "😊"), ("憨笑", "☺️"), ("色", "😍"),
        ("亲亲", "😙"), ("惊恐", "😱"), ("流泪", "😭"), ("亲", "😚"),
        ("呆", "😳"), ("哀伤", "😔"), ("呲牙", "😁"), ("吐舌", "😝"),
        ("撇嘴", "😒"), ("怒", "😡"), ("奸笑", "😏"), ("汗", "😓"),
        ("痛苦", "😖"), ("惶恐", "😰"), ("生病", "😨"), ("口罩", "😷"),
        ("大哭", "😂"), ("晕", "😵"), ("发怒", "👿"), ("开心", "😄"),
        ("鬼脸", "😜"), ("皱眉", "😞"), ("流感", "😢"), ("爱心", "❤️"),
        ("心碎", "💔"), ("钟情", "💘"), ("星星", "⭐️"), ("生气", "💢"),
        ("便便", "💩"), ("强", "👍"), ("弱", "👎"), ("拜", "🙏"),
        ("牵手", "👫"), ("跳舞", "👯‍♀️"), ("禁止", "🙅‍♀️"), ("这边", "💁‍♀️"),
        ("爱意", "💏"), ("示爱", "👩‍❤️‍👨"), ("嘴唇", "👄"), ("狗", "🐶"),
        ("猫", "🐱"), ("猪", "🐷"), ("兔子", "🐰"), ("小鸡", "🐤"),
        ("公鸡", "🐔"), ("幽灵", "👻"), ("圣诞", "🎅"), ("外星", "👽"),
        ("钻石", "💎"), ("礼物", "🎁"), ("男孩", "👦"), ("女孩", "👧"),
        ("蛋糕", "🎂"), ("18", "🔞"), ("圈", "⭕"), ("叉", "❌"),
    ];

    let mut result = text.to_string();
    for (name, emoji) in emojis {
        result = result.replace(&format!("[{}]", name), emoji);
    }
    result
}
