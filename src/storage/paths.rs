use std::env;
use std::fs;
use std::path::PathBuf;

fn home_dir(fallback: &str) -> PathBuf {
    match env::var_os("HOME") {
        Some(h) if !h.is_empty() => PathBuf::from(h).join(fallback),
        _ => PathBuf::from("."),
    }
}

fn xdg_dir(var: &str, home_fallback: &str) -> PathBuf {
    match env::var_os(var) {
        Some(dir) if !dir.is_empty() => PathBuf::from(dir),
        _ => home_dir(home_fallback),
    }
}

pub struct Paths {
    pub config_dir: PathBuf,
    pub data_dir: PathBuf,
    pub state_dir: PathBuf,
    pub run_dir: PathBuf,
    pub config: PathBuf,
    pub db: PathBuf,
    pub state: PathBuf,
    pub log: PathBuf,
    pub sock: PathBuf,
    pub pid: PathBuf,
    pub mpv_sock: PathBuf,
}

impl Paths {
    pub fn resolve() -> Self {
        let config_dir = match env::var_os("VMP_CONFIG_DIR") {
            Some(dir) if !dir.is_empty() => PathBuf::from(dir),
            _ => xdg_dir("XDG_CONFIG_HOME", ".config").join("vmp"),
        };
        let data_dir = match env::var_os("VMP_DATA_DIR") {
            Some(dir) if !dir.is_empty() => PathBuf::from(dir),
            _ => xdg_dir("XDG_DATA_HOME", ".local/share").join("vmp"),
        };
        let state_dir = match env::var_os("VMP_STATE_DIR") {
            Some(dir) if !dir.is_empty() => PathBuf::from(dir),
            _ => xdg_dir("XDG_STATE_HOME", ".local/state").join("vmp"),
        };
        let run_dir = match env::var_os("VMP_RUN_DIR") {
            Some(dir) if !dir.is_empty() => PathBuf::from(dir),
            _ => match env::var_os("XDG_RUNTIME_DIR") {
                Some(dir) if !dir.is_empty() => PathBuf::from(dir).join("vmp"),
                _ => home_dir(".cache/vmp"),
            },
        };
        Self::for_dirs(config_dir, data_dir, state_dir, run_dir)
    }

    pub fn for_root(root: PathBuf) -> Self {
        Self::for_dirs(
            root.join("config"),
            root.join("share"),
            root.join("state"),
            root.join("run"),
        )
    }

    pub fn for_dirs(
        config_dir: PathBuf,
        data_dir: PathBuf,
        state_dir: PathBuf,
        run_dir: PathBuf,
    ) -> Self {
        let _ = fs::create_dir_all(&config_dir);
        let _ = fs::create_dir_all(&data_dir);
        let _ = fs::create_dir_all(&state_dir);
        let _ = fs::create_dir_all(&run_dir);
        let config = config_dir.join("init.lua");
        let db = data_dir.join("library.sqlite3");
        let state = state_dir.join("session.json");
        let log = state_dir.join("vmp.log");
        let sock = run_dir.join("vmp.sock");
        let pid = run_dir.join("vmp.pid");
        let mpv_sock = run_dir.join("mpv.sock");
        Self {
            config_dir,
            data_dir,
            state_dir,
            run_dir,
            config,
            db,
            state,
            log,
            sock,
            pid,
            mpv_sock,
        }
    }

    pub fn lua_dir(&self) -> PathBuf {
        self.config_dir.join("lua")
    }

    pub fn probe_sock(&self, pid: u32) -> PathBuf {
        self.run_dir.join(format!("probe-{pid}.sock"))
    }
}
