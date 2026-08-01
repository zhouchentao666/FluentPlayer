<script setup lang="ts">
import { ref, computed } from 'vue'
import type { Song } from '../types'
import { resolveOnlineUrl, QUALITY_SHORT, QUALITY_LADDER, type Quality } from '@online/player'
import { getBuiltinLyric } from '@online/lib/lyric'
import { OpenMusicFolder } from '../bridge/app'
import { useDownloadConfig, type LyricFormat } from '../composables/useDownloadConfig'
import { toast } from '../composables/useToast'

const props = defineProps<{
  songs: Song[]
}>()

const emit = defineEmits<{
  (e: 'cancel'): void
  (e: 'done'): void
}>()

const cfg = useDownloadConfig()
const busy = ref(false)
const progress = ref(0)

const lyricFormats: { id: LyricFormat; label: string }[] = [
  { id: 'lrc', label: 'LRC（标准）' },
  { id: 'yrc', label: '逐字 LRC（YRC）' },
  { id: 'ttml', label: 'TTML' },
]

const availableQualities = computed<Quality[]>(() => {
  // 取交集：所有歌曲都支持的音质才可选
  return QUALITY_LADDER.filter((q) =>
    props.songs.every((s) => !s.online || s.online.meta.qualitys.some((x) => x.type === q)),
  )
})

function sanitize(name: string): string {
  return name.replace(/[\\/:*?"<>|]/g, '_').trim().slice(0, 80) || '未知'
}

/** 根据音质选择文件后缀（flac / flac24bit 保持 flac 后缀，其余为 mp3）。 */
function extFor(quality: Quality): string {
  if (quality === 'flac' || quality === 'flac24bit') return 'flac'
  return 'mp3'
}

async function pickDir() {
  try {
    const dir = await OpenMusicFolder()
    if (dir) cfg.value.saveDir = dir
  } catch {
    /* 用户取消 */
  }
}

async function start() {
  if (!cfg.value.saveDir) {
    toast('请先选择下载位置', 'error')
    return
  }
  const list = props.songs.filter((s) => s.online)
  if (list.length === 0) {
    toast('没有可下载的在线歌曲', 'error')
    return
  }
  busy.value = true
  progress.value = 0
  let ok = 0
  let failed = 0
  for (let i = 0; i < list.length; i++) {
    const s = list[i]
    progress.value = Math.round(((i + 1) / list.length) * 100)
    try {
      const meta = s.online!
      const res = await resolveOnlineUrl(meta, cfg.value.quality)
      const url = res.url
      const ext = extFor(cfg.value.quality)
      const filename = `${sanitize(meta.singer)} - ${sanitize(meta.name)}.${ext}`
      const dest = `${cfg.value.saveDir.replace(/[\\/]$/, '')}/${filename}`

      let lyricText: string | null = null
      if (cfg.value.embedLyric) {
        try {
          const lr = await getBuiltinLyric(meta)
          if (lr) {
            if (cfg.value.lyricFormat === 'yrc') lyricText = lr.lxlyric || lr.lyric
            else if (cfg.value.lyricFormat === 'ttml') lyricText = lr.tlyric || lr.lyric
            else lyricText = lr.lyric
          }
        } catch {
          lyricText = null
        }
      }

      const { DownloadSong } = await import('../bridge/app')
      await DownloadSong(
        url,
        dest,
        null,
        {
          title: meta.name,
          artist: meta.singer,
          album: meta.albumName || '',
          album_artist: meta.singer,
          track_number: null,
          genre: null,
          year: null,
          cover_url: meta.meta.picUrl ?? null,
          lyric: lyricText,
        },
        cfg.value.embedMetadata,
        cfg.value.embedLyric,
        cfg.value.embedCover,
      )
      ok++
    } catch (err) {
      failed++
      console.error('下载失败', err)
    }
  }
  busy.value = false
  if (failed === 0) toast(`已下载 ${ok} 首`, 'success')
  else toast(`下载完成：${ok} 成功 / ${failed} 失败`, 'error')
  emit('done')
}

function setQuality(q: Quality) {
  cfg.value.quality = q
}
</script>

<template>
  <div class="download-mask" @click.self="!busy && emit('cancel')">
    <div class="download-card">
      <div class="dl-title">下载设置</div>
      <div class="dl-sub">共 {{ songs.filter((s) => s.online).length }} 首在线歌曲</div>

      <div class="dl-row">
        <span class="dl-label">下载位置</span>
        <div class="dl-dir">
          <span class="dl-path">{{ cfg.saveDir || '未选择（将使用默认音乐目录）' }}</span>
          <button class="dl-btn" :disabled="busy" @click="pickDir">选择目录</button>
        </div>
      </div>

      <div class="dl-row">
        <span class="dl-label">音质</span>
        <div class="dl-seg">
          <button
            v-for="q in availableQualities"
            :key="q"
            class="dl-seg-item"
            :class="{ active: cfg.quality === q }"
            :disabled="busy"
            @click="setQuality(q)"
          >
            {{ QUALITY_SHORT[q] }}
          </button>
          <span v-if="availableQualities.length === 0" class="dl-hint">无可用音质</span>
        </div>
      </div>

      <div class="dl-row">
        <span class="dl-label">内嵌</span>
        <div class="dl-checks">
          <label class="dl-check">
            <input type="checkbox" v-model="cfg.embedMetadata" :disabled="busy" /> 元数据
          </label>
          <label class="dl-check">
            <input type="checkbox" v-model="cfg.embedCover" :disabled="busy" /> 封面
          </label>
          <label class="dl-check">
            <input type="checkbox" v-model="cfg.embedLyric" :disabled="busy" /> 歌词
          </label>
        </div>
      </div>

      <div v-if="cfg.embedLyric" class="dl-row">
        <span class="dl-label">歌词格式</span>
        <div class="dl-seg">
          <button
            v-for="f in lyricFormats"
            :key="f.id"
            class="dl-seg-item"
            :class="{ active: cfg.lyricFormat === f.id }"
            :disabled="busy"
            @click="cfg.lyricFormat = f.id"
          >
            {{ f.label }}
          </button>
        </div>
      </div>

      <div v-if="busy" class="dl-progress">
        <div class="dl-bar" :style="{ width: progress + '%' }"></div>
        <span class="dl-pct">{{ progress }}%</span>
      </div>

      <div class="dl-actions">
        <button class="dl-cancel" :disabled="busy" @click="emit('cancel')">取消</button>
        <button class="dl-confirm" :disabled="busy" @click="start">开始下载</button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.download-mask {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.45);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 100;
}
.download-card {
  width: 460px;
  max-width: 92%;
  background: var(--fluent-bg-glass);
  backdrop-filter: blur(28px) saturate(160%);
  border: 1px solid var(--fluent-border);
  border-radius: 18px;
  padding: 22px;
  box-shadow: 0 20px 60px rgba(0, 0, 0, 0.4);
}
.dl-title {
  font-size: 18px;
  font-weight: 800;
  color: var(--fluent-text);
}
.dl-sub {
  font-size: 13px;
  color: var(--fluent-text-secondary);
  margin: 4px 0 16px;
}
.dl-row {
  margin-bottom: 14px;
}
.dl-label {
  display: block;
  font-size: 12px;
  color: var(--fluent-text-secondary);
  margin-bottom: 6px;
}
.dl-dir {
  display: flex;
  align-items: center;
  gap: 8px;
}
.dl-path {
  flex: 1;
  min-width: 0;
  font-size: 12px;
  color: var(--fluent-text);
  background: var(--fluent-bg-card);
  padding: 8px 10px;
  border-radius: 8px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.dl-btn {
  flex-shrink: 0;
  border: none;
  background: var(--fluent-bg-active);
  color: var(--fluent-text);
  padding: 8px 12px;
  border-radius: 8px;
  cursor: pointer;
  font-size: 13px;
}
.dl-seg {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}
.dl-seg-item {
  border: 1px solid var(--fluent-border);
  background: var(--fluent-bg-card);
  color: var(--fluent-text);
  padding: 7px 12px;
  border-radius: 8px;
  cursor: pointer;
  font-size: 12px;
}
.dl-seg-item.active {
  background: var(--fluent-bg-active);
  border-color: transparent;
  font-weight: 700;
}
.dl-checks {
  display: flex;
  gap: 16px;
}
.dl-check {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 13px;
  color: var(--fluent-text);
  cursor: pointer;
}
.dl-hint {
  font-size: 12px;
  color: var(--fluent-text-secondary);
}
.dl-progress {
  position: relative;
  height: 22px;
  background: var(--fluent-bg-card);
  border-radius: 11px;
  overflow: hidden;
  margin-bottom: 14px;
}
.dl-bar {
  position: absolute;
  inset: 0 auto 0 0;
  background: var(--fluent-bg-active);
  transition: width 0.2s ease;
}
.dl-pct {
  position: absolute;
  inset: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 11px;
  color: var(--fluent-text);
}
.dl-actions {
  display: flex;
  justify-content: flex-end;
  gap: 10px;
}
.dl-cancel,
.dl-confirm {
  border: none;
  padding: 10px 20px;
  border-radius: 10px;
  cursor: pointer;
  font-size: 14px;
}
.dl-cancel {
  background: var(--fluent-bg-card);
  color: var(--fluent-text-secondary);
}
.dl-confirm {
  background: var(--fluent-bg-active);
  color: var(--fluent-text);
  font-weight: 700;
}
</style>
