import * as aesjs from "aes-js"

/**
 * NetEase Cloud Music weapi signing — used by the comment endpoints
 * (`/weapi/comment/resource/comments/get`) and `/weapi/song/lyric`.
 *
 * Ported from lx-music-desktop wy/utils/crypto.js (weapi), cross-checked
 * against the Go reference implementation (netease/crypto.go EncryptWeApi).
 *
 * The scheme is:
 *   params    = base64( AES-128-CBC( base64( AES-128-CBC(text, FIRST_KEY) ), secret ) )
 *   encSecKey = RSA-NoPadding( reverse(secret) )  -> 256 lowercase hex chars
 *
 * NOTE: both AES rounds emit **base64** (not hex), and `encSecKey` is a real
 * RSA "textbook" exponentiation (m^e mod n) — not an AES encryption. Getting
 * either of those wrong makes the server reply with `{"code": 250, ...}` /
 * "参数错误", which is what broke NetEase comments previously.
 */

const IV = "0102030405060708"
const FIRST_KEY = "0CoJUm6Qyw8W8jud"

// NetEase's RSA public key (1024-bit modulus, e = 65537).
const RSA_MODULUS =
  "00e0b509f6259df8642dbc35662901477df22677ec152b5ff68ace615bb7b725" +
  "152b3ab17a876aea8a5aa76d2e417629ec4ee341f56135fccf695280104e0312" +
  "ecbda92557c93870114af6c9d05c4f7f0c3685b7a46bee255932575cce10b424" +
  "d813cfe4875d3e82047b97ddef52741d546b8e289dc6935b3ece0462db0a22b8e"
const RSA_EXPONENT = "010001"

/** AES-128-CBC encrypt with PKCS#7 padding, returned as base64. */
function aesCbcB64(text: string, key: string): string {
  const cipher = new aesjs.ModeOfOperation.cbc(
    aesjs.utils.utf8.toBytes(key),
    aesjs.utils.utf8.toBytes(IV),
  )
  const encrypted = cipher.encrypt(aesjs.padding.pkcs7.pad(aesjs.utils.utf8.toBytes(text)))
  let binary = ""
  for (const b of encrypted) binary += String.fromCharCode(b)
  return btoa(binary)
}

/** Modular exponentiation base^exp mod mod, all BigInt. */
function powMod(base: bigint, exp: bigint, mod: bigint): bigint {
  let result = 1n
  base %= mod
  while (exp > 0n) {
    if (exp & 1n) result = (result * base) % mod
    base = (base * base) % mod
    exp >>= 1n
  }
  return result
}

/**
 * "Textbook" RSA (no padding) over the reversed secret, matching the Go
 * reference: reverse -> utf8 hex -> big int -> m^e mod n -> zero-padded to
 * 256 hex chars.
 */
function rsaEncrypt(text: string): string {
  const reversed = text.split("").reverse().join("")
  let hex = ""
  for (let i = 0; i < reversed.length; i++) {
    hex += reversed.charCodeAt(i).toString(16).padStart(2, "0")
  }
  const result = powMod(BigInt("0x" + hex), BigInt("0x" + RSA_EXPONENT), BigInt("0x" + RSA_MODULUS))
  return result.toString(16).padStart(256, "0")
}

/**
 * Build the `{ params, encSecKey }` body used by `/weapi/*` endpoints.
 * `secret` is a random 16-char base62 string (per-request).
 */
export function weapi(object: unknown, secret: string): { params: string; encSecKey: string } {
  const text = typeof object === "object" ? JSON.stringify(object) : String(object)
  // Round 1 with the fixed nonce key, then round 2 over that base64 output.
  const round1 = aesCbcB64(text, FIRST_KEY)
  const params = aesCbcB64(round1, secret)
  return { params, encSecKey: rsaEncrypt(secret) }
}

const BASE62 = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789"

/** Generate a random 16-char secret for each weapi call. */
export function randomSecret(): string {
  let s = ""
  const buf = new Uint8Array(16)
  crypto.getRandomValues(buf)
  for (let i = 0; i < 16; i++) s += BASE62[buf[i] % BASE62.length]
  return s
}
