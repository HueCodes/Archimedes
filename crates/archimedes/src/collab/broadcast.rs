//! `BroadcastChannel` transport.
//!
//! Browser-native cross-tab messaging — every same-origin tab listening on
//! the same channel name receives every other tab's posts. No server, no
//! origin issues, works against the static GitHub Pages bundle.
//!
//! Per the Web spec, BroadcastChannel does **not** echo to the originating
//! tab, so we don't have to filter self-traffic.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::{closure::Closure, JsCast};

pub struct BroadcastChannelTransport {
    inbound: Arc<Mutex<VecDeque<Vec<u8>>>>,
    #[cfg(target_arch = "wasm32")]
    bc: web_sys::BroadcastChannel,
    /// Kept alive so the JS callback stays registered for the lifetime of
    /// the transport. Dropping this would unregister the listener.
    #[cfg(target_arch = "wasm32")]
    _on_message: Closure<dyn FnMut(web_sys::MessageEvent)>,
}

impl BroadcastChannelTransport {
    /// Open a BroadcastChannel by name. Returns `None` on native or if the
    /// browser doesn't support BroadcastChannel (very old).
    #[cfg(target_arch = "wasm32")]
    pub fn open(channel_name: &str) -> Option<Self> {
        let bc = web_sys::BroadcastChannel::new(channel_name).ok()?;
        let inbound: Arc<Mutex<VecDeque<Vec<u8>>>> = Arc::new(Mutex::new(VecDeque::new()));
        let inbound_for_cb = inbound.clone();
        let on_message = Closure::wrap(Box::new(move |e: web_sys::MessageEvent| {
            let data = e.data();
            // Posted via Uint8Array; some senders may use ArrayBuffer.
            let bytes = if let Some(arr) = data.dyn_ref::<js_sys::Uint8Array>() {
                let mut buf = vec![0u8; arr.length() as usize];
                arr.copy_to(&mut buf);
                Some(buf)
            } else if let Some(ab) = data.dyn_ref::<js_sys::ArrayBuffer>() {
                let arr = js_sys::Uint8Array::new(ab);
                let mut buf = vec![0u8; arr.length() as usize];
                arr.copy_to(&mut buf);
                Some(buf)
            } else {
                None
            };
            if let Some(bytes) = bytes {
                if let Ok(mut q) = inbound_for_cb.lock() {
                    q.push_back(bytes);
                }
            }
        }) as Box<dyn FnMut(_)>);
        bc.set_onmessage(Some(on_message.as_ref().unchecked_ref()));
        Some(Self {
            inbound,
            bc,
            _on_message: on_message,
        })
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn open(_channel_name: &str) -> Option<Self> {
        None
    }

    pub fn send(&self, bytes: Vec<u8>) {
        #[cfg(target_arch = "wasm32")]
        {
            let arr = js_sys::Uint8Array::new_with_length(bytes.len() as u32);
            arr.copy_from(&bytes);
            let _ = self.bc.post_message(&arr);
        }
        #[cfg(not(target_arch = "wasm32"))]
        let _ = bytes;
    }

    pub fn drain_inbound(&self) -> Vec<Vec<u8>> {
        let mut guard = self.inbound.lock().expect("broadcast inbound lock");
        guard.drain(..).collect()
    }
}

/// Map a `?room=` value to a per-room channel name. Same-origin pages with
/// the same room name discover each other; different rooms are isolated.
pub fn channel_name_for_room(room: &str) -> String {
    format!("archimedes:{room}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn channel_name_includes_room() {
        assert_eq!(channel_name_for_room("alpha"), "archimedes:alpha");
        assert_eq!(channel_name_for_room("default"), "archimedes:default");
    }

    #[test]
    fn open_returns_none_on_native() {
        assert!(BroadcastChannelTransport::open("test").is_none());
    }
}
