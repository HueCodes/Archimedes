use eframe::egui::{self, Color32, CursorIcon, Pos2, Rect, Sense, Stroke};

use crate::canvas;
use crate::geometry::primitives::{orient2d_naive, orient2d_robust};
use crate::theme;

const HIT_RADIUS: f32 = 10.0;
const GRID_CELLS: usize = 44;

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
        *self = Self::default();
        self.show_diff_field = keep;
    }

    pub fn preset_nearly_collinear(&mut self) {
        self.a = Pos2::new(320.0, 400.0);
        self.b = Pos2::new(760.0, 400.0005);
        self.c = Pos2::new(540.0, 400.00025);
    }

    pub fn readout(&self) -> Readout {
        let naive = orient2d_naive(self.a, self.b, self.c);
        let robust = orient2d_robust(self.a, self.b, self.c);
        let sign_naive = sign(naive as f64);
        let sign_robust = sign(robust);
        Readout {
            naive,
            robust,
            sign_naive,
            sign_robust,
            agree: sign_naive == sign_robust,
        }
    }

    pub fn disagreements(&self) -> u64 {
        self.disagreements_total
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) {
        let size = ui.available_size();
        let (response, painter) = ui.allocate_painter(size, Sense::click_and_drag());
        let rect = painter.clip_rect();
        self.last_rect = Some(rect);
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

        if self.show_diff_field {
            paint_disagreement_field(&painter, rect, self.a, self.b);
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
            if d2 <= r2 && best.map_or(true, |(_, bd)| d2 < bd) {
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

/// Visit a coarse grid of sample points across the viewport. At each cell,
/// compare orient2d_naive (f32) against orient2d_robust (f64): where the signs
/// disagree, shade the cell in WARN; otherwise leave it transparent. The
/// unstable strip hugs the AB line and flanks both sides within ~1 cell of
/// cancellation. This is the whole point of the tab.
fn paint_disagreement_field(painter: &egui::Painter, rect: Rect, a: Pos2, b: Pos2) {
    let cell_w = rect.width() / GRID_CELLS as f32;
    let cell_h = rect.height() / GRID_CELLS as f32;
    let disagree = theme::WARN.linear_multiply(0.28);
    for gy in 0..GRID_CELLS {
        for gx in 0..GRID_CELLS {
            let px = rect.min.x + (gx as f32 + 0.5) * cell_w;
            let py = rect.min.y + (gy as f32 + 0.5) * cell_h;
            let p = Pos2::new(px, py);
            let n = orient2d_naive(a, b, p);
            let r = orient2d_robust(a, b, p);
            if sign(n as f64) != sign(r) {
                let cell_rect = Rect::from_min_size(
                    Pos2::new(rect.min.x + gx as f32 * cell_w, rect.min.y + gy as f32 * cell_h),
                    egui::vec2(cell_w, cell_h),
                );
                painter.rect_filled(cell_rect, 0.0, disagree);
            }
        }
    }
    // Legend strip in the corner.
    let legend = Rect::from_min_size(rect.min + egui::vec2(12.0, 12.0), egui::vec2(10.0, 10.0));
    painter.rect_filled(legend, 2.0, disagree);
    painter.text(
        legend.right_center() + egui::vec2(6.0, 0.0),
        egui::Align2::LEFT_CENTER,
        "naive vs robust disagree",
        egui::FontId::monospace(11.0),
        theme::FG_DIM,
    );
    let _ = Color32::TRANSPARENT;
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
