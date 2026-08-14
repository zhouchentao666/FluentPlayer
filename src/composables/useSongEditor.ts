import { ref } from 'vue'
import type { Song } from '../types'

/**
 * 编辑歌曲信息的全局状态。原先是开一个 Tauri 新窗口（WebviewWindow）
 * 加载 index.html?editor=1，现在改为在应用内弹出一个覆盖层界面，
 * 不再创建独立窗口，避免多窗口割裂与启动开销。
 */
const editingSong = ref<Song | null>(null)

export function useSongEditor() {
  /** 打开编辑界面（应用内覆盖层）。 */
  function openEditor(song: Song) {
    editingSong.value = song
  }
  /** 关闭编辑界面。 */
  function closeEditor() {
    editingSong.value = null
  }
  return { editingSong, openEditor, closeEditor }
}
