use aes_gcm::{Aes256Gcm, Nonce};
use aes_gcm::aead::{Aead, KeyInit};
use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use hmac::{Hmac, Mac};
use pbkdf2::pbkdf2_hmac;
use serde_json::Value;
use sha2::{Digest, Sha256};
use tauri::State;

use crate::db::{self, playlist_db, AppDb};

type HmacSha256 = Hmac<Sha256>;

fn sha256_hex(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}

fn hmac_sha256(key: &[u8], data: &[u8]) -> Vec<u8> {
    let mut mac = <HmacSha256 as Mac>::new_from_slice(key).expect("HMAC key error");
    mac.update(data);
    mac.finalize().into_bytes().to_vec()
}

fn extract_host(raw_url: &str) -> Result<String, String> {
    let s = raw_url.trim();
    let rest = s
        .strip_prefix("https://")
        .or_else(|| s.strip_prefix("http://"))
        .ok_or("URL 格式错误，需要 http:// 或 https://")?;
    Ok(rest.split('/').next().unwrap_or(rest).to_string())
}

struct S3Signer {
    access_key: String,
    secret_key: String,
    region: String,
}

impl S3Signer {
    fn sign(
        &self,
        method: &str,
        host: &str,
        canonical_uri: &str,
        canonical_query: &str,
        payload_hash: &str,
        content_type: Option<&str>,
    ) -> (String, String) {
        use std::collections::BTreeMap;

        let now = chrono::Utc::now();
        let date_stamp = now.format("%Y%m%d").to_string();
        let amz_date = now.format("%Y%m%dT%H%M%SZ").to_string();

        let mut headers = BTreeMap::new();
        headers.insert("host".to_string(), host.to_string());
        headers.insert("x-amz-content-sha256".to_string(), payload_hash.to_string());
        headers.insert("x-amz-date".to_string(), amz_date.clone());
        if let Some(ct) = content_type {
            headers.insert("content-type".to_string(), ct.to_string());
        }

        let signed_headers: String = headers.keys().map(|k| k.as_str()).collect::<Vec<_>>().join(";");
        let canonical_headers: String = headers
            .iter()
            .map(|(k, v)| format!("{}:{}\n", k, v.trim()))
            .collect();

        let canonical_request = format!(
            "{}\n{}\n{}\n{}\n{}\n{}",
            method, canonical_uri, canonical_query, canonical_headers, signed_headers, payload_hash
        );

        let scope = format!("{}/{}/s3/aws4_request", date_stamp, self.region);
        let string_to_sign = format!(
            "AWS4-HMAC-SHA256\n{}\n{}\n{}",
            amz_date,
            scope,
            sha256_hex(canonical_request.as_bytes())
        );

        let k_date = hmac_sha256(format!("AWS4{}", self.secret_key).as_bytes(), date_stamp.as_bytes());
        let k_region = hmac_sha256(&k_date, self.region.as_bytes());
        let k_service = hmac_sha256(&k_region, b"s3");
        let k_signing = hmac_sha256(&k_service, b"aws4_request");
        let signature = hex::encode(hmac_sha256(&k_signing, string_to_sign.as_bytes()));

        let credential = format!("{}/{}/{}/s3/aws4_request", self.access_key, date_stamp, self.region);
        let authorization = format!(
            "AWS4-HMAC-SHA256 Credential={}, SignedHeaders={}, Signature={}",
            credential, signed_headers, signature
        );

        (authorization, amz_date)
    }
}

fn get_str<'a>(args: &'a Value, key: &str) -> Result<&'a str, String> {
    args.get(key).and_then(|v| v.as_str()).ok_or(format!("缺少 {}", key))
}

fn get_str_or<'a>(args: &'a Value, key: &str, default: &str) -> String {
    args.get(key).and_then(|v| v.as_str()).unwrap_or(default).to_string()
}

fn resolve_region(region: &str) -> String {
    let r = region.trim();
    if r.is_empty() || r == "auto" {
        "us-east-1".to_string()
    } else {
        r.to_string()
    }
}

fn resolve_endpoint_and_bucket(endpoint: &str, bucket_input: &str) -> Result<(String, String), String> {
    let endpoint = endpoint.trim().trim_end_matches('/');
    let bucket_input = bucket_input.trim();

    let (scheme, rest) = if let Some(rest) = endpoint.strip_prefix("https://") {
        ("https", rest)
    } else if let Some(rest) = endpoint.strip_prefix("http://") {
        ("http", rest)
    } else {
        return Err("服务地址格式错误，需要 http:// 或 https://".to_string());
    };

    let mut parts = rest.split('/');
    let host = parts.next().unwrap_or("").trim();
    if host.is_empty() {
        return Err("服务地址缺少 host".to_string());
    }

    let path_bucket = parts.find(|s| !s.trim().is_empty()).map(|s| s.trim().to_string());

    let final_bucket = if let Some(pb) = path_bucket {
        pb
    } else if !bucket_input.is_empty() {
        bucket_input.to_string()
    } else {
        return Err("缺少存储桶：请在 bucket 字段填写，或在 endpoint 后追加 /bucket".to_string());
    };

    let base_endpoint = format!("{}://{}", scheme, host);
    Ok((base_endpoint, final_bucket))
}

fn extract_xml_tag(content: &str, tag: &str) -> Option<String> {
    let open = format!("<{}>", tag);
    let close = format!("</{}>", tag);
    let start = content.find(&open)?;
    let val_start = start + open.len();
    let end = content[val_start..].find(&close)?;
    Some(content[val_start..val_start + end].to_string())
}

fn parse_list_objects(xml: &str) -> Vec<(String, String)> {
    let mut results = Vec::new();
    let mut pos = 0;
    while let Some(start) = xml[pos..].find("<Contents>") {
        let content_start = pos + start + "<Contents>".len();
        if let Some(end) = xml[content_start..].find("</Contents>") {
            let content = &xml[content_start..content_start + end];
            if let (Some(key), Some(lm)) = (extract_xml_tag(content, "Key"), extract_xml_tag(content, "LastModified")) {
                results.push((key, lm));
            }
            pos = content_start + end + "</Contents>".len();
        } else {
            break;
        }
    }
    results
}

const EMPTY_HASH: &str = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

// ====================
// Encryption helpers
// ====================

const PBKDF2_ITERATIONS: u32 = 600_000;
const SALT_LEN: usize = 16;
const NONCE_LEN: usize = 12;
const BACKUP_MAGIC: &[u8; 4] = b"MIOB";
const BACKUP_ENVELOPE_VERSION: u8 = 1;

fn derive_key(password: &str, salt: &[u8]) -> [u8; 32] {
    let mut key = [0u8; 32];
    pbkdf2_hmac::<Sha256>(password.as_bytes(), salt, PBKDF2_ITERATIONS, &mut key);
    key
}

fn encrypt_data(plaintext: &[u8], password: &str) -> Result<Vec<u8>, String> {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let salt: [u8; SALT_LEN] = rng.gen();
    let nonce_bytes: [u8; NONCE_LEN] = rng.gen();

    let key = derive_key(password, &salt);
    let cipher = Aes256Gcm::new_from_slice(&key)
        .map_err(|e| format!("创建加密器失败: {}", e))?;
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher.encrypt(nonce, plaintext)
        .map_err(|e| format!("加密失败: {}", e))?;

    // Output: salt(16) + nonce(12) + ciphertext+tag
    let mut result = Vec::with_capacity(SALT_LEN + NONCE_LEN + ciphertext.len());
    result.extend_from_slice(&salt);
    result.extend_from_slice(&nonce_bytes);
    result.extend_from_slice(&ciphertext);
    Ok(result)
}

fn decrypt_data(encrypted: &[u8], password: &str) -> Result<Vec<u8>, String> {
    if encrypted.len() < SALT_LEN + NONCE_LEN + 16 {
        return Err("加密数据格式错误".to_string());
    }

    let salt = &encrypted[..SALT_LEN];
    let nonce_bytes = &encrypted[SALT_LEN..SALT_LEN + NONCE_LEN];
    let ciphertext = &encrypted[SALT_LEN + NONCE_LEN..];

    let key = derive_key(password, salt);
    let cipher = Aes256Gcm::new_from_slice(&key)
        .map_err(|e| format!("创建解密器失败: {}", e))?;
    let nonce = Nonce::from_slice(nonce_bytes);

    cipher.decrypt(nonce, ciphertext)
        .map_err(|_| "解密失败：密码错误或数据损坏".to_string())
}

fn encode_backup_envelope(plaintext: &[u8], password: &str) -> Result<String, String> {
    let encrypted = encrypt_data(plaintext, password)?;
    let mut envelope = Vec::with_capacity(BACKUP_MAGIC.len() + 1 + encrypted.len());
    envelope.extend_from_slice(BACKUP_MAGIC);
    envelope.push(BACKUP_ENVELOPE_VERSION);
    envelope.extend_from_slice(&encrypted);
    Ok(BASE64.encode(envelope))
}

fn decode_backup_envelope(body: &str, password: &str) -> Result<Vec<u8>, String> {
    let envelope = BASE64
        .decode(body.trim())
        .map_err(|_| "备份格式错误：不是有效的加密备份".to_string())?;
    if envelope.len() <= BACKUP_MAGIC.len() || !envelope.starts_with(BACKUP_MAGIC) {
        return Err("备份格式错误：缺少加密信封".to_string());
    }
    if envelope[BACKUP_MAGIC.len()] != BACKUP_ENVELOPE_VERSION {
        return Err("备份格式错误：不支持的信封版本".to_string());
    }
    decrypt_data(&envelope[BACKUP_MAGIC.len() + 1..], password)
}

fn validate_backup_data(data: &Value) -> Result<(), String> {
    if data.get("version").and_then(Value::as_u64) != Some(3) {
        return Err("不支持的备份数据版本".to_string());
    }
    if !data.get("settings").is_some_and(Value::is_object) {
        return Err("备份设置格式错误".to_string());
    }
    Ok(())
}

// ====================
// Plugin helpers
// ====================

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

fn read_plugins_data() -> Result<Vec<Value>, String> {
    let plugins_dir = db::get_app_data_dir().join("plugins");
    if !plugins_dir.exists() {
        return Ok(Vec::new());
    }

    let entries = std::fs::read_dir(&plugins_dir)
        .map_err(|e| format!("读取插件目录失败: {}", e))?;

    let mut plugins = Vec::new();

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() { continue; }

        let file_name = match path.file_name().and_then(|n| n.to_str()) {
            Some(n) => n.to_string(),
            None => continue,
        };

        if file_name.ends_with(".config.json") { continue; }

        let parts: Vec<&str> = file_name.splitn(2, '-').collect();
        if parts.len() < 2 { continue; }
        let plugin_id = parts[0];
        let plugin_name = parts[1..].join("-");

        let code = std::fs::read_to_string(&path)
            .map_err(|e| format!("读取插件 {} 失败: {}", file_name, e))?;

        let config = {
            let config_path = plugins_dir.join(format!("{}.config.json", plugin_id));
            if config_path.exists() {
                let content = std::fs::read_to_string(&config_path).unwrap_or_default();
                serde_json::from_str(&content).unwrap_or(Value::Null)
            } else {
                Value::Null
            }
        };

        plugins.push(serde_json::json!({
            "id": plugin_id,
            "name": plugin_name,
            "code": code,
            "config": config,
        }));
    }

    Ok(plugins)
}

fn restore_plugins(plugins: &[Value]) -> Result<u32, String> {
    let plugins_dir = db::get_app_data_dir().join("plugins");
    std::fs::create_dir_all(&plugins_dir)
        .map_err(|e| format!("创建插件目录失败: {}", e))?;

    // Collect existing plugin IDs to skip
    let existing_ids: Vec<String> = {
        let entries = std::fs::read_dir(&plugins_dir)
            .map_err(|e| format!("读取插件目录失败: {}", e))?;
        entries.flatten().filter_map(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            if name.ends_with(".config.json") { return None; }
            name.split('-').next().map(|s| s.to_string())
        }).collect()
    };

    let mut restored = 0u32;

    for plugin in plugins {
        let id = plugin.get("id").and_then(|v| v.as_str()).unwrap_or("");
        validate_plugin_id(id)?;
    }

    for plugin in plugins {
        let id = plugin.get("id").and_then(|v| v.as_str()).unwrap_or("");
        let name = plugin.get("name").and_then(|v| v.as_str()).unwrap_or("unknown");
        let code = plugin.get("code").and_then(|v| v.as_str()).unwrap_or("");
        let config = plugin.get("config");

        if existing_ids.iter().any(|e| e == id) {
            continue;
        }

        let safe_name = name.replace(|c: char| !c.is_alphanumeric() && c != '-' && c != '_', "_");
        let file_path = plugins_dir.join(format!("{}-{}", id, safe_name));
        std::fs::write(&file_path, code)
            .map_err(|e| format!("写入插件 {} 失败: {}", id, e))?;

        if let Some(cfg) = config {
            if !cfg.is_null() {
                let config_path = plugins_dir.join(format!("{}.config.json", id));
                let content = serde_json::to_string_pretty(cfg)
                    .map_err(|e| format!("序列化配置失败: {}", e))?;
                std::fs::write(&config_path, content)
                    .map_err(|e| format!("写入配置失败: {}", e))?;
            }
        }

        restored += 1;
    }

    Ok(restored)
}

// ====================
// S3 delete helper
// ====================

async fn delete_s3_object(
    base_endpoint: &str,
    bucket: &str,
    key: &str,
    signer: &S3Signer,
    client: &reqwest::Client,
) -> Result<(), String> {
    let url = format!("{}/{}/{}", base_endpoint, bucket, key);
    let host = extract_host(&url)?;
    let canonical_uri = format!("/{}/{}", bucket, key);

    let (auth, amz_date) = signer.sign("DELETE", &host, &canonical_uri, "", EMPTY_HASH, None);

    let resp = client
        .delete(&url)
        .header("Authorization", auth)
        .header("x-amz-content-sha256", EMPTY_HASH)
        .header("x-amz-date", amz_date)
        .send()
        .await
        .map_err(|e| format!("删除失败: {}", e))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        eprintln!("[S3] 删除旧备份 {} 失败: HTTP {} - {}", key, status, &text[..text.len().min(100)]);
    }

    Ok(())
}

async fn cleanup_old_backups(
    base_endpoint: &str,
    bucket: &str,
    region: &str,
    access_key: &str,
    secret_key: &str,
    max_backups: u32,
    client: &reqwest::Client,
) -> Result<(), String> {
    let base_url = format!("{}/{}", base_endpoint, bucket);
    let host = extract_host(&base_url)?;
    let signer = S3Signer {
        access_key: access_key.to_string(),
        secret_key: secret_key.to_string(),
        region: region.to_string(),
    };

    let query = "list-type=2&max-keys=1000&prefix=mio-backup-";
    let list_url = format!("{}?{}", base_url, query);
    let canonical_uri = format!("/{}", bucket);

    let (auth, amz_date) = signer.sign("GET", &host, &canonical_uri, query, EMPTY_HASH, None);

    let resp = client
        .get(&list_url)
        .header("Authorization", auth)
        .header("x-amz-content-sha256", EMPTY_HASH)
        .header("x-amz-date", amz_date)
        .send()
        .await
        .map_err(|e| format!("列出备份失败: {}", e))?;

    if !resp.status().is_success() {
        return Ok(()); // Non-critical: skip cleanup
    }

    let xml_body = resp.text().await.map_err(|e| format!("读取响应失败: {}", e))?;
    let mut objects = parse_list_objects(&xml_body);
    if objects.len() <= max_backups as usize {
        return Ok(());
    }

    objects.sort_by(|a, b| b.1.cmp(&a.1)); // newest first
    let to_delete = &objects[max_backups as usize..];

    for (key, _) in to_delete {
        let _ = delete_s3_object(base_endpoint, bucket, key, &signer, client).await;
    }

    Ok(())
}

// ====================
// Tauri Commands
// ====================

#[allow(non_snake_case)]
#[tauri::command]
pub async fn s3__test_connection(args: Value) -> Result<bool, String> {
    let endpoint = get_str(&args, "endpoint")?;
    let region = resolve_region(&get_str_or(&args, "region", "auto"));
    let access_key = get_str(&args, "accessKeyId")?;
    let secret_key = get_str(&args, "secretAccessKey")?;
    let bucket_input = get_str_or(&args, "bucket", "");

    let (base_endpoint, bucket) = resolve_endpoint_and_bucket(endpoint, &bucket_input)?;

    let query = "list-type=2&max-keys=1";
    let url = format!("{}/{}?{}", base_endpoint, bucket, query);
    let host = extract_host(&url)?;
    let canonical_uri = format!("/{}", bucket);

    let signer = S3Signer { access_key: access_key.to_string(), secret_key: secret_key.to_string(), region };
    let (auth, amz_date) = signer.sign("GET", &host, &canonical_uri, query, EMPTY_HASH, None);

    let client = reqwest::Client::new();
    let resp = client
        .get(&url)
        .header("Authorization", auth)
        .header("x-amz-content-sha256", EMPTY_HASH)
        .header("x-amz-date", amz_date)
        .send()
        .await
        .map_err(|e| format!("连接失败: {}", e))?;

    if resp.status().is_success() {
        Ok(true)
    } else {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        Err(format!("连接失败: HTTP {} - {}", status, &body[..body.len().min(200)]))
    }
}

#[allow(non_snake_case)]
#[tauri::command]
pub async fn s3__backup(state: State<'_, AppDb>, args: Value) -> Result<Value, String> {
    let endpoint = get_str(&args, "endpoint")?;
    let region = resolve_region(&get_str_or(&args, "region", "auto"));
    let access_key = get_str(&args, "accessKeyId")?;
    let secret_key = get_str(&args, "secretAccessKey")?;
    let bucket_input = get_str_or(&args, "bucket", "");
    let password = get_str(&args, "password")?;
    let max_backups = args.get("maxBackups").and_then(|v| v.as_u64()).unwrap_or(10) as u32;
    let settings = args.get("settings").cloned().unwrap_or(Value::Null);
    if !settings.is_object() {
        return Err("备份设置格式错误".to_string());
    }

    let (base_endpoint, bucket) = resolve_endpoint_and_bucket(endpoint, &bucket_input)?;

    // Read plugins from filesystem
    let plugins = read_plugins_data()?;
    let playlists = {
        let conn = state.playlist.lock().map_err(|e| e.to_string())?;
        playlist_db::export_playlists(&conn).map_err(|e| format!("导出歌单失败: {}", e))?
    };

    let now = chrono::Utc::now();
    let timestamp = now.to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    let key_name = format!("mio-backup-{}.enc", now.format("%Y-%m-%dT%H-%M-%S-%fZ"));

    let backup_data = serde_json::json!({
        "version": 3,
        "timestamp": timestamp,
        "playlists": playlists,
        "settings": settings,
        "plugins": plugins,
    });
    let json_str = serde_json::to_string(&backup_data)
        .map_err(|e| format!("序列化失败: {}", e))?;

    let body = encode_backup_envelope(json_str.as_bytes(), password)?;

    let url = format!("{}/{}/{}", base_endpoint, bucket, key_name);
    let host = extract_host(&url)?;
    let canonical_uri = format!("/{}/{}", bucket, key_name);
    let payload_hash = sha256_hex(body.as_bytes());

    let signer = S3Signer { access_key: access_key.to_string(), secret_key: secret_key.to_string(), region: region.clone() };
    let (auth, amz_date) = signer.sign("PUT", &host, &canonical_uri, "", &payload_hash, Some("application/octet-stream"));

    let client = reqwest::Client::new();
    let resp = client
        .put(&url)
        .header("Authorization", auth)
        .header("x-amz-content-sha256", &payload_hash)
        .header("x-amz-date", amz_date)
        .header("Content-Type", "application/octet-stream")
        .body(body)
        .send()
        .await
        .map_err(|e| format!("上传失败: {}", e))?;

    if resp.status().is_success() {
        // Cleanup old backups
        let _ = cleanup_old_backups(&base_endpoint, &bucket, &region, access_key, secret_key, max_backups, &client).await;

        Ok(serde_json::json!({ "success": true, "timestamp": timestamp, "pluginsCount": plugins.len() }))
    } else {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        Err(format!("上传失败: HTTP {} - {}", status, &text[..text.len().min(200)]))
    }
}

#[allow(non_snake_case)]
#[tauri::command]
pub async fn s3__restore(state: State<'_, AppDb>, args: Value) -> Result<Value, String> {
    let endpoint = get_str(&args, "endpoint")?;
    let region = resolve_region(&get_str_or(&args, "region", "auto"));
    let access_key = get_str(&args, "accessKeyId")?;
    let secret_key = get_str(&args, "secretAccessKey")?;
    let bucket_input = get_str_or(&args, "bucket", "");
    let password = get_str(&args, "password")?;
    let mode = get_str_or(&args, "mode", "overwrite");
    if mode != "overwrite" && mode != "merge" {
        return Err("恢复模式无效".to_string());
    }

    let (base_endpoint, bucket) = resolve_endpoint_and_bucket(endpoint, &bucket_input)?;

    let base_url = format!("{}/{}", base_endpoint, bucket);
    let host = extract_host(&base_url)?;

    let signer = S3Signer { access_key: access_key.to_string(), secret_key: secret_key.to_string(), region: region.clone() };

    // List objects
    let query = "list-type=2&max-keys=1000&prefix=mio-backup-";
    let list_url = format!("{}?{}", base_url, query);
    let canonical_uri = format!("/{}", bucket);

    let (auth, amz_date) = signer.sign("GET", &host, &canonical_uri, query, EMPTY_HASH, None);

    let client = reqwest::Client::new();
    let resp = client
        .get(&list_url)
        .header("Authorization", auth)
        .header("x-amz-content-sha256", EMPTY_HASH)
        .header("x-amz-date", amz_date)
        .send()
        .await
        .map_err(|e| format!("列出备份失败: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("列出备份失败: HTTP {}", resp.status()));
    }

    let xml_body = resp.text().await.map_err(|e| format!("读取响应失败: {}", e))?;
    let mut objects = parse_list_objects(&xml_body);
    if objects.is_empty() {
        return Err("未找到备份数据".to_string());
    }
    objects.sort_by(|a, b| b.1.cmp(&a.1));
    let latest_key = objects[0].0.clone();

    // Download latest backup
    let download_url = format!("{}/{}", base_url, latest_key);
    let download_uri = format!("/{}/{}", bucket, latest_key);

    let signer2 = S3Signer { access_key: access_key.to_string(), secret_key: secret_key.to_string(), region };
    let (auth, amz_date) = signer2.sign("GET", &host, &download_uri, "", EMPTY_HASH, None);

    let resp = client
        .get(&download_url)
        .header("Authorization", auth)
        .header("x-amz-content-sha256", EMPTY_HASH)
        .header("x-amz-date", amz_date)
        .send()
        .await
        .map_err(|e| format!("下载备份失败: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("下载备份失败: HTTP {}", resp.status()));
    }

    let body = resp.text().await.map_err(|e| format!("读取备份数据失败: {}", e))?;

    let decrypted = decode_backup_envelope(&body, password)?;
    let data: Value = serde_json::from_slice(&decrypted)
        .map_err(|e| format!("解析备份数据失败: {}", e))?;
    validate_backup_data(&data)?;

    let playlists: playlist_db::PlaylistBackup = serde_json::from_value(
        data.get("playlists")
            .cloned()
            .ok_or_else(|| "备份缺少歌单数据".to_string())?,
    )
    .map_err(|e| format!("歌单备份格式错误: {}", e))?;

    // Restore plugins if present
    let mut plugins_restored = 0u32;
    let plugins_arr: Vec<Value> = data.get("plugins")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    if !plugins_arr.is_empty() {
        plugins_restored = restore_plugins(&plugins_arr)?;
    }

    {
        let mut conn = state.playlist.lock().map_err(|e| e.to_string())?;
        playlist_db::restore_playlists(&mut conn, &playlists, &mode)
            .map_err(|e| format!("恢复歌单失败: {}", e))?;
    }

    Ok(serde_json::json!({
        "success": true,
        "data": data,
        "pluginsRestored": plugins_restored,
    }))
}

#[cfg(test)]
mod tests {
    use super::{decode_backup_envelope, encode_backup_envelope, validate_backup_data, validate_plugin_id};

    #[test]
    fn backup_envelope_round_trips() {
        let body = encode_backup_envelope(br#"{"version":3}"#, "password").unwrap();
        let decoded = decode_backup_envelope(&body, "password").unwrap();
        assert_eq!(br#"{"version":3}"#, decoded.as_slice());
    }

    #[test]
    fn backup_envelope_rejects_plain_json() {
        let error = decode_backup_envelope(r#"{"version":2}"#, "password").unwrap_err();
        assert!(error.contains("格式"));
    }

    #[test]
    fn backup_envelope_rejects_wrong_password() {
        let body = encode_backup_envelope(b"secret", "right").unwrap();
        assert!(decode_backup_envelope(&body, "wrong").is_err());
    }

    #[test]
    fn plugin_id_rejects_path_components() {
        assert!(validate_plugin_id("../../outside").is_err());
        assert!(validate_plugin_id("").is_err());
        assert!(validate_plugin_id("safe_ID-123").is_ok());
    }

    #[test]
    fn backup_data_requires_object_settings() {
        assert!(validate_backup_data(&serde_json::json!({ "version": 3 })).is_err());
        assert!(validate_backup_data(&serde_json::json!({ "version": 3, "settings": [] })).is_err());
        assert!(validate_backup_data(&serde_json::json!({ "version": 3, "settings": {} })).is_ok());
    }
}
