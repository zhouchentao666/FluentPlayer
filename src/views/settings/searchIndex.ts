import i18n from '@/locales'

export interface SearchItem {
  id: string
  category: string
  title: string
  description: string
  keywords: string[]
}

const t = i18n.global.t

export function getSearchIndex(): SearchItem[] {
  return [
    // 外观设置
    { id: 'appearance-titlebar', category: 'appearance', title: t('common.searchIndex.appearanceTitlebar.title'), description: t('common.searchIndex.appearanceTitlebar.description'), keywords: t('common.searchIndex.appearanceTitlebar.keywords') as unknown as string[] },
    { id: 'appearance-close-behavior', category: 'appearance', title: t('common.searchIndex.appearanceCloseBehavior.title'), description: t('common.searchIndex.appearanceCloseBehavior.description'), keywords: t('common.searchIndex.appearanceCloseBehavior.keywords') as unknown as string[] },
    { id: 'appearance-lyric-font', category: 'appearance', title: t('common.searchIndex.appearanceLyricFont.title'), description: t('common.searchIndex.appearanceLyricFont.description'), keywords: t('common.searchIndex.appearanceLyricFont.keywords') as unknown as string[] },
    { id: 'appearance-desktop-lyric', category: 'appearance', title: t('common.searchIndex.appearanceDesktopLyric.title'), description: t('common.searchIndex.appearanceDesktopLyric.description'), keywords: t('common.searchIndex.appearanceDesktopLyric.keywords') as unknown as string[] },

    // AI 功能
    { id: 'ai-api-config', category: 'ai', title: t('common.searchIndex.aiApiConfig.title'), description: t('common.searchIndex.aiApiConfig.description'), keywords: t('common.searchIndex.aiApiConfig.keywords') as unknown as string[] },
    { id: 'ai-floatball', category: 'ai', title: t('common.searchIndex.aiFloatball.title'), description: t('common.searchIndex.aiFloatball.description'), keywords: t('common.searchIndex.aiFloatball.keywords') as unknown as string[] },

    // 播放设置
    { id: 'playback-playlist', category: 'playlist', title: t('common.searchIndex.playbackPlaylist.title'), description: t('common.searchIndex.playbackPlaylist.description'), keywords: t('common.searchIndex.playbackPlaylist.keywords') as unknown as string[] },
    { id: 'playback-audio-output', category: 'playlist', title: t('common.searchIndex.playbackAudioOutput.title'), description: t('common.searchIndex.playbackAudioOutput.description'), keywords: t('common.searchIndex.playbackAudioOutput.keywords') as unknown as string[] },
    { id: 'playback-performance', category: 'playlist', title: t('common.searchIndex.playbackPerformance.title'), description: t('common.searchIndex.playbackPerformance.description'), keywords: t('common.searchIndex.playbackPerformance.keywords') as unknown as string[] },

    // 音效设置
    { id: 'effects-equalizer', category: 'effects', title: t('common.searchIndex.playbackEqualizer.title'), description: t('common.searchIndex.playbackEqualizer.description'), keywords: t('common.searchIndex.playbackEqualizer.keywords') as unknown as string[] },
    { id: 'effects-audio-effect', category: 'effects', title: t('common.searchIndex.playbackAudioEffect.title'), description: t('common.searchIndex.playbackAudioEffect.description'), keywords: t('common.searchIndex.playbackAudioEffect.keywords') as unknown as string[] },

    // 音乐源
    { id: 'music-source', category: 'music', title: t('common.searchIndex.musicSource.title'), description: t('common.searchIndex.musicSource.description'), keywords: t('common.searchIndex.musicSource.keywords') as unknown as string[] },
    { id: 'music-quality', category: 'music', title: t('common.searchIndex.musicQuality.title'), description: t('common.searchIndex.musicQuality.description'), keywords: t('common.searchIndex.musicQuality.keywords') as unknown as string[] },

    // 存储管理
    { id: 'storage-directory', category: 'storage', title: t('common.searchIndex.storageDirectory.title'), description: t('common.searchIndex.storageDirectory.description'), keywords: t('common.searchIndex.storageDirectory.keywords') as unknown as string[] },
    { id: 'storage-cache', category: 'storage', title: t('common.searchIndex.storageCache.title'), description: t('common.searchIndex.storageCache.description'), keywords: t('common.searchIndex.storageCache.keywords') as unknown as string[] },
    { id: 'storage-cache-strategy', category: 'storage', title: t('common.searchIndex.storageCacheStrategy.title'), description: t('common.searchIndex.storageCacheStrategy.description'), keywords: t('common.searchIndex.storageCacheStrategy.keywords') as unknown as string[] },
    { id: 'storage-filename', category: 'storage', title: t('common.searchIndex.storageFilename.title'), description: t('common.searchIndex.storageFilename.description'), keywords: t('common.searchIndex.storageFilename.keywords') as unknown as string[] },
    { id: 'storage-tags', category: 'storage', title: t('common.searchIndex.storageTags.title'), description: t('common.searchIndex.storageTags.description'), keywords: t('common.searchIndex.storageTags.keywords') as unknown as string[] },

    // 快捷键
    { id: 'hotkey-settings', category: 'hotkeys', title: t('common.searchIndex.hotkeySettings.title'), description: t('common.searchIndex.hotkeySettings.description'), keywords: t('common.searchIndex.hotkeySettings.keywords') as unknown as string[] },

    // 插件
    { id: 'plugin-settings', category: 'plugins', title: t('common.searchIndex.pluginSettings.title'), description: t('common.searchIndex.pluginSettings.description'), keywords: t('common.searchIndex.pluginSettings.keywords') as unknown as string[] },

    // 关于
    { id: 'about-version', category: 'about', title: t('common.searchIndex.aboutVersion.title'), description: t('common.searchIndex.aboutVersion.description'), keywords: t('common.searchIndex.aboutVersion.keywords') as unknown as string[] },
    { id: 'about-tech', category: 'about', title: t('common.searchIndex.aboutTech.title'), description: t('common.searchIndex.aboutTech.description'), keywords: t('common.searchIndex.aboutTech.keywords') as unknown as string[] },
    { id: 'about-team', category: 'about', title: t('common.searchIndex.aboutTeam.title'), description: t('common.searchIndex.aboutTeam.description'), keywords: t('common.searchIndex.aboutTeam.keywords') as unknown as string[] },
    { id: 'about-legal', category: 'about', title: t('common.searchIndex.aboutLegal.title'), description: t('common.searchIndex.aboutLegal.description'), keywords: t('common.searchIndex.aboutLegal.keywords') as unknown as string[] }
  ]
}
