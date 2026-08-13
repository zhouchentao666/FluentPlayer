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

/** 评论排序方式：热门 / 最新 / 最旧。 */
export type CommentSort = "hot" | "new" | "old"

export const COMMENT_SORTS: { value: CommentSort; label: string }[] = [
  { value: "hot", label: "热门" },
  { value: "new", label: "最新" },
  { value: "old", label: "最旧" },
]

export interface CommentPage {
  source: string
  comments: CommentItem[]
  total: number
  page: number
  limit: number
  maxPage: number
  sort: CommentSort
  /**
   * 游标翻页令牌。部分平台（网易云 / 咪咕）不支持按页号随机跳页，
   * 只能凭上一页返回的游标顺序取下一页。为 null 表示没有下一页。
   */
  nextCursor?: string | null
  /** 该平台是否支持随机跳页（false 时 UI 只提供「上一页 / 下一页」）。 */
  cursorPaging?: boolean
}

/** 各平台支持的排序方式，UI 据此决定显示哪些分类按钮。 */
export function supportedSorts(source: string): CommentSort[] {
  switch (source) {
    case "wy":
      return ["hot", "new", "old"]
    case "tx":
    case "kg":
    case "kw":
    case "mg":
      return ["hot", "new"]
    default:
      return ["new"]
  }
}

export interface CommentQuery {
  page?: number
  limit?: number
  sort?: CommentSort
  /** 游标翻页平台在翻下一页时回传上一页的 nextCursor。 */
  cursor?: string | null
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

const WY_HEADERS = {
  "User-Agent":
    "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/60.0.3112.90 Safari/537.36",
  "Content-Type": "application/x-www-form-urlencoded",
  origin: "https://music.163.com",
  referer: "https://music.163.com/",
}

function mapWyComment(item: any): CommentItem {
  return {
    id: String(item.commentId),
    text: item.content ? applyWyEmoji(item.content) : "",
    userName: item.user?.nickname ?? "",
    avatar: item.user?.avatarUrl ?? null,
    timeStr: item.time ? dateFormat(item.time) : "",
    likedCount: item.likedCount ?? 0,
    location: item.ipLocation?.location ?? null,
    reply: (item.beReplied ?? []).map((r: any) => ({
      id: String(r.beRepliedCommentId ?? `${item.commentId}_re`),
      text: r.content ? applyWyEmoji(r.content) : "",
      userName: r.user?.nickname ?? "",
      avatar: r.user?.avatarUrl ?? null,
      timeStr: r.time ? dateFormat(r.time) : "",
      likedCount: r.likedCount ?? null,
    })),
  }
}

/**
 * 网易云热门评论：走独立的 hotcomments 接口，用 offset 分页。
 * comments/get 接口的 sortType 对本资源并不生效（实测排序不变），
 * 因此热门必须单独取，否则「热门」和「最新」会返回一模一样的内容。
 */
async function getWyHotComments(threadId: string, page: number, limit: number): Promise<CommentPage> {
  const body = weapi(
    { rid: threadId, limit, offset: (page - 1) * limit, beforeTime: "0" },
    randomSecret(),
  )
  const res = await tauriFetch(`https://music.163.com/weapi/v1/resource/hotcomments/${threadId}`, {
    method: "POST",
    headers: WY_HEADERS,
    body: formBody(body),
  })
  if (!res.ok) throw new Error("获取热门评论失败")
  const data = (await res.json()) as {
    code?: number
    hotComments?: any[]
    total?: number
    hasMore?: boolean
  }
  if (data.code !== 200) throw new Error("获取热门评论失败")
  const total = data.total ?? 0
  return {
    source: "wy",
    comments: (data.hotComments ?? []).map(mapWyComment),
    total,
    page,
    limit,
    maxPage: Math.ceil(total / limit) || 1,
    sort: "hot",
    cursorPaging: false,
    nextCursor: null,
  }
}

/**
 * 网易云最新 / 最旧评论。
 *
 * 关键点（实测）：该接口完全靠 cursor + orderType 决定顺序，sortType 无效。
 *   - 最新：orderType=1，首页 cursor 必须为当前毫秒时间戳；传 "0" 会返回空列表。
 *   - 最旧：orderType=0，首页 cursor 传 "1"（即从最早时间往后取）。
 * 翻页时使用上一次响应里的 data.cursor（是字符串，不是 [prev,next] 数组，
 * 旧实现按数组取 [1] 永远拿到 undefined，导致第二页开始重复/为空）。
 */
async function getWyTimeComments(
  threadId: string,
  page: number,
  limit: number,
  sort: "new" | "old",
  cursor: string | null,
): Promise<CommentPage> {
  const orderType = sort === "new" ? 1 : 0
  const startCursor = sort === "new" ? String(Date.now()) : "1"
  const useCursor = page > 1 && cursor ? cursor : startCursor
  const body = weapi(
    {
      rid: threadId,
      threadId,
      cursor: useCursor,
      offset: 0,
      orderType,
      pageNo: page,
      pageSize: limit,
    },
    randomSecret(),
  )
  const res = await tauriFetch("https://music.163.com/weapi/comment/resource/comments/get", {
    method: "POST",
    headers: WY_HEADERS,
    body: formBody(body),
  })
  if (!res.ok) throw new Error("获取评论失败")
  const data = (await res.json()) as {
    code?: number
    data?: { comments?: any[]; totalCount?: number; cursor?: unknown; hasMore?: boolean }
  }
  if (data.code !== 200 || !data.data) throw new Error("获取评论失败")
  const raw = data.data.comments ?? []
  const total = data.data.totalCount ?? 0
  // 响应 cursor 为字符串；个别情况下可能是数组，做一次兼容。
  const respCursor = data.data.cursor
  let nextCursor: string | null = null
  if (Array.isArray(respCursor)) nextCursor = respCursor[respCursor.length - 1] != null ? String(respCursor[respCursor.length - 1]) : null
  else if (respCursor != null && respCursor !== "") nextCursor = String(respCursor)
  if (data.data.hasMore === false || raw.length === 0) nextCursor = null
  return {
    source: "wy",
    comments: raw.map(mapWyComment),
    total,
    page,
    limit,
    maxPage: Math.ceil(total / limit) || 1,
    sort,
    cursorPaging: true,
    nextCursor,
  }
}

async function getWyComments(
  song: MusicInfo,
  page: number,
  limit: number,
  sort: CommentSort,
  cursor: string | null,
): Promise<CommentPage> {
  const songId = song.meta.songId
  if (!songId) throw new Error("缺少歌曲 ID")
  const threadId = `R_SO_4_${songId}`
  if (sort === "hot") return getWyHotComments(threadId, page, limit)
  return getWyTimeComments(threadId, page, limit, sort, cursor)
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

/** QQ 热门评论：musicu.fcg 的 GetHotCommentList 模块。 */
async function getTxHotComments(song: MusicInfo, page: number, limit: number): Promise<CommentPage> {
  const songId = await getTxSongId(song)
  const body = {
    comm: {
      cv: 4747474,
      ct: 24,
      format: "json",
      inCharset: "utf-8",
      outCharset: "utf-8",
      notice: 0,
      platform: "yqq.json",
      needNewCode: 1,
      uin: 0,
    },
    req: {
      module: "music.globalComment.CommentRead",
      method: "GetHotCommentList",
      param: {
        BizType: 1,
        BizId: String(songId),
        LastCommentSeqNo: "",
        PageSize: limit,
        PageNum: page - 1,
        HotType: 1,
        WithAirborne: 0,
        PicEnable: 1,
      },
    },
  }
  const res = await tauriFetch("https://u.y.qq.com/cgi-bin/musicu.fcg", {
    method: "POST",
    headers: {
      "User-Agent":
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/113.0.0.0 Safari/537.36",
      "Content-Type": "application/json",
      referer: "https://y.qq.com/",
      origin: "https://y.qq.com",
    },
    body: JSON.stringify(body),
  })
  if (!res.ok) throw new Error("获取热门评论失败")
  const data = (await res.json()) as any
  if (data?.code !== 0 || data?.req?.code !== 0) throw new Error("获取热门评论失败")
  const list = data.req.data?.CommentList
  const total = list?.Total ?? 0
  const comments: CommentItem[] = (list?.Comments ?? []).map((item: any) => {
    const time = txFormatTime(item.PubTime)
    return {
      id: String(item.CommentId ?? item.SeqNo),
      text: item.Content ? replaceTxEmoji(item.Content).replace(/\\n/g, "\n") : "",
      userName: item.Nick ?? "",
      avatar: item.Avatar ?? null,
      timeStr: time ? dateFormat(time) : "",
      likedCount: item.PraiseNum ?? 0,
      location: item.LocationInfo?.Name ?? null,
      reply: [],
    }
  })
  return {
    source: "tx",
    comments,
    total,
    page,
    limit,
    maxPage: Math.ceil(total / limit) || 1,
    sort: "hot",
    cursorPaging: false,
  }
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
  return {
    source: "tx",
    comments,
    total,
    page,
    limit,
    maxPage: Math.ceil(total / limit) || 1,
    sort: "new",
    cursorPaging: false,
  }
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

async function getKgComments(
  song: MusicInfo,
  page: number,
  limit: number,
  sort: CommentSort,
): Promise<CommentPage> {
  const hash = song.meta.hash
  if (!hash) throw new Error("缺少歌曲 hash")
  const timestamp = Math.floor(Date.now() / 1000)
  const params = `dfid=0&mid=16249512204336365674023395779019&clienttime=${timestamp}&uuid=0&extdata=${hash}&appid=1005&code=fc4be23b4e972707f36b8a828a93ba8a&schash=${hash}&clientver=11409&p=${page}&clienttoken=&pagesize=${limit}&ver=10&kugouid=0`
  // 酷狗用两个不同的排行接口区分热门与最新。
  const path = sort === "hot" ? "rank/topliked" : "rank/newest"
  const url = `http://m.comment.service.kugou.com/r/v1/${path}?${params}&signature=${kgSignature(params)}`
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
  return {
    source: "kg",
    comments,
    total,
    page,
    limit,
    maxPage: Math.ceil(total / limit) || 1,
    sort,
    cursorPaging: false,
  }
}

function replaceAt(raw: string, atList: { id: string; name: string }[]): string {
  let s = raw || ""
  for (const at of atList) s = s.split(`[at=${at.id}]`).join(`@${at.name} `)
  return s
}

// --- Kuwo (kw) ------------------------------------------------------------

async function getKwComments(
  song: MusicInfo,
  page: number,
  limit: number,
  sort: CommentSort,
): Promise<CommentPage> {
  const songmid = song.meta.songId
  if (!songmid) throw new Error("缺少歌曲 ID")
  // 酷我热门评论用 get_rec_comment（推荐/精彩评论），最新用 get_comment。
  const type = sort === "hot" ? "get_rec_comment" : "get_comment"
  const url = `https://ncomment.kuwo.cn/com.s?f=web&type=${type}&aapiver=1&prod=kwplayer_ar_10.5.2.0&digest=15&sid=${songmid}&start=${limit * (page - 1)}&msgflag=1&count=${limit}&newver=3&uid=0`
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
  return {
    source: "kw",
    comments,
    total,
    page,
    limit,
    maxPage: Math.ceil(total / limit) || 1,
    sort,
    cursorPaging: false,
  }
}

// --- Migu (mg) ------------------------------------------------------------

async function getMgComments(
  song: MusicInfo,
  page: number,
  limit: number,
  sort: CommentSort,
  cursor: string | null,
): Promise<CommentPage> {
  const songId = song.meta.songId
  if (!songId) throw new Error("缺少歌曲 ID")
  const isHot = sort === "hot"
  // 咪咕：热门用 queryType=2 + hotCommentStart 偏移；
  // 最新用 queryType=1，只能靠上一页最后一条 commentId 做游标顺序翻页。
  const url = isHot
    ? `https://app.c.nf.migu.cn/MIGUM3.0/user/comment/stack/v1.0?pageSize=${limit}&queryType=2&resourceId=${songId}&resourceType=2&hotCommentStart=${(page - 1) * limit}`
    : `https://app.c.nf.migu.cn/MIGUM3.0/user/comment/stack/v1.0?pageSize=${limit}&queryType=1&resourceId=${songId}&resourceType=2&commentId=${page > 1 && cursor ? cursor : ""}`
  const res = await tauriFetch(url, {
    headers: {
      "User-Agent":
        "Mozilla/5.0 (iPhone; CPU iPhone OS 13_2_3 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/13.0.3 Mobile/15E148 Safari/604.1",
    },
  })
  if (!res.ok) throw new Error("获取评论失败")
  const data = (await res.json()) as {
    code?: string
    data?: { commentNums?: string; cfgHotCount?: string | number; comments?: any[]; hotComments?: any[] }
  }
  if (data.code !== "000000" || !data.data) throw new Error("获取评论失败")
  const total = isHot
    ? parseInt(String(data.data.cfgHotCount ?? "0")) || 0
    : parseInt(data.data.commentNums ?? "0") || 0
  const rawList = (isHot ? data.data.hotComments : data.data.comments) ?? []
  const comments: CommentItem[] = rawList.map((item) => ({
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
  return {
    source: "mg",
    comments,
    total,
    page,
    limit,
    maxPage: Math.ceil(total / limit) || 1,
    sort,
    cursorPaging: !isHot,
    nextCursor: isHot ? null : comments.length ? comments[comments.length - 1].id : null,
  }
}

// --- entry ----------------------------------------------------------------

/**
 * Fetch online comments for a song. Supports NetEase (wy), QQ (tx), Kugou (kg),
 * Kuwo (kw) and Migu (mg). Unsupported platforms throw so callers can show a
 * friendly "not supported" state.
 */
export async function getComments(song: MusicInfo, query: CommentQuery = {}): Promise<CommentPage> {
  const page = query.page ?? 1
  const limit = query.limit ?? 20
  const cursor = query.cursor ?? null
  // 平台不支持所请求的排序时回退到它支持的第一种，避免 UI 抛错。
  const allowed = supportedSorts(song.source)
  const sort: CommentSort = query.sort && allowed.includes(query.sort) ? query.sort : allowed[0]

  switch (song.source) {
    case "wy":
      return getWyComments(song, page, limit, sort, cursor)
    case "tx":
      return sort === "hot"
        ? getTxHotComments(song, page, limit)
        : getTxComments(song, page, limit)
    case "kg":
      return getKgComments(song, page, limit, sort)
    case "kw":
      return getKwComments(song, page, limit, sort)
    case "mg":
      return getMgComments(song, page, limit, sort, cursor)
    default:
      throw new Error(`平台「${song.source}」暂不支持在线评论`)
  }
}
