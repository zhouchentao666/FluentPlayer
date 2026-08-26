use once_cell::sync::Lazy;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[allow(dead_code)]
static HTTP_CLIENT: Lazy<Client> = Lazy::new(|| {
    Client::builder()
        .user_agent("Mozilla/5.0 (Linux; Android 6.0; Nexus 5 Build/MRA58N) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/139.0.0.0 Mobile Safari/537.36")
        .connect_timeout(std::time::Duration::from_secs(10))
        .timeout(std::time::Duration::from_secs(30))
        .gzip(true)
        .brotli(true)
        .build()
        .expect("Failed to build HTTP client")
});

#[allow(dead_code)]
pub fn get_client() -> &'static Client {
    &HTTP_CLIENT
}

/// Sanitize JSON text by fixing invalid unicode escape sequences.
/// Handles lone surrogates (\uD800-\uDFFF) by replacing them with U+FFFD,
/// and properly decodes valid surrogate pairs into Unicode code points.
pub fn sanitize_json(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let len = chars.len();
    let mut result = String::with_capacity(text.len());
    let mut i = 0;
    while i < len {
        if chars[i] == '\\' && i + 5 < len && chars[i + 1] == 'u' {
            let hex: String = chars[i + 2..i + 6].iter().collect();
            if let Ok(cp) = u32::from_str_radix(&hex, 16) {
                if (0xD800..=0xDFFF).contains(&cp) {
                    if i + 11 < len && chars[i + 6] == '\\' && chars[i + 7] == 'u' {
                        let hex2: String = chars[i + 8..i + 12].iter().collect();
                        if let Ok(cp2) = u32::from_str_radix(&hex2, 16) {
                            if (0xDC00..=0xDFFF).contains(&cp2) {
                                let full = ((cp - 0xD800) << 10) + (cp2 - 0xDC00) + 0x10000;
                                if let Some(ch) = char::from_u32(full) {
                                    result.push(ch);
                                    i += 12;
                                    continue;
                                }
                            }
                        }
                    }
                    result.push('\u{FFFD}');
                    i += 6;
                    continue;
                }
            }
        }
        result.push(chars[i]);
        i += 1;
    }
    result
}

/// Extension trait for reqwest::Response that sanitizes invalid unicode before JSON parsing.
pub trait ResponseExt {
    async fn json_sanitized(self) -> Result<serde_json::Value, String>;
}

impl ResponseExt for reqwest::Response {
    async fn json_sanitized(self) -> Result<serde_json::Value, String> {
        let text = self.text().await.map_err(|e| e.to_string())?;
        serde_json::from_str(&sanitize_json(&text)).map_err(|e| e.to_string())
    }
}

// --- Shared types matching CeruMusic's MusicItem ---

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QualityInfo {
    #[serde(default)]
    pub size: String,
    #[serde(default)]
    pub hash: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MusicItem {
    #[serde(default)]
    pub songmid: serde_json::Value,
    #[serde(default)]
    pub singer: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub album_name: String,
    #[serde(default)]
    pub album_id: serde_json::Value,
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub interval: String,
    #[serde(default)]
    pub img: String,
    #[serde(default)]
    pub lrc: Option<String>,
    #[serde(default)]
    pub types: Option<Vec<String>>,
    #[serde(default, rename = "_types")]
    pub types_map: Option<HashMap<String, QualityInfo>>,
    #[serde(default)]
    pub type_url: Option<serde_json::Value>,
    #[serde(default)]
    pub hash: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub singer_id: Option<String>,
    // Provider-specific extra fields
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub song_id: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub str_media_mid: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub album_mid: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub copyright_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lrc_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mrc_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trc_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResult {
    pub list: Vec<MusicItem>,
    pub all_page: i64,
    pub limit: i64,
    pub total: i64,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaylistItem {
    pub id: serde_json::Value,
    pub name: String,
    pub img: String,
    pub source: String,
    #[serde(default)]
    pub desc: String,
    #[serde(default)]
    pub play_count: serde_json::Value,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub total: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaylistResult {
    pub list: Vec<PlaylistItem>,
    pub all_page: i64,
    pub limit: i64,
    pub total: i64,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SingerDetail {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub desc: String,
    #[serde(default)]
    pub avatar: String,
    #[serde(default)]
    pub gender: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SingerCount {
    #[serde(default)]
    pub music: i64,
    #[serde(default)]
    pub album: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SingerInfo {
    pub id: serde_json::Value,
    pub source: String,
    pub info: SingerDetail,
    pub count: SingerCount,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AlbumBrief {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub img: String,
    #[serde(default)]
    pub desc: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SingerAlbumItem {
    pub id: serde_json::Value,
    #[serde(default)]
    pub count: i64,
    pub info: AlbumBrief,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SingerAlbumListResult {
    pub list: Vec<SingerAlbumItem>,
    pub all_page: i64,
    pub limit: i64,
    pub total: i64,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaylistDetailResult {
    pub list: Vec<MusicItem>,
    pub info: serde_json::Value,
    pub all_page: i64,
    pub limit: i64,
    pub total: i64,
    pub source: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LyricResult {
    #[serde(default)]
    pub lyric: String,
    #[serde(default)]
    pub tlyric: Option<String>,
    #[serde(default)]
    pub crlyric: Option<String>,
    #[serde(default)]
    pub lxlyric: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommentItem {
    #[serde(default)]
    pub id: serde_json::Value,
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub time: serde_json::Value,
    #[serde(default)]
    pub time_str: String,
    #[serde(default)]
    pub user_name: String,
    #[serde(default)]
    pub avatar: String,
    #[serde(default)]
    pub user_id: serde_json::Value,
    #[serde(default)]
    pub liked_count: serde_json::Value,
    #[serde(default)]
    pub images: Vec<String>,
    #[serde(default)]
    pub reply: Vec<CommentItem>,
}

/// Main entry point for SDK requests.
/// Routes to the appropriate source handler based on the current source.
pub async fn handle_request(
    method: &str,
    args: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let source = args
        .get("source")
        .and_then(|v| v.as_str())
        .unwrap_or("kw")
        .to_string();
    crate::music_sdk::sources::dispatch(&source, method, args).await
}

const ALL_SOURCES: &[&str] = &["kw", "bd", "kg", "tx", "wy", "mg", "git"];
const SEARCH_EXCLUDE: &[&str] = &["xm"];

/// Search across all sources concurrently
pub async fn search_music(args: serde_json::Value) -> Result<serde_json::Value, String> {
    let name = args
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim()
        .to_string();
    let singer = args
        .get("singer")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim()
        .to_string();
    let current_source = args.get("source").and_then(|v| v.as_str()).unwrap_or("");
    let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(25);
    let query = format!("{} {}", name, singer).trim().to_string();

    let searches = ALL_SOURCES
        .iter()
        .filter(|source| **source != current_source && !SEARCH_EXCLUDE.contains(source))
        .map(|source| {
            let search_args = serde_json::json!({ "keyword": query, "page": 1, "limit": limit });
            let source = (*source).to_string();
            async move {
                crate::music_sdk::sources::dispatch(&source, "search", search_args)
                    .await
                    .ok()
            }
        });
    let responses = tokio::time::timeout(
        std::time::Duration::from_secs(35),
        futures_util::future::join_all(searches),
    )
    .await
    .map_err(|_| "跨源搜索超时".to_string())?;
    let results: Vec<_> = responses
        .into_iter()
        .flatten()
        .filter(|response| {
            response
                .get("list")
                .and_then(|value| value.as_array())
                .is_some_and(|list| !list.is_empty())
        })
        .collect();
    Ok(serde_json::json!(results))
}

/// Find matching songs across sources using fuzzy matching
pub async fn find_music(args: serde_json::Value) -> Result<serde_json::Value, String> {
    let name = args
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim()
        .to_string();
    let singer = args
        .get("singer")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim()
        .to_string();
    let album_name = args
        .get("albumName")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim()
        .to_string();
    let interval = args
        .get("interval")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim()
        .to_string();
    let current_source = args.get("source").and_then(|v| v.as_str()).unwrap_or("");

    let lists = search_music(serde_json::json!({
        "name": name, "singer": singer, "source": current_source, "limit": 25
    }))
    .await?;

    let source_lists = match lists.as_array() {
        Some(arr) => arr,
        None => return Ok(serde_json::json!([])),
    };

    let filter_str = |s: &str| -> String {
        s.chars()
            .filter(|c| {
                !c.is_whitespace()
                    && !matches!(
                        c,
                        '\'' | '.'
                            | ','
                            | '&'
                            | '"'
                            | '、'
                            | '('
                            | ')'
                            | '（'
                            | '）'
                            | '`'
                            | '~'
                            | '-'
                            | '<'
                            | '>'
                            | '|'
                            | '/'
                            | ']'
                            | '['
                            | '!'
                            | '！'
                    )
            })
            .collect::<String>()
            .to_lowercase()
    };
    let sort_single = |singer: &str| -> String {
        let parts: Vec<&str> = singer
            .split(&['、', '&', ';', '；', '/', ',', '，', '|'][..])
            .collect();
        if parts.len() > 1 {
            let mut sorted: Vec<String> = parts.iter().map(|s| s.trim().to_string()).collect();
            sorted.sort();
            sorted.join("、")
        } else {
            singer.to_string()
        }
    };
    let get_interval_secs = |interval: &str| -> i64 {
        let parts: Vec<&str> = interval.split(':').collect();
        match parts.len() {
            2 => parts[0].parse::<i64>().unwrap_or(0) * 60 + parts[1].parse::<i64>().unwrap_or(0),
            1 => parts[0].parse::<i64>().unwrap_or(0),
            _ => 0,
        }
    };

    let f_name = filter_str(&name);
    let f_singer = filter_str(&sort_single(&singer));
    let f_album = filter_str(&album_name);
    let f_interval = get_interval_secs(&interval);

    let is_eq_interval = |item_interval: i64| -> bool {
        if f_interval == 0 || item_interval == 0 {
            return true;
        }
        (f_interval - item_interval).abs() < 5
    };
    let is_includes_name =
        |item_name: &str| -> bool { f_name.contains(item_name) || item_name.contains(&f_name) };
    let is_includes_singer = |item_singer: &str| -> bool {
        if f_singer.is_empty() {
            return true;
        }
        f_singer.contains(item_singer) || item_singer.contains(&f_singer)
    };
    let is_eq_album = |item_album: &str| -> bool { f_album.is_empty() || f_album == item_album };

    let mut all_items: Vec<serde_json::Value> = Vec::new();

    for source_result in source_lists {
        let items = source_result
            .get("list")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        let mut matched: Option<serde_json::Value> = None;

        // Priority 1: exact name + singer match
        for item in &items {
            let item_name = filter_str(item.get("name").and_then(|v| v.as_str()).unwrap_or(""));
            let item_singer = filter_str(&sort_single(
                item.get("singer").and_then(|v| v.as_str()).unwrap_or(""),
            ));
            let item_interval =
                get_interval_secs(item.get("interval").and_then(|v| v.as_str()).unwrap_or(""));

            if !is_eq_interval(item_interval) {
                continue;
            }
            if item_name == f_name && is_includes_singer(&item_singer) {
                matched = Some(item.clone());
                break;
            }
        }

        // Priority 2: singer match + name includes
        if matched.is_none() {
            for item in &items {
                let item_name = filter_str(item.get("name").and_then(|v| v.as_str()).unwrap_or(""));
                let item_singer = filter_str(&sort_single(
                    item.get("singer").and_then(|v| v.as_str()).unwrap_or(""),
                ));
                let item_interval =
                    get_interval_secs(item.get("interval").and_then(|v| v.as_str()).unwrap_or(""));

                if !is_eq_interval(item_interval) {
                    continue;
                }
                if item_singer == f_singer && is_includes_name(&item_name) {
                    matched = Some(item.clone());
                    break;
                }
            }
        }

        // Priority 3: album + singer + name
        if matched.is_none() {
            for item in &items {
                let item_name = filter_str(item.get("name").and_then(|v| v.as_str()).unwrap_or(""));
                let item_singer = filter_str(&sort_single(
                    item.get("singer").and_then(|v| v.as_str()).unwrap_or(""),
                ));
                let item_album =
                    filter_str(item.get("albumName").and_then(|v| v.as_str()).unwrap_or(""));

                if is_eq_album(&item_album)
                    && is_includes_singer(&item_singer)
                    && is_includes_name(&item_name)
                {
                    matched = Some(item.clone());
                    break;
                }
            }
        }

        if let Some(m) = matched {
            all_items.push(m);
        }
    }

    // Sort results by match quality
    let mut sorted = Vec::new();
    let mut remaining: Vec<usize> = (0..all_items.len()).collect();

    // Sort by: singer+name+interval match
    let extract = |idx: usize| -> (String, String, String, i64) {
        let item = &all_items[idx];
        (
            filter_str(item.get("name").and_then(|v| v.as_str()).unwrap_or("")),
            filter_str(&sort_single(
                item.get("singer").and_then(|v| v.as_str()).unwrap_or(""),
            )),
            filter_str(item.get("albumName").and_then(|v| v.as_str()).unwrap_or("")),
            get_interval_secs(item.get("interval").and_then(|v| v.as_str()).unwrap_or("")),
        )
    };

    // Priority sort passes
    #[allow(clippy::type_complexity)]
    let passes: Vec<Box<dyn Fn(&(String, String, String, i64)) -> bool>> = vec![
        Box::new(|(n, s, _, i)| {
            *n == f_name && *s == f_singer && (*i == f_interval || f_interval == 0)
        }),
        Box::new(|(n, s, a, _)| *n == f_name && *s == f_singer && *a == f_album),
        Box::new(|(n, s, _, _)| *n == f_name && *s == f_singer),
        Box::new(|(n, _, _, i)| *n == f_name && (*i == f_interval || f_interval == 0)),
        Box::new(|(_, s, _, i)| *s == f_singer && (*i == f_interval || f_interval == 0)),
        Box::new(|(_, _, _, i)| *i == f_interval && f_interval > 0),
        Box::new(|(n, _, _, _)| *n == f_name),
        Box::new(|(_, s, _, _)| *s == f_singer),
        Box::new(|(_, _, a, _)| *a == f_album && !f_album.is_empty()),
    ];

    for pass_fn in &passes {
        let mut matched_indices = Vec::new();
        remaining.retain(|&idx| {
            let info = extract(idx);
            if pass_fn(&info) {
                matched_indices.push(idx);
                false
            } else {
                true
            }
        });
        sorted.extend(matched_indices);
    }
    sorted.extend(remaining);

    let result: Vec<&serde_json::Value> = sorted.iter().map(|&idx| &all_items[idx]).collect();
    Ok(serde_json::json!(result))
}
