import { nextTick, type Ref } from 'vue'
import { Window, Application } from '@bridge/runtime'
import type { AppSettings, ConfigPlayback, ConfigWindow } from './useConfig'
import type { Playlist } from '../types'
import type { useAudioPlayer } from './useAudioPlayer'

export function useSession(
  settings: Ref<AppSettings>,
  playbackState: Ref<ConfigPlayback>,
  windowState: Ref<ConfigWindow>,
  save: () => Promise<void>,
  playlists: Ref<Playlist[]>,
  audio: ReturnType<typeof useAudioPlayer>,
  selectPlaylist: (id: string) => void
) {
  async function handleClose(forceQuit = false) {
    if (!forceQuit && settings.value.closeToTray && settings.value.trayEnabled) {
      Window.Hide()
      return
    }

    if (settings.value.savePlaylistAndSong && audio.playlistId.value && audio.currentSong.value) {
      playbackState.value = {
        playlistId: audio.playlistId.value,
        songIndex: audio.index.value,
        time: audio.currentTime.value,
      }
    }
    if (settings.value.saveWindowPosition) {
      try {
        const pos = await Window.Position()
        const size = await Window.Size()
        windowState.value = {
          x: pos.x,
          y: pos.y,
          width: size.width,
          height: size.height,
        }
      } catch {
        // ignore
      }
    }
    await save()
    Application.Quit()
  }

  async function restoreSession() {
    if (settings.value.saveWindowPosition && windowState.value.width > 0 && windowState.value.height > 0) {
      try {
        await Window.SetPosition(windowState.value.x, windowState.value.y)
        await Window.SetSize(windowState.value.width, windowState.value.height)
      } catch {
        // ignore
      }
    }

    if (settings.value.savePlaylistAndSong && playbackState.value.playlistId) {
      const playlist = playlists.value.find(p => p.id === playbackState.value.playlistId)
      if (playlist && playbackState.value.songIndex >= 0 && playbackState.value.songIndex < playlist.songs.length) {
        selectPlaylist(playbackState.value.playlistId)
        await audio.playSongs(
          playlist.songs,
          playbackState.value.songIndex,
          playlist.id,
          settings.value.autoplay
        )
        if (!settings.value.autoplay && playbackState.value.time > 0) {
          await nextTick()
          audio.seek(playbackState.value.time)
        }
      }
    }
  }

  return { handleClose, restoreSession }
}
