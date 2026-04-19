use std::time::Duration;

use eframe::egui::{self, Align2, FontId, Pos2, Rect, Stroke};
use web_time::Instant;

use crate::canvas;
use crate::geometry::primitives::orient2d_naive;
use crate::theme;
use crate::ui::point_editor::{next_seed, seeded_points, PointEditor, HIT_RADIUS};

const INITIAL_SEED: u64 = 0x8F3A_2C71;
const DEFAULT_INTERVAL_MS: u64 = 120;

pub struct ConvexHullDemo {
    editor: PointEditor,
    seed: u64,
    orient_tests: usize,
    hull_len: usize,
    last_ms: f32,
    last_rect: Option<egui::Rect>,
    anim: Option<HullAnim>,
    show_duality: bool,
    /// Last computed hull, kept so the dual pane can color lines by hull
    /// membership without recomputing. Updated on every non-animated frame.
    last_hull: Vec<Pos2>,
}

impl Default for ConvexHullDemo {
    fn default() -> Self {
        Self {
            editor: PointEditor::default(),
            seed: INITIAL_SEED,
            orient_tests: 0,
            hull_len: 0,
            last_ms: 0.0,
            last_rect: None,
            anim: None,
            show_duality: false,
            last_hull: Vec::new(),
        }
    }
}

struct HullAnim {
    plan: HullPlan,
    step: usize,
    playing: bool,
    last_tick: Instant,
    interval: Duration,
    plan_version: u64,
}

#[derive(Clone, Copy)]
enum HullEv {
    Consider(usize),
    PopLower,
    PushLower(usize),
    PopUpper,
    PushUpper(usize),
}

struct HullPlan {
    sorted: Vec<Pos2>,
    events: Vec<HullEv>,
    hull: Vec<Pos2>,
    orient_tests: usize,
}

struct AnimFrame {
    lower: Vec<Pos2>,
    upper: Vec<Pos2>,
    active: Option<Pos2>,
    just_popped: Option<Pos2>,
}

impl ConvexHullDemo {
    pub fn metrics(&self) -> (usize, usize, usize, f32) {
        (
            self.editor.len(),
            self.hull_len,
            self.orient_tests,
            self.last_ms,
        )
    }

    pub fn seed(&self) -> u64 {
        self.seed
    }

    pub fn show_duality_mut(&mut self) -> &mut bool {
        &mut self.show_duality
    }

    pub fn anim_progress(&self) -> Option<(usize, usize, bool)> {
        self.anim
            .as_ref()
            .map(|a| (a.step, a.plan.events.len(), a.playing))
    }

    pub fn clear(&mut self) {
        self.editor.clear();
        self.hull_len = 0;
        self.orient_tests = 0;
        self.last_ms = 0.0;
        self.anim = None;
    }

    pub fn random_into_last_rect(&mut self, n: usize) {
        if let Some(r) = self.last_rect {
            self.editor.set(seeded_points(r, n, self.seed));
            self.seed = next_seed(self.seed);
            self.anim = None;
        }
    }

    pub fn toggle_play(&mut self) {
        if self.editor.len() < 3 {
            return;
        }
        let version = self.editor.version();
        let anim = self.anim.get_or_insert_with(|| HullAnim {
            plan: plan_monotone_chain(self.editor.points()),
            step: 0,
            playing: false,
            last_tick: Instant::now(),
            interval: Duration::from_millis(DEFAULT_INTERVAL_MS),
            plan_version: version,
        });
        if anim.plan_version != version {
            anim.plan = plan_monotone_chain(self.editor.points());
            anim.plan_version = version;
            anim.step = 0;
        }
        if anim.step >= anim.plan.events.len() {
            anim.step = 0;
        }
        anim.playing = !anim.playing;
        anim.last_tick = Instant::now();
    }

    pub fn reset_anim(&mut self) {
        if let Some(a) = self.anim.as_mut() {
            a.step = 0;
            a.playing = false;
        }
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) {
        if !self.show_duality {
            self.ui_primal(ui, None);
            return;
        }

        // Horizontal split: primal pane on top, dual pane below. Compute
        // focus_idx once from the global cursor so cross-highlight works in
        // both directions in a single render pass.
        let total = ui.available_size();
        let top_h = (total.y * 0.5).floor();
        let top_size = egui::vec2(total.x, top_h);
        let bot_size = egui::vec2(total.x, total.y - top_h);

        let max = ui.max_rect();
        let top_rect = Rect::from_min_size(max.min, top_size);
        let bot_rect = Rect::from_min_size(
            Pos2::new(max.min.x, max.min.y + top_h),
            bot_size,
        );

        let cursor = ui.input(|i| i.pointer.hover_pos());
        let focus_idx = match cursor {
            Some(c) if top_rect.contains(c) => {
                self.editor.nearest_within(c, HIT_RADIUS)
            }
            Some(c) if bot_rect.contains(c) => {
                nearest_dual_line(c, self.editor.points(), bot_rect)
            }
            _ => None,
        };

        ui.allocate_ui_with_layout(
            top_size,
            egui::Layout::top_down(egui::Align::Min),
            |ui| self.ui_primal(ui, focus_idx),
        );
        ui.allocate_ui_with_layout(
            bot_size,
            egui::Layout::top_down(egui::Align::Min),
            |ui| self.ui_dual(ui, focus_idx),
        );
    }

    fn ui_primal(&mut self, ui: &mut egui::Ui, override_focus: Option<usize>) {
        let frame = self.editor.run(ui);
        self.last_rect = Some(frame.rect);

        canvas::paint_grid(&frame.painter, frame.rect);

        if self.editor.is_empty() {
            canvas::paint_empty_state(
                &frame.painter,
                frame.rect,
                "Click anywhere to add a point",
                "click add · drag move · right-click delete · R random · C clear · Space play",
            );
            self.hull_len = 0;
            self.orient_tests = 0;
            self.last_ms = 0.0;
            self.anim = None;
            return;
        }

        let ctx = ui.ctx().clone();

        if let Some(anim) = self.anim.as_mut() {
            if anim.plan_version != self.editor.version() {
                anim.plan = plan_monotone_chain(self.editor.points());
                anim.plan_version = self.editor.version();
                if anim.step > anim.plan.events.len() {
                    anim.step = anim.plan.events.len();
                }
            }
            if anim.playing {
                let now = Instant::now();
                while now.duration_since(anim.last_tick) >= anim.interval
                    && anim.step < anim.plan.events.len()
                {
                    anim.step += 1;
                    anim.last_tick += anim.interval;
                }
                if anim.step >= anim.plan.events.len() {
                    anim.playing = false;
                } else {
                    ctx.request_repaint_after(anim.interval);
                }
            }

            let frame_state = replay(&anim.plan, anim.step);

            for &p in &anim.plan.sorted {
                canvas::paint_point(&frame.painter, p, theme::FG.linear_multiply(0.35));
            }

            paint_partial_polyline(&frame.painter, &frame_state.lower);
            paint_partial_polyline(&frame.painter, &frame_state.upper);

            if anim.step >= anim.plan.events.len() {
                canvas::paint_hull(&frame.painter, &anim.plan.hull);
                self.last_hull = anim.plan.hull.clone();
            }

            if let Some(p) = frame_state.active {
                paint_active_ring(&frame.painter, p);
                canvas::paint_point(&frame.painter, p, theme::WARN);
            }
            if let Some(p) = frame_state.just_popped {
                paint_popped_ring(&frame.painter, p);
            }

            self.hull_len = anim.plan.hull.len();
            self.orient_tests = anim.plan.orient_tests;
            self.last_ms = 0.0;
            return;
        }

        let t0 = Instant::now();
        let (hull, tests) = monotone_chain(self.editor.points());
        self.last_ms = t0.elapsed().as_secs_f32() * 1000.0;
        self.orient_tests = tests;
        self.hull_len = hull.len();
        self.last_hull = hull.clone();

        canvas::paint_hull(&frame.painter, &hull);
        let focus = override_focus.or_else(|| {
            frame
                .response
                .hover_pos()
                .and_then(|h| self.editor.nearest_within(h, HIT_RADIUS))
        });
        self.editor
            .paint_with_focus(&frame.painter, theme::FG, focus);
    }

    fn ui_dual(&self, ui: &mut egui::Ui, focus_idx: Option<usize>) {
        let size = ui.available_size();
        let (_response, painter) = ui.allocate_painter(size, egui::Sense::hover());
        let rect = painter.clip_rect();

        canvas::paint_grid(&painter, rect);

        let points = self.editor.points();
        if points.len() < 2 {
            canvas::paint_empty_state(
                &painter,
                rect,
                "Dual lines appear here",
                "each point (a, b) ↦ line  y = a·x + b",
            );
            return;
        }

        let viewport = DualViewport::fit(points);
        let to_screen = |x: f32, y: f32| viewport.to_screen(rect, x, y);

        // Faint center axis at dual-x = 0.
        let axis = Stroke::new(1.0, theme::FG_DIM.linear_multiply(0.2));
        painter.line_segment(
            [
                to_screen(0.0, viewport.y_max),
                to_screen(0.0, viewport.y_min),
            ],
            axis,
        );

        // Determine hull membership by set-equality against last_hull. Hull is
        // a small subset so the O(n·h) scan is negligible at our point counts.
        let on_hull: Vec<bool> = points
            .iter()
            .map(|p| self.last_hull.iter().any(|h| h == p))
            .collect();

        let dim_stroke = Stroke::new(1.0, theme::FG_DIM.linear_multiply(0.55));
        let hull_stroke = Stroke::new(1.75, theme::ACCENT);
        let hull_glow = Stroke::new(5.0, theme::ACCENT.linear_multiply(0.15));
        let focus_stroke = Stroke::new(2.25, theme::WARN);

        // Pass 1: non-hull lines, dim.
        for (i, &p) in points.iter().enumerate() {
            if on_hull[i] {
                continue;
            }
            let a = to_screen(-viewport.x_range, viewport.y_at(p, -viewport.x_range));
            let b = to_screen(viewport.x_range, viewport.y_at(p, viewport.x_range));
            painter.line_segment([a, b], dim_stroke);
        }

        // Pass 2: hull lines, ACCENT with glow. Their visible upper edge IS the
        // upper envelope — the envelope emerges from the overlap, so the viewer
        // sees "hull points → envelope lines" geometrically.
        for (i, &p) in points.iter().enumerate() {
            if !on_hull[i] {
                continue;
            }
            let a = to_screen(-viewport.x_range, viewport.y_at(p, -viewport.x_range));
            let b = to_screen(viewport.x_range, viewport.y_at(p, viewport.x_range));
            painter.line_segment([a, b], hull_glow);
            painter.line_segment([a, b], hull_stroke);
        }

        // Focus overlay (cross-highlight).
        if let Some(i) = focus_idx {
            if let Some(&p) = points.get(i) {
                let a = to_screen(-viewport.x_range, viewport.y_at(p, -viewport.x_range));
                let b = to_screen(viewport.x_range, viewport.y_at(p, viewport.x_range));
                painter.line_segment([a, b], focus_stroke);
            }
        }

        painter.text(
            rect.min + egui::vec2(12.0, 12.0),
            Align2::LEFT_TOP,
            "dual plane · (a, b) ↦ y = a·x + b · upper hull ↔ upper envelope",
            FontId::monospace(10.5),
            theme::FG_DIM,
        );
    }
}

/// Viewport for the dual pane. Maps dual-space coordinates to screen pixels.
/// `x_range` is the half-width — dual x runs from `-x_range` to `+x_range`.
/// y bounds are fit to all line endpoints so every dual line is visible.
struct DualViewport {
    x_range: f32,
    y_min: f32,
    y_max: f32,
}

impl DualViewport {
    fn fit(points: &[Pos2]) -> Self {
        let x_range = 1.0;
        let mut y_min = f32::INFINITY;
        let mut y_max = f32::NEG_INFINITY;
        for p in points {
            let y_l = -p.x + p.y; // y at x = -1
            let y_r = p.x + p.y; // y at x = +1
            y_min = y_min.min(y_l).min(y_r);
            y_max = y_max.max(y_l).max(y_r);
        }
        let pad = (y_max - y_min).max(1.0) * 0.06;
        Self {
            x_range,
            y_min: y_min - pad,
            y_max: y_max + pad,
        }
    }

    fn y_at(&self, p: Pos2, dual_x: f32) -> f32 {
        p.x * dual_x + p.y
    }

    fn to_screen(&self, rect: Rect, dual_x: f32, dual_y: f32) -> Pos2 {
        let sx = rect.min.x
            + (dual_x + self.x_range) / (2.0 * self.x_range) * rect.width();
        let sy = rect.min.y
            + (dual_y - self.y_min) / (self.y_max - self.y_min) * rect.height();
        Pos2::new(sx, sy)
    }

    /// Inverse of the screen-x mapping: given a screen-space x coordinate
    /// inside `rect`, return the corresponding dual-space x.
    fn screen_x_to_dual(&self, rect: Rect, screen_x: f32) -> f32 {
        (screen_x - rect.min.x) / rect.width() * 2.0 * self.x_range - self.x_range
    }

    /// Inverse of the screen-y mapping for a dual-y coordinate.
    fn dual_y_to_screen_y(&self, rect: Rect, dual_y: f32) -> f32 {
        rect.min.y + (dual_y - self.y_min) / (self.y_max - self.y_min) * rect.height()
    }
}

/// Find which dual line is under `cursor` (in the dual pane `rect`) within
/// HIT_RADIUS pixels. Returns the source point's index. Used for cross-
/// highlighting — hover a dual line, see the source point in primal glow.
fn nearest_dual_line(cursor: Pos2, points: &[Pos2], rect: Rect) -> Option<usize> {
    if points.len() < 2 {
        return None;
    }
    let viewport = DualViewport::fit(points);
    let dual_x = viewport.screen_x_to_dual(rect, cursor.x);
    let mut best: Option<(usize, f32)> = None;
    for (i, &p) in points.iter().enumerate() {
        let line_dual_y = viewport.y_at(p, dual_x);
        let line_screen_y = viewport.dual_y_to_screen_y(rect, line_dual_y);
        let dist = (line_screen_y - cursor.y).abs();
        if dist <= HIT_RADIUS && best.is_none_or(|(_, bd)| dist < bd) {
            best = Some((i, dist));
        }
    }
    best.map(|(i, _)| i)
}

fn paint_partial_polyline(painter: &egui::Painter, pts: &[Pos2]) {
    if pts.len() < 2 {
        return;
    }
    let stroke = Stroke::new(2.0, theme::ACCENT.linear_multiply(0.7));
    let glow = Stroke::new(5.0, theme::ACCENT.linear_multiply(0.12));
    for w in pts.windows(2) {
        painter.line_segment([w[0], w[1]], glow);
    }
    for w in pts.windows(2) {
        painter.line_segment([w[0], w[1]], stroke);
    }
}

fn paint_active_ring(painter: &egui::Painter, p: Pos2) {
    painter.circle_stroke(p, 10.0, Stroke::new(1.5, theme::WARN.linear_multiply(0.9)));
    painter.circle_stroke(p, 14.0, Stroke::new(1.0, theme::WARN.linear_multiply(0.35)));
}

fn paint_popped_ring(painter: &egui::Painter, p: Pos2) {
    painter.circle_stroke(p, 8.0, Stroke::new(1.5, theme::WARN));
    painter.text(
        p + egui::vec2(0.0, -16.0),
        Align2::CENTER_BOTTOM,
        "pop",
        FontId::monospace(10.0),
        theme::WARN,
    );
}

fn replay(plan: &HullPlan, up_to: usize) -> AnimFrame {
    let mut lower: Vec<Pos2> = Vec::new();
    let mut upper: Vec<Pos2> = Vec::new();
    let mut active: Option<Pos2> = None;
    let mut just_popped: Option<Pos2> = None;
    for ev in plan.events.iter().take(up_to) {
        match *ev {
            HullEv::Consider(i) => {
                active = Some(plan.sorted[i]);
                just_popped = None;
            }
            HullEv::PopLower => {
                just_popped = lower.pop();
            }
            HullEv::PushLower(i) => {
                lower.push(plan.sorted[i]);
                just_popped = None;
            }
            HullEv::PopUpper => {
                just_popped = upper.pop();
            }
            HullEv::PushUpper(i) => {
                upper.push(plan.sorted[i]);
                just_popped = None;
            }
        }
    }
    AnimFrame {
        lower,
        upper,
        active,
        just_popped,
    }
}

fn plan_monotone_chain(input: &[Pos2]) -> HullPlan {
    if input.len() < 3 {
        return HullPlan {
            sorted: input.to_vec(),
            events: Vec::new(),
            hull: input.to_vec(),
            orient_tests: 0,
        };
    }
    let mut sorted: Vec<Pos2> = input.to_vec();
    sorted.sort_by(|a, b| {
        a.x.partial_cmp(&b.x)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.y.partial_cmp(&b.y).unwrap_or(std::cmp::Ordering::Equal))
    });
    let mut events: Vec<HullEv> = Vec::new();
    let mut tests = 0usize;

    let mut lower_idx: Vec<usize> = Vec::new();
    for (i, &p) in sorted.iter().enumerate() {
        events.push(HullEv::Consider(i));
        while lower_idx.len() >= 2 {
            let a = sorted[lower_idx[lower_idx.len() - 2]];
            let b = sorted[lower_idx[lower_idx.len() - 1]];
            tests += 1;
            if orient2d_naive(a, b, p) <= 0.0 {
                lower_idx.pop();
                events.push(HullEv::PopLower);
            } else {
                break;
            }
        }
        lower_idx.push(i);
        events.push(HullEv::PushLower(i));
    }

    let mut upper_idx: Vec<usize> = Vec::new();
    for i in (0..sorted.len()).rev() {
        let p = sorted[i];
        events.push(HullEv::Consider(i));
        while upper_idx.len() >= 2 {
            let a = sorted[upper_idx[upper_idx.len() - 2]];
            let b = sorted[upper_idx[upper_idx.len() - 1]];
            tests += 1;
            if orient2d_naive(a, b, p) <= 0.0 {
                upper_idx.pop();
                events.push(HullEv::PopUpper);
            } else {
                break;
            }
        }
        upper_idx.push(i);
        events.push(HullEv::PushUpper(i));
    }

    let mut lower_pts: Vec<Pos2> = lower_idx.iter().map(|&i| sorted[i]).collect();
    let mut upper_pts: Vec<Pos2> = upper_idx.iter().map(|&i| sorted[i]).collect();
    lower_pts.pop();
    upper_pts.pop();
    lower_pts.extend(upper_pts);
    HullPlan {
        sorted,
        events,
        hull: lower_pts,
        orient_tests: tests,
    }
}

fn monotone_chain(input: &[Pos2]) -> (Vec<Pos2>, usize) {
    let plan = plan_monotone_chain(input);
    (plan.hull, plan.orient_tests)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dual_viewport_y_at_follows_line_equation() {
        let p = Pos2::new(3.0, 5.0);
        let vp = DualViewport::fit(&[p, Pos2::new(0.0, 0.0)]);
        assert!((vp.y_at(p, 0.0) - 5.0).abs() < 1e-4);
        assert!((vp.y_at(p, 1.0) - 8.0).abs() < 1e-4);
        assert!((vp.y_at(p, -1.0) - 2.0).abs() < 1e-4);
    }

    #[test]
    fn dual_viewport_fits_all_line_endpoints() {
        let points = vec![Pos2::new(1.0, 0.0), Pos2::new(-1.0, 0.0)];
        let vp = DualViewport::fit(&points);
        // Line (1, 0): y at ±1 is ±1. Line (-1, 0): y at ±1 is ∓1.
        // Range [-1, 1] must be enclosed by [y_min, y_max] (with pad).
        assert!(vp.y_min <= -1.0);
        assert!(vp.y_max >= 1.0);
    }

    #[test]
    fn dual_viewport_x_mapping_round_trips() {
        let rect = Rect::from_min_size(Pos2::new(100.0, 50.0), egui::vec2(400.0, 200.0));
        let vp = DualViewport::fit(&[Pos2::new(0.0, 0.0), Pos2::new(1.0, 1.0)]);
        for screen_x in [100.0_f32, 200.0, 300.0, 500.0] {
            let dual_x = vp.screen_x_to_dual(rect, screen_x);
            let back = vp.to_screen(rect, dual_x, 0.0).x;
            assert!((back - screen_x).abs() < 1e-3, "round-trip failed at {screen_x}");
        }
    }

    #[test]
    fn nearest_dual_line_picks_the_line_under_the_cursor() {
        // Two lines with very different slopes. Cursor sits on one of them.
        let points = vec![Pos2::new(0.0, 0.0), Pos2::new(100.0, 200.0)];
        let rect = Rect::from_min_size(Pos2::new(0.0, 0.0), egui::vec2(400.0, 300.0));
        let vp = DualViewport::fit(&points);
        // Sample cursor dead center horizontally (screen_x = 200 = dual_x = 0).
        // At dual x = 0: line 0 y = 0, line 1 y = 200.
        let line1_dual_y = vp.y_at(points[1], 0.0);
        let line1_screen_y = vp.dual_y_to_screen_y(rect, line1_dual_y);
        let cursor = Pos2::new(rect.min.x + rect.width() * 0.5, line1_screen_y);
        let hit = nearest_dual_line(cursor, &points, rect);
        assert_eq!(hit, Some(1));
    }

    #[test]
    fn triangle_hull_is_triangle() {
        let pts = vec![
            Pos2::new(0.0, 0.0),
            Pos2::new(10.0, 0.0),
            Pos2::new(5.0, 8.0),
        ];
        let (hull, _) = monotone_chain(&pts);
        assert_eq!(hull.len(), 3);
    }

    #[test]
    fn interior_point_is_excluded() {
        let pts = vec![
            Pos2::new(0.0, 0.0),
            Pos2::new(10.0, 0.0),
            Pos2::new(10.0, 10.0),
            Pos2::new(0.0, 10.0),
            Pos2::new(5.0, 5.0),
        ];
        let (hull, _) = monotone_chain(&pts);
        assert_eq!(hull.len(), 4);
    }

    #[test]
    fn plan_events_reconstruct_hull() {
        let pts = vec![
            Pos2::new(0.0, 0.0),
            Pos2::new(10.0, 0.0),
            Pos2::new(10.0, 10.0),
            Pos2::new(0.0, 10.0),
            Pos2::new(5.0, 5.0),
            Pos2::new(3.0, 2.0),
        ];
        let plan = plan_monotone_chain(&pts);
        let frame = replay(&plan, plan.events.len());
        let mut combined = frame.lower.clone();
        let mut upper = frame.upper.clone();
        combined.pop();
        upper.pop();
        combined.extend(upper);
        assert_eq!(combined, plan.hull);
    }
}
