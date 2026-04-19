use eframe::egui::{self, Color32, CursorIcon, Pos2, Rect, Sense};

use crate::canvas;
use crate::theme;

pub const HIT_RADIUS: f32 = 8.0;

/// Interactive point set: click to add, drag to move, right-click to delete.
/// Shared substrate for every demo tab.
#[derive(Default)]
pub struct PointEditor {
    points: Vec<Pos2>,
    drag_idx: Option<usize>,
    version: u64,
}

/// Per-frame handles produced by `run`. Painter/response are moved out so the
/// editor can be called mutably again during the same frame.
pub struct EditorFrame {
    pub painter: egui::Painter,
    pub response: egui::Response,
    pub rect: Rect,
}

impl PointEditor {
    pub fn points(&self) -> &[Pos2] {
        &self.points
    }

    pub fn len(&self) -> usize {
        self.points.len()
    }

    pub fn is_empty(&self) -> bool {
        self.points.is_empty()
    }

    pub fn clear(&mut self) {
        self.points.clear();
        self.drag_idx = None;
        self.version = self.version.wrapping_add(1);
    }

    pub fn set(&mut self, pts: Vec<Pos2>) {
        self.points = pts;
        self.drag_idx = None;
        self.version = self.version.wrapping_add(1);
    }

    /// Bumps on every mutation (add / drag / delete / clear / set). Callers use this
    /// to invalidate cached work that depends on the point set.
    pub fn version(&self) -> u64 {
        self.version
    }

    /// Allocate a canvas-filling painter, then consume pointer input for this frame:
    /// drag an existing point under the cursor, right-click delete the nearest point,
    /// or left-click to add a new one (unless within `HIT_RADIUS` of an existing point).
    pub fn run(&mut self, ui: &mut egui::Ui) -> EditorFrame {
        let size = ui.available_size();
        let (response, painter) = ui.allocate_painter(size, Sense::click_and_drag());
        let rect = painter.clip_rect();

        let hover = response.hover_pos();

        if response.secondary_clicked() {
            if let Some(pos) = hover {
                if let Some(idx) = self.nearest_within(pos, HIT_RADIUS) {
                    self.points.remove(idx);
                    self.drag_idx = None;
                    self.version = self.version.wrapping_add(1);
                }
            }
        }

        if response.drag_started() {
            if let Some(pos) = hover {
                self.drag_idx = self.nearest_within(pos, HIT_RADIUS);
            }
        }
        if response.dragged() {
            if let (Some(idx), Some(pos)) = (self.drag_idx, hover) {
                if idx < self.points.len() {
                    self.points[idx] = pos;
                    self.version = self.version.wrapping_add(1);
                }
            }
        }
        if response.drag_stopped() {
            self.drag_idx = None;
        }

        if response.clicked() {
            if let Some(pos) = hover {
                if self.nearest_within(pos, HIT_RADIUS).is_none() {
                    self.points.push(pos);
                    self.version = self.version.wrapping_add(1);
                }
            }
        }

        if self.drag_idx.is_some() {
            ui.ctx().set_cursor_icon(CursorIcon::Grabbing);
        } else if let Some(pos) = hover {
            if self.nearest_within(pos, HIT_RADIUS).is_some() {
                ui.ctx().set_cursor_icon(CursorIcon::Grab);
            }
        }

        EditorFrame {
            painter,
            response,
            rect,
        }
    }

    /// Paint every stored point. Any point under the cursor (or being dragged)
    /// gets the warning accent so drag affordance is obvious.
    pub fn paint(&self, painter: &egui::Painter, base: Color32, hover: Option<Pos2>) {
        let hover_idx = hover.and_then(|h| self.nearest_within(h, HIT_RADIUS));
        for (i, &p) in self.points.iter().enumerate() {
            let highlight = Some(i) == self.drag_idx || Some(i) == hover_idx;
            let color = if highlight { theme::WARN } else { base };
            canvas::paint_point(painter, p, color);
        }
    }

    fn nearest_within(&self, pos: Pos2, radius: f32) -> Option<usize> {
        let r2 = radius * radius;
        let mut best: Option<(usize, f32)> = None;
        for (i, &p) in self.points.iter().enumerate() {
            let d2 = (p - pos).length_sq();
            if d2 <= r2 && best.is_none_or(|(_, bd)| d2 < bd) {
                best = Some((i, d2));
            }
        }
        best.map(|(i, _)| i)
    }
}

/// Step an LCG seed so that repeated calls to `seeded_points` produce a new but
/// still-reproducible scene. Used by the per-demo Random buttons.
pub fn next_seed(seed: u64) -> u64 {
    seed.wrapping_mul(0x5851_F42D_4C95_7F2D)
        .wrapping_add(0x1405_7B7E_F767_814F)
}

/// Deterministic xorshift64 fill of `rect` (inset by 40px) with `n` points.
pub fn seeded_points(rect: Rect, n: usize, seed: u64) -> Vec<Pos2> {
    let mut s = if seed == 0 { 0x8F3A_2C71_u64 } else { seed };
    let pad = 40.0_f32;
    let w = (rect.width() - 2.0 * pad).max(1.0);
    let h = (rect.height() - 2.0 * pad).max(1.0);
    let mut out = Vec::with_capacity(n);
    for _ in 0..n {
        s ^= s << 13;
        s ^= s >> 7;
        s ^= s << 17;
        let rx = (s as f32 / u64::MAX as f32).abs();
        s ^= s << 13;
        s ^= s >> 7;
        s ^= s << 17;
        let ry = (s as f32 / u64::MAX as f32).abs();
        out.push(Pos2::new(
            rect.min.x + pad + rx * w,
            rect.min.y + pad + ry * h,
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reject_click_within_hit_radius() {
        let mut e = PointEditor::default();
        e.set(vec![Pos2::new(100.0, 100.0)]);
        assert!(e.nearest_within(Pos2::new(104.0, 103.0), HIT_RADIUS).is_some());
        assert!(e.nearest_within(Pos2::new(200.0, 200.0), HIT_RADIUS).is_none());
    }

    #[test]
    fn version_advances_on_mutation() {
        let mut e = PointEditor::default();
        let v0 = e.version();
        e.set(vec![Pos2::new(1.0, 2.0)]);
        assert_ne!(e.version(), v0);
        let v1 = e.version();
        e.clear();
        assert_ne!(e.version(), v1);
    }

    #[test]
    fn seeded_points_are_deterministic() {
        let r = Rect::from_min_size(Pos2::ZERO, egui::vec2(800.0, 600.0));
        let a = seeded_points(r, 50, 0xABCD);
        let b = seeded_points(r, 50, 0xABCD);
        assert_eq!(a, b);
    }
}
