use std::fs;
use std::path::PathBuf;
use vmp::config::defaults::{default_bindings, default_init_lua};
use vmp::config::{Config, Titles};

fn temp_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("vmp-it-{}-{}", tag, std::process::id()));
    let _ = fs::create_dir_all(&dir);
    dir
}

fn write_config(tag: &str, body: &str) -> (PathBuf, PathBuf) {
    let dir = temp_dir(tag);
    let config = dir.join("init.lua");
    fs::write(&config, body).unwrap();
    (dir, config)
}

fn eval_body(tag: &str, body: &str) -> Result<Config, String> {
    let (dir, config) = write_config(tag, body);
    let result = Config::load(&config, &dir.join("lua")).map_err(|e| e.to_string());
    let _ = fs::remove_dir_all(dir);
    result
}

fn eval_ok(tag: &str, body: &str) -> Config {
    eval_body(tag, body).expect("init.lua should evaluate")
}

fn eval_err(tag: &str, body: &str) -> String {
    eval_body(tag, body).expect_err("init.lua should fail")
}

#[test]
fn missing_init_lua_yields_defaults_without_creating_the_file() {
    let dir = temp_dir("missing");
    let config = dir.join("init.lua");
    let cfg = Config::load(&config, &dir.join("lua")).unwrap();
    assert_eq!(cfg.mpv_binary, "mpv");
    assert!(cfg.enter_resume_loaded);
    assert_eq!(cfg.titles.queue, "Queue");
    assert_eq!(cfg.keybindings, default_bindings());
    assert!(!config.exists(), "vmp never writes init.lua itself");
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn empty_init_lua_yields_defaults() {
    let cfg = eval_ok("empty", "");
    assert_eq!(cfg.mpv_binary, "mpv");
    assert_eq!(cfg.volume_step, 5);
    assert!((cfg.seek_step - 5.0).abs() < f64::EPSILON);
    assert!(cfg.enter_resume_loaded);
    assert_eq!(cfg.titles.filter, "Filter");
    assert_eq!(cfg.titles.queue, "Queue");
    assert_eq!(cfg.titles.mini, "Mini Queue");
    assert_eq!(cfg.titles.info, "Info");
    assert_eq!(cfg.keybindings, default_bindings());
}

#[test]
fn opt_assignments_override_the_defaults() {
    let cfg = eval_ok(
        "opts",
        "vmp.opt.mpv_binary = \"mpv2\"\n\
         vmp.opt.volume_step = 3\n\
         vmp.opt.seek_step = 9.5\n\
         vmp.opt.enter_resume_loaded = false\n",
    );
    assert_eq!(cfg.mpv_binary, "mpv2");
    assert_eq!(cfg.volume_step, 3);
    assert!((cfg.seek_step - 9.5).abs() < f64::EPSILON);
    assert!(!cfg.enter_resume_loaded);
}

#[test]
fn titles_are_nested_and_merge_with_defaults() {
    let cfg = eval_ok("titles", "vmp.opt.titles.queue = \"Playlist\"\n");
    assert_eq!(cfg.titles.filter, "Filter");
    assert_eq!(cfg.titles.queue, "Playlist");
    assert_eq!(cfg.titles.mini, "Mini Queue");
    assert_eq!(cfg.titles.info, "Info");

    let cfg = eval_ok(
        "titles-table",
        "vmp.opt.titles = { filter = \"F\", queue = \"Q\", mini = \"M\", info = \"I\" }\n",
    );
    assert_eq!(
        cfg.titles,
        Titles {
            filter: "F".into(),
            queue: "Q".into(),
            mini: "M".into(),
            info: "I".into(),
        }
    );
}

#[test]
fn map_bindings_override_and_merge() {
    let cfg = eval_ok(
        "binds",
        "vmp.map.move_down = \"x\"\n\
         vmp.map.seek_back = { \"f8\", \"f9\" }\n",
    );
    assert_eq!(cfg.keybindings.get("move_down"), Some(&vec!["x".into()]));
    assert_eq!(
        cfg.keybindings.get("seek_back"),
        Some(&vec!["f8".into(), "f9".into()])
    );
    assert_eq!(
        cfg.keybindings.get("toggle"),
        Some(&vec!["space".into()]),
        "untouched actions keep their defaults"
    );
}

#[test]
fn map_none_still_disables_a_default() {
    let cfg = eval_ok("none", "vmp.map.move_down = \"none\"\n");
    assert_eq!(cfg.keybindings.get("move_down"), Some(&vec!["none".into()]));
}

#[test]
fn unknown_option_is_rejected() {
    let err = eval_err("unknown-opt", "vmp.opt.enter_resume_load = true\n");
    assert!(err.contains("unknown option 'enter_resume_load'"), "{err}");
    assert!(err.starts_with("invalid config:"), "{err}");
}

#[test]
fn unknown_action_in_map_is_rejected() {
    let err = eval_err("unknown-map", "vmp.map.frobnicate = \"q\"\n");
    assert!(err.contains("unknown action 'frobnicate'"), "{err}");
}

#[test]
fn wrong_value_types_are_reported_as_invalid_config() {
    let err = eval_err("bad-step", "vmp.opt.volume_step = 5.5\n");
    assert!(err.contains("volume_step: expected integer"), "{err}");
    let err = eval_err("bad-bool", "vmp.opt.enter_resume_loaded = 1\n");
    assert!(
        err.contains("enter_resume_loaded: expected boolean"),
        "{err}"
    );
    let err = eval_err("bad-binary", "vmp.opt.mpv_binary = 42\n");
    assert!(err.contains("mpv_binary: expected string"), "{err}");
}

#[test]
fn lua_runtime_errors_carry_the_init_lua_prefix() {
    let err = eval_err("runtime", "this_does_not_exist()\n");
    assert!(err.contains("invalid config:"), "{err}");
}

#[test]
fn requires_resolve_from_the_config_lua_dir() {
    let (dir, config) = write_config(
        "require",
        "local opts = require(\"custom.opts\")\n\
         vmp.opt.volume_step = opts.step\n",
    );
    let lua = dir.join("lua");
    fs::create_dir_all(&lua).unwrap();
    fs::create_dir_all(lua.join("custom")).unwrap();
    fs::write(lua.join("custom/opts.lua"), "return { step = 11 }\n").unwrap();
    let cfg = Config::load(&config, &lua).unwrap();
    assert_eq!(cfg.volume_step, 11);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn duplicate_keys_across_actions_still_fail() {
    let err = eval_err(
        "dup",
        "vmp.map.search = \"q\"\nvmp.map.confirm_quit = { \"q\", \"esc\" }\n",
    );
    assert!(err.contains("bound to multiple actions"), "{err}");
    assert!(err.starts_with("invalid config:"));
}

#[test]
fn default_init_lua_roundtrips_through_the_host() {
    let raw = default_init_lua();
    let cfg = eval_ok("default-text", &raw);
    assert_eq!(cfg.mpv_binary, "mpv");
    assert!(cfg.enter_resume_loaded);
    assert_eq!(cfg.keybindings, default_bindings());
}
