use rmp::db::{Library, NewMedia, classify_uri};
use rmp::proto::FAVORITE_TAG;
use std::fs;
use std::path::PathBuf;

fn temp_db(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("rmp2-db-{}-{}", tag, std::process::id()));
    let _ = fs::create_dir_all(&dir);
    dir.join("library.sqlite3")
}

fn entry<'a>(uri: &'a str, title: &'a str, tags: Vec<&'a str>) -> (NewMedia<'a>, Vec<String>) {
    (
        NewMedia {
            uri,
            title: Some(title),
            kind: "offline",
            artist: None,
            duration: Some(95.0),
            bitrate: Some(256_000),
            source: None,
        },
        tags.into_iter().map(String::from).collect(),
    )
}

#[test]
fn classify_uri_kinds() {
    assert_eq!(classify_uri("/local/a.mp3").0, "offline");
    assert_eq!(classify_uri("/local/a.mp3").1, None);
    assert_eq!(classify_uri("https://radio.example.org/stream").0, "online");
    assert_eq!(
        classify_uri("https://radio.example.org/stream")
            .1
            .as_deref(),
        Some("radio.example.org")
    );
    assert_eq!(
        classify_uri("https://radio.example.org:8443/x?q=1")
            .1
            .as_deref(),
        Some("radio.example.org"),
        "port, query and fragment are stripped"
    );
    assert_eq!(
        classify_uri("http://127.0.0.1:8000").1.as_deref(),
        Some("127.0.0.1"),
    );
}

#[test]
fn full_media_lifecycle_with_tags() {
    let db = temp_db("lifecycle");
    let _ = fs::remove_file(&db);
    let mut lib = Library::open(&db).unwrap();

    let (m, tags) = entry("/music/one.flac", "One", vec!["rock", "live"]);
    let id = lib.add_media(m, &tags).unwrap();
    assert!(id > 0);

    let got = lib.media(id).unwrap().expect("added media readable");
    assert_eq!(got.title.as_deref(), Some("One"));
    assert_eq!(got.kind, "offline");
    assert!((got.duration.unwrap() - 95.0).abs() < f64::EPSILON);
    assert_eq!(got.tags, vec!["live".to_string(), "rock".to_string()]);
    assert!(!got.favorite, "favorite only via the reserved tag");

    lib.set_favorite(id, false).unwrap();
    let got = lib.media(id).unwrap().unwrap();
    assert!(!got.favorite);
    assert_eq!(got.tags, vec!["live".to_string(), "rock".to_string()]);

    lib.update_media(id, Some("Renamed"), Some("Artist"), &["jazz".into()])
        .unwrap();
    let got = lib.media(id).unwrap().unwrap();
    assert_eq!(got.title.as_deref(), Some("Renamed"));
    assert_eq!(got.artist.as_deref(), Some("Artist"));
    assert_eq!(got.tags, vec!["jazz".to_string()], "update replaces tags");

    let all = lib.all_media().unwrap();
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].tags, vec!["jazz".to_string()]);

    let counts = lib.tag_counts().unwrap();
    assert!(counts.contains(&("jazz".to_string(), 1)));
    assert!(counts.contains(&("rock".to_string(), 0)));

    lib.delete_media(id).unwrap();
    assert!(lib.media(id).unwrap().is_none());
    let _ = fs::remove_dir_all(db.parent().unwrap());
}

#[test]
fn uri_upsert_keeps_one_row_per_uri() {
    let db = temp_db("upsert");
    let _ = fs::remove_file(&db);
    let mut lib = Library::open(&db).unwrap();
    let (m1, t1) = entry("/u/same.mp3", "First", vec!["a"]);
    let (m2, t2) = entry("/u/same.mp3", "Second", vec!["b"]);
    let id1 = lib.add_media(m1, &t1).unwrap();
    let id2 = lib.add_media(m2, &t2).unwrap();
    assert_eq!(id1, id2);
    let got = lib.media(id1).unwrap().unwrap();
    assert_eq!(got.title.as_deref(), Some("Second"), "latest write wins");
    assert_eq!(
        got.tags,
        vec!["b".to_string()],
        "tags are replaced, not merged"
    );
    assert_eq!(lib.all_media().unwrap().len(), 1);
    let _ = fs::remove_dir_all(db.parent().unwrap());
}

#[test]
fn playback_stats_fill_blanks_only_and_title_artist_fallbacks() {
    let db = temp_db("stats");
    let _ = fs::remove_file(&db);
    let mut lib = Library::open(&db).unwrap();
    let blank = NewMedia {
        uri: "/v/blank.mp3",
        title: None,
        kind: "offline",
        artist: None,
        duration: None,
        bitrate: None,
        source: None,
    };
    let id = lib.add_media(blank, &[]).unwrap();

    lib.update_playback_stats(id, Some(150.0), Some(192_000))
        .unwrap();
    let got = lib.media(id).unwrap().unwrap();
    assert_eq!(got.duration, Some(150.0));
    assert_eq!(got.bitrate, Some(192_000));

    lib.update_playback_stats(id, Some(999.0), Some(8_000))
        .unwrap();
    let got = lib.media(id).unwrap().unwrap();
    assert_eq!(got.duration, Some(150.0), "existing duration preserved");
    assert_eq!(got.bitrate, Some(192_000), "existing bitrate preserved");

    lib.set_title_artist(id, Some("Live Title"), Some("Act"))
        .unwrap();
    let got = lib.media(id).unwrap().unwrap();
    assert_eq!(got.title.as_deref(), Some("Live Title"));
    assert_eq!(got.artist.as_deref(), Some("Act"));
    let _ = fs::remove_dir_all(db.parent().unwrap());
}

#[test]
fn legacy_name_column_is_migrated_on_open() {
    let dir = std::env::temp_dir().join(format!("rmp2-db-legacy-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let db = dir.join("library.sqlite3");
    let _ = fs::remove_file(&db);
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
         VALUES ('/old/a.mp3', 'Raw Name', NULL, 1);
         INSERT INTO media(uri, name, title, added_at)
         VALUES ('/old/b.mp3', 'Throws Title Away', 'Existing', 2);",
    )
    .unwrap();
    drop(conn);

    let mut lib = Library::open(&db).unwrap();
    let all = lib.all_media().unwrap();
    assert_eq!(all.len(), 2);
    let a = all.iter().find(|m| m.uri == "/old/a.mp3").unwrap();
    assert_eq!(
        a.title.as_deref(),
        Some("Raw Name"),
        "name moved into title"
    );
    let b = all.iter().find(|m| m.uri == "/old/b.mp3").unwrap();
    assert_eq!(
        b.title.as_deref(),
        Some("Existing"),
        "title not overwritten"
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn favorite_persists_across_reopen() {
    let db = temp_db("favorite");
    let _ = fs::remove_file(&db);
    let mut lib = Library::open(&db).unwrap();
    let (m, _t) = entry("/f/a.mp3", "A", vec![]);
    let id = lib.add_media(m, &[]).unwrap();
    assert!(
        !lib.media(id).unwrap().unwrap().favorite,
        "added without favorite tag"
    );
    lib.set_favorite(id, true).unwrap();
    assert!(lib.media(id).unwrap().unwrap().favorite);

    drop(lib);
    let mut lib = Library::open(&db).unwrap();
    assert!(lib.media(id).unwrap().unwrap().favorite);
    lib.set_favorite(id, false).unwrap();
    assert!(!lib.media(id).unwrap().unwrap().favorite);
    let counts = lib.tag_counts().unwrap();
    assert_eq!(
        counts.iter().find(|(n, _)| n == FAVORITE_TAG).unwrap().1,
        0,
        "tag row survives with count zero"
    );
    let _ = fs::remove_dir_all(db.parent().unwrap());
}
