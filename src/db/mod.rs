pub mod connection;
pub mod migrations;
pub mod queries;
pub mod schema;

use rusqlite::Connection;

pub struct Library {
    pub(super) conn: Connection,
}

pub struct NewMedia<'a> {
    pub uri: &'a str,
    pub title: Option<&'a str>,
    pub kind: &'a str,
    pub artist: Option<&'a str>,
    pub duration: Option<f64>,
    pub bitrate: Option<u64>,
    pub source: Option<&'a str>,
}

pub fn classify_uri(uri: &str) -> (&'static str, Option<String>) {
    for scheme in ["http://", "https://"] {
        if let Some(rest) = uri.strip_prefix(scheme) {
            let host = rest
                .split(['/', '?', '#'])
                .next()
                .unwrap_or("")
                .split(':')
                .next()
                .unwrap_or("")
                .to_string();
            return ("online", if host.is_empty() { None } else { Some(host) });
        }
    }
    ("offline", None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_kinds() {
        assert_eq!(classify_uri("https://example.com/a.mp3").0, "online");
        assert_eq!(
            classify_uri("https://example.com/a.mp3").1.as_deref(),
            Some("example.com")
        );
        assert_eq!(
            classify_uri("http://x.io:8080/s").1.as_deref(),
            Some("x.io")
        );
        assert_eq!(classify_uri("/home/u/m.mp3").0, "offline");
    }

    #[test]
    fn roundtrip_media() {
        let dir = std::env::temp_dir().join(format!("vmp-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let db = dir.join("roundtrip.sqlite3");
        let _ = std::fs::remove_file(&db);
        let mut lib = Library::open(&db).unwrap();
        let id = lib
            .add_media(
                NewMedia {
                    uri: "/tmp/a.mp3",
                    title: Some("A Song"),
                    kind: "offline",
                    artist: Some("Artist"),
                    duration: Some(220.0),
                    bitrate: Some(128_000),
                    source: None,
                },
                &["rock".into(), "favorite".into()],
            )
            .unwrap();
        let m = lib.media(id).unwrap().unwrap();
        assert_eq!(m.title.as_deref(), Some("A Song"));
        assert!(m.tags.contains(&"rock".into()));
        assert!(m.tags.contains(&"favorite".into()));

        lib.set_favorite(id, false).unwrap();
        let m = lib.media(id).unwrap().unwrap();
        assert!(!m.favorite);

        lib.update_media(id, Some("T"), Some("Ar"), &["jazz".into()])
            .unwrap();
        let m = lib.media(id).unwrap().unwrap();
        assert_eq!(m.title.as_deref(), Some("T"));
        assert_eq!(m.tags, vec!["jazz".to_string()]);

        let all = lib.all_media().unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].tags, vec!["jazz".to_string()]);
        let counts = lib.tag_counts().unwrap();
        assert!(counts.contains(&("jazz".to_string(), 1)));
        assert!(counts.contains(&("rock".to_string(), 0)));

        lib.delete_media(id).unwrap();
        assert!(lib.media(id).unwrap().is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn add_updates_existing_uri() {
        let dir = std::env::temp_dir().join(format!("vmp-test2-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let db = dir.join("t.sqlite3");
        let _ = std::fs::remove_file(&db);
        let mut lib = Library::open(&db).unwrap();
        let id1 = lib
            .add_media(
                NewMedia {
                    uri: "/x.mp3",
                    title: Some("One"),
                    kind: "offline",
                    artist: None,
                    duration: None,
                    bitrate: None,
                    source: None,
                },
                &[],
            )
            .unwrap();
        let id2 = lib
            .add_media(
                NewMedia {
                    uri: "/x.mp3",
                    title: Some("Two"),
                    kind: "offline",
                    artist: None,
                    duration: None,
                    bitrate: None,
                    source: None,
                },
                &[],
            )
            .unwrap();
        assert_eq!(id1, id2);
        let m = lib.media(id1).unwrap().unwrap();
        assert_eq!(m.title.as_deref(), Some("Two"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn migrates_legacy_name_column() {
        let dir = std::env::temp_dir().join(format!("vmp-test3-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let db = dir.join("legacy.sqlite3");
        let _ = std::fs::remove_file(&db);
        let conn = rusqlite::Connection::open(&db).unwrap();
        conn.execute_batch(
            "CREATE TABLE media (
                 id INTEGER PRIMARY KEY AUTOINCREMENT,
                 uri TEXT NOT NULL UNIQUE,
                 name TEXT NOT NULL,
                 kind TEXT NOT NULL DEFAULT 'offline',
                 title TEXT,
                 artist TEXT,
                 duration REAL,
                 bitrate INTEGER,
                 source TEXT,
                 added_at INTEGER NOT NULL
             );
             INSERT INTO media(uri, name, title, added_at)
             VALUES ('/a.mp3', 'legacy name', NULL, 1);
             INSERT INTO media(uri, name, title, added_at)
             VALUES ('/b.mp3', 'legacy Two', NULL, 2);",
        )
        .unwrap();
        drop(conn);
        let mut lib = Library::open(&db).unwrap();
        let m = lib.media(1).unwrap().unwrap();
        assert_eq!(m.title.as_deref(), Some("legacy name"));
        let m = lib.media(2).unwrap().unwrap();
        assert_eq!(m.title.as_deref(), Some("legacy Two"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
