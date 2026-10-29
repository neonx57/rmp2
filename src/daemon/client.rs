use crate::proto::{Command, Snapshot};
use serde_json::Value;
use std::io::{self, BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

pub struct Connection {
    cmd_tx: Sender<Command>,
    snap_rx: Receiver<Value>,
}

impl Connection {
    pub fn connect(path: &Path) -> io::Result<Connection> {
        let stream = UnixStream::connect(path)?;
        let cmd_stream = stream.try_clone()?;
        let (cmd_tx, cmd_rx) = mpsc::channel();
        thread::spawn(move || writer_loop(cmd_stream, cmd_rx));
        let (snap_tx, snap_rx) = mpsc::channel();
        thread::spawn(move || reader_loop(stream, snap_tx));
        Ok(Connection { cmd_tx, snap_rx })
    }

    #[cfg(test)]
    pub(crate) fn from_channels(cmd_tx: Sender<Command>, snap_rx: Receiver<Value>) -> Connection {
        Connection { cmd_tx, snap_rx }
    }

    pub fn send(&self, cmd: Command) {
        let _ = self.cmd_tx.send(cmd);
    }

    pub fn pull(&self) -> Option<Snapshot> {
        let mut latest = None;
        while let Ok(v) = self.snap_rx.try_recv() {
            latest = Some(v);
        }
        latest.and_then(|v| serde_json::from_value::<Snapshot>(v).ok())
    }
}

fn writer_loop(mut stream: UnixStream, rx: Receiver<Command>) {
    for cmd in rx {
        let mut body = serde_json::to_string(&cmd).unwrap_or_default();
        body.push('\n');
        if stream.write_all(body.as_bytes()).is_err() {
            break;
        }
    }
}

fn reader_loop(stream: UnixStream, snap_tx: Sender<Value>) {
    let reader = BufReader::new(stream);
    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };
        if let Ok(v) = serde_json::from_str::<Value>(&line)
            && snap_tx.send(v).is_err()
        {
            break;
        }
    }
}
