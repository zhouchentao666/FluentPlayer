use crate::db::music_db::{self, TrackRow, TrackStat};
use lofty::config::WriteOptions;
use lofty::file::{AudioFile, TaggedFileExt};
use lofty::tag::{Accessor, ItemKey, ItemValue, Tag, TagType};
use std::fs::Metadata;
use std::path::Path;
use std::time::SystemTime;
use walkdir::WalkDir;

const AUDIO_EXTENSIONS: &[&str] = &[
    "mp3", "flac", "wav", "ogg", "m4a", "aac", "wma", "opus", "ape", "alac",
];

fn is_audio_file(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| AUDIO_EXTENSIONS.contains(&e.to_lowercase().as_str()))
        .unwrap_or(false)
}

fn read_sidecar_lrc(path: &Path) -> Option<String> {
    let stem = path.file_stem()?.to_string_lossy();
    let dir = path.parent()?;
    let lrc_path = dir.join(format!("{}.lrc", stem));
    let bytes = std::fs::read(&lrc_path).ok().filter(|b| !b.is_empty())?;
    Some(String::from_utf8_lossy(&bytes).into_owned())
}

fn default_tag_type(path: &Path) -> TagType {
    match path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
    {
        Some(ext) if ext == "mp3" => TagType::Id3v2,
        Some(ext) if ext == "m4a" || ext == "mp4" || ext == "aac" || ext == "alac" => {
            TagType::Mp4Ilst
        }
        Some(ext) if ext == "wav" => TagType::RiffInfo,
        _ => TagType::VorbisComments,
    }
}

fn ensure_primary_tag<'a>(
    tagged_file: &'a mut lofty::file::TaggedFile,
    path: &Path,
) -> Result<&'a mut Tag, String> {
    if tagged_file.primary_tag().is_none() {
        tagged_file.insert_tag(Tag::new(default_tag_type(path)));
    }
    tagged_file
        .primary_tag_mut()
        .ok_or_else(|| "No tag found".to_string())
}

pub fn read_lyrics_from_file(path: &Path) -> Option<String> {
    if let Ok(tagged_file) = lofty::read_from_path(path) {
        if let Some(lrc) = tagged_file
            .primary_tag()
            .and_then(|t| t.get_string(&ItemKey::Lyrics).map(|s| s.to_owned()))
            .filter(|s: &String| !s.trim().is_empty())
        {
            return Some(lrc);
        }
    }
    read_sidecar_lrc(path)
}

fn read_tags(path: &Path) -> Option<TrackRow> {
    let tagged_file = lofty::read_from_path(path).ok()?;
    let properties = tagged_file.properties();
    let tag = tagged_file.primary_tag();

    let file_name = path.file_stem()?.to_string_lossy().to_string();
    let (singer, name, album_name, year, has_cover) = match tag {
        Some(t) => (
            t.artist().map(|s| s.to_string()).unwrap_or_default(),
            t.title()
                .map(|s| s.to_string())
                .unwrap_or_else(|| file_name.clone()),
            t.album().map(|s| s.to_string()).unwrap_or_default(),
            t.year().unwrap_or(0) as i64,
            t.pictures().first().is_some(),
        ),
        None => (String::new(), file_name.clone(), String::new(), 0, false),
    };
    let duration = properties.duration().as_secs_f64();
    let bitrate = properties.audio_bitrate().unwrap_or(0) as i64;
    let sample_rate = properties.sample_rate().unwrap_or(0) as i64;
    let channels = properties.channels().unwrap_or(0) as i64;

    let mtime_ms = path
        .metadata()
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);

    let file_size = path.metadata().map(|m| m.len() as i64).unwrap_or(0);

    let songmid = format!("local_{}", md5_hash(path.to_string_lossy().as_bytes()));

    // Read lyrics from tag, fall back to sidecar .lrc file
    let lrc = tag
        .and_then(|t| t.get_string(&ItemKey::Lyrics).map(|s| s.to_owned()))
        .filter(|s: &String| !s.trim().is_empty())
        .or_else(|| read_sidecar_lrc(path));

    Some(TrackRow {
        songmid,
        path: path.to_string_lossy().to_string(),
        url: None,
        singer,
        name,
        album_name,
        album_id: 0,
        source: "local".to_string(),
        interval: format_duration(duration),
        has_cover: if has_cover { 1 } else { 0 },
        cover_key: None,
        year,
        lrc,
        types: "[]".to_string(),
        _types: "{}".to_string(),
        type_url: "{}".to_string(),
        bitrate,
        sample_rate,
        channels,
        duration,
        size: file_size,
        mtime_ms,
        hash: None,
        updated_at: chrono::Utc::now().timestamp(),
    })
}

fn md5_hash(data: &[u8]) -> String {
    // Simple hash for generating unique IDs (not cryptographic)
    let mut hash: u64 = 5381;
    for &byte in data {
        hash = hash.wrapping_mul(33).wrapping_add(byte as u64);
    }
    format!("{:016x}", hash)
}

fn format_duration(secs: f64) -> String {
    let total_secs = secs as u64;
    let min = total_secs / 60;
    let sec = total_secs % 60;
    format!("{:02}:{:02}", min, sec)
}

pub struct ScanResult {
    pub scanned: usize,
    pub added: usize,
    pub updated: usize,
    pub errors: usize,
}

fn metadata_mtime_ms(metadata: &Metadata) -> i64 {
    metadata
        .modified()
        .ok()
        .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn fallback_track(path: &Path, metadata: &Metadata, mtime_ms: i64) -> TrackRow {
    let file_name = path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "Unknown".to_string());
    TrackRow {
        songmid: format!("local_{}", md5_hash(path.to_string_lossy().as_bytes())),
        path: path.to_string_lossy().to_string(),
        url: None,
        singer: String::new(),
        name: file_name,
        album_name: String::new(),
        album_id: 0,
        source: "local".to_string(),
        interval: String::new(),
        has_cover: 0,
        cover_key: None,
        year: 0,
        lrc: None,
        types: "[]".to_string(),
        _types: "{}".to_string(),
        type_url: "{}".to_string(),
        bitrate: 0,
        sample_rate: 0,
        channels: 0,
        duration: 0.0,
        size: metadata.len() as i64,
        mtime_ms,
        hash: None,
        updated_at: chrono::Utc::now().timestamp(),
    }
}

fn flush_tracks(conn: &rusqlite::Connection, tracks: &mut Vec<TrackRow>) {
    if tracks.is_empty() {
        return;
    }
    let _ = music_db::upsert_tracks(conn, tracks);
    tracks.clear();
}

fn is_hidden(path: &Path) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .map(|n| n.starts_with('.'))
        .unwrap_or(false)
}

fn root_scan_can_prune(walk_error_depth: Option<usize>) -> bool {
    walk_error_depth.is_none()
}

pub fn scan_directories(
    conn: &rusqlite::Connection,
    dirs: &[String],
    skip_hidden: bool,
) -> ScanResult {
    const UPSERT_BATCH_SIZE: usize = 128;
    let mut result = ScanResult {
        scanned: 0,
        added: 0,
        updated: 0,
        errors: 0,
    };

    let existing_stats: Vec<TrackStat> = music_db::get_all_stats(conn).unwrap_or_default();
    let stat_map: std::collections::HashMap<String, TrackStat> = existing_stats
        .into_iter()
        .map(|s| (s.path.clone(), s))
        .collect();

    let mut new_tracks: Vec<TrackRow> = Vec::with_capacity(UPSERT_BATCH_SIZE);
    let mut found_paths: Vec<String> = Vec::new();
    let mut completed_roots: Vec<String> = Vec::new();

    for dir in dirs {
        let walker = WalkDir::new(dir).into_iter().filter_entry(|e| {
            if !skip_hidden {
                return true;
            }
            !is_hidden(e.path())
        });
        let mut walk_error_depth = None;
        for entry in walker {
            let entry = match entry {
                Ok(entry) => entry,
                Err(error) => {
                    result.errors += 1;
                    walk_error_depth.get_or_insert(error.depth());
                    continue;
                }
            };
            if !entry.file_type().is_file() {
                continue;
            }
            let path = entry.path();
            if !is_audio_file(path) {
                continue;
            }

            result.scanned += 1;
            let path_str = path.to_string_lossy().to_string();
            found_paths.push(path_str.clone());

            let metadata = match entry.metadata() {
                Ok(metadata) => metadata,
                Err(_) => {
                    result.errors += 1;
                    continue;
                }
            };
            let mtime_ms = metadata_mtime_ms(&metadata);

            if let Some(existing) = stat_map.get(&path_str) {
                if existing.mtime_ms == mtime_ms {
                    continue;
                }
                result.updated += 1;
            } else {
                result.added += 1;
            }

            match read_tags(path) {
                Some(track) => new_tracks.push(track),
                None => {
                    new_tracks.push(fallback_track(path, &metadata, mtime_ms));
                    result.errors += 1;
                }
            }

            if new_tracks.len() >= UPSERT_BATCH_SIZE {
                flush_tracks(conn, &mut new_tracks);
            }
        }
        if root_scan_can_prune(walk_error_depth) {
            completed_roots.push(dir.clone());
        }
    }

    flush_tracks(conn, &mut new_tracks);

    let _ = music_db::prune_scanned_roots(conn, &completed_roots, &found_paths);

    result
}

pub fn get_cover_data(path: &Path) -> Option<Vec<u8>> {
    let tagged_file = lofty::read_from_path(path).ok()?;
    let tag = tagged_file.primary_tag()?;
    let picture = tag.pictures().first()?;
    Some(picture.data().to_vec())
}

pub fn read_file_tags(path: &Path) -> Option<TrackRow> {
    read_tags(path)
}

pub fn write_tags(path: &Path, info: &TrackRow) -> Result<(), String> {
    let mut tagged_file = lofty::read_from_path(path).map_err(|e| e.to_string())?;
    let tag = ensure_primary_tag(&mut tagged_file, path)?;

    if tag.title().map(|v| v.trim().is_empty()).unwrap_or(true) {
        tag.set_title(info.name.clone().into());
    }
    if tag.artist().map(|v| v.trim().is_empty()).unwrap_or(true) {
        tag.set_artist(info.singer.clone().into());
    }
    if tag.album().map(|v| v.trim().is_empty()).unwrap_or(true) {
        tag.set_album(info.album_name.clone().into());
    }
    if info.year > 0 && tag.year().is_none() {
        tag.set_year(info.year as u32);
    }

    tagged_file
        .save_to_path(path, WriteOptions::default())
        .map_err(|e| e.to_string())
}

/// Write tags to a downloaded file with full options
pub fn write_download_tags(
    path: &Path,
    song_info: &serde_json::Value,
    tag_options: &serde_json::Value,
    cover_data: Option<&[u8]>,
    lyrics: Option<&str>,
) -> Result<(), String> {
    let mut tagged_file = lofty::read_from_path(path).map_err(|e| e.to_string())?;
    let tag = ensure_primary_tag(&mut tagged_file, path)?;

    let basic = tag_options
        .get("basicInfo")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    if basic {
        if let Some(name) = song_info.get("name").and_then(|v| v.as_str()) {
            if tag.title().map(|v| v.trim().is_empty()).unwrap_or(true) {
                tag.set_title(name.to_string().into());
            }
        }
        if let Some(singer) = song_info.get("singer").and_then(|v| v.as_str()) {
            if tag.artist().map(|v| v.trim().is_empty()).unwrap_or(true) {
                tag.set_artist(singer.to_string().into());
            }
        }
        if let Some(album) = song_info.get("albumName").and_then(|v| v.as_str()) {
            if tag.album().map(|v| v.trim().is_empty()).unwrap_or(true) {
                tag.set_album(album.to_string().into());
            }
        }
    }

    let write_cover = tag_options
        .get("cover")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if write_cover {
        if tag.pictures().is_empty() {
            if let Some(data) = cover_data {
                let mime_type = if data.starts_with(b"\x89PNG\r\n\x1a\n") {
                    lofty::picture::MimeType::Png
                } else if data.starts_with(b"GIF87a") || data.starts_with(b"GIF89a") {
                    lofty::picture::MimeType::Gif
                } else if data.starts_with(b"BM") {
                    lofty::picture::MimeType::Bmp
                } else {
                    lofty::picture::MimeType::Jpeg
                };
                let pic = lofty::picture::Picture::new_unchecked(
                    lofty::picture::PictureType::CoverFront,
                    Some(mime_type),
                    None,
                    data.to_vec(),
                );
                tag.push_picture(pic);
            }
        }
    }

    let write_lyrics = tag_options
        .get("lyrics")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if write_lyrics {
        let has_lyrics = tag
            .get_string(&ItemKey::Lyrics)
            .map(|v| !v.trim().is_empty())
            .unwrap_or(false);
        if !has_lyrics {
            if let Some(lrc) = lyrics {
                if !lrc.is_empty() {
                    tag.push(lofty::tag::TagItem::new(
                        ItemKey::Lyrics,
                        ItemValue::Text(lrc.to_string()),
                    ));
                }
            }
        }
    }

    tagged_file
        .save_to_path(path, WriteOptions::default())
        .map_err(|e| e.to_string())?;

    // Save separate LRC file if option enabled
    let download_lrc = tag_options
        .get("downloadLyrics")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if download_lrc {
        if let Some(lrc) = lyrics {
            if !lrc.is_empty() {
                let lrc_path = path.with_extension("lrc");
                if !lrc_path.exists() {
                    std::fs::write(&lrc_path, lrc)
                        .map_err(|e| format!("保存歌词文件失败: {}", e))?;
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn track(path: &str) -> TrackRow {
        TrackRow {
            songmid: format!("track-{path}"),
            path: path.to_string(),
            url: None,
            singer: String::new(),
            name: "Existing track".to_string(),
            album_name: String::new(),
            album_id: 0,
            source: "local".to_string(),
            interval: String::new(),
            has_cover: 0,
            cover_key: None,
            year: 0,
            lrc: None,
            types: "[]".to_string(),
            _types: "{}".to_string(),
            type_url: "{}".to_string(),
            bitrate: 0,
            sample_rate: 0,
            channels: 0,
            duration: 0.0,
            size: 0,
            mtime_ms: 0,
            hash: None,
            updated_at: 0,
        }
    }

    #[test]
    fn empty_failed_scan_does_not_clear_existing_tracks() {
        let conn = Connection::open_in_memory().unwrap();
        music_db::init_tables(&conn).unwrap();
        music_db::upsert_track(&conn, &track("/existing/song.mp3")).unwrap();

        let missing_root = std::env::temp_dir().join(format!(
            "mio-missing-scan-{}-{}",
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default(),
        ));
        assert!(!missing_root.exists());

        scan_directories(&conn, &[missing_root.to_string_lossy().into_owned()], false);

        assert!(music_db::get_track_by_path(&conn, "/existing/song.mp3")
            .unwrap()
            .is_some());
    }

    #[test]
    fn nested_walk_error_disables_pruning_for_its_root() {
        assert!(!root_scan_can_prune(Some(2)));
        assert!(root_scan_can_prune(None));
    }

    #[test]
    fn successful_empty_scan_prunes_only_its_root() {
        let conn = Connection::open_in_memory().unwrap();
        music_db::init_tables(&conn).unwrap();
        let root = std::env::temp_dir().join(format!(
            "mio-empty-scan-{}-{}",
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default(),
        ));
        std::fs::create_dir(&root).unwrap();
        let stale_path = root.join("stale.mp3").to_string_lossy().into_owned();
        let sibling_path = format!("{}-sibling/song.mp3", root.display());
        music_db::upsert_track(&conn, &track(&stale_path)).unwrap();
        music_db::upsert_track(&conn, &track("/outside/song.mp3")).unwrap();
        music_db::upsert_track(&conn, &track(&sibling_path)).unwrap();

        scan_directories(&conn, &[root.to_string_lossy().into_owned()], false);

        assert!(music_db::get_track_by_path(&conn, &stale_path)
            .unwrap()
            .is_none());
        assert!(music_db::get_track_by_path(&conn, "/outside/song.mp3")
            .unwrap()
            .is_some());
        assert!(music_db::get_track_by_path(&conn, &sibling_path)
            .unwrap()
            .is_some());
        std::fs::remove_dir(&root).unwrap();
    }
}
