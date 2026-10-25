use super::Connection;
use crate::commands::{ExCommand, parse_ex_command};
use crate::config::Config;
use crate::health::{self, HealthEntry};
use crate::proto::{Command, Snapshot, Theme};
use crate::ui::dialog::{self, Dialog, DialogOutcome, FormCommand};
use crate::ui::input::{Action, Keymap, LineInput, LineOutcome};
use crate::ui::panes::{Section, scroll_offset};
use crate::ui::theme::{SettingsState, cursor_next, settings_options};
use crossterm::event::{
    Event, KeyCode, KeyEvent, MouseButton, MouseEvent, MouseEventKind, poll, read,
};
use ratatui::layout::{Position, Rect};
use std::io::{self};
use std::time::{Duration, Instant};

pub struct App {
    pub(super) connection: Connection,
    pub(super) cfg: Config,
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
    fn new(connection: Connection, cfg: Config) -> Self {
        let keymap = Keymap::build(&cfg);
        App {
            connection,
            cfg,
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
            let outcome = self.command.as_mut().map(|c| c.handle_key(key));
            match outcome {
                Some(LineOutcome::Done) => {
                    let line = self.command.take().map(|c| c.text).unwrap_or_default();
                    self.run_command(&line);
                }
                Some(LineOutcome::Cancel) => self.command = None,
                Some(LineOutcome::None) | None => {}
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
            ExCommand::Unknown(cmd) => {
                self.msg = Some((
                    format!(
                        "unknown command: {cmd} (try {})",
                        crate::commands::builtins::known_commands_hint()
                    ),
                    Instant::now(),
                ));
            }
        }
    }
}

pub fn run(connection: Connection, cfg: Config) -> io::Result<()> {
    let mut terminal = ratatui::init();
    let res = (|| {
        crossterm::execute!(std::io::stdout(), crossterm::event::EnableMouseCapture)?;
        let mut app = App::new(connection, cfg);
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
