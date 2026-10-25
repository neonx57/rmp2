pub use crate::app::action::Action;
use crate::app::action::action_from_str;
use crate::config::Config;
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum LineOutcome {
    Done,
    Cancel,
    None,
}

pub(super) struct LineInput {
    pub(super) text: String,
    pub(super) cursor: usize,
}

impl LineInput {
    pub(super) fn new() -> Self {
        LineInput {
            text: String::new(),
            cursor: 0,
        }
    }

    pub(super) fn handle_key(&mut self, key: KeyEvent) -> LineOutcome {
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            return LineOutcome::None;
        }
        match key.code {
            KeyCode::Esc => LineOutcome::Cancel,
            KeyCode::Enter => LineOutcome::Done,
            KeyCode::Backspace => {
                if let Some(prev) = self.prev_boundary() {
                    self.text.remove(prev);
                    self.cursor = prev;
                }
                LineOutcome::None
            }
            KeyCode::Delete => {
                if self.cursor < self.text.len() {
                    self.text.remove(self.cursor);
                }
                LineOutcome::None
            }
            KeyCode::Left => {
                if let Some(prev) = self.prev_boundary() {
                    self.cursor = prev;
                }
                LineOutcome::None
            }
            KeyCode::Right => {
                if self.cursor < self.text.len() {
                    self.cursor = self.next_boundary();
                }
                LineOutcome::None
            }
            KeyCode::Home => {
                self.cursor = 0;
                LineOutcome::None
            }
            KeyCode::End => {
                self.cursor = self.text.len();
                LineOutcome::None
            }
            KeyCode::Char(c) => {
                self.text.insert(self.cursor, c);
                self.cursor += c.len_utf8();
                LineOutcome::None
            }
            _ => LineOutcome::None,
        }
    }

    fn prev_boundary(&self) -> Option<usize> {
        if self.cursor == 0 {
            return None;
        }
        self.text
            .char_indices()
            .take_while(|(i, _)| *i < self.cursor)
            .last()
            .map(|(i, _)| i)
    }

    fn next_boundary(&self) -> usize {
        self.text
            .char_indices()
            .find(|(i, _)| *i > self.cursor)
            .map(|(i, _)| i)
            .unwrap_or(self.text.len())
    }

    pub(super) fn pattern(&self) -> Option<String> {
        let t = self.text.trim();
        if t.is_empty() {
            None
        } else {
            Some(t.to_string())
        }
    }
}

pub struct Keymap {
    map: HashMap<String, Action>,
}

impl Keymap {
    pub fn build(cfg: &Config) -> Self {
        let mut map = HashMap::new();
        for (action, keys) in &cfg.keybindings {
            let Some(a) = action_from_str(action) else {
                continue;
            };
            if keys.iter().any(|k| k == "none") {
                continue;
            }
            for k in keys {
                map.insert(k.clone(), a);
            }
        }
        Keymap { map }
    }

    pub fn resolve(&self, key: KeyEvent) -> Option<Action> {
        key_string(key).and_then(|s| self.map.get(&s).copied())
    }
}

fn key_string(key: KeyEvent) -> Option<String> {
    if key.kind != KeyEventKind::Press {
        return None;
    }
    match key.code {
        KeyCode::Char(c) => {
            if key.modifiers.contains(KeyModifiers::CONTROL) {
                Some(format!("ctrl+{c}"))
            } else if c.is_ascii_uppercase() {
                Some(c.to_string())
            } else if c == ' ' {
                Some("space".into())
            } else {
                Some(c.to_string())
            }
        }
        KeyCode::Enter => Some("enter".into()),
        KeyCode::Esc => Some("esc".into()),
        KeyCode::Tab => Some("tab".into()),
        KeyCode::BackTab => Some("backtab".into()),
        KeyCode::Up => Some("up".into()),
        KeyCode::Down => Some("down".into()),
        KeyCode::Left => Some("left".into()),
        KeyCode::Right => Some("right".into()),
        KeyCode::Home => Some("home".into()),
        KeyCode::End => Some("end".into()),
        KeyCode::PageUp => Some("pageup".into()),
        KeyCode::PageDown => Some("pagedown".into()),
        KeyCode::Insert => Some("insert".into()),
        KeyCode::Delete => Some("delete".into()),
        KeyCode::Backspace => Some("backspace".into()),
        KeyCode::F(n) => Some(format!("f{n}")),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode as KC, KeyEvent, KeyModifiers};

    fn press(code: KC, mods: KeyModifiers) -> KeyEvent {
        KeyEvent::new(code, mods)
    }

    #[test]
    fn shift_letters_become_uppercase_keys() {
        assert_eq!(
            key_string(press(KC::Char('j'), KeyModifiers::NONE)).as_deref(),
            Some("j")
        );
        assert_eq!(
            key_string(press(KC::Char('J'), KeyModifiers::SHIFT)).as_deref(),
            Some("J")
        );
    }

    #[test]
    fn ctrl_letters_map_to_ctrl_prefix() {
        assert_eq!(
            key_string(press(KC::Char('j'), KeyModifiers::CONTROL)).as_deref(),
            Some("ctrl+j")
        );
        assert_eq!(
            key_string(press(KC::Char('k'), KeyModifiers::CONTROL)).as_deref(),
            Some("ctrl+k")
        );
    }

    #[test]
    fn uppercase_without_shift_flag_still_uppercase() {
        assert_eq!(
            key_string(press(KC::Char('Q'), KeyModifiers::NONE)).as_deref(),
            Some("Q")
        );
    }

    #[test]
    fn special_keys() {
        assert_eq!(
            key_string(press(KC::Enter, KeyModifiers::NONE)).as_deref(),
            Some("enter")
        );
        assert_eq!(
            key_string(press(KC::Tab, KeyModifiers::NONE)).as_deref(),
            Some("tab")
        );
        assert_eq!(
            key_string(press(KC::BackTab, KeyModifiers::NONE)).as_deref(),
            Some("backtab")
        );
        assert_eq!(
            key_string(press(KC::Char(' '), KeyModifiers::NONE)).as_deref(),
            Some("space")
        );
    }

    #[test]
    fn repeat_events_ignored() {
        let mut e = press(KC::Char('j'), KeyModifiers::NONE);
        e.kind = crossterm::event::KeyEventKind::Repeat;
        assert!(key_string(e).is_none());
    }

    #[test]
    fn none_unbinds_a_default_action() {
        let mut raw = Config::default();
        raw.keybindings
            .insert("move_up".into(), vec!["none".into()]);
        let km = Keymap::build(&raw);
        assert_eq!(
            km.resolve(press(KC::Char('j'), KeyModifiers::NONE)),
            Some(Action::MoveDown)
        );
        assert_eq!(km.resolve(press(KC::Char('k'), KeyModifiers::NONE)), None);
        assert_eq!(km.resolve(press(KC::Char('x'), KeyModifiers::NONE)), None);
    }

    #[test]
    fn remapped_action_uses_new_key() {
        let mut raw = Config::default();
        raw.keybindings.insert("toggle".into(), vec!["x".into()]);
        let km = Keymap::build(&raw);
        assert_eq!(
            km.resolve(press(KC::Char('x'), KeyModifiers::NONE)),
            Some(Action::Toggle)
        );
        assert_eq!(km.resolve(press(KC::Char(' '), KeyModifiers::NONE)), None);
    }

    #[test]
    fn default_bindings_resolve() {
        let cfg = Config::default();
        let km = Keymap::build(&cfg);
        assert_eq!(
            km.resolve(press(KC::Char('j'), KeyModifiers::NONE)),
            Some(Action::MoveDown)
        );
        assert_eq!(
            km.resolve(press(KC::Char('K'), KeyModifiers::SHIFT)),
            Some(Action::VolumeUp)
        );
        assert_eq!(
            km.resolve(press(KC::Char('Q'), KeyModifiers::SHIFT)),
            Some(Action::Detach)
        );
        assert_eq!(
            km.resolve(press(KC::Char(' '), KeyModifiers::NONE)),
            Some(Action::Toggle)
        );
        assert_eq!(
            km.resolve(press(KC::Char(':'), KeyModifiers::SHIFT)),
            Some(Action::Command)
        );
    }
}

#[cfg(test)]
mod line_tests {
    use super::*;
    use crossterm::event::KeyCode;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    #[test]
    fn searching_accumulates_text_and_live_pattern() {
        let mut sb = LineInput::new();
        assert_eq!(sb.pattern(), None);
        for c in "beat".chars() {
            assert_eq!(sb.handle_key(key(KeyCode::Char(c))), LineOutcome::None);
        }
        assert_eq!(sb.pattern(), Some("beat".to_string()));
        assert_eq!(sb.handle_key(key(KeyCode::Esc)), LineOutcome::Cancel);
    }

    #[test]
    fn search_backspace_and_enter() {
        let mut sb = LineInput::new();
        for c in "abc".chars() {
            sb.handle_key(key(KeyCode::Char(c)));
        }
        sb.handle_key(key(KeyCode::Backspace));
        assert_eq!(sb.text, "ab");
        assert_eq!(sb.handle_key(key(KeyCode::Enter)), LineOutcome::Done);
    }

    #[test]
    fn search_empty_or_whitespace_is_no_pattern() {
        let mut sb = LineInput::new();
        assert_eq!(sb.pattern(), None);
        sb.handle_key(key(KeyCode::Char(' ')));
        sb.handle_key(key(KeyCode::Char(' ')));
        assert_eq!(sb.pattern(), None);
    }

    #[test]
    fn line_input_keeps_cursor_on_char_boundaries() {
        let mut input = LineInput::new();
        for c in "caf\u{e9}".chars() {
            input.handle_key(key(KeyCode::Char(c)));
        }
        assert_eq!(input.text, "caf\u{e9}");
        assert_eq!(input.cursor, input.text.len());
        input.handle_key(key(KeyCode::Left));
        assert_eq!(input.cursor, 3);
        assert!(input.text.is_char_boundary(input.cursor));
        input.handle_key(key(KeyCode::Right));
        assert_eq!(input.cursor, input.text.len());
        input.handle_key(key(KeyCode::Backspace));
        assert_eq!(input.text, "caf");
        assert_eq!(input.cursor, 3);
        input.handle_key(key(KeyCode::Char('\u{e9}')));
        assert_eq!(input.text, "caf\u{e9}");
        assert!(input.text.is_char_boundary(input.cursor));
    }
}
