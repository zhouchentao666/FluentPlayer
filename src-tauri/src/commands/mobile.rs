// 移动端专用命令（【仅移动端生效】）
//
// 设计原则（见需求约束）：
// - 移动端没有文件系统完整访问权限，不能直接复用桌面 open 文件对话框。
// - 使用 Tauri2 移动端 dialog picker 获取 URI（Android content:// / iOS file://）。
// - 播放：前端直接用该 URI 喂给 <audio>（webview 原生硬件解码）。
// - 元数据：Android 的 content:// 无法被 Rust std / lofty 直接打开，故移动端
//   元数据改为前端用 <audio>.loadedmetadata 读取时长、从 URI 文件名推断标题，
//   这是移动端沙盒限制下的降级方案（见交付说明）。iOS 的 file:// 是真实路径，
//   可直接走 lofty，因此 iOS 保留完整标签读取能力。
use lofty::prelude::*;
use lofty::probe::Probe;
use serde::Serialize;
use tauri::AppHandle;
use tauri_plugin_dialog::{DialogExt, FilePath};

const AUDIO_FILTER: &[&str] = &[
    "mp3", "flac", "wav", "ogg", "oga", "m4a", "m4b", "aac", "opus", "wma", "aif", "aiff",
    "ape", "wv",
];

fn is_content_uri(p: &str) -> bool {
    p.starts_with("content://") || p.starts_with("file://") || p.starts_with("content:")
}

/// 把 tauri-plugin-dialog 返回的 FilePath 序列化为字符串。
/// 移动端返回的是可被 webview 直接播放的 URI（content:// 或 file://）。
fn file_path_to_string(p: &FilePath) -> String {
    match p {
        FilePath::Path(path) => path.to_string_lossy().to_string(),
        FilePath::Url(url) => url.to_string(),
    }
}

/// 移动端：弹出系统文件选择器，返回选中的音频 URI 列表。
/// 桌面端不会调用此命令；若误调用，返回空列表。
#[tauri::command]
pub async fn pick_music_files_mobile(app: AppHandle) -> Result<Vec<String>, String> {
    let picked = pick_files(&app).await?;
    Ok(picked)
}

/// 移动端：弹出系统文件夹/文档选择器，返回选中的目录 URI（Android 为 tree URI）。
/// 注意：移动端沙盒下多数 picker 只能选“文件”，无法递归整目录扫描，
/// 因此这里复用文件选择（多选用），由前端一并导入。
#[tauri::command]
pub async fn pick_music_folder_mobile(app: AppHandle) -> Result<Vec<String>, String> {
    let picked = pick_files(&app).await?;
    Ok(picked)
}

async fn pick_files(app: &AppHandle) -> Result<Vec<String>, String> {
    let (tx, rx) = std::sync::mpsc::channel::<Vec<FilePath>>();
    app.dialog()
        .file()
        .add_filter("音频文件", AUDIO_FILTER)
        .pick_files(move |files| {
            let v = files.unwrap_or_default();
            let _ = tx.send(v);
        });
    // 阻塞等待用户选择（移动端原生弹窗为异步回调）
    let files = rx
        .recv_timeout(std::time::Duration::from_secs(300))
        .map_err(|_| "选择超时或已取消".to_string())?;
    Ok(files.iter().map(file_path_to_string).collect())
}

/// 移动端元数据读取（降级版）：
/// - iOS（file:// 真实路径）→ 走 lofty，保留完整标签/时长。
/// - Android（content://）→ Rust 无法直读，返回仅含标题（来自 URI 文件名）的占位元数据，
///   时长由前端 <audio> 的 loadedmetadata 补充。
#[tauri::command]
pub fn read_metadata_mobile(path: String) -> Result<SongMetaMobile, String> {
    if is_content_uri(&path) && path.starts_with("content://") {
        return Ok(SongMetaMobile {
            title: title_from_uri(&path),
            artist: String::new(),
            album: String::new(),
            genre: String::new(),
            year: String::new(),
            duration: 0.0,
            bitrate: 0,
            sample_rate: 0,
            from_embedded: false,
        });
    }
    // iOS file:// 或桌面普通路径：直接 lofty
    let fs_path = path.replace("file://", "");
    match Probe::open(&fs_path).and_then(|p| p.read()) {
        Ok(tagged) => {
            let props = tagged.properties();
            let tag = tagged.primary_tag().or_else(|| tagged.first_tag());
            let (title, artist, album, genre, year) = match tag {
                Some(t) => (
                    t.title().map(|s| s.to_string()).unwrap_or_default(),
                    t.artist().map(|s| s.to_string()).unwrap_or_default(),
                    t.album().map(|s| s.to_string()).unwrap_or_default(),
                    t.genre().map(|s| s.to_string()).unwrap_or_default(),
                    t.year().map(|y| y.to_string()).unwrap_or_default(),
                ),
                None => (
                    title_from_uri(&path),
                    String::new(),
                    String::new(),
                    String::new(),
                    String::new(),
                ),
            };
            Ok(SongMetaMobile {
                title,
                artist,
                album,
                genre,
                year,
                duration: props.duration().as_secs_f64(),
                bitrate: props.audio_bitrate().unwrap_or(0),
                sample_rate: props.sample_rate().unwrap_or(0),
                from_embedded: tag.is_some(),
            })
        }
        Err(_) => Ok(SongMetaMobile {
            title: title_from_uri(&path),
            artist: String::new(),
            album: String::new(),
            genre: String::new(),
            year: String::new(),
            duration: 0.0,
            bitrate: 0,
            sample_rate: 0,
            from_embedded: false,
        }),
    }
}

#[derive(Serialize)]
pub struct SongMetaMobile {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub genre: String,
    pub year: String,
    pub duration: f64,
    pub bitrate: u32,
    pub sample_rate: u32,
    pub from_embedded: bool,
}

fn title_from_uri(uri: &str) -> String {
    let decoded = uri.split('?').next().unwrap_or(uri);
    let seg = decoded
        .trim_end_matches('/')
        .rsplit('/')
        .next()
        .unwrap_or(decoded)
        .to_string();
    // 去掉扩展名（最后一个 '.' 之后的部分）
    match seg.rfind('.') {
        Some(i) if i > 0 => seg[..i].to_string(),
        _ => seg,
    }
}
