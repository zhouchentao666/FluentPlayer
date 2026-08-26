use rusqlite::{params, Connection, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaylistRow {
    pub id: String,
    pub name: String,
    pub description: String,
    pub cover_img_url: String,
    pub source: String,
    pub meta: String,
    pub create_time: String,
    pub update_time: String,
    #[serde(default)]
    pub song_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaylistSongRow {
    #[serde(default)]
    pub playlist_id: String,
    #[serde(default)]
    pub position: i64,
    #[serde(default)]
    pub data: String,
    pub songmid: String,
    pub name: String,
    #[serde(default)]
    pub singer: String,
    #[serde(default, rename = "albumName")]
    pub album_name: String,
    #[serde(default)]
    pub img: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaylistBackupItem {
    pub playlist: PlaylistRow,
    pub songs: Vec<PlaylistSongRow>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaylistBackup {
    pub playlists: Vec<PlaylistBackupItem>,
}

pub fn init_tables(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS playlists (
            id           TEXT PRIMARY KEY,
            name         TEXT NOT NULL,
            description  TEXT DEFAULT '',
            coverImgUrl  TEXT DEFAULT 'default-cover',
            source       TEXT NOT NULL,
            meta         TEXT DEFAULT '{}',
            createTime   TEXT NOT NULL,
            updateTime   TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS playlist_songs (
            playlist_id  TEXT NOT NULL,
            songmid      TEXT NOT NULL,
            position     INTEGER NOT NULL,
            data         TEXT NOT NULL,
            name         TEXT DEFAULT '',
            singer       TEXT DEFAULT '',
            albumName    TEXT DEFAULT '',
            img          TEXT DEFAULT '',
            PRIMARY KEY (playlist_id, songmid),
            FOREIGN KEY (playlist_id) REFERENCES playlists(id) ON DELETE CASCADE
        );

        CREATE INDEX IF NOT EXISTS idx_playlist_songs_position ON playlist_songs(playlist_id, position);
        CREATE INDEX IF NOT EXISTS idx_playlist_songs_name ON playlist_songs(playlist_id, name);",
    )?;
    Ok(())
}

// --- Playlist CRUD ---

pub fn list_playlists(conn: &Connection) -> Result<Vec<PlaylistRow>> {
    let mut stmt = conn.prepare(
        "SELECT p.id, p.name, p.description, p.coverImgUrl, p.source, p.meta, p.createTime, p.updateTime, \
         (SELECT COUNT(*) FROM playlist_songs WHERE playlist_id = p.id) FROM playlists p ORDER BY p.createTime"
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(PlaylistRow {
            id: row.get(0)?,
            name: row.get(1)?,
            description: row.get(2)?,
            cover_img_url: row.get(3)?,
            source: row.get(4)?,
            meta: row.get(5)?,
            create_time: row.get(6)?,
            update_time: row.get(7)?,
            song_count: row.get(8)?,
        })
    })?;
    rows.collect()
}

pub fn get_playlist(conn: &Connection, id: &str) -> Result<Option<PlaylistRow>> {
    let mut stmt = conn.prepare(
        "SELECT p.id, p.name, p.description, p.coverImgUrl, p.source, p.meta, p.createTime, p.updateTime, \
         (SELECT COUNT(*) FROM playlist_songs WHERE playlist_id = p.id) FROM playlists p WHERE p.id = ?1"
    )?;
    let mut rows = stmt.query_map([id], |row| {
        Ok(PlaylistRow {
            id: row.get(0)?,
            name: row.get(1)?,
            description: row.get(2)?,
            cover_img_url: row.get(3)?,
            source: row.get(4)?,
            meta: row.get(5)?,
            create_time: row.get(6)?,
            update_time: row.get(7)?,
            song_count: row.get(8)?,
        })
    })?;
    match rows.next() {
        Some(row) => Ok(Some(row?)),
        None => Ok(None),
    }
}

pub fn insert_playlist(conn: &Connection, p: &PlaylistRow) -> Result<()> {
    conn.execute(
        "INSERT INTO playlists (id, name, description, coverImgUrl, source, meta, createTime, updateTime)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
        params![p.id, p.name, p.description, p.cover_img_url, p.source, p.meta, p.create_time, p.update_time],
    )?;
    Ok(())
}

pub fn delete_playlist(conn: &Connection, id: &str) -> Result<usize> {
    // Songs are cascade-deleted via FK
    conn.execute("DELETE FROM playlists WHERE id = ?1", [id])
}

pub fn update_playlist(conn: &Connection, id: &str, name: &str, description: &str) -> Result<usize> {
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "UPDATE playlists SET name = ?1, description = ?2, updateTime = ?3 WHERE id = ?4",
        params![name, description, now, id],
    )
}

pub fn update_cover(conn: &Connection, id: &str, cover_url: &str) -> Result<usize> {
    conn.execute(
        "UPDATE playlists SET coverImgUrl = ?1 WHERE id = ?2",
        params![cover_url, id],
    )
}

pub fn playlist_exists(conn: &Connection, id: &str) -> Result<bool> {
    let mut stmt = conn.prepare("SELECT 1 FROM playlists WHERE id = ?1")?;
    let mut rows = stmt.query_map([id], |_| Ok(true))?;
    match rows.next() {
        Some(Ok(true)) => Ok(true),
        _ => Ok(false),
    }
}

// --- Playlist Songs CRUD ---

pub fn list_songs(conn: &Connection, playlist_id: &str) -> Result<Vec<PlaylistSongRow>> {
    let mut stmt = conn.prepare(
        "SELECT playlist_id, songmid, position, data, name, singer, albumName, img
         FROM playlist_songs WHERE playlist_id = ?1 ORDER BY position"
    )?;
    let rows = stmt.query_map([playlist_id], |row| {
        Ok(PlaylistSongRow {
            playlist_id: row.get(0)?,
            songmid: row.get(1)?,
            position: row.get(2)?,
            data: row.get(3)?,
            name: row.get(4)?,
            singer: row.get(5)?,
            album_name: row.get(6)?,
            img: row.get(7)?,
        })
    })?;
    rows.collect()
}

pub fn export_playlists(conn: &Connection) -> Result<PlaylistBackup> {
    let playlists = list_playlists(conn)?
        .into_iter()
        .map(|playlist| {
            let songs = list_songs(conn, &playlist.id)?;
            Ok(PlaylistBackupItem { playlist, songs })
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(PlaylistBackup { playlists })
}

pub fn restore_playlists(
    conn: &mut Connection,
    backup: &PlaylistBackup,
    mode: &str,
) -> Result<()> {
    if mode != "overwrite" && mode != "merge" {
        return Err(rusqlite::Error::InvalidParameterName(mode.to_string()));
    }

    let tx = conn.transaction()?;
    if mode == "overwrite" {
        tx.execute("DELETE FROM playlist_songs", [])?;
        tx.execute("DELETE FROM playlists", [])?;
    }

    for item in &backup.playlists {
        let playlist = &item.playlist;
        tx.execute(
            "INSERT OR IGNORE INTO playlists (id, name, description, coverImgUrl, source, meta, createTime, updateTime)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
            params![
                playlist.id,
                playlist.name,
                playlist.description,
                playlist.cover_img_url,
                playlist.source,
                playlist.meta,
                playlist.create_time,
                playlist.update_time
            ],
        )?;
        let mut next_position = if mode == "merge" {
            tx.query_row(
                "SELECT COALESCE(MAX(position), -1) + 1 FROM playlist_songs WHERE playlist_id = ?1",
                [&playlist.id],
                |row| row.get::<_, i64>(0),
            )?
        } else {
            0
        };
        for song in &item.songs {
            let position = if mode == "merge" {
                next_position
            } else {
                song.position
            };
            let inserted = tx.execute(
                "INSERT OR IGNORE INTO playlist_songs (playlist_id, songmid, position, data, name, singer, albumName, img)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
                params![
                    playlist.id,
                    song.songmid,
                    position,
                    song.data,
                    song.name,
                    song.singer,
                    song.album_name,
                    song.img
                ],
            )?;
            if mode == "merge" && inserted > 0 {
                next_position += 1;
            }
        }
    }
    tx.commit()
}

pub fn count_songs(conn: &Connection, playlist_id: &str) -> Result<i64> {
    conn.query_row(
        "SELECT COUNT(*) FROM playlist_songs WHERE playlist_id = ?1",
        [playlist_id],
        |row| row.get(0),
    )
}

pub fn has_song(conn: &Connection, playlist_id: &str, songmid: &str) -> Result<bool> {
    let mut stmt = conn.prepare(
        "SELECT 1 FROM playlist_songs WHERE playlist_id = ?1 AND songmid = ?2"
    )?;
    let mut rows = stmt.query_map(params![playlist_id, songmid], |_| Ok(true))?;
    match rows.next() {
        Some(Ok(true)) => Ok(true),
        _ => Ok(false),
    }
}

pub fn add_songs(conn: &Connection, playlist_id: &str, songs: &[PlaylistSongRow]) -> Result<()> {
    let max_pos: i64 = conn.query_row(
        "SELECT COALESCE(MAX(position), -1) FROM playlist_songs WHERE playlist_id = ?1",
        [playlist_id],
        |row| row.get(0),
    )?;
    let tx = conn.unchecked_transaction()?;
    for (i, s) in songs.iter().enumerate() {
        let pos = max_pos + 1 + i as i64;
        tx.execute(
            "INSERT OR IGNORE INTO playlist_songs (playlist_id, songmid, position, data, name, singer, albumName, img)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
            params![playlist_id, s.songmid, pos, s.data, s.name, s.singer, s.album_name, s.img],
        )?;
    }
    tx.commit()?;
    Ok(())
}

pub fn add_songs_head(conn: &Connection, playlist_id: &str, songs: &[PlaylistSongRow]) -> Result<()> {
    // Shift existing songs up
    let _count = count_songs(conn, playlist_id)?;
    conn.execute(
        "UPDATE playlist_songs SET position = position + ?1 WHERE playlist_id = ?2",
        params![songs.len() as i64, playlist_id],
    )?;
    let tx = conn.unchecked_transaction()?;
    for (i, s) in songs.iter().enumerate() {
        tx.execute(
            "INSERT OR IGNORE INTO playlist_songs (playlist_id, songmid, position, data, name, singer, albumName, img)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
            params![playlist_id, s.songmid, i as i64, s.data, s.name, s.singer, s.album_name, s.img],
        )?;
    }
    tx.commit()?;
    Ok(())
}

pub fn remove_song(conn: &Connection, playlist_id: &str, songmid: &str) -> Result<usize> {
    conn.execute(
        "DELETE FROM playlist_songs WHERE playlist_id = ?1 AND songmid = ?2",
        params![playlist_id, songmid],
    )
}

pub fn remove_songs(conn: &Connection, playlist_id: &str, songmids: &[String]) -> Result<usize> {
    if songmids.is_empty() { return Ok(0); }
    let placeholders: Vec<String> = songmids.iter().enumerate().map(|(i, _)| format!("?{}", i + 2)).collect();
    let sql = format!(
        "DELETE FROM playlist_songs WHERE playlist_id = ?1 AND songmid IN ({})",
        placeholders.join(",")
    );
    let mut params_vec: Vec<Box<dyn rusqlite::types::ToSql>> = vec![Box::new(playlist_id.to_string())];
    for id in songmids {
        params_vec.push(Box::new(id.clone()));
    }
    let params: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();
    conn.execute(&sql, params.as_slice())
}

pub fn clear_songs(conn: &Connection, playlist_id: &str) -> Result<usize> {
    conn.execute("DELETE FROM playlist_songs WHERE playlist_id = ?1", [playlist_id])
}

pub fn search_playlists(conn: &Connection, keyword: &str, source: Option<&str>) -> Result<Vec<PlaylistRow>> {
    let pattern = format!("%{}%", keyword);
    let mut stmt = if source.is_some() {
        conn.prepare(
            "SELECT p.id, p.name, p.description, p.coverImgUrl, p.source, p.meta, p.createTime, p.updateTime, \
             (SELECT COUNT(*) FROM playlist_songs WHERE playlist_id = p.id) FROM playlists p WHERE p.name LIKE ?1 AND p.source = ?2 ORDER BY p.createTime"
        )?
    } else {
        conn.prepare(
            "SELECT p.id, p.name, p.description, p.coverImgUrl, p.source, p.meta, p.createTime, p.updateTime, \
             (SELECT COUNT(*) FROM playlist_songs WHERE playlist_id = p.id) FROM playlists p WHERE p.name LIKE ?1 ORDER BY p.createTime"
        )?
    };
    let map_row = |row: &rusqlite::Row| -> Result<PlaylistRow> {
        Ok(PlaylistRow {
            id: row.get(0)?,
            name: row.get(1)?,
            description: row.get(2)?,
            cover_img_url: row.get(3)?,
            source: row.get(4)?,
            meta: row.get(5)?,
            create_time: row.get(6)?,
            update_time: row.get(7)?,
            song_count: row.get(8)?,
        })
    };
    let rows: Vec<PlaylistRow> = if let Some(s) = source {
        stmt.query_map(params![pattern, s], map_row)?.filter_map(|r| r.ok()).collect()
    } else {
        stmt.query_map(params![pattern], map_row)?.filter_map(|r| r.ok()).collect()
    };
    Ok(rows)
}

pub fn search_songs(conn: &Connection, playlist_id: &str, keyword: &str) -> Result<Vec<PlaylistSongRow>> {
    let pattern = format!("%{}%", keyword);
    let mut stmt = conn.prepare(
        "SELECT playlist_id, songmid, position, data, name, singer, albumName, img
         FROM playlist_songs WHERE playlist_id = ?1 AND (name LIKE ?2 OR singer LIKE ?2)
         ORDER BY position"
    )?;
    let rows = stmt.query_map(params![playlist_id, pattern], |row| {
        Ok(PlaylistSongRow {
            playlist_id: row.get(0)?,
            songmid: row.get(1)?,
            position: row.get(2)?,
            data: row.get(3)?,
            name: row.get(4)?,
            singer: row.get(5)?,
            album_name: row.get(6)?,
            img: row.get(7)?,
        })
    })?;
    rows.collect()
}

// --- Batch delete & reorder ---

pub fn batch_delete_playlists(conn: &Connection, ids: &[String]) -> Result<usize> {
    if ids.is_empty() { return Ok(0); }
    let placeholders: Vec<String> = ids.iter().enumerate().map(|(i, _)| format!("?{}", i + 1)).collect();
    let sql = format!("DELETE FROM playlists WHERE id IN ({})", placeholders.join(","));
    let params_vec: Vec<Box<dyn rusqlite::types::ToSql>> = ids.iter().map(|id| Box::new(id.clone()) as Box<dyn rusqlite::types::ToSql>).collect();
    let params: Vec<&dyn rusqlite::types::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();
    conn.execute(&sql, params.as_slice())
}

pub fn move_song(conn: &Connection, playlist_id: &str, songmid: &str, to_position: i64) -> Result<()> {
    let current_pos: i64 = conn.query_row(
        "SELECT position FROM playlist_songs WHERE playlist_id = ?1 AND songmid = ?2",
        params![playlist_id, songmid],
        |row| row.get(0),
    )?;
    if current_pos == to_position { return Ok(()); }
    let tx = conn.unchecked_transaction()?;
    if to_position > current_pos {
        tx.execute(
            "UPDATE playlist_songs SET position = position - 1 WHERE playlist_id = ?1 AND position > ?2 AND position <= ?3",
            params![playlist_id, current_pos, to_position],
        )?;
    } else {
        tx.execute(
            "UPDATE playlist_songs SET position = position + 1 WHERE playlist_id = ?1 AND position >= ?2 AND position < ?3",
            params![playlist_id, to_position, current_pos],
        )?;
    }
    tx.execute(
        "UPDATE playlist_songs SET position = ?1 WHERE playlist_id = ?2 AND songmid = ?3",
        params![to_position, playlist_id, songmid],
    )?;
    tx.commit()?;
    Ok(())
}

// --- Favorites ID persistence (stored in a key-value table) ---

pub fn ensure_kv_table(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS kv_store (key TEXT PRIMARY KEY, value TEXT NOT NULL);"
    )
}

pub fn kv_get(conn: &Connection, key: &str) -> Result<Option<String>> {
    conn.query_row(
        "SELECT value FROM kv_store WHERE key = ?1",
        [key],
        |row| row.get(0),
    ).ok().map_or(Ok(None), |v| Ok(Some(v)))
}

pub fn kv_set(conn: &Connection, key: &str, value: &str) -> Result<()> {
    conn.execute(
        "INSERT INTO kv_store (key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value=excluded.value",
        params![key, value],
    )?;
    Ok(())
}

#[cfg(test)]
mod backup_tests {
    use super::*;

    fn playlist(id: &str, name: &str) -> PlaylistRow {
        PlaylistRow {
            id: id.into(),
            name: name.into(),
            description: String::new(),
            cover_img_url: "default-cover".into(),
            source: "local".into(),
            meta: "{}".into(),
            create_time: "2026-01-01T00:00:00Z".into(),
            update_time: "2026-01-01T00:00:00Z".into(),
            song_count: 0,
        }
    }

    fn song(playlist_id: &str, songmid: &str) -> PlaylistSongRow {
        PlaylistSongRow {
            playlist_id: playlist_id.into(),
            position: 0,
            data: "{}".into(),
            songmid: songmid.into(),
            name: songmid.into(),
            singer: String::new(),
            album_name: String::new(),
            img: String::new(),
        }
    }

    #[test]
    fn export_and_overwrite_restore_playlists() {
        let source = Connection::open_in_memory().unwrap();
        init_tables(&source).unwrap();
        insert_playlist(&source, &playlist("source", "Source")).unwrap();
        add_songs(&source, "source", &[song("source", "song-1")]).unwrap();
        let backup = export_playlists(&source).unwrap();

        let mut target = Connection::open_in_memory().unwrap();
        init_tables(&target).unwrap();
        insert_playlist(&target, &playlist("old", "Old")).unwrap();
        restore_playlists(&mut target, &backup, "overwrite").unwrap();

        assert!(get_playlist(&target, "old").unwrap().is_none());
        assert_eq!(1, list_songs(&target, "source").unwrap().len());
    }

    #[test]
    fn merge_restore_keeps_existing_and_adds_missing_songs() {
        let mut target = Connection::open_in_memory().unwrap();
        init_tables(&target).unwrap();
        insert_playlist(&target, &playlist("shared", "Local Name")).unwrap();
        add_songs(&target, "shared", &[song("shared", "local")]).unwrap();

        let backup = PlaylistBackup {
            playlists: vec![PlaylistBackupItem {
                playlist: playlist("shared", "Cloud Name"),
                songs: vec![song("shared", "cloud")],
            }],
        };
        restore_playlists(&mut target, &backup, "merge").unwrap();

        assert_eq!("Local Name", get_playlist(&target, "shared").unwrap().unwrap().name);
        let songs = list_songs(&target, "shared").unwrap();
        assert_eq!(vec!["local", "cloud"], songs.iter().map(|song| song.songmid.as_str()).collect::<Vec<_>>());
        assert_eq!(vec![0, 1], songs.iter().map(|song| song.position).collect::<Vec<_>>());
    }
}
