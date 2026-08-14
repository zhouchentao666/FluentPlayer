use std::collections::HashMap;
use std::path::Path;
use std::time::Instant;

use tauri::{AppHandle, Emitter};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_http::reqwest;
use tauri_plugin_http::reqwest::header::{HeaderMap, HeaderName, HeaderValue, USER_AGENT};
use lofty::config::WriteOptions;
use lofty::file::{AudioFile, TaggedFileExt};
use lofty::picture::{MimeType, Picture, PictureType};
use lofty::probe::Probe;
use lofty::tag::{ItemKey, Tag};

const CHROME_UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/122.0.0.0 Safari/537.36";

#[derive(serde::Serialize, Clone)]
struct DownloadProgress {
    url: String,
    dest: String,
    downloaded: u64,
    total: Option<u64>,
    done: bool,
    ok: bool,
    error: Option<String>,
}

#[derive(serde::Serialize)]
pub struct DownloadResult {
    pub size: u64,
    pub path: String,
    pub duration_ms: u64,
}

/// 从 URL 中提取主机名（小写），用于匹配各平台的防盗链规则。
fn host_of(url: &str) -> String {
    let without_scheme = match url.find("://") {
        Some(i) => &url[i + 3..],
        None => url,
    };
    let end = without_scheme.find('/').unwrap_or(without_scheme.len());
    let authority = &without_scheme[..end];
    let host = authority.split(':').next().unwrap_or(authority);
    host.to_lowercase()
}

/// 根据主机名返回常用的防盗链请求头。
fn cdn_headers(host: &str) -> Vec<(String, String)> {
    if host.contains("126.net") || host.contains("163.com") || host.contains("netease") || host.contains("lazyaudio") {
        vec![
            ("Referer".into(), "https://music.163.com/".into()),
            ("User-Agent".into(), CHROME_UA.into()),
        ]
    } else if host.contains("qq.com") || host.contains("gtimg.cn") || host.contains("tencentmusic") {
        vec![
            ("Referer".into(), "https://y.qq.com/".into()),
            ("User-Agent".into(), CHROME_UA.into()),
        ]
    } else if host.contains("kuwo") || host.contains("koowo") {
        vec![
            ("Referer".into(), "https://www.kuwo.cn/".into()),
            ("User-Agent".into(), CHROME_UA.into()),
        ]
    } else if host.contains("kugou") || host.contains("kgimg") {
        vec![
            ("Referer".into(), "https://www.kugou.com/".into()),
            ("User-Agent".into(), CHROME_UA.into()),
        ]
    } else if host.contains("migu") || host.contains("nf.migu") {
        vec![
            ("Referer".into(), "https://music.migu.cn/".into()),
            ("User-Agent".into(), CHROME_UA.into()),
        ]
    } else {
        vec![("User-Agent".into(), CHROME_UA.into())]
    }
}

/// 不同平台尝试多种请求头策略，规避防盗链拦截。
fn fetch_strategies(host: &str) -> Vec<Vec<(String, String)>> {
    if host.contains("126.net") || host.contains("163.com") || host.contains("netease") || host.contains("lazyaudio") {
        vec![
            vec![
                ("Referer".into(), "https://music.163.com/".into()),
                ("User-Agent".into(), CHROME_UA.into()),
            ],
            vec![("User-Agent".into(), CHROME_UA.into())],
            vec![
                ("Referer".into(), "https://music.163.com/".into()),
                ("User-Agent".into(), CHROME_UA.into()),
                ("X-Real-IP".into(), "211.161.244.70".into()),
                ("X-Forwarded-For".into(), "211.161.244.70".into()),
            ],
        ]
    } else {
        vec![cdn_headers(host)]
    }
}

/// 校验下载到的内容是否为音频（避免把防盗链返回的错误页面当文件存盘）。
fn looks_like_audio(b: &[u8]) -> bool {
    if b.len() < 4 {
        return false;
    }
    // ID3 (MP3 标签)
    if b[0] == 0x49 && b[1] == 0x44 && b[2] == 0x33 {
        return true;
    }
    // MPEG 帧同步 / ADTS (AAC)
    if b[0] == 0xff && (b[1] & 0xe0) == 0xe0 {
        return true;
    }
    // fLaC
    if b.starts_with(b"fLaC") {
        return true;
    }
    // Ogg
    if b.starts_with(b"OggS") {
        return true;
    }
    // RIFF (WAV)
    if b.starts_with(b"RIFF") {
        return true;
    }
    // MP4 / M4A (ftyp 位于偏移 4)
    if b.len() >= 8 && b[4] == 0x66 && b[5] == 0x74 && b[6] == 0x79 && b[7] == 0x70 {
        return true;
    }
    false
}

/// 弹出“保存文件”对话框，返回用户选择的完整路径（取消则返回 null）。
/// 移动端不支持 blocking 对话框，仅桌面端可用。
#[cfg(desktop)]
#[tauri::command]
pub async fn save_file(app: AppHandle, default_name: String) -> Result<Option<String>, String> {
    let picked = app
        .dialog()
        .file()
        .set_file_name(default_name)
        .blocking_save_file();
    Ok(picked.map(|p| p.to_string()))
}

/// 把在线音频直链下载为本地文件（真正的离线文件下载）。
///
/// - 按主机名自动补全防盗链请求头，并支持策略回退；
/// - 下载完成后校验内容确为音频，否则删除并尝试下一种策略；
/// - `embed_lyrics` / `embed_cover` 为真时，把 `lyric` 文本与
///   `cover_url` 图片写入音频文件标签（失败不影响已完成的下载）；
/// - 通过 `download-progress` 事件上报进度。
#[tauri::command]
pub async fn download_file(
    app: AppHandle,
    url: String,
    dest: String,
    headers: Option<HashMap<String, String>>,
    embed_lyrics: Option<bool>,
    embed_cover: Option<bool>,
    lyric: Option<String>,
    cover_url: Option<String>,
    title: Option<String>,
    artist: Option<String>,
    album: Option<String>,
) -> Result<DownloadResult, String> {
    let embed_lyrics = embed_lyrics.unwrap_or(false);
    let embed_cover = embed_cover.unwrap_or(false);
    let host = host_of(&url);
    let provided: Vec<(String, String)> = headers.unwrap_or_default().into_iter().collect();
    let mut strategies = fetch_strategies(&host);
    // 前端显式传入的 headers 优先级最高，放在策略列表最前面
    if !provided.is_empty() {
        strategies.insert(0, provided);
    }

    let client = reqwest::Client::builder()
        .build()
        .map_err(|e| format!("创建下载客户端失败: {e}"))?;

    let start = Instant::now();
    let emit_progress = |p: DownloadProgress| {
        let _ = app.emit("download-progress", p);
    };
    let mut last_err = "下载失败：所有请求策略均无法获取音频".to_string();

    for strategy in &strategies {
        let mut hdrs = HeaderMap::new();
        for (k, v) in strategy {
            if let (Ok(n), Ok(val)) = (HeaderName::from_bytes(k.as_bytes()), HeaderValue::from_str(v)) {
                hdrs.insert(n, val);
            }
        }
        emit_progress(DownloadProgress {
            url: url.clone(),
            dest: dest.clone(),
            downloaded: 0,
            total: None,
            done: false,
            ok: false,
            error: None,
        });

        let resp = match client.get(&url).headers(hdrs).send().await {
            Ok(r) => r,
            Err(e) => {
                last_err = format!("请求失败: {e}");
                continue;
            }
        };
        if !resp.status().is_success() {
            last_err = format!("HTTP 状态 {}", resp.status());
            continue;
        }
        let total = resp.content_length();
        let bytes = match resp.bytes().await {
            Ok(b) => b,
            Err(e) => {
                last_err = format!("读取响应失败: {e}");
                continue;
            }
        };
        if !looks_like_audio(&bytes) {
            last_err = "下载内容不是音频文件（可能被防盗链拦截，返回了错误页面）".to_string();
            continue;
        }
        if let Some(parent) = Path::new(&dest).parent() {
            if !parent.as_os_str().is_empty() {
                if let Err(e) = std::fs::create_dir_all(parent) {
                    return Err(format!("创建目录失败: {e}"));
                }
            }
        }
        match std::fs::write(&dest, &bytes) {
            Ok(_) => {
                let size = bytes.len() as u64;
                // 下载成功后再尝试内嵌歌词 / 封面（失败不影响已完成的下载）。
                if embed_lyrics || embed_cover || title.is_some() || artist.is_some() || album.is_some() {
                    if let Err(e) = embed_tags(
                        &app,
                        &client,
                        &dest,
                        embed_lyrics,
                        embed_cover,
                        lyric.as_deref(),
                        cover_url.as_deref(),
                        title.as_deref(),
                        artist.as_deref(),
                        album.as_deref(),
                    )
                    .await
                    {
                        eprintln!("[download] 内嵌标签失败（不影响已下载文件）: {e}");
                    }
                }
                emit_progress(DownloadProgress {
                    url: url.clone(),
                    dest: dest.clone(),
                    downloaded: size,
                    total,
                    done: true,
                    ok: true,
                    error: None,
                });
                return Ok(DownloadResult {
                    size,
                    path: dest,
                    duration_ms: start.elapsed().as_millis() as u64,
                });
            }
            Err(e) => return Err(format!("写入文件失败: {e}")),
        }
    }

    emit_progress(DownloadProgress {
        url: url.clone(),
        dest: dest.clone(),
        downloaded: 0,
        total: None,
        done: true,
        ok: false,
        error: Some(last_err.clone()),
    });
    Err(last_err)
}

/// 把歌词文本与封面图片内嵌进已下载的音频文件标签。
/// 出错时仅返回 Err（调用方已决定忽略），不影响已完成的下载。
async fn embed_tags(
    _app: &AppHandle,
    client: &reqwest::Client,
    dest: &str,
    embed_lyrics: bool,
    embed_cover: bool,
    lyric: Option<&str>,
    cover_url: Option<&str>,
    title: Option<&str>,
    artist: Option<&str>,
    album: Option<&str>,
) -> Result<(), String> {
    let mut tagged = Probe::open(dest)
        .map_err(|e| e.to_string())?
        .read()
        .map_err(|e| e.to_string())?;
    let mut tag = match tagged.primary_tag() {
        Some(t) => t.clone(),
        None => Tag::new(tagged.primary_tag_type()),
    };

    if let Some(t) = title {
        let t = t.trim();
        if !t.is_empty() {
            tag.insert_text(ItemKey::TrackTitle, t.to_string());
        }
    }
    if let Some(a) = artist {
        let a = a.trim();
        if !a.is_empty() {
            tag.insert_text(ItemKey::TrackArtist, a.to_string());
        }
    }
    if let Some(al) = album {
        let al = al.trim();
        if !al.is_empty() {
            tag.insert_text(ItemKey::AlbumTitle, al.to_string());
        }
    }

    if embed_lyrics {
        if let Some(l) = lyric {
            let text = l.trim();
            if !text.is_empty() {
                tag.insert_text(ItemKey::Lyrics, text.to_string());
            }
        }
    }

    if embed_cover {
        if let Some(cu) = cover_url {
            if let Ok(cu_url) = reqwest::Url::parse(cu) {
                let referer = match cu_url.host_str() {
                    Some(h) if h.contains("music.126.net") || h.contains("126.net") => "https://music.163.com/",
                    Some(h) if h.contains("gtimg.cn") || h.contains("qq.com") => "https://y.qq.com/",
                    Some(h) if h.contains("kugou.com") => "https://www.kugou.com/",
                    Some(h) if h.contains("kuwo.cn") => "https://www.kuwo.cn/",
                    Some(h) if h.contains("music.migu.cn") => "https://music.migu.cn/",
                    _ => "",
                };
                let mut req = client.get(cu).header(USER_AGENT, CHROME_UA);
                if !referer.is_empty() {
                    req = req.header("Referer", referer);
                }
                if let Ok(resp) = req.send().await {
                    if let Ok(bytes) = resp.bytes().await {
                        let pic = Picture::new_unchecked(
                            PictureType::CoverFront,
                            Some(infer_mime(&bytes)),
                            None,
                            bytes.to_vec(),
                        );
                        tag.push_picture(pic);
                    }
                }
            }
        }
    }

    tagged.insert_tag(tag);
    tagged
        .save_to_path(dest, WriteOptions::default())
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// 根据图片二进制头部推断 MIME 类型（用于封面标签）。
fn infer_mime(b: &[u8]) -> MimeType {
    if b.len() >= 3 && b[0] == 0xFF && b[1] == 0xD8 && b[2] == 0xFF {
        MimeType::Jpeg
    } else if b.len() >= 8 && b.starts_with(b"\x89PNG\r\n\x1a\n") {
        MimeType::Png
    } else if b.len() >= 4 && &b[0..4] == b"GIF8" {
        MimeType::Gif
    } else if b.len() >= 12 && &b[0..4] == b"RIFF" && &b[8..12] == b"WEBP" {
        MimeType::Png
    } else {
        MimeType::Png
    }
}
