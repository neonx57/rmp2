use serde::Deserialize;
use std::collections::BTreeMap;

mod keybindings {
    use serde::{Deserialize, Deserializer};
    use std::collections::BTreeMap;

    #[derive(Deserialize)]
    #[serde(untagged)]
    enum KeySpec {
        One(String),
        Many(Vec<String>),
    }

    pub fn deserialize<'de, D>(d: D) -> Result<BTreeMap<String, Vec<String>>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw: BTreeMap<String, KeySpec> = BTreeMap::deserialize(d)?;
        Ok(raw
            .into_iter()
            .map(|(action, spec)| {
                let keys = match spec {
                    KeySpec::One(s) => vec![s],
                    KeySpec::Many(v) => v,
                };
                (action, keys)
            })
            .collect())
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
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

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Config {
    pub mpv_binary: String,
    pub volume_step: i32,
    pub seek_step: f64,
    pub titles: Titles,
    #[serde(with = "keybindings")]
    pub keybindings: BTreeMap<String, Vec<String>>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            mpv_binary: "mpv".into(),
            volume_step: 5,
            seek_step: 5.0,
            titles: Titles::default(),
            keybindings: crate::config::defaults::default_bindings(),
        }
    }
}
