let timer: ReturnType<typeof setInterval> | null = null
let currentVersion = 0

export function transitionVolume(
  audio: HTMLAudioElement,
  volume: number,
  target: boolean = true,
  lengthen: boolean = false
): Promise<undefined> {
  if (timer !== null) {
    clearInterval(timer)
    timer = null
  }

  // 版本号递增使旧回调自动失效
  const version = ++currentVersion

  const playVolume = lengthen ? 40 : 20
  const pauseVolume = lengthen ? 30 : 20

  return new Promise((resolve) => {
    if (target) {
      timer = setInterval(() => {
        if (version !== currentVersion) { clearInterval(timer!); timer = null; return }
        audio.volume = Math.min(audio.volume + volume / playVolume, volume)
        if (audio.volume >= volume) {
          clearInterval(timer!)
          timer = null
          resolve(undefined)
        }
      }, 50)
      return
    }

    timer = setInterval(() => {
      if (version !== currentVersion) { clearInterval(timer!); timer = null; return }
      audio.volume = Math.max(audio.volume - volume / pauseVolume, 0)
      if (audio.volume <= 0) {
        clearInterval(timer!)
        timer = null
        audio.volume = volume
        resolve(undefined)
      }
    }, 50)
  })
}
