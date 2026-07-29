use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use lofty::picture::MimeType;
use lofty::prelude::*;
use lofty::probe::Probe;
use serde::Serialize;
use std::fs;
use std::path::Path;
use walkdir::WalkDir;

pub const AUDIO_EXTS: &[&str] = &[
    "mp3", "flac", "wav", "ogg", "oga", "m4a", "m4b", "aac", "opus", "wma", "aif", "aiff", "ape",
    "wv",
];

pub const IMAGE_EXTS: &[&str] = &["jpg", "jpeg", "png", "webp", "gif", "bmp"];

fn is_audio(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| AUDIO_EXTS.contains(&e.to_ascii_lowercase().as_str()))
        .unwrap_or(false)
}

/// 递归扫描文件夹内的音频文件
#[tauri::command]
pub fn scan_music_folder(folder: String) -> Result<Vec<String>, String> {
    let mut out: Vec<String> = Vec::new();
    for entry in WalkDir::new(&folder)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if entry.file_type().is_file() && is_audio(entry.path()) {
            out.push(entry.path().to_string_lossy().to_string());
        }
    }
    out.sort_by(|a, b| a.to_lowercase().cmp(&b.to_lowercase()));
    Ok(out)
}

#[derive(Serialize)]
pub struct SongMetadata {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub genre: String,
    pub year: String,
    pub duration: f64,
    pub bitrate: u32,
    pub sample_rate: u32,
}

/// 读取音频标签元数据（lofty）
#[tauri::command]
pub fn read_metadata(path: String) -> Result<SongMetadata, String> {
    let tagged = Probe::open(&path)
        .map_err(|e| format!("无法打开文件: {e}"))?
        .read()
        .map_err(|e| format!("无法读取标签: {e}"))?;

    let props = tagged.properties();
    let tag = tagged.primary_tag().or_else(|| tagged.first_tag());

    let (title, artist, album, genre, year) = match tag {
        Some(t) => (
            t.title().map(|s| s.to_string()).unwrap_or_default(),
            t.artist().map(|s| s.to_string()).unwrap_or_default(),
            t.album().map(|s| s.to_string()).unwrap_or_default(),
            t.genre().map(|s| s.to_string()).unwrap_or_default(),
            t.year().map(|y| y.to_string()).unwrap_or_default(),
        ),
        None => Default::default(),
    };

    Ok(SongMetadata {
        title,
        artist,
        album,
        genre,
        year,
        duration: props.duration().as_secs_f64(),
        bitrate: props.audio_bitrate().unwrap_or(0),
        sample_rate: props.sample_rate().unwrap_or(0),
    })
}

fn mime_to_str(mime: Option<&MimeType>) -> &str {
    match mime {
        Some(MimeType::Png) => "image/png",
        Some(MimeType::Jpeg) => "image/jpeg",
        Some(MimeType::Tiff) => "image/tiff",
        Some(MimeType::Bmp) => "image/bmp",
        Some(MimeType::Gif) => "image/gif",
        _ => "image/jpeg",
    }
}

fn ext_mime(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .unwrap_or_default()
        .as_str()
    {
        "png" => "image/png",
        "webp" => "image/webp",
        "gif" => "image/gif",
        "bmp" => "image/bmp",
        _ => "image/jpeg",
    }
}

fn to_data_url(mime: &str, data: &[u8]) -> String {
    format!("data:{};base64,{}", mime, B64.encode(data))
}

/// 读取封面：优先内嵌图片，其次同目录 cover/folder 等图片文件
#[tauri::command]
pub fn read_cover_art(path: String) -> Result<String, String> {
    // 1. 内嵌封面
    if let Ok(tagged) = Probe::open(&path).and_then(|p| p.read()) {
        for tag in tagged.tags() {
            if let Some(pic) = tag.pictures().first() {
                return Ok(to_data_url(mime_to_str(pic.mime_type()), pic.data()));
            }
        }
    }

    // 2. 同目录封面文件（同名图片 / cover.* / folder.* / front.*）
    let p = Path::new(&path);
    if let Some(dir) = p.parent() {
        let stem = p
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase();
        let mut candidates: Vec<std::path::PathBuf> = Vec::new();
        for name in [stem.as_str(), "cover", "folder", "front", "albumart"] {
            if name.is_empty() {
                continue;
            }
            for ext in IMAGE_EXTS {
                candidates.push(dir.join(format!("{name}.{ext}")));
            }
        }
        for cand in candidates {
            if cand.is_file() {
                if let Ok(data) = fs::read(&cand) {
                    return Ok(to_data_url(ext_mime(&cand), &data));
                }
            }
        }
    }

    Err("没有找到封面".into())
}

fn decode_text(data: &[u8]) -> String {
    // UTF-8（含 BOM）优先，失败回退 GBK
    if let Ok(s) = std::str::from_utf8(data) {
        return s.trim_start_matches('\u{feff}').to_string();
    }
    let (cow, _, had_errors) = encoding_rs::GBK.decode(data);
    if !had_errors {
        return cow.into_owned();
    }
    String::from_utf8_lossy(data).into_owned()
}

/// 读取歌词：优先内嵌 LYRICS 标签，其次同名 .lrc / .txt 文件
#[tauri::command]
pub fn read_lyrics(path: String) -> Result<String, String> {
    if let Ok(tagged) = Probe::open(&path).and_then(|p| p.read()) {
        for tag in tagged.tags() {
            if let Some(lyrics) = tag.get_string(&ItemKey::Lyrics) {
                if !lyrics.trim().is_empty() {
                    return Ok(lyrics.to_string());
                }
            }
        }
    }

    let p = Path::new(&path);
    for ext in ["lrc", "LRC", "txt"] {
        let sidecar = p.with_extension(ext);
        if sidecar.is_file() {
            if let Ok(data) = fs::read(&sidecar) {
                let text = decode_text(&data);
                if !text.trim().is_empty() {
                    return Ok(text);
                }
            }
        }
    }

    Ok(String::new())
}

/// 读取任意图片文件为 data URL（窗口背景 / 封面编辑用）
#[tauri::command]
pub fn read_image_file(path: String) -> Result<String, String> {
    let p = Path::new(&path);
    let data = fs::read(p).map_err(|e| format!("读取图片失败: {e}"))?;
    Ok(to_data_url(ext_mime(p), &data))
}
