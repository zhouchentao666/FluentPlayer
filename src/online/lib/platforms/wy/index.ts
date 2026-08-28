/**
 * NetEase (wy) platform module — crypto locality lives here; feature folders
 * re-export specific ops so existing import paths keep working.
 */
export { eapi, eapiParams } from "./eapi"
export { searchWangyi } from "@online/lib/search/wy"
export { wyBoards, getWyBoardSongs } from "@online/lib/charts/wy"
export { getWyHotSearch } from "@online/lib/hotSearch/wy"
export {
  getWyPlaylistTags,
  getWyHotPlaylists,
  getWyPlaylistDetail,
} from "@online/lib/playlists/wy"
export { getWyBuiltinMusicUrl } from "@online/lib/playlists/wyUrl"
