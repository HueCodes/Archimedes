# Archimedes

An interactive computational-geometry playground — convex hulls, Delaunay /
Voronoi, polygon boolean operations, semiconductor critical-area, and a naive-vs-
robust predicate showdown — written in Rust, compiled to WebAssembly, rendered
via WebGPU through [`egui`](https://github.com/emilk/egui).

**Live demo**: https://huecodes.github.io/Archimedes/

[![Convex hull tab](docs/screenshots/hull.png)](https://huecodes.github.io/Archimedes/)

## What's in it

| Tab | One-liner |
|---|---|
| **Convex Hull** | Andrew's monotone chain, animated step-through, live orientation-test counter; toggleable point-line duality view (upper hull ↔ upper envelope of dual lines) with bidirectional cross-highlight |
| **Delaunay + Voronoi** | Incremental Bowyer-Watson via `spade`; hover a site for its degree, cell area, and nearest neighbor; Euler `V − E + F = 2` readout; toggleable empty-circumcircle overlay; **power-diagram (weighted Voronoi)** mode — scroll over a site to grow / shrink its weight, watch its cell engulf or vanish |
| **Polygon Ops** | Union / intersection / difference / xor / symmetric difference on two draggable polygons via `i_overlay`; click on any edge inserts a vertex at the projected point, right-click deletes; live Euler V/E/F + component count on the result |
| **Critical Area** | Two "wires" and a defect-radius slider; shades the region where a disk of radius `r` shorts them — the canonical semiconductor yield-analysis primitive |
| **Robustness** | Naive `f32` vs. Shewchuk adaptive `orient2d` on a near-degenerate point set; renders the untrustworthy band where the static error bound straddles zero, with a `\|naive\|/bound` ratio readout tier-colored by safety margin |

Screenshots: [Delaunay / Voronoi](docs/screenshots/delaunay-voronoi.png) · [Polygon Ops](docs/screenshots/polygon-ops.png) · [Critical Area](docs/screenshots/critical-area.png) · [Robustness](docs/screenshots/robustness.png)

## Why "Archimedes"

Archimedes approximated π by exhausting inscribed and circumscribed polygons —
the same orientation-test machinery that drives the convex-hull tab in the limit.

## Stack

- [`eframe`](https://crates.io/crates/eframe) 0.30 + [`egui`](https://crates.io/crates/egui) — UI, with `wgpu` rendering backend
- [`trunk`](https://trunkrs.dev/) — wasm build + dev server
- [`spade`](https://crates.io/crates/spade) — Delaunay / Voronoi (incremental Bowyer-Watson)
- [`i_overlay`](https://crates.io/crates/i_overlay) — polygon boolean operations
- [`robust`](https://crates.io/crates/robust) — Shewchuk adaptive-precision predicates
- [`web-time`](https://crates.io/crates/web-time) — monotonic clocks that compile on both native and `wasm32`

## Build

```sh
cargo install trunk
rustup target add wasm32-unknown-unknown

# dev server, no auto-reload
trunk serve --port 8080 --no-autoreload
# → http://127.0.0.1:8080/Archimedes/

# release build for deploy
trunk build --release --public-url /Archimedes/

# native desktop build (same source)
cargo run --release
```

## Algorithms implemented from scratch

- Andrew's monotone chain convex hull (`src/demos/convex_hull.rs`)
- 2D orientation predicate, naive `f32` and Shewchuk-robust variants (`src/geometry/primitives.rs`)
- Minkowski-style critical-area dilation via offset + intersection (`src/demos/critical_area.rs`)

## References

- Shewchuk, *Adaptive Precision Floating-Point Arithmetic and Fast Robust Geometric Predicates*, 1997
- Papadopoulou & Lee, *Critical Area Computation via Voronoi Diagrams*, 1999
- de Berg, van Kreveld, Overmars, Schwarzkopf, *Computational Geometry: Algorithms and Applications*, 3rd ed.
- Fortune, *A Sweepline Algorithm for Voronoi Diagrams*, 1987
- Greiner & Hormann, *Efficient Clipping of Arbitrary Polygons*, 1998
- Aurenhammer, *Power Diagrams: Properties, Algorithms, and Applications*, SIAM J. Comput., 1987

## License

MIT OR Apache-2.0, at your option.
