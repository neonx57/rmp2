use super::Mpv;
use serde_json::{Value, json};
use std::io::Write;
use std::sync::atomic::Ordering;
use std::sync::mpsc;
use std::time::Duration;

pub(super) type Pending = std::sync::Arc<
    std::sync::Mutex<std::collections::HashMap<u32, mpsc::Sender<Result<Value, String>>>>,
>;

impl Mpv {
    pub(super) fn next_id(&self) -> u32 {
        self.next_id.fetch_add(1, Ordering::SeqCst)
    }

    pub fn call(&mut self, cmd: Value, wait: Duration) -> Result<Value, String> {
        let id = self.next_id();
        let (tx, rx) = mpsc::channel();
        self.pending.lock().unwrap().insert(id, tx);
        let body = json!({ "command": cmd, "request_id": id }).to_string();
        let res = self
            .sock
            .write_all(body.as_bytes())
            .and_then(|_| self.sock.write_all(b"\n"))
            .and_then(|_| self.sock.flush());
        if res.is_err() {
            self.pending.lock().unwrap().remove(&id);
        }
        res.map_err(|e| e.to_string())?;
        match rx.recv_timeout(wait) {
            Ok(Ok(v)) => Ok(v),
            Ok(Err(e)) => Err(e),
            Err(_) => Err("mpv timeout".into()),
        }
    }
}
