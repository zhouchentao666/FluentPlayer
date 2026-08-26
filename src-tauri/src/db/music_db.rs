use rusqlite::{params, params_from_iter, Connection, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackRow {
    pub songmid: String,
    pub path: String,
    pub url: Option<String>,
    pub singer: String,
    pub name: String,
    pub album_name: String,
    pub album_id: i64,
    pub source: String,
    pub interval: String,
    pub has_cover: i64,
    pub cover_key: Option<String>,
    pub year: i64,
    pub lrc: Option<String>,
    pub types: String,
    #[serde(rename = "_types")]
    pub _types: String,
    pub type_url: String,
    pub bitrate: i64,
    pub sample_rate: i64,
    pub channels: i64,
    pub duration: f64,
    pub size: i64,
    pub mtime_ms: i64,
    pub hash: Option<String>,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackStat {
    pub path: String,
    pub mtime_ms: i64,
    pub hash: Option<String>,
}

pub fn init_tables(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS tracks (
            songmid      TEXT PRIMARY KEY,
            path         TEXT NOT NULL UNIQUE,
            url          TEXT,
            singer       TEXT DEFAULT '',
            name         TEXT DEFAULT '',
            albumName    TEXT DEFAULT '',
            albumId      INTEGER DEFAULT 0,
            source       TEXT DEFAULT 'local',
            interval     TEXT DEFAULT '',
            hasCover     INTEGER DEFAULT 0,
            coverKey     TEXT,
            year         INTEGER DEFAULT 0,
            lrc          TEXT,
            types        TEXT DEFAULT '[]',
            _types       TEXT DEFAULT '{}',
            typeUrl      TEXT DEFAULT '{}',
            bitrate      INTEGER DEFAULT 0,
            sampleRate   INTEGER DEFAULT 0,
            channels     INTEGER DEFAULT 0,
            duration     REAL DEFAULT 0,
            size         INTEGER DEFAULT 0,
            mtime_ms     INTEGER DEFAULT 0,
            hash         TEXT,
            updated_at   INTEGER NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_tracks_path ON tracks(path);
        CREATE INDEX IF NOT EXISTS idx_tracks_name ON tracks(name COLLATE NOCASE);
        CREATE INDEX IF NOT EXISTS idx_tracks_source ON tracks(source);
        CREATE INDEX IF NOT EXISTS idx_tracks_album ON tracks(albumName COLLATE NOCASE);

        CREATE TABLE IF NOT EXISTS dirs (
            path TEXT PRIMARY KEY
        );",
    )?;
    Ok(())
}

// --- Track CRUD ---

pub fn get_all_tracks(conn: &Connection) -> Result<Vec<TrackRow>> {
    let mut stmt = conn.prepare(
        "SELECT songmid, path, url, singer, name, albumName, albumId, source, interval,
                hasCover, coverKey, year, lrc, types, _types, typeUrl, bitrate, sampleRate,
                channels, duration, size, mtime_ms, hash, updated_at
         FROM tracks ORDER BY name COLLATE NOCASE",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(TrackRow {
            songmid: row.get(0)?,
            path: row.get(1)?,
            url: row.get(2)?,
            singer: row.get(3)?,
            name: row.get(4)?,
            album_name: row.get(5)?,
            album_id: row.get(6)?,
            source: row.get(7)?,
            interval: row.get(8)?,
            has_cover: row.get(9)?,
            cover_key: row.get(10)?,
            year: row.get(11)?,
            lrc: row.get(12)?,
            types: row.get(13)?,
            _types: row.get(14)?,
            type_url: row.get(15)?,
            bitrate: row.get(16)?,
            sample_rate: row.get(17)?,
            channels: row.get(18)?,
            duration: row.get(19)?,
            size: row.get(20)?,
            mtime_ms: row.get(21)?,
            hash: row.get(22)?,
            updated_at: row.get(23)?,
        })
    })?;
    rows.collect()
}

pub fn get_track_paths_by_ids(
    conn: &Connection,
    songmids: &[String],
) -> Result<Vec<(String, String)>> {
    let mut stmt = conn.prepare("SELECT songmid, path FROM tracks WHERE songmid = ?1")?;
    let mut result = Vec::with_capacity(songmids.len());
    for songmid in songmids {
        let mut rows = stmt.query_map([songmid], |row| Ok((row.get(0)?, row.get(1)?)))?;
        if let Some(row) = rows.next() {
            result.push(row?);
        }
    }
    Ok(result)
}

pub fn get_track_by_id(conn: &Connection, songmid: &str) -> Result<Option<TrackRow>> {
    let mut stmt = conn.prepare(
        "SELECT songmid, path, url, singer, name, albumName, albumId, source, interval,
                hasCover, coverKey, year, lrc, types, _types, typeUrl, bitrate, sampleRate,
                channels, duration, size, mtime_ms, hash, updated_at
         FROM tracks WHERE songmid = ?1",
    )?;
    let mut rows = stmt.query_map([songmid], |row| {
        Ok(TrackRow {
            songmid: row.get(0)?,
            path: row.get(1)?,
            url: row.get(2)?,
            singer: row.get(3)?,
            name: row.get(4)?,
            album_name: row.get(5)?,
            album_id: row.get(6)?,
            source: row.get(7)?,
            interval: row.get(8)?,
            has_cover: row.get(9)?,
            cover_key: row.get(10)?,
            year: row.get(11)?,
            lrc: row.get(12)?,
            types: row.get(13)?,
            _types: row.get(14)?,
            type_url: row.get(15)?,
            bitrate: row.get(16)?,
            sample_rate: row.get(17)?,
            channels: row.get(18)?,
            duration: row.get(19)?,
            size: row.get(20)?,
            mtime_ms: row.get(21)?,
            hash: row.get(22)?,
            updated_at: row.get(23)?,
        })
    })?;
    match rows.next() {
        Some(row) => Ok(Some(row?)),
        None => Ok(None),
    }
}

pub fn get_track_by_path(conn: &Connection, path: &str) -> Result<Option<TrackRow>> {
    let mut stmt = conn.prepare(
        "SELECT songmid, path, url, singer, name, albumName, albumId, source, interval,
                hasCover, coverKey, year, lrc, types, _types, typeUrl, bitrate, sampleRate,
                channels, duration, size, mtime_ms, hash, updated_at
         FROM tracks WHERE path = ?1",
    )?;
    let mut rows = stmt.query_map([path], |row| {
        Ok(TrackRow {
            songmid: row.get(0)?,
            path: row.get(1)?,
            url: row.get(2)?,
            singer: row.get(3)?,
            name: row.get(4)?,
            album_name: row.get(5)?,
            album_id: row.get(6)?,
            source: row.get(7)?,
            interval: row.get(8)?,
            has_cover: row.get(9)?,
            cover_key: row.get(10)?,
            year: row.get(11)?,
            lrc: row.get(12)?,
            types: row.get(13)?,
            _types: row.get(14)?,
            type_url: row.get(15)?,
            bitrate: row.get(16)?,
            sample_rate: row.get(17)?,
            channels: row.get(18)?,
            duration: row.get(19)?,
            size: row.get(20)?,
            mtime_ms: row.get(21)?,
            hash: row.get(22)?,
            updated_at: row.get(23)?,
        })
    })?;
    match rows.next() {
        Some(row) => Ok(Some(row?)),
        None => Ok(None),
    }
}

pub fn upsert_track(conn: &Connection, t: &TrackRow) -> Result<()> {
    conn.execute(
        "INSERT INTO tracks (songmid, path, url, singer, name, albumName, albumId, source, interval,
                            hasCover, coverKey, year, lrc, types, _types, typeUrl, bitrate, sampleRate,
                            channels, duration, size, mtime_ms, hash, updated_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21,?22,?23,?24)
         ON CONFLICT(songmid) DO UPDATE SET
            path=excluded.path, url=excluded.url, singer=excluded.singer, name=excluded.name,
            albumName=excluded.albumName, albumId=excluded.albumId, source=excluded.source,
            interval=excluded.interval, hasCover=excluded.hasCover, coverKey=excluded.coverKey,
            year=excluded.year, lrc=excluded.lrc, types=excluded.types, _types=excluded._types,
            typeUrl=excluded.typeUrl, bitrate=excluded.bitrate, sampleRate=excluded.sampleRate,
            channels=excluded.channels, duration=excluded.duration, size=excluded.size,
            mtime_ms=excluded.mtime_ms, hash=excluded.hash, updated_at=excluded.updated_at",
        params![
            t.songmid, t.path, t.url, t.singer, t.name, t.album_name, t.album_id,
            t.source, t.interval, t.has_cover, t.cover_key, t.year, t.lrc, t.types,
            t._types, t.type_url, t.bitrate, t.sample_rate, t.channels, t.duration,
            t.size, t.mtime_ms, t.hash, t.updated_at
        ],
    )?;
    Ok(())
}

pub fn upsert_tracks(conn: &Connection, tracks: &[TrackRow]) -> Result<()> {
    let tx = conn.unchecked_transaction()?;
    for t in tracks {
        tx.execute(
            "INSERT INTO tracks (songmid, path, url, singer, name, albumName, albumId, source, interval,
                                hasCover, coverKey, year, lrc, types, _types, typeUrl, bitrate, sampleRate,
                                channels, duration, size, mtime_ms, hash, updated_at)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21,?22,?23,?24)
             ON CONFLICT(songmid) DO UPDATE SET
                path=excluded.path, url=excluded.url, singer=excluded.singer, name=excluded.name,
                albumName=excluded.albumName, albumId=excluded.albumId, source=excluded.source,
                interval=excluded.interval, hasCover=excluded.hasCover, coverKey=excluded.coverKey,
                year=excluded.year, lrc=excluded.lrc, types=excluded.types, _types=excluded._types,
                typeUrl=excluded.typeUrl, bitrate=excluded.bitrate, sampleRate=excluded.sampleRate,
                channels=excluded.channels, duration=excluded.duration, size=excluded.size,
                mtime_ms=excluded.mtime_ms, hash=excluded.hash, updated_at=excluded.updated_at",
            params![
                t.songmid, t.path, t.url, t.singer, t.name, t.album_name, t.album_id,
                t.source, t.interval, t.has_cover, t.cover_key, t.year, t.lrc, t.types,
                t._types, t.type_url, t.bitrate, t.sample_rate, t.channels, t.duration,
                t.size, t.mtime_ms, t.hash, t.updated_at
            ],
        )?;
    }
    tx.commit()?;
    Ok(())
}

pub fn delete_by_path(conn: &Connection, path: &str) -> Result<usize> {
    conn.execute("DELETE FROM tracks WHERE path = ?1", [path])
}

pub fn delete_by_songmid(conn: &Connection, songmid: &str) -> Result<usize> {
    conn.execute("DELETE FROM tracks WHERE songmid = ?1", [songmid])
}

pub fn clear_tracks(conn: &Connection) -> Result<usize> {
    conn.execute("DELETE FROM tracks", [])
}

pub fn get_stat_by_path(conn: &Connection, path: &str) -> Result<Option<TrackStat>> {
    let mut stmt = conn.prepare("SELECT path, mtime_ms, hash FROM tracks WHERE path = ?1")?;
    let mut rows = stmt.query_map([path], |row| {
        Ok(TrackStat {
            path: row.get(0)?,
            mtime_ms: row.get(1)?,
            hash: row.get(2)?,
        })
    })?;
    match rows.next() {
        Some(row) => Ok(Some(row?)),
        None => Ok(None),
    }
}

pub fn get_all_stats(conn: &Connection) -> Result<Vec<TrackStat>> {
    let mut stmt = conn.prepare("SELECT path, mtime_ms, hash FROM tracks")?;
    let rows = stmt.query_map([], |row| {
        Ok(TrackStat {
            path: row.get(0)?,
            mtime_ms: row.get(1)?,
            hash: row.get(2)?,
        })
    })?;
    rows.collect()
}

pub fn prune_outside_keep(conn: &Connection, keep_paths: &[String]) -> Result<usize> {
    if keep_paths.is_empty() {
        return Ok(0);
    }

    let tx = conn.unchecked_transaction()?;
    tx.execute(
        "CREATE TEMP TABLE IF NOT EXISTS scan_keep_paths (path TEXT PRIMARY KEY)",
        [],
    )?;
    tx.execute("DELETE FROM scan_keep_paths", [])?;

    {
        let mut stmt = tx.prepare("INSERT OR IGNORE INTO scan_keep_paths (path) VALUES (?1)")?;
        for chunk in keep_paths.chunks(500) {
            for path in chunk {
                stmt.execute(params_from_iter([path]))?;
            }
        }
    }

    let deleted = tx.execute(
        "DELETE FROM tracks WHERE NOT EXISTS (SELECT 1 FROM scan_keep_paths WHERE scan_keep_paths.path = tracks.path)",
        [],
    )?;
    tx.execute("DELETE FROM scan_keep_paths", [])?;
    tx.commit()?;
    Ok(deleted)
}

pub fn prune_scanned_roots(
    conn: &Connection,
    roots: &[String],
    keep_paths: &[String],
) -> Result<usize> {
    if roots.is_empty() {
        return Ok(0);
    }

    let tx = conn.unchecked_transaction()?;
    tx.execute_batch(
        "CREATE TEMP TABLE IF NOT EXISTS scan_keep_paths (path TEXT PRIMARY KEY);
         CREATE TEMP TABLE IF NOT EXISTS scan_roots (path TEXT PRIMARY KEY);",
    )?;
    tx.execute("DELETE FROM scan_keep_paths", [])?;
    tx.execute("DELETE FROM scan_roots", [])?;

    {
        let mut stmt = tx.prepare("INSERT OR IGNORE INTO scan_roots (path) VALUES (?1)")?;
        for root in roots {
            stmt.execute([root])?;
        }
    }
    {
        let mut stmt = tx.prepare("INSERT OR IGNORE INTO scan_keep_paths (path) VALUES (?1)")?;
        for path in keep_paths {
            stmt.execute([path])?;
        }
    }

    let deleted = tx.execute(
        "DELETE FROM tracks
         WHERE EXISTS (
             SELECT 1 FROM scan_roots
             WHERE substr(tracks.path, 1, length(scan_roots.path)) = scan_roots.path
               AND (
                   substr(scan_roots.path, -1, 1) IN ('/', '\\')
                   OR substr(tracks.path, length(scan_roots.path) + 1, 1) IN ('/', '\\')
               )
         )
           AND NOT EXISTS (
             SELECT 1 FROM scan_keep_paths WHERE scan_keep_paths.path = tracks.path
         )",
        [],
    )?;
    tx.execute("DELETE FROM scan_keep_paths", [])?;
    tx.execute("DELETE FROM scan_roots", [])?;
    tx.commit()?;
    Ok(deleted)
}

// --- Directory management ---

pub fn get_dirs(conn: &Connection) -> Result<Vec<String>> {
    let mut stmt = conn.prepare("SELECT path FROM dirs")?;
    let rows = stmt.query_map([], |row| row.get(0))?;
    rows.collect()
}

pub fn set_dirs(conn: &Connection, dirs: &[String]) -> Result<()> {
    conn.execute("DELETE FROM dirs", [])?;
    for d in dirs {
        conn.execute("INSERT INTO dirs (path) VALUES (?1)", [d])?;
    }
    Ok(())
}
