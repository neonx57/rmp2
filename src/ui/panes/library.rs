use super::{display_name, frame_block, scroll_offset};
use crate::proto::{MediaInfo, NowPlaying};
use crate::ui::theme::Palette;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{List, ListItem};

#[allow(clippy::too_many_arguments)]
pub fn queue_pane(
    frame: &mut Frame,
    title: &str,
    area: Rect,
    media: &[MediaInfo],
    queue: &[i64],
    now: Option<&NowPlaying>,
    focused: bool,
    cursor: usize,
    search: Option<&str>,
    p: Palette,
) {
    let re = search
        .filter(|p| !p.trim().is_empty())
        .and_then(|p| regex::Regex::new(p).ok());
    let by_id: std::collections::HashMap<i64, &MediaInfo> =
        media.iter().map(|m| (m.id, m)).collect();
    let height = area.height.saturating_sub(2) as usize;
    let start = scroll_offset(queue.len(), cursor, height);
    let items: Vec<ListItem> = queue
        .iter()
        .enumerate()
        .filter_map(|(i, id)| by_id.get(id).copied().map(|m| (i, m)))
        .skip(start)
        .take(height)
        .map(|(i, m)| {
            let now_marks = now.filter(|n| n.id == m.id).is_some();
            let play_mark = if now_marks { ">" } else { " " };
            let fav = if m.favorite { "*" } else { " " };
            let name = display_name(m);
            let mut spans = vec![Span::raw(format!("{play_mark}{fav} "))];
            spans.extend(highlight_matches(&name, re.as_ref()));
            if let Some(a) = m.artist.as_ref().filter(|a| !a.trim().is_empty()) {
                spans.push(Span::raw(format!(" - {a}")));
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

fn highlight_matches(text: &str, re: Option<&regex::Regex>) -> Vec<Span<'static>> {
    let mut spans: Vec<Span> = Vec::new();
    let Some(re) = re else {
        return vec![Span::raw(text.to_string())];
    };
    let lowered = text.to_lowercase();
    let mut last = 0;
    for m in re.find_iter(&lowered) {
        if m.start() > last {
            spans.push(Span::raw(text[last..m.start()].to_string()));
        }
        spans.push(Span::styled(
            text[m.start()..m.end()].to_string(),
            Style::default().add_modifier(Modifier::UNDERLINED),
        ));
        last = m.end();
    }
    if last < text.len() {
        spans.push(Span::raw(text[last..].to_string()));
    }
    if spans.is_empty() {
        spans.push(Span::raw(text.to_string()));
    }
    spans
}
