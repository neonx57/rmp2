use super::Mpv;
use super::events::{MpvMsg, read_loop};
use super::ipc::Pending;
use std::collections::HashMap;
use std::fs;
use std::io::BufReader;
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::atomic::AtomicU32;
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

impl Mpv {
    pub fn spawn(
        binary: &str,
        ipc_path: &Path,
        probe: bool,
    ) -> Result<(Mpv, Receiver<MpvMsg>), String> {
        let _ = fs::remove_file(ipc_path);
        let mut cmd = Command::new(binary);
        cmd.arg("--idle=yes")
            .arg("--no-terminal")
            .arg("--quiet")
            .arg("--no-video")
            .arg("--audio-display=no")
            .arg(format!("--input-ipc-server={}", ipc_path.to_string_lossy()))
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        if probe {
            cmd.arg("--no-config")
                .arg("--load-scripts=no")
                .arg("--ao=null");
        } else {
            cmd.arg("--volume=70");
        }
        let mut child = cmd.spawn().map_err(|e| format!("spawn mpv: {e}"))?;
        let sock = match connect_retry(ipc_path, 40, Duration::from_millis(50)) {
            Ok(s) => s,
            Err(e) => {
                let _ = child.kill();
                return Err(format!("connect mpv ipc: {e}"));
            }
        };
        let reader = BufReader::new(sock.try_clone().map_err(|e| e.to_string())?);
        let pending: Pending = Arc::new(Mutex::new(HashMap::new()));
        let (tx, rx) = mpsc::channel();
        let td = pending.clone();
        thread::spawn(move || read_loop(reader, tx, td));
        Ok((
            Mpv {
                sock,
                next_id: AtomicU32::new(0),
                pending,
                child,
                ipc_path: ipc_path.to_string_lossy().into_owned(),
            },
            rx,
        ))
    }
}

impl Drop for Mpv {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = fs::remove_file(&self.ipc_path);
    }
}

fn connect_retry(path: &Path, attempts: usize, delay: Duration) -> Result<UnixStream, String> {
    let mut last = "no attempt".to_string();
    for _ in 0..attempts {
        match UnixStream::connect(path) {
            Ok(s) => return Ok(s),
            Err(e) => {
                last = e.to_string();
                thread::sleep(delay);
            }
        }
    }
    Err(last)
}
