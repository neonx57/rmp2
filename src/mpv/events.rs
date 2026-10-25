use serde_json::{Value, json};
use std::io::{BufRead, BufReader};
use std::os::unix::net::UnixStream;
use std::sync::mpsc::Sender;

use super::ipc::Pending;

pub enum MpvMsg {
    Event { name: String, data: Option<Value> },
    Shutdown,
}

pub(super) fn read_loop(reader: BufReader<UnixStream>, tx: Sender<MpvMsg>, pending: Pending) {
    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };
        let msg: Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(_) => continue,
        };
        if let Some(rid) = msg.get("request_id").and_then(|v| v.as_u64()) {
            let data = match msg.get("data") {
                Some(d) if !d.is_null() => Ok(d.clone()),
                _ => match msg.get("error").and_then(|e| e.as_str()) {
                    Some(e) if e != "success" => Err(e.to_string()),
                    _ => Ok(Value::Null),
                },
            };
            if let Some(t) = pending.lock().unwrap().remove(&(rid as u32)) {
                let _ = t.send(data);
            }
        } else if let Some(ev) = msg.get("event").and_then(|v| v.as_str()) {
            let name = msg
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or(ev)
                .to_string();
            let data = if let Some(r) = msg.get("reason").and_then(|v| v.as_str()) {
                Some(json!({ "reason": r, "file_error": msg.get("file_error") }))
            } else if name == "log-message" {
                Some(json!({
                    "prefix": msg.get("prefix"),
                    "level": msg.get("level"),
                    "text": msg.get("text"),
                }))
            } else {
                msg.get("data").cloned().filter(|d| !d.is_null())
            };
            if tx.send(MpvMsg::Event { name, data }).is_err() {
                break;
            }
        }
    }
    let _ = tx.send(MpvMsg::Shutdown);
}
