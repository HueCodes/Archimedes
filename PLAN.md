# archimedes — weekend build plan

Full reasoning in `~/Dev/resume-docs/Atomic-Semi/compgeom-playground-plan.md`.
This file is the execution checklist.

## Locked decisions (do not revisit)

- Stack: `eframe` 0.29 + `egui` + `trunk` + `wgpu` backend + `spade` + `i_overlay` + `robust`
- Four tabs: Convex Hull, Delaunay / Voronoi, Polygon Ops, Robustness
- Palette: Tokyo Night-ish (`#1a1b26` bg, `#c0caf5` points, `#7aa2f7` hull, `#f7768e` highlight)
- Deploy: GitHub Pages via the workflow in `.github/workflows/deploy.yml`

## Saturday

- [x] Scaffold: Cargo.toml, Trunk.toml, index.html, skeleton src/, CI, README (pre-built)
- [ ] 09:00-10:00 — `cargo install trunk && rustup target add wasm32-unknown-unknown && trunk serve`. Verify blank egui canvas loads at http://127.0.0.1:8080
- [ ] 10:00-12:00 — `git init`, create `HueCodes/archimedes` repo, first push, verify GitHub Pages deploy works end-to-end. Settings → Pages → deploy from `gh-pages` branch
- [ ] 12:00-15:00 — Shared point editor (click-add, drag-move, right-click-delete). Used across all tabs
- [ ] 15:00-18:00 — Convex hull polish: animated step-through, algorithm dropdown (monotone / Graham / Jarvis), complexity counters, "add 100 random" button
- [ ] 18:00-23:00 — Delaunay + Voronoi via `spade`. Dual-view render, colored cells, circumcircle hover
- [ ] 23:00 — tag v0.1, push, sleep

## Sunday

- [ ] 09:00-13:00 — Polygon boolean ops. Two draggable polygons, 5 ops via `i_overlay`
- [ ] 13:00-15:30 — Robustness demo. Naive vs. `robust::orient2d` toggle on degenerate preset
- [ ] 15:30-17:00 — Polish: palette, typography, sidebar explainer text, SVG export, keyboard shortcuts
- [ ] 17:00-18:30 — PID motor controller demo video (60-90s)
- [ ] 18:30-20:00 — Update huecodes.github.io with portfolio section
- [ ] 20:00-21:30 — Cover letter + resume PDF final pass
- [ ] 21:30-22:30 — Submit Ashby form

## Cut list if behind (cut from bottom up)

1. Robustness tab → README bullet instead
2. Polygon boolean ops tab
3. Voronoi circumcircle hover
4. SVG export
5. Jarvis march + Graham scan alternatives (keep monotone chain only)

## Must-never-cut

- Deploy pipeline working
- Convex hull tab
- README + screenshot
- PID video
- huecodes.github.io portfolio section
- Application submission
