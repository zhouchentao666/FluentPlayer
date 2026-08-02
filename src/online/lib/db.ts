const PREFIX = "fluentplayer-online:"

export async function readData<T>(file: string, fallback: T): Promise<T> {
  try {
    const raw = localStorage.getItem(PREFIX + file)
    if (raw == null) return fallback
    return JSON.parse(raw) as T
  } catch {
    return fallback
  }
}

export function writeData(file: string, data: unknown): void {
  try {
    localStorage.setItem(PREFIX + file, JSON.stringify(data))
  } catch (e) {
    console.error("writeData failed", e)
  }
}
