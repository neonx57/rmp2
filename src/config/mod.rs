pub mod defaults;
pub mod loader;
pub mod settings;
pub mod validation;

pub use settings::{Config, Titles};

#[cfg(test)]
mod tests {
    use super::loader::migrate_legacy;
    use super::*;
    use crate::app::action::*;
    use std::fs;

    fn bindings(cfg: &Config) -> &std::collections::BTreeMap<String, Vec<String>> {
        &cfg.keybindings
    }

    #[test]
    fn parse_merges_defaults_into_partial_config() {
        let cfg = Config::parse("[keybindings]\ntoggle = \"x\"\n").unwrap();
        assert_eq!(bindings(&cfg).get("toggle"), Some(&vec!["x".to_string()]));
        assert_eq!(
            bindings(&cfg).get("move_up"),
            Some(&vec!["k".to_string(), "up".to_string()])
        );
    }

    #[test]
    fn none_unbinds_an_action() {
        let cfg = Config::parse("[keybindings]\nmove_down = \"none\"\n").unwrap();
        assert_eq!(
            bindings(&cfg).get("move_down"),
            Some(&vec!["none".to_string()])
        );
    }

    #[test]
    fn rejects_unknown_action() {
        assert!(Config::parse("[keybindings]\nfrobnicate = \"q\"\n").is_err());
    }

    #[test]
    fn rejects_invalid_key_name() {
        assert!(Config::parse("[keybindings]\ntoggle = \"ctrl+zz\"\n").is_err());
    }

    #[test]
    fn rejects_duplicate_keys_across_actions() {
        let raw = "[keybindings]\nsearch = \"q\"\nconfirm_quit = [\"q\", \"esc\"]\n";
        let err = Config::parse(raw).unwrap_err();
        assert!(err.contains("bound to multiple actions"), "{err}");
    }

    #[test]
    fn single_and_array_binding_forms_both_work() {
        let cfg =
            Config::parse("[keybindings]\nconfirm_quit = \"q\"\nseek_back = [\"x\", \"y\"]\n")
                .unwrap();
        assert_eq!(
            bindings(&cfg).get("confirm_quit"),
            Some(&vec!["q".to_string()])
        );
        assert_eq!(
            bindings(&cfg).get("seek_back"),
            Some(&vec!["x".to_string(), "y".to_string()])
        );
    }

    #[test]
    fn titles_parse_with_defaults_filling_in() {
        let cfg = Config::parse("[titles]\nqueue = \"Playlist\"\n").unwrap();
        assert_eq!(cfg.titles.filter, "Filter");
        assert_eq!(cfg.titles.queue, "Playlist");
        assert_eq!(cfg.titles.mini, "Mini Queue");
        assert_eq!(cfg.titles.info, "Info");
    }

    #[test]
    fn default_text_roundtrips() {
        let raw = crate::config::defaults::default_config_text();
        let cfg = Config::parse(&raw).unwrap();
        assert_eq!(bindings(&cfg), &crate::config::defaults::default_bindings());
        assert!(raw.contains("confirm_quit = [\"q\", \"esc\"]"));
    }

    #[test]
    fn action_names_roundtrip() {
        for (a, name) in ACTIONS {
            assert_eq!(action_from_str(name), Some(*a));
            assert_eq!(action_str(*a), *name);
        }
        assert_eq!(action_from_str("nope"), None);
    }

    #[test]
    fn every_action_has_a_group() {
        let groups: Vec<&str> = ACTIONS.iter().map(|(a, _)| action_group(*a)).collect();
        assert!(groups.iter().all(|g| !g.is_empty()));
        assert_eq!(
            action_group(Action::NextTrack),
            action_group(Action::Repeat)
        );
        assert_ne!(action_group(Action::Search), action_group(Action::Detach));
    }

    #[test]
    fn action_labels_are_humanized() {
        assert_eq!(action_label(Action::NextTrack), "next track");
        assert_eq!(action_label(Action::MiniMoveUp), "mini move up");
        assert_eq!(action_label(Action::Settings), "settings");
    }

    #[test]
    fn legacy_config_is_migrated() {
        let raw = "mpv_binary = \"mpv\"\n[keybindings]\nj = \"move_down\"\nk = \"move_up\"\nA = \"add_mini\"\nq = \"none\"\n";
        let out = migrate_legacy(raw).unwrap();
        let cfg = Config::parse(&out).unwrap();
        assert_eq!(
            bindings(&cfg).get("move_down"),
            Some(&vec!["down".to_string(), "j".to_string()])
        );
        assert_eq!(bindings(&cfg).get("add_mini"), Some(&vec!["A".to_string()]));
        assert_eq!(cfg.mpv_binary, "mpv");
        assert!(out.contains("move_down = [\"down\", \"j\"]"));
        assert_eq!(
            bindings(&cfg).get("confirm_quit"),
            Some(&vec!["esc".to_string()])
        );
    }

    #[test]
    fn new_format_config_is_not_migrated() {
        let raw = "[keybindings]\nconfirm_quit = \"q\"\n";
        let out = migrate_legacy(raw);
        assert_eq!(out.as_deref(), None);
    }

    #[test]
    fn load_migrates_and_rewrites_file() {
        let dir = std::env::temp_dir().join(format!("rmp2-test-{}", std::process::id()));
        let _ = fs::create_dir_all(&dir);
        let path = dir.join("config.toml");
        let _ = fs::write(&path, "[keybindings]\nq = \"confirm_quit\"\n");
        let cfg = Config::load(&path).unwrap();
        assert!(cfg.keybindings.contains_key("confirm_quit"));
        let rewritten = fs::read_to_string(&path).unwrap();
        let cfg2 = Config::parse(&rewritten).unwrap();
        assert_eq!(cfg.keybindings, cfg2.keybindings);
        let _ = fs::remove_dir_all(&dir);
    }
}
