import { httpFetch as tauriFetch } from "@online/lib/http"
import { weapi, randomSecret } from "@online/lib/platforms/wy/weapi"
import type { MusicInfo } from "@online/types/music"

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
  const body = weapi(
    {
      cursor: String(Date.now()),
      offset: String((page - 1) * limit),
      orderType: "1",
      pageNo: String(page),
      pageSize: String(limit),
      rid: threadId,
      threadId,
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
    data?: { comments?: any[]; totalCount?: number }
  }
  if (data.code !== 200 || !data.data) throw new Error("获取评论失败")
  const raw = data.data.comments ?? []
  const total = data.data.totalCount ?? 0
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

async function getTxComments(song: MusicInfo, page: number, limit: number): Promise<CommentPage> {
  const songId = song.meta.songId
  if (!songId) throw new Error("缺少歌曲 ID")
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

// --- entry ----------------------------------------------------------------

/**
 * Fetch online comments for a song. Currently supports NetEase (wy) and QQ (tx);
 * other platforms throw so callers can show a friendly "not supported" state.
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
    default:
      throw new Error(`平台「${song.source}」暂不支持在线评论`)
  }
}
