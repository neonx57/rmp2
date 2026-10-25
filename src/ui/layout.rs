use ratatui::layout::{Constraint, Layout, Rect};

pub(super) struct Panes {
    pub(super) main: Rect,
    pub(super) filter: Rect,
    pub(super) filter_top: Rect,
    pub(super) mini: Rect,
    pub(super) queue: Rect,
    pub(super) state: Rect,
    pub(super) status: Rect,
    pub(super) cmdline: Rect,
}

pub(super) fn compute(area: Rect) -> Panes {
    let [main, status, cmdline] = Layout::vertical([
        Constraint::Min(0),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .areas(area);
    let [filter, queue, state] = Layout::horizontal([
        Constraint::Ratio(1, 4),
        Constraint::Ratio(2, 4),
        Constraint::Ratio(1, 4),
    ])
    .areas(main);
    let [filter_top, mini] =
        Layout::vertical([Constraint::Percentage(65), Constraint::Percentage(35)]).areas(filter);
    Panes {
        main,
        filter,
        filter_top,
        mini,
        queue,
        state,
        status,
        cmdline,
    }
}
