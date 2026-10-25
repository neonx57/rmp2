use std::env;
use std::fs;
use std::path::PathBuf;

pub fn config_dir() -> PathBuf {
    if let Some(dir) = env::var_os("RMP2_DIR") {
        return PathBuf::from(dir);
    }
    let base = match env::var_os("XDG_CONFIG_HOME") {
        Some(p) => PathBuf::from(p),
        None => match env::var_os("HOME") {
            Some(h) => PathBuf::from(h).join(".config"),
            None => PathBuf::from("."),
        },
    };
    base.join("rmp2")
}

pub struct Paths {
    pub root: PathBuf,
    pub config: PathBuf,
    pub db: PathBuf,
    pub sock: PathBuf,
    pub pid: PathBuf,
    pub state: PathBuf,
    pub log: PathBuf,
}

impl Paths {
    pub fn resolve() -> Self {
        let root = config_dir();
        let _ = fs::create_dir_all(&root);
        Self::for_root(root)
    }

    pub fn for_root(root: PathBuf) -> Self {
        Self {
            config: root.join("config.toml"),
            db: root.join("library.sqlite3"),
            sock: root.join("rmp.sock"),
            pid: root.join("rmp.pid"),
            state: root.join("last-state.json"),
            log: root.join("rmp.log"),
            root,
        }
    }
}
