use crate::app::action::{ACTIONS, action_str};

pub fn default_bindings() -> std::collections::BTreeMap<String, Vec<String>> {
    let pairs = [
        ("move_down", &["j", "down"][..]),
        ("move_up", &["k", "up"][..]),
        ("prev_section", &["h"][..]),
        ("next_section", &["l"][..]),
        ("cycle_focus", &["tab"][..]),
        ("cycle_focus_back", &["backtab"][..]),
        ("activate", &["enter"][..]),
        ("toggle", &["space"][..]),
        ("next_track", &["n"][..]),
        ("prev_track", &["p"][..]),
        ("repeat", &["r"][..]),
        ("shuffle", &["s"][..]),
        ("add_media", &["a"][..]),
        ("edit_media", &["e"][..]),
        ("search", &["/"][..]),
        ("favorite", &["f"][..]),
        ("add_mini", &["A"][..]),
        ("focus_mini", &["b"][..]),
        ("delete", &["d"][..]),
        ("mini_move_up", &["ctrl+k"][..]),
        ("mini_move_down", &["ctrl+j"][..]),
        ("volume_down", &["J"][..]),
        ("volume_up", &["K"][..]),
        ("seek_back", &["H", "left"][..]),
        ("seek_fwd", &["L", "right"][..]),
        ("confirm_quit", &["q", "esc"][..]),
        ("detach", &["Q"][..]),
        ("settings", &["f10"][..]),
        ("help", &["?"][..]),
        ("command", &[":"][..]),
    ];
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.iter().map(|s| s.to_string()).collect()))
        .collect()
}

pub(super) fn binding_text(action: &str, keys: &[String]) -> String {
    if keys.len() == 1 {
        format!("{action} = \"{}\"\n", keys[0])
    } else {
        let list = keys
            .iter()
            .map(|k| format!("\"{k}\""))
            .collect::<Vec<_>>()
            .join(", ");
        format!("{action} = [{list}]\n")
    }
}

pub fn default_config_text() -> String {
    let mut out = String::new();
    out.push_str("# rmp2 configuration\n");
    out.push('\n');
    out.push_str("# Keybindings map an action to a key or a list of keys.\n");
    out.push_str("# Use \"none\" to disable a default binding (e.g. confirm_quit = \"none\").\n");
    out.push('\n');
    let actions: Vec<&str> = ACTIONS.iter().map(|(a, _)| action_str(*a)).collect();
    out.push_str(&format!("# Actions: {}\n", actions.join(" ")));
    out.push_str("# Keys: single characters (letters, digits, symbols), uppercase for shift,\n");
    out.push_str("#       \"ctrl+<char>\" for control, \"f1\"..\"f12\", and the named keys\n");
    out.push_str(
        "#       \"space\" \"enter\" \"esc\" \"tab\" \"backtab\" \"up\" \"down\" \"left\"\n",
    );
    out.push_str(
        "#       \"right\" \"home\" \"end\" \"pageup\" \"pagedown\" \"insert\" \"delete\"\n",
    );
    out.push_str("#       \"backspace\"\n");
    out.push('\n');
    out.push_str("mpv_binary = \"mpv\"\n");
    out.push_str("volume_step = 5\n");
    out.push_str("seek_step = 5.0\n");
    out.push('\n');
    out.push_str("# Section titles used in the pane borders.\n");
    out.push_str("[titles]\n");
    out.push_str("filter = \"Filter\"\n");
    out.push_str("queue = \"Queue\"\n");
    out.push_str("mini = \"Mini Queue\"\n");
    out.push_str("info = \"Info\"\n");
    out.push('\n');
    out.push_str("[keybindings]\n");
    for (action, keys) in default_bindings() {
        out.push_str(&binding_text(&action, &keys));
    }
    out
}
