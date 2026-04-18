use std::time::Duration;

use eframe::egui::{self, Align2, FontId, Pos2, Stroke};
use web_time::Instant;

use crate::canvas;
use crate::geometry::primitives::orient2d_naive;
use crate::theme;
use crate::ui::point_editor::{seeded_points, PointEditor};

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
            // Step the seed so repeated Rs produce a new scene, still reproducible.
            self.seed = self
                .seed
                .wrapping_mul(0x5851_F42D_4C95_7F2D)
                .wrapping_add(0x14057B7E_F767_814F);
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

        canvas::paint_hull(&frame.painter, &hull);
        self.editor
            .paint(&frame.painter, theme::FG, frame.response.hover_pos());
    }
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
