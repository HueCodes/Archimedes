# Archimedes

An interactive computational-geometry playground — convex hulls, Delaunay / Voronoi,
polygon boolean operations — written in Rust, compiled to WebAssembly, rendered via
WebGPU through [`egui`](https://github.com/emilk/egui).

**Live demo**: https://huecodes.github.io/Archimedes/ *(deployed after first push)*

## Why "Archimedes"

Archimedes approximated π by exhausting inscribed and circumscribed polygons —
the same orientation-test machinery that drives the convex-hull tab in the limit.

## Stack

- [`eframe`](https://crates.io/crates/eframe) + [`egui`](https://crates.io/crates/egui) — UI, with `wgpu` rendering backend
- [`trunk`](https://trunkrs.dev/) — wasm build + dev server
- [`spade`](https://crates.io/crates/spade) — Delaunay / Voronoi (incremental Bowyer-Watson)
- [`i_overlay`](https://crates.io/crates/i_overlay) — polygon boolean operations
- [`robust`](https://crates.io/crates/robust) — Shewchuk adaptive-precision predicates

## Build

```sh
cargo install trunk
rustup target add wasm32-unknown-unknown

# dev (auto-reload)
trunk serve

# release build for deploy
trunk build --release

# native desktop build
cargo run --release
```

## Algorithms implemented from scratch

- Andrew's monotone chain for convex hull (`src/demos/convex_hull.rs`)
- 2D orientation predicate, naive and robust variants (`src/geometry/primitives.rs`)

## References

- Shewchuk, *Adaptive Precision Floating-Point Arithmetic and Fast Robust Geometric Predicates*, 1997
- de Berg, van Kreveld, Overmars, Schwarzkopf, *Computational Geometry: Algorithms and Applications*, 3rd ed.
- Fortune, *A Sweepline Algorithm for Voronoi Diagrams*, 1987
- Greiner & Hormann, *Efficient Clipping of Arbitrary Polygons*, 1998

## License

MIT OR Apache-2.0, at your option.
