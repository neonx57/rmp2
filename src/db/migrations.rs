use rusqlite::Connection;

pub(super) fn drop_legacy_name(conn: &Connection) -> Result<(), rusqlite::Error> {
    let mut stmt = conn.prepare("PRAGMA table_info(media)")?;
    let cols: Vec<String> = stmt
        .query_map([], |r| r.get::<_, String>(1))?
        .collect::<Result<_, _>>()?;
    if cols.iter().any(|c| c == "name") {
        conn.execute_batch(
            "UPDATE media SET title = name WHERE title IS NULL OR trim(title) = '';
             ALTER TABLE media DROP COLUMN name;",
        )?;
    }
    Ok(())
}
