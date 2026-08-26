use std::collections::HashMap;
use std::sync::Mutex;
use base64::Engine;
use lru::LruCache;
use std::num::NonZeroUsize;

static COVER_CACHE: once_cell::sync::Lazy<Mutex<LruCache<String, String>>> =
    once_cell::sync::Lazy::new(|| {
        Mutex::new(LruCache::new(NonZeroUsize::new(200).unwrap()))
    });

pub fn get_cover_base64(songmid: &str, path: &str) -> Option<String> {
    // Check memory cache first
    {
        let mut cache = COVER_CACHE.lock().unwrap();
        if let Some(cached) = cache.get(songmid) {
            return Some(cached.clone());
        }
    }

    // Read from file
    let path = std::path::Path::new(path);
    let data = crate::local_music::scanner::get_cover_data(path)?;
    let b64 = base64::engine::general_purpose::STANDARD.encode(&data);

    // Detect mime type from extension
    let mime = match path.extension().and_then(|e| e.to_str()).unwrap_or("") {
        "png" => "image/png",
        _ => "image/jpeg",
    };
    let data_uri = format!("data:{};base64,{}", mime, b64);

    // Store in cache (LRU eviction is automatic)
    {
        let mut cache = COVER_CACHE.lock().unwrap();
        cache.put(songmid.to_string(), data_uri.clone());
    }

    Some(data_uri)
}

pub fn batch_get_covers(songmids: &[String], tracks: &[(String, String)]) -> HashMap<String, String> {
    let lookup: std::collections::HashSet<&String> = songmids.iter().collect();
    let mut result = HashMap::new();
    for (songmid, path) in tracks {
        if lookup.contains(songmid) {
            if let Some(cover) = get_cover_base64(songmid, path) {
                result.insert(songmid.clone(), cover);
            }
        }
    }
    result
}

pub fn clear_cache() {
    COVER_CACHE.lock().unwrap().clear();
}
