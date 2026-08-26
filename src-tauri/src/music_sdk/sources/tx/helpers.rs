use crate::music_sdk::client;
use crate::music_sdk::client::QualityInfo;
use std::collections::HashMap;

pub fn get_http() -> &'static reqwest::Client {
    client::get_client()
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

pub fn format_singer(singers: &[serde_json::Value]) -> String {
    singers.iter()
        .filter_map(|s| s.get("name").and_then(|v| v.as_str()))
        .collect::<Vec<_>>()
        .join("、")
}

pub fn build_album_img(album_mid: &str) -> String {
    if album_mid.is_empty() || album_mid == "空" {
        String::new()
    } else {
        format!("https://y.gtimg.cn/music/photo_new/T002R500x500M000{}.jpg", album_mid)
    }
}

pub fn build_singer_img(singer_mid: &str) -> String {
    format!("https://y.gtimg.cn/music/photo_new/T001R500x500M000{}.jpg", singer_mid)
}

pub fn get_song_img(item: &serde_json::Value, album_mid: &str) -> String {
    let album_name = item.get("album").and_then(|a| a.get("name")).and_then(|v| v.as_str()).unwrap_or("");
    if album_name.is_empty() || album_name == "空" {
        let singer_list = item.get("singer").and_then(|v| v.as_array());
        if let Some(singers) = singer_list {
            if let Some(mid) = singers.first().and_then(|s| s.get("mid")).and_then(|v| v.as_str()) {
                return build_singer_img(mid);
            }
        }
        String::new()
    } else {
        build_album_img(album_mid)
    }
}

/// Parse quality types from file info (for leaderboard detail and playlist detail)
pub fn parse_quality_types(file: &serde_json::Value) -> (Vec<String>, HashMap<String, QualityInfo>) {
    let mut types = Vec::new();
    let mut types_map = HashMap::new();

    if file.get("size_128mp3").and_then(|v| v.as_i64()).unwrap_or(0) > 0 {
        let size = format_file_size(file.get("size_128mp3").and_then(|v| v.as_i64()).unwrap_or(0));
        types.push("128k".to_string());
        types_map.insert("128k".to_string(), QualityInfo { size, hash: None });
    }
    if file.get("size_320mp3").and_then(|v| v.as_i64()).unwrap_or(0) > 0 {
        let size = format_file_size(file.get("size_320mp3").and_then(|v| v.as_i64()).unwrap_or(0));
        types.push("320k".to_string());
        types_map.insert("320k".to_string(), QualityInfo { size, hash: None });
    }
    if file.get("size_flac").and_then(|v| v.as_i64()).unwrap_or(0) > 0 {
        let size = format_file_size(file.get("size_flac").and_then(|v| v.as_i64()).unwrap_or(0));
        types.push("flac".to_string());
        types_map.insert("flac".to_string(), QualityInfo { size, hash: None });
    }
    if file.get("size_hires").and_then(|v| v.as_i64()).unwrap_or(0) > 0 {
        let size = format_file_size(file.get("size_hires").and_then(|v| v.as_i64()).unwrap_or(0));
        types.push("hires".to_string());
        types_map.insert("hires".to_string(), QualityInfo { size, hash: None });
    }
    // size_new: [master, atmos, atmos_plus]
    if let Some(size_new) = file.get("size_new").and_then(|v| v.as_array()) {
        if size_new.get(1).and_then(|v| v.as_i64()).unwrap_or(0) > 0 {
            let size = format_file_size(size_new.get(1).and_then(|v| v.as_i64()).unwrap_or(0));
            types.push("atmos".to_string());
            types_map.insert("atmos".to_string(), QualityInfo { size, hash: None });
        }
        if size_new.get(2).and_then(|v| v.as_i64()).unwrap_or(0) > 0 {
            let size = format_file_size(size_new.get(2).and_then(|v| v.as_i64()).unwrap_or(0));
            types.push("atmos_plus".to_string());
            types_map.insert("atmos_plus".to_string(), QualityInfo { size, hash: None });
        }
        if size_new.first().and_then(|v| v.as_i64()).unwrap_or(0) > 0 {
            let size = format_file_size(size_new.first().and_then(|v| v.as_i64()).unwrap_or(0));
            types.push("master".to_string());
            types_map.insert("master".to_string(), QualityInfo { size, hash: None });
        }
    }

    types.sort_by(|a, b| quality_rank(b).cmp(&quality_rank(a)));

    (types, types_map)
}

fn quality_rank(q: &str) -> u8 {
    match q {
        "master" => 7,
        "atmos_plus" => 6,
        "atmos" => 5,
        "hires" => 4,
        "flac" => 3,
        "320k" => 2,
        "128k" => 1,
        _ => 0,
    }
}

/// Replace QQ Music emoji codes with actual emoji characters
pub fn replace_emoji(msg: &str) -> String {
    let emojis: &[(&str, &str)] = &[
        ("e400846", "😘"), ("e400874", "😴"), ("e400825", "😃"), ("e400847", "😙"),
        ("e400835", "😍"), ("e400873", "😳"), ("e400836", "😎"), ("e400867", "😭"),
        ("e400832", "😊"), ("e400837", "😏"), ("e400875", "😫"), ("e400831", "😉"),
        ("e400855", "😡"), ("e400823", "😄"), ("e400862", "😨"), ("e400844", "😖"),
        ("e400841", "😓"), ("e400830", "😈"), ("e400828", "😆"), ("e400833", "😋"),
        ("e400822", "😀"), ("e400843", "😕"), ("e400829", "😇"), ("e400824", "😂"),
        ("e400834", "😌"), ("e400877", "😷"), ("e400132", "🍉"), ("e400181", "🍺"),
        ("e401067", "☕️"), ("e400186", "🥧"), ("e400343", "🐷"), ("e400116", "🌹"),
        ("e400126", "🍃"), ("e400613", "💋"), ("e401236", "❤️"), ("e400622", "💔"),
        ("e400637", "💣"), ("e400643", "💩"), ("e400773", "🔪"), ("e400102", "🌛"),
        ("e401328", "🌞"), ("e400420", "👏"), ("e400914", "🙌"), ("e400408", "👍"),
        ("e400414", "👎"), ("e401121", "✋"), ("e400396", "👋"), ("e400384", "👉"),
        ("e401115", "✊"), ("e400402", "👌"), ("e400905", "🙈"), ("e400906", "🙉"),
        ("e400907", "🙊"), ("e400562", "👻"), ("e400932", "🙏"), ("e400644", "💪"),
        ("e400611", "💉"), ("e400185", "🎁"), ("e400655", "💰"), ("e400325", "🐥"),
        ("e400612", "💊"), ("e400198", "🎉"), ("e401685", "⚡️"), ("e400631", "💝"),
        ("e400768", "🔥"), ("e400432", "👑"),
    ];

    let re = regex_lite::Regex::new(r"\[em\](e\d+)\[/em\]").unwrap();
    let mut result = msg.to_string();
    for caps in re.captures_iter(msg) {
        let full = &caps[0];
        let code = &caps[1];
        let emoji = emojis.iter().find(|(k, _)| *k == code).map(|(_, v)| *v).unwrap_or("");
        result = result.replace(full, emoji);
    }
    result
}

/// Format timestamp to date string
pub fn format_time_str(ts: i64) -> String {
    if ts == 0 { return String::new(); }
    let dt = chrono::DateTime::from_timestamp(ts, 0);
    match dt {
        Some(d) => d.format("%Y-%m-%d %H:%M").to_string(),
        None => String::new(),
    }
}

pub fn random_int(min: u64, max: u64) -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().subsec_nanos();
    min + (nanos as u64 % (max - min + 1))
}

pub fn get_search_id() -> String {
    let e = random_int(1, 20);
    let t = e as f64 * 18014398509481984.0_f64;
    let n = random_int(0, 4194304) as f64 * 4294967296.0_f64;
    let a = chrono::Utc::now().timestamp_millis() as f64;
    let r = (a * 1000.0) as i64 % (24 * 60 * 60 * 1000);
    let result = (t as i64) + (n as i64) + r;
    result.to_string()
}
