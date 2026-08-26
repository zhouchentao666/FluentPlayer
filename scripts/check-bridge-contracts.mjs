import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { fileURLToPath } from 'node:url'

const bridgePath = fileURLToPath(new URL('../src/bridge/index.ts', import.meta.url))
const bridge = readFileSync(bridgePath, 'utf8')

const requiredContracts = [
  /download__pause_task', \{ taskId \}/,
  /download__resume_task', \{ taskId \}/,
  /download__cancel_task', \{ taskId \}/,
  /download__delete_task', \{ taskId, deleteFile \}/,
  /download__retry_task', \{ taskId \}/,
  /download__clear_tasks', \{ taskType: type \}/,
  /download__open_file_location', \{ filePath \}/,
  /local_music__scan', \{ dirs, skipHidden \}/,
]

for (const contract of requiredContracts) {
  assert.match(bridge, contract, `missing camelCase bridge contract: ${contract}`)
}

assert.doesNotMatch(bridge, /\b(task_id|delete_file|task_type|file_path|skip_hidden)\b/)

console.log('Bridge contracts are camelCase.')
