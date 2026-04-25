//! Pluggable transport layer.
//!
//! The default for the wasm build is `BroadcastChannel`: same-origin tabs
//! sync against each other with zero infrastructure, perfect for a
//! reviewer who opens two tabs of the deployed site. `?ws=<base>` opts
//! into the WebSocket relay for cross-device sync. Native builds get
//! `Disabled` (the desktop binary is single-user).

use crate::collab::broadcast::{channel_name_for_room, BroadcastChannelTransport};
use crate::collab::ws::{ws_url_from_query, WsClient, WsStatus};

/// Where to find the room name in the URL.
pub const DEFAULT_ROOM: &str = "default";

pub enum Transport {
    Disabled,
    Ws(WsClient),
    Broadcast(BroadcastChannelTransport),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransportKind {
    Disabled,
    Tabs,
    Relay,
}

impl Transport {
    /// Inspect the page URL and pick the right transport.
    /// Order: `?ws=<url>` → relay; otherwise → `BroadcastChannel`; if
    /// BroadcastChannel isn't available (native), `Disabled`.
    pub fn from_query() -> Self {
        if let Some(url) = ws_url_from_query() {
            return Transport::Ws(WsClient::connect(url));
        }
        let room = room_from_query().unwrap_or_else(|| DEFAULT_ROOM.to_string());
        match BroadcastChannelTransport::open(&channel_name_for_room(&room)) {
            Some(bc) => Transport::Broadcast(bc),
            None => Transport::Disabled,
        }
    }

    pub fn send(&self, bytes: Vec<u8>) {
        match self {
            Transport::Disabled => {}
            Transport::Ws(w) => w.send(bytes),
            Transport::Broadcast(b) => b.send(bytes),
        }
    }

    pub fn drain_inbound(&mut self) -> Vec<Vec<u8>> {
        match self {
            Transport::Disabled => Vec::new(),
            Transport::Ws(w) => w.drain_inbound(),
            Transport::Broadcast(b) => b.drain_inbound(),
        }
    }

    pub fn status(&self) -> WsStatus {
        match self {
            Transport::Disabled => WsStatus::Disabled,
            Transport::Ws(w) => w.status(),
            // BroadcastChannel can't fail or disconnect once open.
            Transport::Broadcast(_) => WsStatus::Connected,
        }
    }

    pub fn kind(&self) -> TransportKind {
        match self {
            Transport::Disabled => TransportKind::Disabled,
            Transport::Broadcast(_) => TransportKind::Tabs,
            Transport::Ws(_) => TransportKind::Relay,
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn room_from_query() -> Option<String> {
    let win = web_sys::window()?;
    let search = win.location().search().ok()?;
    let params = web_sys::UrlSearchParams::new_with_str(&search).ok()?;
    params.get("room")
}

#[cfg(not(target_arch = "wasm32"))]
fn room_from_query() -> Option<String> {
    None
}
