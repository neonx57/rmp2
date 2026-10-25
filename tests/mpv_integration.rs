mod common;

use common::{MPV_TEST_SECONDS, wav_bytes};
use rmp::mpv::{Mpv, MpvMsg, live_info, probe_metadata};
use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};

fn temp_root() -> PathBuf {
    let dir = std::env::temp_dir().join(format!("rmp2-mpv-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    dir
}

fn mpv_available() -> bool {
    Command::new("mpv").arg("--version").output().is_ok()
}

fn fixture() -> (PathBuf, PathBuf) {
    let root = temp_root();
    let wav = root.join("tone.wav");
    std::fs::write(&wav, wav_bytes(8_000, MPV_TEST_SECONDS)).unwrap();
    (root, wav)
}

#[test]
fn mpv_is_present_or_self_skipped() {
    if mpv_available() {
        println!("mpv found: integration playback checks run");
    } else {
        println!("mpv missing: playback checks skipped");
    }
}

#[test]
fn probe_metadata_reports_wav_duration_and_name() {
    if !mpv_available() {
        eprintln!("skip: mpv not available");
        return;
    }
    let (root, wav) = fixture();
    let info = probe_metadata("mpv", &root.join("probe.sock"), &wav.to_string_lossy());
    let (title, _artist, duration, _bitrate) = info.expect("mpv should probe a valid wav file");
    assert_eq!(
        title.as_deref(),
        Some("tone"),
        "media-title defaults to stem"
    );
    let d = duration.expect("duration available for decoded file");
    assert!(
        (MPV_TEST_SECONDS - 0.2..MPV_TEST_SECONDS + 0.2).contains(&d),
        "duration {d} must be near {MPV_TEST_SECONDS}s"
    );
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn interactive_playback_session_controls_the_wav() {
    if !mpv_available() {
        eprintln!("skip: mpv not available");
        return;
    }
    let (root, wav) = fixture();
    let ipc = root.join("playback.sock");
    let (mut mpv, rx) = Mpv::spawn("mpv", &ipc, false).expect("mpv spawn with a valid binary");

    let loaded = Arc::new(AtomicBool::new(false));
    let drainer_flag = loaded.clone();
    let drainer = thread::spawn(move || {
        loop {
            match rx.recv_timeout(Duration::from_millis(50)) {
                Ok(MpvMsg::Event { name, .. }) if name == "file-loaded" => {
                    drainer_flag.store(true, Ordering::SeqCst);
                }
                Ok(MpvMsg::Shutdown) | Err(_) => break,
                Ok(_) => {}
            }
        }
    });

    mpv.loadfile(wav.to_string_lossy().as_ref()).unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    while !loaded.load(Ordering::SeqCst) {
        assert!(
            Instant::now() < deadline,
            "mpv never reported file-loaded for the wav fixture"
        );
        thread::sleep(Duration::from_millis(20));
    }

    let (title, _artist, duration, _bitrate) = live_info(&mut mpv);
    assert_eq!(title.as_deref(), Some("tone"));
    let d = duration.expect("duration numeric after file-loaded");
    assert!((MPV_TEST_SECONDS - 0.2..MPV_TEST_SECONDS + 0.2).contains(&d));

    mpv.set_pause(true).unwrap();
    assert_eq!(
        mpv.property("pause").unwrap().as_bool(),
        Some(true),
        "pause roundtrips through the ipc client"
    );

    let _ = mpv.command(serde_json::json!(["quit"]));
    drainer.join().expect("drainer exits on Shutdown");
    let _ = std::fs::remove_dir_all(root);
}
