// 复刻 convertLrcFormat / isA2Format，测试 krc 转换后是否能被 AMLL 正确解析
function formatTimestamp(timeMs) {
  const t = Math.max(0, Math.floor(timeMs))
  const minutes = Math.floor(t / 60000)
  const seconds = Math.floor((t % 60000) / 1000)
  const milliseconds = t % 1000
  return `${minutes.toString().padStart(2, '0')}:${seconds.toString().padStart(2, '0')}.${milliseconds.toString().padStart(3, '0')}`
}

function convertNewFormat(baseTimeMs, content) {
  const baseTimestamp = formatTimestamp(baseTimeMs)
  let convertedContent = `<${formatTimestamp(0)}>`
  const charPattern = /\((\d+),(\d+),(\d+)\)([^\(]*?)(?=\(|$)/g
  let match
  let isFirstChar = true
  let lastConsumedIndex = 0
  while ((match = charPattern.exec(content)) !== null) {
    const [, charStartMs, , ] = match
    const charTimeMs = parseInt(charStartMs, 10)
    const charTimestamp = formatTimestamp(charTimeMs)
    const char = match[4] ?? ''
    if (match.index > lastConsumedIndex) {
      convertedContent += content.substring(lastConsumedIndex, match.index)
    }
    if (isFirstChar) {
      convertedContent = `<${charTimestamp}>`
      isFirstChar = false
    } else {
      convertedContent += `<${charTimestamp}>${char}`
    }
    lastConsumedIndex = charPattern.lastIndex
  }
  if (lastConsumedIndex < content.length) {
    convertedContent += content.substring(lastConsumedIndex)
  }
  return `[${baseTimestamp}]${convertedContent}`
}

function convertLrcFormat(lrc) {
  if (!lrc) return ''
  const lines = lrc.replace(/\\n/g, '\n').split('\n')
  const convertedLines = []
  for (const line of lines) {
    if (!line.trim()) continue
    const newFormatMatch = line.match(/^\[(\d+),(\d+)\](.*)$/)
    if (newFormatMatch) {
      const [, startTimeMs, , content] = newFormatMatch
      const baseTimeMs = parseInt(startTimeMs, 10)
      if (!/\(\d+,\d+,\d+\)/.test(content)) {
        convertedLines.push(`[${formatTimestamp(baseTimeMs)}]${content}`)
        continue
      }
      const convertedLine = convertNewFormat(baseTimeMs, content)
      convertedLines.push(convertedLine ?? `[${formatTimestamp(baseTimeMs)}]${content}`)
      continue
    }
    const oldFormatMatch = line.match(/^\[(\d{2}:\d{2}\.\d{3})\](.*)$/)
    if (oldFormatMatch) {
      convertedLines.push(line)
      continue
    }
    convertedLines.push(line)
  }
  return convertedLines.join('\n')
}

function isA2Format(lrc) {
  return /^\[\d{2}:\d{2}(?:\.\d+)?\].*<\d{2}:\d{2}(?:\.\d+)?>/m.test(lrc)
}

// 模拟 decodeKrc 对一行 krc 的处理：krc 行 = [123.45]<0,1000,3>你<1000,500,2>好<1500,400,1>啊
// 注意 krc 行首 [123.45] 是秒，但 decodeKrc 里 parseInt("123.45")=123 -> 错误，这里先按正确毫秒算
const krcLine = '[123450,0]<0,1000,3>你<1000,500,2>好<1500,400,1>啊'
const out = convertLrcFormat(krcLine)
console.log('CONVERTED:', out)
console.log('isA2Format:', isA2Format(out))

// 模拟 decodeKrc 的行首 baseMs 计算 bug：parseInt("123.45")=123
const badBaseMs = 123 // parseInt("123.45") 的结果
const badLine = `[${badBaseMs},0]<0,1000,3>你<1000,500,2>好<1500,400,1>啊`
console.log('BAD CONVERTED:', convertLrcFormat(badLine))
