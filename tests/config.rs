use rmp::app::action::{ACTIONS, Action, action_from_str, action_group, action_label, action_str};
use rmp::config::Config;
use rmp::config::defaults::{default_bindings, default_config_text};
use std::fs;
use std::path::PathBuf;

fn temp_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("rmp2-it-{}-{}", tag, std::process::id()));
    let _ = fs::create_dir_all(&dir);
    dir
}

#[test]
fn parse_merges_defaults_into_partial_config() {
    let cfg = Config::parse("[keybindings]\ntoggle = \"x\"\n").unwrap();
    assert_eq!(cfg.keybindings.get("toggle"), Some(&vec!["x".to_string()]));
    assert_eq!(
        cfg.keybindings.get("move_up"),
        Some(&vec!["k".to_string(), "up".to_string()])
    );
    assert_eq!(cfg.mpv_binary, "mpv");
    assert_eq!(cfg.volume_step, 5);
    assert!((cfg.seek_step - 5.0).abs() < f64::EPSILON);
}

#[test]
fn single_and_array_forms_and_validation_errors() {
    let cfg =
        Config::parse("[keybindings]\nconfirm_quit = \"q\"\nseek_back = [\"x\", \"y\"]\n").unwrap();
    assert_eq!(cfg.keybindings.get("confirm_quit"), Some(&vec!["q".into()]));
    assert_eq!(
        cfg.keybindings.get("seek_back"),
        Some(&vec!["x".into(), "y".into()])
    );

    assert!(Config::parse("[keybindings]\nfrobnicate = \"q\"\n").is_err());
    assert!(Config::parse("[keybindings]\ntoggle = \"ctrl+zz\"\n").is_err());
    let err = Config::parse("[keybindings]\nsearch = \"q\"\nconfirm_quit = [\"q\", \"esc\"]\n")
        .unwrap_err();
    assert!(err.contains("bound to multiple actions"));
    assert!(err.starts_with("invalid config:"));
}

#[test]
fn none_unbinds_and_titles_default() {
    let cfg =
        Config::parse("[keybindings]\nmove_down = \"none\"\n[titles]\nqueue = \"List\"\n").unwrap();
    assert_eq!(cfg.keybindings.get("move_down"), Some(&vec!["none".into()]));

    let bare = Config::parse("").unwrap();
    assert_eq!(bare.titles.filter, "Filter");
    assert_eq!(bare.titles.mini, "Mini Queue");
    assert_eq!(bare.titles.info, "Info");
    assert_eq!(bare.titles.queue, "Queue");
}

#[test]
fn default_text_roundtrips_through_parse() {
    let raw = default_config_text();
    let cfg = Config::parse(&raw).unwrap();
    assert_eq!(cfg.keybindings, default_bindings());
    assert!(raw.contains("confirm_quit = [\"q\", \"esc\"]"));
    assert!(raw.contains("mpv_binary = \"mpv\""));
    assert!(raw.contains("seek_step = 5.0"));
}

#[test]
fn action_catalog_is_well_formed() {
    for (a, name) in ACTIONS {
        assert_eq!(action_from_str(name), Some(*a));
        assert_eq!(action_str(*a), *name);
        assert!(!action_group(*a).is_empty());
        assert!(!action_label(*a).is_empty());
    }
    assert_eq!(action_from_str("definitely-not-real"), None);
    assert_eq!(action_label(Action::NextTrack), "next track");
    assert_eq!(action_group(Action::NextTrack), "Playback");
}

#[test]
fn load_creates_a_default_config_file_when_missing() {
    let dir = temp_dir("create-default");
    let path = dir.join("config.toml");
    let cfg = Config::load(&path).expect("missing file yields defaults");
    assert_eq!(cfg.mpv_binary, "mpv");
    let written = fs::read_to_string(&path).unwrap();
    assert!(written.contains("[keybindings]"));
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn load_rejects_invalid_config_without_touching_disk() {
    let dir = temp_dir("invalid");
    let path = dir.join("config.toml");
    fs::write(&path, "[keybindings]\ntoggle = \"definitely-bad-key\"\n").unwrap();
    let err = Config::load(&path).unwrap_err().to_string();
    assert!(err.contains("invalid key \"definitely-bad-key\""), "{err}");
    assert_eq!(
        fs::read_to_string(&path).unwrap(),
        "[keybindings]\ntoggle = \"definitely-bad-key\"\n"
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn legacy_key_action_format_is_migrated_in_file() {
    let dir = temp_dir("legacy");
    let path = dir.join("config.toml");
    fs::write(
        &path,
        "[keybindings]\nj = \"move_down\"\nk = \"move_up\"\nA = \"add_mini\"\nq = \"none\"\n",
    )
    .unwrap();
    let cfg = Config::load(&path).expect("migrates legacy format");
    assert_eq!(
        cfg.keybindings.get("move_down"),
        Some(&vec!["down".into(), "j".into()])
    );
    assert_eq!(cfg.keybindings.get("add_mini"), Some(&vec!["A".into()]));
    assert_eq!(
        cfg.keybindings.get("confirm_quit"),
        Some(&vec!["esc".into()]),
        "q was disabled, default esc remains"
    );
    let rewritten = fs::read_to_string(&path).unwrap();
    assert!(rewritten.contains("Auto-migrated"));
    assert!(rewritten.contains("move_down = [\"down\", \"j\"]"));
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn migrate_legacy_carries_over_tuned_scalars() {
    let raw = "mpv_binary = \"mpv2\"\nvolume_step = 3\nseek_step = 9.5\n[keybindings]\nj = \"move_down\"\n";
    let out = load_legacy_raw(raw);
    assert!(out.contains("mpv_binary = \"mpv2\""));
    assert!(out.contains("volume_step = 3"));
    assert!(out.contains("seek_step = 9.5"));
    let cfg = Config::parse(&out).unwrap();
    assert_eq!(cfg.mpv_binary, "mpv2");
    assert_eq!(cfg.volume_step, 3);
}

fn load_legacy_raw(raw: &str) -> String {
    let dir = temp_dir("legacy-migrate");
    let path = dir.join("config.toml");
    fs::write(&path, raw).unwrap();
    let cfg = Config::load(&path).unwrap();
    let text = fs::read_to_string(&path).unwrap();
    assert_eq!(cfg.volume_step, 3);
    let _ = fs::remove_dir_all(&dir);
    text
}
