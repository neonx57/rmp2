use super::settings::Config;
use crate::error::Error;
use std::path::Path;

impl Config {
    pub fn load(config_path: &Path, lua_dir: &Path) -> Result<Self, Error> {
        if !config_path.exists() {
            return Ok(Config::default());
        }
        let raw = std::fs::read_to_string(config_path)
            .map_err(|e| Error::Config(format!("invalid config: failed to read init.lua: {e}")))?;
        super::lua_host::eval_config(&raw, lua_dir).map_err(Error::Config)
    }
}
