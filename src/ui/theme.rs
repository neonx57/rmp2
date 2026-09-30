use crate::proto::Theme;
use ratatui::style::{Color, Modifier, Style};

#[derive(Debug, Clone, Copy)]
pub struct Palette {
    pub text: Color,
    pub muted: Color,
    pub accent: Color,
    pub now_playing: Color,
    pub warn: Color,
    pub pane_bg: Option<Color>,
}

const DARK: Palette = Palette {
    text: Color::White,
    muted: Color::DarkGray,
    accent: Color::Cyan,
    now_playing: Color::Green,
    warn: Color::Yellow,
    pane_bg: None,
};

const LIGHT: Palette = Palette {
    text: Color::Black,
    muted: Color::Gray,
    accent: Color::Blue,
    now_playing: Color::Green,
    warn: Color::Yellow,
    pane_bg: Some(Color::White),
};

pub fn palette(theme: Theme) -> Palette {
    match theme {
        Theme::Dark => DARK,
        Theme::Light => LIGHT,
    }
}

pub fn settings_options() -> Vec<(&'static str, Vec<&'static str>)> {
    vec![
        ("colorscheme", vec!["dark", "light"]),
        ("transparency", vec!["off", "on"]),
    ]
}

pub struct SettingsState {
    pub cursor: usize,
}

impl SettingsState {
    pub fn new() -> Self {
        SettingsState { cursor: 0 }
    }

    pub fn move_cursor(&mut self, delta: i64) {
        let len = settings_options().len();
        self.cursor = (self.cursor as i64 + delta).clamp(0, len as i64 - 1) as usize;
    }
}

pub fn cursor_next(delta: i64, current: usize, count: usize) -> usize {
    (current as i64 + delta).rem_euclid(count as i64) as usize
}

pub fn value_style(value: &str) -> Style {
    Style::default().add_modifier(if value == "on" || value == "light" || value == "dark" {
        Modifier::BOLD
    } else {
        Modifier::empty()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_state_moves_and_clamps() {
        let mut s = SettingsState::new();
        assert_eq!(s.cursor, 0);
        s.move_cursor(1);
        assert_eq!(s.cursor, 1);
        s.move_cursor(5);
        assert_eq!(s.cursor, 1);
        s.move_cursor(-5);
        assert_eq!(s.cursor, 0);
    }

    #[test]
    fn cursor_next_wraps() {
        assert_eq!(cursor_next(1, 0, 2), 1);
        assert_eq!(cursor_next(1, 1, 2), 0);
        assert_eq!(cursor_next(-1, 0, 2), 1);
        assert_eq!(cursor_next(3, 0, 2), 1);
    }

    #[test]
    fn palettes_differ_per_theme() {
        let dark = palette(Theme::Dark);
        let light = palette(Theme::Light);
        assert_ne!(dark.text, light.text);
        assert_ne!(dark.accent, light.accent);
        assert_ne!(dark.pane_bg, light.pane_bg);
    }
}
