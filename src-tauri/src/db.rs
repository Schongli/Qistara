use rusqlite::{params, Connection, Row};
use serde::Serialize;
use std::path::Path;

// Append only: each entry runs once, tracked by PRAGMA user_version.
const MIGRATIONS: &[&str] = &[r#"
CREATE TABLE folders(path TEXT PRIMARY KEY);
CREATE TABLE tracks(
  id INTEGER PRIMARY KEY,
  path TEXT NOT NULL UNIQUE,
  mtime INTEGER NOT NULL,
  size INTEGER NOT NULL,
  kind TEXT NOT NULL,
  title TEXT NOT NULL,
  artist TEXT NOT NULL DEFAULT '',
  album_artist TEXT NOT NULL DEFAULT '',
  album TEXT NOT NULL DEFAULT '',
  genre TEXT NOT NULL DEFAULT '',
  composer TEXT NOT NULL DEFAULT '',
  year INTEGER,
  track_no INTEGER,
  disc_no INTEGER,
  duration REAL NOT NULL DEFAULT 0,
  codec TEXT NOT NULL DEFAULT '',
  sample_rate INTEGER,
  bit_depth INTEGER,
  channels INTEGER,
  bitrate INTEGER,
  lossless INTEGER NOT NULL DEFAULT 0,
  art TEXT,
  rating INTEGER NOT NULL DEFAULT 0,
  loved INTEGER NOT NULL DEFAULT 0,
  play_count INTEGER NOT NULL DEFAULT 0,
  skip_count INTEGER NOT NULL DEFAULT 0,
  last_played INTEGER,
  added_at INTEGER NOT NULL
);
CREATE TABLE plays(track_id INTEGER NOT NULL REFERENCES tracks(id) ON DELETE CASCADE, played_at INTEGER NOT NULL);
CREATE INDEX plays_time ON plays(played_at);
CREATE TABLE playlists(id INTEGER PRIMARY KEY, name TEXT NOT NULL, rules TEXT, created_at INTEGER NOT NULL);
CREATE TABLE playlist_items(
  playlist_id INTEGER NOT NULL REFERENCES playlists(id) ON DELETE CASCADE,
  pos INTEGER NOT NULL,
  track_id INTEGER NOT NULL REFERENCES tracks(id) ON DELETE CASCADE
);
"#, r#"
CREATE TABLE kv(key TEXT PRIMARY KEY, value TEXT NOT NULL);
"#];

pub fn open(path: &Path) -> rusqlite::Result<Connection> {
    let mut c = Connection::open(path)?;
    c.pragma_update(None, "journal_mode", "WAL")?;
    c.pragma_update(None, "foreign_keys", true)?;
    let done: i64 = c.pragma_query_value(None, "user_version", |r| r.get(0))?;
    for (i, sql) in MIGRATIONS.iter().enumerate().skip(done as usize) {
        let tx = c.transaction()?;
        tx.execute_batch(sql)?;
        tx.pragma_update(None, "user_version", i as i64 + 1)?;
        tx.commit()?;
    }
    Ok(c)
}

#[derive(Serialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Track {
    pub id: i64,
    pub path: String,
    #[serde(skip)]
    pub mtime: i64,
    #[serde(skip)]
    pub size: i64,
    pub kind: String,
    pub title: String,
    pub artist: String,
    pub album_artist: String,
    pub album: String,
    pub genre: String,
    pub composer: String,
    pub year: Option<i32>,
    pub track: Option<u32>,
    pub disc: Option<u32>,
    pub duration: f64,
    pub codec: String,
    pub sample_rate: Option<u32>,
    pub bit_depth: Option<u8>,
    pub channels: Option<u8>,
    pub bitrate: Option<u32>,
    pub lossless: bool,
    pub art: Option<String>,
    pub rating: u8,
    pub loved: bool,
    pub plays: u32,
    pub skips: u32,
    pub last_played: Option<i64>,
    pub added_at: i64,
}

const COLS: &str = "id, path, mtime, size, kind, title, artist, album_artist, album, genre, composer, year, track_no, \
  disc_no, duration, codec, sample_rate, bit_depth, channels, bitrate, lossless, art, rating, loved, play_count, \
  skip_count, last_played, added_at";

fn row(r: &Row) -> rusqlite::Result<Track> {
    Ok(Track {
        id: r.get(0)?, path: r.get(1)?, mtime: r.get(2)?, size: r.get(3)?, kind: r.get(4)?, title: r.get(5)?,
        artist: r.get(6)?, album_artist: r.get(7)?, album: r.get(8)?, genre: r.get(9)?, composer: r.get(10)?,
        year: r.get(11)?, track: r.get(12)?, disc: r.get(13)?, duration: r.get(14)?, codec: r.get(15)?,
        sample_rate: r.get(16)?, bit_depth: r.get(17)?, channels: r.get(18)?, bitrate: r.get(19)?,
        lossless: r.get(20)?, art: r.get(21)?, rating: r.get(22)?, loved: r.get(23)?, plays: r.get(24)?,
        skips: r.get(25)?, last_played: r.get(26)?, added_at: r.get(27)?,
    })
}

pub fn tracks(c: &Connection) -> rusqlite::Result<Vec<Track>> {
    let mut s = c.prepare(&format!("SELECT {COLS} FROM tracks"))?;
    s.query_map([], row)?.collect()
}

pub fn folders(c: &Connection) -> rusqlite::Result<Vec<String>> {
    let mut s = c.prepare("SELECT path FROM folders ORDER BY path")?;
    s.query_map([], |r| r.get(0))?.collect()
}

/// Insert or refresh file metadata; user data (rating, loved, counts, added_at) survives a rescan.
pub fn upsert(c: &Connection, t: &Track) -> rusqlite::Result<()> {
    c.prepare_cached(
        "INSERT INTO tracks(path, mtime, size, kind, title, artist, album_artist, album, genre, composer, year,
           track_no, disc_no, duration, codec, sample_rate, bit_depth, channels, bitrate, lossless, art, added_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21,?22)
         ON CONFLICT(path) DO UPDATE SET mtime=excluded.mtime, size=excluded.size, kind=excluded.kind,
           title=excluded.title, artist=excluded.artist, album_artist=excluded.album_artist, album=excluded.album,
           genre=excluded.genre, composer=excluded.composer, year=excluded.year, track_no=excluded.track_no,
           disc_no=excluded.disc_no, duration=excluded.duration, codec=excluded.codec,
           sample_rate=excluded.sample_rate, bit_depth=excluded.bit_depth, channels=excluded.channels,
           bitrate=excluded.bitrate, lossless=excluded.lossless, art=excluded.art",
    )?
    .execute(params![
        t.path, t.mtime, t.size, t.kind, t.title, t.artist, t.album_artist, t.album, t.genre, t.composer, t.year,
        t.track, t.disc, t.duration, t.codec, t.sample_rate, t.bit_depth, t.channels, t.bitrate, t.lossless,
        t.art, t.added_at
    ])?;
    Ok(())
}

/// Small app state (settings, EQ, queue) written straight to disk, so a crash can't lose it.
pub fn kv_get(c: &Connection, key: &str) -> rusqlite::Result<Option<String>> {
    match c.query_row("SELECT value FROM kv WHERE key = ?1", [key], |r| r.get(0)) {
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        v => v.map(Some),
    }
}

pub fn kv_set(c: &Connection, key: &str, value: &str) -> rusqlite::Result<()> {
    c.execute("INSERT INTO kv(key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value = excluded.value", [key, value])?;
    Ok(())
}
