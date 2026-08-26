<script setup lang="ts">
import { ref, computed, onMounted, onBeforeUnmount, onActivated, onDeactivated, type CSSProperties } from 'vue'
import { useRouter } from 'vue-router'
import { MessagePlugin, DialogPlugin } from 'tdesign-vue-next'
import { Edit2Icon, PlayCircleIcon, DeleteIcon, ViewListIcon, DownloadIcon } from 'tdesign-icons-vue-next'
import { LocalUserDetailStore, type PlaylistRow } from '@/store/LocalUserDetail'
import { playSong } from '@/utils/audio/globaPlayList'
import { useGlobalPlayStatusStore } from '@/store/GlobalPlayStatus'
import { downloadSingleSong } from '@/utils/audio/download'
import type { SongList } from '@/types/audio'
import { fillMissingCoversWithResolver } from '@/utils/songCover'
import { useSourceAccess } from '@/composables/useSourceAccess'
import LiquidGlass from '@/components/LiquidGlass.vue'
import defaultCover from '/default-cover.png'

const router = useRouter()
const localUserStore = LocalUserDetailStore()
const { getSourceName } = useSourceAccess()
const playStatus = useGlobalPlayStatusStore()
const { t } = useI18n()

const playlists = ref<PlaylistRow[]>([])
const loading = ref(false)
const favoritesId = ref<string | null>(null)
const scrollRef = ref<HTMLElement>()
const scrollTop = ref(0)

// 对话框状态
const showCreatePlaylistDialog = ref(false)
const showEditPlaylistDialog = ref(false)
const showImportDialog = ref(false)
const importView = ref<'menu' | 'network'>('menu')
const createPlaylistLoading = ref(false)

// 创建表单
const newPlaylistForm = ref({ name: '', description: '' })

// 编辑表单
const editPlaylistForm = ref({ name: '', description: '' })
const currentEditingPlaylist = ref<PlaylistRow | null>(null)

// 网络导入
const networkPlaylistUrl = ref('')
const importPlatformType = ref('wy')
const songlistFileInputRef = ref<HTMLInputElement | null>(null)
const networkImportLoading = ref(false)
const networkImportResult = ref<{ type: 'success' | 'error' | 'warning', message: string } | null>(null)

// LiquidGlass 响应式
const isMobile = ref(false)
const mobileMql = typeof window !== 'undefined' ? window.matchMedia('(max-width: 768px)') : null
const onMobileChange = (e: MediaQueryListEvent | MediaQueryList) => { isMobile.value = e.matches }

const networkImportCornerRadius = computed(() => {
  if (!isMobile.value) return 22
  const cssVal = getComputedStyle(document.documentElement).getPropertyValue('--mobile-card-radius')?.trim()
  if (cssVal) { const num = parseFloat(cssVal); if (Number.isFinite(num)) return num }
  return 18
})

const networkImportContentStyle: CSSProperties = {
  color: 'var(--td-text-color-primary)',
  font: 'inherit',
  lineHeight: 'normal',
  textShadow: 'none',
}

async function fillMissingImportedSongPics(songsNeedPic: SongList[], platform: string) {
  await fillMissingCoversWithResolver(songsNeedPic, {
    resolver: async (song) => {
      const url = await (window as any).api?.music?.requestSdk?.('getPic', {
        source: platform,
        songInfo: song
      })
      return typeof url === 'string' ? url : null
    }
  })
}

// 加载歌单列表
const loadPlaylists = async () => {
  loading.value = true
  try {
    await localUserStore.loadPlaylists()
    playlists.value = [...localUserStore.playlists]

    // "我的喜欢"置顶
    try {
      const favId = await localUserStore.getFavoritesId()
      favoritesId.value = favId
      if (favId) {
        const idx = playlists.value.findIndex(p => p.id === favId)
        if (idx > 0) {
          const fav = playlists.value.splice(idx, 1)[0]
          playlists.value.unshift(fav)
        }
      }
    } catch {}
  } catch {
    MessagePlugin.error(t('music.songlist.loadFailed'))
  } finally {
    loading.value = false
  }
}

// 创建歌单
const openCreatePlaylistDialog = () => {
  showCreatePlaylistDialog.value = true
  newPlaylistForm.value = { name: '', description: '' }
  createPlaylistLoading.value = false
  document.body.style.overflow = 'hidden'
}

const closeCreatePlaylistDialog = () => {
  if (createPlaylistLoading.value) return
  showCreatePlaylistDialog.value = false
  document.body.style.overflow = ''
}

const createPlaylist = async () => {
  if (!newPlaylistForm.value.name.trim()) {
    MessagePlugin.warning(t('music.songlist.nameEmpty'))
    return
  }
  createPlaylistLoading.value = true
  try {
    const created = await localUserStore.createPlaylist(
      newPlaylistForm.value.name.trim(),
      newPlaylistForm.value.description.trim() || undefined,
      'local'
    )
    if (created) {
      showCreatePlaylistDialog.value = false
      newPlaylistForm.value = { name: '', description: '' }
      document.body.style.overflow = ''
      MessagePlugin.success(t('music.songlist.createSuccess'))
    } else {
      MessagePlugin.error(t('music.songlist.createFailed'))
    }
  } catch {
    MessagePlugin.error(t('music.songlist.createFailed'))
  } finally {
    createPlaylistLoading.value = false
  }
}

// 编辑歌单
const editPlaylist = (playlist: PlaylistRow) => {
  currentEditingPlaylist.value = playlist
  editPlaylistForm.value = {
    name: playlist.name,
    description: playlist.description || ''
  }
  showEditPlaylistDialog.value = true
}

const savePlaylistEdit = async () => {
  if (!currentEditingPlaylist.value) return
  if (!editPlaylistForm.value.name.trim()) {
    MessagePlugin.warning(t('music.songlist.nameEmpty'))
    return
  }
  try {
    const ok = await localUserStore.updatePlaylist(
      currentEditingPlaylist.value.id,
      editPlaylistForm.value.name.trim(),
      editPlaylistForm.value.description.trim()
    )
    if (ok) {
      MessagePlugin.success(t('music.songlist.updateSuccess'))
      showEditPlaylistDialog.value = false
      currentEditingPlaylist.value = null
      await loadPlaylists()
    } else {
      MessagePlugin.error(t('music.songlist.updateFailed'))
    }
  } catch {
    MessagePlugin.error(t('music.songlist.updateFailed'))
  }
}

// 删除歌单
const deletePlaylist = (playlist: PlaylistRow) => {
  const confirmDialog = DialogPlugin.confirm({
    header: t('music.songlist.confirmDelete'),
    body: t('music.songlist.deleteConfirm', { name: playlist.name }),
    confirmBtn: t('common.delete'),
    cancelBtn: t('common.cancel'),
    theme: 'danger',
    onConfirm: async () => {
      try {
        const ok = await localUserStore.deletePlaylist(playlist.id)
        if (ok) {
          MessagePlugin.success(t('music.songlist.deleteSuccess'))
          playlists.value = playlists.value.filter(p => p.id !== playlist.id)
        } else {
          MessagePlugin.error(t('music.songlist.deleteFailed'))
        }
      } catch {
        MessagePlugin.error(t('music.songlist.deleteFailed'))
      }
      confirmDialog.destroy()
    },
    onCancel: () => confirmDialog.destroy()
  })
}

// 查看歌单详情
const viewPlaylist = (playlist: PlaylistRow) => {
  router.push({
    name: 'list',
    params: { id: playlist.id },
    query: {
      title: playlist.name,
      cover: playlist.coverImgUrl && playlist.coverImgUrl !== 'default-cover' ? playlist.coverImgUrl : '',
      source: playlist.source,
      type: 'local',
      description: playlist.description || ''
    }
  })
}

// 播放歌单
const playPlaylist = async (playlist: PlaylistRow) => {
  try {
    const rows = await localUserStore.getSongsForPlaylist(playlist.id)
    if (!rows || rows.length === 0) {
      MessagePlugin.warning(t('music.songlist.noSongs'))
      return
    }
    const songs: SongList[] = rows.map(r => {
      try {
        return JSON.parse(r.data)
      } catch {
        return {
          songmid: r.songmid, name: r.name, singer: r.singer,
          albumName: r.albumName, img: r.img, source: 'local', url: ''
        } as any
      }
    })
    localUserStore.replaceSongList(songs)
    playSong(songs[0] as any)
    playStatus.updatePlayerInfo(songs[0] as any)
    MessagePlugin.success(t('music.songlist.playingPlaylist', { name: playlist.name }))
  } catch {
    MessagePlugin.error(t('music.songlist.playFailed'))
  }
}

// 下载歌单全部歌曲
const downloadPlaylist = async (playlist: PlaylistRow) => {
  try {
    const rows = await localUserStore.getSongsForPlaylist(playlist.id)
    if (!rows || rows.length === 0) {
      MessagePlugin.warning(t('music.songlist.noSongs'))
      return
    }
    const songs: any[] = rows.map(r => {
      try { return JSON.parse(r.data) } catch {
        return { songmid: r.songmid, name: r.name, singer: r.singer, albumName: r.albumName, img: r.img, source: 'local' }
      }
    })
    MessagePlugin.info(t('music.songlist.startDownloadCount', { count: songs.length }))
    songs.forEach(s => downloadSingleSong(s as any))
  } catch {
    MessagePlugin.error(t('music.songlist.downloadFailed'))
  }
}

// --- 导入功能 ---

// 从播放列表导入
const importFromPlaylist = async () => {
  showImportDialog.value = false
  document.body.style.overflow = ''
  const currentList = JSON.parse(JSON.stringify(localUserStore.list))
  if (!currentList || currentList.length === 0) {
    MessagePlugin.warning(t('music.songlist.playingListEmpty'))
    return
  }
  try {
    const now = new Date()
    const playlistName = `播放列表 ${now.getFullYear()}-${(now.getMonth() + 1).toString().padStart(2, '0')}-${now.getDate().toString().padStart(2, '0')} ${now.getHours().toString().padStart(2, '0')}:${now.getMinutes().toString().padStart(2, '0')}`
    const created = await localUserStore.createPlaylist(playlistName, t('music.songlist.fromPlaylistImport', { count: currentList.length }), 'local')
    if (created) {
      const added = await localUserStore.addSongsToPlaylist(created.id, currentList)
      MessagePlugin.success(t('music.songlist.fromPlaylistSuccess', { count: added, name: playlistName }))
      await loadPlaylists()
    } else {
      MessagePlugin.error(t('music.songlist.createFailed'))
    }
  } catch {
    MessagePlugin.error(t('music.songlist.fromPlaylistFailed'))
  }
}

// 从本地文件导入
const triggerSonglistFileInput = () => {
  showImportDialog.value = false
  document.body.style.overflow = ''
  songlistFileInputRef.value?.click()
}

const handleSonglistFileChange = async (e: Event) => {
  const input = e.target as HTMLInputElement
  if (!input.files?.length) return
  const file = input.files[0]
  try {
    const text = await file.text()
    let imported: any[]
    try {
      imported = JSON.parse(text)
    } catch {
      MessagePlugin.error(t('music.songlist.parseFileFailed'))
      return
    }
    if (!Array.isArray(imported)) {
      MessagePlugin.error(t('music.songlist.fileFormatError'))
      return
    }
    const rawName = file.name.replace(/\.(cmpl|cpl|json)$/i, '')
    const created = await localUserStore.createPlaylist(rawName, t('music.songlist.fromFileImport'), 'local')
    if (created) {
      const added = await localUserStore.addSongsToPlaylist(created.id, imported)
      MessagePlugin.success(t('music.songlist.fromFileSuccess', { count: added, name: rawName }))
      await loadPlaylists()
    }
  } catch (err) {
    MessagePlugin.error(t('music.songlist.importFailed', { error: (err as Error).message }))
  } finally {
    input.value = ''
  }
}

// 从网络歌单导入
const importFromNetwork = () => {
  importView.value = 'network'
  networkPlaylistUrl.value = ''
  importPlatformType.value = 'wy'
  networkImportLoading.value = false
  networkImportResult.value = null
}

const confirmNetworkImport = async () => {
  if (!networkPlaylistUrl.value.trim()) {
    networkImportResult.value = { type: 'warning', message: t('music.songlist.invalidLink') }
    return
  }
  await handleNetworkPlaylistImport(networkPlaylistUrl.value.trim())
}

const cancelNetworkImport = () => {
  if (networkImportLoading.value) return
  showImportDialog.value = false
  importView.value = 'menu'
  networkPlaylistUrl.value = ''
  networkImportResult.value = null
  document.body.style.overflow = ''
}

const openImportDialog = () => {
  importView.value = 'menu'
  showImportDialog.value = true
  document.body.style.overflow = 'hidden'
}

// 解析歌单 ID
const resolvePlaylistId = (input: string, platform: string): string | null => {
  const isNumeric = /^\d+$/.test(input)
  if (platform === 'wy') {
    const m = input.match(/(?:music\.163\.com\/.*[?&]id=|playlist\?id=|playlist\/|id=)(\d+)/i)
    return m?.[1] || (isNumeric ? input : null)
  }
  if (platform === 'tx') {
    const m = input.match(/(?:y\.qq\.com\/n\/ryqq\/playlist\/|music\.qq\.com\/.*[?&]id=|playlist[?&]id=|[?&]id=)(\d+)/i)
    return m?.[1] || (isNumeric ? input : null)
  }
  if (platform === 'kw') {
    const m = input.match(/(?:kuwo\.cn\/playlist_detail\/|kuwo\.cn\/.*[?&]pid=|[?&](?:pid|id)=)(\d+)/i)
    return m?.[1] || (isNumeric ? input : null)
  }
  if (platform === 'kg') {
    return input // 酷狗传完整链接
  }
  if (platform === 'mg') {
    const m = input.match(/[?&]id=(\d+)/i)
    return m?.[1] || (isNumeric ? input : null)
  }
  if (platform === 'bd') {
    const m = input.match(/[?&]playlistId=(\d+)/i)
    return m?.[1] || (isNumeric ? input : null)
  }
  return isNumeric ? input : null
}

const platformNames = computed<Record<string, string>>(() => ({
  wy: t('music.songlist.netease'), tx: t('music.songlist.qq'), kw: t('music.songlist.kuwo'),
  bd: t('music.songlist.bodo'), kg: t('music.songlist.kugou'), mg: t('music.songlist.migu')
}))

const handleNetworkPlaylistImport = async (input: string) => {
  const platform = importPlatformType.value
  const pName = platformNames.value[platform] || t('music.songlist.selectPlatform')
  const playlistId = resolvePlaylistId(input, platform)

  if (!playlistId) {
    networkImportResult.value = { type: 'error', message: t('music.songlist.unrecognizedLink', { platform: pName }) }
    return
  }

  networkImportLoading.value = true
  networkImportResult.value = null
  try {
    let allSongs: any[] = []
    let detailInfo: any = {}
    let page = 1
    let total = Infinity

    while (allSongs.length < total) {
      const detailResult = await (window as any).api?.music?.requestSdk?.('getPlaylistDetail', {
        source: platform,
        id: playlistId,
        page
      })
      if (!detailResult || detailResult.error) {
        networkImportResult.value = { type: 'error', message: t('music.songlist.fetchDetailFailed', { platform: pName }) + (detailResult?.error ? `：${detailResult.error}` : '') }
        return
      }
      const list = detailResult.list || []
      if (detailResult.info) detailInfo = detailResult.info
      total = detailResult.total || list.length
      allSongs = allSongs.concat(list)
      page++
      if (list.length === 0) break
    }

    if (allSongs.length === 0) {
      networkImportResult.value = { type: 'warning', message: t('music.songlist.emptyPlaylist') }
      return
    }

    const songsNeedPic = allSongs.filter(s => !s.img)
    if (songsNeedPic.length > 0) {
      await fillMissingImportedSongPics(songsNeedPic, platform)
    }

    const coverImg = detailInfo.cover || detailInfo.img || ''
    const name = t('music.songlist.importedFrom', { platform: pName })
    const created = await localUserStore.createPlaylist(name, t('music.songlist.fromNetworkImport', { platform: pName, count: allSongs.length }), platform)
    if (created) {
      const added = await localUserStore.addSongsToPlaylist(created.id, allSongs)
      if (coverImg) await localUserStore.updatePlaylistCover(created.id, coverImg)
      networkImportResult.value = { type: 'success', message: t('music.songlist.networkImportSuccess', { count: added, name }) }
      await loadPlaylists()
      setTimeout(() => {
        showImportDialog.value = false
        importView.value = 'menu'
        document.body.style.overflow = ''
      }, 1500)
    }
  } catch (err) {
    networkImportResult.value = { type: 'error', message: t('music.songlist.importFailed', { error: (err as Error).message }) }
  } finally {
    networkImportLoading.value = false
  }
}

// 右键菜单
const contextMenuVisible = ref(false)
const contextMenuPos = ref({ top: 0, left: 0 })
const contextMenuPlaylist = ref<PlaylistRow | null>(null)

const handleContextMenu = (e: MouseEvent, playlist: PlaylistRow) => {
  e.preventDefault()
  e.stopPropagation()
  contextMenuPlaylist.value = playlist
  const rect = (e.currentTarget as HTMLElement).getBoundingClientRect()
  contextMenuPos.value = { top: e.clientY, left: e.clientX }
  contextMenuVisible.value = true
}

const closeContextMenu = () => {
  contextMenuVisible.value = false
  contextMenuPlaylist.value = null
}

const handleMenuAction = async (action: string) => {
  const pl = contextMenuPlaylist.value
  if (!pl) return
  closeContextMenu()
  if (action === 'play') await playPlaylist(pl)
  else if (action === 'view') viewPlaylist(pl)
  else if (action === 'edit') editPlaylist(pl)
  else if (action === 'delete') deletePlaylist(pl)
  else if (action === 'download') await downloadPlaylist(pl)
}

// 格式化日期
const formatDate = (iso: string) => {
  if (!iso) return ''
  try {
    const d = new Date(iso)
    return `${d.getFullYear()}-${(d.getMonth() + 1).toString().padStart(2, '0')}-${d.getDate().toString().padStart(2, '0')}`
  } catch { return '' }
}

const handleGlobalClick = () => { if (contextMenuVisible.value) closeContextMenu() }

// 生命周期
onMounted(() => {
  loadPlaylists()
  document.addEventListener('click', handleGlobalClick)
  if (mobileMql) { onMobileChange(mobileMql); mobileMql.addEventListener('change', onMobileChange) }
})
onBeforeUnmount(() => {
  document.removeEventListener('click', handleGlobalClick)
  if (mobileMql) mobileMql.removeEventListener('change', onMobileChange)
})
onActivated(() => { if (scrollRef.value) scrollRef.value.scrollTop = scrollTop.value })
onDeactivated(() => { if (scrollRef.value) scrollTop.value = scrollRef.value.scrollTop })
</script>

<template>
  <div ref="scrollRef" class="page">
    <input
      ref="songlistFileInputRef"
      accept=".cmpl,.cpl,.json"
      style="display: none"
      type="file"
      @change="handleSonglistFileChange"
    />
    <div class="local-container">
      <!-- 页面标题 -->
      <div class="page-header">
        <div class="header-left">
          <h2>{{ t('music.songlist.title') }}</h2>
        </div>
        <div class="header-actions">
          <t-button theme="primary" variant="outline" @click="openCreatePlaylistDialog">
            <i class="iconfont icon-zengjia"></i>
            {{ t('music.songlist.newPlaylist') }}
          </t-button>
          <t-button theme="primary" @click="openImportDialog">
            <i class="iconfont icon-daoru"></i>
            {{ t('common.import') }}
          </t-button>
        </div>
      </div>

      <!-- 歌单区域 -->
      <div class="playlists-section">
        <div class="section-header">
          <h3>{{ t('music.songlist.myPlaylists', { count: playlists.length }) }}</h3>
          <div class="section-actions">
            <t-button :loading="loading" size="small" theme="primary" variant="text" @click="loadPlaylists">
              <i class="iconfont icon-shuaxin"></i>
              {{ t('common.refresh') }}
            </t-button>
          </div>
        </div>

        <!-- 加载状态 -->
        <div v-if="loading" class="loading-state">
          <t-loading size="large" :text="t('common.loading')" />
        </div>

        <!-- 歌单网格 -->
        <div v-else-if="playlists.length > 0" class="playlists-grid">
          <div
            v-for="playlist in playlists"
            :key="playlist.id"
            class="playlist-card"
            @contextmenu="handleContextMenu($event, playlist)"
          >
            <div class="playlist-cover" @click="viewPlaylist(playlist)">
              <img
                v-if="playlist.coverImgUrl && playlist.coverImgUrl !== 'default-cover'"
                :alt="playlist.name"
                :src="playlist.coverImgUrl"
                class="cover-image"
                loading="lazy"
              />
              <img v-else :alt="playlist.name" :src="defaultCover" class="cover-image" />
              <div class="cover-overlay">
                <i class="iconfont icon-bofang"></i>
              </div>
            </div>
            <div class="playlist-info">
              <div class="playlist-name-row" @click="viewPlaylist(playlist)">
                <div :title="playlist.name" class="playlist-name-text">
                  {{ playlist.name }}
                </div>
                <div v-if="playlist.id === favoritesId" class="playlist-tags">
                  <t-tag size="small" theme="danger" variant="light-outline">{{ t('music.songlist.myFavorite') }}</t-tag>
                </div>
              </div>
              <div :title="playlist.description" class="playlist-description">
                {{ playlist.description || t('music.songlist.noDescription') }}
              </div>
              <div class="playlist-meta">
                <span class="source-tag">{{ getSourceName(playlist.source || 'local') }}</span>
                <span v-if="playlist.createTime">{{ t('music.songlist.createdAt', { date: formatDate(playlist.createTime) }) }}</span>
              </div>
            </div>
            <div class="playlist-actions">
              <t-tooltip :content="t('music.songlist.playPlaylist')">
                <t-button shape="circle" size="small" theme="primary" variant="text" @click="playPlaylist(playlist)">
                  <i class="iconfont icon-bofang"></i>
                </t-button>
              </t-tooltip>
              <t-tooltip :content="t('music.songlist.viewDetail')">
                <t-button shape="circle" size="small" theme="default" variant="text" @click="viewPlaylist(playlist)">
                  <ViewListIcon :stroke-width="1.5" />
                </t-button>
              </t-tooltip>
              <t-tooltip :content="t('music.songlist.editPlaylist')">
                <t-button shape="circle" size="small" theme="success" variant="text" @click="editPlaylist(playlist)">
                  <Edit2Icon />
                </t-button>
              </t-tooltip>
              <t-tooltip :content="t('music.songlist.deletePlaylist')">
                <t-button shape="circle" size="small" theme="danger" variant="text" @click="deletePlaylist(playlist)">
                  <i class="iconfont icon-shanchu"></i>
                </t-button>
              </t-tooltip>
            </div>
          </div>
        </div>

        <!-- 空状态 -->
        <div v-else class="empty-playlists">
          <div class="empty-icon">
            <i class="iconfont icon-gedan"></i>
          </div>
          <h4>{{ t('music.songlist.emptyTitle') }}</h4>
          <p>{{ t('music.songlist.emptyDesc') }}</p>
          <t-button theme="primary" @click="openCreatePlaylistDialog">
            <i class="iconfont icon-zengjia"></i>
            {{ t('music.songlist.createPlaylist') }}
          </t-button>
        </div>
      </div>
    </div>

    <!-- 创建歌单对话框 -->
    <Teleport to="body">
      <Transition name="create-pl-fade">
        <div v-if="showCreatePlaylistDialog" class="create-pl-overlay" @click.self="closeCreatePlaylistDialog">
          <div class="overlay-drag-bar" data-tauri-drag-region />
          <LiquidGlass
            class="create-pl-panel"
            :corner-radius="networkImportCornerRadius"
            :displacement-scale="48"
            :blur-amount="0.08"
            :saturation="180"
            :aberration-intensity="1.5"
            padding="0"
            mode="standard"
            :content-style="networkImportContentStyle"
            @click.stop
          >
            <div class="create-pl-panel__content">
              <!-- Header -->
              <div class="create-pl-header" data-tauri-drag-region>
                <div class="create-pl-title-group">
                  <div class="create-pl-icon">
                    <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
                      <line x1="12" y1="5" x2="12" y2="19" /><line x1="5" y1="12" x2="19" y2="12" />
                    </svg>
                  </div>
                  <div class="create-pl-title-text">
                    <h2 class="create-pl-title">{{ t('music.songlist.createNewPlaylist') }}</h2>
                  </div>
                </div>
                <button class="create-pl-close-btn" @click="closeCreatePlaylistDialog">
                  <i class="iconfont icon-a-quxiaoguanbi" />
                </button>
              </div>

              <!-- Form -->
              <div class="create-pl-form">
                <div class="form-field">
                  <label class="field-label">{{ t('music.songlist.playlistName') }} <span class="required">*</span></label>
                  <input
                    v-model="newPlaylistForm.name"
                    class="field-input"
                    :placeholder="t('music.songlist.playlistNamePlaceholder')"
                    maxlength="50"
                    autofocus
                    @keydown.enter="createPlaylist"
                  />
                </div>
                <div class="form-field">
                  <label class="field-label">{{ t('music.songlist.playlistDesc') }}</label>
                  <textarea
                    v-model="newPlaylistForm.description"
                    class="field-textarea"
                    :placeholder="t('music.songlist.playlistDescPlaceholder')"
                    maxlength="200"
                    rows="3"
                  />
                </div>
              </div>

              <!-- Actions -->
              <div class="create-pl-actions">
                <button class="create-pl-btn outline" @click="closeCreatePlaylistDialog">
                  {{ t('common.cancel') }}
                </button>
                <button
                  :class="['create-pl-btn', 'primary', { disabled: createPlaylistLoading || !newPlaylistForm.name.trim() }]"
                  @click="createPlaylist"
                >
                  <span v-if="createPlaylistLoading" class="create-pl-spinner" />
                  {{ t('common.create') }}
                </button>
              </div>
            </div>
          </LiquidGlass>
        </div>
      </Transition>
    </Teleport>

    <!-- 编辑歌单对话框 -->
    <t-dialog
      v-model:visible="showEditPlaylistDialog"
      :cancel-btn="{ content: t('common.cancel'), variant: 'outline' }"
      :confirm-btn="{ content: t('common.save'), theme: 'primary' }"
      :header="t('music.songlist.editPlaylistInfo')"
      placement="center"
      width="500px"
      @confirm="savePlaylistEdit"
    >
      <div class="edit-playlist-content">
        <div class="form-item">
          <label class="form-label">{{ t('music.songlist.playlistName') }}</label>
          <t-input
            v-model="editPlaylistForm.name"
            autofocus
            clearable
            maxlength="50"
            :placeholder="t('music.songlist.playlistNamePlaceholder')"
            show-word-limit
          />
        </div>
        <div class="form-item">
          <label class="form-label">{{ t('music.songlist.playlistDesc') }}</label>
          <t-textarea
            v-model="editPlaylistForm.description"
            :autosize="{ minRows: 3, maxRows: 6 }"
            maxlength="200"
            :placeholder="t('music.songlist.playlistDescPlaceholder')"
            show-word-limit
          />
        </div>
      </div>
    </t-dialog>

    <!-- 导入对话框 (统一 LiquidGlass) -->
    <Teleport to="body">
      <Transition name="netimport-fade">
        <div v-if="showImportDialog" class="netimport-overlay" @click.self="cancelNetworkImport">
          <div class="overlay-drag-bar" data-tauri-drag-region />
          <LiquidGlass
            class="netimport-panel"
            :corner-radius="networkImportCornerRadius"
            :displacement-scale="48"
            :blur-amount="0.08"
            :saturation="180"
            :aberration-intensity="1.5"
            padding="0"
            mode="standard"
            :content-style="networkImportContentStyle"
            @click.stop
          >
            <div class="netimport-panel__content">
              <!-- Header -->
              <div class="netimport-header" data-tauri-drag-region>
                <div class="netimport-title-group">
                  <button v-if="importView === 'network'" class="netimport-back-btn" @click="importView = 'menu'">
                    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
                      <polyline points="15 18 9 12 15 6" />
                    </svg>
                  </button>
                  <div class="netimport-icon">
                    <svg v-if="importView === 'menu'" width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
                      <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
                      <polyline points="17 8 12 3 7 8" />
                      <line x1="12" y1="3" x2="12" y2="15" />
                    </svg>
                    <svg v-else width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
                      <path d="M12 2a10 10 0 1 0 0 20 10 10 0 0 0 0-20z" />
                      <line x1="2" y1="12" x2="22" y2="12" />
                      <path d="M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10A15.3 15.3 0 0 1 12 2z" />
                    </svg>
                  </div>
                  <div class="netimport-title-text">
                    <h2 class="netimport-title">{{ importView === 'menu' ? t('music.songlist.selectImportMethod') : t('music.songlist.importNetworkPlaylist') }}</h2>
                  </div>
                </div>
                <button class="netimport-close-btn" @click="cancelNetworkImport">
                  <i class="iconfont icon-a-quxiaoguanbi" />
                </button>
              </div>

              <!-- Menu View -->
              <div v-if="importView === 'menu'" class="import-options">
                <div class="import-option" @click="importFromPlaylist">
                  <div class="option-icon">
                    <i class="iconfont icon-liebiao"></i>
                  </div>
                  <div class="option-content">
                    <h4>{{ t('music.songlist.fromPlayingList') }}</h4>
                    <p>{{ t('music.songlist.fromPlayingListDesc') }}</p>
                  </div>
                  <div class="option-arrow">
                    <i class="iconfont icon-youjiantou"></i>
                  </div>
                </div>
                <div class="import-option" @click="triggerSonglistFileInput">
                  <div class="option-icon">
                    <i class="iconfont icon-daoru"></i>
                  </div>
                  <div class="option-content">
                    <h4>{{ t('music.songlist.fromLocalFile') }}</h4>
                    <p>{{ t('music.songlist.fromLocalFileDesc') }}</p>
                  </div>
                  <div class="option-arrow">
                    <i class="iconfont icon-youjiantou"></i>
                  </div>
                </div>
                <div class="import-option" @click="importFromNetwork">
                  <div class="option-icon">
                    <i class="iconfont icon-wangluo"></i>
                  </div>
                  <div class="option-content">
                    <h4>{{ t('music.songlist.fromNetwork') }}</h4>
                    <p>{{ t('music.songlist.fromNetworkDesc') }}</p>
                    <span class="coming-soon">{{ t('common.experimental') }}</span>
                  </div>
                  <div class="option-arrow">
                    <i class="iconfont icon-youjiantou"></i>
                  </div>
                </div>
              </div>

              <!-- Network Import View -->
              <template v-else>
                <div class="platform-grid">
                  <button
                    v-for="p in [
                      { key: 'wy', color: '#e60026' },
                      { key: 'tx', color: '#31c27a' },
                      { key: 'kw', color: '#f5a623' },
                      { key: 'bd', color: '#5b9df5' },
                      { key: 'kg', color: '#2ca2f9' },
                      { key: 'mg', color: '#ff6c37' },
                    ]"
                    :key="p.key"
                    :class="['platform-card', { active: importPlatformType === p.key }]"
                    @click="importPlatformType = p.key"
                  >
                    <span class="platform-dot" :style="{ background: p.color }" />
                    <span class="platform-name">{{ platformNames[p.key] }}</span>
                  </button>
                </div>

                <div class="netimport-input-group">
                  <p class="input-guide">{{ t('music.songlist.networkImportGuide', { platform: platformNames[importPlatformType] || t('music.songlist.selectPlatform') }) }}</p>
                  <div class="input-row">
                    <input
                      v-model="networkPlaylistUrl"
                      :placeholder="t('music.songlist.networkImportPlaceholder')"
                      class="netimport-input"
                      :disabled="networkImportLoading"
                      autofocus
                      @keydown.enter="confirmNetworkImport"
                    />
                    <button
                      :class="['netimport-btn', 'primary', { disabled: networkImportLoading }]"
                      @click="confirmNetworkImport"
                    >
                      <span v-if="networkImportLoading" class="netimport-spinner" />
                      <template v-else>
                        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
                          <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
                          <polyline points="7 10 12 15 17 10" />
                          <line x1="12" y1="15" x2="12" y2="3" />
                        </svg>
                      </template>
                      {{ networkImportLoading ? t('music.songlist.fetchingInfo') : t('music.songlist.startImport') }}
                    </button>
                  </div>

                  <!-- Result -->
                  <Transition name="result-fade">
                    <div v-if="networkImportResult" :class="['netimport-result', networkImportResult.type]">
                      <svg v-if="networkImportResult.type === 'success'" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
                        <polyline points="20 6 9 17 4 12" />
                      </svg>
                      <svg v-else-if="networkImportResult.type === 'error'" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                        <circle cx="12" cy="12" r="10" /><line x1="15" y1="9" x2="9" y2="15" /><line x1="9" y1="9" x2="15" y2="15" />
                      </svg>
                      <svg v-else width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                        <circle cx="12" cy="12" r="10" /><line x1="12" y1="16" x2="12" y2="12" /><line x1="12" y1="8" x2="12.01" y2="8" />
                      </svg>
                      <span>{{ networkImportResult.message }}</span>
                    </div>
                  </Transition>
                </div>
              </template>
            </div>
          </LiquidGlass>
        </div>
      </Transition>
    </Teleport>

    <!-- 右键菜单 -->
    <Teleport to="body">
      <div
        v-if="contextMenuVisible"
        class="context-menu"
        :style="{ top: contextMenuPos.top + 'px', left: contextMenuPos.left + 'px' }"
        @click.stop
      >
        <div class="menu-item" @click="handleMenuAction('play')">
          <i class="iconfont icon-bofang"></i> {{ t('music.songlist.playPlaylist') }}
        </div>
        <div class="menu-item" @click="handleMenuAction('view')">
          <ViewListIcon :stroke-width="1.5" style="width:14px;height:14px" /> {{ t('music.songlist.viewDetail') }}
        </div>
        <div class="menu-item" @click="handleMenuAction('download')">
          <DownloadIcon style="width:14px;height:14px" /> {{ t('music.songlist.downloadAll') }}
        </div>
        <div class="menu-separator"></div>
        <div class="menu-item" @click="handleMenuAction('edit')">
          <Edit2Icon style="width:14px;height:14px" /> {{ t('music.songlist.editPlaylist') }}
        </div>
        <div class="menu-item danger" @click="handleMenuAction('delete')">
          <DeleteIcon style="width:14px;height:14px" /> {{ t('music.songlist.deletePlaylist') }}
        </div>
      </div>
    </Teleport>
  </div>
</template>

<style scoped>
.page {
  width: 100%;
  height: 100%;
  overflow-y: auto;
  box-sizing: border-box;
}

.local-container {
  padding: 0 2rem;
  padding-top: 1rem;
  width: 100%;
  box-sizing: border-box;
  color: var(--td-text-color-primary);
}

/* 页面标题 */
.page-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  margin-bottom: 2rem;
}

.page-header h2 {
  border-left: 8px solid var(--td-brand-color-3);
  padding-left: 12px;
  border-radius: 8px;
  line-height: 1.5em;
  color: var(--td-text-color-primary);
  margin: 0;
  font-size: 1.875rem;
  font-weight: 600;
}

.header-actions {
  display: flex;
  gap: 0.75rem;
}

/* 歌单区域 */
.playlists-section {
  margin-bottom: 3rem;
}

.section-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 1.5rem;
}

.section-header h3 {
  font-size: 1.25rem;
  font-weight: 600;
  color: var(--td-text-color-primary);
}

.section-actions {
  display: flex;
  gap: 0.5rem;
}

.loading-state {
  display: flex;
  justify-content: center;
  align-items: center;
  padding: 4rem 2rem;
}

/* 歌单网格 */
.playlists-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(240px, 1fr));
  gap: 1.5rem;
}

.playlist-card {
  display: flex;
  flex-direction: column;
  background: var(--td-bg-color-container);
  border-radius: 0.75rem;
  overflow: hidden;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.08);
  transition: transform 0.2s ease, box-shadow 0.2s ease;
  content-visibility: auto;
  contain-intrinsic-size: 0 340px;
}

.playlist-card:hover {
  transform: translateY(-4px);
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.15);
}

.playlist-card:hover .cover-overlay {
  opacity: 1;
}

.playlist-cover {
  height: 180px;
  background: var(--td-bg-color-component-hover);
  position: relative;
  cursor: pointer;
  overflow: hidden;
}

.cover-image {
  width: 100%;
  height: 100%;
  object-fit: cover;
}

.cover-overlay {
  position: absolute;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
  background: rgba(0, 0, 0, 0.4);
  display: flex;
  align-items: center;
  justify-content: center;
  opacity: 0;
  transition: opacity 0.2s ease;
}

.cover-overlay .iconfont {
  font-size: 3rem;
  color: #fff;
}

.playlist-info {
  padding: 1rem;
  flex: 1;
  display: flex;
  flex-direction: column;
}

.playlist-name-row {
  display: flex;
  align-items: flex-start;
  margin-bottom: 0.5rem;
  cursor: pointer;
  gap: 6px;
}

.playlist-name-row:hover .playlist-name-text {
  color: var(--td-brand-color);
}

.playlist-name-text {
  font-weight: 600;
  color: var(--td-text-color-primary);
  font-size: 1rem;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
  text-overflow: ellipsis;
  word-break: break-all;
  line-height: 1.4;
}

.playlist-tags {
  display: flex;
  align-items: center;
  gap: 6px;
  flex-shrink: 0;
  height: 1.4em;
  margin-top: 1px;
}

.playlist-description {
  flex: 1;
  font-size: 0.78rem;
  color: var(--td-text-color-secondary);
  margin-bottom: 0.5rem;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
  text-overflow: ellipsis;
}

.playlist-meta {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
  font-size: 0.75rem;
  color: var(--td-text-color-placeholder);
}

.source-tag {
  text-transform: uppercase;
  font-weight: 500;
  color: var(--td-brand-color);
}

.playlist-actions {
  display: flex;
  justify-content: flex-end;
  padding: 0 1rem 1rem;
  gap: 0.5rem;
}

/* 空状态 */
.empty-playlists {
  text-align: center;
  padding: 4rem 2rem;
}

.empty-icon {
  margin-bottom: 1.5rem;
}

.empty-icon .iconfont {
  font-size: 4rem;
  color: var(--td-text-color-placeholder);
}

.empty-playlists h4 {
  color: var(--td-text-color-primary);
  margin-bottom: 0.5rem;
  font-size: 1.125rem;
  font-weight: 600;
}

.empty-playlists p {
  color: var(--td-text-color-secondary);
  margin-bottom: 2rem;
}


.edit-playlist-content .form-item {
  margin-bottom: 1.5rem;
}

.edit-playlist-content .form-item:last-child {
  margin-bottom: 0;
}

.edit-playlist-content .form-label {
  display: block;
  margin-bottom: 0.5rem;
  font-weight: 500;
  color: var(--td-text-color-primary);
  font-size: 14px;
}


@media (max-width: 768px) {
  .page {
    -webkit-overflow-scrolling: touch;
  }

  .local-container {
    padding: var(--mobile-page-top-gutter) var(--mobile-page-gutter) 0;
  }

  .page-header {
    flex-direction: column;
    gap: 1rem;
    margin-bottom: 1.5rem;
  }

  .page-header h2 {
    border-left: none;
    padding-left: 0;
    font-size: clamp(2rem, 9vw, 2.6rem);
    line-height: 1.1;
    letter-spacing: -0.04em;
  }

  .header-actions {
    width: 100%;
    gap: 0.5rem;
  }

  .header-actions :deep(.t-button) {
    flex: 1;
    min-height: var(--mobile-touch-target);
    border-radius: var(--mobile-control-radius);
    touch-action: manipulation;
  }

  .section-header {
    align-items: center;
    margin-bottom: 1rem;
  }

  .section-header h3 {
    font-size: 1.1rem;
    margin: 0;
  }

  .section-actions :deep(.t-button) {
    min-height: var(--mobile-touch-target);
    border-radius: var(--mobile-control-radius);
  }

  .playlists-grid {
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 12px;
  }

  .playlist-card {
    border-radius: var(--mobile-card-radius-small);
    box-shadow: 0 8px 24px rgba(15, 23, 42, 0.08);
    contain-intrinsic-size: 0 260px;
  }

  .playlist-card:hover {
    transform: none;
    box-shadow: 0 8px 24px rgba(15, 23, 42, 0.08);
  }

  .playlist-cover {
    height: auto;
    aspect-ratio: 1;
    touch-action: manipulation;
  }

  .cover-overlay {
    opacity: 1;
    background: rgba(0, 0, 0, 0.18);
  }

  .cover-overlay .iconfont {
    font-size: 2rem;
  }

  .playlist-info {
    padding: 0.75rem;
  }

  .playlist-name-row {
    min-height: var(--mobile-touch-target);
    margin-bottom: 0.35rem;
  }

  .playlist-name-text {
    font-size: 0.95rem;
    line-height: 1.25;
  }

  .playlist-description {
    display: none;
  }

  .playlist-meta {
    font-size: 0.7rem;
  }

  .playlist-actions {
    justify-content: space-between;
    padding: 0 0.5rem 0.75rem;
    gap: 0;
  }

  .playlist-actions :deep(.t-button) {
    min-width: var(--mobile-touch-target);
    min-height: var(--mobile-touch-target);
    width: var(--mobile-touch-target);
    height: var(--mobile-touch-target);
    touch-action: manipulation;
  }

  .empty-playlists {
    padding: 3rem 1rem;
  }

  .empty-playlists :deep(.t-button) {
    min-height: var(--mobile-touch-target);
    border-radius: var(--mobile-control-radius);
  }
}
</style>

<style>
/* 右键菜单 (unscoped, teleported to body) */
.context-menu {
  position: fixed;
  z-index: 9999;
  min-width: 156px;
  background: var(--td-bg-color-container);
  border-radius: 10px;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15), 0 0 0 1px var(--td-border-level-1-color);
  padding: 4px;
  animation: menuIn 0.15s ease;
}

.context-menu .menu-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 12px;
  font-size: 13px;
  color: var(--td-text-color-primary);
  border-radius: 6px;
  cursor: pointer;
  transition: background 0.12s;
}

.context-menu .menu-item:hover {
  background: var(--td-bg-color-component-hover);
}

.context-menu .menu-item.danger {
  color: var(--td-error-color);
}

.context-menu .menu-item.danger:hover {
  background: var(--td-error-color-light);
}

.context-menu .menu-separator {
  height: 1px;
  background: var(--td-border-level-1-color);
  margin: 4px 8px;
}

@keyframes menuIn {
  from { opacity: 0; transform: translateY(-4px) scale(0.96); }
  to { opacity: 1; transform: translateY(0) scale(1); }
}

/* ============================
   网络歌单导入 — LiquidGlass (unscoped for Teleport)
   ============================ */

.netimport-overlay {
  position: fixed;
  inset: 0;
  z-index: 9999;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(0, 0, 0, 0.45);
  backdrop-filter: blur(var(--glass-blur-overlay)) saturate(140%);
  -webkit-backdrop-filter: blur(var(--glass-blur-overlay)) saturate(140%);
}

.netimport-panel {
  width: min(500px, calc(100vw - 32px));
  max-width: 100%;
  flex: 0 0 auto;
}

.netimport-panel__content {
  position: relative;
  width: 100%;
  overflow: hidden;
  border-radius: 22px;
  padding: 28px;
}

.netimport-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 20px;
}

.netimport-title-group {
  display: flex;
  align-items: center;
  gap: 12px;
  min-width: 0;
}

.netimport-icon {
  width: 42px;
  height: 42px;
  border-radius: 13px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: linear-gradient(135deg, rgba(var(--td-brand-color-rgb, 0, 82, 204), 0.18), rgba(140, 80, 255, 0.12));
  border: 1px solid color-mix(in srgb, var(--td-text-color-primary) 8%, transparent);
  box-shadow: 0 3px 10px color-mix(in srgb, var(--td-brand-color) 12%, transparent);
}

.netimport-icon svg {
  color: var(--td-brand-color, #0052d9);
  filter: drop-shadow(0 0 3px rgba(100, 140, 255, 0.25));
}

.netimport-title-text {
  min-width: 0;
}

.netimport-title {
  font-size: 17px;
  font-weight: 600;
  margin: 0;
  color: var(--td-text-color-primary);
  line-height: 1.2;
}

.netimport-close-btn {
  width: 30px;
  height: 30px;
  border-radius: 9px;
  border: 1px solid color-mix(in srgb, var(--td-text-color-primary) 8%, transparent);
  background: color-mix(in srgb, var(--td-text-color-primary) 3%, transparent);
  color: var(--td-text-color-secondary);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  transition: background-color var(--motion-duration-quick) var(--motion-ease-standard), border-color var(--motion-duration-quick) var(--motion-ease-standard), color var(--motion-duration-quick) var(--motion-ease-standard);
}

.netimport-close-btn:hover {
  background: rgba(255, 80, 80, 0.15);
  border-color: rgba(255, 80, 80, 0.25);
  color: var(--td-error-color, #d54941);
}

.netimport-close-btn .iconfont {
  font-size: 13px;
}

.netimport-back-btn {
  width: 30px;
  height: 30px;
  border-radius: 9px;
  border: 1px solid color-mix(in srgb, var(--td-text-color-primary) 8%, transparent);
  background: color-mix(in srgb, var(--td-text-color-primary) 3%, transparent);
  color: var(--td-text-color-secondary);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  transition: all 0.2s;
}

.netimport-back-btn:hover {
  background: color-mix(in srgb, var(--td-text-color-primary) 10%, transparent);
  color: var(--td-text-color-primary);
}

.netimport-overlay .import-options {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.netimport-overlay .import-option {
  display: flex;
  align-items: center;
  padding: 14px 16px;
  border-radius: 13px;
  border: 1px solid color-mix(in srgb, var(--td-text-color-primary) 8%, transparent);
  background: color-mix(in srgb, var(--td-text-color-primary) 5%, transparent);
  cursor: pointer;
  transition: all 0.2s;
  touch-action: manipulation;
}

.netimport-overlay .import-option:hover {
  background: color-mix(in srgb, var(--td-text-color-primary) 10%, transparent);
  border-color: color-mix(in srgb, var(--td-text-color-primary) 14%, transparent);
}

.netimport-overlay .import-option .option-icon {
  margin-right: 14px;
  flex-shrink: 0;
}

.netimport-overlay .import-option .option-icon .iconfont {
  font-size: 1.3rem;
  color: var(--td-brand-color, #0052d9);
}

.netimport-overlay .import-option .option-content {
  flex: 1;
  min-width: 0;
}

.netimport-overlay .import-option .option-content h4 {
  margin: 0;
  font-size: 14px;
  font-weight: 600;
  color: var(--td-text-color-primary);
  line-height: 1.3;
}

.netimport-overlay .import-option .option-content p {
  margin: 3px 0 0;
  font-size: 12px;
  color: var(--td-text-color-secondary);
  line-height: 1.4;
}

.netimport-overlay .import-option .option-content .coming-soon {
  display: inline-block;
  margin-top: 4px;
  font-size: 10px;
  padding: 1px 6px;
  border-radius: 4px;
  background: rgba(237, 181, 0, 0.12);
  color: var(--td-warning-color, #edb500);
  border: 1px solid rgba(237, 181, 0, 0.2);
}

.netimport-overlay .import-option .option-arrow {
  margin-left: 12px;
  color: var(--td-text-color-placeholder);
  opacity: 0.5;
  flex-shrink: 0;
}

.netimport-overlay .platform-grid {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  margin-bottom: 22px;
}

.netimport-overlay .platform-card {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 8px 12px;
  border-radius: 12px;
  border: 1px solid color-mix(in srgb, var(--td-text-color-primary) 8%, transparent);
  background: color-mix(in srgb, var(--td-text-color-primary) 5%, transparent);
  cursor: pointer;
  transition: all 0.2s ease;
  touch-action: manipulation;
}

.netimport-overlay .platform-card:hover {
  background: color-mix(in srgb, var(--td-text-color-primary) 10%, transparent);
  border-color: color-mix(in srgb, var(--td-text-color-primary) 14%, transparent);
}

.netimport-overlay .platform-card.active {
  background: color-mix(in srgb, var(--td-brand-color) 14%, transparent);
  border-color: color-mix(in srgb, var(--td-brand-color) 35%, transparent);
  box-shadow: 0 0 0 1px color-mix(in srgb, var(--td-brand-color) 20%, transparent);
}

.netimport-overlay .platform-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  flex-shrink: 0;
}

.netimport-overlay .platform-name {
  font-size: 12px;
  font-weight: 500;
  color: var(--td-text-color-primary);
  white-space: nowrap;
}

.netimport-input-group .input-guide {
  margin: 0 0 10px;
  font-size: 13px;
  color: var(--td-text-color-secondary);
  line-height: 1.5;
}

.netimport-overlay .input-row {
  display: flex;
  gap: 8px;
}

.netimport-input {
  flex: 1;
  padding: 10px 14px;
  border-radius: 11px;
  border: 1px solid color-mix(in srgb, var(--td-text-color-primary) 10%, transparent);
  background: color-mix(in srgb, var(--td-bg-color-component) 50%, transparent);
  color: var(--td-text-color-primary);
  font-size: 14px;
  outline: none;
  transition: border-color 0.2s, box-shadow 0.2s, background 0.2s;
  box-sizing: border-box;
  min-width: 0;
}

.netimport-input::placeholder { color: var(--td-text-color-placeholder); }

.netimport-input:focus {
  border-color: var(--td-brand-color, #0052d9);
  background: color-mix(in srgb, var(--td-bg-color-component) 65%, transparent);
  box-shadow: 0 0 0 3px color-mix(in srgb, var(--td-brand-color) 12%, transparent);
}

.netimport-btn {
  padding: 10px 18px;
  border-radius: 11px;
  border: 1px solid color-mix(in srgb, var(--td-text-color-primary) 8%, transparent);
  background: color-mix(in srgb, var(--td-text-color-primary) 6%, transparent);
  color: var(--td-text-color-primary);
  font-size: 13px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.2s ease;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  flex-shrink: 0;
  white-space: nowrap;
}

.netimport-btn.primary {
  background: var(--td-brand-color, #0052d9);
  border-color: var(--td-brand-color, #0052d9);
  color: #fff;
}

.netimport-btn.primary:hover {
  background: var(--td-brand-color-hover, #4787f0);
  border-color: var(--td-brand-color-hover, #4787f0);
  box-shadow: 0 4px 16px color-mix(in srgb, var(--td-brand-color) 30%, transparent);
}

.netimport-fade-enter-active .netimport-panel {
  animation: netimport-in var(--motion-duration-standard) var(--motion-ease-out);
}
.netimport-fade-leave-active .netimport-panel {
  animation: netimport-in var(--motion-duration-quick) var(--motion-ease-out) reverse;
}
.netimport-fade-enter-active,
.netimport-fade-leave-active {
  transition: opacity var(--motion-duration-quick) var(--motion-ease-standard);
}
.netimport-fade-enter-from,
.netimport-fade-leave-to {
  opacity: 0;
}

@keyframes netimport-in {
  from { opacity: 0; transform: scale(0.96) translateY(8px); }
  to { opacity: 1; transform: scale(1) translateY(0); }
}

.netimport-spinner {
  width: 14px;
  height: 14px;
  border: 2px solid rgba(255, 255, 255, 0.3);
  border-top-color: #fff;
  border-radius: 50%;
  animation: netimport-spin 0.6s linear infinite;
  display: inline-block;
}

@keyframes netimport-spin {
  to { transform: rotate(360deg); }
}

.netimport-result {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-top: 14px;
  padding: 10px 14px;
  border-radius: 11px;
  font-size: 13px;
  font-weight: 500;
  line-height: 1.4;
}

.netimport-result.success {
  background: rgba(43, 164, 113, 0.12);
  border: 1px solid rgba(43, 164, 113, 0.2);
  color: var(--td-success-color, #2ba471);
}

.netimport-result.error {
  background: rgba(213, 73, 65, 0.1);
  border: 1px solid rgba(213, 73, 65, 0.18);
  color: var(--td-error-color, #d54941);
}

.netimport-result.warning {
  background: rgba(237, 181, 0, 0.1);
  border: 1px solid rgba(237, 181, 0, 0.18);
  color: var(--td-warning-color, #edb500);
}

.netimport-result svg {
  flex-shrink: 0;
}

.netimport-btn.disabled {
  opacity: 0.6;
  pointer-events: none;
}

.result-fade-enter-active { transition: all 0.3s ease; }
.result-fade-leave-active { transition: all 0.2s ease; }
.result-fade-enter-from, .result-fade-leave-to {
  opacity: 0;
  transform: translateY(-6px);
}

/* ============================
   创建歌单 — LiquidGlass
   ============================ */

.create-pl-overlay {
  position: fixed;
  inset: 0;
  z-index: 9999;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(0, 0, 0, 0.45);
  backdrop-filter: blur(var(--glass-blur-overlay)) saturate(140%);
  -webkit-backdrop-filter: blur(var(--glass-blur-overlay)) saturate(140%);
}

.create-pl-panel {
  width: min(440px, calc(100vw - 32px));
  max-width: 100%;
  flex: 0 0 auto;
}

.create-pl-panel__content {
  position: relative;
  width: 100%;
  overflow: hidden;
  border-radius: 22px;
  padding: 28px;
}

.create-pl-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 22px;
}

.create-pl-title-group {
  display: flex;
  align-items: center;
  gap: 12px;
  min-width: 0;
}

.create-pl-icon {
  width: 42px;
  height: 42px;
  border-radius: 13px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: linear-gradient(135deg, rgba(var(--td-brand-color-rgb, 0, 82, 204), 0.18), rgba(140, 80, 255, 0.12));
  border: 1px solid color-mix(in srgb, var(--td-text-color-primary) 8%, transparent);
  box-shadow: 0 3px 10px color-mix(in srgb, var(--td-brand-color) 12%, transparent);
}

.create-pl-icon svg {
  color: var(--td-brand-color, #0052d9);
  filter: drop-shadow(0 0 3px rgba(100, 140, 255, 0.25));
}

.create-pl-title-text { min-width: 0; }

.create-pl-title {
  font-size: 17px;
  font-weight: 600;
  margin: 0;
  color: var(--td-text-color-primary);
  line-height: 1.2;
}

.create-pl-close-btn {
  width: 30px;
  height: 30px;
  border-radius: 9px;
  border: 1px solid color-mix(in srgb, var(--td-text-color-primary) 8%, transparent);
  background: color-mix(in srgb, var(--td-text-color-primary) 3%, transparent);
  color: var(--td-text-color-secondary);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  transition: all 0.2s;
}

.create-pl-close-btn:hover {
  background: rgba(255, 80, 80, 0.15);
  border-color: rgba(255, 80, 80, 0.25);
  color: var(--td-error-color, #d54941);
}

.create-pl-close-btn .iconfont { font-size: 13px; }

.create-pl-form {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.create-pl-form .form-field {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.create-pl-form .field-label {
  font-size: 12px;
  font-weight: 600;
  color: var(--td-text-color-primary);
  opacity: 0.75;
  letter-spacing: 0.01em;
  padding-left: 2px;
}

.create-pl-form .field-label .required {
  color: var(--td-error-color, #d54941);
}

.create-pl-form .field-input,
.create-pl-form .field-textarea {
  width: 100%;
  padding: 10px 14px;
  border-radius: 11px;
  border: 1px solid color-mix(in srgb, var(--td-text-color-primary) 10%, transparent);
  background: color-mix(in srgb, var(--td-bg-color-component) 50%, transparent);
  color: var(--td-text-color-primary);
  font-size: 14px;
  outline: none;
  transition: border-color 0.2s, box-shadow 0.2s, background 0.2s;
  box-sizing: border-box;
  font-family: inherit;
  resize: vertical;
}

.create-pl-form .field-input::placeholder,
.create-pl-form .field-textarea::placeholder {
  color: var(--td-text-color-placeholder);
}

.create-pl-form .field-input:focus,
.create-pl-form .field-textarea:focus {
  border-color: var(--td-brand-color, #0052d9);
  background: color-mix(in srgb, var(--td-bg-color-component) 65%, transparent);
  box-shadow: 0 0 0 3px color-mix(in srgb, var(--td-brand-color) 12%, transparent);
}

.create-pl-actions {
  display: flex;
  justify-content: flex-end;
  gap: 10px;
  margin-top: 24px;
}

.create-pl-btn {
  min-width: 92px;
  min-height: 38px;
  border-radius: 11px;
  border: 1px solid color-mix(in srgb, var(--td-text-color-primary) 8%, transparent);
  background: color-mix(in srgb, var(--td-text-color-primary) 6%, transparent);
  color: var(--td-text-color-primary);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 7px;
  padding: 9px 18px;
  font-size: 14px;
  font-weight: 600;
  transition: all 0.2s;
}

.create-pl-btn:hover:not(.disabled) {
  background: color-mix(in srgb, var(--td-text-color-primary) 12%, transparent);
  transform: translateY(-1px);
  box-shadow: var(--glass-shadow-control);
}

.create-pl-btn:active:not(.disabled) {
  transform: translateY(0);
}

.create-pl-btn.disabled {
  opacity: 0.45;
  cursor: not-allowed;
}

.create-pl-btn.primary {
  background: var(--td-brand-color, #0052d9);
  border-color: var(--td-brand-color, #0052d9);
  color: #fff;
}

.create-pl-btn.primary:hover:not(.disabled) {
  background: var(--td-brand-color-hover, #4787f0);
  border-color: var(--td-brand-color-hover, #4787f0);
  box-shadow: 0 4px 16px color-mix(in srgb, var(--td-brand-color) 30%, transparent);
}

.create-pl-btn.outline {
  background: transparent;
  border-color: color-mix(in srgb, var(--td-text-color-primary) 8%, transparent);
  color: var(--td-text-color-primary);
}

.create-pl-btn.outline:hover:not(.disabled) {
  background: color-mix(in srgb, var(--td-text-color-primary) 3%, transparent);
  border-color: color-mix(in srgb, var(--td-text-color-primary) 14%, transparent);
}

.create-pl-spinner {
  width: 14px;
  height: 14px;
  border: 2px solid rgba(255, 255, 255, 0.3);
  border-top-color: #fff;
  border-radius: 50%;
  animation: create-pl-spin 0.6s linear infinite;
  display: inline-block;
}

@keyframes create-pl-spin {
  to { transform: rotate(360deg); }
}

.create-pl-fade-enter-active .create-pl-panel {
  animation: create-pl-in var(--motion-duration-standard) var(--motion-ease-out);
}
.create-pl-fade-leave-active .create-pl-panel {
  animation: create-pl-in var(--motion-duration-quick) var(--motion-ease-out) reverse;
}
.create-pl-fade-enter-active,
.create-pl-fade-leave-active {
  transition: opacity var(--motion-duration-quick) var(--motion-ease-standard);
}
.create-pl-fade-enter-from,
.create-pl-fade-leave-to {
  opacity: 0;
}

@keyframes create-pl-in {
  from { opacity: 0; transform: scale(0.96) translateY(8px); }
  to { opacity: 1; transform: scale(1) translateY(0); }
}

/* 网络导入移动端 */
@media (max-width: 768px) {
  .netimport-overlay {
    align-items: flex-end;
    justify-content: center;
    padding: calc(var(--mobile-safe-top) + 12px) var(--mobile-page-gutter) calc(var(--mobile-safe-bottom) + 12px);
    background: var(--mobile-scrim);
    backdrop-filter: saturate(var(--mobile-glass-saturate)) blur(var(--mobile-glass-blur));
    -webkit-backdrop-filter: saturate(var(--mobile-glass-saturate)) blur(var(--mobile-glass-blur));
  }

  .netimport-overlay .overlay-drag-bar {
    display: none;
  }

  .netimport-panel {
    width: min(440px, 100%);
    max-height: min(82dvh, 680px);
    display: flex;
  }

  .netimport-panel .glass {
    height: 100%;
    overflow: hidden;
  }

  .netimport-panel .liquid-glass__content {
    height: 100%;
    overflow: hidden;
  }

  .netimport-panel__content {
    border-radius: var(--mobile-card-radius);
    padding: 20px 16px calc(var(--mobile-safe-bottom) + 16px);
    height: 100%;
    overflow-y: auto;
    -webkit-overflow-scrolling: touch;
  }

  .netimport-panel__content::before {
    content: '';
    display: block;
    width: 38px;
    height: 4px;
    border-radius: 999px;
    background: rgba(120, 120, 128, 0.36);
    margin: -8px auto 12px;
  }

  .netimport-header {
    margin-bottom: 16px;
  }

  .netimport-icon {
    width: 38px;
    height: 38px;
    border-radius: 12px;
  }

  .netimport-close-btn,
  .netimport-back-btn {
    min-width: var(--mobile-touch-target);
    min-height: var(--mobile-touch-target);
    border-radius: var(--mobile-control-radius);
    touch-action: manipulation;
  }

  .netimport-overlay .import-option {
    min-height: 60px;
    padding: 12px 14px;
    border-radius: var(--mobile-card-radius-small);
    touch-action: manipulation;
  }

  .platform-grid {
    gap: 5px;
    margin-bottom: 18px;
  }

  .platform-card {
    padding: 8px 10px;
    border-radius: var(--mobile-control-radius);
    touch-action: manipulation;
  }

  .platform-name {
    font-size: 11px;
  }

  .input-row {
    flex-direction: column;
  }

  .netimport-input {
    min-height: var(--mobile-touch-target);
    font-size: 16px;
  }

  .netimport-btn {
    min-height: var(--mobile-touch-target);
    border-radius: var(--mobile-control-radius);
    touch-action: manipulation;
    width: 100%;
  }

  /* 创建歌单移动端 */
  .create-pl-overlay {
    align-items: flex-end;
    justify-content: center;
    padding: calc(var(--mobile-safe-top) + 12px) var(--mobile-page-gutter) calc(var(--mobile-safe-bottom) + 12px);
    background: var(--mobile-scrim);
    backdrop-filter: saturate(var(--mobile-glass-saturate)) blur(var(--mobile-glass-blur));
    -webkit-backdrop-filter: saturate(var(--mobile-glass-saturate)) blur(var(--mobile-glass-blur));
  }

  .create-pl-overlay .overlay-drag-bar {
    display: none;
  }

  .create-pl-panel {
    width: min(440px, 100%);
    max-height: min(82dvh, 680px);
    display: flex;
  }

  .create-pl-panel .glass {
    height: 100%;
    overflow: hidden;
  }

  .create-pl-panel .liquid-glass__content {
    height: 100%;
    overflow: hidden;
  }

  .create-pl-panel__content {
    border-radius: var(--mobile-card-radius);
    padding: 20px 16px calc(var(--mobile-safe-bottom) + 16px);
    height: 100%;
    overflow-y: auto;
    -webkit-overflow-scrolling: touch;
  }

  .create-pl-panel__content::before {
    content: '';
    display: block;
    width: 38px;
    height: 4px;
    border-radius: 999px;
    background: rgba(120, 120, 128, 0.36);
    margin: -8px auto 12px;
  }

  .create-pl-header {
    margin-bottom: 16px;
  }

  .create-pl-icon {
    width: 38px;
    height: 38px;
    border-radius: 12px;
  }

  .create-pl-close-btn {
    min-width: var(--mobile-touch-target);
    min-height: var(--mobile-touch-target);
    border-radius: var(--mobile-control-radius);
    touch-action: manipulation;
  }

  .create-pl-form .field-input,
  .create-pl-form .field-textarea {
    min-height: var(--mobile-touch-target);
    font-size: 16px;
  }

  .create-pl-actions {
    flex-direction: column-reverse;
    gap: 8px;
    margin-top: 20px;
  }

  .create-pl-btn {
    min-height: var(--mobile-touch-target);
    border-radius: var(--mobile-control-radius);
    touch-action: manipulation;
    width: 100%;
  }
}
</style>
