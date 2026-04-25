//! WebSocket transport for cross-device collab.
//!
//! On `wasm32` the client opens a real WebSocket and pumps bytes between
//! the egui frame loop and the network via futures channels. On native
//! targets `connect` returns a `Disabled` client — the desktop binary
//! stays single-user.
//!
//! CP20 wires `drain_inbound` / `send` into `CollabDoc`.

use std::sync::{Arc, Mutex};

#[cfg(target_arch = "wasm32")]
use futures::channel::mpsc;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WsStatus {
    /// No URL configured (desktop build, or no `?ws=` query param).
    Disabled,
    Connecting,
    Connected,
    /// Lost connection; backing off before the next attempt.
    Reconnecting,
}

pub struct WsClient {
    status: Arc<Mutex<WsStatus>>,
    #[cfg(target_arch = "wasm32")]
    inbound_rx: mpsc::UnboundedReceiver<Vec<u8>>,
    #[cfg(target_arch = "wasm32")]
    outbound_tx: mpsc::UnboundedSender<Vec<u8>>,
}

impl WsClient {
    /// A client that never connects. Used on native and as the fallback
    /// when no `?ws=` query param is present.
    pub fn disabled() -> Self {
        #[cfg(target_arch = "wasm32")]
        {
            let (_, inbound_rx) = mpsc::unbounded();
            let (outbound_tx, _) = mpsc::unbounded();
            return Self {
                status: Arc::new(Mutex::new(WsStatus::Disabled)),
                inbound_rx,
                outbound_tx,
            };
        }
        #[cfg(not(target_arch = "wasm32"))]
        Self {
            status: Arc::new(Mutex::new(WsStatus::Disabled)),
        }
    }

    /// Open a connection to `url`, retrying with exponential backoff on
    /// failure or disconnect. Bytes received from the server are pushed
    /// onto the inbound channel; bytes from `send` are forwarded to the
    /// server. Native targets return `disabled()`.
    #[cfg(target_arch = "wasm32")]
    pub fn connect(url: String) -> Self {
        let status = Arc::new(Mutex::new(WsStatus::Connecting));
        let (inbound_tx, inbound_rx) = mpsc::unbounded();
        let (outbound_tx, outbound_rx) = mpsc::unbounded();
        let status_for_task = status.clone();
        wasm_bindgen_futures::spawn_local(async move {
            run_ws_loop(url, status_for_task, inbound_tx, outbound_rx).await;
        });
        Self {
            status,
            inbound_rx,
            outbound_tx,
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn connect(_url: String) -> Self {
        Self::disabled()
    }

    pub fn status(&self) -> WsStatus {
        *self.status.lock().expect("ws status lock")
    }

    /// Queue bytes for delivery to the server. No-op when disabled.
    pub fn send(&self, bytes: Vec<u8>) {
        #[cfg(target_arch = "wasm32")]
        {
            let _ = self.outbound_tx.unbounded_send(bytes);
        }
        #[cfg(not(target_arch = "wasm32"))]
        let _ = bytes;
    }

    /// Drain every byte slice received since the last call. Intended to be
    /// called once per frame from the egui update loop.
    pub fn drain_inbound(&mut self) -> Vec<Vec<u8>> {
        #[cfg(target_arch = "wasm32")]
        {
            let mut out = Vec::new();
            while let Ok(Some(b)) = self.inbound_rx.try_next() {
                out.push(b);
            }
            out
        }
        #[cfg(not(target_arch = "wasm32"))]
        Vec::new()
    }
}

/// Read `?ws=<base-url>&room=<room>` from the page URL and assemble a
/// connection target. Returns `None` if no `ws` param is set.
#[cfg(target_arch = "wasm32")]
pub fn ws_url_from_query() -> Option<String> {
    let win = web_sys::window()?;
    let search = win.location().search().ok()?;
    let params = web_sys::UrlSearchParams::new_with_str(&search).ok()?;
    let base = params.get("ws")?;
    let room = params.get("room").unwrap_or_else(|| "default".to_string());
    Some(format!("{base}?room={room}"))
}

#[cfg(not(target_arch = "wasm32"))]
pub fn ws_url_from_query() -> Option<String> {
    None
}

#[cfg(target_arch = "wasm32")]
async fn run_ws_loop(
    url: String,
    status: Arc<Mutex<WsStatus>>,
    inbound_tx: mpsc::UnboundedSender<Vec<u8>>,
    mut outbound_rx: mpsc::UnboundedReceiver<Vec<u8>>,
) {
    use futures::{select, FutureExt, SinkExt, StreamExt};
    use gloo_net::websocket::{futures::WebSocket, Message};

    const INITIAL_BACKOFF_MS: u64 = 250;
    const MAX_BACKOFF_MS: u64 = 30_000;

    let mut backoff_ms = INITIAL_BACKOFF_MS;

    loop {
        *status.lock().unwrap() = if backoff_ms == INITIAL_BACKOFF_MS {
            WsStatus::Connecting
        } else {
            WsStatus::Reconnecting
        };

        let ws = match WebSocket::open(&url) {
            Ok(w) => w,
            Err(e) => {
                log::warn!("ws open failed: {e}");
                *status.lock().unwrap() = WsStatus::Reconnecting;
                gloo_timers::future::sleep(std::time::Duration::from_millis(backoff_ms))
                    .await;
                backoff_ms = (backoff_ms * 2).min(MAX_BACKOFF_MS);
                continue;
            }
        };

        *status.lock().unwrap() = WsStatus::Connected;
        backoff_ms = INITIAL_BACKOFF_MS;

        let (mut sink, mut stream) = ws.split();

        loop {
            select! {
                outbound = outbound_rx.next().fuse() => {
                    match outbound {
                        Some(bytes) => {
                            if sink.send(Message::Bytes(bytes)).await.is_err() {
                                break;
                            }
                        }
                        None => return, // client dropped
                    }
                }
                inbound = stream.next().fuse() => {
                    match inbound {
                        Some(Ok(Message::Bytes(b))) => {
                            if inbound_tx.unbounded_send(b).is_err() {
                                return;
                            }
                        }
                        Some(Ok(Message::Text(t))) => {
                            if inbound_tx.unbounded_send(t.into_bytes()).is_err() {
                                return;
                            }
                        }
                        Some(Err(e)) => {
                            log::warn!("ws read error: {e}");
                            break;
                        }
                        None => break,
                    }
                }
            }
        }

        *status.lock().unwrap() = WsStatus::Reconnecting;
        gloo_timers::future::sleep(std::time::Duration::from_millis(backoff_ms)).await;
        backoff_ms = (backoff_ms * 2).min(MAX_BACKOFF_MS);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disabled_client_starts_disabled_and_drains_empty() {
        let mut client = WsClient::disabled();
        assert_eq!(client.status(), WsStatus::Disabled);
        assert!(client.drain_inbound().is_empty());
        client.send(b"ignored".to_vec()); // must not panic
    }

    #[test]
    fn connect_on_native_returns_disabled() {
        let client = WsClient::connect("ws://example.invalid".into());
        assert_eq!(client.status(), WsStatus::Disabled);
    }

    #[test]
    fn url_helper_returns_none_on_native() {
        assert_eq!(ws_url_from_query(), None);
    }
}
