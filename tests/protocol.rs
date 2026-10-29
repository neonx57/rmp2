use vmp::config::Titles;
use vmp::proto::{
    Command, ConfigState, MediaInfo, NowPlaying, RepeatMode, Snapshot, TagInfo, Theme,
};

fn sample_media(id: i64) -> MediaInfo {
    MediaInfo {
        id,
        uri: "/music/one.flac".into(),
        kind: "offline".into(),
        title: Some("One".into()),
        artist: Some("Band".into()),
        duration: Some(181.5),
        bitrate: Some(320_000),
        source: None,
        tags: vec!["rock".into(), "favorite".into()],
        favorite: true,
    }
}

fn sample_snapshot() -> Snapshot {
    Snapshot {
        theme: Theme::Dark,
        transparent: false,
        all_media: vec![sample_media(1), sample_media(2)],
        queue: vec![1, 2],
        mini_queue: vec![2, 1],
        tags: vec![
            TagInfo {
                name: "rock".into(),
                checked: true,
                count: 2,
            },
            TagInfo {
                name: "jazz".into(),
                checked: false,
                count: 0,
            },
        ],
        search: Some("one".into()),
        selected: Some(sample_media(2)),
        now: Some(NowPlaying {
            id: 2,
            position: 12.25,
            duration: Some(181.5),
            paused: false,
            loading: true,
        }),
        volume: 65,
        repeat: RepeatMode::All,
        shuffle: false,
        notify: Some("added: One".into()),
        error: None,
        config: ConfigState {
            mpv_binary: "mpv".into(),
            titles: Titles {
                filter: "Filter".into(),
                queue: "Queue".into(),
                mini: "Mini Queue".into(),
                info: "Info".into(),
            },
            volume_step: 5,
            seek_step: 5.0,
            map: [(
                "move_down".to_string(),
                vec!["j".to_string(), "down".to_string()],
            )]
            .into_iter()
            .collect(),
        },
    }
}

#[test]
fn command_line_format_matches_tagged_action_names() {
    let c = Command::Select { id: 5 };
    let raw = serde_json::to_string(&c).unwrap();
    assert_eq!(raw, r#"{"action":"select","id":5}"#);

    let c = Command::ToggleTag { tag: "rock".into() };
    let raw = serde_json::to_string(&c).unwrap();
    assert_eq!(raw, r#"{"action":"toggle_tag","tag":"rock"}"#);

    let c = Command::Stop;
    assert_eq!(serde_json::to_string(&c).unwrap(), r#"{"action":"stop"}"#);

    let c = Command::SetRepeat {
        mode: RepeatMode::One,
    };
    assert_eq!(
        serde_json::to_string(&c).unwrap(),
        r#"{"action":"set_repeat","mode":"One"}"#
    );

    let c = Command::SetShuffle { on: false };
    assert_eq!(
        serde_json::to_string(&c).unwrap(),
        r#"{"action":"set_shuffle","on":false}"#
    );

    let c = Command::AddMini { id: 9 };
    assert_eq!(
        serde_json::to_string(&c).unwrap(),
        r#"{"action":"add_mini","id":9}"#
    );
}

#[test]
fn commands_roundtrip_through_json() {
    let cases = vec![
        Command::PlayPause,
        Command::Next,
        Command::Prev,
        Command::Play { id: 3 },
        Command::Select { id: 4 },
        Command::Stop,
        Command::Seek { delta: -7.5 },
        Command::Volume { delta: -5 },
        Command::SetVolume { volume: 0 },
        Command::RepeatCycle,
        Command::SetRepeat {
            mode: RepeatMode::All,
        },
        Command::ShuffleToggle,
        Command::SetShuffle { on: true },
        Command::ToggleTag { tag: "live".into() },
        Command::ClearTags,
        Command::SetSearch { pattern: None },
        Command::Add {
            uri: "/n.mp3".into(),
            title: Some("N".into()),
            tags: vec!["a".into()],
        },
        Command::Update {
            id: 1,
            title: None,
            artist: Some("A".into()),
            tags: vec![],
        },
        Command::Delete { id: 2 },
        Command::AddMini { id: 5 },
        Command::RemoveMini { id: 5 },
        Command::MiniMove { id: 5, delta: -1 },
        Command::ToggleFavorite { id: 6 },
        Command::SetTheme {
            theme: Theme::Light,
        },
        Command::SetTransparency { on: true },
        Command::Shutdown,
    ];
    for c in cases {
        let text = serde_json::to_string(&c).unwrap();
        let back: Command = serde_json::from_str(&text).unwrap();
        assert_eq!(back, c, "roundtrip of {text}");
    }
}

#[test]
fn snapshot_roundtrip_preserves_every_field() {
    let snap = sample_snapshot();
    let text = serde_json::to_string(&snap).unwrap();
    let back: Snapshot = serde_json::from_str(&text).unwrap();
    assert_eq!(back, snap);

    let empty: Snapshot = serde_json::from_str(
        r#"{"theme":"light","transparent":true,"all_media":[],"queue":[],"mini_queue":[],
            "tags":[],"search":null,"selected":null,"now":null,"volume":70,
            "repeat":"Off","shuffle":false,"notify":null,"error":null}"#,
    )
    .unwrap();
    assert_eq!(empty.theme, Theme::Light);
    assert!(empty.transparent);
    assert!(empty.all_media.is_empty());
    assert_eq!(empty.volume, 70);
    assert_eq!(
        empty.config,
        ConfigState::default(),
        "legacy snapshot json gains default config"
    );
}

#[test]
fn theme_and_repeat_names_serialize_as_expected() {
    assert_eq!(
        serde_json::to_value(Theme::Dark).unwrap(),
        serde_json::json!("dark")
    );
    assert_eq!(
        serde_json::to_value(Theme::Light).unwrap(),
        serde_json::json!("light")
    );
    for (mode, name) in [
        (RepeatMode::Off, "Off"),
        (RepeatMode::All, "All"),
        (RepeatMode::One, "One"),
    ] {
        assert_eq!(serde_json::to_value(mode).unwrap(), serde_json::json!(name));
        let back: RepeatMode = serde_json::from_value(serde_json::json!(name)).unwrap();
        assert_eq!(back, mode);
    }
    assert!(
        serde_json::from_str::<RepeatMode>("\"off\"").is_err(),
        "repeat wire values are PascalCase, unlike theme values"
    );
}

#[test]
fn bad_command_json_is_rejected() {
    let res = serde_json::from_str::<Command>(r#"{"action":"frobnicate"}"#);
    assert!(res.is_err());
    let res = serde_json::from_str::<Command>(r#"{"action":"play"}"#);
    assert!(res.is_err(), "play requires id");
    let res = serde_json::from_str::<Command>("not json at all");
    assert!(res.is_err());
}

#[test]
fn favorite_tag_constant_is_load_bearing() {
    let m = sample_media(1);
    assert!(m.tags.contains(&vmp::proto::FAVORITE_TAG.to_string()));
    assert_eq!(vmp::proto::FAVORITE_TAG, "favorite");
}
