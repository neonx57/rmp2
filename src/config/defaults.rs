use crate::app::action::{ACTIONS, action_str};
use std::collections::BTreeMap;

pub fn default_bindings() -> BTreeMap<String, Vec<String>> {
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

fn is_lua_keyword(action: &str) -> bool {
    matches!(
        action,
        "and"
            | "break"
            | "do"
            | "else"
            | "elseif"
            | "end"
            | "false"
            | "for"
            | "function"
            | "goto"
            | "if"
            | "in"
            | "local"
            | "nil"
            | "not"
            | "or"
            | "repeat"
            | "return"
            | "then"
            | "true"
            | "until"
            | "while"
    )
}

fn binding_text(action: &str, keys: &[String]) -> String {
    let target = if is_lua_keyword(action) {
        format!("vmp.map[\"{action}\"]")
    } else {
        format!("vmp.map.{action}")
    };
    if keys.len() == 1 {
        format!("{target} = \"{k}\"\n", k = keys[0])
    } else {
        let list = keys
            .iter()
            .map(|k| format!("\"{k}\""))
            .collect::<Vec<_>>()
            .join(", ");
        format!("{target} = {{ {list} }}\n")
    }
}

pub fn default_init_lua() -> String {
    let mut out = String::new();
    out.push_str("-- vmp configuration\n");
    out.push_str("-- Executed as Lua on startup. Everything here mirrors the entry text\n");
    out.push_str("-- (options have these defaults already, so lines can be deleted).\n");
    out.push('\n');
    out.push_str("vmp.opt.mpv_binary = \"mpv\"\n");
    out.push_str("vmp.opt.volume_step = 5\n");
    out.push_str("vmp.opt.seek_step = 5.0\n");
    out.push_str("vmp.opt.enter_resume_loaded = true\n");
    out.push('\n');
    out.push_str("vmp.opt.titles.filter = \"Filter\"\n");
    out.push_str("vmp.opt.titles.queue = \"Queue\"\n");
    out.push_str("vmp.opt.titles.mini = \"Mini Queue\"\n");
    out.push_str("vmp.opt.titles.info = \"Info\"\n");
    out.push('\n');
    for (action, keys) in default_bindings() {
        out.push_str(&binding_text(&action, &keys));
    }
    out.push('\n');
    let actions: Vec<&str> = ACTIONS.iter().map(|(a, _)| action_str(*a)).collect();
    out.push_str(&format!("-- Actions: {}\n", actions.join(" ")));
    out
}
