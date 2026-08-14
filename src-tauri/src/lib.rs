mod commands;

#[cfg(desktop)]
use tauri::Manager;

pub fn run() {
    let mut builder = tauri::Builder::default();

    // 桌面专属插件（single-instance / autostart 不支持移动端）
    #[cfg(desktop)]
    {
        builder = builder.plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            // 第二实例启动时聚焦主窗口
            if let Some(win) = app.get_webview_window("main") {
                let _ = win.show();
                let _ = win.unminimize();
                let _ = win.set_focus();
            }
        }));
        builder = builder.plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ));
    }

    builder
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_opener::init())
        .manage(commands::watcher::WatcherState::default())
        // 托盘状态仅在桌面使用
        #[cfg(desktop)]
        .manage(commands::tray::TrayState::default())
        .invoke_handler(tauri::generate_handler![
            // 对话框
            commands::dialogs::open_music_files,
            commands::dialogs::open_music_folder,
            commands::dialogs::open_image_file,
            // 离线文件下载
            commands::download::download_file,
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
            // 系统（跨平台）
            commands::system::version,
            commands::system::emit_metadata_changed,
            commands::system::default_music_folder,
            // 文件夹监听
            commands::watcher::watch_music_folder,
            // ---- 以下命令仅桌面端可用 ----
            #[cfg(desktop)]
            commands::system::open_in_explorer,
            #[cfg(desktop)]
            commands::system::quit_app,
            #[cfg(desktop)]
            commands::system::show_main_window,
            #[cfg(desktop)]
            commands::system::apply_auto_start,
            #[cfg(desktop)]
            commands::system::open_song_editor,
            #[cfg(desktop)]
            commands::system::set_close_to_tray,
            #[cfg(desktop)]
            commands::download::save_file,
            #[cfg(desktop)]
            commands::tray::enable_tray,
            #[cfg(desktop)]
            commands::tray::set_tray_song_info,
            #[cfg(desktop)]
            commands::lyric_window::toggle_desktop_lyric,
            #[cfg(desktop)]
            commands::lyric_window::close_desktop_lyric,
            #[cfg(desktop)]
            commands::lyric_window::set_desktop_lyric_bounds,
            #[cfg(desktop)]
            commands::lyric_window::set_desktop_lyric_ignore_mouse_events,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
