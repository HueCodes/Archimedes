# Archimedes

An interactive computational-geometry playground — convex hulls, Delaunay /
Voronoi, polygon boolean operations, semiconductor critical-area, and a naive-vs-
robust predicate showdown — written in Rust, compiled to WebAssembly, rendered
via WebGPU through [`egui`](https://github.com/emilk/egui).

![Archimedes hero](docs/hero.gif)

**Live demo**: https://huecodes.github.io/Archimedes/

[![Convex hull tab](docs/screenshots/hull.png)](https://huecodes.github.io/Archimedes/)

## What's in it

<!--
  Screenshots referenced below live in docs/screenshots/ and must exist with
  these exact filenames before publishing the README:
    - hull.png
    - delaunay-voronoi.png
    - polygon-ops.png
    - critical-area.png
    - robustness.png
-->

| Tab | One-liner |
|---|---|
| **Convex Hull** | Andrew's monotone chain, animated step-through, live orientation-test counter; toggleable point-line duality view (upper hull ↔ upper envelope of dual lines) with bidirectional cross-highlight |
| **Delaunay + Voronoi** | Incremental Bowyer-Watson via `spade`; hover a site for its degree, cell area, and nearest neighbor; Euler `V − E + F = 2` readout; toggleable empty-circumcircle overlay; **step-through replay** with `← / →` and `Space` (bad triangles light up in WARN before the new vertex is wired in); **power-diagram (weighted Voronoi)** mode — scroll over a site to grow / shrink its weight, watch its cell engulf or vanish |
| **Polygon Ops** | Union / intersection / difference / xor / symmetric difference on two draggable polygons via `i_overlay`; click on any edge inserts a vertex at the projected point, right-click deletes; live Euler V/E/F + component count on the result |
| **Critical Area** | Two "wires" and a defect-radius slider; shades the region where a disk of radius `r` shorts them — the canonical semiconductor yield-analysis primitive |
| **Robustness** | Naive `f32` vs. Shewchuk adaptive `orient2d` on a near-degenerate point set; renders the untrustworthy band where the static error bound straddles zero, with a `\|naive\|/bound` ratio readout tier-colored by safety margin |

Screenshots: [Delaunay / Voronoi](docs/screenshots/delaunay-voronoi.png) · [Polygon Ops](docs/screenshots/polygon-ops.png) · [Critical Area](docs/screenshots/critical-area.png) · [Robustness](docs/screenshots/robustness.png)

## Real-time collaboration

The Convex Hull tab is a CRDT-backed shared document. Two browser tabs pointed at
the same room see each other's edits sub-100ms — drag a point in tab A, the hull
updates in tab B, presence cursors show where the other person is pointing. The
convergence is conflict-free (yrs / Yjs port) and the wire format is versioned
Protobuf.

The default transport is browser-native **`BroadcastChannel`** — same-origin tabs
of the deployed site discover each other automatically with no server, no
infrastructure, no setup. Just open two tabs of
[huecodes.github.io/Archimedes/](https://huecodes.github.io/Archimedes/) and start
dragging.

For cross-device sync (two laptops, two phones), opt into the WebSocket relay:

```sh
# Terminal 1: relay server
cargo run -p relay

# Terminal 2: dev server (from crates/archimedes/)
trunk serve
# Open in two browsers:
# → http://127.0.0.1:8080/?ws=ws://127.0.0.1:8787/ws&room=demo
```

A tab joining mid-session converges from a server snapshot without any sync
handshake.

### Wire format

`proto/messages.proto`:

```proto
syntax = "proto3";
package archimedes.v1;

message ClientHello { string room = 1; string client_id = 2; uint32 protocol_version = 3; }
message DocUpdate   { bytes yrs_update = 1; }
message Presence    { string client_id = 1; float x = 2; float y = 3; uint32 color = 4; uint64 ts_ms = 5; }
message Envelope    { oneof payload { ClientHello hello = 1; DocUpdate update = 2; Presence presence = 3; } }
```

### Schema evolution

- Field numbers are forever — never reuse, never re-type, mark removed fields
  reserved.
- Adding a new oneof variant (e.g., `EditCursor`, `Selection`) is non-breaking:
  old clients ignore unknown variants, old relays log and drop them.
- `protocol_version` is reserved for changes that can't be made non-breaking
  (e.g., a doc-encoding swap from yrs v1 to v2). The relay can refuse a hello
  with an incompatible major.

### Known shortcuts

- The relay broadcasts every relayed update back to the originator; the
  client tolerates the echo because yrs `apply_update` is idempotent for
  already-applied operations. A future `Envelope.client_id` field would
  let receivers skip self-traffic to save bandwidth.
- BroadcastChannel only sees same-origin same-browser tabs — the
  cross-device demo requires the relay.
- The y-sync protocol (sync step 1 / step 2) is skipped — joining clients
  receive the full state snapshot in their first frame instead.
- Rooms are in-memory only; a relay restart loses room state.
- Cursor and point coords are sent in canvas pixel space (cursor
  normalized, points raw); two clients with very different canvas sizes
  will see misalignment until normalization is extended to points.

## Why "Archimedes"

Archimedes approximated π by exhausting inscribed and circumscribed polygons —
the same orientation-test machinery that drives the convex-hull tab in the limit.

## Stack

- [`eframe`](https://crates.io/crates/eframe) 0.30 + [`egui`](https://crates.io/crates/egui) — UI, with `wgpu` rendering backend
- [`trunk`](https://trunkrs.dev/) — wasm build + dev server
- [`spade`](https://crates.io/crates/spade) — Delaunay / Voronoi (incremental Bowyer-Watson)
- [`i_overlay`](https://crates.io/crates/i_overlay) — polygon boolean operations
- [`robust`](https://crates.io/crates/robust) — Shewchuk adaptive-precision predicates
- [`yrs`](https://crates.io/crates/yrs) — Yjs CRDT, Rust port — backs the shared doc
- [`axum`](https://crates.io/crates/axum) + [`tokio-tungstenite`](https://crates.io/crates/tokio-tungstenite) — relay server
- [`prost`](https://crates.io/crates/prost) + [`protox`](https://crates.io/crates/protox) — Protobuf wire format, pure-Rust schema compile (no `protoc` needed)
- [`gloo-net`](https://crates.io/crates/gloo-net) — WebSocket client in WASM
- `web_sys::BroadcastChannel` — browser-native cross-tab transport, no server
- [`web-time`](https://crates.io/crates/web-time) — monotonic clocks that compile on both native and `wasm32`

## Build

```sh
cargo install trunk
rustup target add wasm32-unknown-unknown

# wasm dev server, no auto-reload
cd crates/archimedes
trunk serve --port 8080 --no-autoreload
# → http://127.0.0.1:8080/Archimedes/

# release build for deploy (from crates/archimedes/)
trunk build --release --public-url /Archimedes/

# native desktop build (same source, single-user — no collab)
cargo run -p archimedes --release

# relay server for cross-device collab
cargo run -p relay
```

## Algorithms implemented from scratch

- Andrew's monotone chain convex hull (`crates/archimedes/src/demos/convex_hull.rs`)
- 2D orientation predicate, naive `f32` and Shewchuk-robust variants (`crates/archimedes/src/geometry/primitives.rs`)
- Minkowski-style critical-area dilation via offset + intersection (`crates/archimedes/src/demos/critical_area.rs`)
- Reconciliation diff that mirrors per-frame `PointEditor` mutations into the CRDT (`crates/archimedes/src/demos/convex_hull.rs`, `reconcile_ops`)

## References

- Shewchuk, *Adaptive Precision Floating-Point Arithmetic and Fast Robust Geometric Predicates*, 1997
- Papadopoulou & Lee, *Critical Area Computation via Voronoi Diagrams*, 1999
- de Berg, van Kreveld, Overmars, Schwarzkopf, *Computational Geometry: Algorithms and Applications*, 3rd ed.
- Fortune, *A Sweepline Algorithm for Voronoi Diagrams*, 1987
- Greiner & Hormann, *Efficient Clipping of Arbitrary Polygons*, 1998
- Aurenhammer, *Power Diagrams: Properties, Algorithms, and Applications*, SIAM J. Comput., 1987

## License

MIT OR Apache-2.0, at your option.
