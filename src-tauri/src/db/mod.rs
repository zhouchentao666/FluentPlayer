pub mod music_db;
pub mod playlist_db;

use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

static APP_DATA_DIR: OnceLock<PathBuf> = OnceLock::new();

pub fn set_app_data_dir(path: PathBuf) {
    let _ = APP_DATA_DIR.set(path);
}

pub struct AppDb {
    pub music: Mutex<rusqlite::Connection>,
    pub playlist: Mutex<rusqlite::Connection>,
}

impl AppDb {
    pub fn new(app_data_dir: &PathBuf) -> Result<Self, rusqlite::Error> {
        std::fs::create_dir_all(app_data_dir).ok();

        let music_path = app_data_dir.join("local-music.db");
        let playlist_path = app_data_dir.join("playlists.db");

        let music_conn = rusqlite::Connection::open(&music_path)?;
        music_conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL; PRAGMA foreign_keys=OFF;")?;
        music_db::init_tables(&music_conn)?;

        let playlist_conn = rusqlite::Connection::open(&playlist_path)?;
        playlist_conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL; PRAGMA foreign_keys=ON;")?;
        playlist_db::init_tables(&playlist_conn)?;

        Ok(Self {
            music: Mutex::new(music_conn),
            playlist: Mutex::new(playlist_conn),
        })
    }
}

pub fn get_app_data_dir() -> PathBuf {
    if let Some(dir) = APP_DATA_DIR.get() {
        return dir.clone();
    }
    dirs::data_local_dir()
        .or_else(dirs::data_dir)
        .unwrap_or_else(|| PathBuf::from("."))
        .join("com.vant.Music")
}
