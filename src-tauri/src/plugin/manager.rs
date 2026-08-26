use super::engine::PluginEngine;
use super::types::LoadedPlugin;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

fn validate_plugin_id(id: &str) -> Result<(), String> {
    if id.is_empty()
        || id.len() > 128
        || !id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
    {
        return Err("插件 ID 格式无效".to_string());
    }
    Ok(())
}

pub struct PluginManager {
    plugins: Arc<RwLock<HashMap<String, LoadedPlugin>>>,
    engines: Arc<RwLock<HashMap<String, PluginEngine>>>,
    plugins_dir: PathBuf,
    logs: Arc<RwLock<HashMap<String, Vec<String>>>>,
}

impl PluginManager {
    pub fn new(app_data_dir: &std::path::Path) -> Self {
        let plugins_dir = app_data_dir.join("plugins");
        Self {
            plugins: Arc::new(RwLock::new(HashMap::new())),
            engines: Arc::new(RwLock::new(HashMap::new())),
            plugins_dir,
            logs: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Initialize: load all plugins from the plugins directory.
    pub async fn initialize(&self) -> Result<Vec<LoadedPlugin>, String> {
        fs::create_dir_all(&self.plugins_dir)
            .map_err(|e| format!("创建插件目录失败: {}", e))?;

        let mut plugins = self.plugins.write().await;
        let mut engines = self.engines.write().await;
        plugins.clear();
        engines.clear();

        let entries = fs::read_dir(&self.plugins_dir)
            .map_err(|e| format!("读取插件目录失败: {}", e))?;

        let mut result = Vec::new();

        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_file() { continue; }

            let file_name = match path.file_name().and_then(|n| n.to_str()) {
                Some(n) => n.to_string(),
                None => continue,
            };

            // Skip config files
            if file_name.ends_with(".config.json") { continue; }

            let parts: Vec<&str> = file_name.splitn(2, '-').collect();
            if parts.len() < 2 { continue; }
            let plugin_id = parts[0].to_string();
            let plugin_name = parts[1..].join("-");

            match PluginEngine::from_file(&path) {
                Ok(engine) => {
                    let loaded = LoadedPlugin {
                        plugin_id: plugin_id.clone(),
                        plugin_name,
                        plugin_info: engine.info().clone(),
                        supported_sources: engine.sources().to_vec(),
                        plugin_type: engine.plugin_type().to_string(),
                    };
                    engines.insert(plugin_id.clone(), engine);
                    plugins.insert(plugin_id.clone(), loaded.clone());
                    result.push(loaded);
                }
                Err(e) => {
                    eprintln!("[PluginManager] 加载插件 {} 失败: {}", file_name, e);
                }
            }
        }

        Ok(result)
    }

    /// Add a plugin from code string.
    pub async fn add_plugin(
        &self,
        plugin_code: &str,
        plugin_name: &str,
        target_plugin_id: Option<String>,
    ) -> Result<LoadedPlugin, String> {
        let engine = PluginEngine::new(plugin_code)?;
        let info = engine.info().clone();

        fs::create_dir_all(&self.plugins_dir)
            .map_err(|e| e.to_string())?;

        let plugin_id = if let Some(id) = target_plugin_id {
            validate_plugin_id(&id)?;
            id
        } else {
            let plugins = self.plugins.read().await;
            if let Some(existing) = plugins.values().find(|p| p.plugin_info.name == info.name) {
                if existing.plugin_info.version == info.version {
                    return Err(format!("插件 \"{} v{}\" 已存在", info.name, info.version));
                }
                existing.plugin_id.clone()
            } else {
                uuid::Uuid::new_v4().to_string().replace('-', "")
            }
        };

        self.remove_plugin_file(&plugin_id).await;

        let safe_name = plugin_name.replace(|c: char| !c.is_alphanumeric() && c != '-' && c != '_', "_");
        let file_path = self.plugins_dir.join(format!("{}-{}", plugin_id, safe_name));
        fs::write(&file_path, plugin_code)
            .map_err(|e| format!("写入插件文件失败: {}", e))?;

        let engine = PluginEngine::from_file(&file_path)?;
        let loaded = LoadedPlugin {
            plugin_id: plugin_id.clone(),
            plugin_name: safe_name,
            plugin_info: engine.info().clone(),
            supported_sources: engine.sources().to_vec(),
            plugin_type: engine.plugin_type().to_string(),
        };

        {
            let mut engines = self.engines.write().await;
            engines.insert(plugin_id.clone(), engine);
        }
        {
            let mut plugins = self.plugins.write().await;
            plugins.insert(plugin_id.clone(), loaded.clone());
        }

        Ok(loaded)
    }

    /// Remove a plugin.
    pub async fn uninstall_plugin(&self, plugin_id: &str) -> Result<(), String> {
        validate_plugin_id(plugin_id)?;
        self.remove_plugin_file(plugin_id).await;
        // Also remove config file
        let config_path = self.plugins_dir.join(format!("{}.config.json", plugin_id));
        let _ = fs::remove_file(config_path);
        {
            let mut engines = self.engines.write().await;
            engines.remove(plugin_id);
        }
        {
            let mut plugins = self.plugins.write().await;
            plugins.remove(plugin_id);
        }
        {
            let mut logs = self.logs.write().await;
            logs.remove(plugin_id);
        }
        Ok(())
    }

    /// Get all loaded plugins info.
    pub async fn get_plugins_list(&self) -> Vec<LoadedPlugin> {
        let plugins = self.plugins.read().await;
        plugins.values().cloned().collect()
    }

    /// Get plugin type.
    pub async fn get_plugin_type(&self, plugin_id: &str) -> Option<String> {
        let plugins = self.plugins.read().await;
        plugins.get(plugin_id).map(|p| p.plugin_type.clone())
    }

    /// Get config schema for a service plugin.
    pub async fn get_config_schema(&self, plugin_id: &str) -> Result<Vec<serde_json::Value>, String> {
        let engines = self.engines.read().await;
        let engine = engines.get(plugin_id)
            .ok_or_else(|| format!("插件 {} 未找到", plugin_id))?;
        let schema = engine.config_schema();
        Ok(schema.iter().map(|f| serde_json::to_value(f).unwrap_or_default()).collect())
    }

    /// Get saved config for a plugin.
    pub async fn get_config(&self, plugin_id: &str) -> Result<serde_json::Value, String> {
        validate_plugin_id(plugin_id)?;
        let config_path = self.plugins_dir.join(format!("{}.config.json", plugin_id));
        if !config_path.exists() {
            return Ok(serde_json::json!({}));
        }
        let content = fs::read_to_string(&config_path)
            .map_err(|e| format!("读取配置失败: {}", e))?;
        serde_json::from_str(&content)
            .map_err(|e| format!("解析配置失败: {}", e))
    }

    /// Save config for a plugin.
    pub async fn save_config(&self, plugin_id: &str, config: serde_json::Value) -> Result<(), String> {
        validate_plugin_id(plugin_id)?;
        let config_path = self.plugins_dir.join(format!("{}.config.json", plugin_id));
        let content = serde_json::to_string_pretty(&config)
            .map_err(|e| format!("序列化配置失败: {}", e))?;
        fs::write(&config_path, content)
            .map_err(|e| format!("写入配置失败: {}", e))?;
        Ok(())
    }

    /// Test connection for a service plugin.
    pub async fn test_connection(&self, plugin_id: &str) -> Result<serde_json::Value, String> {
        let engines = self.engines.read().await;
        let _engine = engines.get(plugin_id)
            .ok_or_else(|| format!("插件 {} 未找到", plugin_id))?;
        drop(engines);

        let config = self.get_config(plugin_id).await.unwrap_or(serde_json::json!({}));

        // Extract serverUrl from config
        let server_url = config.get("serverUrl")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        if server_url.is_empty() {
            return Ok(serde_json::json!({
                "success": false,
                "message": "未配置服务器地址"
            }));
        }

        // Basic connectivity test
        let test_url = format!("{}/rest/ping?f=json", server_url.trim_end_matches('/'));
        match reqwest::get(&test_url).await {
            Ok(resp) => {
                if resp.status().is_success() {
                    Ok(serde_json::json!({
                        "success": true,
                        "message": "连接成功"
                    }))
                } else {
                    Ok(serde_json::json!({
                        "success": false,
                        "message": format!("HTTP {}", resp.status())
                    }))
                }
            }
            Err(e) => Ok(serde_json::json!({
                "success": false,
                "message": format!("连接失败: {}", e)
            }))
        }
    }

    /// Add a log entry for a plugin.
    #[allow(dead_code)]
    pub async fn add_log(&self, plugin_id: &str, message: &str) {
        let mut logs = self.logs.write().await;
        let entry = logs.entry(plugin_id.to_string()).or_insert_with(Vec::new);
        entry.push(message.to_string());
        // Keep last 1000 entries
        if entry.len() > 1000 {
            let drain_count = entry.len() - 1000;
            entry.drain(0..drain_count);
        }
    }

    /// Get logs for a plugin.
    #[allow(dead_code)]
    pub async fn get_logs(&self, plugin_id: &str) -> Vec<String> {
        let logs = self.logs.read().await;
        logs.get(plugin_id).cloned().unwrap_or_default()
    }

    /// Read plugin log from file (if plugin writes to a log file).
    pub async fn get_plugin_log(&self, plugin_id: &str) -> Vec<String> {
        // First check in-memory logs
        let mem_logs = {
            let logs = self.logs.read().await;
            logs.get(plugin_id).cloned().unwrap_or_default()
        };
        if !mem_logs.is_empty() {
            return mem_logs;
        }

        // Otherwise, return a synthetic log with plugin info
        let plugins = self.plugins.read().await;
        if let Some(plugin) = plugins.get(plugin_id) {
            vec![
                format!("[info] 插件 {} v{} 已加载", plugin.plugin_info.name, plugin.plugin_info.version),
                format!("[info] 类型: {}", plugin.plugin_type),
                format!("[info] 作者: {}", plugin.plugin_info.author),
            ]
        } else {
            vec![]
        }
    }

    /// Get a specific plugin engine for calling methods.
    #[allow(dead_code)]
    pub fn get_engine(&self, plugin_id: &str) -> Option<PluginEngine> {
        let engines = self.engines.try_read().ok()?;
        engines.get(plugin_id).cloned()
    }

    /// Get plugin source code by ID (for frontend JS execution).
    pub fn get_plugin_code(&self, plugin_id: &str) -> Option<String> {
        // Find the plugin file
        if let Ok(entries) = fs::read_dir(&self.plugins_dir) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with(&format!("{}-", plugin_id)) && !name.ends_with(".config.json") {
                    return fs::read_to_string(entry.path()).ok();
                }
            }
        }
        None
    }

    /// Call a plugin method (placeholder - requires JS engine integration).
    pub fn call_plugin_method(
        &self,
        _plugin_id: &str,
        method: &str,
        _args_json: &str,
    ) -> Result<String, String> {
        Err(format!("插件方法 {} 暂未实现（需要JS引擎集成）", method))
    }

    async fn remove_plugin_file(&self, plugin_id: &str) {
        if let Ok(entries) = fs::read_dir(&self.plugins_dir) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with(&format!("{}-", plugin_id)) {
                    let _ = fs::remove_file(entry.path());
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::validate_plugin_id;

    #[test]
    fn plugin_id_allows_only_filename_safe_characters() {
        assert!(validate_plugin_id("safe_ID-123").is_ok());
        assert!(validate_plugin_id("../outside").is_err());
        assert!(validate_plugin_id("nested/path").is_err());
        assert!(validate_plugin_id("").is_err());
    }
}
