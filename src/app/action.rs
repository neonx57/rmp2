#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    MoveUp,
    MoveDown,
    PrevSection,
    NextSection,
    CycleFocus,
    CycleFocusBack,
    Activate,
    Toggle,
    NextTrack,
    PrevTrack,
    VolumeUp,
    VolumeDown,
    SeekBack,
    SeekFwd,
    Repeat,
    Shuffle,
    AddMedia,
    EditMedia,
    Search,
    Favorite,
    AddMini,
    FocusMini,
    Delete,
    MiniMoveUp,
    MiniMoveDown,
    ConfirmQuit,
    Detach,
    Settings,
    Help,
    Command,
}

pub const ACTIONS: &[(Action, &str)] = &[
    (Action::MoveUp, "move_up"),
    (Action::MoveDown, "move_down"),
    (Action::PrevSection, "prev_section"),
    (Action::NextSection, "next_section"),
    (Action::CycleFocus, "cycle_focus"),
    (Action::CycleFocusBack, "cycle_focus_back"),
    (Action::Activate, "activate"),
    (Action::Toggle, "toggle"),
    (Action::NextTrack, "next_track"),
    (Action::PrevTrack, "prev_track"),
    (Action::VolumeUp, "volume_up"),
    (Action::VolumeDown, "volume_down"),
    (Action::SeekBack, "seek_back"),
    (Action::SeekFwd, "seek_fwd"),
    (Action::Repeat, "repeat"),
    (Action::Shuffle, "shuffle"),
    (Action::AddMedia, "add_media"),
    (Action::EditMedia, "edit_media"),
    (Action::Search, "search"),
    (Action::Favorite, "favorite"),
    (Action::AddMini, "add_mini"),
    (Action::FocusMini, "focus_mini"),
    (Action::Delete, "delete"),
    (Action::MiniMoveUp, "mini_move_up"),
    (Action::MiniMoveDown, "mini_move_down"),
    (Action::ConfirmQuit, "confirm_quit"),
    (Action::Detach, "detach"),
    (Action::Settings, "settings"),
    (Action::Help, "help"),
    (Action::Command, "command"),
];

pub fn action_from_str(name: &str) -> Option<Action> {
    ACTIONS.iter().find(|(_, n)| *n == name).map(|(a, _)| *a)
}

pub fn action_str(action: Action) -> &'static str {
    ACTIONS
        .iter()
        .find(|(a, _)| *a == action)
        .map(|(_, n)| *n)
        .unwrap_or("?")
}

pub fn action_group(action: Action) -> &'static str {
    match action {
        Action::MoveUp
        | Action::MoveDown
        | Action::PrevSection
        | Action::NextSection
        | Action::CycleFocus
        | Action::CycleFocusBack
        | Action::Activate => "Navigation",
        Action::Toggle
        | Action::NextTrack
        | Action::PrevTrack
        | Action::VolumeUp
        | Action::VolumeDown
        | Action::SeekBack
        | Action::SeekFwd
        | Action::Repeat
        | Action::Shuffle => "Playback",
        Action::AddMedia
        | Action::EditMedia
        | Action::Search
        | Action::Favorite
        | Action::Delete => "Library",
        Action::AddMini | Action::FocusMini | Action::MiniMoveUp | Action::MiniMoveDown => {
            "Mini Queue"
        }
        Action::ConfirmQuit
        | Action::Detach
        | Action::Settings
        | Action::Help
        | Action::Command => "App",
    }
}

pub fn action_label(action: Action) -> String {
    action_str(action).split('_').collect::<Vec<_>>().join(" ")
}
