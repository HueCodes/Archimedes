//! Cursor presence — ephemeral, not part of the CRDT.
//!
//! Each client samples its own pointer position, throttles to ~30Hz,
//! sends a `Presence` proto. Incoming presence frames update an
//! in-memory map of remote cursors. Entries older than `STALE_MS`
//! are dropped on each `prune` call so a tab that quietly disconnects
//! disappears within a second or two.
//!
//! Coordinates are sent **normalized** into the source canvas
//! (`[0..1]` per axis) so two clients with different canvas sizes
//! still render each other's cursor in roughly the same relative spot.

use std::collections::HashMap;

use eframe::egui::Pos2;
use uuid::Uuid;
use web_time::Instant;

use crate::collab::wire::{
    encode_envelope, envelope::Payload, Envelope, Presence,
};

const SEND_THROTTLE_MS: u128 = 33;
/// Drop remote cursors that haven't pinged us in this long.
const STALE_MS: u128 = 3_000;

#[cfg(target_arch = "wasm32")]
const STORAGE_KEY: &str = "archimedes:client_id";

#[derive(Clone, Debug)]
pub struct RemoteCursor {
    /// Normalized canvas coords; receiver scales by its own canvas size.
    pub pos_norm: Pos2,
    pub color: u32,
    pub last_seen: Instant,
}

pub struct PresenceTracker {
    client_id: String,
    color: u32,
    last_send_at: Instant,
    remotes: HashMap<String, RemoteCursor>,
}

impl PresenceTracker {
    pub fn new() -> Self {
        let client_id = load_or_generate_client_id();
        let color = color_for_id(&client_id);
        Self {
            client_id,
            color,
            last_send_at: Instant::now() - std::time::Duration::from_secs(1),
            remotes: HashMap::new(),
        }
    }

    pub fn client_id(&self) -> &str {
        &self.client_id
    }

    pub fn color(&self) -> u32 {
        self.color
    }

    /// Send a `Presence` if at least `SEND_THROTTLE_MS` has passed since
    /// the last send and the local cursor is over the canvas. The `send`
    /// closure is responsible for delivering bytes to whichever transport
    /// the caller is using; this module is transport-agnostic.
    pub fn maybe_send<F: FnOnce(Vec<u8>)>(
        &mut self,
        local_pos_norm: Option<Pos2>,
        send: F,
    ) {
        let pos = match local_pos_norm {
            Some(p) => p,
            None => return,
        };
        let now = Instant::now();
        if now.duration_since(self.last_send_at).as_millis() < SEND_THROTTLE_MS {
            return;
        }
        let env = Envelope {
            payload: Some(Payload::Presence(Presence {
                client_id: self.client_id.clone(),
                x: pos.x,
                y: pos.y,
                color: self.color,
                ts_ms: now_unix_ms(),
            })),
        };
        send(encode_envelope(&env));
        self.last_send_at = now;
    }

    /// Update or insert a remote cursor. Skips self-echoes.
    pub fn ingest(&mut self, presence: Presence) {
        if presence.client_id == self.client_id {
            return;
        }
        let cursor = RemoteCursor {
            pos_norm: Pos2::new(presence.x, presence.y),
            color: presence.color,
            last_seen: Instant::now(),
        };
        self.remotes.insert(presence.client_id, cursor);
    }

    /// Drop cursors older than `STALE_MS`. Call once per frame.
    pub fn prune(&mut self) {
        let now = Instant::now();
        self.remotes
            .retain(|_, c| now.duration_since(c.last_seen).as_millis() < STALE_MS);
    }

    pub fn remotes(&self) -> impl Iterator<Item = (&String, &RemoteCursor)> {
        self.remotes.iter()
    }

    pub fn len(&self) -> usize {
        self.remotes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.remotes.is_empty()
    }
}

impl Default for PresenceTracker {
    fn default() -> Self {
        Self::new()
    }
}

fn now_unix_ms() -> u64 {
    web_time::SystemTime::now()
        .duration_since(web_time::SystemTime::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// FNV-1a hash of the client id, biased so each channel has at least
/// some brightness on the dark theme.
pub fn color_for_id(id: &str) -> u32 {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in id.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    let r = ((h & 0xff) as u32).max(96);
    let g = (((h >> 8) & 0xff) as u32).max(96);
    let b = (((h >> 16) & 0xff) as u32).max(96);
    (r << 16) | (g << 8) | b
}

#[cfg(target_arch = "wasm32")]
fn load_or_generate_client_id() -> String {
    let win = match web_sys::window() {
        Some(w) => w,
        None => return Uuid::new_v4().to_string(),
    };
    let storage = match win.local_storage().ok().flatten() {
        Some(s) => s,
        None => return Uuid::new_v4().to_string(),
    };
    if let Ok(Some(id)) = storage.get_item(STORAGE_KEY) {
        if !id.is_empty() {
            return id;
        }
    }
    let id = Uuid::new_v4().to_string();
    let _ = storage.set_item(STORAGE_KEY, &id);
    id
}

#[cfg(not(target_arch = "wasm32"))]
fn load_or_generate_client_id() -> String {
    // Native sessions don't persist — every run gets a fresh id. The
    // demo lives in the browser, so this is fine.
    Uuid::new_v4().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn color_is_stable_per_id() {
        assert_eq!(color_for_id("alice"), color_for_id("alice"));
        assert_ne!(color_for_id("alice"), color_for_id("bob"));
    }

    #[test]
    fn color_channels_are_bright_enough() {
        let c = color_for_id("dark-id-test");
        let r = (c >> 16) & 0xff;
        let g = (c >> 8) & 0xff;
        let b = c & 0xff;
        assert!(r >= 96 && g >= 96 && b >= 96);
    }

    #[test]
    fn ingest_skips_self_echo() {
        let mut p = PresenceTracker::new();
        let self_id = p.client_id().to_string();
        p.ingest(Presence {
            client_id: self_id,
            x: 0.5,
            y: 0.5,
            color: 0xff0000,
            ts_ms: 0,
        });
        assert!(p.is_empty(), "self echo should not be stored");
    }

    #[test]
    fn ingest_stores_remote_cursor() {
        let mut p = PresenceTracker::new();
        p.ingest(Presence {
            client_id: "remote-1".into(),
            x: 0.25,
            y: 0.75,
            color: 0x123456,
            ts_ms: 0,
        });
        assert_eq!(p.len(), 1);
        let (_, cursor) = p.remotes().next().unwrap();
        assert!((cursor.pos_norm.x - 0.25).abs() < 1e-6);
        assert_eq!(cursor.color, 0x123456);
    }

    #[test]
    fn ingest_overwrites_same_id() {
        let mut p = PresenceTracker::new();
        for x in [0.1_f32, 0.2, 0.3] {
            p.ingest(Presence {
                client_id: "remote".into(),
                x,
                y: 0.5,
                color: 0,
                ts_ms: 0,
            });
        }
        assert_eq!(p.len(), 1);
        let (_, cursor) = p.remotes().next().unwrap();
        assert!((cursor.pos_norm.x - 0.3).abs() < 1e-6);
    }
}
