use super::{Daemon, LoopEvent};
use crate::db::NewMedia;
use crate::db::classify_uri;
use crate::engine::{mpv_error_line, next_index};
use crate::mpv::{MpvMsg, live_info, probe_metadata};
use crate::proto::Command;
use crate::storage::Paths;
use regex::Regex;
use serde_json::Value;
use std::path::Path;

impl Daemon {
    pub(super) fn handle_event(&mut self, ev: LoopEvent) {
        match ev {
            LoopEvent::Command { cmd, .. } => self.handle_command(cmd),
            LoopEvent::Drop { id } => {
                self.clients.remove(&id);
            }
            LoopEvent::Mpv(m) => self.handle_mpv(m),
        }
    }

    fn handle_mpv(&mut self, msg: MpvMsg) {
        match msg {
            MpvMsg::Event { name, data } => {
                let got = data.as_ref().and_then(|v| v.as_bool());
                match name.as_str() {
                    "time-pos" => {
                        if let Some(v) = data.as_ref().and_then(|v| v.as_f64()) {
                            self.position = v;
                            self.dirty = true;
                        }
                    }
                    "pause" => {
                        if let Some(v) = got {
                            self.paused = v;
                            self.dirty = true;
                        }
                    }
                    "paused-for-cache" => {
                        if let Some(v) = got {
                            self.paused_for_cache = v;
                            self.dirty = true;
                        }
                    }
                    "end-file" => {
                        let reason = data
                            .as_ref()
                            .and_then(|v| v.get("reason"))
                            .and_then(|v| v.as_str());
                        if reason == Some("eof") && !self.eof_pending {
                            self.eof_pending = true;
                            self.track_ended();
                        } else if reason == Some("error") {
                            self.on_playback_error(data.as_ref());
                        }
                    }
                    "log-message" => {
                        let obj = data.unwrap_or(Value::Null);
                        let prefix = obj.get("prefix").and_then(|v| v.as_str()).unwrap_or("mpv");
                        let level = obj.get("level").and_then(|v| v.as_str()).unwrap_or("info");
                        let text = obj.get("text").and_then(|v| v.as_str()).unwrap_or("");
                        let line = mpv_error_line(prefix, text);
                        self.log(&format!("mpv[{level}] {line}"));
                        if (level == "error" || level == "fatal") && self.error.is_none() {
                            self.error = Some(line);
                            self.push();
                        }
                    }
                    "file-loaded" => {
                        self.eof_pending = false;
                        self.on_file_loaded();
                    }
                    _ => {}
                }
            }
            MpvMsg::Shutdown => {
                self.log("mpv exited, restarting");
                self.mpv = None;
                self.mpv_rx = None;
                if !self.stopping {
                    if let Err(e) = self.ensure_mpv() {
                        self.log(&format!("mpv restart failed: {e}"));
                        self.notify = Some("mpv restart failed".into());
                    }
                    self.push();
                }
            }
        }
    }

    fn on_playback_error(&mut self, data: Option<&Value>) {
        let detail = data
            .and_then(|v| v.get("file_error"))
            .and_then(|v| v.as_str())
            .unwrap_or("failed to load media");
        self.log(&format!("mpv end-file error: {detail}"));
        if self.error.is_none() {
            self.error = Some(detail.to_string());
        }
        self.now_id = None;
        self.position = 0.0;
        self.loading = false;
        self.paused_for_cache = false;
        self.pending_resume = None;
        self.eof_pending = false;
        self.push();
    }

    fn on_file_loaded(&mut self) {
        self.eof_pending = false;
        if let Some((id, pos, playing)) = self.pending_resume.take() {
            self.now_id = Some(id);
            self.selected = Some(id);
            if let Some(m) = self.mpv.as_mut() {
                if pos > 1.0 {
                    let _ = m.seek(pos, true);
                }
                let _ = m.set_pause(!playing);
            }
            self.paused = !playing;
            self.dirty = true;
        }
        let id = self.now_id;
        if let Some(id) = id
            && let Some(m) = self.mpv.as_mut()
        {
            let (title, artist, duration, bitrate) = live_info(m);
            if self.pending_resume.is_none() {
                let _ = self
                    .lib
                    .set_title_artist(id, title.as_deref(), artist.as_deref());
            }
            let _ = self.lib.update_playback_stats(id, duration, bitrate);
        }
        if let Some(m) = self.mpv.as_mut() {
            let _ = m.set_pause(self.paused);
        }
        self.loading = false;
        self.paused_for_cache = false;
        self.media_dirty = true;
        self.push();
    }

    pub(super) fn handle_command(&mut self, cmd: Command) {
        let before = cmd.clone();
        match before {
            Command::PlayPause => self.toggle_pause(),
            Command::Next => {
                let q = self.queue_ids();
                if let Some(n) = next_index(
                    &q,
                    self.main_ref(&q),
                    1,
                    self.shuffle,
                    self.repeat,
                    &mut |n| self.rng.next(n),
                ) {
                    self.play_id(n);
                }
            }
            Command::Prev => {
                let q = self.queue_ids();
                if let Some(n) = next_index(
                    &q,
                    self.main_ref(&q),
                    -1,
                    self.shuffle,
                    self.repeat,
                    &mut |n| self.rng.next(n),
                ) {
                    self.play_id(n);
                }
            }
            Command::Play { id } => self.play_id(id),
            Command::Select { id } => {
                self.selected = Some(id);
                self.push();
            }
            Command::Seek { delta } => {
                if let Some(m) = self.mpv.as_mut() {
                    let _ = m.seek(delta, false);
                }
                self.position = (self.position + delta).max(0.0);
                self.push();
            }
            Command::Volume { delta } => {
                self.volume = (self.volume + delta).clamp(0, 130);
                if let Some(m) = self.mpv.as_mut() {
                    let _ = m.volume(self.volume);
                }
                self.persist();
                self.push();
            }
            Command::SetVolume { volume } => {
                self.volume = volume.clamp(0, 130);
                if let Some(m) = self.mpv.as_mut() {
                    let _ = m.volume(self.volume);
                }
                self.persist();
                self.push();
            }
            Command::RepeatCycle => {
                self.repeat = self.repeat.cycle();
                self.persist();
                self.push();
            }
            Command::ShuffleToggle => {
                self.shuffle = !self.shuffle;
                self.persist();
                self.push();
            }
            Command::ToggleTag { tag } => {
                let tag = tag.to_lowercase();
                if self.active_tags.contains(&tag) {
                    self.active_tags.retain(|t| *t != tag);
                } else {
                    self.active_tags.push(tag);
                }
                self.persist();
                self.push();
            }
            Command::ClearTags => {
                self.active_tags.clear();
                self.persist();
                self.push();
            }
            Command::SetSearch { pattern } => {
                self.search = pattern.filter(|p| !p.is_empty());
                self.search_re = self.search.as_deref().map(Regex::new).and_then(|r| r.ok());
                self.push();
            }
            Command::Add { uri, title, tags } => self.add_media(uri, title, tags),
            Command::Update {
                id,
                title,
                artist,
                tags,
            } => {
                if let Err(e) =
                    self.lib
                        .update_media(id, title.as_deref(), artist.as_deref(), &tags)
                {
                    self.log(&format!("update error: {e}"));
                    self.notify = Some("update failed".into());
                }
                self.media_dirty = true;
                self.push();
            }
            Command::Delete { id } => {
                let _ = self.lib.delete_media(id);
                self.media_dirty = true;
                self.mini_queue.retain(|x| *x != id);
                if self.now_id == Some(id) {
                    self.now_id = None;
                    self.selected = None;
                    self.pending_resume = None;
                    if let Some(m) = self.mpv.as_mut() {
                        let _ = m.command(serde_json::json!(["stop"]));
                    }
                }
                if self.selected == Some(id) {
                    self.selected = None;
                }
                self.push();
            }
            Command::AddMini { id } => {
                if self.lib.media(id).ok().flatten().is_some() {
                    self.mini_queue.retain(|x| *x != id);
                    self.mini_queue.push(id);
                    self.notify = Some("added to queue".into());
                }
                self.push();
            }
            Command::RemoveMini { id } => {
                self.mini_queue.retain(|x| *x != id);
                self.push();
            }
            Command::MiniMove { id, delta } => {
                let Some(i) = self.mini_queue.iter().position(|x| *x == id) else {
                    self.push();
                    return;
                };
                let j = (i as i64 + delta).clamp(0, self.mini_queue.len() as i64 - 1) as usize;
                self.mini_queue.swap(i, j);
                self.push();
            }
            Command::ToggleFavorite { id } => {
                if let Ok(Some(m)) = self.lib.media(id) {
                    let _ = self.lib.set_favorite(id, !m.favorite);
                    self.media_dirty = true;
                }
                self.push();
            }
            Command::SetTheme { theme } => {
                self.theme = theme;
                self.persist();
                self.push();
            }
            Command::SetTransparency { on } => {
                self.transparent = on;
                self.persist();
                self.push();
            }
            Command::Shutdown => {
                self.stopping = true;
            }
        }
    }

    fn add_media(&mut self, uri: String, title: Option<String>, tags: Vec<String>) {
        let uri = uri.trim().to_string();
        if uri.is_empty() {
            self.notify = Some("uri cannot be empty".into());
            self.push();
            return;
        }
        let (kind, source) = classify_uri(&uri);
        if kind == "offline" && !Path::new(&uri).exists() {
            self.notify = Some("file does not exist".into());
            self.push();
            return;
        }
        let title = title
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        let tags: Vec<String> = {
            let mut seen = vec![];
            for t in tags {
                let t = t.trim().to_lowercase();
                if !t.is_empty() && !seen.contains(&t) {
                    seen.push(t);
                }
            }
            seen
        };
        let (probe_title, artist, duration, bitrate) = if kind == "offline" {
            let probe = probe_path();
            let res = probe_metadata(&self.cfg.mpv_binary, &probe, &uri);
            let _ = std::fs::remove_file(&probe);
            res.unwrap_or((None, None, None, None))
        } else {
            (None, None, None, None)
        };
        let title = title.or(probe_title);
        match self.lib.add_media(
            NewMedia {
                uri: &uri,
                title: title.as_deref(),
                kind,
                artist: artist.as_deref(),
                duration,
                bitrate,
                source: source.as_deref(),
            },
            &tags,
        ) {
            Ok(id) => {
                let display = title
                    .as_deref()
                    .filter(|t| !t.trim().is_empty())
                    .map(str::to_string)
                    .unwrap_or_else(|| uri.clone());
                self.log(&format!("added media {id}: {display}"));
                self.notify = Some(format!("added: {display}"));
                if self.selected.is_none() {
                    self.selected = Some(id);
                }
            }
            Err(e) => {
                self.log(&format!("add error: {e}"));
                self.notify = Some("add failed".into());
            }
        }
        self.media_dirty = true;
        self.push();
    }
}

fn probe_path() -> std::path::PathBuf {
    Paths::resolve()
        .root
        .join(format!("probe-{}.sock", std::process::id()))
}
