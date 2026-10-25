use super::frame_block;
use crate::proto::TagInfo;
use crate::ui::theme::Palette;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{List, ListItem};

pub fn filter_pane(
    frame: &mut Frame,
    title: &str,
    area: Rect,
    tags: &[TagInfo],
    focused: bool,
    cursor: usize,
    p: Palette,
) {
    let items: Vec<ListItem> = tags
        .iter()
        .enumerate()
        .map(|(i, t)| {
            let mark = if t.checked { "x" } else { " " };
            let text = format!("[{mark}] {} ({})", t.name, t.count);
            let style = if focused && i == cursor {
                Style::default().add_modifier(Modifier::REVERSED)
            } else {
                Style::default()
            };
            ListItem::new(text).style(style)
        })
        .collect();
    let list = List::new(items)
        .style(Style::default().fg(p.text))
        .block(frame_block(title, focused, p));
    frame.render_widget(list, area);
}
