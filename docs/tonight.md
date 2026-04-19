# Tonight — Archimedes iteration plan

Session brief for a 10-14 hour push that takes Archimedes from "polished CG
playground" to "resource a DCG-aware reviewer will bookmark." Application
deadline is Tuesday night, with one buffer day. Portfolio landing card is
being handled in a separate session; this plan is pure Archimedes.

This document is load-bearing. I will likely hit context compaction during the
session — when that happens, this file is the re-entry point. Every checkpoint
below has acceptance criteria concrete enough to resume from cold.

---

## Goal statement

A DCG-aware reviewer at Atomic Semi opens the deployed URL, spends three
minutes in the app without reading any outside docs, and walks away with two
impressions:

1. **This person knows computational geometry.** The demos state real
   invariants, the citations are correct, the predicates are adaptive, the
   visualizations expose the underlying math (duality, empty-circumcircle,
   error bounds) rather than hiding it.
2. **This person writes production code.** UI is disciplined, typography is
   consistent, state is reproducible (seeds), performance is measured and
   displayed, tests exist, clippy is clean, history is legible.

Secondary goal: make it a resource other people link to. Someone teaching a
CG course forwards `huecodes.github.io/Archimedes/#delaunay` to a student.
That is the "cool resource" target.

---

## Definition of done-done

All of the following true before GitHub Pages is flipped on:

1. `cargo build --release` clean on native
2. `cargo build --release --target wasm32-unknown-unknown` clean
3. `cargo test --release` — ≥ 20 tests pass (baseline is 17; duality + power
   + Euler invariant test add at least 3)
4. `cargo clippy --release` — zero warnings
5. `trunk build --release` — wasm bundle builds, `dist/` populated, bundle
   under 5 MB gzipped
6. Each tab opens to a non-empty default state that implicitly explains what
   to do
7. Each tab sidebar has, in order: `EXPLAINER` (new), `ALGORITHM`, `INVARIANT`,
   `LIVE`, domain-specific sections, `REFERENCES`
8. Status-bar left and right labels share a vertical baseline
9. Hull tab has a working Duality toggle with cross-highlighted hover
10. Manual 10-minute walkthrough recording: open each tab, exercise every
    toggle, drag a point, click Random, click Reset, observe fade transitions
11. README reflects every user-visible change (toggle list, new sections, new
    controls)

Done-done explicitly does **not** require:

- Power diagrams (stretch only)
- Mobile UX beyond "not awful"
- SVG export
- URL-encoded state beyond seeds already shown in status bar

---

## Checkpoint C1 — verification and housekeeping (30-45 min)

This checkpoint exists to surface anything broken from the prior commit and
clear obvious blockers before narrative and duality work.

**Steps**

1. `cargo run --release` and walk through each tab
2. Verify the Robustness band renders as designed:
   - Default preset: visible strip hugging the AB line, thickness roughly
     1-2 cells (at 96 cells that's ~12 px)
   - Drag B to (1500, 1500): band should widen visibly
   - Drag B to (50, 50): band should shrink / vanish
3. Verify canvas border visible but not crossing panel edges
4. Verify layer fades smooth on D/V tab (Voronoi cells, Delaunay edges,
   circumcircles-on-hover, empty-circumcircles)
5. Verify Euler counts on Polygon Ops tab are sensible:
   - Default pentagon + rectangle union: V = E ≈ 8-10, F = 3 (one filled,
     one hole-free ring, one unbounded), components = 1, invariant OK
   - Change op to Intersect: V/E stay similar, F = 2, invariant OK
6. Fix status-bar alignment (see Open Questions for my best-guess root cause)

**Acceptance**

- [ ] Manual walkthrough clean: no visible rendering bugs, no panics, no
      unexpected behavior
- [ ] Status-bar fix committed
- [ ] No regressions from prior session's commit
- [ ] One commit: "Verify polish pass, fix status-bar alignment"

**Blocker protocol**: if anything is visibly broken that isn't trivially
fixable in ≤ 20 min, commit a stub revert, flag in this file's "open
questions," and move on. Do not derail the session.

---

## Checkpoint C2 — narrative and pedagogy pass (2-3 hrs)

The single highest-ROI move tonight. Adds a new `EXPLAINER` section to every
tab plus targeted additions per tab. Positions Archimedes as a resource, not
just a demo.

### C2.1 — EXPLAINER section on every tab

New section, inserted at the top of every tab's sidebar (before `ALGORITHM`).
Content rules:

- 2-3 sentences, 40-80 words
- Proportional font, size 13
- Written for a viewer who has not read any docs, but who knows basic
  geometry (expects the reader to know what a polygon is, not what
  orient2d is)
- States "what you are looking at" and "why this is non-trivial"
- Distinct from INVARIANT, which states the theorem the demo upholds

Strawman drafts (polish during the pass):

**Convex Hull** — The smallest convex polygon containing every point. Andrew's
monotone chain sorts by x and sweeps the sorted list twice, popping any point
that would make a right turn. Dominated by the sort at O(n log n), and
numerically delicate: the sign of the orient2d test decides every pop.

**Delaunay / Voronoi** — Two views of the same structure. Voronoi cells
partition the plane by nearest site; Delaunay triangulates sites whose cells
meet along an edge. The empty-circumcircle property makes Delaunay the most
balanced triangulation — it maximizes the minimum angle over all possible
triangulations of the same points.

**Polygon Ops** — Boolean set operations on simple polygons, handled by a
sweep-line over edge-intersection events. i_overlay is designed to survive the
degenerate cases that crash naive implementations: shared edges, coincident
vertices, nested contours, self-intersection.

**Critical Area** — VLSI yield modeling. A point defect of radius r causes a
short between features A and B iff its center lies in the Minkowski
intersection `dilate(A, r/2) ∩ dilate(B, r/2)`. Integrating that area over a
defect-radius distribution gives the expected number of shorts per wafer.

**Robustness** — `orient2d(a, b, c)` on naive f32 can silently return the
wrong sign for near-collinear inputs, because the subtraction of two near-equal
products eats its significant bits. Every downstream algorithm — hull, Delaunay,
boolean ops — branches on this sign, so one silent flip can ruin an otherwise-
correct pipeline. Shewchuk's adaptive predicates promote to extended precision
exactly when the error bound crosses zero.

### C2.2 — per-tab targeted additions

**Convex Hull**
- Inline formula row below ALGORITHM: `orient(a, b, c) = (b.x−a.x)(c.y−a.y) − (b.y−a.y)(c.x−a.x)` in monospace, size 11
- Below it: "> 0 left turn · < 0 right turn · = 0 collinear"

**Delaunay / Voronoi**
- Step-through mode. Requires a Bowyer-Watson event recorder similar to
  `HullPlan`. Keyboard controls: `←` back one step, `→` advance one step,
  `Space` play/pause, `R` reset to start. Implementation sketch in C2.2 notes
  below.
- Visual: current step highlights the inserted site in WARN, the "bad
  triangles" (those whose circumcircle contains the new site) stroked in WARN,
  and the new flipped edges briefly in OK.

**Polygon Ops**
- Skip step-through — the sweep events live inside i_overlay and our code
  doesn't see them.
- Add a LEGEND section between INVARIANT and OPERATION: fill/stroke swatches
  showing what A, B, and result look like.

**Critical Area**
- Add inline formula below ALGORITHM: `CA(r) = area( dilate(A, r/2) ∩ dilate(B, r/2) )`
- Add a second formula: `expected_shorts = ∫ CA(r) · p(r) dr` where p(r) is
  the defect size distribution (typically Rayleigh). Cite Stapper.

**Robustness**
- Add below READOUT: "Shewchuk static bound: N" where N is the computed error
  bound. Makes the otherwise-abstract number visible and the relationship
  between signal and bound concrete.

### C2.3 — step-through mode (Delaunay)

New state on `DelaunayVoronoiDemo`:

```rust
struct BowyerAnim {
    plan: BWPlan,
    step: usize,
    playing: bool,
    last_tick: Instant,
}

struct BWPlan {
    // Events emitted in order by a dry-run of Bowyer-Watson.
    events: Vec<BWEvent>,
    // Snapshot of triangulation state at each event boundary, so replay is cheap.
    // (Alternative: re-run up to `step` on each frame. For n ≤ 200 sites that's fine.)
}

enum BWEvent {
    ConsiderSite(usize),          // about to insert site[i]
    FoundBadTriangle(FaceId),      // circumcircle contains the new site
    RemoveBadTriangle(FaceId),     // dig the cavity
    AddEdge(VertexId, VertexId),   // star-fan the new vertex
}
```

Minimum viable implementation: re-run spade up to `step` sites on each frame
and render; the event granularity is "per site inserted" rather than
"per flip." That's enough to convey the algorithm without a bespoke implementation.
If time allows, decompose per-flip.

### C2.4 — typography + spacing pass

- Verify section spacing is 14 px everywhere (there are currently `ui.add_space(14.0)`
  calls; audit for consistency)
- All numeric values monospace, all prose proportional (audit existing code)
- Metric rows align vertically — check that label + value columns line up
  across different tabs' sidebars
- Verify section-header 10.5 px monospace is consistent

**Acceptance (C2)**

- [ ] Every tab has an EXPLAINER section with finalized prose
- [ ] Delaunay step-through works at "one site per step" granularity, with
      keyboard controls documented in the left panel's SHORTCUTS section
- [ ] Inline formulas land on Hull, Critical Area, Robustness per the above
- [ ] Typography audit complete; any fixes committed
- [ ] Commit: "CP9: per-tab EXPLAINER, inline formulas, Delaunay step-through"

---

## Checkpoint C3 — Duality view on the Convex Hull tab (1-2 hrs)

**The math wow move.** A classical result in DCG: point-line duality maps
`(a, b) → y = ax − b`. Under this duality, the upper convex hull of a point
set corresponds to the upper envelope of the dual lines. Showing this live
is the single most likely thing tonight to make a math-aware reviewer realize
you know real geometry.

### C3.1 — design

**Toggle**: new checkbox in Hull sidebar, between LIVE and ANIMATION sections:
`[ ] Show duality view`. Off by default.

**Layout when on**: horizontal split of the canvas. Top half = existing point
view. Bottom half = dual plane.

Why horizontal: the dual lines span a wide x range; compressing that range
would make the upper envelope structure harder to read. Vertical split wastes
horizontal space. (Flagging this in Open Questions; I can pivot if asked.)

**Dual plane axes**:
- x axis: slope of the dual line (= source point's x-coordinate)
- y axis: y-intercept is `−b`; render y-values in [−300, 300] or autoscaled
- A faint grid at both x=0 and y=0 axes for visual orientation
- No tick labels — rely on visual cleanliness (per CRAFT.md "don't put long text on canvas")

**Rendering**:
- Every point in the top pane has a corresponding dual line in the bottom pane.
- Color: inactive points and their lines in `FG.linear_multiply(0.5)`; hull
  points and their lines in `ACCENT`.
- The upper envelope of the dual lines is stroked in `ACCENT` width 2.25 with
  the same 0.15-alpha glow used for the hull stroke.
- Hovered point → both point and dual line render in `WARN`. Reverse also
  true: hovering a dual line highlights its source point.

**Computation**:
- Point `(a, b)` maps to line `y = ax − b` (standard point-line duality).
- Upper envelope = lower envelope of dual lines when lifted — but because our
  y-axis is screen-flipped, care needed. I'll derive the formula once and
  write it as a comment.
- Upper envelope of n lines computed in O(n log n) by a deque-based scan
  after sorting lines by slope. Output: sorted list of breakpoints and the
  line segment active between each.

### C3.2 — implementation outline

New module: `src/demos/convex_hull_duality.rs` (or inline in `convex_hull.rs`
if under 150 lines). Exports:

```rust
pub fn upper_envelope(lines: &[Line]) -> Vec<LineSegment> { ... }
fn point_to_line(p: Pos2) -> Line { Line { slope: p.x, intercept: -p.y } }
```

New state on `ConvexHullDemo`:

```rust
pub struct ConvexHullDemo {
    // existing fields...
    show_duality: bool,
    hover_idx: Option<usize>,  // index of hovered point, for cross-highlighting
}
```

Rendering path: when `show_duality` is on, `allocate_painter` takes only the
top half of `ui.available_size()`, then a separate painter for the bottom
half. Both get the same grid background; the bottom one also gets the dual
axes.

### C3.3 — acceptance

- [ ] Toggle present in sidebar, default off
- [ ] With toggle on, canvas splits into two panes; top pane behaves exactly
      as before
- [ ] Dual lines render in bottom pane, one per point, colored by hull
      membership
- [ ] Upper envelope stroked in the bottom pane matches the hull in the top
      pane (visual check)
- [ ] Hovering a point in the top pane highlights the corresponding dual line
      in the bottom, and vice versa
- [ ] Test `upper_envelope_matches_hull_under_duality`: for 20 random seeds,
      verify that the points lying on the upper hull are exactly the points
      whose dual lines appear in the upper envelope
- [ ] Performance: 60 fps with 100 points and toggle on
- [ ] Commit: "CP10: point-line duality view on Convex Hull tab"

**Explicit non-goals for duality**:
- Lower envelope / lower hull view
- Editing points in the dual pane
- Axis labels and tick marks
- Animating the transition from point to line

---

## Checkpoint C4 — Power diagrams (STRETCH, 2-4 hrs)

Only if C1-C3 are shipping clean. Per `docs/concepts.md` Tier 1 #2. This is
the "Atomic-Semi-on-the-nose" stretch move.

### C4.1 — design

**Toggle**: `[ ] Weighted (power diagram)` in the Delaunay/Voronoi sidebar's
LAYERS section.

**Weight editing**: each site gets a "radius handle" — a ring drawn around it
at `w_i` pixels. Dragging the ring tangentially resizes the weight (drag out
= grow, drag in = shrink, can go negative). Initial weights: all 0 (reduces
to standard Voronoi).

**Rendering**: when toggle is on, replace standard Voronoi cell computation
with power-cell computation. Cell palette unchanged. Sites whose power cell
is empty (dominated by a neighbor) render dimmed.

### C4.2 — math

Power cell of site i:
```
cell_i = { p : d²(p, s_i) − w_i ≤ d²(p, s_j) − w_j  ∀j ≠ i }
```

Each inequality defines a halfplane in p:
```
2p·(s_j − s_i) ≤ (|s_j|² − |s_i|²) + (w_i − w_j)
```

Intersection of halfplanes = convex polygon. Clip against the viewport as
the final step. We already have Sutherland-Hodgman for halfplane clipping.

### C4.3 — implementation outline

New module: `src/geometry/power_diagram.rs`.

```rust
pub fn compute_power_cell(
    site_idx: usize,
    sites: &[Pos2],
    weights: &[f32],
    viewport: Rect,
) -> Vec<Pos2>
```

For each `j ≠ i`, compute the halfplane and clip the running polygon (starts
as the viewport rect) against it. Returns the cell polygon.

Total cost: O(n²) per frame at n ≤ 100 sites. At 100 sites that's 10k
halfplane clips per frame — roughly 1-2 ms in release.

Synchronization with PointEditor: add a `weights: Vec<f32>` field on the
demo struct. When `editor.version()` changes, resize `weights` to match
`editor.points().len()` (extending with 0.0, truncating as needed).

### C4.4 — acceptance

- [ ] Toggle renders power cells when on, Voronoi when off
- [ ] Radius handle drags modify weights; cells update live
- [ ] Test `power_cells_equal_voronoi_at_zero_weights`: for a fixed seed,
      all cells with weights[i] = 0 match the Voronoi output
- [ ] Test `heavy_weight_dominates_neighbors`: one site with w = 1000,
      others with w = 0, result is the heavy site's cell covering the
      entire viewport
- [ ] Performance: 60 fps at 50 sites with toggle on
- [ ] Commit: "CP11 (stretch): power diagrams with per-site weights"

---

## Checkpoint C5 — ship (30-60 min)

Executed sequentially, no exceptions.

**Steps**

1. `cargo clippy --release` — zero warnings
2. `cargo test --release` — all tests pass
3. `cargo build --release` — clean
4. `cargo build --release --target wasm32-unknown-unknown` — clean
5. `trunk build --release` — populate `dist/`
6. Check bundle size: `du -sh dist/` — target < 5 MB
7. Update README:
   - Per-tab summary table reflects new EXPLAINER, Euler on Polygon Ops,
     empty-circumcircles toggle, duality toggle, (stretch) power diagrams
   - Screenshot slots updated with fresh captures if visual changes are
     significant
8. Commit + tag:
   - `git tag -a v0.2 -m "Duality, Euler on Polygon Ops, empty-circumcircles, narrative pass"`
   - Push with tag
9. GitHub Pages enable:
   ```
   gh api -X POST repos/HueCodes/Archimedes/pages -f 'source[branch]=gh-pages' -f 'source[path]=/'
   ```
10. Wait 30-60 seconds, open `https://huecodes.github.io/Archimedes/`
11. Smoke test the live URL in a fresh browser tab: every tab loads, every
    toggle works, no console errors
12. Announce the URL to the other Claude (so the portfolio card can link)

**Acceptance (C5)**

- [ ] All commands above exit clean
- [ ] README accurate
- [ ] Live URL renders correctly
- [ ] Tag pushed

---

## Cut order if behind

Cut bottom-up. Never cut C1 or C5 — they are the floor.

1. **C4 Power diagrams** — already deferred once. Fine to defer again.
2. **C2.3 Delaunay step-through** — keep the EXPLAINER prose additions, cut
   the Bowyer-Watson animation. It's the most involved piece of C2.
3. **C3 Duality view** — the wow move, cut last. If cutting, leave a sidebar
   citation under REFERENCES linking to Edelsbrunner's *Algorithms in
   Combinatorial Geometry* §1 on duality, so the *concept* is still surfaced
   even without the live demo.
4. **C2.2 per-tab additions** — keep at minimum the EXPLAINER section on
   every tab; formulas and visual legends can be trimmed.

If we are cruising fast (C1-C5 ship before midnight), the unused budget goes
to:
- A Welzl smallest-enclosing-disk toggle on the Hull tab (concepts.md Tier 2 #7)
- SVG export per CRAFT.md

But those are explicit extras. No implicit scope creep.

---

## Out of scope tonight

- The portfolio site card (separate Claude)
- Mobile UX beyond "panels fit on desktop sizes" — call out mobile
  limitation in README
- Snap-rounding demo (concepts.md Tier 2 #5)
- Jump-flood GPU Voronoi (concepts.md Tier 2 #4)
- Exact-predicates comparison battery (concepts.md Tier 3)
- URL state encoding beyond the seed field already shown in status bar
- Refactoring the `*_sidebar` functions into a macro (low value, covered
  in prior session's refactor triage)

---

## Open questions — need answers before the relevant checkpoint

These gate work. If unanswered, I'll apply the marked default and flag it
in the commit body.

1. **Status-bar alignment root cause** *(C1)*. Without a screenshot, my
   best-guess fix is: inside `bottom_bar`, wrap both labels in a single
   `ui.horizontal(|ui| { ui.set_min_height(18.0); ... })` with both using the
   same `Layout::left_to_right(Align::Center)`, and split via a spacer rather
   than nested `with_layout`. If you can send a screenshot or describe the
   specific misalignment (vertical baseline? right label cut off? wrong
   spacing?), I'll target it more precisely. **Default: apply the best-guess
   fix.**

2. **Duality view split orientation** *(C3)*. Horizontal (top/bottom) or
   vertical (left/right)? Horizontal preserves x-axis range for the dual
   lines; vertical is less disruptive visually. **Default: horizontal split.**

3. **Landing tab on first load** *(C5)*. Currently Convex Hull. Delaunay/
   Voronoi is visually more impressive on first paint and might be a better
   "hello." **Default: keep Convex Hull** (simplest algorithm, most intuitive
   entry point for a non-CG viewer — though DCG-aware reviewers would be
   immediately impressed by DV too).

4. **Pages flip timing** *(C5)*. Flip at the end of C5 tonight, or wait
   until closer to the application? If the portfolio landing card needs the
   URL to link to, flipping tonight is required. **Default: flip tonight.**

5. **Default weights for power diagrams** *(C4, if we reach it)*. Start all
   at 0 (reduces to Voronoi) or use a position-derived nonzero pattern to
   make the power-diagram effect visible on first load? **Default: all zero
   + a one-shot "Randomize weights" button in the sidebar, so the effect is
   discoverable but not imposed.**

---

## Implementation notes and gotchas

**egui animate_bool state**: Already using `animate_bool` for toggles. Note
that the Id string matters — colliding Ids share animation state across
widgets. I've namespaced with prefixes (`dv_show_voronoi` etc.). Duality
toggle will use `hull_show_duality`.

**Step-through on Delaunay (C2.3)**: spade does not expose its internal
Bowyer-Watson events. Options: (a) track events by comparing triangulation
state before / after each site insertion; (b) reimplement Bowyer-Watson
ourselves; (c) use "per-site" granularity and just show snapshots. Option
(c) is the MVP for tonight. If time, (a) gives per-flip granularity.

**Duality upper envelope (C3)**: the deque-based algorithm is subtle around
parallel lines and coincident intercepts. Protect with an explicit test that
includes two collinear source points (they map to parallel dual lines).

**Power diagram weight-handle interaction (C4)**: the "drag the ring to
resize" gesture is elegant but has conflict with dragging the site itself.
Resolution: use a small triangular handle at the top-right of the ring (like
a graphic-design scale handle) for weight adjustment, reserve click-on-site
for dragging the site.

**Bundle size (C5)**: adding fonts (already done) and a couple of new small
modules shouldn't meaningfully change bundle size. If bundle > 5 MB, check
that no debug artifacts are included; if bundle > 8 MB, investigate.

---

## Session log (updated live)

Entries appended as checkpoints complete. Use timestamp format `HH:MM`.

- `xx:xx` — C1 started
- `xx:xx` — C1 complete, commit `<hash>`
- ...

---

## Reference aesthetics (reminder, do not re-audit)

Already absorbed into CRAFT.md. Relisting here for convenience during C2
typography pass:

- **redblobgames.com** — per-tab narrative structure, inline controls
- **ciechanow.ski** — no-chrome visualization aesthetic
- **desmos.com** — constraint interaction feel
- **observablehq.com** — reactive sidebar + main pane rhythm

The goal is discipline, not imitation.
