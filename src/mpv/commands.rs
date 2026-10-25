use super::Mpv;
use serde_json::{Value, json};
use std::time::Duration;

impl Mpv {
    pub fn observe(&mut self, prop_id: u32, name: &str) {
        let _ = self.command(json!(["observe_property", prop_id, name]));
    }

    pub fn command(&mut self, cmd: Value) -> Result<Value, String> {
        self.call(cmd, Duration::from_secs(5))
    }

    pub fn loadfile(&mut self, uri: &str) -> Result<(), String> {
        self.command(json!(["loadfile", uri, "replace"]))
            .map(|_| ())
    }

    pub fn set_pause(&mut self, paused: bool) -> Result<(), String> {
        self.command(json!(["set_property", "pause", paused]))
            .map(|_| ())
    }

    pub fn seek(&mut self, seconds: f64, absolute: bool) -> Result<(), String> {
        if absolute {
            self.command(json!(["seek", seconds, "absolute"]))
                .map(|_| ())
        } else {
            self.command(json!(["seek", seconds, "relative"]))
                .map(|_| ())
        }
    }

    pub fn volume(&mut self, value: i32) -> Result<(), String> {
        self.command(json!(["set_property", "volume", value]))
            .map(|_| ())
    }

    pub fn property(&mut self, name: &str) -> Result<Value, String> {
        self.command(json!(["get_property", name]))
    }
}
