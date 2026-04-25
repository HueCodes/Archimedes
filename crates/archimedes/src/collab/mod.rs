//! Real-time collaboration for Archimedes.
//!
//! `CollabDoc` is a thin wrapper over a `yrs` CRDT document holding the
//! shared point set. Local edits and remote updates flow through the same
//! API. The transport layer (BroadcastChannel, WebSocket) lives elsewhere
//! and only sees opaque update bytes.

pub mod broadcast;
pub mod doc;
pub mod presence;
pub mod transport;
pub mod wire;
pub mod ws;

#[allow(unused_imports)]
pub use doc::{CollabDoc, CollabError, CollabPoint, PointId};
#[allow(unused_imports)]
pub use transport::{Transport, TransportKind};
#[allow(unused_imports)]
pub use ws::{ws_url_from_query, WsClient, WsStatus};
