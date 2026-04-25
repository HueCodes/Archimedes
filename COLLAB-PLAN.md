# Archimedes — Collab Mode Plan

Multi-tab live collaboration for the Convex Hull tab. Pitch: *Figma-for-computational-geometry — drag a point in tab A, the convex hull updates in tab B in real time.*

Closes the two unchecked Atomic Semi JD nice-to-haves: **CRDT/WebSockets** and **Protobuf/schema-evolution**.

## Locked decisions

- CRDT lib: `yrs` (Yjs Rust port). Smaller WASM payload than automerge, mature.
- Wire format: `prost` Protobuf, `Envelope { oneof { ClientHello, DocUpdate, Presence } }`.
- Default transport: **`BroadcastChannel`** (browser-native cross-tab API, zero server, works on GitHub Pages). Falls back to **`WebSocket`** relay for cross-device dev (`?ws=ws://localhost:8787`).
- Sync model: server (when used) holds room doc, snapshots full state on join, blind-broadcasts updates after.

## Phase A — Local plumbing, no network

- **CP13** — `yrs` dep + `CollabDoc` skeleton (`src/collab/{mod,doc}.rs`). Methods: insert/move/delete point, points(), apply_remote_update, subscribe. Native `cargo test` proves two-doc convergence.
- **CP14** — Convex hull tab reads through `CollabDoc`. Single-tab behavior unchanged. Existing animations / duality view untouched.
- **CP15** — Two-doc in-process sync harness. Native test only.

## Phase B — Relay server (for cross-device fallback)

- **CP16** — Cargo workspace; move existing crate to `crates/archimedes/`, add `crates/relay/`.
- **CP17** — `axum` + `tokio-tungstenite` room broadcast. `?room=<name>`. ~150 LOC.
- **CP18** — Room owns a `yrs::Doc`; snapshot on join, apply+broadcast after.

## Phase C — Client networking

- **CP19** — WASM WebSocket via `gloo-net` with reconnect + status enum.
- **CP20** — `CollabDoc` ↔ WS bridge. **Throttle drag to 30Hz**. **Tag origin** to skip rebroadcast of remote updates (feedback-loop fix).
- **CP21** — Manual end-to-end test pass; findings → `docs/collab-known-issues.md`.

## Phase D — Protobuf wire format

- **CP22** — `prost` + `proto/messages.proto` + `build.rs` both crates.
- **CP23** — Replace raw bytes with `Envelope` framing.
- **CP24** — `## Schema evolution` section in README with the `.proto` inline.

## Phase E — Presence cursors

- **CP25** — Local cursor → `Presence` proto, throttle 30Hz, stable per-tab `client_id` in `localStorage`.
- **CP26** — Remote cursor rendering, color from id hash, drop after 3s idle.

## Phase F' — `BroadcastChannel` transport (replaces fly.io deploy)

- **CP27'** — `Transport` trait in `src/collab/transport.rs`. Two impls: `BroadcastChannelTransport` (default, WASM, zero config) and `WebSocketTransport` (opt-in via `?ws=…`).
- **CP28'** — README "Open two tabs at huecodes.github.io/Archimedes/" demo section. Relay crate docs as cross-device dev path.

## Phase G — Polish

- **CP30** — `wasm-opt -Os` audit, before/after bundle sizes in README.
- **CP31** — 60s demo recording (two tabs side-by-side, drag + presence cursors).
- **CP32** — README final pass: collab section near top, GIF, `.proto` inline, "Known shortcuts" subsection.

## Dependency graph

```
A: CP13 → CP14 → CP15
B: CP16 → CP17 → CP18           (CP16 blocks Phase C build)
C: CP19 → CP20 → CP21           (needs A complete + CP18)
D: CP22 → CP23 → CP24           (needs C complete)
E: CP25 → CP26                  (needs D — uses Envelope)
F': CP27' → CP28'               (needs C, D for full transport split)
G: CP30, CP31, CP32             (any order, after F')
```

## Cut list (cut from bottom up)

1. CP30 bundle audit — nice number, not load-bearing.
2. CP19 reconnect logic — manual page refresh OK for demo.
3. CP18 doc-aware server — fall back to "always join empty room".

**Never cut:** CP24 schema-evolution paragraph (the protobuf artifact), CP31 video, CP26 presence cursors (the wow moment).

## Estimate

Realistic: **22–32h focused**, ~10 days of mixed evening + weekend time on M2 8GB.
