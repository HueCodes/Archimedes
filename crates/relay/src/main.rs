//! Archimedes collab relay.
//!
//! One process, many rooms. Each room is an opaque-bytes broadcast channel.
//! Clients connect via `GET /ws?room=<name>`; messages from any client in a
//! room are forwarded to every other client in the same room.
//!
//! No persistence, no auth, no presence tracking on the server. The CRDT
//! convergence and presence state live entirely on clients.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Query, State,
    },
    response::IntoResponse,
    routing::get,
    Router,
};
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use tokio::sync::broadcast;
use prost::Message as _;
use yrs::updates::decoder::Decode;
use yrs::{Doc, ReadTxn, StateVector, Transact, Update};

mod wire {
    #![allow(clippy::all)]
    include!(concat!(env!("OUT_DIR"), "/archimedes.v1.rs"));
}

use wire::{envelope::Payload, DocUpdate, Envelope};

const DEFAULT_PORT: u16 = 8787;
const DEFAULT_ROOM: &str = "default";
/// Per-room broadcast queue depth. Beyond this, slow subscribers
/// will see Lagged errors and drop intermediate updates — CRDTs converge
/// regardless, so this is acceptable.
const ROOM_CAPACITY: usize = 256;

/// Per-room state. The doc is the authoritative copy; the sender fan-outs
/// updates to every connected subscriber (including the originator —
/// CP22's Envelope.client_id will let receivers skip-self).
struct Room {
    doc: Mutex<Doc>,
    sender: broadcast::Sender<Vec<u8>>,
}

impl Room {
    fn new() -> Self {
        Self {
            doc: Mutex::new(Doc::new()),
            sender: broadcast::channel(ROOM_CAPACITY).0,
        }
    }
}

type Rooms = Arc<Mutex<HashMap<String, Arc<Room>>>>;

#[derive(Debug, Deserialize)]
struct WsParams {
    room: Option<String>,
}

#[tokio::main]
async fn main() {
    init_tracing();

    let port = std::env::var("PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(DEFAULT_PORT);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = tokio::net::TcpListener::bind(addr).await.expect("bind");

    tracing::info!(%addr, "relay listening");

    axum::serve(listener, build_app())
        .with_graceful_shutdown(shutdown_signal())
        .await
        .expect("serve");
}

fn init_tracing() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info,relay=debug")),
        )
        .init();
}

fn build_app() -> Router {
    let rooms: Rooms = Arc::new(Mutex::new(HashMap::new()));
    Router::new()
        .route("/healthz", get(|| async { "ok" }))
        .route("/ws", get(ws_handler))
        .with_state(rooms)
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    Query(params): Query<WsParams>,
    State(rooms): State<Rooms>,
) -> impl IntoResponse {
    let room = params.room.unwrap_or_else(|| DEFAULT_ROOM.to_string());
    ws.on_upgrade(move |socket| handle_socket(socket, room, rooms))
}

async fn handle_socket(socket: WebSocket, room_name: String, rooms: Rooms) {
    let room = get_or_create_room(&rooms, &room_name);
    let mut receiver = room.sender.subscribe();
    let n_subs = room.sender.receiver_count();
    tracing::debug!(room = %room_name, subscribers = n_subs, "client connected");

    let (mut sink, mut stream) = socket.split();

    // Send the room snapshot first so the new client converges to current
    // state without needing any prior sync handshake. Wrapped in an
    // Envelope::DocUpdate so the client decodes it exactly like any
    // other relayed update.
    let snapshot = encode_room_state(&room.doc);
    let snapshot_env = Envelope {
        payload: Some(Payload::Update(DocUpdate {
            yrs_update: snapshot,
        })),
    };
    let snapshot_bytes = snapshot_env.encode_to_vec();
    if sink.send(Message::Binary(snapshot_bytes)).await.is_err() {
        tracing::debug!(room = %room_name, "client dropped before snapshot");
        return;
    }

    // Outbound: broadcast → this client.
    let mut send_task = tokio::spawn(async move {
        while let Ok(bytes) = receiver.recv().await {
            if sink.send(Message::Binary(bytes)).await.is_err() {
                break;
            }
        }
    });

    // Inbound: this client → decode Envelope → route by payload type.
    let room_for_recv = room.clone();
    let room_name_for_recv = room_name.clone();
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = stream.next().await {
            let bytes = match msg {
                Message::Binary(b) => b,
                Message::Text(t) => t.into_bytes(),
                Message::Close(_) => break,
                _ => continue,
            };
            let env = match Envelope::decode(&bytes[..]) {
                Ok(e) => e,
                Err(e) => {
                    tracing::warn!(room = %room_name_for_recv, "dropping malformed envelope: {e}");
                    continue;
                }
            };
            match env.payload {
                Some(Payload::Update(ref u)) => {
                    if let Err(e) = apply_update_to_doc(&room_for_recv.doc, &u.yrs_update) {
                        tracing::warn!(room = %room_name_for_recv, "dropping malformed yrs update: {e}");
                        continue;
                    }
                    let _ = room_for_recv.sender.send(bytes);
                }
                Some(Payload::Presence(_)) => {
                    // Ephemeral; not stored, just relayed.
                    let _ = room_for_recv.sender.send(bytes);
                }
                Some(Payload::Hello(ref h)) => {
                    tracing::debug!(
                        room = %room_name_for_recv,
                        client = %h.client_id,
                        version = h.protocol_version,
                        "client hello"
                    );
                }
                None => {}
            }
        }
    });

    tokio::select! {
        _ = (&mut send_task) => recv_task.abort(),
        _ = (&mut recv_task) => send_task.abort(),
    }

    // Drop the room when the last subscriber leaves. We held an Arc<Room>
    // for the duration of the handler, so receiver_count drops to 0 only
    // after our subscribe handle is dropped on task exit.
    drop(room);
    let mut guard = rooms.lock().expect("rooms lock");
    if let Some(r) = guard.get(&room_name) {
        if r.sender.receiver_count() == 0 {
            guard.remove(&room_name);
            tracing::debug!(room = %room_name, "room empty, removed");
        }
    }
}

fn get_or_create_room(rooms: &Rooms, name: &str) -> Arc<Room> {
    let mut guard = rooms.lock().expect("rooms lock");
    guard
        .entry(name.to_string())
        .or_insert_with(|| Arc::new(Room::new()))
        .clone()
}

fn encode_room_state(doc: &Mutex<Doc>) -> Vec<u8> {
    let doc = doc.lock().expect("doc lock");
    let txn = doc.transact();
    txn.encode_state_as_update_v1(&StateVector::default())
}

fn apply_update_to_doc(doc: &Mutex<Doc>, bytes: &[u8]) -> Result<(), String> {
    let update = Update::decode_v1(bytes).map_err(|e| format!("decode: {e}"))?;
    let doc = doc.lock().map_err(|_| "doc lock poisoned".to_string())?;
    let mut txn = doc.transact_mut();
    txn.apply_update(update).map_err(|e| format!("apply: {e}"))
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
    tracing::info!("shutdown signal received");
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_util::{SinkExt, StreamExt};
    use std::time::Duration;
    use tokio_tungstenite::tungstenite::Message as TMessage;
    use yrs::Map;

    /// Spin up the server on an ephemeral port and return its address.
    async fn spawn_server() -> SocketAddr {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, build_app()).await.unwrap();
        });
        tokio::time::sleep(Duration::from_millis(20)).await;
        addr
    }

    async fn connect(
        addr: SocketAddr,
        room: &str,
    ) -> tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
    > {
        let url = format!("ws://{addr}/ws?room={room}");
        let (ws, _) = tokio_tungstenite::connect_async(&url).await.unwrap();
        ws
    }

    /// Drain the snapshot frame the server sends as soon as a client connects.
    async fn drain_snapshot(
        ws: &mut tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
        >,
    ) -> Vec<u8> {
        let next = tokio::time::timeout(Duration::from_secs(1), ws.next())
            .await
            .expect("snapshot timeout")
            .expect("snapshot stream end")
            .expect("snapshot frame error");
        match next {
            TMessage::Binary(b) => b,
            other => panic!("expected binary snapshot, got {other:?}"),
        }
    }

    /// Build an Envelope::DocUpdate that inserts a single i64.
    fn make_update_with(key: &str, value: i64) -> Vec<u8> {
        let doc = Doc::new();
        let map = doc.get_or_insert_map("test");
        {
            let mut txn = doc.transact_mut();
            map.insert(&mut txn, key.to_string(), value);
        }
        let txn = doc.transact();
        let yrs = txn.encode_state_as_update_v1(&StateVector::default());
        Envelope {
            payload: Some(Payload::Update(DocUpdate { yrs_update: yrs })),
        }
        .encode_to_vec()
    }

    /// Decode an Envelope::DocUpdate and apply its yrs payload to a fresh doc.
    fn doc_from_envelope(bytes: &[u8]) -> Doc {
        let env = Envelope::decode(bytes).expect("envelope decode");
        let yrs = match env.payload {
            Some(Payload::Update(u)) => u.yrs_update,
            other => panic!("expected DocUpdate, got {other:?}"),
        };
        let doc = Doc::new();
        let update = Update::decode_v1(&yrs).expect("yrs decode");
        {
            let mut txn = doc.transact_mut();
            txn.apply_update(update).expect("apply");
        }
        doc
    }

    fn read_test_value(doc: &Doc, key: &str) -> Option<i64> {
        let map = doc.get_or_insert_map("test");
        let txn = doc.transact();
        let value = map.get(&txn, key)?;
        match value {
            yrs::Out::Any(yrs::Any::BigInt(n)) => Some(n),
            yrs::Out::Any(yrs::Any::Number(n)) => Some(n as i64),
            other => panic!("unexpected value shape: {other:?}"),
        }
    }

    #[test]
    fn make_update_round_trips_locally() {
        // Sanity: our test-helper construction can be decoded back to the
        // same value. If this fails the network tests are red-herrings.
        let bytes = make_update_with("k", 42);
        let doc = doc_from_envelope(&bytes);
        assert_eq!(read_test_value(&doc, "k"), Some(42));
    }

    #[tokio::test]
    async fn broadcast_within_room() {
        let addr = spawn_server().await;
        let mut a = connect(addr, "alpha").await;
        let mut b = connect(addr, "alpha").await;
        let _ = drain_snapshot(&mut a).await;
        let _ = drain_snapshot(&mut b).await;

        let update = make_update_with("k", 42);
        a.send(TMessage::Binary(update.clone())).await.unwrap();

        let received = tokio::time::timeout(Duration::from_secs(1), async {
            loop {
                if let Some(Ok(TMessage::Binary(bytes))) = b.next().await {
                    return bytes;
                }
            }
        })
        .await
        .expect("timeout waiting for broadcast");

        let doc = doc_from_envelope(&received);
        assert_eq!(read_test_value(&doc, "k"), Some(42));
    }

    #[tokio::test]
    async fn rooms_are_isolated() {
        let addr = spawn_server().await;
        let mut a = connect(addr, "alpha").await;
        let mut b = connect(addr, "beta").await;
        let _ = drain_snapshot(&mut a).await;
        let _ = drain_snapshot(&mut b).await;

        let update = make_update_with("k", 1);
        a.send(TMessage::Binary(update)).await.unwrap();

        let result = tokio::time::timeout(Duration::from_millis(200), b.next()).await;
        assert!(result.is_err(), "beta client received cross-room message");
    }

    #[tokio::test]
    async fn late_joiner_receives_prior_state_in_snapshot() {
        let addr = spawn_server().await;
        let mut a = connect(addr, "history").await;
        let _ = drain_snapshot(&mut a).await;

        // A makes a change while alone in the room.
        a.send(TMessage::Binary(make_update_with("first", 7)))
            .await
            .unwrap();
        // Give the server a moment to apply.
        tokio::time::sleep(Duration::from_millis(50)).await;

        // B joins late; first frame should already include A's change.
        let mut b = connect(addr, "history").await;
        let snapshot = drain_snapshot(&mut b).await;
        let doc = doc_from_envelope(&snapshot);
        assert_eq!(
            read_test_value(&doc, "first"),
            Some(7),
            "late joiner did not see prior state"
        );
    }

    #[tokio::test]
    async fn malformed_update_is_dropped_not_broadcast() {
        let addr = spawn_server().await;
        let mut a = connect(addr, "noise").await;
        let mut b = connect(addr, "noise").await;
        let _ = drain_snapshot(&mut a).await;
        let _ = drain_snapshot(&mut b).await;

        a.send(TMessage::Binary(b"\xff\xff garbage".to_vec()))
            .await
            .unwrap();

        let result = tokio::time::timeout(Duration::from_millis(200), b.next()).await;
        assert!(result.is_err(), "garbage was relayed instead of dropped");
    }
}
