use std::fs;
use std::path::{Path, PathBuf};

/// Sanitize cache key to be filesystem-safe.
fn safe_path(cache_dir: &Path, key: &str) -> PathBuf {
    let safe: String = key
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect();
    cache_dir.join("audio").join(safe)
}

/// Check if a cached file exists and return its path.
/// Updates mtime on hit for LRU tracking.
pub fn lookup(cache_dir: &Path, key: &str) -> Option<PathBuf> {
    let path = safe_path(cache_dir, key);
    if path.exists() {
        if let Ok(meta) = fs::metadata(&path) {
            if meta.len() > 0 {
                // Touch mtime for LRU
                let _ = fs::File::options().write(true).open(&path).and_then(|f| {
                    f.set_modified(std::time::SystemTime::now())
                });
                return Some(path);
            }
        }
    }
    None
}

/// Copy a downloaded file into the cache and evict if over limit.
pub fn insert(
    cache_dir: &Path,
    key: &str,
    source: &Path,
    max_size: u64,
) -> Result<PathBuf, String> {
    let dir = cache_dir.join("audio");
    fs::create_dir_all(&dir).map_err(|e| format!("创建缓存目录失败: {e}"))?;

    let dest = safe_path(cache_dir, key);
    fs::copy(source, &dest).map_err(|e| format!("缓存写入失败: {e}"))?;

    if max_size > 0 {
        let _ = evict_if_needed(&dir, max_size);
    }
    Ok(dest)
}

/// Delete oldest files until total size is under max_size.
fn evict_if_needed(audio_dir: &Path, max_size: u64) -> Result<(), String> {
    let mut entries: Vec<(PathBuf, u64, std::time::SystemTime)> = Vec::new();
    let mut total: u64 = 0;

    for entry in fs::read_dir(audio_dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        if let Ok(meta) = entry.metadata() {
            if meta.is_file() {
                let size = meta.len();
                let mtime = meta.modified().unwrap_or(std::time::SystemTime::UNIX_EPOCH);
                total += size;
                entries.push((entry.path(), size, mtime));
            }
        }
    }

    if total <= max_size {
        return Ok(());
    }

    entries.sort_by_key(|(_, _, mtime)| *mtime);

    for (path, size, _) in entries {
        if total <= max_size {
            break;
        }
        if fs::remove_file(&path).is_ok() {
            total -= size;
        }
    }
    Ok(())
}

/// Total size of audio cache in bytes.
#[allow(dead_code)]
pub fn total_size(cache_dir: &Path) -> u64 {
    let audio_dir = cache_dir.join("audio");
    if !audio_dir.exists() {
        return 0;
    }

    let mut total: u64 = 0;
    if let Ok(entries) = fs::read_dir(&audio_dir) {
        for entry in entries.flatten() {
            if let Ok(meta) = entry.metadata() {
                if meta.is_file() {
                    total += meta.len();
                }
            }
        }
    }
    total
}

/// Delete all audio cache files.
#[allow(dead_code)]
pub fn clear(cache_dir: &Path) -> Result<(), String> {
    let audio_dir = cache_dir.join("audio");
    if !audio_dir.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(&audio_dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let p = entry.path();
        if p.is_dir() {
            fs::remove_dir_all(&p).map_err(|e| e.to_string())?;
        } else {
            fs::remove_file(&p).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn tmp_dir() -> PathBuf {
        let d = std::env::temp_dir().join("mio_cache_test");
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn test_insert_and_lookup() {
        let dir = tmp_dir();
        let src = dir.join("source.bin");
        fs::write(&src, b"hello cache").unwrap();

        let cached = insert(&dir, "kw_123_320k", &src, 0).unwrap();
        assert!(cached.exists());

        let found = lookup(&dir, "kw_123_320k");
        assert!(found.is_some());
        assert_eq!(fs::read(found.unwrap()).unwrap(), b"hello cache");
    }

    #[test]
    fn test_lookup_miss() {
        let dir = tmp_dir();
        assert!(lookup(&dir, "nonexistent").is_none());
    }

    #[test]
    fn test_eviction() {
        let dir = tmp_dir();

        // Write 3 files of 100 bytes each, max 200 bytes
        for i in 0..3u8 {
            let src = dir.join(format!("src_{i}"));
            let mut f = fs::File::create(&src).unwrap();
            f.write_all(&[i; 100]).unwrap();
            drop(f);

            // Small sleep to ensure different mtimes
            std::thread::sleep(std::time::Duration::from_millis(10));
            insert(&dir, &format!("key_{i}"), &src, 200).unwrap();
        }

        // Only 2 files should remain (newest)
        assert!(lookup(&dir, "key_0").is_none()); // evicted
        assert!(lookup(&dir, "key_1").is_some());
        assert!(lookup(&dir, "key_2").is_some());

        let size = total_size(&dir);
        assert!(size <= 200);
    }

    #[test]
    fn test_clear() {
        let dir = tmp_dir();
        let src = dir.join("src");
        fs::write(&src, b"data").unwrap();

        insert(&dir, "key_1", &src, 0).unwrap();
        insert(&dir, "key_2", &src, 0).unwrap();

        clear(&dir).unwrap();
        assert!(lookup(&dir, "key_1").is_none());
        assert!(lookup(&dir, "key_2").is_none());
        assert_eq!(total_size(&dir), 0);
    }

    #[test]
    fn test_special_chars_in_key() {
        let dir = tmp_dir();
        let src = dir.join("src");
        fs::write(&src, b"data").unwrap();

        let key = "tx_abc/def:ghi";
        insert(&dir, key, &src, 0).unwrap();
        let found = lookup(&dir, key);
        assert!(found.is_some());
    }
}
