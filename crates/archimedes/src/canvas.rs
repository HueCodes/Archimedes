use eframe::egui::{self, Align2, Color32, FontId, Pos2, Rect, Stroke};

use crate::theme;

pub const GRID_SPACING: f32 = 40.0;

pub fn paint_grid(painter: &egui::Painter, rect: Rect) {
    let grid = theme::FG_DIM.linear_multiply(0.08);
    let stroke = Stroke::new(1.0, grid);

    let x0 = (rect.min.x / GRID_SPACING).ceil() * GRID_SPACING;
    let mut x = x0;
    while x < rect.max.x {
        painter.line_segment(
            [Pos2::new(x, rect.min.y), Pos2::new(x, rect.max.y)],
            stroke,
        );
        x += GRID_SPACING;
    }

    let y0 = (rect.min.y / GRID_SPACING).ceil() * GRID_SPACING;
    let mut y = y0;
    while y < rect.max.y {
        painter.line_segment(
            [Pos2::new(rect.min.x, y), Pos2::new(rect.max.x, y)],
            stroke,
        );
        y += GRID_SPACING;
    }
}

pub fn paint_empty_state(painter: &egui::Painter, rect: Rect, primary: &str, hint: &str) {
    let center = rect.center();
    painter.text(
        center - egui::vec2(0.0, 10.0),
        Align2::CENTER_CENTER,
        primary,
        FontId::proportional(16.0),
        theme::FG_DIM,
    );
    painter.text(
        center + egui::vec2(0.0, 14.0),
        Align2::CENTER_CENTER,
        hint,
        FontId::monospace(12.0),
        theme::FG_DIM.linear_multiply(0.7),
    );
}

pub fn paint_point(painter: &egui::Painter, p: Pos2, color: Color32) {
    painter.circle_filled(p + egui::vec2(1.0, 1.0), 5.0, theme::BG.linear_multiply(0.6));
    painter.circle_filled(p, 5.0, color);
    painter.circle_filled(p, 2.0, theme::BG);
}

pub fn paint_hull(painter: &egui::Painter, hull: &[Pos2]) {
    if hull.len() < 2 {
        return;
    }
    let glow = Stroke::new(6.0, theme::ACCENT.linear_multiply(0.15));
    let stroke = Stroke::new(2.25, theme::ACCENT);
    let pts: Vec<Pos2> = hull.iter().copied().chain(std::iter::once(hull[0])).collect();
    for w in pts.windows(2) {
        painter.line_segment([w[0], w[1]], glow);
    }
    for w in pts.windows(2) {
        painter.line_segment([w[0], w[1]], stroke);
    }
}
