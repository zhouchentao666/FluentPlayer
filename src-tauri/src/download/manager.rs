use super::types::{DownloadStatus, DownloadTask};
use futures_util::{future::BoxFuture, FutureExt, StreamExt};
use reqwest::Client;
use serde::Serialize;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use std::sync::{
    atomic::{AtomicU64, AtomicUsize, Ordering},
    Arc,
};
use tauri::Emitter;
use tokio::io::AsyncWriteExt;
use tokio::sync::{oneshot, Mutex, RwLock};
use unicode_casefold::UnicodeCaseFold;
use unicode_normalization::UnicodeNormalization;
use uuid::Uuid;

#[derive(Debug, PartialEq, Eq)]
enum DownloadMode {
    Append { downloaded: u64, total: u64 },
    Truncate { total: u64 },
}

fn normalize_restored_tasks(tasks: &mut [DownloadTask]) -> Vec<String> {
    mark_duplicate_restored_task_ids(tasks);
    for task in tasks.iter_mut() {
        if task.status == DownloadStatus::Downloading {
            task.status = DownloadStatus::Paused;
            task.speed = 0.0;
            task.remaining_time = None;
        }
    }

    let mut ordered_indexes: Vec<usize> = (0..tasks.len()).collect();
    ordered_indexes.sort_by_key(|&index| {
        (
            usize::from(tasks[index].status == DownloadStatus::Queued),
            tasks[index].created_at,
            tasks[index].id.clone(),
            index,
        )
    });
    let mut identities = Vec::new();
    let mut queued = Vec::new();
    for index in ordered_indexes {
        let task = &mut tasks[index];
        let runnable = task.status == DownloadStatus::Queued;
        if reserve_restored_target(task, &mut identities) && runnable {
            queued.push(task.id.clone());
        }
    }
    queued
}

fn mark_duplicate_restored_task_ids(tasks: &mut [DownloadTask]) {
    let mut ids = HashSet::new();
    for (index, task) in tasks.iter_mut().enumerate() {
        if ids.insert(task.id.clone()) {
            continue;
        }
        task.id = unique_restored_task_id(&task.id, index, &ids);
        ids.insert(task.id.clone());
        task.status = DownloadStatus::Error;
        task.error = Some("恢复时检测到重复任务 ID，任务已停止以保留记录".into());
        task.speed = 0.0;
        task.remaining_time = None;
    }
}

fn unique_restored_task_id(original_id: &str, index: usize, ids: &HashSet<String>) -> String {
    let base = format!("{original_id}#restored-duplicate-{index}");
    let mut suffix = 1;
    let mut candidate = base.clone();
    while ids.contains(&candidate) {
        candidate = format!("{base}-{suffix}");
        suffix += 1;
    }
    candidate
}

fn reserve_restored_target(task: &mut DownloadTask, identities: &mut Vec<String>) -> bool {
    match download_target_identity(&task.file_path) {
        Ok(identity) => match reserve_restored_identity(identities, identity) {
            Ok(true) => true,
            Ok(false) => {
                mark_restored_target_conflict(task);
                false
            }
            Err(error) => {
                mark_restored_target_error(task, error);
                false
            }
        },
        Err(error) => {
            mark_restored_target_error(task, error);
            false
        }
    }
}

fn reserve_restored_identity(
    identities: &mut Vec<String>,
    identity: String,
) -> Result<bool, String> {
    for existing in identities.iter() {
        if download_target_identities_match(existing, &identity)? {
            return Ok(false);
        }
    }
    identities.push(identity);
    Ok(true)
}

fn mark_restored_target_conflict(task: &mut DownloadTask) {
    task.status = DownloadStatus::Error;
    task.error = Some("恢复时检测到重复下载目标，任务已停止以避免共享临时文件".into());
    task.speed = 0.0;
    task.remaining_time = None;
}

fn mark_restored_target_error(task: &mut DownloadTask, error: String) {
    task.status = DownloadStatus::Error;
    task.error = Some(format!("恢复时无法确认下载目标，任务已停止：{error}"));
    task.speed = 0.0;
    task.remaining_time = None;
}

fn parse_content_range(value: &reqwest::header::HeaderValue) -> Option<(u64, u64)> {
    let value = value.to_str().ok()?.strip_prefix("bytes ")?;
    let (range, total) = value.split_once('/')?;
    let (start, _) = range.split_once('-')?;
    Some((start.parse().ok()?, total.parse().ok()?))
}

fn resolve_download_mode(
    status: reqwest::StatusCode,
    content_range: Option<&reqwest::header::HeaderValue>,
    start_byte: u64,
    content_length: Option<u64>,
) -> Result<DownloadMode, String> {
    if status == reqwest::StatusCode::PARTIAL_CONTENT {
        let (range_start, total) = content_range
            .and_then(parse_content_range)
            .ok_or_else(|| "断点续传响应缺少有效 Content-Range".to_string())?;
        if range_start != start_byte {
            return Err("断点续传起点与本地临时文件不一致".to_string());
        }
        if content_length == Some(0) || total <= start_byte {
            return Err("断点续传响应大小无效".to_string());
        }
        return Ok(DownloadMode::Append {
            downloaded: start_byte,
            total,
        });
    }
    if status == reqwest::StatusCode::OK {
        let total = content_length.ok_or_else(|| "响应缺少文件大小".to_string())?;
        if total == 0 {
            return Err("文件大小为0".to_string());
        }
        return Ok(DownloadMode::Truncate { total });
    }
    Err(format!("下载失败: HTTP {}", status))
}

fn normalize_download_path(file_path: &str) -> Result<PathBuf, String> {
    if file_path.trim().is_empty() {
        return Err("下载路径不能为空".into());
    }

    let path = Path::new(file_path);
    let absolute_path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|error| error.to_string())?
            .join(path)
    };
    let mut normalized = PathBuf::new();
    for component in absolute_path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            Component::Prefix(prefix) => normalized.push(prefix.as_os_str()),
            Component::RootDir | Component::Normal(_) => normalized.push(component.as_os_str()),
        }
    }
    if normalized.file_name().is_none() {
        return Err("下载路径必须包含文件名".into());
    }
    Ok(normalized)
}

fn canonicalize_target_path(path: &Path) -> Result<PathBuf, String> {
    match fs::canonicalize(path) {
        Ok(canonical) => return Ok(canonical),
        Err(error) if error.kind() != std::io::ErrorKind::NotFound => {
            return Err(error.to_string());
        }
        Err(_) => {}
    }

    let mut missing_components = Vec::new();
    let mut existing_ancestor = path;
    loop {
        match fs::canonicalize(existing_ancestor) {
            Ok(mut canonical) => {
                for component in missing_components.iter().rev() {
                    canonical.push(component);
                }
                return Ok(canonical);
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                let component = existing_ancestor
                    .file_name()
                    .ok_or_else(|| "下载路径缺少可解析的父目录".to_string())?;
                missing_components.push(component.to_os_string());
                existing_ancestor = existing_ancestor
                    .parent()
                    .ok_or_else(|| "下载路径缺少可解析的父目录".to_string())?;
            }
            Err(error) => return Err(error.to_string()),
        }
    }
}

fn download_target_identity(file_path: &str) -> Result<String, String> {
    let canonical = canonical_download_target(file_path)?;
    fold_path_case_for_identity(&canonical)
}

fn canonical_download_target(file_path: &str) -> Result<PathBuf, String> {
    let path = normalize_download_path(file_path)?;
    canonicalize_target_path(&path)
}

fn fold_path_case_for_identity(path: &Path) -> Result<String, String> {
    path.to_str()
        .ok_or_else(|| "下载路径包含非 UTF-8 字符，无法安全确认目标身份".to_string())
        .map(|path| {
            path.nfkc()
                .flat_map(char::to_uppercase)
                .case_fold()
                .collect()
        })
}

#[cfg(not(target_os = "windows"))]
fn download_target_identities_match(left: &str, right: &str) -> Result<bool, String> {
    Ok(left == right)
}

#[cfg(target_os = "windows")]
fn download_target_identities_match(left: &str, right: &str) -> Result<bool, String> {
    const CSTR_EQUAL: i32 = 2;
    let left: Vec<u16> = left.encode_utf16().collect();
    let right: Vec<u16> = right.encode_utf16().collect();
    let left_len = i32::try_from(left.len()).map_err(|_| "下载路径过长".to_string())?;
    let right_len = i32::try_from(right.len()).map_err(|_| "下载路径过长".to_string())?;
    let result =
        unsafe { CompareStringOrdinal(left.as_ptr(), left_len, right.as_ptr(), right_len, 1) };
    if result == 0 {
        return Err(std::io::Error::last_os_error().to_string());
    }
    Ok(result == CSTR_EQUAL)
}

#[cfg(target_os = "windows")]
#[link(name = "kernel32")]
unsafe extern "system" {
    fn CompareStringOrdinal(
        left: *const u16,
        left_len: i32,
        right: *const u16,
        right_len: i32,
        ignore_case: i32,
    ) -> i32;
}

fn temporary_path(file_path: &Path) -> PathBuf {
    PathBuf::from(format!("{}.temp", file_path.to_string_lossy()))
}

fn temporary_path_for_file_path(file_path: &str) -> PathBuf {
    let file_path = normalize_download_path(file_path).unwrap_or_else(|_| PathBuf::from(file_path));
    temporary_path(&file_path)
}

#[derive(Clone, Default)]
struct TaskGeneration(Arc<AtomicU64>);

impl TaskGeneration {
    fn next(&self) -> u64 {
        self.0.fetch_add(1, Ordering::SeqCst) + 1
    }

    fn is_current(&self, generation: u64) -> bool {
        self.0.load(Ordering::SeqCst) == generation
    }
}

struct WorkerHandle {
    abort: tokio::task::AbortHandle,
    done: oneshot::Receiver<()>,
    generation: u64,
}

struct WorkerCompletion {
    done: Option<oneshot::Sender<()>>,
    active_workers: Arc<AtomicUsize>,
}

impl WorkerCompletion {
    fn new(done: oneshot::Sender<()>, active_workers: Arc<AtomicUsize>) -> Self {
        active_workers.fetch_add(1, Ordering::SeqCst);
        Self {
            done: Some(done),
            active_workers,
        }
    }
}

impl Drop for WorkerCompletion {
    fn drop(&mut self) {
        self.active_workers.fetch_sub(1, Ordering::SeqCst);
        if let Some(done) = self.done.take() {
            let _ = done.send(());
        }
    }
}

fn replace_file_atomically(temp_path: &Path, path: &Path) -> std::io::Result<()> {
    #[cfg(not(windows))]
    {
        fs::rename(temp_path, path)
    }

    #[cfg(windows)]
    {
        use std::iter::once;
        use std::os::windows::ffi::OsStrExt;

        unsafe extern "system" {
            fn MoveFileExW(existing: *const u16, replacement: *const u16, flags: u32) -> i32;
            fn ReplaceFileW(
                replaced: *const u16,
                replacement: *const u16,
                backup: *const u16,
                flags: u32,
                exclude: *mut std::ffi::c_void,
                reserved: *mut std::ffi::c_void,
            ) -> i32;
        }

        let destination_exists = path.exists();
        let temp_wide: Vec<u16> = temp_path.as_os_str().encode_wide().chain(once(0)).collect();
        let destination_wide: Vec<u16> = path.as_os_str().encode_wide().chain(once(0)).collect();
        let success = unsafe {
            if destination_exists {
                ReplaceFileW(
                    destination_wide.as_ptr(),
                    temp_wide.as_ptr(),
                    std::ptr::null(),
                    0,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                ) != 0
            } else {
                MoveFileExW(
                    temp_wide.as_ptr(),
                    destination_wide.as_ptr(),
                    0x0000_0001 | 0x0000_0008,
                ) != 0
            }
        };
        if success {
            Ok(())
        } else {
            Err(std::io::Error::last_os_error())
        }
    }
}

fn atomic_write(path: &Path, data: &[u8]) -> std::io::Result<()> {
    let parent = path.parent().ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::InvalidInput, "持久化文件缺少父目录")
    })?;
    fs::create_dir_all(parent)?;

    let file_name = path.file_name().ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::InvalidInput, "持久化文件缺少名称")
    })?;
    let temp_path = parent.join(format!(
        ".{}.{}.tmp",
        file_name.to_string_lossy(),
        Uuid::new_v4()
    ));

    let result = (|| {
        let mut file = fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temp_path)?;
        file.write_all(data)?;
        file.sync_all()?;
        replace_file_atomically(&temp_path, path)?;
        #[cfg(unix)]
        fs::File::open(parent)?.sync_all()?;
        Ok(())
    })();

    if result.is_err() {
        let _ = fs::remove_file(&temp_path);
    }
    result
}

#[derive(Clone)]
pub struct DownloadManager {
    tasks: Arc<RwLock<HashMap<String, DownloadTask>>>,
    queue: Arc<Mutex<Vec<String>>>,
    scheduler: Arc<Mutex<()>>,
    max_concurrent: Arc<RwLock<usize>>,
    lifecycle: Arc<Mutex<()>>,
    workers: Arc<Mutex<HashMap<String, WorkerHandle>>>,
    active_workers: Arc<AtomicUsize>,
    generations: Arc<Mutex<HashMap<String, TaskGeneration>>>,
    persistence: Arc<Mutex<()>>,
    persistence_writer: Arc<std::sync::Mutex<()>>,
    client: Client,
    app_data_dir: PathBuf,
    app_handle: Option<tauri::AppHandle>,
}

impl DownloadManager {
    pub fn new(app_data_dir: &std::path::Path, app_handle: tauri::AppHandle) -> Self {
        let client = Client::builder()
            .connect_timeout(std::time::Duration::from_secs(30))
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
            .build()
            .unwrap_or_default();

        Self {
            tasks: Arc::new(RwLock::new(HashMap::new())),
            queue: Arc::new(Mutex::new(Vec::new())),
            scheduler: Arc::new(Mutex::new(())),
            max_concurrent: Arc::new(RwLock::new(3)),
            lifecycle: Arc::new(Mutex::new(())),
            workers: Arc::new(Mutex::new(HashMap::new())),
            active_workers: Arc::new(AtomicUsize::new(0)),
            generations: Arc::new(Mutex::new(HashMap::new())),
            persistence: Arc::new(Mutex::new(())),
            persistence_writer: Arc::new(std::sync::Mutex::new(())),
            client,
            app_data_dir: app_data_dir.to_path_buf(),
            app_handle: Some(app_handle),
        }
    }

    #[cfg(test)]
    fn new_for_test(app_data_dir: &Path) -> Self {
        let client = Client::new();
        Self {
            tasks: Arc::new(RwLock::new(HashMap::new())),
            queue: Arc::new(Mutex::new(Vec::new())),
            scheduler: Arc::new(Mutex::new(())),
            max_concurrent: Arc::new(RwLock::new(0)),
            lifecycle: Arc::new(Mutex::new(())),
            workers: Arc::new(Mutex::new(HashMap::new())),
            active_workers: Arc::new(AtomicUsize::new(0)),
            generations: Arc::new(Mutex::new(HashMap::new())),
            persistence: Arc::new(Mutex::new(())),
            persistence_writer: Arc::new(std::sync::Mutex::new(())),
            client,
            app_data_dir: app_data_dir.to_path_buf(),
            app_handle: None,
        }
    }

    fn emit<S: Serialize + Clone>(&self, event: &str, payload: S) {
        if let Some(app_handle) = &self.app_handle {
            let _ = app_handle.emit(event, payload);
        }
    }

    fn downloads_file(&self) -> PathBuf {
        self.app_data_dir.join("downloads.json")
    }

    async fn save_tasks(&self) -> Result<(), String> {
        let persistence = self.persistence.clone().lock_owned().await;
        let snapshot = {
            let tasks = self.tasks.read().await;
            let mut tasks: Vec<DownloadTask> = tasks.values().cloned().collect();
            tasks.sort_by(|left, right| left.id.cmp(&right.id));
            tasks
        };
        let data = serde_json::to_vec_pretty(&snapshot).map_err(|error| error.to_string())?;
        let path = self.downloads_file();
        let writer = self.persistence_writer.clone();

        let coordinator = tokio::spawn(async move {
            let _persistence = persistence;
            tokio::task::spawn_blocking(move || {
                let _writer = writer
                    .lock()
                    .map_err(|_| std::io::Error::other("下载任务持久化写锁已中毒"))?;
                atomic_write(&path, &data)
            })
            .await
            .map_err(|error| error.to_string())?
            .map_err(|error| error.to_string())
        })
        .await;

        coordinator.map_err(|error| error.to_string())?
    }

    async fn save_tasks_or_emit(&self) {
        if let Err(error) = self.save_tasks().await {
            self.emit(
                "download:persistence-error",
                serde_json::json!({ "error": error }),
            );
        }
    }

    async fn next_generation(&self, task_id: &str) -> u64 {
        let mut generations = self.generations.lock().await;
        generations.entry(task_id.to_string()).or_default().next()
    }

    async fn generation_is_current(&self, task_id: &str, generation: u64) -> bool {
        self.generations
            .lock()
            .await
            .get(task_id)
            .is_some_and(|state| state.is_current(generation))
    }

    async fn finalize_current_worker_file(
        &self,
        task_id: &str,
        generation: u64,
        temp_path: &Path,
        file_path: &Path,
    ) -> Result<DownloadTask, String> {
        let _scheduler = self.scheduler.lock().await;
        let generations = self.generations.lock().await;
        if !generations
            .get(task_id)
            .is_some_and(|state| state.is_current(generation))
        {
            return Err("任务已被取消".into());
        }
        replace_file_atomically(temp_path, file_path).map_err(|error| error.to_string())?;
        let mut tasks = self.tasks.write().await;
        let task = tasks.get_mut(task_id).ok_or("任务已被取消")?;
        task.status = DownloadStatus::Completed;
        task.progress = 100.0;
        task.downloaded_size = task.total_size;
        task.file_path = file_path.to_string_lossy().into_owned();
        Ok(task.clone())
    }

    async fn update_current_worker_task(
        &self,
        task_id: &str,
        generation: u64,
        update: impl FnOnce(&mut DownloadTask),
    ) -> Option<DownloadTask> {
        let generations = self.generations.lock().await;
        if !generations
            .get(task_id)
            .is_some_and(|state| state.is_current(generation))
        {
            return None;
        }
        let mut tasks = self.tasks.write().await;
        let task = tasks.get_mut(task_id)?;
        update(task);
        Some(task.clone())
    }

    async fn emit_worker_event(
        &self,
        task_id: &str,
        generation: u64,
        event: &str,
        task: &DownloadTask,
    ) {
        let generations = self.generations.lock().await;
        if generations
            .get(task_id)
            .is_some_and(|state| state.is_current(generation))
        {
            self.emit(event, task);
        }
    }

    async fn enqueue_current_worker(&self, task_id: &str, generation: u64) -> bool {
        let generations = self.generations.lock().await;
        if !generations
            .get(task_id)
            .is_some_and(|state| state.is_current(generation))
        {
            return false;
        }
        let tasks = self.tasks.read().await;
        if !matches!(tasks.get(task_id), Some(task) if task.status == DownloadStatus::Queued) {
            return false;
        }
        let mut queue = self.queue.lock().await;
        if !queue.iter().any(|queued| queued == task_id) {
            queue.push(task_id.to_string());
        }
        true
    }

    async fn stop_worker_locked(
        &self,
        task_id: &str,
        transition: impl FnOnce(&mut DownloadTask) -> Result<bool, String>,
    ) -> Result<Option<DownloadTask>, String> {
        let (task, worker) = {
            let _scheduler = self.scheduler.lock().await;
            let mut generations = self.generations.lock().await;
            let mut tasks = self.tasks.write().await;
            let task = tasks.get_mut(task_id).ok_or("任务不存在")?;
            if !transition(task)? {
                return Ok(None);
            }
            let task = task.clone();
            generations.entry(task_id.to_string()).or_default().next();
            let worker = self.workers.lock().await.remove(task_id);
            if let Some(worker) = &worker {
                worker.abort.abort();
            }
            (task, worker)
        };
        if let Some(worker) = worker {
            let _ = worker.done.await;
        }
        Ok(Some(task))
    }

    async fn finish_worker(
        &self,
        task_id: &str,
        generation: u64,
        completion: WorkerCompletion,
        should_retry: bool,
    ) {
        {
            let _scheduler = self.scheduler.lock().await;
            {
                let mut workers = self.workers.lock().await;
                if workers
                    .get(task_id)
                    .is_some_and(|worker| worker.generation == generation)
                {
                    workers.remove(task_id);
                }
            }
            drop(completion);
            if should_retry {
                self.enqueue_current_worker(task_id, generation).await;
            }
        }
        if self.generation_is_current(task_id, generation).await {
            self.spawn_next().await;
        }
    }

    async fn remove_from_queue(&self, task_id: &str) {
        self.queue.lock().await.retain(|id| id != task_id);
    }

    pub async fn load_tasks(&self) {
        if let Ok(data) = fs::read_to_string(self.downloads_file()) {
            if let Ok(mut vec) = serde_json::from_str::<Vec<DownloadTask>>(&data) {
                for task in &mut vec {
                    if let Ok(file_path) = normalize_download_path(&task.file_path) {
                        task.file_path = file_path.to_string_lossy().into_owned();
                    }
                }
                let restored_queue = normalize_restored_tasks(&mut vec);
                let mut tasks = self.tasks.write().await;
                let mut queue = self.queue.lock().await;
                queue.clear();
                queue.extend(restored_queue);
                for t in vec {
                    tasks.insert(t.id.clone(), t);
                }
            }
        }
    }

    pub async fn get_tasks(&self) -> Vec<DownloadTask> {
        self.tasks.read().await.values().cloned().collect()
    }

    pub async fn add_task(
        &self,
        song_info: Value,
        url: String,
        file_path: String,
        plugin_id: Option<String>,
        quality: Option<String>,
        priority: u32,
    ) -> Result<DownloadTask, String> {
        let file_path = normalize_download_path(&file_path)?;
        let file_path_string = file_path.to_string_lossy().into_owned();
        let task = {
            let _lifecycle = self.lifecycle.lock().await;
            let target_identity = download_target_identity(&file_path_string)?;
            let mut tasks = self.tasks.write().await;
            for existing in tasks.values() {
                if download_target_identities_match(
                    &download_target_identity(&existing.file_path)?,
                    &target_identity,
                )? {
                    return match existing.status {
                        DownloadStatus::Completed => Err("歌曲已下载完成".into()),
                        DownloadStatus::Downloading
                        | DownloadStatus::Queued
                        | DownloadStatus::Paused => Err("歌曲正在下载中".into()),
                        DownloadStatus::Error | DownloadStatus::Cancelled => {
                            Err("下载目标已被现有任务占用，请先删除该任务".into())
                        }
                    };
                }
            }

            let task = DownloadTask {
                id: Uuid::new_v4().to_string(),
                song_info,
                url,
                plugin_id,
                quality,
                file_path: file_path_string,
                status: DownloadStatus::Queued,
                progress: 0.0,
                speed: 0.0,
                total_size: 0,
                downloaded_size: 0,
                remaining_time: None,
                retries: 0,
                error: None,
                priority,
                created_at: chrono::Utc::now().timestamp_millis(),
            };

            tasks.insert(task.id.clone(), task.clone());
            drop(tasks);
            self.queue.lock().await.push(task.id.clone());
            task
        };
        self.save_tasks().await?;
        self.emit("download:task-added", &task);
        self.spawn_next().await;
        Ok(task)
    }

    pub async fn pause_task(&self, task_id: &str) -> Result<(), String> {
        let _lifecycle = self.lifecycle.lock().await;
        let task_snapshot = self
            .stop_worker_locked(task_id, |task| {
                if !matches!(
                    task.status,
                    DownloadStatus::Downloading | DownloadStatus::Queued
                ) {
                    return Ok(false);
                }
                task.status = DownloadStatus::Paused;
                task.speed = 0.0;
                task.remaining_time = None;
                Ok(true)
            })
            .await?;
        let Some(task_snapshot) = task_snapshot else {
            return Ok(());
        };
        self.remove_from_queue(task_id).await;
        self.save_tasks().await?;
        self.emit("download:task-status-changed", &task_snapshot);
        self.spawn_next().await;
        Ok(())
    }

    pub async fn resume_task(&self, task_id: &str) -> Result<(), String> {
        let _lifecycle = self.lifecycle.lock().await;
        let task_snapshot;
        {
            let mut tasks = self.tasks.write().await;
            if let Some(t) = tasks.get_mut(task_id) {
                if t.status != DownloadStatus::Paused {
                    return Ok(());
                }
                t.status = DownloadStatus::Queued;
                task_snapshot = t.clone();
            } else {
                return Err("任务不存在".into());
            }
        }
        self.queue.lock().await.push(task_id.to_string());
        self.save_tasks().await?;
        self.emit("download:task-status-changed", &task_snapshot);
        self.spawn_next().await;
        Ok(())
    }

    pub async fn cancel_task(&self, task_id: &str) -> Result<(), String> {
        let _lifecycle = self.lifecycle.lock().await;
        let task_snapshot = self
            .stop_worker_locked(task_id, |task| {
                task.status = DownloadStatus::Cancelled;
                task.speed = 0.0;
                task.remaining_time = None;
                Ok(true)
            })
            .await?
            .ok_or("任务不存在")?;
        self.remove_from_queue(task_id).await;
        let _ = fs::remove_file(temporary_path_for_file_path(&task_snapshot.file_path));
        self.save_tasks().await?;
        self.emit("download:task-status-changed", &task_snapshot);
        self.spawn_next().await;
        Ok(())
    }

    pub async fn delete_task(&self, task_id: &str, delete_file: bool) -> Result<(), String> {
        let _lifecycle = self.lifecycle.lock().await;
        let task = self
            .stop_worker_locked(task_id, |_| Ok(true))
            .await?
            .ok_or("任务不存在")?;
        self.remove_from_queue(task_id).await;
        let _ = fs::remove_file(temporary_path_for_file_path(&task.file_path));
        if delete_file && !task.file_path.is_empty() {
            let _ = fs::remove_file(&task.file_path);
        }
        self.tasks.write().await.remove(task_id);
        self.generations.lock().await.remove(task_id);
        self.save_tasks().await?;
        self.emit("download:task-deleted", task_id);
        Ok(())
    }

    pub async fn retry_task(&self, task_id: &str) -> Result<(), String> {
        let _lifecycle = self.lifecycle.lock().await;
        {
            let tasks = self.tasks.read().await;
            let task = tasks.get(task_id).ok_or("任务不存在")?;
            if task.status == DownloadStatus::Completed {
                return Err("已完成任务不能重试".into());
            }
            if !matches!(
                task.status,
                DownloadStatus::Error | DownloadStatus::Cancelled
            ) {
                return Ok(());
            }
            let target_identity = download_target_identity(&task.file_path)?;
            for (existing_id, existing) in tasks.iter() {
                if existing_id != task_id
                    && download_target_identities_match(
                        &download_target_identity(&existing.file_path)?,
                        &target_identity,
                    )?
                {
                    return Err("下载目标已被其他任务占用".into());
                }
            }
        }
        let task_snapshot = self
            .stop_worker_locked(task_id, |task| {
                task.status = DownloadStatus::Queued;
                task.retries = 0;
                task.error = None;
                task.progress = 0.0;
                task.downloaded_size = 0;
                task.speed = 0.0;
                task.remaining_time = None;
                Ok(true)
            })
            .await?;
        let Some(task_snapshot) = task_snapshot else {
            return Ok(());
        };
        self.remove_from_queue(task_id).await;
        self.queue.lock().await.push(task_id.to_string());
        self.save_tasks().await?;
        self.emit("download:task-status-changed", &task_snapshot);
        self.spawn_next().await;
        Ok(())
    }

    pub async fn pause_all_tasks(&self) {
        let _lifecycle = self.lifecycle.lock().await;
        let ids: Vec<String> = self
            .tasks
            .read()
            .await
            .values()
            .filter(|task| {
                matches!(
                    task.status,
                    DownloadStatus::Downloading | DownloadStatus::Queued
                )
            })
            .map(|t| t.id.clone())
            .collect();

        let mut paused = Vec::with_capacity(ids.len());
        for id in &ids {
            match self
                .stop_worker_locked(id, |task| {
                    if !matches!(
                        task.status,
                        DownloadStatus::Downloading | DownloadStatus::Queued
                    ) {
                        return Ok(false);
                    }
                    task.status = DownloadStatus::Paused;
                    task.speed = 0.0;
                    task.remaining_time = None;
                    Ok(true)
                })
                .await
            {
                Ok(Some(task)) => {
                    self.remove_from_queue(id).await;
                    paused.push(task);
                }
                Ok(None) | Err(_) => {}
            }
        }
        self.save_tasks_or_emit().await;
        for task in paused {
            self.emit("download:task-status-changed", task);
        }
    }

    pub async fn resume_all_tasks(&self) {
        let ids: Vec<String> = self
            .tasks
            .read()
            .await
            .values()
            .filter(|t| t.status == DownloadStatus::Paused)
            .map(|t| t.id.clone())
            .collect();
        for id in ids {
            let _ = self.resume_task(&id).await;
        }
    }

    pub async fn clear_tasks(&self, task_type: &str) {
        let _lifecycle = self.lifecycle.lock().await;
        let selected_tasks: Vec<(String, PathBuf)> = self
            .tasks
            .read()
            .await
            .values()
            .filter(|t| match task_type {
                "queue" => matches!(
                    t.status,
                    DownloadStatus::Downloading | DownloadStatus::Queued | DownloadStatus::Paused
                ),
                "completed" => t.status == DownloadStatus::Completed,
                "failed" => matches!(t.status, DownloadStatus::Error | DownloadStatus::Cancelled),
                "all" => true,
                _ => false,
            })
            .map(|task| {
                let file_path = normalize_download_path(&task.file_path)
                    .unwrap_or_else(|_| PathBuf::from(&task.file_path));
                (task.id.clone(), file_path)
            })
            .collect();
        let ids: Vec<&str> = selected_tasks.iter().map(|(id, _)| id.as_str()).collect();
        for id in &ids {
            let _ = self
                .stop_worker_locked(id, |task| {
                    task.status = DownloadStatus::Cancelled;
                    task.speed = 0.0;
                    task.remaining_time = None;
                    Ok(true)
                })
                .await;
        }
        let mut queue = self.queue.lock().await;
        queue.retain(|id| !ids.iter().any(|selected| selected == &id.as_str()));
        drop(queue);
        for (_, file_path) in &selected_tasks {
            let _ = fs::remove_file(temporary_path(file_path));
        }
        {
            let mut tasks = self.tasks.write().await;
            for id in &ids {
                tasks.remove(*id);
            }
        }
        {
            let mut generations = self.generations.lock().await;
            for id in &ids {
                generations.remove(*id);
            }
        }
        self.save_tasks_or_emit().await;
        let all = self.get_tasks().await;
        self.emit("download:tasks-reset", all);
    }

    pub async fn set_max_concurrent(&self, max: usize) {
        *self.max_concurrent.write().await = max;
        self.spawn_next().await;
    }

    pub async fn get_max_concurrent(&self) -> usize {
        *self.max_concurrent.read().await
    }

    pub async fn validate_files(&self) {
        let mut changed = false;
        {
            let mut tasks = self.tasks.write().await;
            for t in tasks.values_mut() {
                if t.status == DownloadStatus::Completed
                    && !t.file_path.is_empty()
                    && !std::path::Path::new(&t.file_path).exists()
                {
                    t.status = DownloadStatus::Error;
                    t.error = Some("文件已删除或移动".into());
                    changed = true;
                }
            }
        }
        if changed {
            self.save_tasks_or_emit().await;
        }
    }

    pub async fn open_file_location(&self, file_path: &str) -> Result<(), String> {
        #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
        let _ = file_path;

        #[cfg(target_os = "macos")]
        {
            std::process::Command::new("open")
                .args(["-R", file_path])
                .spawn()
                .map_err(|e| e.to_string())?;
        }
        #[cfg(target_os = "windows")]
        {
            std::process::Command::new("explorer")
                .args(["/select,", file_path])
                .spawn()
                .map_err(|e| e.to_string())?;
        }
        #[cfg(target_os = "linux")]
        {
            if let Some(p) = std::path::Path::new(file_path).parent() {
                std::process::Command::new("xdg-open")
                    .arg(p)
                    .spawn()
                    .map_err(|e| e.to_string())?;
            }
        }
        Ok(())
    }

    pub(crate) fn spawn_next(&self) -> BoxFuture<'static, ()> {
        let manager = self.clone();
        async move { manager.spawn_next_inner().await }.boxed()
    }

    async fn spawn_next_inner(&self) {
        let _scheduler_guard = self.scheduler.lock().await;
        loop {
            let mc = *self.max_concurrent.read().await;
            let active = self.active_workers.load(Ordering::SeqCst);
            if active >= mc {
                return;
            }

            let task_id = {
                let tasks = self.tasks.read().await;
                let workers = self.workers.lock().await;
                let mut queue = self.queue.lock().await;
                queue.retain(|id| {
                    matches!(tasks.get(id), Some(task) if task.status == DownloadStatus::Queued)
                });
                let Some(position) = queue.iter().position(|id| !workers.contains_key(id)) else {
                    return;
                };
                queue.remove(position)
            };

            let task = {
                let mut tasks = self.tasks.write().await;
                match tasks.get_mut(&task_id) {
                    Some(task) if task.status == DownloadStatus::Queued => {
                        task.status = DownloadStatus::Downloading;
                        Some(task.clone())
                    }
                    _ => None,
                }
            };
            let task = match task {
                Some(t) => t,
                None => continue,
            };
            let generation = self.next_generation(&task_id).await;

            self.emit("download:task-status-changed", &task);

            let dm = self.clone();
            let id = task_id.clone();
            let client = self.client.clone();
            let file_path = task.file_path.clone();
            let url = task.url.clone();
            let (done, done_receiver) = oneshot::channel();

            let handle = tokio::spawn(run_download_task(
                dm.clone(),
                id.clone(),
                client,
                url,
                file_path,
                generation,
                WorkerCompletion::new(done, self.active_workers.clone()),
            ));

            let mut workers = self.workers.lock().await;
            match workers.entry(task_id) {
                std::collections::hash_map::Entry::Vacant(entry) => {
                    entry.insert(WorkerHandle {
                        abort: handle.abort_handle(),
                        done: done_receiver,
                        generation,
                    });
                }
                std::collections::hash_map::Entry::Occupied(_) => {
                    handle.abort();
                }
            }
        }
    }
}

async fn run_download_task(
    dm: DownloadManager,
    id: String,
    client: Client,
    url: String,
    file_path: String,
    generation: u64,
    completion: WorkerCompletion,
) {
    let mut should_retry = false;
    match execute_download(&client, &url, &file_path, &dm, &id, generation).await {
        Ok(task_snapshot) => {
            dm.save_tasks_or_emit().await;
            dm.emit_worker_event(&id, generation, "download:task-completed", &task_snapshot)
                .await;
        }
        Err(e) => {
            let task_snapshot = dm
                .update_current_worker_task(&id, generation, |task| {
                    if task.retries < 3 {
                        task.retries += 1;
                        task.status = DownloadStatus::Queued;
                        task.error = None;
                        task.progress = 0.0;
                        task.downloaded_size = 0;
                        should_retry = true;
                    } else {
                        task.status = DownloadStatus::Error;
                        task.error = Some(e);
                    }
                })
                .await;
            if let Some(task_snapshot) = task_snapshot {
                dm.save_tasks_or_emit().await;
                if should_retry {
                    dm.emit_worker_event(
                        &id,
                        generation,
                        "download:task-status-changed",
                        &task_snapshot,
                    )
                    .await;
                } else if !should_retry {
                    dm.emit_worker_event(&id, generation, "download:task-error", &task_snapshot)
                        .await;
                }
            }
        }
    }
    dm.finish_worker(&id, generation, completion, should_retry)
        .await;
}

async fn execute_download(
    client: &Client,
    url: &str,
    file_path: &str,
    dm: &DownloadManager,
    task_id: &str,
    generation: u64,
) -> Result<DownloadTask, String> {
    let file_path = normalize_download_path(file_path)?;
    let temp_path = temporary_path(&file_path);

    if let Some(parent) = file_path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    let start_byte = fs::metadata(&temp_path).map(|m| m.len()).unwrap_or(0);

    let mut request = client.get(url);
    if start_byte > 0 {
        request = request.header("Range", format!("bytes={}-", start_byte));
    }

    let response = request.send().await.map_err(|e| e.to_string())?;
    let mode = resolve_download_mode(
        response.status(),
        response.headers().get(reqwest::header::CONTENT_RANGE),
        start_byte,
        response.content_length(),
    )?;
    let (downloaded_start, total_size, append) = match mode {
        DownloadMode::Append { downloaded, total } => (downloaded, total, true),
        DownloadMode::Truncate { total } => (0, total, false),
    };

    dm.update_current_worker_task(task_id, generation, |task| task.total_size = total_size)
        .await
        .ok_or_else(|| "任务已被取消".to_string())?;

    let mut stream = response.bytes_stream();

    let mut file = tokio::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .append(append)
        .truncate(!append)
        .open(&temp_path)
        .await
        .map_err(|e| e.to_string())?;

    let mut downloaded = downloaded_start;
    let mut last_emit = std::time::Instant::now();
    let mut last_speed_time = std::time::Instant::now();
    let mut last_speed_bytes = downloaded;
    let emit_interval = std::time::Duration::from_millis(500);
    let progress_threshold = 256 * 1024u64; // 256KB

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| e.to_string())?;
        file.write_all(&chunk).await.map_err(|e| e.to_string())?;
        downloaded += chunk.len() as u64;

        let now = std::time::Instant::now();
        let size_since_last = downloaded.abs_diff(downloaded_start);

        if size_since_last >= progress_threshold || now.duration_since(last_emit) >= emit_interval {
            let progress = if total_size > 0 {
                (downloaded as f64 / total_size as f64) * 100.0
            } else {
                0.0
            };
            let elapsed = now.duration_since(last_speed_time).as_secs_f64();
            let speed = if elapsed > 0.0 {
                ((downloaded - last_speed_bytes) as f64 / elapsed) as f64
            } else {
                0.0
            };
            let remaining = if speed > 0.0 {
                Some(total_size.saturating_sub(downloaded) as f64 / speed)
            } else {
                None
            };

            let task_snapshot = dm
                .update_current_worker_task(task_id, generation, |task| {
                    task.progress = progress;
                    task.downloaded_size = downloaded;
                    task.speed = speed;
                    task.remaining_time = remaining;
                })
                .await
                .ok_or_else(|| "任务已被取消".to_string())?;
            dm.emit_worker_event(
                task_id,
                generation,
                "download:task-progress",
                &task_snapshot,
            )
            .await;
            last_emit = now;
            last_speed_time = now;
            last_speed_bytes = downloaded;
        }
    }

    file.flush().await.map_err(|e| e.to_string())?;
    drop(file);
    if downloaded != total_size {
        return Err(format!(
            "下载文件大小不一致: 预期 {}, 实际 {}",
            total_size, downloaded
        ));
    }
    dm.finalize_current_worker_file(task_id, generation, &temp_path, &file_path)
        .await
}

#[cfg(test)]
mod tests {
    use super::{
        atomic_write, download_target_identities_match, download_target_identity,
        fold_path_case_for_identity, normalize_restored_tasks, resolve_download_mode,
        temporary_path_for_file_path, DownloadManager, DownloadMode, TaskGeneration,
        WorkerCompletion, WorkerHandle,
    };
    use crate::download::types::{DownloadStatus, DownloadTask};
    use reqwest::{header::HeaderValue, StatusCode};
    use serde_json::Value;
    use std::sync::atomic::Ordering;
    use tokio::sync::oneshot;

    fn task(id: &str, status: DownloadStatus) -> DownloadTask {
        DownloadTask {
            id: id.into(),
            song_info: Value::Null,
            url: "https://example.com/file".into(),
            plugin_id: None,
            quality: None,
            file_path: format!("/tmp/{id}"),
            status,
            progress: 0.0,
            speed: 0.0,
            total_size: 0,
            downloaded_size: 0,
            remaining_time: None,
            retries: 0,
            error: None,
            priority: 0,
            created_at: 0,
        }
    }

    fn manager() -> (DownloadManager, std::path::PathBuf) {
        let directory = std::env::temp_dir().join(format!("mio-manager-{}", uuid::Uuid::new_v4()));
        (DownloadManager::new_for_test(&directory), directory)
    }

    #[test]
    fn restored_downloading_tasks_become_paused_and_queued_are_scheduled() {
        let mut tasks = vec![
            task("active", DownloadStatus::Downloading),
            task("queued", DownloadStatus::Queued),
        ];
        let queue = normalize_restored_tasks(&mut tasks);

        assert_eq!(DownloadStatus::Paused, tasks[0].status);
        assert_eq!(vec!["queued"], queue);
    }

    #[test]
    fn download_target_identity_folds_case_on_every_platform() {
        let directory = std::env::temp_dir().join(format!("mio-manager-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&directory).unwrap();
        let upper = directory.join("Song.MP3");
        let lower = directory.join("song.mp3");

        assert_eq!(
            download_target_identity(&upper.to_string_lossy()).unwrap(),
            download_target_identity(&lower.to_string_lossy()).unwrap()
        );

        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn download_target_identity_folds_unicode_final_sigma() {
        let directory = std::env::temp_dir().join(format!("mio-manager-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&directory).unwrap();

        assert_eq!(
            download_target_identity(&directory.join("σ.mp3").to_string_lossy()).unwrap(),
            download_target_identity(&directory.join("ς.mp3").to_string_lossy()).unwrap()
        );

        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn download_target_identity_normalizes_unicode_composition() {
        let directory = std::env::temp_dir().join(format!("mio-manager-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&directory).unwrap();

        assert_eq!(
            download_target_identity(&directory.join("é.mp3").to_string_lossy()).unwrap(),
            download_target_identity(&directory.join("e\u{301}.mp3").to_string_lossy()).unwrap()
        );

        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn download_target_identity_catches_ntfs_dotless_i_alias() {
        let upper = fold_path_case_for_identity(std::path::Path::new("I.mp3")).unwrap();
        let dotless = fold_path_case_for_identity(std::path::Path::new("ı.mp3")).unwrap();

        assert_eq!(upper, dotless);
        assert!(download_target_identities_match(&upper, &dotless).unwrap());
    }

    #[tokio::test]
    async fn load_tasks_uses_created_at_and_id_to_choose_the_queued_target_owner() {
        let (manager, directory) = manager();
        std::fs::create_dir_all(&directory).unwrap();
        let mut later = task("later", DownloadStatus::Queued);
        later.created_at = 2;
        later.file_path = directory.join("Song.mp3").to_string_lossy().into_owned();
        let mut first = task("first", DownloadStatus::Queued);
        first.created_at = 1;
        first.file_path = directory.join("song.mp3").to_string_lossy().into_owned();
        std::fs::write(
            manager.downloads_file(),
            serde_json::to_vec(&[later, first]).unwrap(),
        )
        .unwrap();

        manager.load_tasks().await;

        let tasks = manager.tasks.read().await;
        assert_eq!(2, tasks.len());
        assert_eq!(DownloadStatus::Queued, tasks["first"].status);
        assert_eq!(DownloadStatus::Error, tasks["later"].status);
        drop(tasks);
        assert_eq!(vec!["first"], *manager.queue.lock().await);
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[tokio::test]
    async fn load_tasks_never_queues_a_target_owned_by_any_existing_status() {
        for owner_status in [
            DownloadStatus::Completed,
            DownloadStatus::Paused,
            DownloadStatus::Downloading,
            DownloadStatus::Error,
            DownloadStatus::Cancelled,
        ] {
            let (manager, directory) = manager();
            std::fs::create_dir_all(&directory).unwrap();
            let mut queued = task("queued", DownloadStatus::Queued);
            queued.created_at = 0;
            queued.file_path = directory.join("Song.mp3").to_string_lossy().into_owned();
            let mut owner = task("owner", owner_status.clone());
            owner.created_at = 1;
            owner.file_path = directory.join("song.mp3").to_string_lossy().into_owned();
            std::fs::write(
                manager.downloads_file(),
                serde_json::to_vec(&[queued, owner]).unwrap(),
            )
            .unwrap();

            manager.load_tasks().await;

            let tasks = manager.tasks.read().await;
            assert_ne!(DownloadStatus::Queued, tasks["owner"].status);
            assert_eq!(DownloadStatus::Error, tasks["queued"].status);
            assert!(tasks["queued"]
                .error
                .as_deref()
                .is_some_and(|error| error.contains("共享临时文件")));
            drop(tasks);
            assert!(manager.queue.lock().await.is_empty());
            std::fs::remove_dir_all(directory).unwrap();
        }
    }

    #[tokio::test]
    async fn load_tasks_retains_duplicate_task_ids_as_visible_error_records() {
        let (manager, directory) = manager();
        std::fs::create_dir_all(&directory).unwrap();
        let mut original = task("duplicate-id", DownloadStatus::Queued);
        original.file_path = directory
            .join("original.mp3")
            .to_string_lossy()
            .into_owned();
        let mut duplicate = task("duplicate-id", DownloadStatus::Queued);
        duplicate.file_path = directory
            .join("duplicate.mp3")
            .to_string_lossy()
            .into_owned();
        std::fs::write(
            manager.downloads_file(),
            serde_json::to_vec(&[original, duplicate]).unwrap(),
        )
        .unwrap();

        manager.load_tasks().await;

        let tasks = manager.tasks.read().await;
        assert_eq!(2, tasks.len());
        assert_eq!(DownloadStatus::Queued, tasks["duplicate-id"].status);
        let renamed = tasks
            .values()
            .find(|task| task.id != "duplicate-id")
            .unwrap();
        assert_eq!(DownloadStatus::Error, renamed.status);
        assert!(renamed
            .error
            .as_deref()
            .is_some_and(|error| error.contains("重复任务 ID")));
        drop(tasks);
        assert_eq!(vec!["duplicate-id"], *manager.queue.lock().await);
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn download_target_identity_rejects_non_utf8_canonical_paths() {
        use std::ffi::OsString;
        use std::os::unix::ffi::OsStringExt;

        let path = std::path::PathBuf::from(OsString::from_vec(b"non-utf8-\xff".to_vec()));
        let error = fold_path_case_for_identity(&path).unwrap_err();

        assert!(error.contains("UTF-8"));
    }

    #[test]
    fn rejects_unsuccessful_download_response() {
        assert!(resolve_download_mode(StatusCode::NOT_FOUND, None, 0, Some(10)).is_err());
    }

    #[test]
    fn appends_only_matching_partial_content() {
        let range = HeaderValue::from_static("bytes 5-9/10");
        assert_eq!(
            DownloadMode::Append {
                downloaded: 5,
                total: 10
            },
            resolve_download_mode(StatusCode::PARTIAL_CONTENT, Some(&range), 5, Some(5)).unwrap()
        );
        let wrong = HeaderValue::from_static("bytes 0-4/10");
        assert!(
            resolve_download_mode(StatusCode::PARTIAL_CONTENT, Some(&wrong), 5, Some(5)).is_err()
        );
    }

    #[test]
    fn restarts_when_server_ignores_range() {
        assert_eq!(
            DownloadMode::Truncate { total: 10 },
            resolve_download_mode(StatusCode::OK, None, 5, Some(10)).unwrap()
        );
    }

    #[tokio::test]
    async fn stale_worker_cannot_commit_after_resume() {
        let state = TaskGeneration::default();
        let old = state.next();
        let current = state.next();

        assert!(!state.is_current(old));
        assert!(state.is_current(current));
    }

    #[test]
    fn atomic_write_replaces_the_persisted_snapshot() {
        let directory = std::env::temp_dir().join(format!("mio-manager-{}", uuid::Uuid::new_v4()));
        let path = directory.join("downloads.json");

        atomic_write(&path, b"first").unwrap();
        atomic_write(&path, b"second").unwrap();

        assert_eq!(b"second", std::fs::read(&path).unwrap().as_slice());
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[tokio::test]
    async fn concurrent_adds_with_equivalent_paths_create_only_one_task() {
        let (manager, directory) = manager();
        let final_path = directory.join("song.mp3");
        let equivalent_path = directory.join("subdir").join("..").join("song.mp3");
        let lifecycle = manager.lifecycle.clone().lock_owned().await;

        let first_manager = manager.clone();
        let first = tokio::spawn(async move {
            first_manager
                .add_task(
                    Value::Null,
                    "https://example.com/first".into(),
                    final_path.to_string_lossy().into_owned(),
                    None,
                    None,
                    0,
                )
                .await
        });
        let second_manager = manager.clone();
        let second = tokio::spawn(async move {
            second_manager
                .add_task(
                    Value::Null,
                    "https://example.com/second".into(),
                    equivalent_path.to_string_lossy().into_owned(),
                    None,
                    None,
                    0,
                )
                .await
        });

        for _ in 0..16 {
            tokio::task::yield_now().await;
        }
        assert!(
            manager.tasks.read().await.is_empty(),
            "add_task must reserve its path under the lifecycle lock"
        );
        drop(lifecycle);

        let first = first.await.unwrap();
        let second = second.await.unwrap();
        assert_eq!(1, usize::from(first.is_ok()) + usize::from(second.is_ok()));
        assert_eq!(1, manager.tasks.read().await.len());
        assert_eq!(1, manager.queue.lock().await.len());
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[tokio::test]
    async fn failed_and_cancelled_tasks_keep_their_target_reserved() {
        for status in [DownloadStatus::Error, DownloadStatus::Cancelled] {
            let (manager, directory) = manager();
            std::fs::create_dir_all(&directory).unwrap();
            let file_path = directory.join("song.mp3");
            let mut existing = task("existing", status);
            existing.file_path = file_path.to_string_lossy().into_owned();
            manager
                .tasks
                .write()
                .await
                .insert(existing.id.clone(), existing);

            let result = manager
                .add_task(
                    Value::Null,
                    "https://example.com/replacement".into(),
                    file_path.to_string_lossy().into_owned(),
                    None,
                    None,
                    0,
                )
                .await;

            assert!(result.is_err());
            assert_eq!(1, manager.tasks.read().await.len());
            assert!(manager.queue.lock().await.is_empty());
            std::fs::remove_dir_all(directory).unwrap();
        }
    }

    #[tokio::test]
    async fn retry_rejects_a_target_reserved_by_another_task() {
        let (manager, directory) = manager();
        std::fs::create_dir_all(&directory).unwrap();
        let file_path = directory.join("song.mp3");
        let mut retrying = task("retrying", DownloadStatus::Error);
        retrying.file_path = file_path.to_string_lossy().into_owned();
        let mut reserved = task("reserved", DownloadStatus::Cancelled);
        reserved.file_path = file_path.to_string_lossy().into_owned();
        {
            let mut tasks = manager.tasks.write().await;
            tasks.insert(retrying.id.clone(), retrying);
            tasks.insert(reserved.id.clone(), reserved);
        }

        let result = manager.retry_task("retrying").await;

        assert!(result.is_err());
        assert_eq!(
            DownloadStatus::Error,
            manager.tasks.read().await["retrying"].status
        );
        assert!(manager.queue.lock().await.is_empty());
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn symlink_parent_alias_cannot_reserve_the_same_target_twice() {
        use std::os::unix::fs::symlink;

        let (manager, directory) = manager();
        let real_parent = directory.join("real");
        let alias_parent = directory.join("alias");
        std::fs::create_dir_all(&real_parent).unwrap();
        symlink(&real_parent, &alias_parent).unwrap();

        manager
            .add_task(
                Value::Null,
                "https://example.com/first".into(),
                real_parent.join("song.mp3").to_string_lossy().into_owned(),
                None,
                None,
                0,
            )
            .await
            .unwrap();
        let result = manager
            .add_task(
                Value::Null,
                "https://example.com/alias".into(),
                alias_parent.join("song.mp3").to_string_lossy().into_owned(),
                None,
                None,
                0,
            )
            .await;

        assert!(result.is_err());
        assert_eq!(1, manager.tasks.read().await.len());
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[cfg(any(target_os = "windows", target_os = "macos"))]
    #[tokio::test]
    async fn case_alias_cannot_reserve_the_same_target_twice() {
        let (manager, directory) = manager();
        std::fs::create_dir_all(&directory).unwrap();

        manager
            .add_task(
                Value::Null,
                "https://example.com/first".into(),
                directory.join("Song.mp3").to_string_lossy().into_owned(),
                None,
                None,
                0,
            )
            .await
            .unwrap();
        let result = manager
            .add_task(
                Value::Null,
                "https://example.com/case-alias".into(),
                directory.join("song.mp3").to_string_lossy().into_owned(),
                None,
                None,
                0,
            )
            .await;

        assert!(result.is_err());
        assert_eq!(1, manager.tasks.read().await.len());
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[tokio::test]
    async fn clear_tasks_removes_selected_temp_files() {
        let (manager, directory) = manager();
        let mut failed = task("failed", DownloadStatus::Error);
        failed.file_path = directory.join("failed.mp3").to_string_lossy().into_owned();
        let temp_path = temporary_path_for_file_path(&failed.file_path);
        std::fs::create_dir_all(&directory).unwrap();
        std::fs::write(&temp_path, b"partial").unwrap();
        manager
            .tasks
            .write()
            .await
            .insert(failed.id.clone(), failed);

        manager.clear_tasks("failed").await;

        assert!(!temp_path.exists());
        assert!(manager.tasks.read().await.is_empty());
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[tokio::test]
    async fn completed_task_cannot_be_retried() {
        let (manager, directory) = manager();
        manager.tasks.write().await.insert(
            "completed".into(),
            task("completed", DownloadStatus::Completed),
        );

        let error = manager.retry_task("completed").await.unwrap_err();

        assert!(error.contains("已完成"));
        assert_eq!(
            DownloadStatus::Completed,
            manager.tasks.read().await["completed"].status
        );
        assert!(manager.queue.lock().await.is_empty());
        let _ = std::fs::remove_dir_all(directory);
    }

    #[tokio::test]
    async fn clear_blocks_retry_until_selected_tasks_are_removed() {
        let (manager, directory) = manager();
        {
            let mut tasks = manager.tasks.write().await;
            tasks.insert("failed-1".into(), task("failed-1", DownloadStatus::Error));
            tasks.insert("failed-2".into(), task("failed-2", DownloadStatus::Error));
        }
        let lifecycle = manager.lifecycle.clone().lock_owned().await;
        let clear_manager = manager.clone();
        let clear = tokio::spawn(async move { clear_manager.clear_tasks("failed").await });
        tokio::task::yield_now().await;
        let retry_manager_1 = manager.clone();
        let retry_1 = tokio::spawn(async move { retry_manager_1.retry_task("failed-1").await });
        let retry_manager_2 = manager.clone();
        let retry_2 = tokio::spawn(async move { retry_manager_2.retry_task("failed-2").await });

        drop(lifecycle);
        clear.await.unwrap();
        assert!(retry_1.await.unwrap().is_err());
        assert!(retry_2.await.unwrap().is_err());
        assert!(manager.tasks.read().await.is_empty());
        assert!(manager.queue.lock().await.is_empty());
        assert_eq!(
            "[]",
            std::fs::read_to_string(manager.downloads_file()).unwrap()
        );
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[tokio::test]
    async fn aborted_save_still_finishes_before_newer_snapshot_commits() {
        let (manager, directory) = manager();
        manager
            .tasks
            .write()
            .await
            .insert("first".into(), task("first", DownloadStatus::Paused));
        let writer = manager.persistence_writer.clone();
        let writer_guard = writer.lock().unwrap();
        let first_manager = manager.clone();
        let first_save = tokio::spawn(async move { first_manager.save_tasks().await });
        while manager.persistence.try_lock().is_ok() {
            tokio::task::yield_now().await;
        }
        first_save.abort();
        let _ = first_save.await;
        assert!(
            manager.persistence.try_lock().is_err(),
            "aborting the caller must not release an in-flight commit barrier"
        );
        manager.tasks.write().await.clear();
        manager
            .tasks
            .write()
            .await
            .insert("second".into(), task("second", DownloadStatus::Paused));
        drop(writer_guard);

        manager.save_tasks().await.unwrap();
        let saved = std::fs::read_to_string(manager.downloads_file()).unwrap();
        assert!(saved.contains("second"));
        assert!(!saved.contains("first"));
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[tokio::test]
    async fn retry_waits_for_old_worker_completion_without_exceeding_capacity() {
        let (manager, directory) = manager();
        *manager.max_concurrent.write().await = 2;
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let mut connections = Vec::new();
            loop {
                connections.push(listener.accept().await.unwrap().0);
            }
        });
        let mut retrying = task("retrying", DownloadStatus::Queued);
        retrying.url = format!("http://{address}");
        retrying.file_path = directory.join("retrying").to_string_lossy().into_owned();
        let mut other = task("other", DownloadStatus::Queued);
        other.url = format!("http://{address}");
        other.file_path = directory.join("other").to_string_lossy().into_owned();
        manager
            .tasks
            .write()
            .await
            .insert(retrying.id.clone(), retrying);
        manager.tasks.write().await.insert(other.id.clone(), other);
        manager
            .queue
            .lock()
            .await
            .extend(["retrying".into(), "other".into()]);

        let old_generation = manager.next_generation("retrying").await;
        let (release_old, released) = oneshot::channel();
        let (old_done, old_done_receiver) = oneshot::channel();
        let completion = WorkerCompletion::new(old_done, manager.active_workers.clone());
        let finish_manager = manager.clone();
        let old_worker = tokio::spawn(async move {
            let _ = released.await;
            finish_manager
                .finish_worker("retrying", old_generation, completion, true)
                .await;
        });
        manager.workers.lock().await.insert(
            "retrying".into(),
            WorkerHandle {
                abort: old_worker.abort_handle(),
                done: old_done_receiver,
                generation: old_generation,
            },
        );

        manager.spawn_next().await;

        let workers = manager.workers.lock().await;
        let replacement_started = workers
            .get("retrying")
            .is_some_and(|worker| worker.generation != old_generation);
        assert!(!replacement_started, "replacement started before old done");
        assert_eq!(
            old_generation,
            workers.get("retrying").unwrap().generation,
            "replacement overwrote the old WorkerHandle"
        );
        assert!(workers.contains_key("other"));
        assert_eq!(2, manager.active_workers.load(Ordering::SeqCst));
        drop(workers);

        let _ = release_old.send(());
        old_worker.await.unwrap();
        let workers = manager.workers.lock().await;
        assert_eq!(2, workers.len());
        let replacement = workers.get("retrying").unwrap();
        assert_ne!(old_generation, replacement.generation);
        assert_eq!(2, manager.active_workers.load(Ordering::SeqCst));
        drop(workers);

        let workers: Vec<WorkerHandle> = manager
            .workers
            .lock()
            .await
            .drain()
            .map(|(_, worker)| worker)
            .collect();
        for worker in workers {
            worker.abort.abort();
            let _ = worker.done.await;
        }
        assert_eq!(0, manager.active_workers.load(Ordering::SeqCst));
        server.abort();
        std::fs::remove_dir_all(directory).unwrap();
    }
}
