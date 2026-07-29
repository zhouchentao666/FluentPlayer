use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::HashMap;
use std::path::Path;
use std::sync::{mpsc, Mutex};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, State};

#[derive(Default)]
pub struct WatcherState(pub Mutex<HashMap<String, RecommendedWatcher>>);

/// 监听音乐文件夹变化，去抖后向前端广播 `folder:changed`（payload 为文件夹路径）
#[tauri::command]
pub fn watch_music_folder(
    app: AppHandle,
    state: State<'_, WatcherState>,
    folder: String,
) -> Result<(), String> {
    let mut map = state.0.lock().map_err(|e| e.to_string())?;
    if map.contains_key(&folder) {
        return Ok(());
    }

    let (tx, rx) = mpsc::channel::<notify::Result<notify::Event>>();
    let mut watcher =
        notify::recommended_watcher(move |res| {
            let _ = tx.send(res);
        })
        .map_err(|e| format!("创建监听失败: {e}"))?;
    watcher
        .watch(Path::new(&folder), RecursiveMode::Recursive)
        .map_err(|e| format!("监听文件夹失败: {e}"))?;

    let app_handle = app.clone();
    let folder_key = folder.clone();
    std::thread::spawn(move || {
        while rx.recv().is_ok() {
            // 去抖：600ms 内的连续事件合并为一次通知
            let deadline = Instant::now() + Duration::from_millis(600);
            loop {
                let remain = deadline.saturating_duration_since(Instant::now());
                if remain.is_zero() {
                    break;
                }
                if rx.recv_timeout(remain).is_err() {
                    break;
                }
            }
            let _ = app_handle.emit("folder:changed", folder_key.clone());
        }
    });

    map.insert(folder, watcher);
    Ok(())
}
