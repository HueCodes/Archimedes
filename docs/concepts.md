# Computational-geometry concepts — archimedes additions

Survey of foundational + cutting-edge CG concepts with concrete archimedes
applications. Ranked by impression-per-hour for a semiconductor-CAD
reviewer, not theoretical depth. Use this to judge mid-weekend scope
decisions.

---

## TIER 1 — additions that fit the weekend if checkpoints 2-4 stay on time

### 1. Critical-area / Minkowski demo  **(THE Atomic Semi anchor)**

- Two draggable rectangles ("wires" on a mask). Slider for defect radius
  `r`. Shade the region where a disk of radius `r` causes a short or open
  between them.
- Implementation: `i_overlay` offset (expand rectangles by `r`) +
  intersection. ~1 afternoon on top of polygon-ops plumbing.
- Theoretical backing: Papadopoulou & Lee 1999 (see `papers.md`).
- **Slot as Checkpoint 6b (between Polygon Ops and Robustness).** Do not
  make it optional. This is the single most on-the-nose semiconductor
  tie-in the project could have. A screenshot of this alone says "I
  understand what Atomic Semi builds."
- Cut only if Checkpoint 6 (polygon ops) itself slips past Sunday noon.

### 2. Power diagrams / weighted Voronoi

- Per-site radius handle on the Voronoi tab. Interpolate from ordinary
  Voronoi (w=0) to a power diagram (weighted). Cells grow/shrink, some
  sites disappear entirely.
- Substantive CAD link: minimum-clearance between variable-width mask
  features is exactly a weighted Voronoi problem.
- Implementation: stretch within Checkpoint 5 if basic Voronoi lands
  early. ~1 day additional.
- **Cut first if behind.** Falls off naturally — basic Voronoi is
  enough for MVP.

### 3. Euler topology readout  **(free polish)**

- Add live `V / E / F` counters to the Voronoi and Polygon Ops tabs.
  Green/red indicator on `V − E + F = 2`. Flashes red if an invariant
  breaks during interaction.
- Reads as professional-grade debugging. Near-zero cost. Strong correctness
  signal.
- **Slot in Checkpoint 8 polish pass.**

---

## TIER 2 — portfolio v0.2 post-submit, or "skim & reference"

### 4. Jump-flood Voronoi on WebGPU

- Compute-shader JFA produces discrete Voronoi / distance fields in
  O(log n) passes over a texture, independent of seed count.
- Toggle in Voronoi tab: "exact CPU" vs "GPU JFA". Slider 10 → 1M
  sites. CPU chokes, GPU stays interactive.
- Genuinely uses the wgpu pitch. Too heavy for Sunday.
- **v0.2 — phenomenal addition.**

### 5. Iterated snap-rounding (Hershberger 2013/2017)

- Direct follow-up to Hobby 1999. Overlay a 1000-segment arrangement,
  snap-grid slider, highlight collapsed edges.
- "This is why DRC on an integer grid is non-trivial" interview hook.
- ~1 weekend if attempted. v0.2.

### 6. Geometric duality dual-view

- Hull tab side panel where each point becomes its dual line `y = ax − b`.
  Convex hull of points = upper envelope of lines.
- Hover a point, its dual line highlights.
- Afternoon of work. Unique visualization. Signals depth in a 5-sec look.
- Could slot as a Checkpoint 4 stretch if hull polish finishes early.

### 7. Smallest enclosing disk via Welzl

- Randomized incremental. Animates which points become "boundary" as
  constraints are added in random order.
- Mini-tool on the hull tab. Afternoon. Teaches backwards analysis
  visually.

---

## TIER 3 — read + cite only, do not build

- **Indirect predicates (Attene et al. 2020)** — one-line mention in
  Polygon Ops README: "a newer approach represents intersection vertices
  symbolically rather than constructing them numerically."
- **gDel2D / GPU parallel Bowyer-Watson (Qi-Cao-Tan)** — cite in Delaunay
  tab README paired with jump-flood: "CPU serial / GPU jump-flood / GPU
  parallel Bowyer-Watson — three regimes, three tradeoffs."
- **Exact Geometric Computation paradigm (Yap)** — one sidebar sentence
  in Robustness tab: "the theoretical scaffolding behind Shewchuk's
  adaptive predicates."

---

## If adding exactly one thing

**#1 (critical-area demo).** Most on-the-nose semiconductor tie-in the
project could possibly have. Achievable in an afternoon with crates
already in the stack. Elevates the project from "CG playground" to "toy
version of the thing Atomic Semi builds."

## If adding exactly two

**#1 + #3 (critical-area + Euler readout).** Critical-area is the
story; Euler readout is the polish that proves you're not just drawing
pictures.

---

## Cut list for Tier 1 if behind (update as of mid-weekend)

Cut bottom-up:
1. Power diagrams (#2)
2. Euler readout (#3) — cut only if Checkpoint 8 itself is impossible
3. Critical-area demo (#1) — only cut if Checkpoint 6 (polygon ops) also
   slipped. These are joined at the hip (shared `i_overlay` plumbing).

If all three Tier 1 items ship, the README Voronoi paragraph can honestly
cite Shewchuk, Hobby, Kettner, and Papadopoulou-Lee. That's a portfolio
piece.
