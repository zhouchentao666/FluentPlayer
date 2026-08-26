import { readFileSync, writeFileSync } from 'node:fs'

const check = process.argv.includes('--check')

function readVersion(path, pattern, label) {
  const content = readFileSync(path, 'utf8')
  const match = content.match(pattern)
  if (!match) {
    throw new Error(`无法在 ${path} 中找到 ${label}`)
  }
  return match[1]
}

const version =
  process.argv.find((arg) => arg.startsWith('--version='))?.split('=')[1] ??
  JSON.parse(readFileSync('src-tauri/tauri.conf.json', 'utf8')).version

// Android versionCode 与 Tauri 生成规则一致: major*1,000,000 + minor*1000 + patch
const [major, minor, patch] = version.split('.').map(Number)
if (![major, minor, patch].every(Number.isFinite)) {
  throw new Error(`版本号格式无效: ${version} (期望 major.minor.patch)`)
}
const versionCode = (major * 1_000_000 + minor * 1000 + patch).toString()

const updates = []
function syncFile(path, replacements) {
  let content = readFileSync(path, 'utf8')
  let changed = false

  for (const [pattern, replacement] of replacements) {
    const match = content.match(pattern)
    const matched = match?.[0]
    if (!matched) continue
    const replacementText = typeof replacement === 'function'
      ? replacement(...match.slice(0, replacement.length))
      : replacement
    const nextContent = content.replace(matched, () => replacementText)
    if (nextContent === content) continue
    changed = true
    if (check) continue
    content = nextContent
  }

  if (!changed && !check) return

  if (!check) writeFileSync(path, content)
  updates.push(path)
}

function jsonReplacement(_whole, prefix) {
  return prefix + version + String.fromCharCode(34)
}

syncFile('package.json', [[/("version":\s*")([^"]+)(")/, jsonReplacement]])
syncFile('src-tauri/tauri.conf.json', [[/("version":\s*")([^"]+)(")/, jsonReplacement]])
syncFile('src-tauri/Cargo.toml', [[/^version = "([^"]+)"/m, version]])
syncFile(
  'src-tauri/gen/android/app/src/main/assets/tauri.conf.json',
  [[/("version":\s*")([^"]+)(")/, jsonReplacement]]
)
syncFile('src-tauri/gen/android/app/tauri.properties', [
  [/^(tauri\.android\.versionName=)(.+)$/m, version],
  [/^(tauri\.android\.versionCode=)(\d+)$/m, versionCode]
])

// Re-read the canonical version after syncing so check mode validates the final state.
const canonicalVersion = JSON.parse(readFileSync('src-tauri/tauri.conf.json', 'utf8')).version

if (updates.length > 0) {
  console.log(`${check ? '需要同步' : '已同步'}以下文件的版本号到 ${canonicalVersion}:`)
  for (const path of updates) console.log(`  - ${path}`)
  if (check) process.exit(1)
} else {
  console.log(`所有版本号一致: ${canonicalVersion}`)
}
