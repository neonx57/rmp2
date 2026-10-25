use super::Daemon;
use crate::engine::filter_queue;
use crate::proto::{NowPlaying, Snapshot, TagInfo};
use crate::storage::{LastState, state};
use std::time::Instant;

impl Daemon {
    pub(super) fn build_snapshot(&mut self) -> Snapshot {
        if self.media_dirty {
            self.refresh_library();
        }
        let re = self.search_re.as_ref();
        let queue = filter_queue(&self.media_all, &self.active_tags, re);
        let tags: Vec<TagInfo> = self
            .tag_counts
            .iter()
            .map(|(name, count)| TagInfo {
                name: name.clone(),
                checked: self.active_tags.contains(name),
                count: *count,
            })
            .collect();
        let selected = self
            .selected
            .and_then(|id| self.media_all.iter().find(|m| m.id == id).cloned());
        let now = self.now_id.map(|id| NowPlaying {
            id,
            position: self.position,
            duration: self
                .media_all
                .iter()
                .find(|m| m.id == id)
                .and_then(|m| m.duration),
            paused: self.paused,
            loading: self.loading || self.paused_for_cache,
        });
        Snapshot {
            theme: self.theme,
            transparent: self.transparent,
            all_media: self.media_all.clone(),
            queue,
            mini_queue: self.mini_queue.clone(),
            tags,
            search: self.search.clone(),
            selected,
            now,
            volume: self.volume,
            repeat: self.repeat,
            shuffle: self.shuffle,
            notify: self.notify.take(),
            error: self.error.take(),
        }
    }

    pub(super) fn snapshot_text(&mut self) -> String {
        let snap = self.build_snapshot();
        self.last_snap = Some(snap.clone());
        serde_json::to_string(&snap).unwrap_or_else(|e| {
            self.log(&format!("snapshot serialize: {e}"));
            serde_json::Value::Null.to_string()
        })
    }

    pub(super) fn push(&mut self) {
        let snap = self.build_snapshot();
        self.dirty = false;
        if self.last_snap.as_ref() == Some(&snap) {
            return;
        }
        self.last_push = Instant::now();
        let text =
            serde_json::to_string(&snap).unwrap_or_else(|_| serde_json::Value::Null.to_string());
        self.last_snap = Some(snap);
        self.clients.retain(|_, tx| tx.send(text.clone()).is_ok());
    }

    pub(super) fn persist(&self) {
        let s = LastState {
            theme: self.theme,
            transparent: self.transparent,
            volume: self.volume,
            repeat: self.repeat,
            shuffle: self.shuffle,
            active_tags: self.active_tags.clone(),
            media_id: self.now_id,
            position: self.position,
            playing: self.now_id.is_some() && !self.paused,
        };
        state::save(&self.paths.state, &s);
    }
}
