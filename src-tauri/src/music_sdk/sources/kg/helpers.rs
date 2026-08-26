use crate::music_sdk::client;

pub fn get_http() -> &'static reqwest::Client {
    client::get_client()
}

pub fn get_str<'a>(args: &'a serde_json::Value, key: &str) -> &'a str {
    args.get(key).and_then(|v| v.as_str()).unwrap_or("")
}

pub fn get_u64(args: &serde_json::Value, key: &str, default: u64) -> u64 {
    args.get(key).and_then(|v| v.as_u64()).unwrap_or(default)
}

pub fn decode_html(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
}

pub fn format_play_time(seconds: i64) -> String {
    let m = seconds / 60;
    let s = seconds % 60;
    format!("{:02}:{:02}", m, s)
}

pub fn extract_regex(html: &str, pattern: &str) -> Option<String> {
    let re = regex_lite::Regex::new(pattern).ok()?;
    let caps = re.captures(html)?;
    Some(caps.get(1)?.as_str().to_string())
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

/// Parse Kugou count values which may be numbers or pre-formatted strings like "1681.7万", "2.3亿"
pub fn parse_kugou_count(v: &serde_json::Value) -> serde_json::Value {
    if let Some(n) = v.as_f64() {
        return serde_json::json!(n as i64);
    }
    if let Some(s) = v.as_str() {
        let s = s.trim();
        if s.ends_with('亿') {
            if let Ok(n) = s.trim_end_matches('亿').parse::<f64>() {
                return serde_json::json!((n * 100_000_000.0) as i64);
            }
        } else if s.ends_with('万') {
            if let Ok(n) = s.trim_end_matches('万').parse::<f64>() {
                return serde_json::json!((n * 10_000.0) as i64);
            }
        }
        if let Ok(n) = s.parse::<f64>() {
            return serde_json::json!(n as i64);
        }
    }
    serde_json::Value::Null
}
