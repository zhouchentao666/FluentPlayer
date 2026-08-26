use serde::{Deserialize, Serialize};

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginInfo {
    pub name: String,
    pub version: String,
    pub author: String,
    #[serde(default)]
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginSource {
    #[serde(default)]
    pub source_id: String,
    pub name: String,
    #[serde(default)]
    pub qualities: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginConfigFieldOption {
    pub label: String,
    pub value: serde_json::Value,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginConfigField {
    pub key: String,
    pub label: String,
    #[serde(rename = "type")]
    pub field_type: String,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub default: Option<serde_json::Value>,
    #[serde(default)]
    pub placeholder: Option<String>,
    #[serde(default)]
    pub options: Option<Vec<PluginConfigFieldOption>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadedPlugin {
    pub plugin_id: String,
    pub plugin_name: String,
    pub plugin_info: PluginInfo,
    pub supported_sources: Vec<PluginSource>,
    #[serde(default = "default_plugin_type")]
    pub plugin_type: String,
}

fn default_plugin_type() -> String {
    "music-source".to_string()
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServicePlaylist {
    pub id: String,
    pub name: String,
    pub song_count: u32,
    #[serde(default)]
    pub cover_img: String,
    #[serde(default)]
    pub description: String,
}
