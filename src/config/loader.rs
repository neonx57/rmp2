use super::settings::Config;
use crate::app::action::action_from_str;
use crate::config::defaults::{binding_text, default_bindings, default_config_text};
use crate::config::validation::is_valid_key;
use crate::error::Error;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

impl Config {
    pub fn load(path: &Path) -> Result<Self, Error> {
        let raw = match fs::read_to_string(path) {
            Ok(s) => s,
            Err(_) => {
                let text = default_config_text();
                let _ = fs::write(path, text);
                return Ok(Config::default());
            }
        };
        if let Some(migrated) = migrate_legacy(&raw) {
            let _ = fs::write(path, &migrated);
            return Self::parse(&migrated).map_err(Error::Config);
        }
        Self::parse(&raw).map_err(Error::Config)
    }
}

pub(super) fn migrate_legacy(raw: &str) -> Option<String> {
    let top: BTreeMap<String, toml::Value> = toml::from_str(raw).ok()?;
    let table = top.get("keybindings")?.as_table()?;
    if table.keys().any(|k| action_from_str(k).is_some()) {
        return None;
    }
    let legacy = table.iter().any(|(k, v)| {
        is_valid_key(k)
            && action_from_str(k).is_none()
            && v.as_str().is_some_and(|s| action_from_str(s).is_some())
    });
    if !legacy {
        return None;
    }
    let mut out = String::new();
    out.push_str("# rmp2 configuration\n");
    out.push_str("# Auto-migrated from the legacy \"key = action\" format.\n\n");
    out.push_str("mpv_binary = ");
    out.push_str(
        &top.get("mpv_binary")
            .map(|v| v.to_string())
            .unwrap_or_else(|| "\"mpv\"".into()),
    );
    out.push('\n');
    out.push_str("volume_step = ");
    out.push_str(
        &top.get("volume_step")
            .map(|v| v.to_string())
            .unwrap_or_else(|| "5".into()),
    );
    out.push('\n');
    out.push_str("seek_step = ");
    out.push_str(
        &top.get("seek_step")
            .map(|v| v.to_string())
            .unwrap_or_else(|| "5.0".into()),
    );
    out.push('\n');
    out.push_str("[keybindings]\n");
    let mut merged: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let excluded: std::collections::HashSet<String> = table
        .iter()
        .filter(|(_, v)| matches!(v.as_str(), Some("none")))
        .map(|(k, _)| k.clone())
        .collect();
    for (k, v) in table {
        if let Some(action) = v.as_str()
            && action != "none"
            && !excluded.contains(k)
        {
            merged
                .entry(action.to_string())
                .or_default()
                .push(k.clone());
        }
    }
    for (action, default_keys) in default_bindings() {
        let entry = merged.entry(action.clone()).or_default();
        for dk in default_keys {
            if !excluded.contains(&dk) && !entry.contains(&dk) {
                entry.push(dk.clone());
            }
        }
    }
    for keys in merged.values_mut() {
        keys.sort();
    }
    for (action, keys) in merged {
        out.push_str(&binding_text(&action, &keys));
    }
    Some(out)
}
