//! Real-time collaboration for Archimedes.
//!
//! `CollabDoc` is a thin wrapper over a `yrs` CRDT document holding the
//! shared point set. Local edits and remote updates flow through the same
//! API. The transport layer (BroadcastChannel, WebSocket) lives elsewhere
//! and only sees opaque update bytes.

pub mod doc;

// CP14 wires these into the convex_hull tab; until then the binary doesn't
// reference them, but tests in `doc.rs` and downstream CPs will.
#[allow(unused_imports)]
pub use doc::{CollabDoc, CollabError, CollabPoint, PointId};
