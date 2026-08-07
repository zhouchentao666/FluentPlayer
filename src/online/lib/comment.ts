import { httpFetch as tauriFetch } from "@online/lib/http"
import { weapi, randomSecret } from "@online/lib/platforms/wy/weapi"
import * as md5Lib from "js-md5"
import type { MusicInfo } from "@online/types/music"

// js-md5 在不同版本下 default 导出位置不同，统一取函数。
const md5 = ((md5Lib as any).default ?? md5Lib) as (str: string) => string

// Online comments (NetEase "wy" + QQ "tx"). Both endpoints are public and need
// no login. The result is normalised into a shared CommentItem shape so the UI
// doesn't care which platform served the data.

export interface CommentReply {
  id: string
  text: string
  userName: string
  avatar?: string | null
  timeStr?: string | null
  likedCount?: number | null
}

export interface CommentItem {
  id: string
  text: string
  userName: string
  avatar?: string | null
  timeStr?: string | null
  likedCount?: number | null
  location?: string | null
  reply?: CommentReply[]
}

export interface CommentPage {
  source: string
  comments: CommentItem[]
  total: number
  page: number
  limit: number
  maxPage: number
}

// --- helpers --------------------------------------------------------------

function formBody(obj: Record<string, string | number>): string {
  const p = new URLSearchParams()
  for (const [k, v] of Object.entries(obj)) p.set(k, String(v))
  return p.toString()
}

const WY_EMOJIS: Record<string, string> = {
  大笑: "😃", 可爱: "😊", 憨笑: "☺️", 色: "😍", 亲亲: "😙", 惊恐: "😱", 流泪: "😭",
  亲: "😚", 呆: "😳", 哀伤: "😔", 呲牙: "😁", 吐舌: "😝", 撇嘴: "😒", 怒: "😡",
  奸笑: "😏", 汗: "😓", 痛苦: "😖", 惶恐: "😰", 生病: "😨", 口罩: "😷", 大哭: "😂",
  晕: "😵", 发怒: "👿", 开心: "😄", 鬼脸: "😜", 皱眉: "😞", 流感: "😢", 爱心: "❤️",
  心碎: "💔", 钟情: "💘", 星星: "⭐️", 生气: "💢", 便便: "💩", 强: "👍", 弱: "👎",
  拜: "🙏", 牵手: "👫", 跳舞: "👯‍♀️", 禁止: "🙅‍♀️", 这边: "💁‍♀️", 爱意: "💏",
  示爱: "👩‍❤️‍👨", 嘴唇: "👄", 狗: "🐶", 猫: "🐱", 猪: "🐷", 兔子: "🐰", 小鸡: "🐤",
  公鸡: "🐔", 幽灵: "👻", 圣诞: "🎅", 外星: "👽", 钻石: "💎", 礼物: "🎁", 男孩: "👦",
  女孩: "👧", 蛋糕: "🎂", 18: "🔞", 圈: "⭕", 叉: "❌",
}

function applyWyEmoji(text: string): string {
  if (!text) return text
  for (const [k, v] of Object.entries(WY_EMOJIS)) text = text.split(`[${k}]`).join(v)
  return text
}

function dateFormat(ts: number | string): string {
  const t = typeof ts === "number" ? ts : parseInt(ts)
  if (!Number.isFinite(t) || t <= 0) return ""
  const d = new Date(t < 1e12 ? t * 1000 : t)
  if (Number.isNaN(d.getTime())) return ""
  const pad = (n: number) => String(n).padStart(2, "0")
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}`
}

// --- NetEase --------------------------------------------------------------

async function getWyComments(song: MusicInfo, page: number, limit: number): Promise<CommentPage> {
  const songId = song.meta.songId
  if (!songId) throw new Error("缺少歌曲 ID")
  const threadId = `R_SO_4_${songId}`
  // 网易云评论分页：/weapi/comment/resource/comments/get 使用 cursor 翻页（非 page 偏移）。
  // cursor 为毫秒时间戳：首页必须传 0（传 Date.now() 会请求当前时间之后的评论，
  // 导致首页拿不到数据）；翻页用上一次响应返回的 data.cursor（[prev, next] 数组的 next）。
  // 重新打开评论（page<=1）时清空缓存的游标，避免复用上次会话的旧游标。
  if (page <= 1) (song as any).__wyCursor = null
  const prevCursor: number | null = (song as any).__wyCursor ?? null
  const cursor = prevCursor ?? 0
  const body = weapi(
    {
      rid: threadId,
      threadId,
      cursor: String(cursor),
      offset: "0",
      orderType: 1,
      pageNo: "1",
      pageSize: String(limit),
      csrf_token: "",
    },
    randomSecret(),
  )
  const res = await tauriFetch("https://music.163.com/weapi/comment/resource/comments/get", {
    method: "POST",
    headers: {
      "User-Agent":
        "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/60.0.3112.90 Safari/537.36",
      "Content-Type": "application/x-www-form-urlencoded",
      origin: "https://music.163.com",
      referer: "https://music.163.com/",
    },
    body: formBody(body),
  })
  if (!res.ok) throw new Error("获取评论失败")
  const data = (await res.json()) as {
    code?: number
    data?: { comments?: any[]; totalCount?: number; cursor?: number[] | number | string; hasMore?: boolean }
  }
  if (data.code !== 200 || !data.data) throw new Error("获取评论失败")
  const raw = data.data.comments ?? []
  const total = data.data.totalCount ?? 0
  // 记录下一页游标（响应 cursor 为 [上一页, 下一页] 数组，取下一项）。
  const respCursor = data.data.cursor
  if (Array.isArray(respCursor)) {
    ;(song as any).__wyCursor = respCursor[1] ?? cursor
  } else if (respCursor != null) {
    ;(song as any).__wyCursor = respCursor
  } else {
    ;(song as any).__wyCursor = cursor
  }
  const comments: CommentItem[] = raw.map((item) => ({
    id: String(item.commentId),
    text: item.content ? applyWyEmoji(item.content) : "",
    userName: item.user?.nickname ?? "",
    avatar: item.user?.avatarUrl ?? null,
    timeStr: item.time ? dateFormat(item.time) : "",
    likedCount: item.likedCount ?? 0,
    location: item.ipLocation?.location ?? null,
    reply: [],
  }))
  return { source: "wy", comments, total, page, limit, maxPage: Math.ceil(total / limit) || 1 }
}

// --- QQ --------------------------------------------------------------------

const TX_EMOJIS: Record<string, string> = {
  e400846: "😘", e400874: "😴", e400825: "😃", e400847: "😙", e400835: "😍",
  e400873: "😳", e400836: "😎", e400867: "😭", e400832: "😊", e400837: "😏",
  e400875: "😫", e400831: "😉", e400855: "😡", e400823: "😄", e400862: "😨",
  e400844: "😖", e400841: "😓", e400830: "😈", e400828: "😆", e400833: "😋",
  e400822: "😀", e400843: "😕", e400829: "😇", e400824: "😂", e400834: "😌",
  e400877: "😷", e400132: "🍉", e400181: "🍺", e401067: "☕️", e400186: "🥧",
  e400343: "🐷", e400116: "🌹", e400126: "🍃", e400613: "💋", e401236: "❤️",
  e400622: "💔", e400637: "💣", e400643: "💩", e400773: "🔪", e400102: "🌛",
  e401328: "🌞", e400420: "👏", e400914: "🙌", e400408: "👍", e400414: "👎",
  e401121: "✋", e400396: "👋", e400384: "👉", e401115: "✊", e400402: "👌",
  e400905: "🙈", e400906: "🙉", e400907: "🙊", e400562: "👻", e400932: "🙏",
  e400644: "💪", e400611: "💉", e400185: "🎁", e400655: "💰", e400325: "🐥",
  e400612: "💊", e400198: "🎉", e401685: "⚡️", e400631: "💝", e400768: "🔥",
  e400432: "👑",
}

function replaceTxEmoji(msg: string): string {
  if (!msg) return msg
  const result = msg.match(/\[em\]e\d+\[\/em\]/g)
  if (!result) return msg
  const rxp = /^\[em\](e\d+)\[\/em\]$/
  for (const item of Array.from(new Set(result))) {
    const code = item.replace(rxp, "$1")
    msg = msg.split(item).join(TX_EMOJIS[code] ?? "")
  }
  return msg
}

function txFormatTime(time: unknown): number | null {
  const s = String(time)
  if (s.length < 10) return null
  return parseInt(s + "000")
}

// QQ 评论接口 topid 需要数字 songId；meta.songId 实际存的是 songmid（字母数字 mid），
// 必须先用 musicu.fcg 解析出数字 songId（注意：songmid 取 meta.songId，而非 strMediaMid）。
async function getTxSongId(song: MusicInfo): Promise<string> {
  const s = song.meta.songId
  // 已经是纯数字 ID 时直接复用（如从歌曲详情进来的情况）。
  if (s && /^\d+$/.test(s)) return s
  // 真正用于解析的 mid 是 meta.songId（songmid），strMediaMid 是音频文件 mid，二者不同。
  const songmid = s || song.meta.strMediaMid
  if (!songmid) throw new Error("缺少歌曲 mid")
  const body = {
    comm: { ct: "19", cv: "1859", uin: "0" },
    req: {
      module: "music.pf_song_detail_svr",
      method: "get_song_detail_yqq",
      param: { song_type: 0, song_mid: songmid },
    },
  }
  const res = await tauriFetch("https://u.y.qq.com/cgi-bin/musicu.fcg", {
    method: "POST",
    headers: {
      "User-Agent": "Mozilla/5.0 (compatible; MSIE 9.0; Windows NT 6.1; WOW64; Trident/5.0)",
      "Content-Type": "application/json",
    },
    body: JSON.stringify(body),
  })
  if (!res.ok) throw new Error("获取歌曲 ID 失败")
  const data = (await res.json()) as any
  const id = data?.req?.data?.track_info?.id
  if (!id) throw new Error("解析歌曲 ID 失败")
  return String(id)
}

async function getTxComments(song: MusicInfo, page: number, limit: number): Promise<CommentPage> {
  const songId = await getTxSongId(song)
  const res = await tauriFetch("https://c.y.qq.com/base/fcgi-bin/fcg_global_comment_h5.fcg", {
    method: "POST",
    headers: {
      "User-Agent": "Mozilla/5.0 (compatible; MSIE 9.0; Windows NT 6.1; WOW64; Trident/5.0)",
      "Content-Type": "application/x-www-form-urlencoded",
    },
    body: formBody({
      uin: "0",
      format: "json",
      cid: "205360772",
      reqtype: "2",
      biztype: "1",
      topid: songId,
      cmd: "8",
      needmusiccrit: "1",
      pagenum: String(page - 1),
      pagesize: String(limit),
    }),
  })
  if (!res.ok) throw new Error("获取评论失败")
  const data = (await res.json()) as {
    code?: number
    comment?: { commentlist?: any[]; commenttotal?: number }
  }
  if (data.code !== 0 || !data.comment) throw new Error("获取评论失败")
  const list = data.comment.commentlist ?? []
  const total = data.comment.commenttotal ?? 0
  const comments: CommentItem[] = list.map((item) => {
    const time = txFormatTime(item.time)
    const reply: CommentReply[] = (item.middlecommentcontent ?? []).map((c: any) => ({
      id: `sub_${item.rootcommentid}_${c.subcommentid}`,
      text: c.subcommentcontent ? replaceTxEmoji(c.subcommentcontent).replace(/\\n/g, "\n") : "",
      userName: (c.replynick ?? "").substring(1),
      avatar: c.avatarurl ?? null,
      timeStr: time ? dateFormat(time) : "",
      likedCount: c.praisenum ?? 0,
    }))
    return {
      id: `${item.rootcommentid}_${item.commentid}`,
      text: item.rootcommentcontent ? replaceTxEmoji(item.rootcommentcontent).replace(/\\n/g, "\n") : "",
      userName: (item.rootcommentnick ?? "").substring(1),
      avatar: item.avatarurl ?? null,
      timeStr: time ? dateFormat(time) : "",
      likedCount: item.praisenum ?? 0,
      location: null,
      reply,
    }
  })
  return { source: "tx", comments, total, page, limit, maxPage: Math.ceil(total / limit) || 1 }
}

// --- Kugou (kg) -----------------------------------------------------------

// 酷狗评论接口签名：Mio-Music 使用 android key（与 web key 不同）。
const KG_WEB_KEY = "OIlwieks28dk2k092lksi2UIkp"

function kgSignature(params: string): string {
  const paramList = params.split("&").sort()
  const signParams = `${KG_WEB_KEY}${paramList.join("")}${KG_WEB_KEY}`
  return md5(signParams)
}

function kgFormatTime(ts: number | string): number | null {
  const t = typeof ts === "number" ? ts : parseInt(ts)
  if (!Number.isFinite(t)) return null
  return t < 1e12 ? t * 1000 : t
}

async function getKgComments(song: MusicInfo, page: number, limit: number): Promise<CommentPage> {
  const hash = song.meta.hash
  if (!hash) throw new Error("缺少歌曲 hash")
  const timestamp = Math.floor(Date.now() / 1000)
  const params = `dfid=0&mid=16249512204336365674023395779019&clienttime=${timestamp}&uuid=0&extdata=${hash}&appid=1005&code=fc4be23b4e972707f36b8a828a93ba8a&schash=${hash}&clientver=11409&p=${page}&clienttoken=&pagesize=${limit}&ver=10&kugouid=0`
  const url = `http://m.comment.service.kugou.com/r/v1/rank/newest?${params}&signature=${kgSignature(params)}`
  const res = await tauriFetch(url, {
    headers: {
      "User-Agent":
        "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/107.0.0.0 Safari/537.36 Edg/107.0.1418.24",
    },
  })
  if (!res.ok) throw new Error("获取评论失败")
  const data = (await res.json()) as { err_code?: number; count?: number; list?: any[] }
  if (data.err_code !== 0) throw new Error("获取评论失败")
  const total = data.count ?? 0
  const comments: CommentItem[] = (data.list ?? []).map((item) => ({
    id: String(item.id),
    text: item.atlist ? replaceAt(item.content, item.atlist) : item.content || "",
    userName: item.user_name || "",
    avatar: item.user_pic || null,
    timeStr: item.addtime ? dateFormat(kgFormatTime(item.addtime) ?? 0) : "",
    likedCount: item.like?.likenum ?? 0,
    location: item.location || null,
    reply: [],
  }))
  return { source: "kg", comments, total, page, limit, maxPage: Math.ceil(total / limit) || 1 }
}

function replaceAt(raw: string, atList: { id: string; name: string }[]): string {
  let s = raw || ""
  for (const at of atList) s = s.split(`[at=${at.id}]`).join(`@${at.name} `)
  return s
}

// --- Kuwo (kw) ------------------------------------------------------------

async function getKwComments(song: MusicInfo, page: number, limit: number): Promise<CommentPage> {
  const songmid = song.meta.songId
  if (!songmid) throw new Error("缺少歌曲 ID")
  const url = `https://ncomment.kuwo.cn/com.s?f=web&type=get_comment&aapiver=1&prod=kwplayer_ar_10.5.2.0&digest=15&sid=${songmid}&start=${limit * (page - 1)}&msgflag=1&count=${limit}&newver=3&uid=0`
  const res = await tauriFetch(url, {
    headers: { "User-Agent": "Dalvik/2.1.0 (Linux; U; Android 9;)" },
  })
  if (!res.ok) throw new Error("获取评论失败")
  const data = (await res.json()) as { code?: string; comments_counts?: number; comments?: any[] }
  if (data.code !== "200") throw new Error("获取评论失败")
  const total = data.comments_counts ?? 0
  const comments: CommentItem[] = (data.comments ?? []).map((item) => ({
    id: String(item.id),
    text: item.msg || "",
    userName: item.u_name || "",
    avatar: item.u_pic || null,
    timeStr: item.time ? dateFormat(Number(item.time) * 1000) : "",
    likedCount: item.like_num ?? 0,
    location: null,
    reply: (item.child_comments ?? []).map((c: any) => ({
      id: String(c.id),
      text: c.msg || "",
      userName: c.u_name || "",
      avatar: c.u_pic || null,
      timeStr: c.time ? dateFormat(Number(c.time) * 1000) : "",
      likedCount: c.like_num ?? 0,
    })),
  }))
  return { source: "kw", comments, total, page, limit, maxPage: Math.ceil(total / limit) || 1 }
}

// --- Migu (mg) ------------------------------------------------------------

async function getMgComments(song: MusicInfo, page: number, limit: number): Promise<CommentPage> {
  const songId = song.meta.songId
  if (!songId) throw new Error("缺少歌曲 ID")
  const url = `https://app.c.nf.migu.cn/MIGUM3.0/user/comment/stack/v1.0?pageSize=${limit}&queryType=1&resourceId=${songId}&resourceType=2&commentId=`
  const res = await tauriFetch(url, {
    headers: {
      "User-Agent":
        "Mozilla/5.0 (iPhone; CPU iPhone OS 13_2_3 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/13.0.3 Mobile/15E148 Safari/604.1",
    },
  })
  if (!res.ok) throw new Error("获取评论失败")
  const data = (await res.json()) as { code?: string; data?: { commentNums?: string; comments?: any[] } }
  if (data.code !== "000000" || !data.data) throw new Error("获取评论失败")
  const total = parseInt(data.data.commentNums ?? "0") || 0
  const comments: CommentItem[] = (data.data.comments ?? []).map((item) => ({
    id: String(item.commentId),
    text: item.commentInfo || "",
    userName: item.user?.nickName || "",
    avatar: item.user?.middleIcon || item.user?.bigIcon || item.user?.smallIcon || null,
    timeStr: item.commentTime ? dateFormat(new Date(item.commentTime).getTime()) : "",
    likedCount: item.opNumItem?.thumbNum ?? 0,
    location: null,
    reply: (item.replyComments ?? []).map((c: any) => ({
      id: String(c.replyId),
      text: c.replyInfo || "",
      userName: c.user?.nickName || "",
      avatar: c.user?.middleIcon || c.user?.bigIcon || c.user?.smallIcon || null,
      timeStr: c.replyTime ? dateFormat(new Date(c.replyTime).getTime()) : "",
      likedCount: null,
    })),
  }))
  return { source: "mg", comments, total, page, limit, maxPage: Math.ceil(total / limit) || 1 }
}

// --- entry ----------------------------------------------------------------

/**
 * Fetch online comments for a song. Supports NetEase (wy), QQ (tx), Kugou (kg),
 * Kuwo (kw) and Migu (mg). Unsupported platforms throw so callers can show a
 * friendly "not supported" state.
 */
export async function getComments(
  song: MusicInfo,
  page = 1,
  limit = 20,
): Promise<CommentPage> {
  switch (song.source) {
    case "wy":
      return getWyComments(song, page, limit)
    case "tx":
      return getTxComments(song, page, limit)
    case "kg":
      return getKgComments(song, page, limit)
    case "kw":
      return getKwComments(song, page, limit)
    case "mg":
      return getMgComments(song, page, limit)
    default:
      throw new Error(`平台「${song.source}」暂不支持在线评论`)
  }
}
