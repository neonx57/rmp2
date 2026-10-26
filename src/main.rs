use std::io::Write;
use std::path::Path;
use std::process::{Child, exit};
use std::thread;
use std::time::Duration;
use vmp::daemon::{Daemon, spawn_daemon};
use vmp::storage::Paths;
use vmp::ui::{Connection, run};

fn main() {
    if std::env::args().any(|a| a == "--daemon") {
        if let Err(e) = Daemon::run() {
            eprintln!("vmp daemon: {e}");
            exit(1);
        }
        return;
    }
    let paths = Paths::resolve();
    let connection = connect_attached(&paths);
    if let Err(e) = run(connection) {
        let _ = writeln!(std::io::stderr(), "vmp: {e}");
    }
}

fn connect_attached(paths: &Paths) -> Connection {
    if let Ok(c) = Connection::connect(&paths.sock) {
        return c;
    }
    let child = match spawn_daemon() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("vmp: {e}");
            exit(1);
        }
    };
    wait_for_daemon(&paths.sock, child, &paths.run_dir.join("daemon-err"))
}

fn wait_for_daemon(sock: &Path, mut child: Child, err_path: &Path) -> Connection {
    let mut connected = None;
    for _ in 0..80 {
        if let Ok(c) = Connection::connect(sock) {
            connected = Some(c);
            break;
        }
        if matches!(child.try_wait(), Ok(Some(_)) | Err(_)) {
            break;
        }
        thread::sleep(Duration::from_millis(50));
    }
    match connected {
        Some(c) => {
            let _ = std::fs::remove_file(err_path);
            c
        }
        None => {
            let text = std::fs::read_to_string(err_path).unwrap_or_default();
            let _ = std::fs::remove_file(err_path);
            let text = text.trim();
            if text.is_empty() {
                eprintln!("vmp: daemon did not start");
            } else {
                eprintln!("{text}");
            }
            exit(1);
        }
    }
}
