use crate::commands::builtins;
use crate::proto::{RepeatMode, Theme};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HelpTopic {
    Keys,
    Commands,
    Command(String),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SeekTarget {
    Relative(f64),
    Absolute(f64),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VolumeArg {
    Delta(i32),
    Set(i32),
    Show,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddArgs {
    pub uri: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MiniSub {
    Add,
    Remove,
    Up,
    Down,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaneTarget {
    Next,
    Prev,
    Filter,
    Mini,
    Queue,
    Info,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExCommand {
    Empty,
    Quit,
    CheckHealth,
    Help(HelpTopic),
    Settings,
    Detach,
    Play(Option<usize>),
    Pause,
    Resume,
    Toggle,
    Stop,
    Next,
    Prev,
    Seek(Option<SeekTarget>),
    Volume(VolumeArg),
    Repeat(Option<RepeatMode>),
    Shuffle(Option<bool>),
    Tag(String),
    Tags,
    ClearTags,
    Search(Option<String>),
    Add(Option<AddArgs>),
    Edit,
    Delete {
        force: bool,
    },
    Favorite,
    Mini(MiniSub),
    Pane(PaneTarget),
    Up,
    Down,
    Theme(Option<Theme>),
    Transparency(Option<bool>),
    Invalid(String),
    Ambiguous {
        typed: String,
        candidates: Vec<String>,
    },
    Unknown(String),
}

pub fn parse_ex_command(line: &str) -> ExCommand {
    let line = line.trim();
    if line.is_empty() {
        return ExCommand::Empty;
    }
    let (head, rest) = split_head(line);
    let (word, force) = split_bang(head);
    resolve(word, force, rest)
}

fn split_head(line: &str) -> (&str, &str) {
    match line.find(char::is_whitespace) {
        Some(i) => (&line[..i], line[i..].trim_start()),
        None => (line, ""),
    }
}

fn split_bang(word: &str) -> (&str, bool) {
    match word.strip_suffix('!') {
        Some(w) if !w.is_empty() => (w, true),
        _ => (word, false),
    }
}

fn resolve(word: &str, force: bool, rest: &str) -> ExCommand {
    if let Some(b) = builtins::find(word) {
        return dispatch(b.name, force, rest);
    }
    let mut candidates: Vec<&'static str> = Vec::new();
    for b in builtins::BUILTINS {
        let hit = b.name.starts_with(word) || b.aliases.iter().any(|a| a.starts_with(word));
        if hit {
            candidates.push(b.name);
        }
    }
    candidates.sort_unstable();
    match candidates.as_slice() {
        [] => ExCommand::Unknown(word.to_string()),
        [only] => dispatch(only, force, rest),
        many => ExCommand::Ambiguous {
            typed: word.to_string(),
            candidates: many.iter().map(|c| c.to_string()).collect(),
        },
    }
}

fn dispatch(name: &str, force: bool, rest: &str) -> ExCommand {
    match name {
        "quit" => ExCommand::Quit,
        "checkhealth" => ExCommand::CheckHealth,
        "help" => parse_help(rest),
        "settings" => ExCommand::Settings,
        "detach" => ExCommand::Detach,
        "play" => parse_play(rest),
        "pause" => ExCommand::Pause,
        "resume" => ExCommand::Resume,
        "toggle" => ExCommand::Toggle,
        "stop" => ExCommand::Stop,
        "next" => ExCommand::Next,
        "prev" => ExCommand::Prev,
        "seek" => parse_seek(rest),
        "volume" => parse_volume(rest),
        "repeat" => parse_repeat(rest),
        "shuffle" => parse_shuffle(rest),
        "tag" => match arg_word(rest) {
            Some(name) => ExCommand::Tag(name),
            None => ExCommand::Invalid("tag: expected a tag name".into()),
        },
        "tags" => ExCommand::Tags,
        "clear_tags" => ExCommand::ClearTags,
        "search" => ExCommand::Search(arg_line(rest)),
        "noh" => ExCommand::Search(None),
        "add" => parse_add(rest),
        "edit" => ExCommand::Edit,
        "delete" => ExCommand::Delete { force },
        "favorite" => ExCommand::Favorite,
        "mini" => parse_mini(rest),
        "pane" => parse_pane(rest),
        "up" => ExCommand::Up,
        "down" => ExCommand::Down,
        "theme" => parse_theme(rest),
        "transparency" => parse_transparency(rest),
        _ => ExCommand::Unknown(name.to_string()),
    }
}

fn arg_line(rest: &str) -> Option<String> {
    let trimmed = rest.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn arg_word(rest: &str) -> Option<String> {
    rest.split_whitespace().next().map(|s| s.to_lowercase())
}

fn parse_help(rest: &str) -> ExCommand {
    match arg_word(rest).as_deref() {
        None => ExCommand::Help(HelpTopic::Keys),
        Some("keys") => ExCommand::Help(HelpTopic::Keys),
        Some("commands" | "cmd") => ExCommand::Help(HelpTopic::Commands),
        Some(other) => ExCommand::Help(HelpTopic::Command(other.to_string())),
    }
}

fn parse_play(rest: &str) -> ExCommand {
    match arg_word(rest) {
        None => ExCommand::Play(None),
        Some(word) => match word.parse::<usize>() {
            Ok(n) if n > 0 => ExCommand::Play(Some(n)),
            _ => ExCommand::Invalid("play: expected a positive queue number".into()),
        },
    }
}

fn parse_seek(rest: &str) -> ExCommand {
    let Some(word) = arg_word(rest) else {
        return ExCommand::Seek(None);
    };
    if let Some(delta) = signed_number(&word) {
        return ExCommand::Seek(Some(SeekTarget::Relative(delta)));
    }
    match parse_clock(&word) {
        Some(secs) => ExCommand::Seek(Some(SeekTarget::Absolute(secs))),
        None => ExCommand::Invalid("seek: expected [+N|-N|M:SS]".into()),
    }
}

fn signed_number(word: &str) -> Option<f64> {
    if !word.starts_with('+') && !word.starts_with('-') {
        return None;
    }
    word.parse::<f64>().ok().filter(|v| v.is_finite())
}

fn parse_clock(word: &str) -> Option<f64> {
    let parts: Vec<&str> = word.split(':').collect();
    if parts.is_empty() || parts.len() > 3 {
        return None;
    }
    let mut total = 0u64;
    for part in parts {
        if part.is_empty() || !part.chars().all(|c| c.is_ascii_digit()) {
            return None;
        }
        total = total * 60 + part.parse::<u64>().ok()?;
    }
    Some(total as f64)
}

fn parse_volume(rest: &str) -> ExCommand {
    let Some(word) = arg_word(rest) else {
        return ExCommand::Volume(VolumeArg::Show);
    };
    if let Some(delta) = signed_int(&word) {
        return ExCommand::Volume(VolumeArg::Delta(delta));
    }
    match word.parse::<i32>() {
        Ok(v) => ExCommand::Volume(VolumeArg::Set(v)),
        Err(_) => ExCommand::Invalid("volume: expected [+N|-N|N]".into()),
    }
}

fn signed_int(word: &str) -> Option<i32> {
    let (sign, digits) = match word.strip_prefix('+') {
        Some(d) => (1, d),
        None => {
            let d = word.strip_prefix('-')?;
            (-1, d)
        }
    };
    digits.parse::<i32>().ok().map(|v| sign * v)
}

fn parse_repeat(rest: &str) -> ExCommand {
    match arg_word(rest).as_deref() {
        None => ExCommand::Repeat(None),
        Some("off") => ExCommand::Repeat(Some(RepeatMode::Off)),
        Some("all") => ExCommand::Repeat(Some(RepeatMode::All)),
        Some("one") => ExCommand::Repeat(Some(RepeatMode::One)),
        Some(_) => ExCommand::Invalid("repeat: expected off, all or one".into()),
    }
}

fn parse_shuffle(rest: &str) -> ExCommand {
    match arg_word(rest).as_deref() {
        None => ExCommand::Shuffle(None),
        Some("on") => ExCommand::Shuffle(Some(true)),
        Some("off") => ExCommand::Shuffle(Some(false)),
        Some(_) => ExCommand::Invalid("shuffle: expected on or off".into()),
    }
}

fn parse_mini(rest: &str) -> ExCommand {
    match arg_word(rest).as_deref() {
        None | Some("add") => ExCommand::Mini(MiniSub::Add),
        Some("remove" | "rm") => ExCommand::Mini(MiniSub::Remove),
        Some("up") => ExCommand::Mini(MiniSub::Up),
        Some("down" | "dn") => ExCommand::Mini(MiniSub::Down),
        Some(_) => ExCommand::Invalid("mini: expected add, remove, up or down".into()),
    }
}

fn parse_pane(rest: &str) -> ExCommand {
    match arg_word(rest).as_deref() {
        None | Some("next") => ExCommand::Pane(PaneTarget::Next),
        Some("prev" | "previous") => ExCommand::Pane(PaneTarget::Prev),
        Some("filter") => ExCommand::Pane(PaneTarget::Filter),
        Some("mini") => ExCommand::Pane(PaneTarget::Mini),
        Some("queue") => ExCommand::Pane(PaneTarget::Queue),
        Some("info") => ExCommand::Pane(PaneTarget::Info),
        Some(_) => {
            ExCommand::Invalid("pane: expected next, prev, filter, mini, queue or info".into())
        }
    }
}

fn parse_theme(rest: &str) -> ExCommand {
    match arg_word(rest).as_deref() {
        None => ExCommand::Theme(None),
        Some("dark") => ExCommand::Theme(Some(Theme::Dark)),
        Some("light") => ExCommand::Theme(Some(Theme::Light)),
        Some(_) => ExCommand::Invalid("theme: expected dark or light".into()),
    }
}

fn parse_transparency(rest: &str) -> ExCommand {
    match arg_word(rest).as_deref() {
        None => ExCommand::Transparency(None),
        Some("on") => ExCommand::Transparency(Some(true)),
        Some("off") => ExCommand::Transparency(Some(false)),
        Some(_) => ExCommand::Invalid("transparency: expected on or off".into()),
    }
}

fn parse_add(rest: &str) -> ExCommand {
    let tokens = tokenize(rest);
    if tokens.is_empty() {
        return ExCommand::Add(None);
    }
    let uri = tokens[0].clone();
    let mut tags = Vec::new();
    for token in &tokens[1..] {
        for part in token.split(',') {
            let tag = part.trim().to_lowercase();
            if !tag.is_empty() && !tags.contains(&tag) {
                tags.push(tag);
            }
        }
    }
    ExCommand::Add(Some(AddArgs { uri, tags }))
}

fn tokenize(rest: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut quoted = false;
    for c in rest.chars() {
        match c {
            '"' => quoted = !quoted,
            c if c.is_whitespace() && !quoted => {
                if !cur.is_empty() {
                    out.push(std::mem::take(&mut cur));
                }
            }
            c => cur.push(c),
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(line: &str) -> ExCommand {
        parse_ex_command(line)
    }

    #[test]
    fn empty_lines_are_empty() {
        assert_eq!(parse(""), ExCommand::Empty);
        assert_eq!(parse("   "), ExCommand::Empty);
    }

    #[test]
    fn quit_full_name_alias_and_bang() {
        assert_eq!(parse("q"), ExCommand::Quit);
        assert_eq!(parse(" quit "), ExCommand::Quit);
        assert_eq!(parse("q!"), ExCommand::Quit);
        assert_eq!(parse("qu"), ExCommand::Quit);
    }

    #[test]
    fn checkhealth_still_parses() {
        assert_eq!(parse("checkhealth"), ExCommand::CheckHealth);
        assert_eq!(parse("che"), ExCommand::CheckHealth);
    }

    #[test]
    fn aliases_resolve_to_canonical_commands() {
        assert_eq!(parse("n"), ExCommand::Next);
        assert_eq!(parse("h"), ExCommand::Help(HelpTopic::Keys));
        assert_eq!(parse("vol"), ExCommand::Volume(VolumeArg::Show));
    }

    #[test]
    fn unique_prefixes_resolve() {
        assert_eq!(parse("nex"), ExCommand::Next);
        assert_eq!(parse("cle"), ExCommand::ClearTags);
        assert_eq!(parse("sea foo"), ExCommand::Search(Some("foo".into())));
        assert_eq!(parse("st"), ExCommand::Stop);
        assert_eq!(parse("set"), ExCommand::Settings);
        assert_eq!(parse("delet"), ExCommand::Delete { force: false });
    }

    #[test]
    fn ambiguous_prefixes_list_candidates() {
        match parse("t") {
            ExCommand::Ambiguous { typed, candidates } => {
                assert_eq!(typed, "t");
                assert_eq!(
                    candidates,
                    vec![
                        "tag".to_string(),
                        "tags".to_string(),
                        "theme".to_string(),
                        "toggle".to_string(),
                        "transparency".to_string()
                    ]
                );
            }
            other => panic!("expected ambiguous, got {other:?}"),
        }
        assert!(matches!(parse("pa"), ExCommand::Ambiguous { .. }));
    }

    #[test]
    fn unknown_words_report_the_word() {
        assert_eq!(parse("zzz"), ExCommand::Unknown("zzz".into()));
        assert_eq!(parse("plays"), ExCommand::Unknown("plays".into()));
    }

    #[test]
    fn play_takes_a_one_based_queue_number() {
        assert_eq!(parse("play"), ExCommand::Play(None));
        assert_eq!(parse("play 7"), ExCommand::Play(Some(7)));
        assert_eq!(
            parse("play 0"),
            ExCommand::Invalid("play: expected a positive queue number".into())
        );
        assert_eq!(
            parse("play x"),
            ExCommand::Invalid("play: expected a positive queue number".into())
        );
    }

    #[test]
    fn seek_parses_relative_and_clock_targets() {
        assert_eq!(parse("seek"), ExCommand::Seek(None));
        assert_eq!(
            parse("seek +5"),
            ExCommand::Seek(Some(SeekTarget::Relative(5.0)))
        );
        assert_eq!(
            parse("seek -2.5"),
            ExCommand::Seek(Some(SeekTarget::Relative(-2.5)))
        );
        assert_eq!(
            parse("seek 45"),
            ExCommand::Seek(Some(SeekTarget::Absolute(45.0)))
        );
        assert_eq!(
            parse("seek 1:30"),
            ExCommand::Seek(Some(SeekTarget::Absolute(90.0)))
        );
        assert_eq!(
            parse("seek 1:00:05"),
            ExCommand::Seek(Some(SeekTarget::Absolute(3605.0)))
        );
        assert_eq!(
            parse("seek abc"),
            ExCommand::Invalid("seek: expected [+N|-N|M:SS]".into())
        );
        assert_eq!(
            parse("seek 1:2:3:4"),
            ExCommand::Invalid("seek: expected [+N|-N|M:SS]".into())
        );
    }

    #[test]
    fn volume_parses_delta_set_and_show() {
        assert_eq!(parse("volume"), ExCommand::Volume(VolumeArg::Show));
        assert_eq!(parse("volume +5"), ExCommand::Volume(VolumeArg::Delta(5)));
        assert_eq!(parse("vol -5"), ExCommand::Volume(VolumeArg::Delta(-5)));
        assert_eq!(parse("volume 80"), ExCommand::Volume(VolumeArg::Set(80)));
        assert_eq!(
            parse("volume x"),
            ExCommand::Invalid("volume: expected [+N|-N|N]".into())
        );
    }

    #[test]
    fn repeat_shuffle_theme_transparency_take_words() {
        assert_eq!(parse("repeat"), ExCommand::Repeat(None));
        assert_eq!(
            parse("repeat one"),
            ExCommand::Repeat(Some(RepeatMode::One))
        );
        assert_eq!(
            parse("repeat all"),
            ExCommand::Repeat(Some(RepeatMode::All))
        );
        assert_eq!(
            parse("repeat banana"),
            ExCommand::Invalid("repeat: expected off, all or one".into())
        );
        assert_eq!(parse("shuffle"), ExCommand::Shuffle(None));
        assert_eq!(parse("shuffle on"), ExCommand::Shuffle(Some(true)));
        assert_eq!(parse("shuffle off"), ExCommand::Shuffle(Some(false)));
        assert_eq!(parse("theme"), ExCommand::Theme(None));
        assert_eq!(parse("theme light"), ExCommand::Theme(Some(Theme::Light)));
        assert_eq!(parse("transparency"), ExCommand::Transparency(None));
        assert_eq!(
            parse("transparency on"),
            ExCommand::Transparency(Some(true))
        );
        assert_eq!(
            parse("transparency maybe"),
            ExCommand::Invalid("transparency: expected on or off".into())
        );
    }

    #[test]
    fn tag_search_and_noh() {
        assert_eq!(parse("tag Rock"), ExCommand::Tag("rock".into()));
        assert_eq!(
            parse("tag"),
            ExCommand::Invalid("tag: expected a tag name".into())
        );
        assert_eq!(
            parse("search foo bar"),
            ExCommand::Search(Some("foo bar".into()))
        );
        assert_eq!(parse("search"), ExCommand::Search(None));
        assert_eq!(parse("noh"), ExCommand::Search(None));
        assert_eq!(parse("noh ignored"), ExCommand::Search(None));
    }

    #[test]
    fn add_supports_quoted_uris_and_tag_lists() {
        assert_eq!(parse("add"), ExCommand::Add(None));
        assert_eq!(
            parse("add /a.mp3 rock live"),
            ExCommand::Add(Some(AddArgs {
                uri: "/a.mp3".into(),
                tags: vec!["rock".into(), "live".into()]
            }))
        );
        assert_eq!(
            parse(r#"add "my song.mp3" rock, jazz"#),
            ExCommand::Add(Some(AddArgs {
                uri: "my song.mp3".into(),
                tags: vec!["rock".into(), "jazz".into()]
            }))
        );
        assert_eq!(
            parse("add /a.mp3 rock rock"),
            ExCommand::Add(Some(AddArgs {
                uri: "/a.mp3".into(),
                tags: vec!["rock".into()]
            }))
        );
    }

    #[test]
    fn mini_pane_up_down_parse() {
        assert_eq!(parse("mini"), ExCommand::Mini(MiniSub::Add));
        assert_eq!(parse("mini rm"), ExCommand::Mini(MiniSub::Remove));
        assert_eq!(parse("mini up"), ExCommand::Mini(MiniSub::Up));
        assert_eq!(parse("mini dn"), ExCommand::Mini(MiniSub::Down));
        assert_eq!(
            parse("mini x"),
            ExCommand::Invalid("mini: expected add, remove, up or down".into())
        );
        assert_eq!(parse("pane"), ExCommand::Pane(PaneTarget::Next));
        assert_eq!(parse("pane info"), ExCommand::Pane(PaneTarget::Info));
        assert_eq!(parse("pane prev"), ExCommand::Pane(PaneTarget::Prev));
        assert_eq!(
            parse("pane nope"),
            ExCommand::Invalid("pane: expected next, prev, filter, mini, queue or info".into())
        );
        assert_eq!(parse("up"), ExCommand::Up);
        assert_eq!(parse("down"), ExCommand::Down);
    }

    #[test]
    fn delete_takes_a_bang_and_help_takes_topics() {
        assert_eq!(parse("delete"), ExCommand::Delete { force: false });
        assert_eq!(parse("delete!"), ExCommand::Delete { force: true });
        assert_eq!(parse("help"), ExCommand::Help(HelpTopic::Keys));
        assert_eq!(parse("help commands"), ExCommand::Help(HelpTopic::Commands));
        assert_eq!(parse("help cmd"), ExCommand::Help(HelpTopic::Commands));
        assert_eq!(parse("help keys"), ExCommand::Help(HelpTopic::Keys));
        assert_eq!(
            parse("help seek"),
            ExCommand::Help(HelpTopic::Command("seek".into()))
        );
    }

    #[test]
    fn simple_commands_ignore_trailing_words() {
        assert_eq!(parse("stop now"), ExCommand::Stop);
        assert_eq!(parse("detach please"), ExCommand::Detach);
        assert_eq!(parse("settings"), ExCommand::Settings);
        assert_eq!(parse("favorite"), ExCommand::Favorite);
        assert_eq!(parse("edit"), ExCommand::Edit);
        assert_eq!(parse("toggle"), ExCommand::Toggle);
        assert_eq!(parse("pause"), ExCommand::Pause);
        assert_eq!(parse("resume"), ExCommand::Resume);
        assert_eq!(parse("prev"), ExCommand::Prev);
        assert_eq!(parse("tags"), ExCommand::Tags);
    }
}
