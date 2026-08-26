pub mod helpers;
pub mod crypto;
pub mod search;
pub mod playback;
pub mod comment;
pub mod playlist;
pub mod singer;

pub async fn handle(method: &str, args: serde_json::Value) -> Result<serde_json::Value, String> {
    match method {
        "search" => search::search(args).await,
        "tipSearch" => search::tip_search(args).await,
        "hotSearch" => search::hot_search(args).await,
        "getMusicUrl" => playback::get_music_url(args).await,
        "getPic" => playback::get_pic(args).await,
        "getLyric" => playback::get_lyric(args).await,
        "getComment" => comment::get_comment(args).await,
        "getHotComment" => comment::get_hot_comment(args).await,
        "getHotSonglist" | "getHotPlaylists" => playlist::get_hot_songlist(args).await,
        "getPlaylistTags" | "getSongboardTags" => playlist::get_playlist_tags(args).await,
        "getCategoryPlaylists" => playlist::get_category_playlists(args).await,
        "getLeaderboards" => playlist::get_leaderboards(args).await,
        "getPlaylistDetail" | "getPlaylistDetailById" => playlist::get_playlist_detail(args).await,
        "getLeaderboardDetail" => playlist::get_leaderboard_detail(args).await,
        "searchPlaylist" => search::search_playlist(args).await,
        "getSingerInfo" => singer::get_singer_info(args).await,
        "getSingerSongList" => singer::get_singer_song_list(args).await,
        "getSingerAlbumList" => singer::get_singer_album_list(args).await,
        _ => Err(format!("Unknown SDK method for tx: {}", method)),
    }
}
