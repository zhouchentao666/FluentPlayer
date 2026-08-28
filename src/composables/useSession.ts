import { nextTick, type Ref } from 'vue'
import { Window, Application } from '@bridge/runtime'
import type { AppSettings, ConfigPlayback } from './useConfig'
import type { Playlist } from '../types'
import type { useAudioPlayer } from './useAudioPlayer'

export function useSession(
  settings: Ref<AppSettings>,
  playbackState: Ref<ConfigPlayback>,
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
    await save()
    Application.Quit()
  }

  async function restoreSession() {
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
