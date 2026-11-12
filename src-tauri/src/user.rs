// User data: playlists, ratings, Loved, play/skip counts and play history.
use rusqlite::{params, Connection, Result};
use serde::Serialize;

#[derive(Serialize, Debug, PartialEq)]
pub struct Playlist {
    pub id: i64,
    pub name: String,
    pub rules: Option<String>, // JSON smart rules; None = a manual playlist
    pub items: Vec<i64>,
}

pub fn playlists(c: &Connection) -> Result<Vec<Playlist>> {
    let mut items = c.prepare("SELECT track_id FROM playlist_items WHERE playlist_id = ?1 ORDER BY pos")?;
    let mut s = c.prepare("SELECT id, name, rules FROM playlists ORDER BY created_at, id")?;
    let rows = s.query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get(1)?, r.get(2)?)))?;
    rows.map(|row| {
        let (id, name, rules) = row?;
        let items = items.query_map([id], |r| r.get(0))?.collect::<Result<_>>()?;
        Ok(Playlist { id, name, rules, items })
    })
    .collect()
}

pub fn create(c: &Connection, name: &str, rules: Option<&str>, now: i64) -> Result<i64> {
    c.execute("INSERT INTO playlists(name, rules, created_at) VALUES (?1, ?2, ?3)", params![name, rules, now])?;
    Ok(c.last_insert_rowid())
}

pub fn update(c: &Connection, id: i64, name: &str, rules: Option<&str>) -> Result<()> {
    c.execute("UPDATE playlists SET name = ?2, rules = ?3 WHERE id = ?1", params![id, name, rules])?;
    Ok(())
}

pub fn delete(c: &Connection, id: i64) -> Result<()> {
    c.execute("DELETE FROM playlists WHERE id = ?1", [id])?;
    Ok(())
}

/// Replaces a manual playlist's order in one transaction (add, remove and reorder all use this).
pub fn set_items(c: &mut Connection, id: i64, items: &[i64]) -> Result<()> {
    let tx = c.transaction()?;
    tx.execute("DELETE FROM playlist_items WHERE playlist_id = ?1", [id])?;
    {
        let mut ins = tx.prepare("INSERT INTO playlist_items(playlist_id, pos, track_id) VALUES (?1, ?2, ?3)")?;
        for (pos, t) in items.iter().enumerate() {
            ins.execute(params![id, pos as i64, t])?;
        }
    }
    tx.commit()
}

pub fn rate(c: &Connection, id: i64, rating: u8) -> Result<()> {
    c.execute("UPDATE tracks SET rating = ?2 WHERE id = ?1", params![id, rating.min(5)])?;
    Ok(())
}

pub fn love(c: &Connection, id: i64, loved: bool) -> Result<()> {
    c.execute("UPDATE tracks SET loved = ?2 WHERE id = ?1", params![id, loved])?;
    Ok(())
}

pub fn played(c: &Connection, id: i64, at: i64) -> Result<()> {
    c.execute("UPDATE tracks SET play_count = play_count + 1, last_played = ?2 WHERE id = ?1", params![id, at])?;
    c.execute("INSERT INTO plays(track_id, played_at) VALUES (?1, ?2)", params![id, at])?;
    Ok(())
}

pub fn skipped(c: &Connection, id: i64) -> Result<()> {
    c.execute("UPDATE tracks SET skip_count = skip_count + 1 WHERE id = ?1", [id])?;
    Ok(())
}

/// Most-played tracks since `since` (unix seconds), for "Your Top".
pub fn top(c: &Connection, since: i64) -> Result<Vec<(i64, i64)>> {
    let mut s = c.prepare(
        "SELECT track_id, COUNT(*) AS n FROM plays WHERE played_at >= ?1 GROUP BY track_id ORDER BY n DESC LIMIT 500",
    )?;
    s.query_map([since], |r| Ok((r.get(0)?, r.get(1)?)))?.collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

    fn setup() -> (tempfile::TempDir, Connection) {
        let d = tempfile::tempdir().unwrap();
        let c = db::open(&d.path().join("t.db")).unwrap();
        for (i, p) in ["a", "b", "c"].iter().enumerate() {
            db::upsert(&c, &db::Track { path: p.to_string(), title: p.to_string(), kind: "audio".into(), added_at: i as i64, ..Default::default() }).unwrap();
        }
        (d, c)
    }

    #[test]
    fn playlists_keep_order_and_drop_deleted_tracks() {
        let (_d, mut c) = setup();
        let id = create(&c, "Mix", None, 1).unwrap();
        set_items(&mut c, id, &[3, 1, 2, 1]).unwrap();
        assert_eq!(playlists(&c).unwrap()[0].items, vec![3, 1, 2, 1]);
        c.execute("DELETE FROM tracks WHERE id = 1", []).unwrap();
        assert_eq!(playlists(&c).unwrap()[0].items, vec![3, 2]);
        update(&c, id, "Renamed", Some(r#"{"match":"all","rules":[]}"#)).unwrap();
        let p = &playlists(&c).unwrap()[0];
        assert_eq!((p.name.as_str(), p.rules.is_some()), ("Renamed", true));
        delete(&c, id).unwrap();
        assert!(playlists(&c).unwrap().is_empty());
    }

    #[test]
    fn kv_store_upserts() {
        let (_d, c) = setup();
        assert_eq!(db::kv_get(&c, "settings").unwrap(), None);
        db::kv_set(&c, "settings", "{\"a\":1}").unwrap();
        db::kv_set(&c, "settings", "{\"a\":2}").unwrap();
        assert_eq!(db::kv_get(&c, "settings").unwrap().as_deref(), Some("{\"a\":2}"));
    }

    #[test]
    fn counts_plays_skips_and_tops() {
        let (_d, c) = setup();
        played(&c, 2, 100).unwrap();
        played(&c, 2, 200).unwrap();
        played(&c, 3, 300).unwrap();
        skipped(&c, 1).unwrap();
        rate(&c, 3, 9).unwrap();
        love(&c, 3, true).unwrap();
        let t = db::tracks(&c).unwrap();
        let by = |id: i64| t.iter().find(|x| x.id == id).unwrap();
        assert_eq!((by(2).plays, by(2).last_played, by(1).skips), (2, Some(200), 1));
        assert_eq!((by(3).rating, by(3).loved), (5, true)); // rating is capped at 5 stars
        assert_eq!(top(&c, 0).unwrap(), vec![(2, 2), (3, 1)]);
        assert_eq!(top(&c, 250).unwrap(), vec![(3, 1)]);
    }
}
