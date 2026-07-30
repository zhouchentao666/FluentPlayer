use std::sync::mpsc::channel;
use tauri::AppHandle;
use tauri_plugin_dialog::DialogExt;

use super::media::{AUDIO_EXTS, IMAGE_EXTS};

fn file_path_to_string(fp: tauri_plugin_dialog::FilePath) -> Option<String> {
    fp.into_path().ok().map(|p| p.to_string_lossy().to_string())
}

/// 选择音频文件（多选），取消返回空数组
#[tauri::command]
pub async fn open_music_files(app: AppHandle) -> Result<Vec<String>, String> {
    let (tx, rx) = channel();
    app.dialog()
        .file()
        .add_filter("音频文件", AUDIO_EXTS)
        .pick_files(move |files| {
            let _ = tx.send(files);
        });
    let files = rx.recv().map_err(|e| e.to_string())?;
    Ok(files
        .map(|files| files.into_iter().filter_map(file_path_to_string).collect())
        .unwrap_or_default())
}

/// 选择音乐文件夹，取消返回空字符串（文件夹选择仅桌面端可用）
#[cfg(desktop)]
#[tauri::command]
pub async fn open_music_folder(app: AppHandle) -> Result<String, String> {
    let (tx, rx) = channel();
    app.dialog().file().pick_folder(move |folder| {
        let _ = tx.send(folder);
    });
    let picked = rx.recv().map_err(|e| e.to_string())?;
    Ok(picked.and_then(file_path_to_string).unwrap_or_default())
}

#[cfg(mobile)]
#[tauri::command]
pub async fn open_music_folder(_app: AppHandle) -> Result<String, String> {
    Ok(String::new())
}

/// 选择图片文件，取消返回空字符串
#[tauri::command]
pub async fn open_image_file(app: AppHandle) -> Result<String, String> {
    let (tx, rx) = channel();
    app.dialog()
        .file()
        .add_filter("图片文件", IMAGE_EXTS)
        .pick_file(move |file| {
            let _ = tx.send(file);
        });
    let picked = rx.recv().map_err(|e| e.to_string())?;
    Ok(picked.and_then(file_path_to_string).unwrap_or_default())
}
