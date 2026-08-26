import { EQ_FREQUENCIES } from '@/store/Equalizer'
import createLogger from '@/utils/logger'

const log = createLogger('AudioManager')

class AudioManager {
  private static instance: AudioManager
  private audioSources = new WeakMap<HTMLAudioElement, MediaElementAudioSourceNode>()
  private audioContexts = new WeakMap<HTMLAudioElement, AudioContext>()
  private analysers = new Map<string, { node: AnalyserNode; element: HTMLAudioElement }>()
  private splitters = new WeakMap<HTMLAudioElement, GainNode>()
  private equalizers = new WeakMap<HTMLAudioElement, BiquadFilterNode[]>()
  private convolverNodes = new WeakMap<HTMLAudioElement, ConvolverNode>()
  private surroundGainNodes = new WeakMap<HTMLAudioElement, GainNode>()
  private balanceNodes = new WeakMap<HTMLAudioElement, StereoPannerNode>()
  private crossfadeGains = new WeakMap<HTMLAudioElement, GainNode>()
  private crossfadeLowpasses = new WeakMap<HTMLAudioElement, BiquadFilterNode>()

  public readonly EQ_FREQUENCIES = [...EQ_FREQUENCIES]

  static getInstance(): AudioManager {
    if (!AudioManager.instance) AudioManager.instance = new AudioManager()
    return AudioManager.instance
  }

  async resumeContext(audioElement: HTMLAudioElement): Promise<void> {
    const context = this.audioContexts.get(audioElement)
    if (context) {
      if (context.state === 'suspended') {
        try { await context.resume() } catch {}
      }
      try {
        const buffer = context.createBuffer(1, 1, 22050)
        const source = context.createBufferSource()
        source.buffer = buffer
        source.connect(context.destination)
        source.start(0)
      } catch {}
    }
  }

  getOrCreateAudioSource(audioElement: HTMLAudioElement): { source: MediaElementAudioSourceNode; context: AudioContext } | null {
    try {
      let source = this.audioSources.get(audioElement)
      let context = this.audioContexts.get(audioElement)

      if (context && context.state === 'suspended') context.resume().catch(() => {})

      if (!source || !context || context.state === 'closed') {
        const AudioContextClass = window.AudioContext || (window as any).webkitAudioContext
        context = new AudioContextClass({ latencyHint: 'playback' })

        if (context.state === 'suspended') context.resume().catch(() => {})

        source = context.createMediaElementSource(audioElement)

        // EQ chain
        const filters = this.EQ_FREQUENCIES.map((freq) => {
          const filter = context!.createBiquadFilter()
          filter.type = 'peaking'; filter.frequency.value = freq; filter.Q.value = 1.0; filter.gain.value = 0
          return filter
        })
        let lastNode: AudioNode = source
        filters.forEach((f) => { lastNode.connect(f); lastNode = f })
        this.equalizers.set(audioElement, filters)

        // Surround (Convolver)
        const convolver = context.createConvolver()
        const surroundGain = context.createGain()
        surroundGain.gain.value = 0
        this.convolverNodes.set(audioElement, convolver)
        this.surroundGainNodes.set(audioElement, surroundGain)

        const balanceNode = context.createStereoPanner()
        this.balanceNodes.set(audioElement, balanceNode)
        lastNode.connect(balanceNode)
        lastNode.connect(convolver); convolver.connect(surroundGain); surroundGain.connect(balanceNode)
        lastNode = balanceNode

        // Splitter + crossfade nodes
        const splitter = context.createGain()
        splitter.gain.value = 1.0
        lastNode.connect(splitter)

        const crossfadeLowpass = context.createBiquadFilter()
        crossfadeLowpass.type = 'lowpass'; crossfadeLowpass.frequency.value = 22050; crossfadeLowpass.Q.value = 0.707
        const crossfadeGain = context.createGain()
        crossfadeGain.gain.value = 1.0
        splitter.connect(crossfadeLowpass); crossfadeLowpass.connect(crossfadeGain); crossfadeGain.connect(context.destination)

        this.crossfadeLowpasses.set(audioElement, crossfadeLowpass)
        this.crossfadeGains.set(audioElement, crossfadeGain)
        this.splitters.set(audioElement, splitter)
        this.audioSources.set(audioElement, source)
        this.audioContexts.set(audioElement, context)
      }
      return { source, context }
    } catch (error) {
      log.error('创建音频源失败:', error)
      return null
    }
  }

  async setAudioOutputDevice(audioElement: HTMLAudioElement, deviceId: string): Promise<void> {
    // Rust backend devices (numeric IDs) change system default — no setSinkId needed
    if (!Number.isNaN(Number(deviceId))) return

    const context = this.audioContexts.get(audioElement)
    if (context && (context as any).setSinkId) {
      try { await (context as any).setSinkId(deviceId) } catch {}
    }
    if ((audioElement as any).setSinkId) {
      await (audioElement as any).setSinkId(deviceId)
    }
  }

  createAnalyser(audioElement: HTMLAudioElement, id: string, fftSize: number = 256): AnalyserNode | null {
    const audioData = this.getOrCreateAudioSource(audioElement)
    if (!audioData) return null
    const { context } = audioData
    try {
      if (this.analysers.has(id)) this.removeAnalyser(id)
      const analyser = context.createAnalyser()
      analyser.fftSize = fftSize; analyser.smoothingTimeConstant = 0.6
      const splitter = this.splitters.get(audioElement)
      if (splitter) splitter.connect(analyser)
      this.analysers.set(id, { node: analyser, element: audioElement })
      return analyser
    } catch { return null }
  }

  removeAnalyser(id: string): void {
    const entry = this.analysers.get(id)
    if (entry) {
      entry.node.disconnect()
      const splitter = this.splitters.get(entry.element)
      if (splitter) try { splitter.disconnect(entry.node) } catch {}
      this.analysers.delete(id)
    }
  }

  cleanupAudioElement(audioElement: HTMLAudioElement): void {
    const context = this.audioContexts.get(audioElement)
    if (context && context.state !== 'closed') context.close()
    // 清理关联的 analysers
    for (const [id, entry] of this.analysers) {
      if (entry.element === audioElement) {
        try { entry.node.disconnect() } catch {}
        this.analysers.delete(id)
      }
    }
    for (const map of [this.splitters, this.equalizers, this.convolverNodes, this.surroundGainNodes, this.balanceNodes, this.crossfadeGains, this.crossfadeLowpasses, this.audioSources, this.audioContexts] as any[]) {
      if (map.delete) map.delete(audioElement)
    }
  }

  getAnalyser(id: string): AnalyserNode | undefined { return this.analysers.get(id)?.node }

  setEqualizerBand(audioElement: HTMLAudioElement, index: number, gain: number): void {
    const filters = this.equalizers.get(audioElement)
    if (filters?.[index]) filters[index].gain.value = gain
  }

  getEqualizerBands(audioElement: HTMLAudioElement): number[] {
    const filters = this.equalizers.get(audioElement)
    return filters ? filters.map((f) => f.gain.value) : new Array(this.EQ_FREQUENCIES.length).fill(0)
  }

  setBalance(audioElement: HTMLAudioElement, value: number) { this.balanceNodes.get(audioElement)?.pan && (this.balanceNodes.get(audioElement)!.pan.value = value) }

  setSurroundMode(audioElement: HTMLAudioElement, mode: 'off' | 'small' | 'medium' | 'large') {
    const convolver = this.convolverNodes.get(audioElement)
    const gainNode = this.surroundGainNodes.get(audioElement)
    const ctx = this.audioContexts.get(audioElement)
    if (!convolver || !gainNode || !ctx) return
    if (mode === 'off') { gainNode.gain.setTargetAtTime(0, ctx.currentTime, 0.1); return }
    const durationMap = { small: 0.5, medium: 1.5, large: 3.0 }
    const decayMap = { small: 3.0, medium: 2.0, large: 1.5 }
    const rate = ctx.sampleRate; const length = rate * durationMap[mode]
    const impulse = ctx.createBuffer(2, length, rate)
    for (let ch = 0; ch < 2; ch++) {
      const data = impulse.getChannelData(ch)
      for (let i = 0; i < length; i++) data[i] = (Math.random() * 2 - 1) * Math.pow(1 - i / length, decayMap[mode])
    }
    convolver.buffer = impulse
    gainNode.gain.setTargetAtTime({ small: 0.3, medium: 0.5, large: 0.8 }[mode], ctx.currentTime, 0.2)
  }

  getAudioContextStats(audioElement: HTMLAudioElement): { sampleRate: number; channels: number; latency: number } | null {
    const context = this.audioContexts.get(audioElement)
    if (context) {
      return {
        sampleRate: context.sampleRate,
        channels: context.destination.maxChannelCount || 2,
        latency: (context.baseLatency || 0) + (context.outputLatency || 0)
      }
    }
    return null
  }

  private envelopeAnalysers = new WeakMap<HTMLAudioElement, AnalyserNode>()

  getEnvelopeAnalyser(audioElement: HTMLAudioElement): AnalyserNode | null {
    let analyser = this.envelopeAnalysers.get(audioElement)
    if (analyser) return analyser

    const audioData = this.getOrCreateAudioSource(audioElement)
    if (!audioData) return null
    const { context } = audioData

    const splitter = this.splitters.get(audioElement)
    if (!splitter) return null

    try {
      analyser = context.createAnalyser()
      analyser.fftSize = 512
      analyser.smoothingTimeConstant = 0.3
      splitter.connect(analyser)
      this.envelopeAnalysers.set(audioElement, analyser)
      return analyser
    } catch (error) {
      log.error('创建 envelope analyser 失败:', error)
      return null
    }
  }

  getCrossfadeGain(el: HTMLAudioElement): GainNode | null { return this.crossfadeGains.get(el) || null }
  getCrossfadeLowpass(el: HTMLAudioElement): BiquadFilterNode | null { return this.crossfadeLowpasses.get(el) || null }
  getContext(el: HTMLAudioElement): AudioContext | null { return this.audioContexts.get(el) || null }

  resetCrossfadeNodes(audioElement: HTMLAudioElement): void {
    const ctx = this.audioContexts.get(audioElement)
    if (!ctx) return
    const now = ctx.currentTime
    const gain = this.crossfadeGains.get(audioElement)
    const lowpass = this.crossfadeLowpasses.get(audioElement)
    if (gain) { try { gain.gain.cancelScheduledValues(0); gain.gain.setValueAtTime(1, now) } catch {} }
    if (lowpass) { try { lowpass.frequency.cancelScheduledValues(0); lowpass.frequency.setValueAtTime(22050, now) } catch {} }
  }
}

export default AudioManager.getInstance()
