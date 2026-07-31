const zh: Record<string, string> = {
  "sources.err.allFailed": "所有音源均获取失败",
  "sources.err.noEnabled": "没有可用的自定义音源，请先在设置中导入音源脚本",
  "playlists.openEmpty": "请输入歌单链接",
  "playlists.openInvalid": "无法识别的歌单链接",
}

export function t(key: string): string {
  return zh[key] ?? key
}
