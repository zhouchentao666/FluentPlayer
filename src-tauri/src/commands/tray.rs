#![cfg(desktop)]

use std::sync::Mutex;
use tauri::menu::{MenuBuilder, MenuItem, MenuItemBuilder};
use tauri::tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, State, Wry};

use super::system::show_main;

pub struct TrayHandles {
    pub tray: TrayIcon<Wry>,
    pub song_item: MenuItem<Wry>,
}

#[derive(Default)]
pub struct TrayState(pub Mutex<Option<TrayHandles>>);

/// 启用 / 禁用系统托盘
#[tauri::command]
pub fn enable_tray(
    app: AppHandle,
    state: State<'_, TrayState>,
    enabled: bool,
) -> Result<(), String> {
    let mut guard = state.0.lock().map_err(|e| e.to_string())?;

    if !enabled {
        if let Some(handles) = guard.take() {
            drop(handles); // TrayIcon 析构即移除托盘图标
        }
        return Ok(());
    }

    if guard.is_some() {
        return Ok(());
    }

    let song_item = MenuItemBuilder::with_id("tray-song", "未在播放")
        .enabled(false)
        .build(&app)
        .map_err(|e| e.to_string())?;
    let show_item = MenuItemBuilder::with_id("tray-show", "显示主界面")
        .build(&app)
        .map_err(|e| e.to_string())?;
    let prev_item = MenuItemBuilder::with_id("tray-prev", "上一首")
        .build(&app)
        .map_err(|e| e.to_string())?;
    let next_item = MenuItemBuilder::with_id("tray-next", "下一首")
        .build(&app)
        .map_err(|e| e.to_string())?;
    let exit_item = MenuItemBuilder::with_id("tray-exit", "退出")
        .build(&app)
        .map_err(|e| e.to_string())?;

    let menu = MenuBuilder::new(&app)
        .item(&song_item)
        .separator()
        .item(&show_item)
        .item(&prev_item)
        .item(&next_item)
        .separator()
        .item(&exit_item)
        .build()
        .map_err(|e| e.to_string())?;

    let mut builder = TrayIconBuilder::with_id("fluentplayer-tray")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .tooltip("FluentPlayer")
        .on_menu_event(|app, event| match event.id().as_ref() {
            "tray-show" => show_main(app),
            "tray-prev" => {
                let _ = app.emit("tray:prev", ());
            }
            "tray-next" => {
                let _ = app.emit("tray:next", ());
            }
            "tray-exit" => {
                let _ = app.emit("tray:exit", ());
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main(tray.app_handle());
            }
        });

    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }

    let tray = builder.build(&app).map_err(|e| e.to_string())?;

    *guard = Some(TrayHandles { tray, song_item });
    Ok(())
}

/// 更新托盘中显示的当前歌曲信息
#[tauri::command]
pub fn set_tray_song_info(state: State<'_, TrayState>, label: String) -> Result<(), String> {
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    if let Some(handles) = guard.as_ref() {
        let text = if label.trim().is_empty() {
            "未在播放".to_string()
        } else {
            label.clone()
        };
        let _ = handles.song_item.set_text(&text);
        let tooltip = if label.trim().is_empty() {
            "FluentPlayer".to_string()
        } else {
            format!("FluentPlayer - {label}")
        };
        let _ = handles.tray.set_tooltip(Some(tooltip));
    }
    Ok(())
}
