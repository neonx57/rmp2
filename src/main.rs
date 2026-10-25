use std::io::Write;
use std::process::exit;
use std::thread;
use std::time::Duration;
use vmp::config::Config;
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
    let cfg = match Config::load(&paths.config, &paths.lua_dir()) {
        Ok(c) => c,
        Err(e) => {
            let _ = writeln!(std::io::stderr(), "vmp: {e}");
            exit(1);
        }
    };
    let connection = match Connection::connect(&paths.sock) {
        Ok(c) => c,
        Err(_) => {
            if let Err(e) = spawn_daemon() {
                eprintln!("vmp: {e}");
                exit(1);
            }
            let mut connected = None;
            for _ in 0..80 {
                thread::sleep(Duration::from_millis(50));
                if let Ok(c) = Connection::connect(&paths.sock) {
                    connected = Some(c);
                    break;
                }
            }
            match connected {
                Some(c) => c,
                None => {
                    eprintln!("vmp: daemon did not start");
                    exit(1);
                }
            }
        }
    };
    if let Err(e) = run(connection, cfg) {
        let _ = writeln!(std::io::stderr(), "vmp: {e}");
    }
}
