#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExBuiltin {
    pub name: &'static str,
    pub aliases: &'static [&'static str],
    pub usage: &'static str,
    pub desc: &'static str,
}

pub const BUILTINS: &[ExBuiltin] = &[
    ExBuiltin {
        name: "play",
        aliases: &[],
        usage: ":play [n]",
        desc: "play the selection, or the n-th queue item",
    },
    ExBuiltin {
        name: "pause",
        aliases: &[],
        usage: ":pause",
        desc: "pause playback",
    },
    ExBuiltin {
        name: "resume",
        aliases: &[],
        usage: ":resume",
        desc: "resume playback",
    },
    ExBuiltin {
        name: "toggle",
        aliases: &[],
        usage: ":toggle",
        desc: "play or pause",
    },
    ExBuiltin {
        name: "stop",
        aliases: &[],
        usage: ":stop",
        desc: "stop playback and unload the track",
    },
    ExBuiltin {
        name: "next",
        aliases: &["n"],
        usage: ":next, :n",
        desc: "next track",
    },
    ExBuiltin {
        name: "prev",
        aliases: &[],
        usage: ":prev",
        desc: "previous track",
    },
    ExBuiltin {
        name: "seek",
        aliases: &[],
        usage: ":seek [+N|-N|M:SS]",
        desc: "seek by seconds or to a time; bare shows the position",
    },
    ExBuiltin {
        name: "volume",
        aliases: &["vol"],
        usage: ":volume [+N|-N|N]",
        desc: "change or set the volume; bare shows the level",
    },
    ExBuiltin {
        name: "repeat",
        aliases: &[],
        usage: ":repeat [off|all|one]",
        desc: "cycle or set the repeat mode",
    },
    ExBuiltin {
        name: "shuffle",
        aliases: &[],
        usage: ":shuffle [on|off]",
        desc: "toggle or set shuffle",
    },
    ExBuiltin {
        name: "tag",
        aliases: &[],
        usage: ":tag NAME",
        desc: "toggle a tag in the filter",
    },
    ExBuiltin {
        name: "tags",
        aliases: &[],
        usage: ":tags",
        desc: "list tags with counts and checked marks",
    },
    ExBuiltin {
        name: "clear_tags",
        aliases: &[],
        usage: ":clear_tags",
        desc: "uncheck every tag",
    },
    ExBuiltin {
        name: "search",
        aliases: &[],
        usage: ":search PATTERN",
        desc: "set a regex filter; bare clears it",
    },
    ExBuiltin {
        name: "noh",
        aliases: &[],
        usage: ":noh",
        desc: "clear the search filter",
    },
    ExBuiltin {
        name: "add",
        aliases: &[],
        usage: ":add [URI [TAGS]]",
        desc: "add media; quote a uri with spaces; bare opens the dialog",
    },
    ExBuiltin {
        name: "edit",
        aliases: &[],
        usage: ":edit",
        desc: "edit the selected media",
    },
    ExBuiltin {
        name: "delete",
        aliases: &[],
        usage: ":delete [!]",
        desc: "delete the selected media; ! skips the confirm dialog",
    },
    ExBuiltin {
        name: "favorite",
        aliases: &[],
        usage: ":favorite",
        desc: "toggle favorite on the selected media",
    },
    ExBuiltin {
        name: "mini",
        aliases: &[],
        usage: ":mini [add|remove|up|down]",
        desc: "pin, unpin or move the selected media in the mini queue",
    },
    ExBuiltin {
        name: "up",
        aliases: &[],
        usage: ":up",
        desc: "move the cursor up",
    },
    ExBuiltin {
        name: "down",
        aliases: &[],
        usage: ":down",
        desc: "move the cursor down",
    },
    ExBuiltin {
        name: "pane",
        aliases: &[],
        usage: ":pane [next|prev|NAME]",
        desc: "focus a pane: filter, mini, queue or info",
    },
    ExBuiltin {
        name: "help",
        aliases: &["h"],
        usage: ":help [keys|commands|NAME]",
        desc: "open help, list commands, or describe one command",
    },
    ExBuiltin {
        name: "settings",
        aliases: &[],
        usage: ":settings",
        desc: "open the settings dialog",
    },
    ExBuiltin {
        name: "theme",
        aliases: &[],
        usage: ":theme [dark|light]",
        desc: "toggle or set the colorscheme",
    },
    ExBuiltin {
        name: "transparency",
        aliases: &[],
        usage: ":transparency [on|off]",
        desc: "toggle or set transparency",
    },
    ExBuiltin {
        name: "detach",
        aliases: &[],
        usage: ":detach",
        desc: "close the TUI and keep the daemon playing",
    },
    ExBuiltin {
        name: "quit",
        aliases: &["q"],
        usage: ":q, :quit",
        desc: "quit the TUI and stop the daemon",
    },
    ExBuiltin {
        name: "checkhealth",
        aliases: &[],
        usage: ":checkhealth",
        desc: "report mpv and yt-dlp status",
    },
];

pub fn find(name: &str) -> Option<&'static ExBuiltin> {
    BUILTINS
        .iter()
        .find(|b| b.name == name || b.aliases.contains(&name))
}

pub fn resolve_canonical(word: &str) -> Option<&'static str> {
    if let Some(b) = find(word) {
        return Some(b.name);
    }
    let matches: Vec<&'static str> = BUILTINS
        .iter()
        .filter(|b| b.name.starts_with(word) || b.aliases.iter().any(|a| a.starts_with(word)))
        .map(|b| b.name)
        .collect();
    match matches.as_slice() {
        [only] => Some(only),
        _ => None,
    }
}

pub fn known_commands_hint() -> String {
    "try :help commands".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_and_aliases_are_unique() {
        let mut seen: Vec<&str> = Vec::new();
        for b in BUILTINS {
            for name in std::iter::once(b.name).chain(b.aliases.iter().copied()) {
                assert!(!seen.contains(&name), "duplicate command name {name}");
                seen.push(name);
            }
            assert!(b.usage.contains(':'), "usage should show the colon form");
            assert!(!b.desc.is_empty());
        }
    }

    #[test]
    fn find_matches_names_and_aliases() {
        assert_eq!(find("quit").map(|b| b.name), Some("quit"));
        assert_eq!(find("q").map(|b| b.name), Some("quit"));
        assert_eq!(find("n").map(|b| b.name), Some("next"));
        assert_eq!(find("nope"), None);
    }

    #[test]
    fn resolve_canonical_accepts_exact_alias_or_unique_prefix() {
        assert_eq!(resolve_canonical("quit"), Some("quit"));
        assert_eq!(resolve_canonical("q"), Some("quit"));
        assert_eq!(resolve_canonical("qu"), Some("quit"));
        assert_eq!(resolve_canonical("vol"), Some("volume"));
        assert_eq!(resolve_canonical("t"), None, "ambiguous prefix");
        assert_eq!(resolve_canonical("zzz"), None);
    }

    #[test]
    fn hint_points_at_the_command_list() {
        assert_eq!(known_commands_hint(), "try :help commands");
    }
}
