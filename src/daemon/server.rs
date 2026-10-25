use super::{Daemon, LoopEvent};
use crate::proto::Command;
use std::io::ErrorKind;
use std::io::{BufRead, BufReader, Write};
use std::net::Shutdown;
use std::os::unix::net::UnixStream;
use std::sync::mpsc::{Receiver, Sender};
use std::thread;

impl Daemon {
    pub(super) fn drain_accepts(&mut self) {
        loop {
            match self.listener.accept() {
                Ok((stream, _)) => {
                    let id = self.next_client;
                    self.next_client += 1;
                    self.register_client(id, stream);
                }
                Err(ref e) if e.kind() == ErrorKind::WouldBlock => break,
                Err(e) => {
                    self.log(&format!("accept error: {e}"));
                    break;
                }
            }
        }
    }

    pub(super) fn register_client(&mut self, id: u64, stream: UnixStream) {
        let (tx, rx) = std::sync::mpsc::channel();
        self.clients.insert(id, tx.clone());
        let reader = match stream.try_clone() {
            Ok(s) => s,
            Err(e) => {
                self.log(&format!("clone client: {e}"));
                self.clients.remove(&id);
                return;
            }
        };
        let e = self.events_tx.clone();
        thread::spawn(move || client_reader(reader, id, e));
        thread::spawn(move || client_writer(stream, rx));
        let text = self.snapshot_text();
        let _ = tx.send(text);
    }
}

fn client_reader(stream: UnixStream, id: u64, tx: Sender<LoopEvent>) {
    let reader = BufReader::new(stream);
    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };
        if let Ok(cmd) = serde_json::from_str::<Command>(&line)
            && tx.send(LoopEvent::Command { _id: id, cmd }).is_err()
        {
            break;
        }
    }
    let _ = tx.send(LoopEvent::Drop { id });
}

fn client_writer(mut stream: UnixStream, rx: Receiver<String>) {
    for text in rx {
        let mut body = text;
        body.push('\n');
        if stream.write_all(body.as_bytes()).is_err() {
            break;
        }
    }
    let _ = stream.shutdown(Shutdown::Both);
}
