use super::Library;
use super::migrations::drop_legacy_name;
use super::schema::create_tables;
use crate::proto::FAVORITE_TAG;
use rusqlite::Connection;
use std::path::Path;

impl Library {
    pub fn open(path: &Path) -> Result<Self, rusqlite::Error> {
        let conn = Connection::open(path)?;
        let lib = Self { conn };
        lib.migrate()?;
        Ok(lib)
    }

    fn migrate(&self) -> Result<(), rusqlite::Error> {
        self.conn.execute_batch("PRAGMA foreign_keys = ON;")?;
        create_tables(&self.conn)?;
        drop_legacy_name(&self.conn)?;
        let _ = self.ensure_tag(FAVORITE_TAG)?;
        Ok(())
    }
}
