use super::settings::Config;
use crate::app::action::action_from_str;
use crate::config::defaults::default_bindings;
use std::collections::HashMap;

fn valid_plain_key(k: &str) -> bool {
    matches!(
        k,
        "space"
            | "enter"
            | "esc"
            | "tab"
            | "backtab"
            | "up"
            | "down"
            | "left"
            | "right"
            | "home"
            | "end"
            | "insert"
            | "delete"
            | "backspace"
            | "pageup"
            | "pagedown"
    )
}

pub(super) fn is_valid_key(k: &str) -> bool {
    if valid_plain_key(k) {
        return true;
    }
    if k.len() == 1 && k.chars().all(|c| c.is_ascii_graphic()) {
        return true;
    }
    if let Some(r) = k.strip_prefix("ctrl+") {
        return r.len() == 1 && r.chars().all(|c| c.is_ascii_graphic());
    }
    if let Some(f) = k.strip_prefix('f')
        && let Ok(n) = f.parse::<u8>()
    {
        return (1..=24).contains(&n);
    }
    false
}

impl Config {
    pub fn parse(raw: &str) -> Result<Self, String> {
        let mut cfg: Config =
            toml::from_str(raw).map_err(|e| format!("config parse error: {e}"))?;
        for (action, keys) in default_bindings() {
            cfg.keybindings.entry(action).or_insert(keys);
        }
        let mut errors = Vec::new();
        let mut seen: HashMap<&str, &str> = HashMap::new();
        for (action, keys) in &cfg.keybindings {
            if action_from_str(action).is_none() {
                errors.push(format!("invalid action \"{action}\""));
                continue;
            }
            for k in keys {
                if k == "none" {
                    continue;
                }
                if !is_valid_key(k) {
                    errors.push(format!("invalid key \"{k}\" for action \"{action}\""));
                }
                if let Some(prev) = seen.insert(k, action.as_str())
                    && prev != action
                {
                    errors.push(format!(
                        "key \"{k}\" bound to multiple actions (\"{prev}\", \"{action}\")"
                    ));
                }
            }
        }
        if errors.is_empty() {
            Ok(cfg)
        } else {
            Err(format!("invalid config:\n{}", errors.join("\n")))
        }
    }
}
