pub mod client;
pub mod dispatcher;
pub mod lifecycle;
pub mod server;
pub mod snapshot;

pub use client::Connection;
pub use lifecycle::spawn_daemon;

use crate::config::Config;
use crate::db::Library;
use crate::engine::{Lcg, filter_queue, mini_pick, next_index};
use crate::error::Error;
use crate::mpv::{Mpv, MpvMsg, P_CACHE, P_PAUSE, P_TIME_POS};
use crate::proto::{Command, MediaInfo, RepeatMode, Snapshot, Theme};
use crate::storage::{LastState, Paths, state};
use regex::Regex;
use std::collections::HashMap;
use std::fs;
use std::os::unix::net::UnixListener;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::{Duration, Instant};

pub(super) enum LoopEvent {
    Command { _id: u64, cmd: Command },
    Drop { id: u64 },
    Mpv(MpvMsg),
}

pub struct Daemon {
    pub(super) paths: Paths,
    pub(super) cfg: Config,
    pub(super) lib: Library,
    pub(super) mpv: Option<Mpv>,
    pub(super) mpv_rx: Option<Receiver<MpvMsg>>,
    pub(super) listener: UnixListener,
    pub(super) events_tx: Sender<LoopEvent>,
    pub(super) events_rx: Receiver<LoopEvent>,
    pub(super) clients: HashMap<u64, Sender<String>>,
    pub(super) next_client: u64,
    pub(super) stop_flag: Arc<AtomicBool>,
    pub(super) active_tags: Vec<String>,
    pub(super) search: Option<String>,
    pub(super) search_re: Option<Regex>,
    pub(super) theme: Theme,
    pub(super) transparent: bool,
    pub(super) volume: i32,
    pub(super) repeat: RepeatMode,
    pub(super) shuffle: bool,
    pub(super) selected: Option<i64>,
    pub(super) now_id: Option<i64>,
    pub(super) last_main_id: Option<i64>,
    pub(super) playing_mini: bool,
    pub(super) mini_queue: Vec<i64>,
    pub(super) media_all: Vec<MediaInfo>,
    pub(super) tag_counts: Vec<(String, i64)>,
    pub(super) media_dirty: bool,
    pub(super) last_snap: Option<Snapshot>,
    pub(super) position: f64,
    pub(super) paused: bool,
    pub(super) loading: bool,
    pub(super) paused_for_cache: bool,
    pub(super) error: Option<String>,
    pub(super) notify: Option<String>,
    pub(super) dirty: bool,
    pub(super) last_push: Instant,
    pub(super) eof_pending: bool,
    pub(super) pending_resume: Option<(i64, f64, bool)>,
    pub(super) rng: Lcg,
    pub(super) stopping: bool,
}

impl Daemon {
    pub fn run() -> Result<(), Error> {
        let paths = Paths::resolve();
        let cfg = Config::load(&paths.config)?;
        Daemon::serve(paths, cfg)
    }

    pub fn serve(paths: Paths, cfg: Config) -> Result<(), Error> {
        let listener = match UnixListener::bind(&paths.sock) {
            Ok(l) => l,
            Err(_) => {
                if lifecycle::pid_alive(&paths.pid) {
                    return Err(Error::Daemon("daemon already running".into()));
                }
                let _ = fs::remove_file(&paths.sock);
                UnixListener::bind(&paths.sock)
                    .map_err(|e| Error::Daemon(format!("bind socket: {e}")))?
            }
        };
        listener
            .set_nonblocking(true)
            .map_err(|e| Error::Daemon(e.to_string()))?;
        lifecycle::write_pid(&paths);
        let last = state::load(&paths.state);
        let mut daemon = Daemon::new(paths, cfg, listener, last);
        daemon.log("daemon started");
        daemon.event_loop();
        daemon.log("daemon stopped");
        Ok(())
    }

    fn new(paths: Paths, cfg: Config, listener: UnixListener, last: LastState) -> Self {
        let lib = Library::open(&paths.db).unwrap_or_else(|e| {
            eprintln!("db error: {e}");
            std::process::exit(1);
        });
        let (events_tx, events_rx) = mpsc::channel();
        let stop_flag = Arc::new(AtomicBool::new(false));
        let mut d = Daemon {
            paths,
            cfg,
            lib,
            mpv: None,
            mpv_rx: None,
            listener,
            events_tx,
            events_rx,
            clients: HashMap::new(),
            next_client: 0,
            stop_flag: stop_flag.clone(),
            active_tags: last.active_tags.iter().map(|s| s.to_lowercase()).collect(),
            search: None,
            search_re: None,
            theme: last.theme,
            transparent: last.transparent,
            volume: last.volume.clamp(0, 130),
            repeat: last.repeat,
            shuffle: last.shuffle,
            selected: None,
            now_id: last.media_id,
            last_main_id: last.media_id,
            playing_mini: false,
            mini_queue: Vec::new(),
            media_all: Vec::new(),
            tag_counts: Vec::new(),
            media_dirty: true,
            last_snap: None,
            position: last.position,
            paused: true,
            loading: false,
            paused_for_cache: false,
            error: None,
            notify: None,
            dirty: false,
            last_push: Instant::now(),
            eof_pending: false,
            pending_resume: None,
            rng: Lcg::new(),
            stopping: false,
        };
        if let Some(id) = d.now_id {
            if d.lib.media(id).ok().flatten().is_some() {
                d.pending_resume = Some((id, d.position, false));
            } else {
                d.now_id = None;
                d.position = 0.0;
            }
        }
        d.install_signals();
        match d.ensure_mpv() {
            Ok(()) => {}
            Err(e) => d.log(&format!("mpv unavailable: {e}")),
        }
        d.push();
        d
    }

    fn ensure_mpv(&mut self) -> Result<(), String> {
        let ipc = self.paths.root.join("mpv.sock");
        let (mut mpv, rx) = Mpv::spawn(&self.cfg.mpv_binary, &ipc, false)?;
        let _ = mpv.volume(self.volume);
        let _ = mpv.command(serde_json::json!(["request_log_messages", "warn"]));
        mpv.observe(P_PAUSE, "pause");
        mpv.observe(P_TIME_POS, "time-pos");
        mpv.observe(P_CACHE, "paused-for-cache");
        if let Some(id) = self.now_id
            && let Some(m) = self.lib.media(id).ok().flatten()
        {
            let _ = mpv.set_pause(self.paused);
            let _ = mpv.loadfile(&m.uri);
            self.loading = true;
            self.pending_resume = Some((id, self.position, !self.paused));
        }
        self.mpv = Some(mpv);
        self.mpv_rx = Some(rx);
        Ok(())
    }

    fn event_loop(&mut self) {
        while !self.stopping && !self.stop_flag.load(Ordering::SeqCst) {
            self.drain_accepts();
            if let Ok(ev) = self.events_rx.recv_timeout(Duration::from_millis(30)) {
                self.handle_event(ev);
            }
            let mut mpv_events = Vec::new();
            if let Some(rx) = &self.mpv_rx {
                while let Ok(ev) = rx.try_recv() {
                    mpv_events.push(ev);
                }
            }
            for ev in mpv_events {
                self.handle_event(LoopEvent::Mpv(ev));
            }
            if self.dirty
                && self.now_id.is_some()
                && Instant::now().duration_since(self.last_push) >= Duration::from_millis(350)
            {
                self.push();
            }
        }
        self.shutdown();
    }

    fn track_ended(&mut self) {
        if self.now_id.is_none() {
            return;
        }
        if self.repeat == RepeatMode::One {
            let id = self.now_id;
            if let Some(id) = id
                && let Some(m) = self.lib.media(id).ok().flatten()
                && let Some(mv) = self.mpv.as_mut()
            {
                let _ = mv.loadfile(&m.uri);
                self.loading = true;
            }
            return;
        }
        if let Some(id) = mini_pick(&mut self.mini_queue, self.now_id) {
            self.play_mini(id);
            return;
        }
        let queue = self.queue_ids();
        let next = next_index(
            &queue,
            self.main_ref(&queue),
            1,
            self.shuffle,
            self.repeat,
            &mut |n| self.rng.next(n),
        );
        match next {
            Some(id) => self.play_id(id),
            None => {
                self.now_id = None;
                self.paused = false;
                self.position = 0.0;
                self.pending_resume = None;
                self.push();
            }
        }
    }

    fn main_ref(&self, queue: &[i64]) -> Option<i64> {
        match self.now_id {
            Some(id) if !self.playing_mini && queue.contains(&id) => Some(id),
            _ => self.last_main_id,
        }
    }

    fn start_playback(&mut self, id: i64, is_main: bool) {
        let info = match self.lib.media(id) {
            Ok(Some(m)) => m,
            Ok(None) => {
                self.notify = Some("media not found".into());
                return;
            }
            Err(e) => {
                self.log(&format!("db error: {e}"));
                return;
            }
        };
        let mpv = match self.mpv.as_mut() {
            Some(m) => m,
            None => {
                self.notify = Some("mpv unavailable".into());
                return;
            }
        };
        match mpv.loadfile(&info.uri) {
            Ok(()) => {
                let _ = mpv.set_pause(false);
                self.now_id = Some(id);
                self.selected = Some(id);
                self.position = 0.0;
                self.paused = false;
                self.eof_pending = false;
                self.pending_resume = None;
                self.playing_mini = !is_main;
                self.loading = true;
                self.paused_for_cache = false;
                if is_main {
                    self.last_main_id = Some(id);
                }
                self.push();
            }
            Err(e) => {
                self.log(&format!("loadfile error: {e}"));
                self.notify = Some("failed to load media".into());
            }
        }
    }

    fn play_id(&mut self, id: i64) {
        self.start_playback(id, true);
    }

    fn play_mini(&mut self, id: i64) {
        self.start_playback(id, false);
    }

    fn toggle_pause(&mut self) {
        if self.now_id.is_none() {
            let q = self.queue_ids();
            if let Some(first) = q.first() {
                self.play_id(*first);
            }
            return;
        }
        if let Some(m) = self.mpv.as_mut() {
            let _ = m.set_pause(!self.paused);
        }
        self.paused = !self.paused;
        self.push();
    }

    pub(super) fn refresh_library(&mut self) {
        self.media_all = self.lib.all_media().unwrap_or_default();
        self.tag_counts = self.lib.tag_counts().unwrap_or_default();
        self.media_dirty = false;
    }

    pub(super) fn queue_ids(&mut self) -> Vec<i64> {
        if self.media_dirty {
            self.refresh_library();
        }
        filter_queue(&self.media_all, &self.active_tags, self.search_re.as_ref())
    }
}
