use tauri::AppHandle;
use tauri_plugin_dialog::DialogExt;

use super::media::{AUDIO_EXTS, IMAGE_EXTS};

fn file_path_to_string(fp: tauri_plugin_dialog::FilePath) -> Option<String> {
    fp.into_path().ok().map(|p| p.to_string_lossy().to_string())
}

/// 选择音频文件（多选），取消返回空数组
#[tauri::command]
pub async fn open_music_files(app: AppHandle) -> Result<Vec<String>, String> {
    let picked = app
        .dialog()
        .file()
        .add_filter("音频文件", AUDIO_EXTS)
        .blocking_pick_files();
    Ok(picked
        .map(|files| files.into_iter().filter_map(file_path_to_string).collect())
        .unwrap_or_default())
}

/// 选择音乐文件夹，取消返回空字符串
#[tauri::command]
pub async fn open_music_folder(app: AppHandle) -> Result<String, String> {
    let picked = app.dialog().file().blocking_pick_folder();
    Ok(picked.and_then(file_path_to_string).unwrap_or_default())
}

/// 选择图片文件，取消返回空字符串
#[tauri::command]
pub async fn open_image_file(app: AppHandle) -> Result<String, String> {
    let picked = app
        .dialog()
        .file()
        .add_filter("图片文件", IMAGE_EXTS)
        .blocking_pick_file();
    Ok(picked.and_then(file_path_to_string).unwrap_or_default())
}

/// 获取系统默认音乐文件夹路径（不弹窗）。失败时回退到用户目录下的 Music 或当前目录。
#[tauri::command]
pub async fn get_default_music_folder() -> Result<String, String> {
    let mut candidates: Vec<std::path::PathBuf> = Vec::new();
    if let Ok(profile) = std::env::var("USERPROFILE") {
        candidates.push(std::path::Path::new(&profile).join("Music"));
    }
    if let Ok(home) = std::env::var("HOME") {
        candidates.push(std::path::Path::new(&home).join("Music"));
    }
    for path in candidates {
        if path.is_dir() {
            return Ok(path.to_string_lossy().to_string());
        }
    }
    // 回退：取第一个候选路径（即便尚未创建也返回，调用方负责创建）
    if let Some(first) = candidates.into_iter().next() {
        return Ok(first.to_string_lossy().to_string());
    }
    Ok("".to_string())
}
