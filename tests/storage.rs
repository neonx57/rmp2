use std::fs;
use std::path::PathBuf;
use vmp::proto::{RepeatMode, Theme};
use vmp::storage::{LastState, Paths, state};

fn temp_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("vmp-st-{}-{}", tag, std::process::id()));
    let _ = fs::create_dir_all(&dir);
    dir
}

#[test]
fn paths_for_root_names_every_runtime_file() {
    let paths = Paths::for_root(PathBuf::from("/tmp/vmp-fake"));
    assert_eq!(paths.config_dir, PathBuf::from("/tmp/vmp-fake/config"));
    assert_eq!(paths.config, PathBuf::from("/tmp/vmp-fake/config/init.lua"));
    assert_eq!(
        paths.db,
        PathBuf::from("/tmp/vmp-fake/share/library.sqlite3")
    );
    assert_eq!(paths.sock, PathBuf::from("/tmp/vmp-fake/run/vmp.sock"));
    assert_eq!(paths.pid, PathBuf::from("/tmp/vmp-fake/run/vmp.pid"));
    assert_eq!(paths.mpv_sock, PathBuf::from("/tmp/vmp-fake/run/mpv.sock"));
    assert_eq!(
        paths.state,
        PathBuf::from("/tmp/vmp-fake/state/session.json")
    );
    assert_eq!(paths.log, PathBuf::from("/tmp/vmp-fake/state/vmp.log"));
    assert_eq!(paths.lua_dir(), PathBuf::from("/tmp/vmp-fake/config/lua"));
    assert_eq!(
        paths.probe_sock(7),
        PathBuf::from("/tmp/vmp-fake/run/probe-7.sock")
    );
}

#[test]
fn env_overrides_select_each_xdg_directory() {
    unsafe {
        std::env::set_var("VMP_CONFIG_DIR", "/tmp/vmp-x-cfg");
        std::env::set_var("VMP_DATA_DIR", "/tmp/vmp-x-data");
        std::env::set_var("VMP_STATE_DIR", "/tmp/vmp-x-state");
        std::env::set_var("VMP_RUN_DIR", "/tmp/vmp-x-run");
    }
    let paths = Paths::resolve();
    unsafe {
        std::env::remove_var("VMP_CONFIG_DIR");
        std::env::remove_var("VMP_DATA_DIR");
        std::env::remove_var("VMP_STATE_DIR");
        std::env::remove_var("VMP_RUN_DIR");
    }
    assert_eq!(paths.config, PathBuf::from("/tmp/vmp-x-cfg/init.lua"));
    assert_eq!(paths.db, PathBuf::from("/tmp/vmp-x-data/library.sqlite3"));
    assert_eq!(paths.state, PathBuf::from("/tmp/vmp-x-state/session.json"));
    assert_eq!(paths.sock, PathBuf::from("/tmp/vmp-x-run/vmp.sock"));
}

#[test]
fn for_dirs_creates_all_directories() {
    let root = temp_dir("mkdirs");
    let paths = Paths::for_root(root.clone());
    for dir in [
        paths.config_dir.clone(),
        paths.data_dir.clone(),
        paths.state_dir.clone(),
        paths.run_dir.clone(),
    ] {
        assert!(dir.is_dir(), "{}", dir.to_string_lossy());
    }
    let _ = fs::remove_dir_all(root);
}

#[test]
fn session_state_roundtrips_and_defaults() {
    let dir = temp_dir("roundtrip");
    let path = dir.join("session.json");

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
    assert_eq!(corrupt.volume, 70, "corrupt session falls back to defaults");
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
