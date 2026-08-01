// Tauri 后端命令桥接层：保持与原 Wails 绑定同名的导出，最小化前端改动
import { invoke, convertFileSrc } from '@tauri-apps/api/core'
import type { AppConfig, SongMetadata } from './models'

// ---------- 文件对话框 ----------
export function OpenMusicFiles(): Promise<string[]> {
  return invoke('open_music_files')
}

export function OpenMusicFolder(): Promise<string> {
  return invoke('open_music_folder')
}

export function OpenImageFile(): Promise<string> {
  return invoke('open_image_file')
}

// ---------- 本地音乐扫描 / 读取 ----------
export function ScanMusicFolder(folder: string): Promise<string[]> {
  return invoke('scan_music_folder', { folder })
}

export function WatchMusicFolder(folder: string): Promise<void> {
  return invoke('watch_music_folder', { folder })
}

export function ReadMetadata(path: string): Promise<SongMetadata> {
  return invoke('read_metadata', { path })
}

export function ReadCoverArt(path: string): Promise<string> {
  return invoke('read_cover_art', { path })
}

export function ReadLyrics(path: string): Promise<string> {
  return invoke('read_lyrics', { path })
}

export function ReadImageFile(path: string): Promise<string> {
  return invoke('read_image_file', { path })
}

// 本地音频播放地址（Tauri asset 协议，流式读取，无需 HTTP 服务器）
export function AudioSrc(path: string): string {
  return convertFileSrc(path)
}

// ---------- 配置持久化 ----------
export function SaveConfig(config: unknown): Promise<void> {
  return invoke('save_config', { config })
}

export function LoadConfig(): Promise<AppConfig> {
  return invoke('load_config')
}

// ---------- 系统 / 窗口 ----------
export function OpenInExplorer(path: string): Promise<void> {
  return invoke('open_in_explorer', { path })
}

export function OpenSongEditor(path: string): Promise<void> {
  return invoke('open_song_editor', { path })
}

export function EmitMetadataChanged(): Promise<void> {
  return invoke('emit_metadata_changed')
}

export function Version(): Promise<string> {
  return invoke('version')
}

export function ShowMainWindow(): Promise<void> {
  return invoke('show_main_window')
}

export function QuitApp(): Promise<void> {
  return invoke('quit_app')
}

// ---------- 托盘 / 自启动 ----------
export function ApplyAutoStart(enabled: boolean): Promise<void> {
  return invoke('apply_auto_start', { enabled })
}

export function EnableTray(enabled: boolean): Promise<void> {
  return invoke('enable_tray', { enabled })
}

export function SetTraySongInfo(label: string): Promise<void> {
  return invoke('set_tray_song_info', { label })
}

export function SetCloseToTray(enabled: boolean): Promise<void> {
  return invoke('set_close_to_tray', { enabled })
}

// ---------- 桌面歌词窗口 ----------
export function ToggleDesktopLyric(enabled: boolean): Promise<void> {
  return invoke('toggle_desktop_lyric', { enabled })
}

export function CloseDesktopLyric(): Promise<void> {
  return invoke('close_desktop_lyric')
}

export function SetDesktopLyricBounds(x: number, y: number, width: number, height: number): Promise<void> {
  return invoke('set_desktop_lyric_bounds', { x, y, width, height })
}

export function SetDesktopLyricIgnoreMouseEvents(ignore: boolean): Promise<void> {
  return invoke('set_desktop_lyric_ignore_mouse_events', { ignore })
}

export function GetDesktopLyricConfig(): Promise<Record<string, unknown>> {
  return invoke('get_desktop_lyric_config')
}

// ---------- 离线文件下载 ----------
/** 弹出“保存文件”对话框，返回用户选择的完整路径；取消则返回 null。 */
export function SaveFile(defaultName: string): Promise<string | null> {
  return invoke('save_file', { defaultName })
}

/** 把在线音频直链下载为本地文件，返回 { size, path, duration_ms }。 */
export function DownloadFile(
  url: string,
  dest: string,
  headers?: Record<string, string>,
): Promise<{ size: number; path: string; duration_ms: number }> {
  return invoke('download_file', { url, dest, headers: headers ?? null })
}
