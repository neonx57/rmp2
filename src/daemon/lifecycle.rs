use super::Daemon;
use crate::error::Error;
use crate::logging;
use crate::storage::Paths;
use serde_json::json;
use signal_hook::consts::signal::{SIGINT, SIGTERM};
use signal_hook::iterator::Signals;
use std::fs;
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::atomic::Ordering;
use std::thread;

pub fn spawn_daemon() -> Result<(), Error> {
    let exe = std::env::current_exe().map_err(|e| Error::Daemon(e.to_string()))?;
    let paths = Paths::resolve();
    let mut cmd = Command::new(exe);
    cmd.arg("--daemon")
        .env("VMP_CONFIG_DIR", &paths.config_dir)
        .env("VMP_DATA_DIR", &paths.data_dir)
        .env("VMP_STATE_DIR", &paths.state_dir)
        .env("VMP_RUN_DIR", &paths.run_dir)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .process_group(0);
    cmd.spawn()
        .map_err(|e| Error::Daemon(format!("spawn daemon: {e}")))?;
    Ok(())
}

pub(super) fn pid_alive(pid_path: &Path) -> bool {
    let Ok(raw) = fs::read_to_string(pid_path) else {
        return false;
    };
    let Ok(pid) = raw.trim().parse::<u32>() else {
        return false;
    };
    Path::new(&format!("/proc/{pid}")).exists() && pid != std::process::id()
}

pub(super) fn write_pid(paths: &Paths) {
    let _ = fs::write(&paths.pid, std::process::id().to_string());
}

impl Daemon {
    pub(super) fn install_signals(&self) {
        let flag = self.stop_flag.clone();
        let _ = thread::Builder::new().name("sig".into()).spawn(move || {
            if let Ok(mut signals) = Signals::new([SIGINT, SIGTERM]) {
                let _ = signals.wait();
                flag.store(true, Ordering::SeqCst);
            }
        });
    }

    pub(super) fn log(&self, msg: &str) {
        logging::append(&self.paths.log, msg);
    }

    pub(super) fn shutdown(&mut self) {
        self.persist();
        if let Some(m) = self.mpv.as_mut() {
            let _ = m.command(json!(["quit"]));
        }
        let _ = fs::remove_file(&self.paths.sock);
        let _ = fs::remove_file(&self.paths.pid);
    }
}
