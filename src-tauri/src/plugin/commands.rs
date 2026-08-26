#![allow(non_snake_case)]

use crate::network::{HttpPolicy, RestrictedHttpClient};
use crate::plugin::converter;
use crate::plugin::manager::PluginManager;
use serde_json::Value;
use tauri::State;

type ApiResult = Result<serde_json::Value, String>;

fn ok(data: Value) -> ApiResult {
    Ok(serde_json::json!({ "success": true, "data": data }))
}

fn ok_null() -> ApiResult {
    Ok(serde_json::json!({ "success": true, "data": null }))
}

fn err(msg: &str) -> ApiResult {
    Ok(serde_json::json!({ "success": false, "error": msg }))
}

/// Extract payload from bridge's { args: { ... } } wrapper.
fn payload(args: &Value) -> Value {
    args.get("args").cloned().unwrap_or_else(|| args.clone())
}

fn require_str(payload: &Value, key: &str) -> Result<String, String> {
    payload
        .get(key)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| format!("缺少参数: {}", key))
}

fn opt_str(payload: &Value, key: &str) -> Option<String> {
    payload
        .get(key)
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
}

#[tauri::command]
pub async fn plugin__initialize(pm: State<'_, PluginManager>) -> ApiResult {
    let plugins = pm.initialize().await?;
    ok(serde_json::to_value(plugins).unwrap_or_default())
}

#[tauri::command]
pub async fn plugin__get_list(pm: State<'_, PluginManager>) -> ApiResult {
    let plugins = pm.get_plugins_list().await;
    ok(serde_json::to_value(plugins).unwrap_or_default())
}

#[tauri::command]
pub async fn plugin__add(pm: State<'_, PluginManager>, args: Value) -> ApiResult {
    let p = payload(&args);
    let plugin_code = require_str(&p, "pluginCode")?;
    let plugin_name = require_str(&p, "pluginName")?;
    let target_plugin_id = opt_str(&p, "targetPluginId");
    let result = pm
        .add_plugin(&plugin_code, &plugin_name, target_plugin_id)
        .await?;
    ok(serde_json::to_value(result).unwrap_or_default())
}

#[tauri::command]
pub async fn plugin__uninstall(pm: State<'_, PluginManager>, args: Value) -> ApiResult {
    let p = payload(&args);
    let plugin_id = require_str(&p, "pluginId")?;
    pm.uninstall_plugin(&plugin_id).await?;
    ok_null()
}

#[tauri::command]
pub async fn plugin__get_info(pm: State<'_, PluginManager>, args: Value) -> ApiResult {
    let p = payload(&args);
    let plugin_id = require_str(&p, "pluginId")?;
    let plugins = pm.get_plugins_list().await;
    let plugin = plugins
        .iter()
        .find(|pl| pl.plugin_id == plugin_id)
        .ok_or_else(|| format!("插件 {} 未找到", plugin_id))?;
    ok(serde_json::to_value(plugin).unwrap_or_default())
}

#[tauri::command]
pub async fn plugin__call_method(pm: State<'_, PluginManager>, args: Value) -> ApiResult {
    let p = payload(&args);
    let plugin_id = require_str(&p, "pluginId")?;
    let method = require_str(&p, "method")?;
    let args_json = p.get("argsJson").and_then(|v| v.as_str()).unwrap_or("[]");
    let result = pm.call_plugin_method(&plugin_id, &method, args_json)?;
    let parsed: Value = serde_json::from_str(&result).unwrap_or(Value::String(result));
    ok(parsed)
}

#[tauri::command]
pub async fn plugin__download_and_add(pm: State<'_, PluginManager>, args: Value) -> ApiResult {
    let p = payload(&args);
    let url = require_str(&p, "url")?;
    let plugin_type = require_str(&p, "pluginType")?;
    let target_plugin_id = opt_str(&p, "targetPluginId");

    // Plugin packages may be hosted on a user's own LAN/NAS or local dev server.
    let response = RestrictedHttpClient::new(HttpPolicy::plugin_download())
        .fetch_bytes(
            &url,
            5 * 1024 * 1024,
            &[
                "application/javascript",
                "application/x-javascript",
                "text/javascript",
                "text/*",
            ],
        )
        .await
        .map_err(|error| format!("下载失败: {error}"))?;
    let plugin_code = String::from_utf8(response.bytes)
        .map_err(|error| format!("插件内容不是 UTF-8 文本: {error}"))?;

    if plugin_type == "cr" && !plugin_code.to_lowercase().contains("cerumusic") {
        return Err("澜音插件格式校验失败".into());
    }
    if plugin_type == "lx" && !plugin_code.to_lowercase().contains("lx") {
        return Err("洛雪插件格式校验失败".into());
    }

    let plugin_code = if plugin_type == "lx" {
        converter::convert_lx_plugin(&plugin_code)
    } else {
        plugin_code
    };

    let file_name = format!("downloaded_{}", chrono::Utc::now().timestamp_millis());
    let result = pm
        .add_plugin(&plugin_code, &file_name, target_plugin_id)
        .await?;
    ok(serde_json::to_value(result).unwrap_or_default())
}

// ==================== New commands ====================

#[tauri::command]
pub async fn plugin__get_type(pm: State<'_, PluginManager>, args: Value) -> ApiResult {
    let p = payload(&args);
    let plugin_id = require_str(&p, "pluginId")?;
    match pm.get_plugin_type(&plugin_id).await {
        Some(plugin_type) => ok(Value::String(plugin_type)),
        None => err(&format!("插件 {} 未找到", plugin_id)),
    }
}

#[tauri::command]
pub async fn plugin__get_log(pm: State<'_, PluginManager>, args: Value) -> ApiResult {
    let p = payload(&args);
    let plugin_id = require_str(&p, "pluginId")?;
    let logs = pm.get_plugin_log(&plugin_id).await;
    ok(serde_json::to_value(logs).unwrap_or_default())
}

#[tauri::command]
pub async fn plugin__get_config_schema(pm: State<'_, PluginManager>, args: Value) -> ApiResult {
    let p = payload(&args);
    let plugin_id = require_str(&p, "pluginId")?;
    match pm.get_config_schema(&plugin_id).await {
        Ok(schema) => ok(Value::Array(schema)),
        Err(e) => err(&e),
    }
}

#[tauri::command]
pub async fn plugin__get_config(pm: State<'_, PluginManager>, args: Value) -> ApiResult {
    let p = payload(&args);
    let plugin_id = require_str(&p, "pluginId")?;
    match pm.get_config(&plugin_id).await {
        Ok(config) => ok(config),
        Err(e) => err(&e),
    }
}

#[tauri::command]
pub async fn plugin__save_config(pm: State<'_, PluginManager>, args: Value) -> ApiResult {
    let p = payload(&args);
    let plugin_id = require_str(&p, "pluginId")?;
    let config = p.get("config").cloned().unwrap_or(serde_json::json!({}));
    match pm.save_config(&plugin_id, config).await {
        Ok(()) => ok_null(),
        Err(e) => err(&e),
    }
}

#[tauri::command]
pub async fn plugin__test_connection(pm: State<'_, PluginManager>, args: Value) -> ApiResult {
    let p = payload(&args);
    let plugin_id = require_str(&p, "pluginId")?;
    match pm.test_connection(&plugin_id).await {
        Ok(result) => ok(result),
        Err(e) => err(&e),
    }
}

#[tauri::command]
pub async fn plugin__get_code(pm: State<'_, PluginManager>, args: Value) -> ApiResult {
    let p = payload(&args);
    let plugin_id = require_str(&p, "pluginId")?;
    match pm.get_plugin_code(&plugin_id) {
        Some(code) => ok(Value::String(code)),
        None => err(&format!("插件 {} 未找到或无法读取代码", plugin_id)),
    }
}

#[tauri::command]
pub async fn plugin__select_and_add(
    pm: State<'_, PluginManager>,
    app: tauri::AppHandle,
    args: Value,
) -> ApiResult {
    use tauri_plugin_dialog::DialogExt;

    let p = payload(&args);
    let plugin_type = require_str(&p, "pluginType")?;

    let file_path = app
        .dialog()
        .file()
        .add_filter("插件文件", &["js", "txt"])
        .blocking_pick_file();

    let path = match file_path {
        Some(p) => p,
        None => return ok(serde_json::json!({ "canceled": true })),
    };

    let path_str = path.as_path().ok_or_else(|| "无效文件路径".to_string())?;

    let plugin_code =
        std::fs::read_to_string(path_str).map_err(|e| format!("读取文件失败: {}", e))?;

    if plugin_type == "cr" && !plugin_code.to_lowercase().contains("cerumusic") {
        return Err("澜音插件格式校验失败".into());
    }
    if plugin_type == "lx" && !plugin_code.to_lowercase().contains("lx") {
        return Err("洛雪插件格式校验失败".into());
    }

    let plugin_code = if plugin_type == "lx" {
        converter::convert_lx_plugin(&plugin_code)
    } else {
        plugin_code
    };

    let file_name = path_str
        .file_stem()
        .and_then(|s: &std::ffi::OsStr| s.to_str())
        .unwrap_or("imported_plugin")
        .to_string();

    let result = pm.add_plugin(&plugin_code, &file_name, None).await?;
    ok(serde_json::to_value(result).unwrap_or_default())
}
