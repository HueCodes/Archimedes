//! Generated Protobuf types for the collab wire format.
//!
//! Source schema lives at `proto/messages.proto` at the workspace root.
//! `build.rs` runs `protox` (pure-Rust) + `prost-build` to materialize
//! the types into OUT_DIR; we re-export them here.

#![allow(clippy::all)]
include!(concat!(env!("OUT_DIR"), "/archimedes.v1.rs"));

use prost::Message;

/// Encode an `Envelope` to bytes. Length-delimited framing is provided
/// by the WebSocket layer itself, so we encode the bare proto.
pub fn encode_envelope(env: &Envelope) -> Vec<u8> {
    let mut buf = Vec::with_capacity(env.encoded_len());
    env.encode(&mut buf).expect("envelope encode");
    buf
}

pub fn decode_envelope(bytes: &[u8]) -> Result<Envelope, prost::DecodeError> {
    Envelope::decode(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use envelope::Payload;

    #[test]
    fn doc_update_round_trip() {
        let env = Envelope {
            payload: Some(Payload::Update(DocUpdate {
                yrs_update: vec![1, 2, 3, 4, 5],
            })),
        };
        let bytes = encode_envelope(&env);
        let back = decode_envelope(&bytes).unwrap();
        match back.payload {
            Some(Payload::Update(u)) => assert_eq!(u.yrs_update, vec![1, 2, 3, 4, 5]),
            other => panic!("wrong payload: {other:?}"),
        }
    }

    #[test]
    fn presence_round_trip() {
        let env = Envelope {
            payload: Some(Payload::Presence(Presence {
                client_id: "abc".into(),
                x: 1.5,
                y: 2.5,
                color: 0xff8800,
                ts_ms: 1_700_000_000_000,
            })),
        };
        let bytes = encode_envelope(&env);
        let back = decode_envelope(&bytes).unwrap();
        match back.payload {
            Some(Payload::Presence(p)) => {
                assert_eq!(p.client_id, "abc");
                assert!((p.x - 1.5).abs() < 1e-6);
                assert_eq!(p.color, 0xff8800);
            }
            other => panic!("wrong payload: {other:?}"),
        }
    }

    #[test]
    fn unknown_payload_is_none() {
        // An empty Envelope (no oneof set) decodes cleanly with payload = None.
        let env = Envelope { payload: None };
        let bytes = encode_envelope(&env);
        let back = decode_envelope(&bytes).unwrap();
        assert!(back.payload.is_none());
    }
}
