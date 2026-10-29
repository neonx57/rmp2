mod common;

use std::fs;
use std::path::PathBuf;
use std::process::Command as Process;
use std::thread;
use std::time::{Duration, Instant};
use vmp::config::Config;
use vmp::daemon::{Daemon, client::Connection};
use vmp::error::Error;
use vmp::proto::{Command, RepeatMode, Snapshot, Theme};
use vmp::storage::Paths;

fn temp_root(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("vmp-daemon-{}-{}", tag, std::process::id()));
    let _ = fs::create_dir_all(&dir);
    dir
}

fn connect_retry(sock: &PathBuf) -> Connection {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if let Ok(c) = Connection::connect(sock) {
            return c;
        }
        assert!(
            Instant::now() < deadline,
            "daemon socket never appeared at {sock:?}"
        );
        thread::sleep(Duration::from_millis(10));
    }
}

fn wait_snapshot(
    conn: &Connection,
    timeout: Duration,
    want: impl Fn(&Snapshot) -> bool,
) -> Snapshot {
    let deadline = Instant::now() + timeout;
    loop {
        if let Some(s) = conn.pull()
            && want(&s)
        {
            return s;
        }
        assert!(
            Instant::now() < deadline,
            "snapshot condition not met within {timeout:?}"
        );
        thread::sleep(Duration::from_millis(10));
    }
}

fn media_id_by_title(snap: &Snapshot, title: &str) -> Option<i64> {
    snap.all_media
        .iter()
        .find(|m| m.title.as_deref() == Some(title))
        .map(|m| m.id)
}

#[test]
fn daemon_serves_snapshots_and_shuts_down() {
    let root = temp_root("e2e");
    let paths = Paths::for_root(root.clone());
    let sock = paths.sock.clone();
    let state_path = paths.state.clone();

    let handle = thread::Builder::new()
        .name("daemon-e2e".into())
        .spawn(move || Daemon::serve(paths, Config::default()))
        .expect("spawn daemon thread");

    let conn = connect_retry(&sock);

    let snap = wait_snapshot(&conn, Duration::from_secs(2), |_| true);
    assert_eq!(snap.theme, Theme::Dark, "fresh daemon state");
    assert_eq!(snap.volume, 70);
    assert_eq!(snap.repeat, RepeatMode::Off);
    assert!(!snap.shuffle);
    assert!(snap.all_media.is_empty());
    assert!(snap.now.is_none());
    assert!(snap.error.is_none());

    fs::write(
        root.join("tone.wav"),
        common::wav_bytes(8_000, common::MPV_TEST_SECONDS),
    )
    .unwrap();
    let wav = root.join("tone.wav");
    conn.send(Command::Add {
        uri: wav.to_string_lossy().into_owned(),
        title: Some("Take".into()),
        tags: vec!["unit".into()],
    });
    let id = wait_snapshot(&conn, Duration::from_secs(10), |s| {
        media_id_by_title(s, "Take").is_some()
    })
    .all_media[0]
        .id;
    assert_eq!(
        id, 1,
        "fresh db assigns autoincrement id 1 to the first item"
    );

    conn.send(Command::Select { id });
    let snap = wait_snapshot(&conn, Duration::from_secs(2), |s| {
        s.selected.as_ref().is_some_and(|m| m.id == id)
    });
    let sel = snap.selected.unwrap();
    assert_eq!(sel.uri, wav.to_string_lossy());
    assert_eq!(sel.kind, "offline");
    assert_eq!(sel.tags, vec!["unit".to_string()]);

    conn.send(Command::AddMini { id });
    let _ = wait_snapshot(&conn, Duration::from_secs(2), |s| s.mini_queue == vec![id]);

    conn.send(Command::RemoveMini { id });
    let _ = wait_snapshot(&conn, Duration::from_secs(2), |s| s.mini_queue.is_empty());

    conn.send(Command::ToggleTag { tag: "unit".into() });
    let _ = wait_snapshot(&conn, Duration::from_secs(2), |s| {
        s.tags
            .iter()
            .any(|t| t.name == "unit" && t.checked && t.count == 1)
    });

    conn.send(Command::SetVolume { volume: 88 });
    let _ = wait_snapshot(&conn, Duration::from_secs(2), |s| s.volume == 88);

    conn.send(Command::SetTheme {
        theme: Theme::Light,
    });
    let _ = wait_snapshot(&conn, Duration::from_secs(2), |s| s.theme == Theme::Light);

    conn.send(Command::SetSearch {
        pattern: Some("take".into()),
    });
    let snap = wait_snapshot(&conn, Duration::from_secs(2), |s| {
        s.search.as_deref() == Some("take")
    });

    let before = snap.queue.len();
    assert_eq!(before, 1, "tag filter + search keep the wav visible");
    assert_eq!(snap.queue, vec![id]);

    conn.send(Command::ShuffleToggle);
    let _ = wait_snapshot(&conn, Duration::from_secs(2), |s| s.shuffle);

    conn.send(Command::RepeatCycle);
    let _ = wait_snapshot(&conn, Duration::from_secs(2), |s| {
        s.repeat == RepeatMode::All
    });

    conn.send(Command::SetRepeat {
        mode: RepeatMode::One,
    });
    let _ = wait_snapshot(&conn, Duration::from_secs(2), |s| {
        s.repeat == RepeatMode::One
    });

    conn.send(Command::SetShuffle { on: false });
    let _ = wait_snapshot(&conn, Duration::from_secs(2), |s| !s.shuffle);

    conn.send(Command::Stop);
    let _ = wait_snapshot(&conn, Duration::from_secs(2), |s| {
        s.notify.as_deref() == Some("stopped")
    });

    let persisted = fs::read_to_string(state_path).unwrap();
    assert!(persisted.contains("\"theme\":\"light\""));
    assert!(persisted.contains("\"volume\":88"));
    assert!(
        persisted.contains("\"repeat\":\"One\""),
        "set_repeat persists the mode it set"
    );

    conn.send(Command::Shutdown);
    let daemon_result = handle.join().expect("daemon thread exits cleanly");
    assert!(daemon_result.is_ok(), "daemon serve returns without error");
    assert!(!sock.exists(), "daemon removes its socket on shutdown");
}

#[test]
fn foreign_live_pid_blocks_a_second_daemon() {
    let root = temp_root("cross");
    let paths_for_env = Paths::for_root(root.clone());
    let sock = paths_for_env.sock.clone();
    let mut child = Process::new(env!("CARGO_BIN_EXE_vmp"))
        .arg("--daemon")
        .env("VMP_CONFIG_DIR", &paths_for_env.config_dir)
        .env("VMP_DATA_DIR", &paths_for_env.data_dir)
        .env("VMP_STATE_DIR", &paths_for_env.state_dir)
        .env("VMP_RUN_DIR", &paths_for_env.run_dir)
        .stdin(std::process::Stdio::null())
        .spawn()
        .expect("spawn detached vmp daemon from the built binary");

    let conn = connect_retry(&sock);

    let res = Daemon::serve(Paths::for_root(root.clone()), Config::default());
    let err = res.expect_err("second bind against a live foreign daemon must fail");
    match err {
        Error::Daemon(msg) => assert_eq!(msg, "daemon already running"),
        other => panic!("unexpected error kind: {other:?}"),
    }

    conn.send(Command::SetVolume { volume: 5 });
    let _ = wait_snapshot(&conn, Duration::from_secs(2), |s| s.volume == 5);

    conn.send(Command::Shutdown);
    let _ = conn;
    assert_exit_success(&mut child);
    assert!(!sock.exists(), "subprocess daemon removed its socket");
    assert!(!paths_for_env.pid.exists());
    let _ = fs::remove_dir_all(root);
}

#[test]
fn spawned_daemon_delivers_config_over_ipc() {
    let root = temp_root("ipc-cfg");
    let dirs = Paths::for_root(root.clone());
    fs::write(
        &dirs.config,
        "vmp.opt.volume_step = 9\nvmp.opt.titles.mini = \"Picked\"\n",
    )
    .unwrap();

    let mut child = Process::new(env!("CARGO_BIN_EXE_vmp"))
        .arg("--daemon")
        .env("VMP_CONFIG_DIR", &dirs.config_dir)
        .env("VMP_DATA_DIR", &dirs.data_dir)
        .env("VMP_STATE_DIR", &dirs.state_dir)
        .env("VMP_RUN_DIR", &dirs.run_dir)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .expect("spawn detached vmp daemon from the built binary");

    let conn = connect_retry(&dirs.sock);
    let snap = wait_snapshot(&conn, Duration::from_secs(2), |_| true);
    assert_eq!(
        snap.config.volume_step, 9,
        "daemon-effective opt values ride every snapshot"
    );
    assert_eq!(snap.config.titles.mini, "Picked");
    assert_eq!(
        snap.config.map.get("add_mini").map(|v| v.as_slice()),
        Some(&["A".to_string()][..]),
        "default keybindings ride the snapshot for TUI clients"
    );

    conn.send(Command::Shutdown);
    assert_exit_success(&mut child);
    assert!(!dirs.sock.exists());
    let _ = fs::remove_dir_all(root);
}

#[test]
fn config_from_real_init_lua_rides_first_snapshot() {
    let root = temp_root("cfgval");
    let paths = Paths::for_root(root.clone());
    let sock = paths.sock.clone();
    fs::write(
        &paths.config,
        "vmp.opt.titles.queue = \"Mix\"\nvmp.opt.titles.info = \"About\"\nvmp.map.add_mini = \"Z\"\n",
    )
    .unwrap();

    let cfg = Config::load(&paths.config, &paths.lua_dir()).expect("init.lua loads");
    assert_eq!(cfg.titles.queue, "Mix");
    assert_eq!(cfg.titles.info, "About");
    assert_eq!(cfg.titles.filter, "Filter", "unset titles keep defaults");
    assert_eq!(
        cfg.keybindings.get("add_mini").map(|v| v.as_slice()),
        Some(&["Z".to_string()][..])
    );
    assert_eq!(cfg.volume_step, 5, "unset opts keep defaults");
    let want_map = cfg.keybindings.clone();

    let handle = thread::Builder::new()
        .name("daemon-cfg".into())
        .spawn(move || Daemon::serve(paths, cfg))
        .expect("spawn daemon thread");
    let conn = connect_retry(&sock);
    let snap = wait_snapshot(&conn, Duration::from_secs(2), |_| true);

    assert_eq!(snap.config.titles.queue, "Mix");
    assert_eq!(snap.config.titles.info, "About");
    assert_eq!(snap.config.titles.filter, "Filter");
    assert_eq!(snap.config.volume_step, 5);
    assert_eq!(snap.config.mpv_binary, "mpv");
    assert_eq!(
        snap.config.map, want_map,
        "merged default + user keybindings are delivered verbatim"
    );

    conn.send(Command::Shutdown);
    let daemon_result = handle.join().expect("daemon thread exits cleanly");
    assert!(daemon_result.is_ok(), "daemon serve returns without error");
    assert!(!sock.exists(), "daemon removes its socket on shutdown");
    let _ = fs::remove_dir_all(root);
}

fn assert_exit_success(child: &mut std::process::Child) {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                assert!(status.success(), "daemon exits zero after shutdown");
                break;
            }
            Ok(None) => assert!(
                Instant::now() < deadline,
                "daemon subprocess did not exit after Shutdown"
            ),
            Err(e) => panic!("wait failed: {e}"),
        }
        thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn invalid_config_error_is_relayed_from_spawned_daemon() {
    let root = temp_root("relay");
    let dirs = Paths::for_root(root.clone());
    fs::write(dirs.config, "vmp.opt.not_real = 1\n").unwrap();

    let out = Process::new(env!("CARGO_BIN_EXE_vmp"))
        .env("VMP_CONFIG_DIR", &dirs.config_dir)
        .env("VMP_DATA_DIR", &dirs.data_dir)
        .env("VMP_STATE_DIR", &dirs.state_dir)
        .env("VMP_RUN_DIR", &dirs.run_dir)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .output()
        .expect("run vmp with a broken init.lua");

    assert_eq!(out.status.code(), Some(1), "config error exits nonzero");
    let msg = String::from_utf8_lossy(&out.stderr);
    assert!(
        msg.contains("vmp daemon: invalid config:"),
        "TUI relays the daemon's own stderr: {msg}"
    );
    assert!(msg.contains("not_real"), "{msg}");
    assert!(!dirs.sock.exists(), "no daemon socket may exist");
    assert!(
        !dirs.run_dir.join("daemon-err").exists(),
        "daemon-err is cleaned up after the relay"
    );
    let _ = fs::remove_dir_all(root);
}
