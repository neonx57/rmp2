use super::app::App;
use crate::proto::Theme;
use crate::ui::dialog::{
    self, commands_dialog, error_dialog, health_dialog, help_dialog, settings_dialog,
};
use crate::ui::panes::{
    Section, command_bar, filter_pane, mini_pane, queue_pane, search_bar, state_pane, status_bar,
};
use crate::ui::theme::{self, palette};
use ratatui::widgets::Clear;
use std::time::Duration;

impl App {
    pub(super) fn render(&mut self, frame: &mut ratatui::Frame) {
        let area = frame.area();
        let panes = super::layout::compute(area);
        self.filter_area = panes.filter;
        self.queue_area = panes.queue;
        self.state_area = panes.state;
        self.status_area = panes.status;
        self.mini_area = panes.mini;
        let snap = self.snap.as_ref();
        match snap {
            Some(s) => {
                let p = palette(s.theme);
                if s.transparent {
                    frame.render_widget(ratatui::widgets::Paragraph::new(""), panes.main);
                }
                filter_pane(
                    frame,
                    &self.cfg.titles.filter,
                    panes.filter_top,
                    &s.tags,
                    self.section == Section::Filter,
                    self.tag_cursor,
                    p,
                );
                mini_pane(
                    frame,
                    &self.cfg.titles.mini,
                    panes.mini,
                    &s.all_media,
                    &s.mini_queue,
                    s.now.as_ref(),
                    self.section == Section::Mini,
                    self.mini_cursor,
                    p,
                );
                queue_pane(
                    frame,
                    &self.cfg.titles.queue,
                    panes.queue,
                    &s.all_media,
                    &s.queue,
                    s.now.as_ref(),
                    self.section == Section::Queue,
                    self.queue_cursor,
                    s.search.as_deref(),
                    p,
                );
                state_pane(
                    frame,
                    &self.cfg.titles.info,
                    panes.state,
                    s.selected.as_ref(),
                    self.section == Section::Info,
                    p,
                );
                let anim = self.loading_since.map(|t| t.elapsed());
                let msg = self
                    .msg
                    .as_ref()
                    .filter(|(_, t)| t.elapsed() < Duration::from_secs(4))
                    .map(|(m, _)| m.as_str());
                if let Some(sb) = &self.search {
                    let invalid =
                        sb.pattern().is_some() && regex::Regex::new(sb.text.trim()).is_err();
                    search_bar(
                        frame,
                        panes.status,
                        &sb.text,
                        sb.cursor,
                        s.queue.len(),
                        s.all_media.len(),
                        invalid,
                        theme::statusline_palette(),
                    );
                } else {
                    let hits = status_bar(frame, panes.status, s, msg, anim);
                    self.status_rep = hits.rep;
                    self.status_shf = hits.shf;
                }
            }
            None => {
                frame.render_widget(
                    ratatui::widgets::Paragraph::new("connecting..."),
                    panes.main,
                );
            }
        }
        if let Some(input) = &self.command {
            command_bar(
                frame,
                panes.cmdline,
                &input.text,
                input.cursor,
                theme::statusline_palette(),
            );
        }
        if let Some(d) = &self.dialog {
            frame.render_widget(Clear, panes.queue);
            dialog::form_dialog(frame, panes.queue, d);
        }
        if let Some(st) = &self.settings {
            let (name, trans) = match self.snap.as_ref() {
                Some(s) => (s.theme.name(), s.transparent),
                None => ("dark", false),
            };
            let p = palette(self.snap.as_ref().map(|s| s.theme).unwrap_or(Theme::Dark));
            settings_dialog(frame, panes.queue, st.cursor, name, trans, p);
        }
        if let Some(err) = &self.error_popup {
            let p = palette(self.snap.as_ref().map(|s| s.theme).unwrap_or(Theme::Dark));
            error_dialog(frame, panes.queue, err, p);
        }
        if self.help_open {
            let p = palette(self.snap.as_ref().map(|s| s.theme).unwrap_or(Theme::Dark));
            if self.help_commands {
                commands_dialog(frame, panes.main, p);
            } else {
                help_dialog(frame, panes.main, &self.cfg.keybindings, p);
            }
        }
        if let Some(entries) = &self.health {
            let p = palette(self.snap.as_ref().map(|s| s.theme).unwrap_or(Theme::Dark));
            health_dialog(frame, panes.main, entries, p);
        }
    }
}
