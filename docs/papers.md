# Paper notes for archimedes

Ranked paper list and extracted concepts for checkpoints 5-8 and the
Atomic Semi cover letter / README. Do not re-run the research agents —
all output from this file is already distilled.

---

## Must-read (3)

### Hobby (1999) — Practical segment intersection with finite precision output
*Computational Geometry 13(4).* PDF: https://ect.bell-labs.com/who/hobby/93_2-27.pdf

Snap-rounding paper. Defines a tolerance square ("hot pixel") around every
grid point, bends any segment passing through it so the segment visits the
pixel center. Bounds rounding error by construction instead of iterating.
Second-pass bolted onto Bentley-Ottmann sweep, ~25-75% overhead.

**Factual correction to note**: The paper itself is motivated by map data
for compression, not VLSI masks. The Atomic Semi tie-in is Hobby's broader
Bell Labs career, which does cover VLSI. Do not claim this specific paper
is "about VLSI." Honest framing: "snap-rounding came out of Hobby's Bell
Labs work."

**Cover-letter draft opener (honest lineage version)**:
> Robust planar geometry is the quiet pre-requisite for mask layout: a
> silent orientation flip in a float predicate produces a polygon that
> winds the wrong way, a boolean that paints the inverse region, and a
> reticle that ruins a lot. The lineage archimedes draws from — Shewchuk's
> adaptive predicates, Hobby's snap-rounding — exists because Bell Labs
> learned the hard way that finite-precision geometry without care eats
> fabs.

**Robustness-tab demo idea worth stealing**: Recreate Hobby's Figure 1.
Two draggable crossing segments on an integer grid. Toggle between "naive
round the intersection" (extraneous crossings appear) and "snap-round
with tolerance squares" (bent segments meet cleanly at pixel center).
Dashed tolerance squares overlaid on visible grid.

**Interview points**:
- Key move: treat each grid point as a unit tolerance square, bend any
  segment passing through it — bounds rounding error by construction.
- It's a second pass on Bentley-Ottmann; Pass 1 finds intersections,
  Pass 2 inserts tolerance-square vertices. 25-75% overhead, not a
  constant-factor blowup.
- The guarantee is topological, not metric: output segments can shift by
  up to half a grid unit. That's the tradeoff every finite-precision CAD
  kernel has to make explicit.

---

### Kettner, Mehlhorn, Pion, Schirra, Yap (2008) — Classroom Examples of Robustness Problems in Geometric Computations
*Computational Geometry 40(1).* PDF: https://people.mpi-inf.mpg.de/~mehlhorn/ftp/ClassroomExamples.pdf

Gallery of small inputs that break naive float geometry. Verified contents
below — coordinates are transcribed from the paper.

**Preload this exact degenerate config in the Robustness tab** (Figure 2b,
Section 3, p. 5 — intended y = x):

```
p = (0.5, 0.5)
q = (17.3000000000000185, 17.3000000000000185)
r = (24.00000000000005, 24.00000000000005177)
```

Naive `orient2d` returns **three different signs depending on pivot
choice** — same triple. Correct answer is 0 (collinear).

**Best demo setup**: three seed points above + dense sweep of query
points along the diagonal under both predicates. Naive Delaunay fans
flicker / spade throws / non-triangulation. Robust predicates: stable
clean fan every time.

**Backup degenerate**: Figure 5, Section 4.2 hull configuration (Failure
A1). Nine points, p4 sits outside triangle (p1,p2,p3) but naive
`orient2d(p3,p1,p4)` returns wrong sign, so incremental hull omits
extreme lower-left. Good visible hull-style failure if the Delaunay
demo isn't photogenic.

**Citation-ready pull quote** (short, dropable in sidebar as blockquote):
> "The choice of the pivot makes a difference, but nonetheless the
> geometry remains non-trivial and sign reversals happen for all three
> choices."
> — Kettner et al. 2008, p. 6.

**Robustness-tab sidebar copy (draft, ~130 words)**:
```
Setup
Three seed points on y = x:
  p = (0.5, 0.5)
  q = (17.3000000000000185, ...)
  r = (24.0000000000000518, ...)
Plus a dense sweep of query points
near the diagonal. Toggle predicates.

What you see
Naive f64 orient2d: Delaunay fans
flicker, edges cross, spade throws
or returns a non-triangulation.
Shewchuk adaptive (via `robust`):
one clean, stable fan. Every time.

Why it matters
A mask layout is millions of nearly
collinear edges. One silent orient
flip -> a polygon winds the wrong
way -> the rasterizer paints the
inverse region -> the wafer is
scrap. Exact predicates are not
academic; they are the difference
between a working reticle and a
dead lot.
```

(Kept the framing generic — "a dead lot" not "at Atomic Semi scale" — the
demo UI shouldn't name-drop the company, cover letter is where that goes.)

---

### Shewchuk (1996) — Triangle: Engineering a 2D Quality Mesh Generator and Delaunay Triangulator
*Applied Computational Geometry, LNCS 1148.* PDF: https://people.eecs.berkeley.edu/~jrs/papers/triangle.pdf

Gold-standard engineering writeup of a Delaunay + quality-mesh
implementation. Focus §1 (motivation), §5 (robustness: exact arithmetic
+ filtered predicates + incremental construction).

**Note**: Research agent could not fetch this PDF; contents below are
reconstructed from general Triangle knowledge + Shewchuk's 1997 adaptive-
predicates companion. Treat specific section framing as paraphrase.

**Three engineering pillars**:
- Exact arithmetic predicates — orient2d / incircle computed with
  adaptive-precision expansions so the sign is always provably correct.
- Filtered (adaptive) evaluation — cheap f64 approximation with a
  certified error bound, escalates to exact only when the bound says the
  sign is uncertain. Common case stays near hardware speed.
- Incremental construction on a compact adjacency structure (triangle-
  based, O(1) neighbor access) — locality of updates plus clean
  flip/retriangulation.

**How this maps to `spade`**: `spade` inherits Shewchuk-style filtered
exact predicates (via the `robust` crate, which is a direct Rust port of
Shewchuk's expansions) and Bowyer-Watson-style incremental insertion. It
likely diverges on data structure — Rust CDT crates tend to favor
half-edge/DCEL over triangle-based records for cleaner Voronoi duals.

**Vocabulary glossary** (for interview fluency):
- `orient2d` test — sign of a 3x3 determinant: is point C left of, right
  of, or on line AB?
- `incircle` test — sign of a 4x4 determinant: does point D lie inside,
  outside, or on the circumcircle of triangle ABC? THE Delaunay criterion.
- Lawson flip — local edge swap that restores the Delaunay property
  between two adjacent triangles failing the incircle test.
- Bowyer-Watson — incremental insertion that deletes all triangles whose
  circumcircle contains the new point and retriangulates the resulting
  cavity. This is what `spade` does.
- Adaptive-precision expansion — sum of non-overlapping floats
  representing a real number exactly, grown only as far as needed to
  determine a predicate's sign.

**Interview talking points**:
- Naive float predicates don't just give wrong answers — they produce
  non-existent triangulations that loop forever or corrupt the mesh.
  Robustness is a correctness issue, not an accuracy one.
- Filtered predicates win because almost every input is easy: the
  expensive exact path runs on a tiny fraction of calls. Adaptive
  precision is essentially free on average.
- Bowyer-Watson and incremental flip-based insertion converge to the
  same triangulation but differ in how cleanly they compose with exact
  predicates and constraint insertion.
- Degeneracies (four cocircular points, collinear points) are where
  real implementations fall apart. Tie-breaking must be consistent
  across predicates or the mesh becomes non-manifold.

**README Tab 2 one-liner**:
> Tab 2 uses `spade`, a Rust Delaunay library in the tradition of
> Shewchuk's Triangle — the canonical engineering writeup showing that a
> fast 2D mesh generator needs filtered exact-arithmetic predicates
> married to an incremental insertion algorithm.

---

## Skim if time (2)

### Barber, Dobkin, Huhdanpaa (1996) — The Quickhull Algorithm
Engineering paper behind Qhull / SciPy / MATLAB. "Thick facets" is a 5-min
interview story: how production code handles float fuzz — direct parallel
to mask tolerances.

### Papadopoulou & Lee (1999) — Critical Area Computation via Voronoi Diagrams
*IEEE TCAD 18(4).* PDF:
https://www.inf.usi.ch/faculty/papadopoulou/publications/tcad99.pdf

Single most on-the-nose semiconductor citation. Weighted / L∞ Voronoi for
IC yield analysis. One sentence in the Voronoi tab README earns its keep.
Also — this paper is the theoretical backing for the Critical-area demo
proposed as Checkpoint 6b in `concepts.md`.

---

## Keep as reference (cite without reading)

- **Martinez, Rueda, Feito 2009** — sweep-line alternative to Greiner-
  Hormann that `i_overlay` conceptually descends from. Name-drop in
  Polygon Ops tab README.
- **Brönnimann, Burnikel, Pion 2001** — filtered-predicates reference
  that justifies why `robust` crate is fast AND correct. Pairs with
  Shewchuk 1997.
- **de Berg, Halperin, Overmars 2007 — Snap rounding revisited** — cite
  as Hobby follow-up showing awareness of modern variants.
