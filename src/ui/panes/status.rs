use super::{display_name, frame_block};
use crate::proto::{MediaInfo, RepeatMode, Snapshot};
use crate::ui::theme::{self, Palette};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Wrap};
use std::time::Duration;

fn fmt_time(secs: f64) -> String {
    if !secs.is_finite() || secs < 0.0 {
        return "--:--".to_string();
    }
    let s = secs as u64;
    format!("{}:{:02}", s / 60, s % 60)
}

fn progress_bar(pos: f64, dur: f64, width: usize) -> Vec<Span<'static>> {
    let pct = if dur <= 0.0 {
        0.0
    } else {
        (pos / dur).clamp(0.0, 1.0)
    };
    let filled = (pct * width as f64) as usize;
    let bold = Style::default()
        .fg(Color::Black)
        .add_modifier(Modifier::BOLD);
    let plain = Style::default().fg(Color::Black);
    vec![
        Span::styled("[", bold),
        Span::styled("=".repeat(filled), bold),
        Span::styled("-".repeat(width.saturating_sub(filled)), plain),
        Span::styled("]", bold),
    ]
}

fn loading_bar(elapsed: Duration, width: usize) -> Vec<Span<'static>> {
    if width < 4 {
        return vec![Span::styled(
            format!("[{}]", "=".repeat(width)),
            Style::default()
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
        )];
    }
    let span_len = width - 2;
    let step = (elapsed.as_millis() as usize / 40) % (2 * span_len);
    let at = if step < span_len {
        step
    } else {
        2 * span_len - step
    };
    let bold = Style::default()
        .fg(Color::Black)
        .add_modifier(Modifier::BOLD);
    let plain = Style::default().fg(Color::Black);
    vec![
        Span::styled("[", bold),
        Span::styled("-".repeat(at), plain),
        Span::styled("==".to_string(), bold),
        Span::styled("-".repeat(span_len - at), plain),
        Span::styled("]", bold),
    ]
}

pub fn scroll_offset(total: usize, cursor: usize, height: usize) -> usize {
    if height == 0 || total <= height {
        0
    } else {
        cursor.saturating_sub(height / 2).min(total - height)
    }
}

pub fn state_pane(
    frame: &mut Frame,
    title: &str,
    area: Rect,
    selected: Option<&MediaInfo>,
    focused: bool,
    p: Palette,
) {
    let mut lines = Vec::new();
    if let Some(m) = selected {
        lines.push(Line::from(format!("Title:     {}", display_name(m))));
        if let Some(a) = m.artist.as_ref().filter(|a| !a.trim().is_empty()) {
            lines.push(Line::from(format!("Artist:    {a}")));
        }
        let sep_w = area.width.saturating_sub(2) as usize;
        lines.push(Line::from(Span::styled(
            "-".repeat(sep_w),
            Style::default().fg(p.muted),
        )));
        lines.push(Line::from(format!("Type:      {}", m.kind)));
        if let Some(s) = &m.source {
            lines.push(Line::from(format!("Source:    {s}")));
        }
        if let Some(d) = m.duration {
            lines.push(Line::from(format!("Duration:  {}", fmt_time(d))));
        }
        if let Some(b) = m.bitrate {
            lines.push(Line::from(format!("Bitrate:   {} bps", b)));
        }
        if !m.tags.is_empty() {
            lines.push(Line::from(Span::styled(
                "-".repeat(sep_w),
                Style::default().fg(p.muted),
            )));
            for t in &m.tags {
                lines.push(Line::from(format!("- {t}")));
            }
        }
    } else {
        lines.push(Line::from("Nothing selected"));
    }
    let para = Paragraph::new(lines)
        .style(Style::default().fg(p.text))
        .block(frame_block(title, focused, p))
        .wrap(Wrap { trim: false });
    frame.render_widget(para, area);
}

#[allow(clippy::too_many_arguments)]
pub fn search_bar(
    frame: &mut Frame,
    area: Rect,
    text: &str,
    cursor: usize,
    count: usize,
    total: usize,
    invalid: bool,
    p: Palette,
) {
    let mut spans = input_spans("/", text, cursor, p);
    let counter = if invalid {
        Span::styled("0/0", Style::default().fg(p.warn))
    } else {
        Span::styled(format!("{count}/{total}"), Style::default().fg(p.muted))
    };
    let left_w: usize = spans.iter().map(|s| s.content.chars().count()).sum();
    let right_w = counter.content.chars().count();
    let gap = area.width.saturating_sub((left_w + right_w) as u16) as usize;
    spans.push(Span::raw(" ".repeat(gap)));
    spans.push(counter);
    let bareg = p.pane_bg.unwrap_or(Color::White);
    let para = Paragraph::new(Line::from(spans)).style(Style::default().bg(bareg));
    frame.render_widget(para, area);
}

pub fn command_bar(frame: &mut Frame, area: Rect, text: &str, cursor: usize, p: Palette) {
    let mut spans = input_spans(":", text, cursor, p);
    let used: usize = spans.iter().map(|s| s.content.chars().count()).sum();
    let gap = area.width.saturating_sub(used as u16) as usize;
    spans.push(Span::raw(" ".repeat(gap)));
    let bareg = p.pane_bg.unwrap_or(Color::White);
    let para = Paragraph::new(Line::from(spans)).style(Style::default().bg(bareg));
    frame.render_widget(para, area);
}

fn input_spans(prompt: &str, text: &str, cursor: usize, p: Palette) -> Vec<Span<'static>> {
    let text_style = Style::default().fg(p.text);
    let cursor_style = Style::default().fg(p.warn).add_modifier(Modifier::REVERSED);
    let mut spans: Vec<Span> = Vec::new();
    spans.push(Span::styled(
        prompt.to_string(),
        Style::default().fg(p.accent).add_modifier(Modifier::BOLD),
    ));
    let cur = cursor.min(text.len());
    spans.push(Span::styled(text[..cur].to_string(), text_style));
    spans.push(Span::styled(" ", cursor_style));
    spans.push(Span::styled(text[cur..].to_string(), text_style));
    spans
}

pub struct StatusBarHits {
    pub rep: Option<Rect>,
    pub shf: Option<Rect>,
}

pub fn status_bar(
    frame: &mut Frame,
    area: Rect,
    snap: &Snapshot,
    msg: Option<&str>,
    anim: Option<Duration>,
) -> StatusBarHits {
    let p = theme::statusline_palette();
    let text_style = Style::default().fg(p.text);
    let bold_style = text_style.add_modifier(Modifier::BOLD);
    let mut left: Vec<Span> = Vec::new();
    if let Some(text) = msg {
        left.push(Span::styled(
            text,
            Style::default().fg(p.warn).add_modifier(Modifier::BOLD),
        ));
    } else if let Some(n) = &snap.now {
        let title = snap
            .all_media
            .iter()
            .find(|m| m.id == n.id)
            .map(display_name)
            .unwrap_or_else(|| "unknown".into());
        let dur = n.duration.unwrap_or(0.0);
        left.push(Span::styled(
            format!("{} /", fmt_time(n.position)),
            bold_style,
        ));
        left.push(Span::styled(format!(" {} ", fmt_time(dur)), text_style));
        let w = area.width.saturating_sub(46) as usize;
        let bar = match anim {
            Some(t) => loading_bar(t, w.min(20)),
            None => progress_bar(n.position, dur, w.min(20)),
        };
        left.extend(bar);
        left.push(Span::styled(format!(" {title}"), bold_style));
    } else {
        left.push(Span::styled("stopped", bold_style));
    }
    let lbl = bold_style;
    let mut right: Vec<Span> = Vec::new();
    right.push(Span::raw("  "));
    right.push(Span::styled("vol", lbl));
    right.push(Span::styled(format!(" {:>3}", snap.volume), text_style));
    let rep = match snap.repeat {
        RepeatMode::Off => "off",
        RepeatMode::All => "all",
        RepeatMode::One => "one",
    };
    right.push(Span::raw("  "));
    right.push(Span::styled("rep", lbl));
    right.push(Span::styled(format!(" {rep}"), text_style));
    right.push(Span::raw("  "));
    right.push(Span::styled("shf", lbl));
    right.push(Span::styled(
        if snap.shuffle { " on" } else { " off" },
        text_style,
    ));
    if let Some(tags) = snap.tags.iter().find(|t| t.checked) {
        right.push(Span::raw("  "));
        right.push(Span::styled("tags", lbl));
        right.push(Span::styled(format!(":{}", tags.name), text_style));
    }
    let left_w: usize = left.iter().map(|s| s.content.chars().count()).sum();
    let right_w: usize = right.iter().map(|s| s.content.chars().count()).sum();
    let gap = area.width.saturating_sub((left_w + right_w) as u16) as usize;
    let x0 = area.x as i64 + area.width as i64 - right_w as i64;
    let mut x = x0;
    let mut hit_rep = None;
    let mut hit_shf = None;
    for (i, s) in right.iter().enumerate() {
        let w = s.content.chars().count() as u16;
        let rect = Rect::new(x.max(0) as u16, area.y, w, 1);
        match i {
            4 => hit_rep = Some(rect),
            7 => hit_shf = Some(rect),
            _ => {}
        }
        x += w as i64;
    }
    let mut spans = left;
    spans.push(Span::raw(" ".repeat(gap)));
    spans.extend(right);
    let bareg = p.pane_bg.unwrap_or(Color::White);
    let para = Paragraph::new(Line::from(spans)).style(Style::default().bg(bareg));
    frame.render_widget(para, area);
    StatusBarHits {
        rep: hit_rep,
        shf: hit_shf,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scroll_offset_pins_cursor_to_center() {
        assert_eq!(scroll_offset(20, 0, 5), 0);
        assert_eq!(scroll_offset(20, 2, 5), 0);
        assert_eq!(scroll_offset(20, 9, 5), 7);
        assert_eq!(scroll_offset(20, 17, 5), 15);
        assert_eq!(scroll_offset(20, 19, 5), 15);
    }

    #[test]
    fn scroll_offset_no_scrolling_when_fits() {
        assert_eq!(scroll_offset(3, 2, 5), 0);
        assert_eq!(scroll_offset(5, 0, 5), 0);
    }

    #[test]
    fn scroll_offset_zero_height_is_zero() {
        assert_eq!(scroll_offset(20, 15, 0), 0);
    }

    fn bar_text(spans: &[Span]) -> String {
        spans.iter().map(|s| s.content.to_string()).collect()
    }

    fn is_bold(span: &Span) -> bool {
        span.style.add_modifier.contains(Modifier::BOLD)
    }

    #[test]
    fn loading_bar_starts_left_and_reaches_right() {
        let zero = loading_bar(Duration::ZERO, 10);
        assert_eq!(bar_text(&zero), "[==--------]");
        assert!(zero[0].style.fg == Some(Color::Black) && is_bold(&zero[0]));
        assert!(is_bold(&zero[2]));
        assert!(is_bold(&zero[4]));
        assert!(!is_bold(&zero[1]) && !is_bold(&zero[3]));
        let right = loading_bar(Duration::from_millis(320), 10);
        assert_eq!(bar_text(&right), "[--------==]");
    }

    #[test]
    fn loading_bar_bounces_back_left() {
        let back = loading_bar(Duration::from_millis(720), 10);
        assert_eq!(bar_text(&back), "[--==------]");
        let cycle = loading_bar(Duration::from_millis(640), 10);
        assert_eq!(bar_text(&cycle), "[==--------]");
    }

    #[test]
    fn loading_bar_tiny_width() {
        let tiny = loading_bar(Duration::ZERO, 3);
        assert_eq!(bar_text(&tiny), "[===]");
        assert!(is_bold(&tiny[0]));
    }

    #[test]
    fn progress_bar_splits_bold_brackets_and_fill() {
        let bar = progress_bar(100.0, 200.0, 10);
        assert_eq!(bar_text(&bar), "[=====-----]");
        assert!(is_bold(&bar[0]));
        assert!(is_bold(&bar[1]) && bar[1].content == "=====");
        assert!(!is_bold(&bar[2]) && bar[2].content == "-".repeat(5));
        assert!(is_bold(&bar[3]));
    }
}
