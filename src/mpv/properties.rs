use super::Mpv;
use super::events::MpvMsg;
use serde_json::Value;
use std::path::Path;
use std::thread;
use std::time::{Duration, Instant};

pub type LiveInfo = (Option<String>, Option<String>, Option<f64>, Option<u64>);

pub fn probe_metadata(binary: &str, ipc_path: &Path, uri: &str) -> Option<LiveInfo> {
    let (mut mpv, rx) = Mpv::spawn(binary, ipc_path, true).ok()?;
    mpv.loadfile(uri).ok()?;
    let deadline = Instant::now() + Duration::from_secs(6);
    let mut loaded = false;
    while Instant::now() < deadline {
        match rx.recv_timeout(Duration::from_millis(100)) {
            Ok(MpvMsg::Event { name, .. }) if name == "file-loaded" => {
                loaded = true;
                break;
            }
            Ok(_) => {}
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => continue,
            Err(_) => break,
        }
    }
    if !loaded {
        return None;
    }
    thread::sleep(Duration::from_millis(300));
    Some(live_info(&mut mpv))
}

pub fn live_info(mpv: &mut Mpv) -> LiveInfo {
    let title =
        metadata_string(mpv, "media-title").or_else(|| metadata_string(mpv, "metadata/Title"));
    let artist = metadata_string(mpv, "metadata/Artist");
    let duration = mpv
        .property("duration")
        .ok()
        .and_then(|v| v.as_f64())
        .filter(|d| *d > 0.0);
    let bitrate = mpv
        .property("audio-params/bitrate")
        .ok()
        .and_then(|v| v.as_u64());
    (title, artist, duration, bitrate)
}

fn metadata_string(mpv: &mut Mpv, name: &str) -> Option<String> {
    mpv.property(name).ok().and_then(|v| match &v {
        Value::String(s) => Some(s.clone()),
        Value::Null => None,
        _ => v.as_str().map(|s| s.to_string()),
    })
}
