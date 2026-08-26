import { defineStore } from 'pinia'

export const playSetting = defineStore('playSetting', {
  state: () => ({
    isJumpLyric: true,
    bgPlaying: true,
    isAudioVisualizer: true,
    layoutMode: 'cd',
    showLeftPanel: true,
    isImmersiveLyricColor: true,
    isBlurLyric: false,
    autoHideBottom: true,
    isPauseTransition: true,
    isSeamlessTransition: false,
    seamlessMode: 'gapless' as 'gapless' | 'crossfade',
    crossfadeDuration: 3000,
    isGrepLyricInfo: false,
    strictGrep: false
  }),
  getters: {
    getisJumpLyric: (state) => state.isJumpLyric,
    getBgPlaying: (state) => state.bgPlaying,
    getIsAudioVisualizer: (state) => state.isAudioVisualizer,
    getLayoutMode: (state) => state.layoutMode,
    getShowLeftPanel: (state) => state.showLeftPanel,
    getIsImmersiveLyricColor: (state) => state.isImmersiveLyricColor,
    getIsBlurLyric: (state) => state.isBlurLyric,
    getAutoHideBottom: (state) => state.autoHideBottom,
    getIsPauseTransition: (state) => state.isPauseTransition,
    getIsSeamlessTransition: (state) => state.isSeamlessTransition,
    getSeamlessMode: (state) => state.seamlessMode,
    getCrossfadeDuration: (state) => state.crossfadeDuration,
    getIsGrepLyricInfo: (state) => state.isGrepLyricInfo,
    getStrictGrep: (state) => state.strictGrep
  },
  actions: {
    setIsDumpLyric(isDumpLyric: boolean) { this.isJumpLyric = isDumpLyric },
    setIsBlurLyric(isBlurLyric: boolean) { this.isBlurLyric = isBlurLyric },
    setBgPlaying(bgPlaying: boolean) { this.bgPlaying = bgPlaying },
    setIsAudioVisualizer(isAudioVisualizer: boolean) { this.isAudioVisualizer = isAudioVisualizer },
    setLayoutMode(mode: string) { this.layoutMode = mode },
    setShowLeftPanel(show: boolean) { this.showLeftPanel = show },
    setIsImmersiveLyricColor(isImmersiveLyricColor: boolean) { this.isImmersiveLyricColor = isImmersiveLyricColor },
    setAutoHideBottom(autoHideBottom: boolean) { this.autoHideBottom = autoHideBottom },
    setIsPauseTransition(isPauseTransition: boolean) { this.isPauseTransition = isPauseTransition },
    setIsSeamlessTransition(isSeamlessTransition: boolean) { this.isSeamlessTransition = isSeamlessTransition },
    setSeamlessMode(seamlessMode: 'gapless' | 'crossfade') { this.seamlessMode = seamlessMode },
    setCrossfadeDuration(crossfadeDuration: number) { this.crossfadeDuration = crossfadeDuration },
    setIsGrepLyricInfo(isGrepLyricInfo: boolean) { this.isGrepLyricInfo = isGrepLyricInfo },
    setStrictGrep(strictGrep: boolean) { this.strictGrep = strictGrep }
  },
  persist: true
})
