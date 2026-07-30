mod commands;

#[cfg(desktop)]
use tauri::Manager;

pub fn run() {
    let mut builder = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(commands::watcher::WatcherState::default());

    // 桌面专属插件与状态：移动端（android/iOS）不编译、不注册
    #[cfg(desktop)]
    {
        builder = builder
            .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
                // 第二实例启动时聚焦主窗口
                if let Some(win) = app.get_webview_window("main") {
                    let _ = win.show();
                    let _ = win.unminimize();
                    let _ = win.set_focus();
                }
            }))
            .plugin(tauri_plugin_autostart::init(
                tauri_plugin_autostart::MacosLauncher::LaunchAgent,
                None,
            ))
            .manage(commands::tray::TrayState::default());
    }

    // invoke_handler 只能注册一次：桌面端包含全部命令，移动端仅通用命令
    let invoke_handler = {
        #[cfg(desktop)]
        {
            tauri::generate_handler![
                // 对话框（跨平台，使用异步 API）
                commands::dialogs::open_music_files,
                commands::dialogs::open_music_folder,
                commands::dialogs::open_image_file,
                // 本地媒体
                commands::media::scan_music_folder,
                commands::media::read_metadata,
                commands::media::read_cover_art,
                commands::media::read_lyrics,
                commands::media::read_image_file,
                // 配置
                commands::config::save_config,
                commands::config::load_config,
                commands::config::get_desktop_lyric_config,
                // 系统（跨平台部分）
                commands::system::open_in_explorer,
                commands::system::version,
                commands::system::quit_app,
                commands::system::emit_metadata_changed,
                // 文件夹监听
                commands::watcher::watch_music_folder,
                // 系统（桌面专属）
                commands::system::show_main_window,
                commands::system::apply_auto_start,
                commands::system::open_song_editor,
                commands::system::set_close_to_tray,
                // 托盘
                commands::tray::enable_tray,
                commands::tray::set_tray_song_info,
                // 桌面歌词窗口
                commands::lyric_window::toggle_desktop_lyric,
                commands::lyric_window::close_desktop_lyric,
                commands::lyric_window::set_desktop_lyric_bounds,
                commands::lyric_window::set_desktop_lyric_ignore_mouse_events,
            ]
        }
        #[cfg(not(desktop))]
        {
            tauri::generate_handler![
                // 对话框（跨平台，使用异步 API）
                commands::dialogs::open_music_files,
                commands::dialogs::open_music_folder,
                commands::dialogs::open_image_file,
                // 本地媒体
                commands::media::scan_music_folder,
                commands::media::read_metadata,
                commands::media::read_cover_art,
                commands::media::read_lyrics,
                commands::media::read_image_file,
                // 配置
                commands::config::save_config,
                commands::config::load_config,
                commands::config::get_desktop_lyric_config,
                // 系统（跨平台部分）
                commands::system::open_in_explorer,
                commands::system::version,
                commands::system::quit_app,
                commands::system::emit_metadata_changed,
                // 文件夹监听
                commands::watcher::watch_music_folder,
            ]
        }
    };

    builder
        .invoke_handler(invoke_handler)
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
