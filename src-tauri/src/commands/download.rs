use std::collections::HashMap;
use std::path::Path;
use std::time::Instant;

use tauri::{AppHandle, Emitter};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_http::reqwest;
use tauri_plugin_http::reqwest::header::{HeaderMap, HeaderName, HeaderValue};

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
/// - 通过 `download-progress` 事件上报进度。
#[tauri::command]
pub async fn download_file(
    app: AppHandle,
    url: String,
    dest: String,
    headers: Option<HashMap<String, String>>,
) -> Result<DownloadResult, String> {
    let host = host_of(&url);
    let provided: Vec<(String, String)> = match headers {
        Some(h) => h.into_iter().collect(),
        None => Vec::new(),
    };
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

// ===================== 带元数据内嵌的下载 =====================

/// 下载一首歌时的元数据，用于在本地文件内嵌标签（ID3 / MP4 / VorbisComments）。
#[derive(serde::Deserialize, Default)]
pub struct DownloadSongMeta {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub album_artist: Option<String>,
    pub track_number: Option<u32>,
    pub genre: Option<String>,
    pub year: Option<i32>,
    pub cover_url: Option<String>,
    pub lyric: Option<String>,
}

/// 按主机名自动补全防盗链请求头，下载字节；策略失败时返回错误字符串。
async fn download_audio_bytes(
    app: &AppHandle,
    url: &str,
    headers: &Option<HashMap<String, String>>,
) -> Result<(Vec<u8>, u64), String> {
    let host = host_of(url);
    let provided: Vec<(String, String)> = match headers.clone() {
        Some(h) => h.into_iter().collect(),
        None => Vec::new(),
    };
    let mut strategies = fetch_strategies(&host);
    if !provided.is_empty() {
        strategies.insert(0, provided);
    }
    let client = reqwest::Client::builder()
        .build()
        .map_err(|e| format!("创建下载客户端失败: {e}"))?;
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
        let resp = match client.get(url).headers(hdrs).send().await {
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
        let size = bytes.len() as u64;
        emit_progress(DownloadProgress {
            url: url.to_string(),
            dest: String::new(),
            downloaded: size,
            total: None,
            done: true,
            ok: true,
            error: None,
        });
        return Ok((bytes.to_vec(), size));
    }
    Err(last_err)
}

/// 把元数据、歌词、封面写入已下载的音频文件（直接原地修改 dest）。
fn embed_tags(
    dest: &str,
    meta: &DownloadSongMeta,
    embed_metadata: bool,
    embed_lyric: bool,
    embed_cover: bool,
    cover_bytes: Option<Vec<u8>>,
) -> Result<(), String> {
    use lofty::file::{AudioFile, TaggedFileExt};
    use lofty::prelude::*;
    use lofty::{ItemKey, Picture, PictureType, Tag, TagType};

    let mut tagged = lofty::Probe::open(dest)
        .map_err(|e| format!("打开音频失败: {e}"))?
        .guess_file_type()
        .read()
        .map_err(|e| format!("解析音频失败: {e}"))?;

    let ftype = tagged.file_type();
    let tag_type = match ftype {
        lofty::FileType::Mp3 | lofty::FileType::Aiff | lofty::FileType::Wav => TagType::Id3v2,
        lofty::FileType::Mp4 => TagType::Mp4,
        lofty::FileType::Flac => TagType::VorbisComments,
        lofty::FileType::Opus | lofty::FileType::Vorbis => TagType::VorbisComments,
        _ => TagType::Id3v2,
    };

    let mut tag = if let Some(existing) = tagged.tag_mut().take() {
        existing
    } else {
        Tag::new(tag_type)
    };

    if embed_metadata {
        tag.set_title(meta.title.clone());
        tag.set_artist(meta.artist.clone());
        tag.set_album(meta.album.clone());
        if let Some(aa) = &meta.album_artist {
            tag.insert_text(ItemKey::AlbumArtist, aa.clone());
        }
        if let Some(n) = meta.track_number {
            tag.insert_text(ItemKey::TrackNumber, n.to_string());
        }
        if let Some(g) = &meta.genre {
            tag.set_genre(g.clone());
        }
        if let Some(y) = meta.year {
            tag.insert_text(ItemKey::Year, y.to_string());
        }
    }
    if embed_cover {
        if let Some(pic) = cover_bytes {
            let mime = mime_from_bytes(&pic).to_string();
            tag.push_picture(Picture {
                mime_type: mime,
                picture_type: PictureType::CoverFront,
                description: None,
                data: pic.into(),
            });
        }
    }
    if embed_lyric {
        if let Some(lyric) = &meta.lyric {
            tag.insert_text(ItemKey::Lyrics, lyric.clone());
        }
    }

    tagged.set_tag(tag);
    tagged.save().map_err(|e| format!("写入标签失败: {e}"))?;
    Ok(())
}

fn mime_from_bytes(b: &[u8]) -> &'static str {
    if b.starts_with(b"\xff\xd8\xff") {
        "image/jpeg"
    } else if b.starts_with(b"\x89PNG") {
        "image/png"
    } else if b.starts_with(b"GIF8") {
        "image/gif"
    } else if b.len() > 12 && b.starts_with(b"RIFF") && &b[8..12] == b"WEBP" {
        "image/webp"
    } else {
        "image/jpeg"
    }
}

/// 带元数据内嵌的下载：先下载裸音频，再（按选项）内嵌标签，
/// 支持标题/艺术家/专辑/封面/歌词，封面需二次下载。
#[tauri::command]
pub async fn download_song(
    app: AppHandle,
    url: String,
    dest: String,
    headers: Option<HashMap<String, String>>,
    meta: Option<DownloadSongMeta>,
    embed_metadata: bool,
    embed_lyric: bool,
    embed_cover: bool,
) -> Result<DownloadResult, String> {
    let (bytes, size) = download_audio_bytes(&app, &url, &headers).await?;

    if let Some(parent) = Path::new(&dest).parent() {
        if !parent.as_os_str().is_empty() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                return Err(format!("创建目录失败: {e}"));
            }
        }
    }

    let start = Instant::now();
    std::fs::write(&dest, &bytes).map_err(|e| format!("写入文件失败: {e}"))?;

    let cover_bytes = if embed_cover {
        if let Some(m) = &meta {
            if let Some(cov) = &m.cover_url {
                download_audio_bytes(&app, cov, &None).await.ok().map(|(b, _)| b)
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    };

    if (embed_metadata || embed_lyric || embed_cover) && meta.is_some() {
        if let Err(e) = embed_tags(
            &dest,
            meta.as_ref().unwrap(),
            embed_metadata,
            embed_lyric,
            embed_cover,
            cover_bytes,
        ) {
            return Err(e);
        }
    }

    Ok(DownloadResult {
        size,
        path: dest,
        duration_ms: start.elapsed().as_millis() as u64,
    })
}
