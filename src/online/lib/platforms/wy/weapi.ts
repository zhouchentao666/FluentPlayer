import * as md5Lib from "js-md5"
import * as aesjs from "aes-js"

/**
 * NetEase Cloud Music weapi signing — used by the comment endpoints
 * (`/weapi/comment/resource/comments/get`). Ported from lx-music-desktop
 * wy/utils/crypto.js (weapi). Two rounds of AES-256-CBC.
 */

// eslint-disable-next-line @typescript-eslint/no-explicit-any
const md5 = ((md5Lib as any).default ?? md5Lib) as (str: string) => string

const IV = "0102030405060708"
const FIRST_KEY = "0CoJUm6Qyw8W8jud"
const SECOND_KEY = "abcdefghijklmnop"

function aesCbc(text: string, key: string): Uint8Array {
  const keyBytes = aesjs.utils.utf8.toBytes(key)
  const ivBytes = aesjs.utils.utf8.toBytes(IV)
  const cipher = new aesjs.ModeOfOperation.cbc(keyBytes, ivBytes)
  const padded = aesjs.padding.pkcs7.pad(aesjs.utils.utf8.toBytes(text))
  return cipher.encrypt(padded)
}

function bytesToHex(buf: Uint8Array): string {
  return Array.from(buf)
    .map((b) => b.toString(16).padStart(2, "0"))
    .join("")
}

/**
 * Build the `{ params, encSecKey }` body used by `/weapi/*` endpoints.
 * `secret` is a random 16-char base62 string (per-request).
 */
export function weapi(object: unknown, secret: string): { params: string; encSecKey: string } {
  const text = typeof object === "object" ? JSON.stringify(object) : String(object)
  // round 1: encrypt with FIRST_KEY
  const round1 = aesCbc(text, FIRST_KEY)
  // round 2: encrypt round1 result with the random secret key
  const round2 = bytesToHex(aesCbc(bytesToHex(round1), secret))
  // encSecKey: encrypt secret with SECOND_KEY, reversed, no padding
  let key = secret
  const revKey = key.split("").reverse().join("")
  const encBytes = new aesjs.ModeOfOperation.cbc(
    aesjs.utils.utf8.toBytes(SECOND_KEY),
    aesjs.utils.utf8.toBytes(IV),
  ).encrypt(aesjs.utils.utf8.toBytes(revKey))
  const encSecKey = bytesToHex(encBytes)
  void md5
  return { params: round2, encSecKey }
}

const BASE62 = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789"

/** Generate a random 16-char secret for each weapi call. */
export function randomSecret(): string {
  let s = ""
  for (let i = 0; i < 16; i++) s += BASE62[Math.floor(Math.random() * BASE62.length)]
  return s
}
