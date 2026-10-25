use rusqlite::Connection;

pub(super) fn create_tables(conn: &Connection) -> Result<(), rusqlite::Error> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS media (
             id INTEGER PRIMARY KEY AUTOINCREMENT,
             uri TEXT NOT NULL UNIQUE,
             kind TEXT NOT NULL DEFAULT 'offline',
             title TEXT,
             artist TEXT,
             duration REAL,
             bitrate INTEGER,
             source TEXT,
             added_at INTEGER NOT NULL
         );
         CREATE TABLE IF NOT EXISTS tags (
             id INTEGER PRIMARY KEY AUTOINCREMENT,
             name TEXT NOT NULL UNIQUE
         );
         CREATE TABLE IF NOT EXISTS media_tags (
             media_id INTEGER NOT NULL REFERENCES media(id) ON DELETE CASCADE,
             tag_id INTEGER NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
             PRIMARY KEY (media_id, tag_id)
         );
         CREATE INDEX IF NOT EXISTS idx_media_tags_tag ON media_tags(tag_id);",
    )
}
