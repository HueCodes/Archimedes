# Saturday morning prompt

Open a fresh Claude Code session in `~/Dev/projects/archimedes/` and copy
everything between the two `=====` lines below into the first message.

=====BEGIN=====

Context: I'm Hugh. Building archimedes, a Rust + WebAssembly computational-geometry
playground, as the portfolio centerpiece for an Atomic Semi application. I'm
targeting a Sunday-night submit but quality matters more than the clock. If
something needs an extra hour, take the hour. I'll cut scope before I cut
quality.

Environment:
- MacBook Air 8 GB (Apple Silicon). Prefer debug builds during iteration; release
  builds only at deploy time to avoid RAM pressure.
- Tailscale is set up if I need to offload release builds to my dad's Mac mini
  (only if Activity Monitor shows yellow memory pressure).
- Project lives at ~/Dev/projects/archimedes/. GitHub repo HueCodes/archimedes
  exists but has zero commits yet.

READ FIRST, before any code:
1. ~/Dev/resume-docs/Atomic-Semi/compgeom-playground-plan.md  (full spec)
2. ./CRAFT.md                                                  (visual polish spec)
3. ./PLAN.md                                                   (hour-by-hour)

Then summarize back to me in 5 bullets what you understand about: stack,
four tabs, palette, craft priorities, and the cut list. I'll confirm before
you start coding.

Hard rules:
- Never push, commit to a public remote, open a PR, or take any action
  visible outside my machine without asking. Local commits are fine after
  you've shown me the diff.
- No emojis anywhere - not in code, comments, commit messages, UI, or chat.
- Commit messages and PR descriptions are short: one sentence of what + why.
- Stack is locked: eframe 0.29 + egui + trunk + spade + i_overlay + robust.
  Do not propose alternatives unless a dep is actually broken.
- If a crate version in Cargo.toml doesn't resolve, pin to what compiles and
  leave a `# TODO(hugh): revisit once upstream publishes x.y.z` comment.
  Don't silently switch crates.
- Code from scratch only what the plan says to code from scratch (convex hull,
  orientation predicate). Everything else uses the pinned crates.

Behavioral rules:
- Stop at every numbered checkpoint below. Show me the result (trunk serve
  running, screenshot, counter value, whatever is relevant) and wait for
  my go-ahead before starting the next one.
- If you hit a blocker you can't resolve in ~5 minutes of digging, stop and
  ask instead of flailing. Common blockers: trunk version issues, eframe
  wasm-bindgen version mismatch, GitHub Pages permissions.
- Never refactor existing working code to "clean it up" unless I ask. Ship
  first, refactor never (this weekend).
- If you think a plan step is wrong, say so before doing it. Don't silently
  deviate.

Current state of the scaffold:
- Cargo.toml, Trunk.toml, index.html, .github/workflows/deploy.yml written
- src/main.rs + src/app.rs wired with 4 tabs
- src/demos/convex_hull.rs has a working click-to-add + Andrew's monotone chain
  with 2 unit tests passing
- Tabs 2-4 are "TODO" stubs
- Fonts NOT installed yet (TTFs should be in assets/ from last night's prep -
  verify this first)
- Tokyo Night palette NOT applied yet
- Nothing pushed to GitHub

Checkpoint sequence (order matters, pace is mine):

CHECKPOINT 1 - Polish foundation
Quality bar: the app visibly does not look like a default egui demo. A
reviewer's first-impression gut check says "this person cares."
- Verify Inter-Medium.otf and JetBrainsMono-Regular.ttf exist in assets/
- Install fonts via egui::FontDefinitions in App::new
- Apply Tokyo Night palette via egui::Visuals (hex codes in CRAFT.md)
- Set global item_spacing and panel layout per CRAFT.md
- Run `trunk serve`, take a screenshot, show me before moving on.

CHECKPOINT 2 - Deploy pipeline
Quality bar: pushes to main auto-deploy to GitHub Pages within ~4 min, no
manual steps. Must work on mobile (iOS Safari) not just desktop.
- git init, first local commit (ask me before pushing)
- After my OK: push, enable GitHub Pages (gh-pages branch, root)
- Verify workflow runs green, verify huecodes.github.io/archimedes/ loads
  on my laptop AND on my phone. Screenshots of both.

CHECKPOINT 3 - Shared point editor
Quality bar: feels good to use. Cursor state changes on hover. No jitter
on drag. Delete is obvious. This is the interaction substrate for every
other tab, so get it right before moving on.
- Implement in src/ui/point_editor.rs per plan doc section 3 file layout
- Click-add with 8px hit-test radius (don't add a point if cursor is within
  8px of an existing point)
- Drag-move with CursorIcon::Grab / Grabbing
- Right-click delete
- Shift-click multi-select (skip if it takes more than 30 min)
- Used by convex hull tab. Show me a trunk serve demo before moving on.

CHECKPOINT 4 - Convex hull polish
Quality bar: animation is smooth (no jank at 60fps with 1000 points).
Complexity counters are legible. Empty state exists. A reviewer could
screenshot this tab alone and it would be portfolio-worthy.
- Move current convex_hull.rs logic onto the shared point editor
- Add "add 100 random" button with a seeded RNG (seed visible in URL)
- Animated step-through at 120ms/step with play/pause button
- Right-side algorithm sidebar with complexity badge + live counters
  (orientation tests, hull vertex count, last-frame ms)
- Empty-state hint when no points yet (see CRAFT.md)
- Show me the final demo, ask me before committing.

CHECKPOINT 5 - Delaunay + Voronoi
Quality bar: drag a site, everything updates in real time at 60fps with
200+ sites. Cell colors are pleasant (not egui defaults). Dual view works.
- New tab using spade::DelaunayTriangulation
- Draggable sites, colored Voronoi cells (palette in CRAFT.md), Delaunay
  edges overlay, toggle buttons for each layer
- Circumcircle on hover (skip only if genuinely stuck)
- Sutherland-Hodgman for clipping Voronoi to viewport rect
- Tag v0.1 locally, ask before pushing.

CHECKPOINT 6 - Polygon boolean ops
Quality bar: two editable polygons, five ops selectable, result updates
live. Vertex handles are as polished as the point editor from CHECKPOINT 3.
- Via i_overlay crate
- Preset shapes button (pentagon, star, L-shape)
- Show vertex count + signed area of result

CHECKPOINT 7 - Robustness demo
Quality bar: the toggle makes a visible, convincing difference on a
degenerate point set. Reviewer sees the naive version fail and the robust
version hold, and the UI text explains why it matters.
- Use src/geometry/primitives.rs orient2d_naive vs orient2d_robust
- Preload a degenerate configuration (4 nearly-cocircular points)
- Apply the toggle to Delaunay triangulation, show the difference live
- Sidebar text per plan doc section 4 Tab 4

CHECKPOINT 8 - Polish pass
Quality bar: all tabs consistent, no rough edges, SVG export works,
status bar correct, keyboard shortcuts behave. Ship-ready.
- Items from CRAFT.md priority list 1 and 2 (typography, palette, empty
  states, counters - most already done). Priority 3 if time.
- SVG export per tab
- Status bar at bottom with live metrics
- Keyboard shortcuts (C clear, R random, 1-4 tab switch, Space play/pause)

Order is sequential. Don't skip ahead. If I say "ship what you have" mid-
sequence, stop at the current checkpoint and we cut the rest.

Start with verifying the fonts in assets/ exist. Don't install anything
or run any crate-install commands without confirming with me - I may have
already done them last night.

=====END=====
