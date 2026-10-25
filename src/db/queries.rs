use super::Library;
use crate::logging::now_ms;
use crate::proto::{FAVORITE_TAG, MediaInfo};
use rusqlite::{OptionalExtension, params};
use std::collections::HashMap;

impl Library {
    fn tag_data(&self, name: &str) -> Result<Option<i64>, rusqlite::Error> {
        self.conn
            .query_row("SELECT id FROM tags WHERE name = ?1", params![name], |r| {
                r.get(0)
            })
            .optional()
    }

    pub(super) fn ensure_tag(&self, name: &str) -> Result<i64, rusqlite::Error> {
        self.conn.execute(
            "INSERT OR IGNORE INTO tags(name) VALUES (?1)",
            params![name],
        )?;
        Ok(self.tag_data(name)?.unwrap_or(0))
    }

    fn tags_for(&self, media_id: i64) -> Result<Vec<String>, rusqlite::Error> {
        let mut stmt = self.conn.prepare(
            "SELECT t.name FROM media_tags mt JOIN tags t ON t.id = mt.tag_id
             WHERE mt.media_id = ?1 ORDER BY t.name",
        )?;
        let rows = stmt.query_map(params![media_id], |r| r.get::<_, String>(0))?;
        rows.collect()
    }

    fn set_tags(&self, media_id: i64, tags: &[String]) -> Result<(), rusqlite::Error> {
        self.conn.execute(
            "DELETE FROM media_tags WHERE media_id = ?1",
            params![media_id],
        )?;
        for t in tags {
            let tag_id = self.ensure_tag(t)?;
            self.conn.execute(
                "INSERT OR IGNORE INTO media_tags(media_id, tag_id) VALUES (?1, ?2)",
                params![media_id, tag_id],
            )?;
        }
        Ok(())
    }

    pub fn add_media(
        &mut self,
        m: super::NewMedia<'_>,
        tags: &[String],
    ) -> Result<i64, rusqlite::Error> {
        self.conn.execute(
            "INSERT INTO media(uri, kind, title, artist, duration, bitrate, source, added_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
             ON CONFLICT(uri) DO UPDATE SET
                 kind = excluded.kind,
                 title = excluded.title,
                 artist = excluded.artist,
                 duration = excluded.duration,
                 bitrate = excluded.bitrate,
                 source = excluded.source,
                 added_at = excluded.added_at",
            params![
                m.uri,
                m.kind,
                m.title,
                m.artist,
                m.duration,
                m.bitrate,
                m.source,
                now_ms()
            ],
        )?;
        let id =
            self.conn
                .query_row("SELECT id FROM media WHERE uri = ?1", params![m.uri], |r| {
                    r.get(0)
                })?;
        let _ = self.ensure_tag(FAVORITE_TAG)?;
        self.set_tags(id, tags)?;
        Ok(id)
    }

    pub fn update_media(
        &mut self,
        id: i64,
        title: Option<&str>,
        artist: Option<&str>,
        tags: &[String],
    ) -> Result<(), rusqlite::Error> {
        self.conn.execute(
            "UPDATE media SET title = ?2, artist = ?3 WHERE id = ?1",
            params![id, title, artist],
        )?;
        self.set_tags(id, tags)?;
        Ok(())
    }

    pub fn delete_media(&mut self, id: i64) -> Result<(), rusqlite::Error> {
        self.conn
            .execute("DELETE FROM media WHERE id = ?1", params![id])?;
        Ok(())
    }

    fn row_to_media(&self, row: &rusqlite::Row) -> Result<MediaInfo, rusqlite::Error> {
        let id: i64 = row.get(0)?;
        let tags = self.tags_for(id)?;
        Ok(MediaInfo {
            id,
            uri: row.get(1)?,
            kind: row.get(2)?,
            title: row.get(3)?,
            artist: row.get(4)?,
            duration: row.get(5)?,
            bitrate: row.get(6)?,
            source: row.get(7)?,
            favorite: tags.iter().any(|t| t == FAVORITE_TAG),
            tags,
        })
    }

    pub fn media(&mut self, id: i64) -> Result<Option<MediaInfo>, rusqlite::Error> {
        self.conn
            .query_row(
                "SELECT id, uri, kind, title, artist, duration, bitrate, source
                 FROM media WHERE id = ?1",
                params![id],
                |r| self.row_to_media(r),
            )
            .optional()
    }

    pub fn all_media(&mut self) -> Result<Vec<MediaInfo>, rusqlite::Error> {
        let mut stmt = self.conn.prepare(
            "SELECT id, uri, kind, title, artist, duration, bitrate, source
             FROM media ORDER BY added_at DESC",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok(MediaInfo {
                id: r.get(0)?,
                uri: r.get(1)?,
                kind: r.get(2)?,
                title: r.get(3)?,
                artist: r.get(4)?,
                duration: r.get(5)?,
                bitrate: r.get(6)?,
                source: r.get(7)?,
                favorite: false,
                tags: Vec::new(),
            })
        })?;
        let mut out: Vec<MediaInfo> = rows.collect::<Result<_, _>>()?;
        let mut tag_stmt = self.conn.prepare(
            "SELECT mt.media_id, t.name FROM media_tags mt JOIN tags t ON t.id = mt.tag_id
             ORDER BY mt.media_id, t.name",
        )?;
        let pairs =
            tag_stmt.query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))?;
        let mut by_id: HashMap<i64, Vec<String>> = HashMap::new();
        for pair in pairs {
            let (id, name) = pair?;
            by_id.entry(id).or_default().push(name);
        }
        for m in &mut out {
            if let Some(tags) = by_id.remove(&m.id) {
                m.tags = tags;
                m.favorite = m.tags.iter().any(|t| t == FAVORITE_TAG);
            }
        }
        Ok(out)
    }

    pub fn tag_counts(&mut self) -> Result<Vec<(String, i64)>, rusqlite::Error> {
        let mut stmt = self.conn.prepare(
            "SELECT t.name, COUNT(mt.media_id) AS c FROM tags t
             LEFT JOIN media_tags mt ON mt.tag_id = t.id
             GROUP BY t.id ORDER BY c DESC, t.name",
        )?;
        let rows = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))?;
        rows.collect()
    }

    pub fn set_favorite(&mut self, id: i64, on: bool) -> Result<(), rusqlite::Error> {
        let tag_id = self.ensure_tag(FAVORITE_TAG)?;
        if on {
            self.conn.execute(
                "INSERT OR IGNORE INTO media_tags(media_id, tag_id) VALUES (?1, ?2)",
                params![id, tag_id],
            )?;
        } else {
            self.conn.execute(
                "DELETE FROM media_tags WHERE media_id = ?1 AND tag_id = ?2",
                params![id, tag_id],
            )?;
        }
        Ok(())
    }

    pub fn set_title_artist(
        &mut self,
        id: i64,
        title: Option<&str>,
        artist: Option<&str>,
    ) -> Result<(), rusqlite::Error> {
        self.conn.execute(
            "UPDATE media SET title = COALESCE(NULLIF(title, ''), ?2),
             artist = COALESCE(NULLIF(artist, ''), ?3) WHERE id = ?1",
            params![id, title, artist],
        )?;
        Ok(())
    }

    pub fn update_playback_stats(
        &mut self,
        id: i64,
        duration: Option<f64>,
        bitrate: Option<u64>,
    ) -> Result<(), rusqlite::Error> {
        self.conn.execute(
            "UPDATE media SET
                 duration = CASE WHEN duration IS NULL OR duration <= 0 THEN ?2 ELSE duration END,
                 bitrate = CASE WHEN bitrate IS NULL THEN ?3 ELSE bitrate END
             WHERE id = ?1",
            params![id, duration, bitrate],
        )?;
        Ok(())
    }
}
