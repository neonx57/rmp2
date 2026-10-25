pub mod filters;
pub mod library;
pub mod mini_queue;
pub mod status;

pub use filters::filter_pane;
pub use library::queue_pane;
pub use mini_queue::mini_pane;
pub use status::{StatusBarHits, command_bar, scroll_offset, search_bar, state_pane, status_bar};

use crate::proto::MediaInfo;
use crate::ui::theme::Palette;
use ratatui::layout::Alignment;
use ratatui::style::Style;
use ratatui::symbols;
use ratatui::widgets::{Block, Borders};
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Section {
    Filter,
    Mini,
    Queue,
    Info,
}

impl Section {
    pub fn next(self) -> Self {
        match self {
            Section::Filter => Section::Mini,
            Section::Mini => Section::Queue,
            Section::Queue => Section::Info,
            Section::Info => Section::Filter,
        }
    }
    pub fn prev(self) -> Self {
        match self {
            Section::Filter => Section::Info,
            Section::Mini => Section::Filter,
            Section::Queue => Section::Mini,
            Section::Info => Section::Queue,
        }
    }
}

pub const ASCII_BORDER: symbols::border::Set = symbols::border::Set {
    top_left: "+",
    top_right: "+",
    bottom_left: "+",
    bottom_right: "+",
    vertical_left: "|",
    vertical_right: "|",
    horizontal_top: "-",
    horizontal_bottom: "-",
};

fn frame_block(title: &str, focused: bool, p: Palette) -> Block<'static> {
    let mut b = Block::default()
        .borders(Borders::ALL)
        .border_set(ASCII_BORDER)
        .title(format!(" {title} "))
        .title_alignment(Alignment::Center)
        .border_style(Style::default().fg(if focused { p.accent } else { p.muted }));
    if let Some(bg) = p.pane_bg {
        b = b.style(Style::default().bg(bg));
    }
    b
}

fn display_name(m: &MediaInfo) -> String {
    if let Some(t) = &m.title
        && !t.trim().is_empty()
    {
        return t.clone();
    }
    Path::new(&m.uri)
        .file_name()
        .map(|f| f.to_string_lossy().into_owned())
        .unwrap_or_else(|| m.uri.clone())
}
