use rmp::proto::{RepeatMode, Theme};
use rmp::storage::{LastState, Paths, config_dir, state};
use std::fs;
use std::path::PathBuf;

fn temp_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("rmp2-st-{}-{}", tag, std::process::id()));
    let _ = fs::create_dir_all(&dir);
    dir
}

#[test]
fn paths_for_root_names_every_runtime_file() {
    let paths = Paths::for_root(PathBuf::from("/tmp/rmp2-fake"));
    assert_eq!(paths.root, PathBuf::from("/tmp/rmp2-fake"));
    assert_eq!(paths.config, PathBuf::from("/tmp/rmp2-fake/config.toml"));
    assert_eq!(paths.db, PathBuf::from("/tmp/rmp2-fake/library.sqlite3"));
    assert_eq!(paths.sock, PathBuf::from("/tmp/rmp2-fake/rmp.sock"));
    assert_eq!(paths.pid, PathBuf::from("/tmp/rmp2-fake/rmp.pid"));
    assert_eq!(paths.state, PathBuf::from("/tmp/rmp2-fake/last-state.json"));
    assert_eq!(paths.log, PathBuf::from("/tmp/rmp2-fake/rmp.log"));
}

#[test]
fn config_dir_respects_rmp2_dir() {
    unsafe {
        std::env::set_var("RMP2_DIR", "/tmp/rmp2-explicit");
    }
    assert_eq!(config_dir(), PathBuf::from("/tmp/rmp2-explicit"));
    unsafe {
        std::env::remove_var("RMP2_DIR");
    }
}

#[test]
fn last_state_roundtrips_and_defaults() {
    let dir = temp_dir("roundtrip");
    let path = dir.join("last-state.json");

    let missing = state::load(&path);
    assert_eq!(missing.theme, Theme::Dark);
    assert_eq!(missing.volume, 70);
    assert_eq!(missing.repeat, RepeatMode::Off);
    assert!(!missing.shuffle);
    assert_eq!(missing.media_id, None);

    let full = LastState {
        theme: Theme::Light,
        transparent: true,
        volume: 42,
        repeat: RepeatMode::One,
        shuffle: true,
        active_tags: vec!["rock".into(), "live".into()],
        media_id: Some(17),
        position: 88.5,
        playing: false,
    };
    state::save(&path, &full);
    let back = state::load(&path);
    assert_eq!(back.theme, Theme::Light);
    assert!(back.transparent);
    assert_eq!(back.volume, 42);
    assert_eq!(back.repeat, RepeatMode::One);
    assert!(back.shuffle);
    assert_eq!(back.active_tags, vec!["rock".to_string(), "live".into()]);
    assert_eq!(back.media_id, Some(17));
    assert!((back.position - 88.5).abs() < f64::EPSILON);

    fs::write(&path, "garbage { not json").unwrap();
    let corrupt = state::load(&path);
    assert_eq!(corrupt.volume, 70, "corrupt state falls back to defaults");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn missing_optional_fields_deserialize_with_defaults() {
    let s: LastState = serde_json::from_str(
        r#"{"volume": 55, "repeat": "All", "shuffle": false, "active_tags": [], "position": 0.0, "playing": false}"#,
    )
    .unwrap();
    assert_eq!(s.theme, Theme::Dark, "theme defaults on absent field");
    assert!(!s.transparent);
    assert_eq!(s.volume, 55);
    assert_eq!(s.repeat, RepeatMode::All);
    assert_eq!(s.media_id, None);
}
