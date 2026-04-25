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

const DEFAULT_PORT: u16 = 8787;
const DEFAULT_ROOM: &str = "default";
/// Per-room broadcast queue depth. Beyond this, slow subscribers
/// will see Lagged errors and drop intermediate updates — CRDTs converge
/// regardless, so this is acceptable.
const ROOM_CAPACITY: usize = 256;

type Rooms = Arc<Mutex<HashMap<String, broadcast::Sender<Vec<u8>>>>>;

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

async fn handle_socket(socket: WebSocket, room: String, rooms: Rooms) {
    let sender = room_sender(&rooms, &room);
    let mut receiver = sender.subscribe();
    let n_subs = sender.receiver_count();
    tracing::debug!(room = %room, subscribers = n_subs, "client connected");

    let (mut sink, mut stream) = socket.split();

    // Outbound: broadcast → this client.
    let mut send_task = tokio::spawn(async move {
        while let Ok(bytes) = receiver.recv().await {
            if sink.send(Message::Binary(bytes)).await.is_err() {
                break;
            }
        }
    });

    // Inbound: this client → broadcast (echoed back to sender too;
    // CP22's Envelope.client_id will let receivers skip-self).
    let bcast = sender.clone();
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = stream.next().await {
            match msg {
                Message::Binary(bytes) => {
                    let _ = bcast.send(bytes);
                }
                Message::Text(text) => {
                    let _ = bcast.send(text.into_bytes());
                }
                Message::Close(_) => break,
                _ => {}
            }
        }
    });

    tokio::select! {
        _ = (&mut send_task) => recv_task.abort(),
        _ = (&mut recv_task) => send_task.abort(),
    }

    // Drop the room when the last subscriber leaves so memory stays bounded
    // for short-lived demo rooms. The Sender is still held by `bcast` until
    // recv_task aborts, so we recheck after the join above completes.
    let mut guard = rooms.lock().expect("rooms lock");
    if let Some(s) = guard.get(&room) {
        if s.receiver_count() == 0 {
            guard.remove(&room);
            tracing::debug!(room = %room, "room empty, removed");
        }
    }
}

fn room_sender(rooms: &Rooms, room: &str) -> broadcast::Sender<Vec<u8>> {
    let mut guard = rooms.lock().expect("rooms lock");
    guard
        .entry(room.to_string())
        .or_insert_with(|| broadcast::channel(ROOM_CAPACITY).0)
        .clone()
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

    /// Spin up the server on an ephemeral port and return its address.
    async fn spawn_server() -> SocketAddr {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, build_app()).await.unwrap();
        });
        // Give the server a beat to start accepting.
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

    #[tokio::test]
    async fn broadcast_within_room() {
        let addr = spawn_server().await;
        let mut a = connect(addr, "alpha").await;
        let mut b = connect(addr, "alpha").await;

        a.send(TMessage::Binary(b"hello".to_vec())).await.unwrap();

        // Drain frames on B until we see one matching the payload.
        // Echo to A is fine and irrelevant to this assertion.
        let received = tokio::time::timeout(Duration::from_secs(1), async {
            loop {
                if let Some(Ok(TMessage::Binary(bytes))) = b.next().await {
                    if bytes == b"hello" {
                        return bytes;
                    }
                }
            }
        })
        .await
        .expect("timeout waiting for broadcast");

        assert_eq!(received, b"hello");
    }

    #[tokio::test]
    async fn rooms_are_isolated() {
        let addr = spawn_server().await;
        let mut a = connect(addr, "alpha").await;
        let mut b = connect(addr, "beta").await;

        a.send(TMessage::Binary(b"alpha-only".to_vec())).await.unwrap();

        // B is in beta — it must NOT receive the alpha message.
        let result = tokio::time::timeout(Duration::from_millis(200), b.next()).await;
        assert!(result.is_err(), "beta client received cross-room message");
    }
}
