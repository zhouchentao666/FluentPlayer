mod audio_capture;
mod audio_device;
mod commands;
mod db;
mod dlna;
mod download;
mod local_music;
mod music_sdk;
mod network;
mod player;
mod plugin;

#[cfg(target_os = "android")]
static ANDROID_APP_HANDLE: std::sync::OnceLock<tauri::AppHandle> = std::sync::OnceLock::new();

#[cfg(target_os = "android")]
static MUSIC_SERVICE_CLASS: std::sync::OnceLock<jni::objects::GlobalRef> =
    std::sync::OnceLock::new();

use commands::hotkey_commands;
use db::AppDb;
use download::manager::DownloadManager;
#[allow(unused_imports)]
use player::SharedPlayer;
use plugin::manager::PluginManager;
use std::borrow::Cow;
use std::sync::Mutex;
use tauri::http::{header, StatusCode};
#[cfg(desktop)]
use tauri::Emitter;
#[cfg(target_os = "macos")]
use tauri::TitleBarStyle;
use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    std::panic::set_hook(Box::new(|info| {
        eprintln!("[PANIC] {}", info);
    }));

    // Android: install rustls crypto provider before WebView sends any request
    #[cfg(target_os = "android")]
    {
        let _ = rustls::crypto::ring::default_provider().install_default();
    }

    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_process::init());

    #[cfg(desktop)]
    let builder = builder.plugin(tauri_plugin_global_shortcut::Builder::new().build());

    builder
        .manage(Mutex::new(commands::power_save::power_save_blocker_state()))
        .manage(commands::DesktopLyricState {
            is_open: Mutex::new(false),
            is_locked: Mutex::new(false),
        })
        .manage(dlna::DlnaManager::new())
        .register_asynchronous_uri_scheme_protocol("imgproxy", |_ctx, request, responder| {
            let uri = request.uri().to_string();
            let path = request.uri().path().to_string();

            let original_url = if path.starts_with('/') {
                path[1..].to_string()
            } else {
                path
            };

            let original_url = urlencoding::decode(&original_url)
                .unwrap_or_else(|_| std::borrow::Cow::Borrowed(&original_url))
                .into_owned();

            if !original_url.starts_with("http://") && !original_url.starts_with("https://") {
                responder.respond(
                    tauri::http::Response::builder()
                        .status(StatusCode::BAD_REQUEST)
                        .body(Cow::Owned(format!("invalid url: {uri}").into_bytes()))
                        .unwrap(),
                );
                return;
            }

            tauri::async_runtime::spawn(async move {
                match crate::network::RestrictedHttpClient::new(
                    crate::network::HttpPolicy::public_proxy(),
                )
                .fetch_bytes(&original_url, 20 * 1024 * 1024, &["image/*"])
                .await
                {
                    Ok(response) => {
                        let content_type = response
                            .headers
                            .get(header::CONTENT_TYPE)
                            .and_then(|v| v.to_str().ok())
                            .unwrap_or("image/jpeg")
                            .to_string();
                        responder.respond(
                            tauri::http::Response::builder()
                                .status(response.status)
                                .header(header::CONTENT_TYPE, &content_type)
                                .header(header::CACHE_CONTROL, "public, max-age=86400")
                                .header("Access-Control-Allow-Origin", "*")
                                .body(Cow::Owned(response.bytes))
                                .unwrap(),
                        );
                    }
                    Err(error) => responder.respond(
                        tauri::http::Response::builder()
                            .status(StatusCode::BAD_GATEWAY)
                            .body(Cow::Owned(format!("fetch error: {error}").into_bytes()))
                            .unwrap(),
                    ),
                }
            });
        })
        .setup(|app| {
            let app_data_dir = match app.path().app_data_dir() {
                Ok(dir) => dir,
                Err(e) => {
                    eprintln!("获取应用数据目录失败: {e}");
                    return Err(Box::new(std::io::Error::new(
                        std::io::ErrorKind::NotFound,
                        format!("获取应用数据目录失败: {e}"),
                    )) as Box<dyn std::error::Error>);
                }
            };
            db::set_app_data_dir(app_data_dir.clone());

            let app_db = match AppDb::new(&app_data_dir) {
                Ok(db) => db,
                Err(e) => {
                    eprintln!("数据库初始化失败: {e}");
                    return Err(Box::new(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!("数据库初始化失败: {e}"),
                    )) as Box<dyn std::error::Error>);
                }
            };
            app.manage(app_db);

            let plugin_manager = PluginManager::new(&app_data_dir);
            app.manage(plugin_manager);

            let win_builder = WebviewWindowBuilder::new(app, "main", WebviewUrl::default())
                .title("音乐")
                .inner_size(1200.0, 800.0);

            #[cfg(desktop)]
            let win_builder = win_builder.center();

            #[cfg(target_os = "macos")]
            let win_builder = win_builder.title_bar_style(TitleBarStyle::Overlay);

            #[cfg(target_os = "windows")]
            let win_builder = win_builder.decorations(false);

            #[cfg(target_os = "linux")]
            let win_builder = win_builder.decorations(false);

            let _window = match win_builder.build() {
                Ok(w) => w,
                Err(e) => {
                    eprintln!("创建窗口失败: {e}");
                    return Err(Box::new(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!("创建窗口失败: {e}"),
                    )) as Box<dyn std::error::Error>);
                }
            };

            #[cfg(target_os = "macos")]
            {
                use cocoa::appkit::{NSColor, NSWindow, NSWindowButton, NSWindowTitleVisibility};
                use cocoa::base::{id, nil, YES};
                use objc::runtime::Object;
                use objc::{msg_send, sel, sel_impl};

                unsafe fn hide_button(button: id) {
                    let _: () = msg_send![button as *mut Object, setHidden: YES];
                }

                let ns_window = _window.ns_window().expect("Failed to get NSWindow") as id;
                unsafe {
                    let bg_color = NSColor::colorWithRed_green_blue_alpha_(
                        nil,
                        50.0 / 255.0,
                        158.0 / 255.0,
                        163.5 / 255.0,
                        0.45,
                    );
                    ns_window.setBackgroundColor_(bg_color);
                    ns_window.setTitleVisibility_(NSWindowTitleVisibility::NSWindowTitleHidden);
                    ns_window.setTitlebarAppearsTransparent_(YES);

                    for button in [
                        NSWindowButton::NSWindowCloseButton,
                        NSWindowButton::NSWindowMiniaturizeButton,
                        NSWindowButton::NSWindowZoomButton,
                    ] {
                        let btn = ns_window.standardWindowButton_(button);
                        if btn != nil {
                            hide_button(btn);
                        }
                    }
                }
            }

            let app_handle = app.handle().clone();
            #[cfg(target_os = "android")]
            let _ = ANDROID_APP_HANDLE.set(app_handle.clone());
            let download_manager = DownloadManager::new(&app_data_dir, app_handle.clone());
            let startup_download_manager = download_manager.clone();
            app.manage(download_manager);
            tauri::async_runtime::spawn(async move {
                startup_download_manager.load_tasks().await;
                startup_download_manager.spawn_next().await;
            });

            // Start audio device change listener
            audio_device::start_device_listener(app_handle.clone());

            // Initialize native player engine (GStreamer)
            let shared_player = player::init_player(app_handle.clone());
            app.manage(shared_player);

            // Store AppHandle for hotkey re-registration
            hotkey_commands::set_app_handle(app_handle.clone());

            // Register OS-level shortcuts from saved config
            #[cfg(desktop)]
            {
                let db_ref = app_handle.state::<AppDb>();
                let conn = db_ref.playlist.lock().expect("DB lock poisoned");
                let config = hotkey_commands::load_config_from_db(&conn);
                drop(conn);
                hotkey_commands::re_register_shortcuts(&app_handle, &config);
            }

            // Auto-open desktop lyric if it was open on last exit
            #[cfg(desktop)]
            {
                let state_path = db::get_app_data_dir().join("desktop_lyric_window.json");
                let should_open = std::fs::read_to_string(&state_path)
                    .ok()
                    .and_then(|data| serde_json::from_str::<serde_json::Value>(&data).ok())
                    .and_then(|val| val.get("is_open").and_then(|v| v.as_bool()))
                    .unwrap_or(false);

                if should_open {
                    if commands::create_desktop_lyric_window(&app_handle).is_ok() {
                        let desktop_state = app_handle.state::<commands::DesktopLyricState>();
                        if let Ok(mut is_open) = desktop_state.is_open.lock() {
                            *is_open = true;
                        };
                    }
                }
            }

            // Setup system tray for desktop lyric unlock
            #[cfg(desktop)]
            {
                use tauri::menu::{Menu, MenuItem};
                use tauri::tray::TrayIconBuilder;

                let unlock_item = MenuItem::with_id(app, "unlock", "解锁歌词", true, None::<&str>)?;
                let quit_item = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
                let tray_menu = Menu::with_items(app, &[&unlock_item, &quit_item])?;

                let tray = TrayIconBuilder::new()
                    .icon(app.default_window_icon().unwrap().clone())
                    .menu(&tray_menu)
                    .tooltip("Mio Music")
                    .on_menu_event(|app_handle, event| match event.id().as_ref() {
                        "unlock" => {
                            if let Some(window) = app_handle.get_webview_window("desktop-lyric") {
                                let _ = window.set_ignore_cursor_events(false);
                            }
                            let state = app_handle.state::<commands::DesktopLyricState>();
                            if let Ok(mut locked) = state.is_locked.lock() {
                                *locked = false;
                            }
                            let _ = app_handle.emit("desktop-lyric-force-unlock", true);
                        }
                        "quit" => {
                            app_handle.cleanup_before_exit();
                            std::process::exit(0);
                        }
                        _ => {}
                    })
                    .on_tray_icon_event(|tray, event| {
                        if let tauri::tray::TrayIconEvent::Click {
                            button: tauri::tray::MouseButton::Left,
                            button_state: tauri::tray::MouseButtonState::Up,
                            ..
                        } = event
                        {
                            let app_handle = tray.app_handle();
                            if let Some(window) = app_handle.get_webview_window("main") {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                    })
                    .build(app)?;
                std::mem::forget(tray);
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Track commands
            commands::music_commands::track__get_all,
            commands::music_commands::track__get_by_id,
            commands::music_commands::track__get_by_path,
            commands::music_commands::track__upsert,
            commands::music_commands::track__upsert_batch,
            commands::music_commands::track__delete_by_path,
            commands::music_commands::track__delete_by_songmid,
            commands::music_commands::track__clear,
            commands::music_commands::track__get_stat,
            commands::music_commands::track__get_all_stats,
            commands::music_commands::track__prune_outside,
            // Dir commands
            commands::music_commands::dir__get_all,
            commands::music_commands::dir__set,
            // Songlist (playlist) commands
            commands::playlist_commands::songlist__get_all,
            commands::playlist_commands::songlist__get,
            commands::playlist_commands::songlist__create,
            commands::playlist_commands::songlist__delete,
            commands::playlist_commands::songlist__update,
            commands::playlist_commands::songlist__update_cover,
            commands::playlist_commands::songlist__search,
            commands::playlist_commands::songlist__exists,
            // Songlist song commands
            commands::playlist_commands::songlist__list_songs,
            commands::playlist_commands::songlist__count_songs,
            commands::playlist_commands::songlist__has_song,
            commands::playlist_commands::songlist__add_songs,
            commands::playlist_commands::songlist__add_head,
            commands::playlist_commands::songlist__remove_song,
            commands::playlist_commands::songlist__remove_batch,
            commands::playlist_commands::songlist__clear_songs,
            commands::playlist_commands::songlist__search_songs,
            // Batch delete & reorder
            commands::playlist_commands::songlist__batch_delete,
            commands::playlist_commands::songlist__move_song,
            // Favorites
            commands::playlist_commands::songlist__get_favorites_id,
            commands::playlist_commands::songlist__set_favorites_id,
            // Config (KV store)
            commands::config_commands::config__get,
            commands::config_commands::config__set,
            commands::config_commands::config__get_all,
            commands::config_commands::config__delete,
            // Hotkeys
            hotkey_commands::hotkeys__get,
            hotkey_commands::hotkeys__set,
            // Music SDK
            music_sdk::commands::service_music_sdk_request,
            music_sdk::commands::service_music_tip_search,
            music_sdk::commands::service_music_search_music,
            music_sdk::commands::service_music_find_music,
            // Local Music
            local_music::commands::local_music__scan,
            local_music::commands::local_music__get_cover,
            local_music::commands::local_music__get_covers,
            local_music::commands::local_music__write_tags,
            local_music::commands::local_music__get_tags,
            local_music::commands::local_music__get_lyric,
            local_music::commands::local_music__clear_index,
            local_music::commands::local_music__select_dirs,
            // Download Manager
            download::commands::download__add_task,
            download::commands::download__get_tasks,
            download::commands::download__pause_task,
            download::commands::download__resume_task,
            download::commands::download__cancel_task,
            download::commands::download__delete_task,
            download::commands::download__retry_task,
            download::commands::download__pause_all_tasks,
            download::commands::download__resume_all_tasks,
            download::commands::download__set_max_concurrent,
            download::commands::download__get_max_concurrent,
            download::commands::download__clear_tasks,
            download::commands::download__validate_files,
            download::commands::download__open_file_location,
            // Plugin System
            plugin::commands::plugin__initialize,
            plugin::commands::plugin__get_list,
            plugin::commands::plugin__add,
            plugin::commands::plugin__uninstall,
            plugin::commands::plugin__get_info,
            plugin::commands::plugin__call_method,
            plugin::commands::plugin__download_and_add,
            plugin::commands::plugin__get_type,
            plugin::commands::plugin__get_log,
            plugin::commands::plugin__get_config_schema,
            plugin::commands::plugin__get_config,
            plugin::commands::plugin__save_config,
            plugin::commands::plugin__test_connection,
            plugin::commands::plugin__select_and_add,
            plugin::commands::plugin__get_code,
            // Directory Settings
            commands::directory_commands::get_directories,
            commands::directory_commands::save_directories,
            commands::directory_commands::reset_directories,
            commands::directory_commands::get_directory_size,
            commands::directory_commands::open_directory,
            commands::directory_commands::get_cache_info,
            commands::directory_commands::clear_cache,
            // Audio Device
            audio_device::audio__enumerate_devices,
            // Audio Capture (microphone)
            audio_capture::audio_capture_start,
            audio_capture::audio_capture_stop,
            audio_capture::request_mic_permission,
            audio_device::audio__set_output_device,
            audio_device::audio__get_device_volume,
            audio_device::audio__set_device_volume,
            // HTTP Proxy (bypass CORS for plugins)
            commands::http_proxy,
            // Audio Proxy (fetch remote audio → data URI, bypasses CORS for <audio>)
            commands::audio_proxy,
            // Performance telemetry
            commands::performance__memory,
            // Desktop lyric window
            commands::change_desktop_lyric,
            commands::toogle_desktop_lyric_lock,
            commands::get_lyric_open_state,
            commands::get_lyric_lock_state,
            commands::get_font_list,
            commands::get_desktop_lyric_option,
            commands::set_desktop_lyric_option,
            // Power Save Blocker
            commands::power_save::power_save_blocker__start,
            commands::power_save::power_save_blocker__stop,
            // Native Player (rodio + cpal)
            player::commands::player__play,
            player::commands::player__pause,
            player::commands::player__resume,
            player::commands::player__stop,
            player::commands::player__seek,
            player::commands::player__set_volume,
            player::commands::player__get_state,
            player::commands::player__crossfade,
            player::commands::player__swap_slot,
            player::commands::player__set_eq_settings,
            player::commands::player__set_eq_band,
            player::commands::player__get_eq_bands,
            player::commands::player__get_eq_global_gain,
            player::commands::player__set_balance,
            player::commands::player__update_now_playing,
            player::commands::player__shutdown,
            player::commands::player__preload,
            player::commands::player__gapless_swap,
            player::commands::player__clear_secondary,
            player::commands::player__set_seamless_config,
            player::commands::player__set_cache_config,
            player::commands::player__set_native_queue,
            // DLNA / Screen casting
            dlna::commands::dlna__start_search,
            dlna::commands::dlna__stop_search,
            dlna::commands::dlna__get_devices,
            dlna::commands::dlna__play,
            dlna::commands::dlna__pause,
            dlna::commands::dlna__resume,
            dlna::commands::dlna__stop,
            dlna::commands::dlna__seek,
            dlna::commands::dlna__set_volume,
            dlna::commands::dlna__get_position,
            // S3 Backup
            commands::s3_commands::s3__test_connection,
            commands::s3_commands::s3__backup,
            commands::s3_commands::s3__restore,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|_app_handle, _event| {
            #[cfg(target_os = "macos")]
            if let tauri::RunEvent::Reopen { .. } = _event {
                if let Some(window) = _app_handle.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
        });
}

#[cfg(target_os = "android")]
#[no_mangle]
pub extern "system" fn Java_com_vant_Mio_Music_MainActivity_initAndroidContext(
    mut env: jni::JNIEnv,
    _this: jni::objects::JObject,
    activity: jni::objects::JObject,
) {
    use std::os::raw::c_void;

    let vm = match env.get_java_vm() {
        Ok(vm) => vm,
        Err(e) => {
            eprintln!("[Android] get_java_vm failed: {e}");
            return;
        }
    };
    let global_activity = match env.new_global_ref(&activity) {
        Ok(activity) => activity,
        Err(e) => {
            eprintln!("[Android] new_global_ref(activity) failed: {e}");
            return;
        }
    };

    let vm_ptr = vm.get_java_vm_pointer() as *mut c_void;
    let activity_ptr = global_activity.as_obj().as_raw() as *mut c_void;
    std::mem::forget(global_activity);

    // Cache MusicService class from main thread (JNI FindClass uses app ClassLoader here)
    if let Ok(class) = env.find_class("com/vant/Mio/Music/MusicService") {
        if let Ok(global_ref) = env.new_global_ref(&class) {
            let _ = MUSIC_SERVICE_CLASS.set(global_ref);
        }
    }

    unsafe {
        ndk_context::initialize_android_context(vm_ptr, activity_ptr);
    }
    eprintln!("[Android] ndk-context initialized from MainActivity");
}

#[cfg(target_os = "android")]
#[no_mangle]
pub extern "system" fn Java_com_vant_Mio_Music_MusicService_nativePlayerCommand(
    mut _env: jni::JNIEnv,
    _this: jni::objects::JObject,
    cmd: jni::objects::JString,
) {
    let cmd_str: String = match _env.get_string(&cmd) {
        Ok(s) => s.into(),
        Err(e) => {
            eprintln!("[Android] nativePlayerCommand get_string failed: {e}");
            return;
        }
    };

    // Directly control the player (more reliable than event→JS→invoke chain)
    if let Some(app_handle) = ANDROID_APP_HANDLE.get() {
        if let Some(player) = app_handle.try_state::<SharedPlayer>() {
            match cmd_str.as_str() {
                "play" | "resume" => {
                    player.lock().resume();
                }
                "pause" => {
                    player.lock().pause();
                }
                "stop" => {
                    player.lock().pause();
                }
                _ => {}
            }
        }
        // Emit to frontend for state sync (no invoke calls)
        use tauri::Emitter;
        let _ = app_handle.emit("media-button-command", &cmd_str);
    }
}

#[cfg(target_os = "android")]
#[no_mangle]
pub extern "system" fn Java_com_vant_Mio_Music_MusicService_nativePlayerSeek(
    _env: jni::JNIEnv,
    _this: jni::objects::JObject,
    position_ms: jni::sys::jlong,
) {
    let position_secs = (position_ms as f64) / 1000.0;
    if let Some(app_handle) = ANDROID_APP_HANDLE.get() {
        if let Some(player) = app_handle.try_state::<SharedPlayer>() {
            player.lock().seek(position_secs);
        }
        use tauri::Emitter;
        let _ = app_handle.emit("media-button-seek", position_secs);
    }
}
