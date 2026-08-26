/**
 * PluginWorker - 在 Web Worker 中执行音乐源插件代码
 *
 * 将 CPU 密集的加密运算和插件 JS 执行从主线程移到 Worker，
 * 避免阻塞 UI。与主线程通过 postMessage 通信，IPC 调用
 * （httpProxy, plugins.getCode, plugins.getConfig）通过桥接回主线程执行。
 */

import i18n from '@/locales'

function getT() { return i18n.global.t }

// ==================== IPC 桥接 ====================

const pendingIpc = new Map<number, { resolve: (v: any) => void; reject: (e: Error) => void }>()
let ipcSeq = 0

function ipcCall(method: string, args: any): Promise<any> {
  return new Promise((resolve, reject) => {
    const id = ++ipcSeq
    pendingIpc.set(id, { resolve, reject })
    self.postMessage({ type: 'ipc', id, method, args })
  })
}

// ==================== 浏览器兼容的加密工具 ====================

function md5(str: string): string {
  function safeAdd(x: number, y: number): number {
    const lsw = (x & 0xffff) + (y & 0xffff)
    const msw = (x >> 16) + (y >> 16) + (lsw >> 16)
    return (msw << 16) | (lsw & 0xffff)
  }
  function bitRotateLeft(num: number, cnt: number): number {
    return (num << cnt) | (num >>> (32 - cnt))
  }
  function md5cmn(q: number, a: number, b: number, x: number, s: number, t: number): number {
    return safeAdd(bitRotateLeft(safeAdd(safeAdd(a, q), safeAdd(x, t)), s), b)
  }
  function md5ff(a: number, b: number, c: number, d: number, x: number, s: number, t: number): number {
    return md5cmn((b & c) | (~b & d), a, b, x, s, t)
  }
  function md5gg(a: number, b: number, c: number, d: number, x: number, s: number, t: number): number {
    return md5cmn((b & d) | (c & ~d), a, b, x, s, t)
  }
  function md5hh(a: number, b: number, c: number, d: number, x: number, s: number, t: number): number {
    return md5cmn(b ^ c ^ d, a, b, x, s, t)
  }
  function md5ii(a: number, b: number, c: number, d: number, x: number, s: number, t: number): number {
    return md5cmn(c ^ (b | ~d), a, b, x, s, t)
  }

  const encoder = new TextEncoder()
  const msg = encoder.encode(str)
  const msgLen = msg.length

  const bitLen = msgLen * 8
  const padLen = (((bitLen + 64) >>> 9) << 4) + 15
  const words = new Int32Array(padLen + 1)
  for (let i = 0; i < msgLen; i++) {
    words[i >> 2] |= msg[i] << ((i % 4) << 3)
  }
  words[msgLen >> 2] |= 0x80 << ((msgLen % 4) << 3)
  words[padLen] = bitLen

  let a = 0x67452301
  let b = 0xefcdab89
  let c = 0x98badcfe
  let d = 0x10325476

  for (let i = 0; i <= padLen; i += 16) {
    const oa = a, ob = b, oc = c, od = d

    a = md5ff(a, b, c, d, words[i], 7, -680876936)
    d = md5ff(d, a, b, c, words[i + 1], 12, -389564586)
    c = md5ff(c, d, a, b, words[i + 2], 17, 606105819)
    b = md5ff(b, c, d, a, words[i + 3], 22, -1044525330)
    a = md5ff(a, b, c, d, words[i + 4], 7, -176418897)
    d = md5ff(d, a, b, c, words[i + 5], 12, 1200080426)
    c = md5ff(c, d, a, b, words[i + 6], 17, -1473231341)
    b = md5ff(b, c, d, a, words[i + 7], 22, -45705983)
    a = md5ff(a, b, c, d, words[i + 8], 7, 1770035416)
    d = md5ff(d, a, b, c, words[i + 9], 12, -1958414417)
    c = md5ff(c, d, a, b, words[i + 10], 17, -42063)
    b = md5ff(b, c, d, a, words[i + 11], 22, -1990404162)
    a = md5ff(a, b, c, d, words[i + 12], 7, 1804603682)
    d = md5ff(d, a, b, c, words[i + 13], 12, -40341101)
    c = md5ff(c, d, a, b, words[i + 14], 17, -1502002290)
    b = md5ff(b, c, d, a, words[i + 15], 22, 1236535329)

    a = md5gg(a, b, c, d, words[i + 1], 5, -165796510)
    d = md5gg(d, a, b, c, words[i + 6], 9, -1069501632)
    c = md5gg(c, d, a, b, words[i + 11], 14, 643717713)
    b = md5gg(b, c, d, a, words[i], 20, -373897302)
    a = md5gg(a, b, c, d, words[i + 5], 5, -701558691)
    d = md5gg(d, a, b, c, words[i + 10], 9, 38016083)
    c = md5gg(c, d, a, b, words[i + 15], 14, -660478335)
    b = md5gg(b, c, d, a, words[i + 4], 20, -405537848)
    a = md5gg(a, b, c, d, words[i + 9], 5, 568446438)
    d = md5gg(d, a, b, c, words[i + 14], 9, -1019803690)
    c = md5gg(c, d, a, b, words[i + 3], 14, -187363961)
    b = md5gg(b, c, d, a, words[i + 8], 20, 1163531501)
    a = md5gg(a, b, c, d, words[i + 13], 5, -1444681467)
    d = md5gg(d, a, b, c, words[i + 2], 9, -51403784)
    c = md5gg(c, d, a, b, words[i + 7], 14, 1735328473)
    b = md5gg(b, c, d, a, words[i + 12], 20, -1926607734)

    a = md5hh(a, b, c, d, words[i + 5], 4, -378558)
    d = md5hh(d, a, b, c, words[i + 8], 11, -2022574463)
    c = md5hh(c, d, a, b, words[i + 11], 16, 1839030562)
    b = md5hh(b, c, d, a, words[i + 14], 23, -35309556)
    a = md5hh(a, b, c, d, words[i + 1], 4, -1530992060)
    d = md5hh(d, a, b, c, words[i + 4], 11, 1272893353)
    c = md5hh(c, d, a, b, words[i + 7], 16, -155497632)
    b = md5hh(b, c, d, a, words[i + 10], 23, -1094730640)
    a = md5hh(a, b, c, d, words[i + 13], 4, 681279174)
    d = md5hh(d, a, b, c, words[i], 11, -358537222)
    c = md5hh(c, d, a, b, words[i + 3], 16, -722521979)
    b = md5hh(b, c, d, a, words[i + 6], 23, 76029189)
    a = md5hh(a, b, c, d, words[i + 9], 4, -640364487)
    d = md5hh(d, a, b, c, words[i + 12], 11, -421815835)
    c = md5hh(c, d, a, b, words[i + 15], 16, 530742520)
    b = md5hh(b, c, d, a, words[i + 2], 23, -995338651)

    a = md5ii(a, b, c, d, words[i], 6, -198630844)
    d = md5ii(d, a, b, c, words[i + 7], 10, 1126891415)
    c = md5ii(c, d, a, b, words[i + 14], 15, -1416354905)
    b = md5ii(b, c, d, a, words[i + 5], 21, -57434055)
    a = md5ii(a, b, c, d, words[i + 12], 6, 1700485571)
    d = md5ii(d, a, b, c, words[i + 3], 10, -1894986606)
    c = md5ii(c, d, a, b, words[i + 10], 15, -1051523)
    b = md5ii(b, c, d, a, words[i + 1], 21, -2054922799)
    a = md5ii(a, b, c, d, words[i + 8], 6, 1873313359)
    d = md5ii(d, a, b, c, words[i + 15], 10, -30611744)
    c = md5ii(c, d, a, b, words[i + 6], 15, -1560198380)
    b = md5ii(b, c, d, a, words[i + 13], 21, 1309151649)
    a = md5ii(a, b, c, d, words[i + 4], 6, -145523070)
    d = md5ii(d, a, b, c, words[i + 11], 10, -1120210379)
    c = md5ii(c, d, a, b, words[i + 2], 15, 718787259)
    b = md5ii(b, c, d, a, words[i + 9], 21, -343485551)

    a = safeAdd(a, oa)
    b = safeAdd(b, ob)
    c = safeAdd(c, oc)
    d = safeAdd(d, od)
  }

  function hex(x: number): string {
    const s = (x >>> 0).toString(16)
    return '00000000'.slice(s.length) + s
  }
  return hex(a) + hex(b) + hex(c) + hex(d)
}

// ==================== AES 加密 (CBC/ECB, 128-bit) ====================

const AES_SBOX = [
  0x63,0x7c,0x77,0x7b,0xf2,0x6b,0x6f,0xc5,0x30,0x01,0x67,0x2b,0xfe,0xd7,0xab,0x76,
  0xca,0x82,0xc9,0x7d,0xfa,0x59,0x47,0xf0,0xad,0xd4,0xa2,0xaf,0x9c,0xa4,0x72,0xc0,
  0xb7,0xfd,0x93,0x26,0x36,0x3f,0xf7,0xcc,0x34,0xa5,0xe5,0xf1,0x71,0xd8,0x31,0x15,
  0x04,0xc7,0x23,0xc3,0x18,0x96,0x05,0x9a,0x07,0x12,0x80,0xe2,0xeb,0x27,0xb2,0x75,
  0x09,0x83,0x2c,0x1a,0x1b,0x6e,0x5a,0xa0,0x52,0x3b,0xd6,0xb3,0x29,0xe3,0x2f,0x84,
  0x53,0xd1,0x00,0xed,0x20,0xfc,0xb1,0x5b,0x6a,0xcb,0xbe,0x39,0x4a,0x4c,0x58,0xcf,
  0xd0,0xef,0xaa,0xfb,0x43,0x4d,0x33,0x85,0x45,0xf9,0x02,0x7f,0x50,0x3c,0x9f,0xa8,
  0x51,0xa3,0x40,0x8f,0x92,0x9d,0x38,0xf5,0xbc,0xb6,0xda,0x21,0x10,0xff,0xf3,0xd2,
  0xcd,0x0c,0x13,0xec,0x5f,0x97,0x44,0x17,0xc4,0xa7,0x7e,0x3d,0x64,0x5d,0x19,0x73,
  0x60,0x81,0x4f,0xdc,0x22,0x2a,0x90,0x88,0x46,0xee,0xb8,0x14,0xde,0x5e,0x0b,0xdb,
  0xe0,0x32,0x3a,0x0a,0x49,0x06,0x24,0x5c,0xc2,0xd3,0xac,0x62,0x91,0x95,0xe4,0x79,
  0xe7,0xc8,0x37,0x6d,0x8d,0xd5,0x4e,0xa9,0x6c,0x56,0xf4,0xea,0x65,0x7a,0xae,0x08,
  0xba,0x78,0x25,0x2e,0x1c,0xa6,0xb4,0xc6,0xe8,0xdd,0x74,0x1f,0x4b,0xbd,0x8b,0x8a,
  0x70,0x3e,0xb5,0x66,0x48,0x03,0xf6,0x0e,0x61,0x35,0x57,0xb9,0x86,0xc1,0x1d,0x9e,
  0xe1,0xf8,0x98,0x11,0x69,0xd9,0x8e,0x94,0x9b,0x1e,0x87,0xe9,0xce,0x55,0x28,0xdf,
  0x8c,0xa1,0x89,0x0d,0xbf,0xe6,0x42,0x68,0x41,0x99,0x2d,0x0f,0xb0,0x54,0xbb,0x16,
]

const AES_RCON = [0x01,0x02,0x04,0x08,0x10,0x20,0x40,0x80,0x1b,0x36]

function xorBlocks(a: Uint8Array, b: Uint8Array): Uint8Array {
  const r = new Uint8Array(16)
  for (let i = 0; i < 16; i++) r[i] = a[i] ^ b[i]
  return r
}

function aes128EncryptBlock(block: Uint8Array, expandedKey: Uint8Array[]): Uint8Array {
  const state = new Uint8Array(16)
  for (let c = 0; c < 4; c++) {
    for (let r = 0; r < 4; r++) state[r * 4 + c] = block[c * 4 + r]
  }
  for (let c = 0; c < 4; c++) {
    for (let r = 0; r < 4; r++) state[r * 4 + c] ^= expandedKey[0][c * 4 + r]
  }
  for (let round = 1; round <= 10; round++) {
    for (let i = 0; i < 16; i++) state[i] = AES_SBOX[state[i]]
    let t: number
    t = state[1]; state[1] = state[5]; state[5] = state[9]; state[9] = state[13]; state[13] = t
    t = state[2]; state[2] = state[10]; state[10] = t; t = state[6]; state[6] = state[14]; state[14] = t
    t = state[15]; state[15] = state[11]; state[11] = state[7]; state[7] = state[3]; state[3] = t
    if (round < 10) {
      for (let c = 0; c < 4; c++) {
        const i = c * 4
        const a0 = state[i], a1 = state[i + 1], a2 = state[i + 2], a3 = state[i + 3]
        const xtime = (b: number) => (b << 1) ^ (((b >> 7) & 1) * 0x1b)
        state[i]     = xtime(a0) ^ (xtime(a1) ^ a1) ^ a2 ^ a3
        state[i + 1] = a0 ^ xtime(a1) ^ (xtime(a2) ^ a2) ^ a3
        state[i + 2] = a0 ^ a1 ^ xtime(a2) ^ (xtime(a3) ^ a3)
        state[i + 3] = (xtime(a0) ^ a0) ^ a1 ^ a2 ^ xtime(a3)
      }
    }
    for (let c = 0; c < 4; c++) {
      for (let r = 0; r < 4; r++) state[r * 4 + c] ^= expandedKey[round][c * 4 + r]
    }
  }
  const out = new Uint8Array(16)
  for (let c = 0; c < 4; c++) {
    for (let r = 0; r < 4; r++) out[c * 4 + r] = state[r * 4 + c]
  }
  return out
}

function aesExpandKey(key: Uint8Array): Uint8Array[] {
  const w: Uint8Array[] = []
  for (let i = 0; i < 4; i++) {
    w[i] = new Uint8Array([key[i * 4], key[i * 4 + 1], key[i * 4 + 2], key[i * 4 + 3]])
  }
  for (let i = 4; i < 44; i++) {
    let temp = new Uint8Array(w[i - 1])
    if (i % 4 === 0) {
      temp = new Uint8Array([AES_SBOX[temp[1]], AES_SBOX[temp[2]], AES_SBOX[temp[3]], AES_SBOX[temp[0]]])
      temp[0] ^= AES_RCON[(i / 4) - 1]
    }
    const nw = new Uint8Array(4)
    for (let j = 0; j < 4; j++) nw[j] = w[i - 4][j] ^ temp[j]
    w[i] = nw
  }
  const rounds: Uint8Array[] = []
  for (let r = 0; r <= 10; r++) {
    const rk = new Uint8Array(16)
    for (let c = 0; c < 4; c++) {
      for (let row = 0; row < 4; row++) rk[c * 4 + row] = w[r * 4 + c][row]
    }
    rounds.push(rk)
  }
  return rounds
}

function pkcs7Pad(data: Uint8Array, blockSize: number): Uint8Array {
  const padLen = blockSize - (data.length % blockSize)
  const padded = new Uint8Array(data.length + padLen)
  padded.set(data)
  for (let i = data.length; i < padded.length; i++) padded[i] = padLen
  return padded
}

function aesEncrypt(
  data: string | Uint8Array,
  mode: string,
  key: string | Uint8Array,
  iv?: string | Uint8Array
): Uint8Array {
  const toBytes = (v: string | Uint8Array): Uint8Array =>
    typeof v === 'string' ? new TextEncoder().encode(v) : v
  const rawData = toBytes(data)
  const rawKey = toBytes(key)
  const keyBytes = rawKey.length >= 16 ? rawKey.slice(0, 16) : (() => {
    const k = new Uint8Array(16); k.set(rawKey); return k
  })()
  const expandedKey = aesExpandKey(keyBytes)

  if (mode === 'aes-128-ecb') {
    const padded = pkcs7Pad(rawData, 16)
    const out = new Uint8Array(padded.length)
    for (let i = 0; i < padded.length; i += 16) {
      out.set(aes128EncryptBlock(padded.slice(i, i + 16), expandedKey), i)
    }
    return out
  }

  const rawIv = iv ? toBytes(iv) : new Uint8Array(16)
  const ivBytes = rawIv.length >= 16 ? rawIv.slice(0, 16) : (() => {
    const b = new Uint8Array(16); b.set(rawIv); return b
  })()
  const padded = pkcs7Pad(rawData, 16)
  const out = new Uint8Array(padded.length)
  let prev = ivBytes
  for (let i = 0; i < padded.length; i += 16) {
    const block = xorBlocks(padded.slice(i, i + 16), prev)
    const encrypted = aes128EncryptBlock(block, expandedKey)
    out.set(encrypted, i)
    prev = encrypted
  }
  return out
}

function randomBytes(size: number): Uint8Array {
  const buf = new Uint8Array(size)
  crypto.getRandomValues(buf)
  return buf
}

// ==================== RSA 加密 (PKCS1 v1.5) ====================

function parseRsaPublicKey(der: Uint8Array): { n: bigint; e: bigint } {
  let offset = 0
  const readTag = () => der[offset++]
  const readLen = (): number => {
    const b = der[offset++]
    if (b < 0x80) return b
    const bytes = b & 0x7f
    let len = 0
    for (let i = 0; i < bytes; i++) len = (len << 8) | der[offset++]
    return len
  }
  const readInteger = (): bigint => {
    readTag()
    const len = readLen()
    let val = 0n
    for (let i = 0; i < len; i++) val = (val << 8n) | BigInt(der[offset++])
    return val
  }
  readTag(); readLen()
  readTag(); const algLen = readLen(); offset += algLen
  readTag(); readLen(); offset++
  readTag(); readLen()
  const n = readInteger()
  const e = readInteger()
  return { n, e }
}

function modPow(base: bigint, exp: bigint, mod: bigint): bigint {
  let result = 1n
  base = base % mod
  while (exp > 0n) {
    if (exp & 1n) result = (result * base) % mod
    exp >>= 1n
    base = (base * base) % mod
  }
  return result
}

function rsaEncrypt(data: string, pemKey: string): string {
  const pemBody = pemKey
    .replace(/-----BEGIN PUBLIC KEY-----/g, '')
    .replace(/-----END PUBLIC KEY-----/g, '')
    .replace(/[\s\r\n]/g, '')
  const binaryDer = Uint8Array.from(atob(pemBody), c => c.charCodeAt(0))
  const { n, e } = parseRsaPublicKey(binaryDer)

  const keySize = Math.ceil(n.toString(16).length / 2)
  const dataBytes = new TextEncoder().encode(data)
  const padLen = keySize - dataBytes.length - 3
  if (padLen < 8) throw new Error('RSA: data too long for key size')

  const padded = new Uint8Array(keySize)
  padded[1] = 0x02
  const rand = new Uint8Array(padLen)
  crypto.getRandomValues(rand)
  for (let i = 0; i < rand.length; i++) {
    padded[2 + i] = rand[i] === 0 ? 1 : rand[i]
  }
  padded[2 + padLen] = 0x00
  padded.set(dataBytes, 3 + padLen)

  let m = 0n
  for (const b of padded) m = (m << 8n) | BigInt(b)
  const c = modPow(m, e, n)

  const result = new Uint8Array(keySize)
  let tmp = c
  for (let i = keySize - 1; i >= 0; i--) {
    result[i] = Number(tmp & 0xffn)
    tmp >>= 8n
  }
  return btoa(String.fromCharCode(...result))
}

// ==================== cerumusic API 工厂（Worker 版：通过 IPC 桥接 HTTP） ====================

function createCerumusicApi() {
  return {
    env: 'browser',
    version: '1.0.3',
    utils: {
      buffer: {
        from: (data: string | ArrayBuffer | Uint8Array, encoding?: string) => {
          let bytes: Uint8Array
          if (typeof data === 'string') {
            const enc = (encoding || 'utf8').toLowerCase()
            if (enc === 'base64') bytes = Uint8Array.from(atob(data), c => c.charCodeAt(0))
            else if (enc === 'hex') {
              bytes = new Uint8Array(data.length / 2)
              for (let i = 0; i < data.length; i += 2) bytes[i / 2] = parseInt(data.substr(i, 2), 16)
            } else bytes = new TextEncoder().encode(data)
          } else if (data instanceof ArrayBuffer) bytes = new Uint8Array(data)
          else if (data instanceof Uint8Array) bytes = new Uint8Array(data)
          else bytes = new Uint8Array(data as any)
          const origToString = bytes.toString.bind(bytes)
          ;(bytes as any).toString = function(enc?: string) {
            if (!enc || enc === 'utf8' || enc === 'utf-8') return new TextDecoder().decode(bytes)
            if (enc === 'hex') return Array.from(bytes).map(b => b.toString(16).padStart(2, '0')).join('')
            if (enc === 'base64') { let bin = ''; for (let i = 0; i < bytes.length; i++) bin += String.fromCharCode(bytes[i]); return btoa(bin) }
            return origToString()
          }
          return bytes
        },
        bufToString: (buffer: Uint8Array, encoding?: string) => {
          const enc = (encoding || 'utf8').toLowerCase()
          if (enc === 'base64') {
            let binary = ''; for (let i = 0; i < buffer.length; i++) binary += String.fromCharCode(buffer[i])
            return btoa(binary)
          }
          if (enc === 'hex') return Array.from(buffer).map(b => b.toString(16).padStart(2, '0')).join('')
          return new TextDecoder().decode(buffer)
        }
      },
      crypto: { md5, aesEncrypt, randomBytes, rsaEncrypt }
    },
    request: (
      url: string,
      options?: any,
      callback?: (err: any, result?: any) => void
    ): any => {
      if (typeof options === 'function') { callback = options; options = {} }
      const opts = options || {}
      const method = opts.method || (opts.headers?.['Content-Type'] ? 'POST' : 'GET')
      const body = opts.body
        ? (typeof opts.body === 'string' ? opts.body : JSON.stringify(opts.body))
        : undefined

      const doFetch = () => ipcCall('httpProxy', {
        url, method, headers: opts.headers || {}, body, timeout: opts.timeout || 15000,
      }).then((res: any) => ({
        statusCode: res?.statusCode ?? 0,
        headers: res?.headers || {},
        body: res?.body ?? ''
      })).catch((e: any) => {
        throw new Error(getT()('plugin.networkFailed', { message: e?.message || e, url }))
      })

      if (callback) {
        doFetch().then((r: any) => callback!(null, r)).catch((e: any) => callback!(e, { statusCode: 0, headers: {}, body: '' }))
        return
      }
      return doFetch()
    },
    NoticeCenter(type: string, data: any) {
      if (type === 'update' && data) {
        self.postMessage({
          type: 'plugin-update-notice',
          notice: {
            pluginName: data.pluginInfo?.name || '',
            updateUrl: data.url || '',
            newVersion: data.version || '',
            content: data.content || '',
            pluginType: data.pluginInfo?.type || 'cr',
          }
        })
      }
    }
  }
}

// ==================== 插件格式检测 ====================

function isCrPlugin(code: string): boolean {
  return /\bcerumusic\b/.test(code)
}

// ==================== 插件执行引擎 ====================

interface PluginExports {
  pluginInfo?: { name: string; version: string; author: string; description: string }
  sources?: Record<string, { name: string; type: string; qualitys: string[] }>
  musicUrl?: (source: string, musicInfo: any, quality: string) => Promise<string | { error?: string }>
  [key: string]: any
}

function executePluginCode(code: string): PluginExports {
  const isCr = isCrPlugin(code)
  const cerumusicApi = createCerumusicApi()

  const fetchProxy = isCr ? `
    var fetch = function(url, opts) {
      var httpProxy = globalThis.__pluginHttpProxy;
      if (!httpProxy) return Promise.reject(new Error(getT()('plugin.httpProxyUnavailable')));
      var method = (opts && opts.method) || 'GET';
      var headers = (opts && opts.headers) || {};
      var body = opts && opts.body;
      if (body && typeof body !== 'string') body = JSON.stringify(body);
      return httpProxy(url, { method: method, headers: headers, body: body, timeout: 15000 })
        .then(function(res) {
          var rawBody = res && res.body;
          return {
            ok: res && res.statusCode >= 200 && res.statusCode < 300,
            status: res ? res.statusCode : 0,
            statusText: '',
            headers: new Map(Object.entries((res && res.headers) || {})),
            json: function() {
              if (typeof rawBody === 'string') {
                try { return Promise.resolve(JSON.parse(rawBody)); } catch (_) {}
              }
              return Promise.resolve(rawBody);
            },
            text: function() {
              return Promise.resolve(typeof rawBody === 'string' ? rawBody : JSON.stringify(rawBody));
            }
          };
        });
    };
    globalThis.fetch = fetch;
    globalThis.window = globalThis;
  ` : ''

  const sandbox = new Function(
    'globalThis', 'lx', 'console', 'setTimeout', 'clearTimeout',
    'setInterval', 'clearInterval', 'Buffer', 'JSON', 'require',
    '_m', '_e', 'process', 'cerumusic',
    `
    ${fetchProxy}
    var module = { exports: {} };
    var exports = module.exports;
    var global = globalThis;
    try {
      ${code}
    } catch(e) {
      console.warn('[PluginRunner] ' + getT()('plugin.pluginError'), e.message);
    }
    return module.exports;
    `
  )

  const mockExports: any = { exports: {} }
  const mockModule: any = { exports: mockExports }

  const lxState: { requestHandler: Function | null; updateAlertSent: boolean } = { requestHandler: null, updateAlertSent: false }
  const mockLx = {
    on: (event: string, handler: Function) => {
      if (event === 'request') lxState.requestHandler = handler
    },
    send: (event: string, data?: any) => {
      if (event === 'updateAlert' && !lxState.updateAlertSent) {
        lxState.updateAlertSent = true
        const log = data?.log ? String(data.log).substring(0, 1024) : ''
        let updateUrl = data?.updateUrl ? String(data.updateUrl) : ''
        if (updateUrl && (updateUrl.startsWith('http://') || updateUrl.startsWith('https://'))) {
          updateUrl = updateUrl.substring(0, 1024)
        } else {
          updateUrl = ''
        }
        self.postMessage({
          type: 'plugin-update-notice',
          notice: {
            pluginName: '',
            updateUrl,
            newVersion: '',
            content: log,
            pluginType: 'lx',
          }
        })
      }
    },
    version: '1.0.0',
    env: 'browser'
  }

  function patchBufferToString(bytes: Uint8Array): Uint8Array {
    const origToString = bytes.toString.bind(bytes)
    ;(bytes as any).toString = function(encoding?: string, start?: number, end?: number) {
      const s = start || 0, e = end ?? bytes.length
      const slice = bytes.slice(s, e)
      if (!encoding || encoding === 'utf8' || encoding === 'utf-8') return new TextDecoder().decode(slice)
      if (encoding === 'hex') return Array.from(slice).map(b => b.toString(16).padStart(2, '0')).join('')
      if (encoding === 'base64') { let bin = ''; for (let i = 0; i < slice.length; i++) bin += String.fromCharCode(slice[i]); return btoa(bin) }
      if (encoding === 'binary' || encoding === 'latin1') return Array.from(slice).map(b => String.fromCharCode(b)).join('')
      return origToString()
    }
    return bytes
  }
  const BufferPolyfill: any = {
    from(data: any, encodingOrOffset?: string | number, length?: number) {
      let bytes: Uint8Array
      if (typeof data === 'string') {
        const enc = ((encodingOrOffset as string) || 'utf8').toLowerCase()
        if (enc === 'base64') bytes = Uint8Array.from(atob(data), c => c.charCodeAt(0))
        else if (enc === 'hex') {
          bytes = new Uint8Array(data.length / 2)
          for (let i = 0; i < data.length; i += 2) bytes[i / 2] = parseInt(data.substr(i, 2), 16)
        } else bytes = new TextEncoder().encode(data)
      } else if (data instanceof ArrayBuffer) bytes = new Uint8Array(data)
      else if (data instanceof Uint8Array) bytes = new Uint8Array(data)
      else if (Array.isArray(data)) bytes = new Uint8Array(data)
      else bytes = new Uint8Array(data)
      return patchBufferToString(bytes)
    },
    isBuffer(obj: any) { return obj instanceof Uint8Array },
    concat(list: Uint8Array[], totalLength?: number) {
      const len = totalLength ?? list.reduce((s: number, b: Uint8Array) => s + b.length, 0)
      const result = new Uint8Array(len)
      let offset = 0
      for (const buf of list) { result.set(buf, offset); offset += buf.length }
      return patchBufferToString(result)
    },
    alloc(size: number) { return patchBufferToString(new Uint8Array(size)) }
  }

  const sandboxGlobal = {} as any
  sandboxGlobal.cerumusic = cerumusicApi
  sandboxGlobal.__pluginHttpProxy = (url: string, options: any) => ipcCall('httpProxy', options?.url ? options : { url, ...options })
  if (isCr) {
    sandboxGlobal.fetch = fetch
  }

  try {
    const pluginExports = sandbox(
      sandboxGlobal, mockLx, console, setTimeout, clearTimeout,
      setInterval, clearInterval, BufferPolyfill, JSON,
      () => ({}), {}, {}, { env: { NODE_ENV: 'production' } },
      cerumusicApi
    )
    if (pluginExports && typeof pluginExports === 'object') {
      Object.assign(mockModule.exports, pluginExports)
    }
  } catch (e) {
    console.warn('[PluginWorker] ' + getT()('plugin.pluginInitFailed'), e)
  }

  const result = mockModule.exports

  if (!result.musicUrl && lxState.requestHandler) {
    const handler = lxState.requestHandler
    result.musicUrl = async (source: string, musicInfo: any, quality: string) => {
      const res = await handler({
        source,
        action: 'musicUrl',
        info: { musicInfo, type: quality }
      })
      return res
    }
  }

  if (isCr && cerumusicApi && typeof result.musicUrl === 'function') {
    const originalMusicUrl = result.musicUrl
    result.musicUrl = async (source: string, musicInfo: any, quality: string) => {
      return originalMusicUrl.call({ cerumusic: cerumusicApi }, source, musicInfo, quality)
    }
  }

  return result
}

// ==================== 插件管理 ====================

interface LoadedPlugin {
  exports: PluginExports
  code: string
}

const pluginCache = new Map<string, LoadedPlugin>()

async function getPluginCode(pluginId: string): Promise<string> {
  const res = await ipcCall('plugins.getCode', { pluginId })
  if (res?.success && typeof res.data === 'string') return res.data
  throw new Error(res?.error || getT()('plugin.noPluginCode'))
}

async function loadPlugin(pluginId: string): Promise<LoadedPlugin> {
  const cached = pluginCache.get(pluginId)
  if (cached) return cached

  const code = await getPluginCode(pluginId)
  const exports = executePluginCode(code)

  const plugin: LoadedPlugin = { exports, code }
  pluginCache.set(pluginId, plugin)
  return plugin
}

/**
 * 重新加载插件（绕过缓存），用于更新检查场景。
 * 删除缓存后重新获取代码并执行，使插件顶层的 checkUpdate() 等逻辑重新运行。
 */
async function reloadPlugin(pluginId: string): Promise<LoadedPlugin> {
  pluginCache.delete(pluginId)

  const code = await getPluginCode(pluginId)
  const exports = executePluginCode(code)

  const plugin: LoadedPlugin = { exports, code }
  pluginCache.set(pluginId, plugin)
  return plugin
}

function normalizeMusicUrlResult(_source: string, result: any): string {
  if (typeof result === 'string') {
    return result
  }
  if (typeof result === 'object' && result !== null) {
    const obj = result as Record<string, any>
    if (obj.error) throw new Error(String(obj.error))
    if (obj.url && typeof obj.url === 'string') return obj.url
  }
  throw new Error(getT()('plugin.invalidPlayUrl', { type: typeof result }))
}

function parseArgsJson(argsJson: string): any[] {
  try {
    const parsed = JSON.parse(argsJson || '[]')
    if (Array.isArray(parsed)) return parsed
    if (parsed === null || typeof parsed === 'undefined') return []
    return [parsed]
  } catch {
    throw new Error(getT()('plugin.invalidParams'))
  }
}

async function invokePluginMethod(fn: Function, thisArg: any, args: any[]): Promise<any> {
  return new Promise((resolve, reject) => {
    let settled = false
    const callbackExpected = fn.length > args.length
    const timeout = callbackExpected
      ? setTimeout(() => {
        if (settled) return
        settled = true
        reject(new Error(getT()('plugin.callbackTimeout')))
      }, 15000)
      : null

    const finish = (err: any, result?: any) => {
      if (settled) return
      settled = true
      if (timeout) clearTimeout(timeout)
      if (err) {
        reject(err instanceof Error ? err : new Error(String(err)))
        return
      }
      resolve(result)
    }

    try {
      const ret = callbackExpected
        ? fn.call(thisArg, ...args, (err: any, result: any) => finish(err, result))
        : fn.call(thisArg, ...args)

      if (ret && typeof ret.then === 'function') {
        ;(ret as Promise<any>).then((result) => finish(null, result)).catch((err) => finish(err))
        return
      }

      if (!callbackExpected || typeof ret !== 'undefined') {
        finish(null, ret)
      }
    } catch (e) {
      finish(e)
    }
  })
}

// ==================== 公共方法 ====================

async function getMusicUrl(pluginId: string, source: string, songInfo: any, quality: string): Promise<string> {
  const plugin = await loadPlugin(pluginId)
  if (typeof plugin.exports.musicUrl !== 'function') {
    throw new Error(getT()('plugin.noMusicUrl'))
  }
  try {
    const result = await plugin.exports.musicUrl(source, songInfo, quality)
    return normalizeMusicUrlResult(source, result)
  } catch (e: any) {
    throw new Error(`[${source}] ${e.message || String(e)}`)
  }
}

async function getPic(pluginId: string, source: string, songInfo: any): Promise<string> {
  const plugin = await loadPlugin(pluginId)
  if (typeof plugin.exports.getPic !== 'function') {
    throw new Error(getT()('plugin.noGetPic'))
  }
  try {
    const result = await plugin.exports.getPic(source, songInfo)
    if (typeof result === 'string') return result
    if (result?.url) return result.url
    if (result?.picUrl) return result.picUrl
    return ''
  } catch (e: any) {
    throw new Error(`[plugin=${pluginId}][${source}] getPic: ${e.message || String(e)}`)
  }
}

async function getLyric(pluginId: string, source: string, songInfo: any): Promise<string | { lyric?: string; tlyric?: string; rlyric?: string; lxlyric?: string }> {
  const plugin = await loadPlugin(pluginId)
  if (typeof plugin.exports.getLyric !== 'function') {
    throw new Error(getT()('plugin.noGetLyric'))
  }
  try {
    return await plugin.exports.getLyric(source, songInfo)
  } catch (e: any) {
    throw new Error(`[plugin=${pluginId}][${source}] getLyric: ${e.message || String(e)}`)
  }
}

async function callMethod(pluginId: string, method: string, argsJson: string, options: { injectConfig?: boolean } = {}): Promise<any> {
  const plugin = await loadPlugin(pluginId)
  const fn = (plugin.exports as Record<string, any>)[method]
  if (typeof fn !== 'function') {
    throw new Error(getT()('plugin.noMethod', { method }))
  }

  let args = parseArgsJson(argsJson)
  if (options.injectConfig) {
    const configRes = await ipcCall('plugins.getConfig', { pluginId })
    if (!configRes?.success) {
      throw new Error(configRes?.error || getT()('plugin.readConfigFailed'))
    }
    args = [configRes.data || {}, ...args]
  }

  const thisArg = isCrPlugin(plugin.code) ? { cerumusic: createCerumusicApi() } : undefined
  try {
    return await invokePluginMethod(fn, thisArg, args)
  } catch (e: any) {
    const detail = e?.message || String(e)
    let argsPreview = '[unserializable]'
    try {
      argsPreview = JSON.stringify(args).slice(0, 300)
    } catch {
      argsPreview = '[unserializable]'
    }
    throw new Error(`[plugin=${pluginId}][method=${method}] ${detail} | args=${argsPreview}`)
  }
}

async function testConnection(pluginId: string): Promise<{ success: boolean; message: string }> {
  let result: any
  try {
    result = await callMethod(pluginId, 'testConnection', '[]', { injectConfig: true })
  } catch (e: any) {
    const message = e?.message || String(e)
    if (message.includes(getT()('plugin.noTestConnectionMethod'))) {
      try {
        result = await callMethod(pluginId, 'ping', '[]', { injectConfig: true })
      } catch (pingErr: any) {
        const pingMessage = pingErr?.message || String(pingErr)
        return { success: false, message: getT()('plugin.noTestConnection', { message: pingMessage }) }
      }
    } else {
      return { success: false, message }
    }
  }

  if (result && typeof result === 'object') {
    if (typeof result.success === 'boolean') {
      return { success: result.success, message: result.message || (result.success ? getT()('plugin.connectionSuccess') : getT()('plugin.connectionFailed')) }
    }
    if (result.error) {
      return { success: false, message: String(result.error) }
    }
  }

  if (typeof result === 'string') {
    return { success: true, message: result }
  }

  return { success: !!result, message: result ? getT()('plugin.connectionSuccess') : getT()('plugin.connectionFailed') }
}

function clearCache(pluginId?: string) {
  if (pluginId) {
    pluginCache.delete(pluginId)
  } else {
    pluginCache.clear()
  }
}

// ==================== 消息处理 ====================

self.onmessage = async (e: MessageEvent) => {
  const msg = e.data

  // IPC 响应
  if (msg.type === 'ipc-resolve') {
    const pending = pendingIpc.get(msg.id)
    if (pending) {
      pendingIpc.delete(msg.id)
      pending.resolve(msg.result)
    }
    return
  }
  if (msg.type === 'ipc-reject') {
    const pending = pendingIpc.get(msg.id)
    if (pending) {
      pendingIpc.delete(msg.id)
      pending.reject(new Error(msg.error))
    }
    return
  }

  // 方法调用
  if (msg.type === 'call') {
    try {
      let result: any
      switch (msg.method) {
        case 'getMusicUrl':
          result = await getMusicUrl(msg.args[0], msg.args[1], msg.args[2], msg.args[3])
          break
        case 'getPic':
          result = await getPic(msg.args[0], msg.args[1], msg.args[2])
          break
        case 'getLyric':
          result = await getLyric(msg.args[0], msg.args[1], msg.args[2])
          break
        case 'callMethod':
          result = await callMethod(msg.args[0], msg.args[1], msg.args[2], msg.args[3])
          break
        case 'testConnection':
          result = await testConnection(msg.args[0])
          break
        case 'clearCache':
          clearCache(msg.args[0])
          result = undefined
          break
        case 'loadPlugin':
          await loadPlugin(msg.args[0])
          result = undefined
          break
        case 'reloadPlugin':
          await reloadPlugin(msg.args[0])
          result = undefined
          break
        default:
          throw new Error(getT()('plugin.unknownMethod', { method: msg.method }))
      }
      self.postMessage({ type: 'resolve', id: msg.id, result })
    } catch (e: any) {
      self.postMessage({ type: 'reject', id: msg.id, error: e.message || String(e) })
    }
  }
}
