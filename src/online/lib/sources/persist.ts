import { readData, writeData } from "@online/lib/db"
import type { SourceScript } from "@online/types/source"

const STORE_FILE = "sources.json"

export async function loadSourceScripts(): Promise<SourceScript[]> {
  return readData<SourceScript[]>(STORE_FILE, [])
}

export function saveSourceScripts(scripts: SourceScript[]): void {
  writeData(STORE_FILE, scripts)
}
