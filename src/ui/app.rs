use super::Connection;
use crate::commands::builtins;
use crate::commands::{
    ExCommand, HelpTopic, MiniSub, PaneTarget, SeekTarget, VolumeArg, parse_ex_command,
};
use crate::config::Config;
use crate::health::{self, HealthEntry};
use crate::proto::{Command, ConfigState, Snapshot, Theme};
use crate::ui::dialog::{self, Dialog, DialogOutcome, FormCommand};
use crate::ui::input::{Action, Keymap, LineInput, LineOutcome};
use crate::ui::panes::{Section, scroll_offset};
use crate::ui::theme::{SettingsState, cursor_next, settings_options};
use crossterm::event::{
    Event, KeyCode, KeyEvent, MouseButton, MouseEvent, MouseEventKind, poll, read,
};
use ratatui::layout::{Position, Rect};
use std::io::{self};
use std::thread;
use std::time::{Duration, Instant};

pub struct App {
    pub(super) connection: Connection,
    pub(super) cfg: Config,
    pub(super) last_config: Option<ConfigState>,
    pub(super) keymap: Keymap,
    pub(super) snap: Option<Snapshot>,
    pub(super) section: Section,
    pub(super) prev_section: Section,
    pub(super) queue_cursor: usize,
    pub(super) tag_cursor: usize,
    pub(super) mini_cursor: usize,
    pub(super) search: Option<LineInput>,
    pub(super) command: Option<LineInput>,
    pub(super) health: Option<Vec<HealthEntry>>,
    pub(super) dialog: Option<Dialog>,
    pub(super) settings: Option<SettingsState>,
    pub(super) quit: bool,
    pub(super) shutdown: bool,
    pub(super) msg: Option<(String, Instant)>,
    pub(super) loading_since: Option<Instant>,
    pub(super) error_popup: Option<String>,
    pub(super) help_open: bool,
    pub(super) help_commands: bool,
    pub(super) complete: Option<Complete>,
    pub(super) queue_area: Rect,
    pub(super) filter_area: Rect,
    pub(super) mini_area: Rect,
    pub(super) state_area: Rect,
    pub(super) status_area: Rect,
    pub(super) status_rep: Option<Rect>,
    pub(super) status_shf: Option<Rect>,
    pub(super) last_click: Option<(Position, Instant)>,
}

impl App {
    fn new(connection: Connection) -> Self {
        let cfg = Config::default();
        let keymap = Keymap::build(&cfg);
        App {
            connection,
            cfg,
            last_config: None,
            keymap,
            snap: None,
            section: Section::Queue,
            prev_section: Section::Queue,
            queue_cursor: 0,
            tag_cursor: 0,
            mini_cursor: 0,
            search: None,
            command: None,
            health: None,
            dialog: None,
            settings: None,
            quit: false,
            shutdown: false,
            msg: None,
            loading_since: None,
            error_popup: None,
            help_open: false,
            help_commands: false,
            complete: None,
            queue_area: Rect::default(),
            filter_area: Rect::default(),
            mini_area: Rect::default(),
            state_area: Rect::default(),
            status_area: Rect::default(),
            status_rep: None,
            status_shf: None,
            last_click: None,
        }
    }

    fn pull(&mut self) -> bool {
        let Some(snap) = self.connection.pull() else {
            return false;
        };
        if self.last_config.as_ref() != Some(&snap.config) {
            let cfg = Config {
                mpv_binary: snap.config.mpv_binary.clone(),
                volume_step: snap.config.volume_step,
                seek_step: snap.config.seek_step,
                titles: snap.config.titles.clone(),
                keybindings: snap.config.map.clone(),
                ..Config::default()
            };
            self.keymap = Keymap::build(&cfg);
            self.cfg = cfg;
            self.last_config = Some(snap.config.clone());
        }
        if let Some(n) = &snap.notify {
            self.msg = Some((n.clone(), Instant::now()));
        }
        if let Some(err) = &snap.error
            && self.error_popup.is_none()
        {
            self.error_popup = Some(err.clone());
        }
        let now_loading = snap.now.as_ref().is_some_and(|n| n.loading);
        match (now_loading, self.loading_since) {
            (true, None) => self.loading_since = Some(Instant::now()),
            (false, Some(_)) => self.loading_since = None,
            _ => {}
        }
        if let Some(sel) = snap.selected.clone()
            && let Some(i) = snap.queue.iter().position(|id| *id == sel.id)
        {
            self.queue_cursor = i;
        }
        self.queue_cursor = self.queue_cursor.min(snap.queue.len().saturating_sub(1));
        self.tag_cursor = self.tag_cursor.min(snap.tags.len().saturating_sub(1));
        self.mini_cursor = self
            .mini_cursor
            .min(snap.mini_queue.len().saturating_sub(1));
        self.snap = Some(snap);
        true
    }

    fn on_key(&mut self, key: KeyEvent) {
        if self.error_popup.is_some() {
            match key.code {
                KeyCode::Esc | KeyCode::Enter => self.error_popup = None,
                _ => {}
            }
            return;
        }
        if self.health.is_some() {
            if matches!(key.code, KeyCode::Esc | KeyCode::Enter) {
                self.health = None;
            }
            return;
        }
        if self.help_open {
            let close = key.code == KeyCode::Esc || self.keymap.resolve(key) == Some(Action::Help);
            if close {
                self.help_open = false;
                self.help_commands = false;
            }
            return;
        }
        if let Some(st) = self.settings.as_mut() {
            match key.code {
                KeyCode::Esc => self.settings = None,
                KeyCode::Up => st.move_cursor(-1),
                KeyCode::Down => st.move_cursor(1),
                KeyCode::Left | KeyCode::Right => {
                    self.cycle_setting(if key.code == KeyCode::Left { -1 } else { 1 });
                }
                KeyCode::Enter => self.cycle_setting(1),
                _ => {
                    if let Some(action) = self.keymap.resolve(key) {
                        match action {
                            Action::MoveUp => st.move_cursor(-1),
                            Action::MoveDown => st.move_cursor(1),
                            Action::Activate | Action::Toggle => self.cycle_setting(1),
                            Action::ConfirmQuit | Action::Detach => self.settings = None,
                            _ => {}
                        }
                    }
                }
            }
            return;
        }
        if let Some(dlg) = self.dialog.as_mut() {
            let known: Vec<String> = self
                .snap
                .as_ref()
                .map(|s| s.tags.iter().map(|t| t.name.clone()).collect())
                .unwrap_or_default();
            match dlg.handle_key(key, &known) {
                DialogOutcome::Submit(cmd) => {
                    self.apply_form(cmd);
                    self.dialog = None;
                }
                DialogOutcome::ConfirmExitYes => {
                    self.shutdown = true;
                    self.quit = true;
                    self.dialog = None;
                }
                DialogOutcome::ConfirmDeleteYes => {
                    if let Some(sel) = self.snap.as_ref().and_then(|s| s.selected.clone()) {
                        self.connection.send(Command::Delete { id: sel.id });
                    }
                    self.dialog = None;
                }
                DialogOutcome::Cancel => {
                    self.dialog = None;
                }
                DialogOutcome::None => {}
            }
            return;
        }
        if let Some(s) = self.search.as_mut() {
            let before = s.text.clone();
            match s.handle_key(key) {
                LineOutcome::Done => self.search = None,
                LineOutcome::Cancel => {
                    self.connection.send(Command::SetSearch { pattern: None });
                    self.search = None;
                }
                LineOutcome::None => {}
            }
            if let Some(s) = self.search.as_mut()
                && s.text != before
            {
                self.connection.send(Command::SetSearch {
                    pattern: s.pattern(),
                });
            }
            return;
        }
        if self.command.is_some() {
            match key.code {
                KeyCode::Tab | KeyCode::Down => self.complete_command(1),
                KeyCode::BackTab | KeyCode::Up => self.complete_command(-1),
                KeyCode::Backspace if self.command.as_ref().is_some_and(|c| c.text.is_empty()) => {
                    self.command = None;
                    self.complete = None;
                }
                _ => {
                    let before = self.command.as_ref().map(|c| c.text.clone());
                    let outcome = self.command.as_mut().map(|c| c.handle_key(key));
                    match outcome {
                        Some(LineOutcome::Done) => {
                            self.complete = None;
                            let line = self.command.take().map(|c| c.text).unwrap_or_default();
                            self.run_command(&line);
                        }
                        Some(LineOutcome::Cancel) => {
                            self.command = None;
                            self.complete = None;
                        }
                        Some(LineOutcome::None) | None => {}
                    }
                    let after = self.command.as_ref().map(|c| c.text.clone());
                    if before != after {
                        self.refresh_complete();
                    }
                }
            }
            return;
        }
        match self.keymap.resolve(key) {
            Some(Action::Command) => self.command = Some(LineInput::new()),
            Some(Action::Settings) => {
                self.settings = Some(SettingsState::new());
            }
            Some(Action::Help) => {
                self.help_open = true;
                self.help_commands = false;
            }
            Some(Action::ConfirmQuit) => self.dialog = Some(Dialog::ConfirmExit),
            Some(Action::Detach) => {
                self.quit = true;
                self.shutdown = false;
            }
            Some(Action::AddMedia) => self.dialog = Some(Dialog::Add(dialog::Form::new_add())),
            Some(Action::EditMedia) => {
                if let Some(sel) = self.snap.as_ref().and_then(|s| s.selected.clone()) {
                    self.dialog = Some(Dialog::Edit(dialog::Form::new_edit(&sel)));
                }
            }
            Some(Action::Search) => self.search = Some(LineInput::new()),
            Some(Action::AddMini) => {
                if let Some(sel) = self.snap.as_ref().and_then(|s| s.selected.clone()) {
                    self.connection.send(Command::AddMini { id: sel.id });
                }
            }
            Some(Action::FocusMini) => {
                if self.section == Section::Mini {
                    self.section = self.prev_section;
                } else {
                    self.prev_section = self.section;
                    self.section = Section::Mini;
                }
            }
            Some(Action::Delete) => match self.section {
                Section::Mini => {
                    if let Some(id) = self
                        .snap
                        .as_ref()
                        .and_then(|s| s.mini_queue.get(self.mini_cursor))
                        .copied()
                    {
                        self.connection.send(Command::RemoveMini { id });
                    }
                }
                _ => self.dialog = Some(Dialog::ConfirmDelete),
            },
            Some(Action::MiniMoveUp) => self.mini_move(-1),
            Some(Action::MiniMoveDown) => self.mini_move(1),
            Some(Action::MoveUp) => self.move_cursor(-1),
            Some(Action::MoveDown) => self.move_cursor(1),
            Some(Action::PrevSection) => self.section = self.section.prev(),
            Some(Action::NextSection) | Some(Action::CycleFocus) => {
                self.section = self.section.next()
            }
            Some(Action::CycleFocusBack) => self.section = self.section.prev(),
            Some(Action::Activate) => match self.section {
                Section::Queue => self.play_cursor(),
                Section::Filter => self.toggle_tag_cursor(),
                Section::Mini => {}
                Section::Info => {}
            },
            Some(Action::Toggle) => match self.section {
                Section::Queue => self.connection.send(Command::PlayPause),
                Section::Filter => self.toggle_tag_cursor(),
                Section::Mini => {}
                Section::Info => {}
            },
            Some(Action::NextTrack) => self.connection.send(Command::Next),
            Some(Action::PrevTrack) => self.connection.send(Command::Prev),
            Some(Action::VolumeUp) => self.connection.send(Command::Volume {
                delta: self.cfg.volume_step,
            }),
            Some(Action::VolumeDown) => self.connection.send(Command::Volume {
                delta: -self.cfg.volume_step,
            }),
            Some(Action::SeekFwd) => self.connection.send(Command::Seek {
                delta: self.cfg.seek_step,
            }),
            Some(Action::SeekBack) => self.connection.send(Command::Seek {
                delta: -self.cfg.seek_step,
            }),
            Some(Action::Repeat) => self.connection.send(Command::RepeatCycle),
            Some(Action::Shuffle) => self.connection.send(Command::ShuffleToggle),
            Some(Action::Favorite) => {
                if let Some(sel) = self.snap.as_ref().and_then(|s| s.selected.clone()) {
                    self.connection.send(Command::ToggleFavorite { id: sel.id });
                }
            }
            None => {}
        }
    }

    fn on_mouse(&mut self, m: MouseEvent) {
        if self.dialog.is_some()
            || self.search.is_some()
            || self.command.is_some()
            || self.health.is_some()
            || self.help_open
        {
            return;
        }
        let MouseEvent {
            kind, column, row, ..
        } = m;
        if let MouseEventKind::Down(MouseButton::Left) = kind {
            let pos = ratatui::layout::Position::new(column, row);
            let now = Instant::now();
            let double = matches!(self.last_click, Some((p, t)) if p.y == pos.y && now.duration_since(t) < Duration::from_millis(400));
            if double {
                self.last_click = None;
            } else {
                self.last_click = Some((pos, now));
            }
            if self.queue_area.contains(pos) {
                self.section = Section::Queue;
                let rel = row.saturating_sub(self.queue_area.y + 1);
                let len = self.snap.as_ref().map(|s| s.queue.len()).unwrap_or(0);
                let height = self.queue_area.height.saturating_sub(2) as usize;
                let start = scroll_offset(len, self.queue_cursor, height);
                let idx = start + usize::from(rel);
                if usize::from(rel) < height && idx < len {
                    self.queue_cursor = idx;
                    if let Some(id) = self.snap.as_ref().and_then(|s| s.queue.get(idx)).copied() {
                        if double {
                            self.connection.send(Command::Play { id });
                        } else {
                            self.connection.send(Command::Select { id });
                        }
                    }
                }
            } else if self.mini_area.contains(pos) {
                self.section = Section::Mini;
                let rel = row.saturating_sub(self.mini_area.y + 1);
                let len = self.snap.as_ref().map(|s| s.mini_queue.len()).unwrap_or(0);
                if usize::from(rel) < len {
                    self.mini_cursor = usize::from(rel);
                    if double
                        && let Some(id) = self
                            .snap
                            .as_ref()
                            .and_then(|s| s.mini_queue.get(self.mini_cursor))
                            .copied()
                    {
                        self.connection.send(Command::Play { id });
                    }
                }
            } else if self.filter_area.contains(pos) {
                self.section = Section::Filter;
                let rel = row.saturating_sub(self.filter_area.y + 1);
                if let Some(t) = self
                    .snap
                    .as_ref()
                    .and_then(|s| s.tags.get(usize::from(rel)))
                {
                    let name = t.name.clone();
                    self.connection.send(Command::ToggleTag { tag: name });
                }
            } else if self.status_area.contains(pos) {
                if self.status_rep.is_some_and(|r| r.contains(pos)) {
                    self.connection.send(Command::RepeatCycle);
                } else if self.status_shf.is_some_and(|r| r.contains(pos)) {
                    self.connection.send(Command::ShuffleToggle);
                }
            } else if self.state_area.contains(pos) {
                self.section = Section::Info;
            }
        }
    }

    fn cycle_setting(&mut self, delta: i64) {
        let Some(st) = self.settings.as_ref() else {
            return;
        };
        let Some(snap) = self.snap.as_ref() else {
            return;
        };
        let options = settings_options();
        let cursor = st.cursor;
        match cursor {
            0 => {
                let cur = options[0]
                    .1
                    .iter()
                    .position(|v| *v == snap.theme.name())
                    .unwrap_or(0);
                let next = cursor_next(delta, cur, options[0].1.len());
                if let Some(t) = Theme::parse(options[0].1[next]) {
                    self.connection.send(Command::SetTheme { theme: t });
                }
            }
            1 => {
                let next = delta > 0;
                self.connection.send(Command::SetTransparency { on: next });
            }
            _ => {}
        }
    }

    fn apply_form(&mut self, cmd: FormCommand) {
        match cmd {
            FormCommand::Add { uri, title, tags } => {
                self.connection.send(Command::Add {
                    uri,
                    title: if title.is_empty() { None } else { Some(title) },
                    tags,
                });
            }
            FormCommand::Update {
                id,
                title,
                artist,
                tags,
            } => {
                self.connection.send(Command::Update {
                    id,
                    title: if title.is_empty() { None } else { Some(title) },
                    artist: if artist.is_empty() {
                        None
                    } else {
                        Some(artist)
                    },
                    tags,
                });
            }
        }
    }

    fn move_cursor(&mut self, delta: i64) {
        match self.section {
            Section::Queue => {
                let len = self.snap.as_ref().map(|s| s.queue.len()).unwrap_or(0);
                if len == 0 {
                    return;
                }
                let new = (self.queue_cursor as i64 + delta).clamp(0, len as i64 - 1) as usize;
                if new != self.queue_cursor {
                    self.queue_cursor = new;
                    if let Some(id) = self.snap.as_ref().and_then(|s| s.queue.get(new)).copied() {
                        self.connection.send(Command::Select { id });
                    }
                }
            }
            Section::Filter => {
                let len = self.snap.as_ref().map(|s| s.tags.len()).unwrap_or(0);
                if len == 0 {
                    return;
                }
                self.tag_cursor =
                    (self.tag_cursor as i64 + delta).clamp(0, len as i64 - 1) as usize;
            }
            Section::Mini => {
                let len = self.snap.as_ref().map(|s| s.mini_queue.len()).unwrap_or(0);
                if len == 0 {
                    return;
                }
                self.mini_cursor =
                    (self.mini_cursor as i64 + delta).clamp(0, len as i64 - 1) as usize;
            }
            Section::Info => {}
        }
    }

    fn mini_move(&mut self, delta: i64) {
        let Some(id) = self
            .snap
            .as_ref()
            .and_then(|s| s.mini_queue.get(self.mini_cursor))
            .copied()
        else {
            return;
        };
        self.connection.send(Command::MiniMove { id, delta });
    }

    fn play_cursor(&mut self) {
        if let Some(id) = self
            .snap
            .as_ref()
            .and_then(|s| s.queue.get(self.queue_cursor))
            .copied()
        {
            self.connection.send(Command::Play { id });
        }
    }

    fn toggle_tag_cursor(&mut self) {
        if let Some(t) = self.snap.as_ref().and_then(|s| s.tags.get(self.tag_cursor)) {
            let name = t.name.clone();
            self.connection.send(Command::ToggleTag { tag: name });
        }
    }

    fn run_command(&mut self, line: &str) {
        match parse_ex_command(line) {
            ExCommand::Empty => {}
            ExCommand::Quit => {
                self.shutdown = true;
                self.quit = true;
            }
            ExCommand::CheckHealth => {
                self.health = Some(health::report(&self.cfg.mpv_binary));
            }
            ExCommand::Help(topic) => self.run_help(topic),
            ExCommand::Settings => self.settings = Some(SettingsState::new()),
            ExCommand::Detach => {
                self.quit = true;
                self.shutdown = false;
            }
            ExCommand::Play(n) => self.ex_play(n),
            ExCommand::Pause => self.ex_pause(false),
            ExCommand::Resume => self.ex_pause(true),
            ExCommand::Toggle => self.connection.send(Command::PlayPause),
            ExCommand::Stop => self.connection.send(Command::Stop),
            ExCommand::Next => self.connection.send(Command::Next),
            ExCommand::Prev => self.connection.send(Command::Prev),
            ExCommand::Seek(target) => self.ex_seek(target),
            ExCommand::Volume(arg) => self.ex_volume(arg),
            ExCommand::Repeat(mode) => match mode {
                Some(mode) => self.connection.send(Command::SetRepeat { mode }),
                None => self.connection.send(Command::RepeatCycle),
            },
            ExCommand::Shuffle(on) => match on {
                Some(on) => self.connection.send(Command::SetShuffle { on }),
                None => self.connection.send(Command::ShuffleToggle),
            },
            ExCommand::Tag(name) => self.ex_tag(&name),
            ExCommand::Tags => {
                let line = self.tags_line();
                self.notify(line);
            }
            ExCommand::ClearTags => self.connection.send(Command::ClearTags),
            ExCommand::Search(pattern) => self.ex_search(pattern),
            ExCommand::Add(args) => match args {
                None => self.dialog = Some(Dialog::Add(dialog::Form::new_add())),
                Some(args) => self.connection.send(Command::Add {
                    uri: args.uri,
                    title: None,
                    tags: args.tags,
                }),
            },
            ExCommand::Edit => {
                let sel = self.snap.as_ref().and_then(|s| s.selected.clone());
                match sel {
                    Some(sel) => self.dialog = Some(Dialog::Edit(dialog::Form::new_edit(&sel))),
                    None => self.notify("nothing selected".into()),
                }
            }
            ExCommand::Delete { force } => {
                let sel = self
                    .snap
                    .as_ref()
                    .and_then(|s| s.selected.as_ref().map(|m| m.id));
                match (force, sel) {
                    (true, Some(id)) => self.connection.send(Command::Delete { id }),
                    (true, None) => self.notify("nothing selected".into()),
                    (false, _) => self.dialog = Some(Dialog::ConfirmDelete),
                }
            }
            ExCommand::Favorite => {
                let sel = self
                    .snap
                    .as_ref()
                    .and_then(|s| s.selected.as_ref().map(|m| m.id));
                match sel {
                    Some(id) => self.connection.send(Command::ToggleFavorite { id }),
                    None => self.notify("nothing selected".into()),
                }
            }
            ExCommand::Mini(sub) => self.ex_mini(sub),
            ExCommand::Pane(target) => self.ex_pane(target),
            ExCommand::Up => self.move_cursor(-1),
            ExCommand::Down => self.move_cursor(1),
            ExCommand::Theme(choice) => {
                let theme = match choice {
                    Some(theme) => theme,
                    None => match self.snap.as_ref().map(|s| s.theme) {
                        Some(Theme::Dark) => Theme::Light,
                        _ => Theme::Dark,
                    },
                };
                self.connection.send(Command::SetTheme { theme });
            }
            ExCommand::Transparency(on) => {
                let on = match on {
                    Some(on) => on,
                    None => !self.snap.as_ref().map(|s| s.transparent).unwrap_or(false),
                };
                self.connection.send(Command::SetTransparency { on });
            }
            ExCommand::Invalid(msg) => self.notify(msg),
            ExCommand::Ambiguous { typed, candidates } => {
                let msg = format!("ambiguous command: \"{typed}\" ({})", candidates.join(", "));
                self.notify(msg);
            }
            ExCommand::Unknown(cmd) => {
                let msg = format!(
                    "unknown command: {cmd} (try {})",
                    builtins::known_commands_hint()
                );
                self.notify(msg);
            }
        }
    }

    fn notify(&mut self, msg: String) {
        self.msg = Some((msg, Instant::now()));
    }

    fn run_help(&mut self, topic: HelpTopic) {
        match topic {
            HelpTopic::Keys => {
                self.help_open = true;
                self.help_commands = false;
            }
            HelpTopic::Commands => {
                self.help_open = true;
                self.help_commands = true;
            }
            HelpTopic::Command(name) => {
                let msg = match builtins::find(&name) {
                    Some(b) => format!("{} - {}", b.usage, b.desc),
                    None => format!("no such command: {name} (try :help commands)"),
                };
                self.notify(msg);
            }
        }
    }

    fn ex_play(&mut self, n: Option<usize>) {
        let Some(n) = n else {
            self.play_cursor();
            return;
        };
        let hit = self
            .snap
            .as_ref()
            .and_then(|s| s.queue.get(n.saturating_sub(1)).copied());
        match hit {
            Some(id) => self.connection.send(Command::Play { id }),
            None => {
                if self.snap.is_some() {
                    self.notify(format!("no queue item {n}"));
                } else {
                    self.notify("not connected to the daemon".into());
                }
            }
        }
    }

    fn ex_pause(&mut self, resume: bool) {
        let paused = self
            .snap
            .as_ref()
            .and_then(|s| s.now.as_ref().map(|n| n.paused));
        match (resume, paused) {
            (true, Some(true)) | (false, Some(false)) => self.connection.send(Command::PlayPause),
            (true, _) => self.notify("nothing paused".into()),
            (false, _) => self.notify("nothing playing".into()),
        }
    }

    fn ex_seek(&mut self, target: Option<SeekTarget>) {
        let now = self.snap.as_ref().and_then(|s| s.now.clone());
        if now.is_none() && self.snap.is_none() {
            self.notify("not connected to the daemon".into());
            return;
        }
        let Some(target) = target else {
            let msg = match &now {
                Some(n) => {
                    let dur = n
                        .duration
                        .map(ex_fmt_clock)
                        .unwrap_or_else(|| "--:--".into());
                    format!("position {} / {}", ex_fmt_clock(n.position), dur)
                }
                None => "nothing playing".into(),
            };
            self.notify(msg);
            return;
        };
        match target {
            SeekTarget::Relative(delta) => self.connection.send(Command::Seek { delta }),
            SeekTarget::Absolute(t) => match now {
                Some(n) => {
                    let delta = (t - n.position).max(-n.position);
                    self.connection.send(Command::Seek { delta });
                }
                None => self.notify("nothing playing".into()),
            },
        }
    }

    fn ex_volume(&mut self, arg: VolumeArg) {
        match arg {
            VolumeArg::Delta(delta) => self.connection.send(Command::Volume { delta }),
            VolumeArg::Set(volume) => self.connection.send(Command::SetVolume { volume }),
            VolumeArg::Show => {
                let vol = self.snap.as_ref().map(|s| s.volume);
                match vol {
                    Some(v) => self.notify(format!("volume: {v}")),
                    None => self.notify("not connected to the daemon".into()),
                }
            }
        }
    }

    fn ex_tag(&mut self, name: &str) {
        let known = self
            .snap
            .as_ref()
            .map(|s| s.tags.iter().any(|t| t.name == name));
        match known {
            Some(true) => self.connection.send(Command::ToggleTag {
                tag: name.to_string(),
            }),
            Some(false) => self.notify(format!("no such tag: {name}")),
            None => self.notify("not connected to the daemon".into()),
        }
    }

    fn tags_line(&self) -> String {
        let Some(snap) = self.snap.as_ref() else {
            return "not connected to the daemon".into();
        };
        if snap.tags.is_empty() {
            return "no tags".into();
        }
        let parts: Vec<String> = snap
            .tags
            .iter()
            .map(|t| {
                if t.checked {
                    format!("{}({})*", t.name, t.count)
                } else {
                    format!("{}({})", t.name, t.count)
                }
            })
            .collect();
        format!("tags: {}", parts.join(" "))
    }

    fn ex_search(&mut self, pattern: Option<String>) {
        if let Some(p) = &pattern
            && regex::Regex::new(p).is_err()
        {
            self.notify(format!("search: invalid regex: {p}"));
            return;
        }
        self.connection.send(Command::SetSearch { pattern });
    }

    fn ex_mini(&mut self, sub: MiniSub) {
        let sel = self.snap.as_ref().and_then(|s| s.selected.clone());
        let Some(sel) = sel else {
            self.notify("nothing selected".into());
            return;
        };
        let id = sel.id;
        match sub {
            MiniSub::Add => self.connection.send(Command::AddMini { id }),
            MiniSub::Remove => self.connection.send(Command::RemoveMini { id }),
            MiniSub::Up => self.connection.send(Command::MiniMove { id, delta: -1 }),
            MiniSub::Down => self.connection.send(Command::MiniMove { id, delta: 1 }),
        }
    }

    fn ex_pane(&mut self, target: PaneTarget) {
        match target {
            PaneTarget::Next => self.section = self.section.next(),
            PaneTarget::Prev => self.section = self.section.prev(),
            PaneTarget::Filter => self.section = Section::Filter,
            PaneTarget::Mini => {
                self.prev_section = self.section;
                self.section = Section::Mini;
            }
            PaneTarget::Queue => self.section = Section::Queue,
            PaneTarget::Info => self.section = Section::Info,
        }
    }

    fn known_tag_names(&self) -> Vec<String> {
        self.snap
            .as_ref()
            .map(|s| s.tags.iter().map(|t| t.name.clone()).collect())
            .unwrap_or_default()
    }

    fn complete_command(&mut self, delta: i64) {
        let Some(cmd) = self.command.as_ref() else {
            return;
        };
        let text = cmd.text.clone();
        let tags = self.known_tag_names();
        let fresh = completion_for_line(&text, &tags);
        if let (Some(prev), Some(c)) = (self.complete.as_ref(), fresh.as_ref()) {
            let token = text[c.token_start..].trim_end().to_lowercase();
            if c.token_start == prev.token_start
                && prev.sel < prev.candidates.len()
                && prev.candidates[prev.sel] == token
            {
                let candidates = prev.candidates.clone();
                let len = candidates.len();
                let sel = cycled_sel(Some(prev.sel), delta, len);
                let candidate = candidates[sel].clone();
                let with_space = prev.command_word && len == 1;
                let token_start = prev.token_start;
                let command_word = prev.command_word;
                self.write_completion(token_start, &candidate, with_space);
                self.complete = Some(Complete {
                    candidates,
                    sel,
                    token_start,
                    command_word,
                });
                return;
            }
        }
        let Some(c) = fresh else {
            self.complete = None;
            return;
        };
        if c.candidates.is_empty() {
            self.complete = None;
            return;
        }
        let len = c.candidates.len();
        let sel = cycled_sel(None, delta, len);
        let candidate = c.candidates[sel].clone();
        let with_space = c.command_word && len == 1;
        let token_start = c.token_start;
        let command_word = c.command_word;
        self.write_completion(token_start, &candidate, with_space);
        self.complete = Some(Complete {
            candidates: c.candidates,
            sel,
            token_start,
            command_word,
        });
    }

    fn refresh_complete(&mut self) {
        if self.complete.is_none() {
            return;
        }
        let Some(cmd) = self.command.as_ref() else {
            self.complete = None;
            return;
        };
        let text = cmd.text.clone();
        let tags = self.known_tag_names();
        let Some(c) = completion_for_line(&text, &tags) else {
            self.complete = None;
            return;
        };
        if c.candidates.is_empty() {
            self.complete = None;
            return;
        }
        let token = text[c.token_start..].trim_end().to_lowercase();
        let sel = c
            .candidates
            .iter()
            .position(|cand| *cand == token)
            .unwrap_or(0);
        self.complete = Some(Complete {
            candidates: c.candidates,
            sel,
            token_start: c.token_start,
            command_word: c.command_word,
        });
    }

    fn write_completion(&mut self, token_start: usize, candidate: &str, with_space: bool) {
        let Some(cmd) = self.command.as_mut() else {
            return;
        };
        let mut text = cmd.text[..token_start].to_string();
        text.push_str(candidate);
        if with_space {
            text.push(' ');
        }
        cmd.cursor = text.len();
        cmd.text = text;
    }
}

fn ex_fmt_clock(secs: f64) -> String {
    if !secs.is_finite() || secs < 0.0 {
        return "--:--".into();
    }
    let s = secs as u64;
    if s >= 3600 {
        format!("{}:{:02}:{:02}", s / 3600, (s % 3600) / 60, s % 60)
    } else {
        format!("{}:{:02}", s / 60, s % 60)
    }
}

pub(super) struct Complete {
    pub(super) candidates: Vec<String>,
    pub(super) sel: usize,
    token_start: usize,
    command_word: bool,
}

fn cycled_sel(current: Option<usize>, delta: i64, len: usize) -> usize {
    match current {
        Some(i) => ((i as i64 + delta).rem_euclid(len as i64)) as usize,
        None if delta > 0 => 0,
        None => len - 1,
    }
}

struct Completion {
    candidates: Vec<String>,
    token_start: usize,
    command_word: bool,
}

fn completion_for_line(line: &str, known_tags: &[String]) -> Option<Completion> {
    let trimmed = line.trim_start();
    let leading = line.len() - trimmed.len();
    let head_end = trimmed.find(char::is_whitespace).unwrap_or(trimmed.len());
    let head = &trimmed[..head_end];
    if head.is_empty() {
        return Some(Completion {
            candidates: builtin_names(""),
            token_start: leading,
            command_word: true,
        });
    }
    let after_head = leading + head_end;
    let tail = &line[after_head..];
    if tail.is_empty() {
        let prefix = head.to_lowercase();
        return Some(Completion {
            candidates: builtin_names(&prefix),
            token_start: leading,
            command_word: true,
        });
    }
    let token_start = after_head + (tail.len() - tail.trim_start().len());
    let canonical = builtins::resolve_canonical(&head.to_lowercase())?;
    let token = line[token_start..].trim_end().to_lowercase();
    let candidates = arg_candidates(canonical, known_tags)
        .into_iter()
        .filter(|w| w.starts_with(&token))
        .collect();
    Some(Completion {
        candidates,
        token_start,
        command_word: false,
    })
}

fn builtin_names(prefix: &str) -> Vec<String> {
    let mut names: Vec<String> = builtins::BUILTINS
        .iter()
        .map(|b| b.name.to_string())
        .filter(|n| n.starts_with(prefix))
        .collect();
    names.sort();
    names
}

fn arg_candidates(command: &str, known_tags: &[String]) -> Vec<String> {
    let mut words: Vec<String> = match command {
        "repeat" => vec!["off".into(), "all".into(), "one".into()],
        "shuffle" => vec!["on".into(), "off".into()],
        "theme" => vec!["dark".into(), "light".into()],
        "transparency" => vec!["on".into(), "off".into()],
        "mini" => vec!["add".into(), "remove".into(), "up".into(), "down".into()],
        "pane" => vec![
            "next".into(),
            "prev".into(),
            "filter".into(),
            "mini".into(),
            "queue".into(),
            "info".into(),
        ],
        "help" => vec!["keys".into(), "commands".into()],
        "tag" => known_tags.to_vec(),
        _ => Vec::new(),
    };
    if command == "help" {
        words.extend(builtin_names(""));
    }
    words.sort();
    words.dedup();
    words
}

pub fn run(connection: Connection) -> io::Result<()> {
    let mut terminal = ratatui::init();
    let res = (|| {
        crossterm::execute!(std::io::stdout(), crossterm::event::EnableMouseCapture)?;
        let mut app = App::new(connection);
        let boot = Instant::now();
        while app.snap.is_none() && boot.elapsed() < Duration::from_secs(3) {
            app.pull();
            if app.snap.is_none() {
                thread::sleep(Duration::from_millis(5));
            }
        }
        let start = Instant::now();
        let mut last_tick: Option<u64> = None;
        let mut redraw = true;
        while !app.quit {
            if app.pull() {
                redraw = true;
            }
            if app.loading_since.is_some() {
                redraw = true;
            }
            let wait = if app.loading_since.is_some() {
                Duration::from_millis(50)
            } else {
                Duration::from_millis(100)
            };
            if poll(wait)? {
                match read()? {
                    Event::Key(k) => {
                        app.on_key(k);
                        redraw = true;
                    }
                    Event::Mouse(m) => {
                        app.on_mouse(m);
                        redraw = true;
                    }
                    Event::Resize(..) => redraw = true,
                    _ => {}
                }
            }
            let tick = start.elapsed().as_secs();
            if last_tick != Some(tick) {
                last_tick = Some(tick);
                redraw = true;
            }
            if redraw {
                terminal.draw(|f| app.render(f))?;
                redraw = false;
            }
        }
        if app.shutdown {
            app.connection.send(Command::Shutdown);
            std::thread::sleep(Duration::from_millis(100));
        }
        let _ = crossterm::execute!(std::io::stdout(), crossterm::event::DisableMouseCapture);
        Ok::<(), io::Error>(())
    })();
    ratatui::restore();
    res
}

#[cfg(test)]
mod completion_tests {
    use super::*;

    fn complete(line: &str, tags: &[&str]) -> Option<Completion> {
        let owned: Vec<String> = tags.iter().map(|s| s.to_string()).collect();
        completion_for_line(line, &owned)
    }

    #[test]
    fn empty_line_lists_every_command_name() {
        let c = complete("", &[]).unwrap();
        assert_eq!(c.candidates, builtin_names(""));
        assert!(c.command_word);
        assert_eq!(c.token_start, 0);
    }

    #[test]
    fn command_word_filters_by_prefix_and_replaces_from_token_start() {
        let c = complete("rep", &[]).unwrap();
        assert_eq!(c.candidates, vec!["repeat".to_string()]);
        assert!(c.command_word);
        assert_eq!(c.token_start, 0);

        let c = complete("  rep", &[]).unwrap();
        assert_eq!(c.candidates, vec!["repeat".to_string()]);
        assert_eq!(c.token_start, 2);

        let c = complete("ne", &[]).unwrap();
        assert_eq!(c.candidates, vec!["next".to_string()]);
        let c = complete("n", &[]).unwrap();
        assert_eq!(c.candidates, vec!["next".to_string(), "noh".to_string()]);

        let c = complete("zzz", &[]).unwrap();
        assert!(c.candidates.is_empty());
    }

    #[test]
    fn trailing_space_switches_to_argument_completion() {
        let c = complete("repeat ", &[]).unwrap();
        assert_eq!(c.candidates, vec!["all", "off", "one"]);
        assert!(!c.command_word);
        assert_eq!(c.token_start, "repeat ".len());

        let c = complete("rep ", &[]).unwrap();
        assert_eq!(c.candidates, vec!["all", "off", "one"]);

        let c = complete("theme d", &[]).unwrap();
        assert_eq!(c.candidates, vec!["dark".to_string()]);

        let c = complete("pane ", &[]).unwrap();
        assert_eq!(
            c.candidates,
            vec![
                "filter".to_string(),
                "info".to_string(),
                "mini".to_string(),
                "next".to_string(),
                "prev".to_string(),
                "queue".to_string()
            ]
        );

        let c = complete("mini ", &[]).unwrap();
        assert_eq!(c.candidates, vec!["add", "down", "remove", "up"]);
    }

    #[test]
    fn tag_completion_uses_known_tag_names() {
        let c = complete("tag ", &["rock", "jazz"]).unwrap();
        assert_eq!(c.candidates, vec!["jazz".to_string(), "rock".to_string()]);
        let c = complete("tag ro", &["rock", "jazz"]).unwrap();
        assert_eq!(c.candidates, vec!["rock".to_string()]);
        let c = complete("tag ", &[]).unwrap();
        assert!(c.candidates.is_empty());
    }

    #[test]
    fn help_completes_topics_and_command_names() {
        let c = complete("help c", &[]).unwrap();
        assert!(c.candidates.contains(&"commands".to_string()));
        assert!(c.candidates.contains(&"checkhealth".to_string()));
        assert!(c.candidates.contains(&"clear_tags".to_string()));
    }

    #[test]
    fn free_form_commands_complete_nothing() {
        assert!(
            complete("add /a.mp3 ", &["rock"])
                .unwrap()
                .candidates
                .is_empty()
        );
        assert!(complete("search foo", &[]).unwrap().candidates.is_empty());
        assert!(complete("volume ", &[]).unwrap().candidates.is_empty());
    }

    #[test]
    fn ambiguous_head_completes_no_arguments() {
        assert!(complete("t ", &[]).is_none());
        assert!(complete("pa ", &[]).is_none());
    }

    #[test]
    fn unknown_head_yields_no_candidates() {
        assert!(complete("zzz", &[]).unwrap().candidates.is_empty());
        assert!(complete("zzz ", &[]).is_none());
    }

    #[test]
    fn fresh_completion_starts_at_first_or_last_candidate() {
        assert_eq!(cycled_sel(None, 1, 5), 0);
        assert_eq!(cycled_sel(None, -1, 5), 4);
        assert_eq!(cycled_sel(None, 1, 1), 0);
    }

    #[test]
    fn cycling_wraps_in_both_directions() {
        assert_eq!(cycled_sel(Some(4), 1, 5), 0);
        assert_eq!(cycled_sel(Some(0), -1, 5), 4);
        assert_eq!(cycled_sel(Some(2), 1, 5), 3);
        assert_eq!(cycled_sel(Some(2), -1, 5), 1);
        assert_eq!(cycled_sel(Some(0), 1, 1), 0);
    }
}

#[cfg(test)]
mod command_tests {
    use super::*;
    use crossterm::event::KeyModifiers;
    use std::sync::mpsc;

    fn app() -> App {
        let (cmd_tx, _cmd_rx) = mpsc::channel();
        let (_snap_tx, snap_rx) = mpsc::channel();
        App::new(Connection::from_channels(cmd_tx, snap_rx))
    }

    fn press(app: &mut App, code: KeyCode, mods: KeyModifiers) {
        app.on_key(KeyEvent::new(code, mods));
    }

    fn backspaces_down_to_empty_prompt(app: &mut App) {
        press(app, KeyCode::Char(':'), KeyModifiers::NONE);
        assert!(app.command.is_some());
        for c in "tags".chars() {
            press(app, KeyCode::Char(c), KeyModifiers::NONE);
        }
        for _ in 0..4 {
            press(app, KeyCode::Backspace, KeyModifiers::NONE);
            assert!(app.command.is_some());
        }
    }

    #[test]
    fn final_backspace_closes_emptied_command_line() {
        let mut app = app();
        backspaces_down_to_empty_prompt(&mut app);
        press(&mut app, KeyCode::Backspace, KeyModifiers::NONE);
        assert!(app.command.is_none());
        assert!(app.complete.is_none());
    }

    #[test]
    fn backspace_closes_fresh_empty_command_line() {
        let mut app = app();
        press(&mut app, KeyCode::Char(':'), KeyModifiers::NONE);
        press(&mut app, KeyCode::Backspace, KeyModifiers::NONE);
        assert!(app.command.is_none());
        assert!(app.complete.is_none());
    }
}
