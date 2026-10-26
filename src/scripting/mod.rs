pub mod host;

pub use host::eval_config;

use std::fmt;

pub struct ScriptingError(String);

impl fmt::Display for ScriptingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl ScriptingError {
    pub(crate) fn new(message: String) -> Self {
        ScriptingError(message)
    }
}

impl From<String> for ScriptingError {
    fn from(message: String) -> Self {
        ScriptingError(message)
    }
}
