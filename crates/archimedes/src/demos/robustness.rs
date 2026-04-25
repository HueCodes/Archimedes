use eframe::egui::{self, CursorIcon, Pos2, Rect, Sense, Stroke};

use crate::canvas;
use crate::geometry::primitives::{orient2d_naive, orient2d_robust};
use crate::theme;

const HIT_RADIUS: f32 = 10.0;
const GRID_CELLS: usize = 96;

pub struct RobustnessDemo {
    a: Pos2,
    b: Pos2,
    c: Pos2,
    drag: Option<Which>,
    last_rect: Option<Rect>,
    disagreements_total: u64,
    show_diff_field: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Which {
    A,
    B,
    C,
}

pub struct Readout {
    pub naive: f32,
    pub robust: f64,
    pub sign_naive: i8,
    pub sign_robust: i8,
    pub agree: bool,
    /// Shewchuk's static error bound for the naive f32 computation at the
    /// current (a, b, c). If `|naive|` is below this bound, the sign is
    /// formally untrustworthy. Reported so the reader can see the error
    /// budget relative to the signal.
    pub shewchuk_bound: f32,
}

impl Default for RobustnessDemo {
    fn default() -> Self {
        Self {
            a: Pos2::new(320.0, 400.0),
            b: Pos2::new(760.0, 400.0005),
            c: Pos2::new(540.0, 400.00025),
            drag: None,
            last_rect: None,
            disagreements_total: 0,
            show_diff_field: true,
        }
    }
}

impl RobustnessDemo {
    pub fn show_diff_field_mut(&mut self) -> &mut bool {
        &mut self.show_diff_field
    }

    pub fn reset(&mut self) {
        let keep = self.show_diff_field;
        let rect = self.last_rect;
        *self = Self::default();
        self.show_diff_field = keep;
        self.last_rect = rect;
        self.preset_nearly_collinear();
    }

    /// Place A, B, C as a near-collinear triple centered in the current canvas.
    /// Falls back to fixed coordinates if the canvas hasn't been measured yet
    /// (the first frame's `ui()` calls this immediately after capturing the
    /// rect, so the fallback is rarely hit). Canvas-relative positioning means
    /// resizing the window or reloading the preset always lands the points
    /// inside the visible area, instead of stranding them in the old viewport.
    pub fn preset_nearly_collinear(&mut self) {
        let rect = self.last_rect.unwrap_or_else(|| {
            Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(900.0, 700.0))
        });
        let cx = rect.center().x;
        let cy = rect.center().y;
        let half_span = (rect.width() * 0.3).clamp(160.0, 360.0);
        // The y-offsets are deliberately sub-pixel — that's the entire point of
        // the demo. f32 can't resolve them, the line through A and B stays at
        // y = cy on screen, but orient2d's product loses bits and may flip.
        self.a = Pos2::new(cx - half_span, cy);
        self.b = Pos2::new(cx + half_span, cy + 0.0005);
        self.c = Pos2::new(cx, cy + 0.00025);
    }

    pub fn readout(&self) -> Readout {
        let naive = orient2d_naive(self.a, self.b, self.c);
        let robust = orient2d_robust(self.a, self.b, self.c);
        let sign_naive = sign(naive as f64);
        let sign_robust = sign(robust);
        let left = (self.b.x - self.a.x) * (self.c.y - self.a.y);
        let right = (self.b.y - self.a.y) * (self.c.x - self.a.x);
        let shewchuk_bound =
            (3.0 + 16.0 * f32::EPSILON) * f32::EPSILON * (left.abs() + right.abs());
        Readout {
            naive,
            robust,
            sign_naive,
            sign_robust,
            agree: sign_naive == sign_robust,
            shewchuk_bound,
        }
    }

    pub fn disagreements(&self) -> u64 {
        self.disagreements_total
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) {
        let size = ui.available_size();
        let (response, painter) = ui.allocate_painter(size, Sense::click_and_drag());
        let rect = painter.clip_rect();
        let first_frame = self.last_rect.is_none();
        self.last_rect = Some(rect);
        if first_frame {
            self.preset_nearly_collinear();
        }
        canvas::paint_grid(&painter, rect);

        let hover = response.hover_pos();

        if response.drag_started() {
            if let Some(pos) = hover {
                self.drag = self.nearest(pos);
            }
        }
        if response.dragged() {
            if let (Some(w), Some(pos)) = (self.drag, hover) {
                match w {
                    Which::A => self.a = pos,
                    Which::B => self.b = pos,
                    Which::C => self.c = pos,
                }
            }
        }
        if response.drag_stopped() {
            self.drag = None;
        }
        if self.drag.is_some() {
            ui.ctx().set_cursor_icon(CursorIcon::Grabbing);
        } else if let Some(pos) = hover {
            if self.nearest(pos).is_some() {
                ui.ctx().set_cursor_icon(CursorIcon::Grab);
            }
        }

        let field_alpha = ui
            .ctx()
            .animate_bool(egui::Id::new("rb_show_diff_field"), self.show_diff_field);
        if field_alpha > 0.01 {
            paint_disagreement_field(&painter, rect, self.a, self.b, field_alpha);
        }

        let line_color = theme::FG_DIM.linear_multiply(0.85);
        let ray = extend_line(rect, self.a, self.b);
        painter.line_segment(
            [ray.0, ray.1],
            Stroke::new(1.0, line_color),
        );

        let readout = self.readout();
        if !readout.agree {
            self.disagreements_total = self.disagreements_total.saturating_add(1);
        }

        let c_color = if readout.agree { theme::FG } else { theme::WARN };
        canvas::paint_point(&painter, self.a, theme::ACCENT);
        canvas::paint_point(&painter, self.b, theme::ACCENT);
        canvas::paint_point(&painter, self.c, c_color);

        painter.text(
            self.a + egui::vec2(0.0, -14.0),
            egui::Align2::CENTER_BOTTOM,
            "A",
            egui::FontId::monospace(12.0),
            theme::FG_DIM,
        );
        painter.text(
            self.b + egui::vec2(0.0, -14.0),
            egui::Align2::CENTER_BOTTOM,
            "B",
            egui::FontId::monospace(12.0),
            theme::FG_DIM,
        );
        painter.text(
            self.c + egui::vec2(0.0, -14.0),
            egui::Align2::CENTER_BOTTOM,
            "C",
            egui::FontId::monospace(12.0),
            c_color,
        );
    }

    fn nearest(&self, pos: Pos2) -> Option<Which> {
        let r2 = HIT_RADIUS * HIT_RADIUS;
        let mut best: Option<(Which, f32)> = None;
        for (w, p) in [(Which::A, self.a), (Which::B, self.b), (Which::C, self.c)] {
            let d2 = (p - pos).length_sq();
            if d2 <= r2 && best.is_none_or(|(_, bd)| d2 < bd) {
                best = Some((w, d2));
            }
        }
        best.map(|(w, _)| w)
    }
}

fn sign(x: f64) -> i8 {
    if x > 0.0 {
        1
    } else if x < 0.0 {
        -1
    } else {
        0
    }
}

fn extend_line(rect: Rect, a: Pos2, b: Pos2) -> (Pos2, Pos2) {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    let len = (dx * dx + dy * dy).sqrt().max(1e-6);
    let nx = dx / len;
    let ny = dy / len;
    let far = (rect.width() + rect.height()) * 2.0;
    let center = Pos2::new((a.x + b.x) * 0.5, (a.y + b.y) * 0.5);
    (
        Pos2::new(center.x - nx * far, center.y - ny * far),
        Pos2::new(center.x + nx * far, center.y + ny * far),
    )
}

/// Shade cells near the AB line where f32 has shed enough bits of precision
/// that the orient2d sign can't be trusted. We mark cells whose naive f32 value
/// falls inside Shewchuk's static error bound (scaled for visibility): that
/// band is a strict superset of the sign-flip strip and, unlike the flip strip,
/// is wide enough to hit grid cell centers at screen-space coordinates. The
/// true sign-flip region for typical screen coords is sub-pixel — previously
/// the visualization only "flashed" when A/B happened to land on lucky cells.
fn paint_disagreement_field(painter: &egui::Painter, rect: Rect, a: Pos2, b: Pos2, alpha: f32) {
    let cell_w = rect.width() / GRID_CELLS as f32;
    let cell_h = rect.height() / GRID_CELLS as f32;
    let zone = theme::WARN.linear_multiply(0.25 * alpha);
    for gy in 0..GRID_CELLS {
        for gx in 0..GRID_CELLS {
            let px = rect.min.x + (gx as f32 + 0.5) * cell_w;
            let py = rect.min.y + (gy as f32 + 0.5) * cell_h;
            let p = Pos2::new(px, py);
            if f32_precision_lost(a, b, p) {
                let cell_rect = Rect::from_min_size(
                    Pos2::new(rect.min.x + gx as f32 * cell_w, rect.min.y + gy as f32 * cell_h),
                    egui::vec2(cell_w, cell_h),
                );
                painter.rect_filled(cell_rect, 0.0, zone);
            }
        }
    }
    let legend = Rect::from_min_size(rect.min + egui::vec2(12.0, 12.0), egui::vec2(10.0, 10.0));
    painter.rect_filled(legend, 2.0, zone);
    painter.text(
        legend.right_center() + egui::vec2(6.0, 0.0),
        egui::Align2::LEFT_CENTER,
        "f32 sign untrustworthy",
        egui::FontId::monospace(11.0),
        theme::FG_DIM.linear_multiply(alpha),
    );
}

/// Inside-the-band test. Distance from `p` to the AB line, compared to a band
/// thickness that scales with the largest involved coordinate magnitude. Real
/// f32 precision loss for orient2d is `~ε · (coord_magnitude)²`, which maps to
/// a sub-pixel band at screen coords; `VIS_GAIN` scales that up into the visible
/// range while preserving the physics — push points far from the origin and the
/// untrustworthy zone widens, just as it does on a real wafer coordinate system.
fn f32_precision_lost(a: Pos2, b: Pos2, p: Pos2) -> bool {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    let ab_len = (dx * dx + dy * dy).sqrt();
    if ab_len < 1.0 {
        return false;
    }
    let dist = (dx * (p.y - a.y) - dy * (p.x - a.x)).abs() / ab_len;
    const EPS: f32 = f32::EPSILON;
    const VIS_GAIN: f32 = 1.0e5;
    let mag = a.x.abs().max(a.y.abs()).max(p.x.abs()).max(p.y.abs());
    let band = VIS_GAIN * EPS * mag;
    dist < band
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collinear_preset_has_small_orient_values() {
        let d = RobustnessDemo::default();
        let r = d.readout();
        assert!(r.naive.abs() < 1.0);
        assert!(r.robust.abs() < 1.0);
    }

    #[test]
    fn precision_band_hugs_ab_line_at_default_preset() {
        let a = Pos2::new(320.0, 400.0);
        let b = Pos2::new(760.0, 400.0005);
        assert!(f32_precision_lost(a, b, Pos2::new(540.0, 400.0)));
        assert!(f32_precision_lost(a, b, Pos2::new(540.0, 402.0)));
        assert!(!f32_precision_lost(a, b, Pos2::new(540.0, 500.0)));
        assert!(!f32_precision_lost(a, b, Pos2::new(540.0, 300.0)));
    }

    #[test]
    fn clearly_noncollinear_points_agree() {
        let d = RobustnessDemo {
            a: Pos2::new(0.0, 0.0),
            b: Pos2::new(100.0, 0.0),
            c: Pos2::new(50.0, 50.0),
            drag: None,
            last_rect: None,
            disagreements_total: 0,
            show_diff_field: true,
        };
        let r = d.readout();
        assert!(r.agree);
    }
}
