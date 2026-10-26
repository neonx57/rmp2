use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Titles {
    pub filter: String,
    pub queue: String,
    pub mini: String,
    pub info: String,
}

impl Default for Titles {
    fn default() -> Self {
        Titles {
            filter: "Filter".into(),
            queue: "Queue".into(),
            mini: "Mini Queue".into(),
            info: "Info".into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Config {
    pub mpv_binary: String,
    pub volume_step: i32,
    pub seek_step: f64,
    pub enter_resume_loaded: bool,
    pub titles: Titles,
    pub keybindings: BTreeMap<String, Vec<String>>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            mpv_binary: "mpv".into(),
            volume_step: 5,
            seek_step: 5.0,
            enter_resume_loaded: true,
            titles: Titles::default(),
            keybindings: crate::config::defaults::default_bindings(),
        }
    }
}
