use super::{display_name, frame_block};
use crate::proto::{MediaInfo, NowPlaying};
use crate::ui::theme::Palette;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{List, ListItem};

#[allow(clippy::too_many_arguments)]
pub fn mini_pane(
    frame: &mut Frame,
    title: &str,
    area: Rect,
    media: &[MediaInfo],
    mini_queue: &[i64],
    now: Option<&NowPlaying>,
    focused: bool,
    cursor: usize,
    p: Palette,
) {
    let by_id: std::collections::HashMap<i64, &MediaInfo> =
        media.iter().map(|m| (m.id, m)).collect();
    let items: Vec<ListItem> = mini_queue
        .iter()
        .enumerate()
        .filter_map(|(i, id)| by_id.get(id).copied().map(|m| (i, m)))
        .map(|(i, m)| {
            let now_marks = now.filter(|n| n.id == m.id).is_some();
            let mark = if now_marks { ">" } else { " " };
            let name = display_name(m);
            let mut spans = vec![Span::raw(format!("{mark}{}. {} ", i + 1, name))];
            if let Some(a) = m.artist.as_ref().filter(|a| !a.trim().is_empty()) {
                spans.push(Span::raw(format!("- {a}")));
            }
            let mut style = Style::default();
            if focused && i == cursor {
                style = style.add_modifier(Modifier::REVERSED);
            }
            if now_marks {
                style = style.fg(p.now_playing);
            }
            ListItem::new(Line::from(spans)).style(style)
        })
        .collect();
    let list = List::new(items)
        .style(Style::default().fg(p.text))
        .block(frame_block(title, focused, p));
    frame.render_widget(list, area);
}
