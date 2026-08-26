use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum DownloadStatus {
    Queued,
    Downloading,
    Paused,
    Completed,
    Error,
    Cancelled,
}

impl Default for DownloadStatus {
    fn default() -> Self {
        DownloadStatus::Queued
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadTask {
    pub id: String,
    pub song_info: serde_json::Value,
    pub url: String,
    pub plugin_id: Option<String>,
    pub quality: Option<String>,
    pub file_path: String,
    pub status: DownloadStatus,
    pub progress: f64,
    pub speed: f64,
    pub total_size: u64,
    pub downloaded_size: u64,
    pub remaining_time: Option<f64>,
    pub retries: u32,
    pub error: Option<String>,
    pub priority: u32,
    pub created_at: i64,
}
