<script setup lang="ts">
import { computed, nextTick, onMounted, ref, watch } from 'vue'
import { marked } from 'marked'
import DOMPurify from 'dompurify'
import { useSettingsStore } from '@/store/Settings'
import { storeToRefs } from 'pinia'
import { platform as getPlatform } from '@tauri-apps/plugin-os'
import { MessagePlugin } from 'tdesign-vue-next'
import { useAppUpdater } from '@/composables/useAppUpdater'
import currentAuthorSponsorQr from '@/assets/images/current-author-sponsor-qr.jpg'

const { t } = useI18n()

const settingsStore = useSettingsStore()
const { settings } = storeToRefs(settingsStore)

const autoUpdate = ref(settings.value.autoUpdate)
const updateAutoUpdate = () => {
  settingsStore.updateSettings({ autoUpdate: autoUpdate.value })
}

const {
  status: updateStatus,
  currentVersion: appVersion,
  newVersion,
  releaseNotes: updateBody,
  progress: downloadPercent,
  totalBytes,
  downloadedMb,
  totalMb,
  error: errorMsg,
  checkForUpdate,
  downloadAndInstall,
  restartToInstall,
  dismiss: dismissUpdate
} = useAppUpdater()

const isMacOS = ref(false)
try {
  isMacOS.value = getPlatform() === 'macos'
} catch {}

const updateNotesHtml = ref('')
const notesExpanded = ref(false)
const notesContentRef = ref<HTMLElement | null>(null)
const notesOverflowing = ref(false)

const renderUpdateNotes = async () => {
  const notes = updateBody.value.trim()
  if (!notes) {
    updateNotesHtml.value = ''
    notesOverflowing.value = false
    return
  }

  try {
    updateNotesHtml.value = DOMPurify.sanitize(
      await marked(notes, { gfm: true, breaks: true }),
      { USE_PROFILES: { html: true } }
    )
  } catch (error) {
    console.warn('渲染更新日志失败:', error)
    updateNotesHtml.value = ''
  }
}

const measureNotesOverflow = async () => {
  await nextTick()
  const el = notesContentRef.value
  notesOverflowing.value = !!el && el.scrollHeight > el.clientHeight + 4
}

watch(updateBody, () => {
  notesExpanded.value = false
  renderUpdateNotes().then(measureNotesOverflow)
})

watch(notesExpanded, measureNotesOverflow)

onMounted(() => {
  renderUpdateNotes().then(measureNotesOverflow)
})

const handleNotesClick = (event: MouseEvent) => {
  const anchor = (event.target as HTMLElement).closest('a')
  if (!anchor) return
  event.preventDefault()
  const href = anchor.getAttribute('href')
  if (href && /^https?:\/\//i.test(href)) {
    openLink(href)
  }
}

const handleCheckUpdate = async () => {
  await checkForUpdate()
}

const handleUpdateInstall = async () => {
  await downloadAndInstall()
}

const restartNow = async () => {
  try {
    await restartToInstall()
  } catch (e) {
    console.error('重启应用以安装更新失败:', e)
    MessagePlugin.error(t('settings.about.restartFailed'))
  }
}

const openLink = async (url: string) => {
  try {
    const { openUrl } = await import('@tauri-apps/plugin-opener')
    await openUrl(url)
  } catch { window.open(url, '_blank') }
}

const aboutIntroHtml = computed(() => {
  const link1 = `<a class="about-link" data-url="https://github.com/timeshiftsauce/CeruMusic">CeruMusic</a>`
  const link2 = '<strong>时迁酱</strong>'
  const link3 = `<a class="about-link" data-url="https://ceru.docs.shiqianjiang.cn/">CeruMusic</a>`
  return t('settings.about.aboutIntro', [link1, link2, link3])
})

const handleAboutClick = (e: MouseEvent) => {
  const target = e.target as HTMLElement
  if (target.classList.contains('about-link')) {
    const url = target.dataset.url
    if (url) openLink(url)
  }
}
</script>

<template>
  <div class="settings-section">
    <div class="setting-group app-header-group">
      <div class="app-header">
        <div class="app-logo"><img src="/icon.png" alt="Mio Music" /></div>
        <div class="app-info">
          <h2 class="app-name">Mio Music</h2>
          <span class="app-version">v{{ appVersion }}</span>
        </div>
      </div>
      <p class="app-description">{{ t('settings.about.description') }}</p>
    </div>

    <div id="about-version" class="setting-group">
      <h3>{{ t('settings.about.versionInfo') }}</h3>
      <div class="version-section">
        <div class="update-actions">
          <div class="update-option">
            <t-switch v-model:value="autoUpdate" @change="updateAutoUpdate"></t-switch>
            <div>{{ t('settings.about.checkUpdateOnStart') }}</div>
          </div>
          <t-button
            class="update-check-button"
            theme="primary"
            :disabled="updateStatus === 'checking' || updateStatus === 'downloading'"
            :aria-busy="updateStatus === 'checking'"
            @click="handleCheckUpdate"
          >
            <span class="update-check-content">
              <span v-if="updateStatus === 'checking'" class="update-check-spinner" aria-hidden="true" />
              {{ updateStatus === 'checking' ? t('settings.about.checking') : t('settings.about.checkUpdate') }}
            </span>
          </t-button>
        </div>

        <!-- 已是最新版本 -->
        <div v-if="updateStatus === 'up-to-date'" class="update-card success">
          <span class="update-icon">✓</span> {{ t('settings.about.upToDate') }}
        </div>

        <!-- 发现新版本 -->
        <div v-if="updateStatus === 'available'" class="update-card">
          <div class="update-header">
            <span class="update-icon new">↑</span>
            <div class="update-title">
              <span>{{ t('settings.about.newVersionFound') }}</span>
              <span class="version-badge">v{{ newVersion }}</span>
            </div>
          </div>

          <div v-if="updateNotesHtml" class="release-notes">
            <div class="release-notes-heading">{{ t('settings.about.releaseNotes') }}</div>
            <div
              ref="notesContentRef"
              class="markdown-content"
              :class="{ collapsed: !notesExpanded && notesOverflowing }"
              v-html="updateNotesHtml"
              @click="handleNotesClick"
            ></div>
            <div v-if="!notesExpanded && notesOverflowing" class="release-notes-fade" aria-hidden="true"></div>
          </div>

          <button
            v-if="notesOverflowing || notesExpanded"
            type="button"
            class="notes-toggle"
            @click="notesExpanded = !notesExpanded"
          >
            {{ notesExpanded ? t('settings.about.hideFullNotes') : t('settings.about.showFullNotes') }}
          </button>

          <div class="update-actions-row">
            <!-- macOS: 引导手动下载 -->
            <t-button
              v-if="isMacOS"
              theme="primary"
              @click="openLink('https://github.com/Mio888888/Mio-Music/releases/latest')"
            >
              {{ t('settings.about.goToGithub') }}
            </t-button>
            <!-- Windows/Linux: 自动下载安装 -->
            <t-button
              v-else
              theme="primary"
              @click="handleUpdateInstall"
            >
              {{ t('settings.about.downloadAndInstall') }}
            </t-button>
            <t-button variant="text" @click="dismissUpdate">{{ t('settings.about.remindLater') }}</t-button>
          </div>
        </div>

        <!-- 下载中 -->
        <div v-if="updateStatus === 'downloading'" class="update-card">
          <div class="update-header">
            <span class="update-icon downloading">↓</span>
            <span>{{ t('settings.about.downloading') }} v{{ newVersion }}...</span>
          </div>
          <t-progress :percentage="downloadPercent" theme="plump" :label="`${downloadPercent}%`" />
          <div v-if="totalBytes > 0" class="progress-detail">
            {{ downloadedMb }} MB / {{ totalMb }} MB
          </div>
        </div>

        <!-- 下载完成 -->
        <div v-if="updateStatus === 'downloaded'" class="update-card downloaded">
          <div class="update-header">
            <span class="update-icon">✓</span>
            <span>{{ t('settings.about.updateDownloaded') }}</span>
          </div>
          <div v-if="newVersion" class="downloaded-version">{{ t('settings.about.readyVersion') }} v{{ newVersion }}</div>
          <t-button theme="primary" @click="restartNow">{{ t('settings.about.restartNow') }}</t-button>
        </div>

        <!-- 错误 -->
        <div v-if="updateStatus === 'error'" class="update-card error">
          <span class="update-icon err">!</span> {{ errorMsg }}
        </div>
      </div>
    </div>

    <div id="about-legal" class="setting-group">
      <h3>{{ t('settings.about.legalNotice') }}</h3>
      <div class="legal-notice">
        <div class="notice-item"><h4>{{ t('settings.about.dataResponsibility') }}</h4><p>{{ t('settings.about.dataResponsibilityDesc') }}</p></div>
        <div class="notice-item"><h4>{{ t('settings.about.copyrightCompliance') }}</h4><p>{{ t('settings.about.copyrightComplianceDesc') }}</p></div>
        <div class="notice-item"><h4>{{ t('settings.about.usageRestriction') }}</h4><p>{{ t('settings.about.usageRestrictionDesc') }}</p></div>
      </div>
      <h3 style="margin-top: 2rem">{{ t('settings.about.aboutUs') }}</h3>
      <div class="about-us">
        <p class="about-intro" v-html="aboutIntroHtml" @click="handleAboutClick"></p>

        <div class="support-card current-author">
          <div class="support-copy">
            <div class="support-title">
              <span class="support-icon" aria-hidden="true">♥</span>
              {{ t('settings.about.supportCurrentAuthorTitle') }}
            </div>
            <p class="support-text">{{ t('settings.about.supportCurrentAuthorText') }}</p>
            <ul class="support-points">
              <li>{{ t('settings.about.supportPointMaintenance') }}</li>
              <li>{{ t('settings.about.supportPointCompatibility') }}</li>
              <li>{{ t('settings.about.supportPointTransparency') }}</li>
            </ul>
          </div>
          <figure class="support-qr">
            <img :src="currentAuthorSponsorQr" :alt="t('settings.about.currentSponsorImageAlt')" />
            <figcaption>{{ t('settings.about.supportQrCaption') }}</figcaption>
          </figure>
        </div>

        <div class="sponsor-card">
          <p class="sponsor-text">{{ t('settings.about.sponsorText') }} ☕</p>
          <div class="sponsor-qr">
            <img
              src="https://oss.shiqianjiang.cn/storage/default/20250907/image-2025082711173bb1bba3608ef15d0e1fb485f80f29c728186.png"
              :alt="t('settings.about.sponsorImageAlt')"
            />
          </div>
          <p class="sponsor-hint">{{ t('settings.about.sponsorHint') }}</p>
        </div>
      </div>
    </div>
  </div>
</template>

<style lang="scss" scoped>
.settings-section { animation: fadeInUp 0.4s ease-out; animation-fill-mode: both; }
.setting-group {
  background: var(--settings-group-bg, var(--td-bg-color-container));
  border-radius: 0.75rem; padding: 1.5rem; margin-bottom: 1.5rem;
  border: 1px solid var(--settings-group-border, var(--td-border-level-1-color));
  box-shadow: 0 1px 3px var(--settings-group-shadow);
  animation: fadeInUp 0.4s ease-out; animation-fill-mode: both;
  h3 { margin: 0 0 0.5rem; font-size: 1.125rem; font-weight: 600; color: var(--td-text-color-primary); }
  > p { margin: 0 0 1.5rem; color: var(--td-text-color-secondary); font-size: 0.875rem; }
}
.app-header-group {
  display: flex; flex-direction: column; align-items: center; text-align: center; padding: 2.5rem 1.5rem;
  .app-header {
    display: flex; align-items: center; gap: 1rem; margin-bottom: 1rem;
    .app-logo {
      width: 3.5rem; height: 3.5rem; flex-shrink: 0;
      img { width: 100%; height: 100%; object-fit: contain; border-radius: 0.75rem; }
    }
    .app-info {
      display: flex; align-items: baseline; gap: 0.75rem;
      .app-name { margin: 0; font-size: 1.75rem; font-weight: 800; color: var(--td-text-color-primary); letter-spacing: -0.5px; }
    }
  }
  .app-version {
    background: var(--td-brand-color-1); color: var(--td-brand-color-6);
    padding: 0.2rem 0.6rem; border-radius: 1rem; font-size: 0.75rem; font-weight: 600;
    border: 1px solid var(--td-brand-color-3);
  }
  .app-description {
    margin: 0; color: var(--td-text-color-secondary); line-height: 1.6; font-size: 0.9rem; max-width: 420px;
  }
}
.version-section {
  .update-actions {
    display: flex; justify-content: space-between; align-items: center;
    .update-option {
      display: flex; align-items: center; gap: 0.5rem;
      color: var(--td-text-color-primary);
    }
  }
}
.update-check-button {
  .update-check-content {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    line-height: 1;
  }

  .update-check-spinner {
    width: 14px;
    height: 14px;
    margin-right: 6px;
    border: 2px solid color-mix(in srgb, currentColor 30%, transparent);
    border-top-color: currentColor;
    border-radius: 50%;
    will-change: transform;
    animation: update-check-spin 1s linear infinite;
  }
}
.update-card {
  margin-top: 0.75rem; padding: 1rem 1.25rem;
  background: var(--td-bg-color-page); border: 1px solid var(--td-border-level-1-color);
  border-radius: 0.75rem; font-size: 0.875rem;
  display: flex; flex-direction: column; gap: 0.5rem;

  &.success {
    border-color: var(--td-success-color);
    color: var(--td-success-color);
    flex-direction: row; align-items: center; gap: 0.5rem;
  }

  &.downloaded {
    border-color: var(--td-success-color);
  }
  &.error {
    border-color: var(--td-error-color);
    color: var(--td-error-color);
    flex-direction: row; align-items: center; gap: 0.5rem;
  }
}
.update-icon {
  display: inline-flex; align-items: center; justify-content: center;
  width: 1.25rem; height: 1.25rem; border-radius: 50%; font-size: 0.75rem;
  font-weight: 700; flex-shrink: 0;
  background: var(--td-success-color); color: var(--td-text-color-anti);

  &.new { background: var(--td-brand-color); }
  &.downloading { background: var(--td-brand-color); will-change: opacity, transform; animation: pulse 1.5s infinite; }
  &.err { background: var(--td-error-color); }
}
.update-header {
  display: flex; align-items: center; gap: 0.5rem;

  .update-title {
    display: flex; align-items: center; flex-wrap: wrap; gap: 0.45rem;
    color: var(--td-text-color-primary);
    font-weight: 600;
  }

  .version-badge {
    display: inline-flex; align-items: center;
    padding: 0.15rem 0.5rem;
    border-radius: 999px;
    background: var(--td-brand-color-1);
    border: 1px solid var(--td-brand-color-3);
    color: var(--td-brand-color-6);
    font-size: 0.75rem;
    font-weight: 700;
    line-height: 1.2;
  }
}

.release-notes {
  position: relative;
  margin: 0.25rem 0 0.25rem 1.75rem;
  border: 1px solid var(--td-border-level-1-color);
  border-radius: 0.6rem;
  background: var(--td-bg-color-container);
  overflow: hidden;
}

.release-notes-heading {
  position: sticky;
  top: 0;
  z-index: 1;
  padding: 0.65rem 0.85rem;
  background: color-mix(in srgb, var(--td-brand-color) 7%, var(--td-bg-color-container));
  border-bottom: 1px solid var(--td-border-level-1-color);
  color: var(--td-text-color-primary);
  font-size: 0.78rem;
  font-weight: 650;
  letter-spacing: 0.02em;
}

.markdown-content {
  max-height: 180px;
  overflow-y: auto;
  overscroll-behavior: contain;
  padding: 0.75rem 0.9rem 0.9rem;
  color: var(--td-text-color-secondary);
  font-size: 0.8rem;
  line-height: 1.55;

  &.collapsed { overflow: hidden; }

  :deep(h1), :deep(h2), :deep(h3), :deep(h4), :deep(h5), :deep(h6) {
    margin: 0.85rem 0 0.45rem;
    color: var(--td-text-color-primary);
    font-size: 0.86rem;
    font-weight: 700;
    line-height: 1.35;
  }

  :deep(h1:first-child), :deep(h2:first-child), :deep(h3:first-child),
  :deep(p:first-child), :deep(ul:first-child), :deep(ol:first-child) {
    margin-top: 0;
  }

  :deep(h1:last-child), :deep(h2:last-child), :deep(h3:last-child),
  :deep(p:last-child), :deep(ul:last-child), :deep(ol:last-child) {
    margin-bottom: 0;
  }

  :deep(p) { margin: 0.4rem 0; }

  :deep(ul), :deep(ol) {
    margin: 0.4rem 0;
    padding-left: 1.35rem;
  }

  :deep(li) {
    margin: 0.28rem 0;
    line-height: 1.5;
  }

  :deep(li::marker) { color: var(--td-brand-color); }

  :deep(strong) { color: var(--td-text-color-primary); }

  :deep(code) {
    padding: 0.1em 0.32em;
    border-radius: 0.3rem;
    background: color-mix(in srgb, var(--td-text-color-primary) 7%, transparent);
    color: inherit;
    font-size: 0.94em;
  }

  :deep(blockquote) {
    margin: 0.5rem 0;
    padding: 0.35rem 0.7rem;
    border-left: 3px solid var(--td-brand-color);
    background: color-mix(in srgb, var(--td-brand-color) 6%, transparent);
    color: var(--td-text-color-secondary);
  }

  :deep(a) {
    color: var(--td-brand-color);
    text-decoration: none;
    font-weight: 550;
    cursor: pointer;

    &:hover { text-decoration: underline; text-underline-offset: 2px; }
  }
}

.release-notes-fade {
  position: absolute;
  right: 1px;
  bottom: 1px;
  left: 1px;
  height: 48px;
  pointer-events: none;
  background: linear-gradient(
    to bottom,
    transparent,
    color-mix(in srgb, var(--td-bg-color-container) 92%, transparent)
  );
}

.notes-toggle {
  align-self: flex-start;
  margin: -0.15rem 0 0 1.75rem;
  padding: 0.2rem 0;
  border: 0;
  background: transparent;
  color: var(--td-brand-color);
  font-size: 0.76rem;
  font-weight: 600;
  cursor: pointer;
}

.downloaded-version {
  color: var(--td-text-color-secondary);
  font-size: 0.78rem;
  font-weight: 600;
  margin-top: -0.15rem;
}
.update-actions-row {
  display: flex; gap: 0.5rem; margin-top: 0.25rem; padding-left: 1.75rem;
}
.progress-detail {
  font-size: 0.75rem; color: var(--td-text-color-secondary); margin-top: 0.25rem;
}
.tech-stack {
  display: grid; grid-template-columns: repeat(auto-fit, minmax(250px, 1fr)); gap: 0.75rem;
  .tech-item {
    display: flex; justify-content: space-between; align-items: center;
    padding: 0.75rem; background: var(--td-bg-color-page); border-radius: 0.5rem;
    border: 1px solid var(--td-border-level-1-color); transition: 0.3s; gap: 1rem;
    .tech-name { font-weight: 600; flex-shrink: 0; color: var(--td-text-color-primary); }
    .tech-desc { font-size: 0.875rem; color: var(--td-text-color-secondary); }
    &.link:hover { background-color: var(--td-brand-color-1); border-color: var(--td-brand-color); }
  }
}

.support-card {
  margin: 0 0 1.25rem;
  padding: 1.1rem;
  border: 1px solid color-mix(in srgb, var(--td-brand-color) 32%, transparent);
  border-radius: 0.75rem;
  background:
    linear-gradient(135deg, color-mix(in srgb, var(--td-brand-color) 10%, transparent), transparent 58%),
    var(--td-bg-color-container);
  box-shadow: 0 6px 18px color-mix(in srgb, var(--td-brand-color) 8%, transparent);
}

.support-card.current-author {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto;
  gap: 1.25rem;
  align-items: center;
}

.support-copy {
  min-width: 0;
}

.support-title {
  display: flex;
  align-items: center;
  gap: 0.45rem;
  margin-bottom: 0.55rem;
  color: var(--td-text-color-primary);
  font-size: 1rem;
  font-weight: 700;
}

.support-icon {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 1.35rem;
  height: 1.35rem;
  border-radius: 50%;
  flex-shrink: 0;
  background: color-mix(in srgb, var(--td-error-color) 16%, transparent);
  color: var(--td-error-color);
  font-size: 0.7rem;
}

.support-text,
.sponsor-text {
  margin: 0 0 1rem;
  color: var(--td-text-color-secondary);
  font-size: 0.85rem;
  line-height: 1.5;
  text-align: left;
}

.support-points {
  margin: 0;
  padding-left: 1.1rem;
  color: var(--td-text-color-secondary);
  font-size: 0.8rem;
  line-height: 1.65;

  li::marker { color: var(--td-brand-color); }
}

.support-qr {
  width: 168px;
  margin: 0;
  text-align: center;

  img {
    width: 100%;
    aspect-ratio: 1;
    object-fit: cover;
    border-radius: 0.7rem;
    border: 1px solid var(--td-border-level-1-color);
  }

  figcaption {
    margin-top: 0.5rem;
    color: var(--td-text-color-disabled);
    font-size: 0.72rem;
    line-height: 1.3;
  }
}
.legal-notice {
  .notice-item {
    margin-bottom: 1.5rem; &:last-child { margin-bottom: 0; }
    h4 { margin: 0 0 0.5rem; font-size: 0.875rem; font-weight: 600; color: var(--td-text-color-primary); }
    p { margin: 0; font-size: 0.875rem; color: var(--td-text-color-secondary); line-height: 1.5; }
  }
}

.support-card.current-author {
  @media (max-width: 768px) {
    grid-template-columns: 1fr;
    justify-items: center;
    text-align: left;

    .support-qr { width: min(58vw, 180px); }
  }
}

.about-us {
  margin-top: 0.5rem;
  .about-intro {
    margin: 0 0 1.25rem; color: var(--td-text-color-secondary); font-size: 0.9rem; line-height: 1.7;
    .about-link {
      color: var(--td-brand-color); cursor: pointer; font-weight: 500;
      text-decoration: underline; text-decoration-color: var(--td-brand-color-3);
      text-underline-offset: 2px;
      &:hover { text-decoration-color: var(--td-brand-color); }
    }
  }
  .sponsor-card {
    background: var(--td-bg-color-page); border: 1px solid var(--td-border-level-1-color);
    border-radius: 0.75rem; padding: 1.25rem; display: flex; flex-direction: column; align-items: center;
    .sponsor-text { margin: 0 0 1rem; color: var(--td-text-color-secondary); font-size: 0.85rem; line-height: 1.5; text-align: center; }
    .sponsor-qr {
      width: 180px; height: 180px; border-radius: 0.75rem; overflow: hidden;
      border: 1px solid var(--td-border-level-1-color);
      img { width: 100%; height: 100%; object-fit: cover; }
    }
    .sponsor-hint { margin: 0.75rem 0 0; font-size: 0.75rem; color: var(--td-text-color-disabled); }
  }
}
@media (max-width: 768px) {
  .setting-group {
    padding: 14px;
    margin-bottom: 10px;

    h3 {
      font-size: 16px;
      line-height: 1.35;
    }
  }

  .app-header-group {
    padding: 22px 14px;

    .app-header {
      flex-direction: column;
      gap: 10px;
      margin-bottom: 10px;
    }

    .app-logo {
      width: 48px;
      height: 48px;
    }

    .app-info {
      flex-direction: column;
      align-items: center;
      gap: 6px;

      .app-name {
        font-size: 24px;
      }
    }

    .app-description {
      font-size: 13px;
    }
  }

  .version-section .update-actions {
    flex-direction: column;
    gap: 10px;
    align-items: stretch;

    .update-option {
      justify-content: space-between;
      padding: 10px 12px;
      border-radius: 8px;
      background: var(--settings-feature-bg, var(--td-bg-color-container));
      border: 1px solid var(--settings-feature-border, var(--td-border-level-1-color));
      color: var(--td-text-color-primary);
    }
  }

  .update-card {
    padding: 12px;
  }

  .release-notes {
    margin-left: 0;

    .markdown-content {
      max-height: min(46vh, 220px);
    }
  }

  .notes-toggle {
    margin-left: 0;
  }

  .update-actions-row {
    padding-left: 0;
    flex-wrap: wrap;
  }

  .tech-stack {
    grid-template-columns: 1fr;
    gap: 8px;

    .tech-item {
      align-items: flex-start;
      flex-direction: column;
      gap: 4px;
      padding: 12px;
    }
  }

  .legal-notice .notice-item {
    margin-bottom: 14px;

    p {
      font-size: 13px;
      line-height: 1.55;
    }
  }

  .about-us {
    .about-intro {
      font-size: 13px;
      line-height: 1.6;
    }

    .sponsor-card {
      padding: 14px;

      .sponsor-qr {
        width: min(58vw, 180px);
        height: min(58vw, 180px);
      }
    }
  }
}
@keyframes fadeInUp { from { opacity: 0; transform: translateY(20px); } to { opacity: 1; transform: translateY(0); } }
@keyframes pulse { 0%, 100% { opacity: 1; } 50% { opacity: 0.6; } }
@keyframes update-check-spin { to { transform: rotate(360deg); } }
</style>
