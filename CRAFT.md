# Craft spec — what makes this feel premium, not a student project

Default egui apps look like egui apps. The things below are what separate "slapped together in a weekend" from "this person cares." Each is small on its own; all of them together change the whole feel.

## Typography

Default egui fonts read amateur. Bundle two fonts:

- **Inter** (SIL Open Font License) for UI
- **JetBrains Mono** for numbers, complexity notation, code

```rust
fn install_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        "inter".into(),
        egui::FontData::from_static(include_bytes!("../assets/Inter-Medium.otf")).into(),
    );
    fonts.font_data.insert(
        "jbmono".into(),
        egui::FontData::from_static(include_bytes!("../assets/JetBrainsMono-Regular.ttf")).into(),
    );
    fonts.families.get_mut(&egui::FontFamily::Proportional).unwrap().insert(0, "inter".into());
    fonts.families.get_mut(&egui::FontFamily::Monospace).unwrap().insert(0, "jbmono".into());
    ctx.set_fonts(fonts);
}
```

Call from `App::new` via `cc.egui_ctx`. Download fonts into `assets/` before Saturday.

Size scale: 12 / 14 / 16 / 20 / 24. Nothing else.

## Palette (Tokyo Night, committed)

```rust
pub const BG: Color32        = Color32::from_rgb(0x1a, 0x1b, 0x26);
pub const PANEL: Color32     = Color32::from_rgb(0x24, 0x28, 0x3b);
pub const FG: Color32        = Color32::from_rgb(0xc0, 0xca, 0xf5);
pub const FG_DIM: Color32    = Color32::from_rgb(0x56, 0x5f, 0x89);
pub const ACCENT: Color32    = Color32::from_rgb(0x7a, 0xa2, 0xf7);  // blue — hulls, edges
pub const WARN: Color32      = Color32::from_rgb(0xf7, 0x76, 0x8e);  // pink — highlights, errors
pub const OK: Color32        = Color32::from_rgb(0x9e, 0xce, 0x6a);  // green — robust mode, success
pub const VIOLET: Color32    = Color32::from_rgb(0xbb, 0x9a, 0xf7);
pub const ORANGE: Color32    = Color32::from_rgb(0xe0, 0xaf, 0x68);
```

Set egui's style once in `App::new`:

```rust
let mut visuals = egui::Visuals::dark();
visuals.panel_fill = PANEL;
visuals.window_fill = BG;
visuals.extreme_bg_color = BG;
visuals.widgets.noninteractive.bg_stroke.color = FG_DIM;
visuals.selection.bg_fill = ACCENT.linear_multiply(0.3);
ctx.set_visuals(visuals);
```

Voronoi cell palette rotates through `ACCENT / OK / VIOLET / ORANGE / WARN` with 0.25 alpha fill.

## Layout discipline

- **TopBottomPanel** (tabs, 44px): logo + tab selector + global actions (clear, random, reset view)
- **SidePanel::right** (320px, collapsible): algorithm explainer, complexity badges, live counters
- **TopBottomPanel** (bottom, 24px status bar): `n points · k on hull · 47 orient tests · 16.8 ms · seed:0x8F3A`
- **CentralPanel**: the canvas

Never let widgets touch the edges. `ui.spacing_mut().item_spacing = Vec2::new(10.0, 8.0)` once, globally.

## Drawing polish

- Line width ≥ 1.75 (thin lines look "web 1.0")
- Point rendering: outer ring `FG` radius 5, inner dot `BG` radius 2 — reads cleaner than a flat circle
- Subtle drop shadow on points: second circle at +1,+1 offset, alpha 0.25
- Hull stroke: `ACCENT` at width 2.25, with a 0.15-alpha glow underlay at width 6
- Voronoi cells: fill at 0.22 alpha, edges at 0.7 alpha, site dots at full alpha
- Background grid: 40px spacing, `FG_DIM` at 0.08 alpha. Scales the sense of precision.
- Hovered entity gets `WARN` stroke at 2.5px, everything else dims to 0.5 alpha momentarily

## Animation — must be smooth, not jerky

- Hull construction animation: 120ms per step, `ease_in_out` via `egui::lerp`
- Hover state transitions: 80ms
- Tab switch: egui handles this but add a fade via `ctx.animate_bool`
- Request repaint only while animating: `ctx.request_repaint_after(Duration::from_millis(16))` — don't burn CPU at idle

## Empty state

When the canvas is empty, draw a centered hint in `FG_DIM`:

> Click anywhere to add a point
> ⌘R random · ⌘K clear · ⌘Z undo

Disappears as soon as the first point is placed. This single detail makes the app feel alive.

## Interaction quality

- **Click-to-add** only when cursor is not on an existing point (hit-test within 8px)
- **Drag** with a `CursorIcon::Grab` / `Grabbing` cursor state
- **Right-click** → popup menu: Delete, Pin, Duplicate
- **Shift-click** → multi-select
- **Undo / Redo** — keep a `Vec<Snapshot>` with Cmd+Z / Cmd+Shift+Z bindings. Reviewers don't expect this; seeing it signals care.
- **Scroll** zooms the canvas around cursor; **Space+drag** pans
- **Keyboard shortcuts shown in tooltips** on hover — `?` opens a cheat sheet overlay

## Algorithm explainer (right sidebar)

Every tab has the same structure — consistency is the quality signal:

```
ALGORITHM
Andrew's monotone chain
O(n log n)

INVARIANT
After each step the stack holds a counterclockwise
lower hull of all points processed so far.

LIVE
orientation tests · 47
points on hull · 8
expected Ω(n log n) · 46
last frame · 1.2 ms

REFERENCES
Andrew (1979) · textbook
de Berg §1.1 · textbook
```

Monospace for the numbers; proportional for prose. Complexity notation (O(n log n), Ω) gets monospace.

## Reproducibility

The "random points" button takes a seed (visible in URL and status bar). Same seed = same configuration, always. Use `rand::rngs::StdRng::seed_from_u64(seed)`.

URL-encode state in the query string so links reproduce the exact scene:

```
huecodes.github.io/archimedes/?tab=hull&seed=0x8F3A&n=100
```

For the points mode (custom configurations), serialize to base64 + gzip. Serde + `flate2` + `base64`.

## SVG export

Each tab has a "Save SVG" button. Hand-roll the SVG (don't pull in a huge crate) — 80 lines. Output is:

- viewBox matching canvas aspect
- Clean group structure: `<g id="points">`, `<g id="hull">`, `<g id="voronoi-cells">`, etc.
- Comments at top citing the algorithm + timestamp

This is the Atomic-Semi-worthy touch: a reviewer who works with layout tools knows exactly how much care goes into producing a *clean* SVG.

## Performance targets — benchmark these

- **1000 random points**, convex hull recomputed each frame: 60 fps, < 5 ms per frame
- **500 sites**, live Voronoi drag: 60 fps
- **Polygon booleans** with two 50-vertex inputs: < 2 ms per op
- Status bar shows last-frame time always, so the reviewer sees you measured

## Empty polish (easy, do them all)

- `favicon.svg` — a tiny convex hull icon in `ACCENT`
- Meta tags in `index.html`: og:title, og:description, og:image (screenshot), twitter:card
- Loading spinner while wasm bundle downloads (~5 MB) — CSS-only, placed in index.html
- Error boundary: if wasm panics, show "archimedes crashed — reload" with a reload button, not a blank screen

## Reference aesthetics

Look at these before Saturday, even briefly:

- **redblobgames.com** — interactive math explanations, the gold standard
- **ciechanow.ski** — Bartosz Ciechanowski's essays, peerless
- **desmos.com** — point drag feel, constraint handling
- **observablehq.com** — inline code + viz
- **Figma UI** — discipline in spacing, tooltips, microcopy

You don't need to match them. Borrowing 10% of their discipline puts you ahead of 95% of portfolio projects.

## Things that look bad — don't do them

- Raw egui default fonts
- Full-saturation colors (#ff0000 etc.)
- Lines under 1.5px wide
- Tight widget packing (zero whitespace)
- Missing empty states
- Unlabeled numbers in the UI ("47" with no context)
- Animating everything — only animate meaningful state changes
- Long text in the canvas area
- Using both emoji and text icons — pick one, probably neither

## Priority if time runs out

Rank your craft investment by payoff:

1. **Palette + typography + panel layout** — 70% of perceived quality, 1-2 hours of work. Do first.
2. **Empty state + live counters + status bar** — 15% of perceived quality, 1 hour. Do next.
3. **Animation + hover states** — 10%, 2 hours. Do if Saturday night has time.
4. **SVG export + URL state + undo/redo** — 5% each, skip if pressed.
