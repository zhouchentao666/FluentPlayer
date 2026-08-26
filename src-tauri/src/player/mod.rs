pub mod cache;
pub mod commands;
pub mod effects;
pub mod engine;
pub mod media_control;
pub mod spectrum;

use engine::PlayerEngine;
use parking_lot::Mutex;
use std::sync::Arc;

pub type SharedPlayer = Arc<Mutex<PlayerEngine>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum AudioSlot {
    A,
    B,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum PlaybackState {
    Stopped,
    Playing,
    Paused,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerSnapshot {
    pub state: PlaybackState,
    pub position: f64,
    pub duration: f64,
    pub volume: f64,
    pub primary_slot: AudioSlot,
    pub url: String,
    pub is_playing: bool,
}

pub fn init_player(app_handle: tauri::AppHandle) -> SharedPlayer {
    engine::cleanup_temp_files();

    let (stream_handle, shutdown_tx) = match engine::create_output_stream() {
        Ok(pair) => (Some(pair.0), Some(pair.1)),
        Err(e) => {
            eprintln!("音频输出初始化失败: {e}");
            (None, None)
        }
    };

    let engine = PlayerEngine::new(app_handle, stream_handle, shutdown_tx);
    let shared = Arc::new(Mutex::new(engine));
    engine::start_bus_poller(shared.clone());
    shared
}
