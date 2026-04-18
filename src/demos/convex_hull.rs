use eframe::egui::{self, Pos2};
use web_time::Instant;

use crate::canvas;
use crate::geometry::primitives::orient2d_naive;
use crate::theme;
use crate::ui::point_editor::{seeded_points, PointEditor};

const RANDOM_SEED: u64 = 0x8F3A_2C71;

#[derive(Default)]
pub struct ConvexHullDemo {
    editor: PointEditor,
    orient_tests: usize,
    hull_len: usize,
    last_ms: f32,
    last_rect: Option<egui::Rect>,
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

    pub fn clear(&mut self) {
        self.editor.clear();
        self.hull_len = 0;
        self.orient_tests = 0;
        self.last_ms = 0.0;
    }

    pub fn random_into_last_rect(&mut self, n: usize) {
        if let Some(r) = self.last_rect {
            self.editor.set(seeded_points(r, n, RANDOM_SEED));
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
                "click add · drag move · right-click delete · R random · C clear",
            );
            self.hull_len = 0;
            self.orient_tests = 0;
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

/// Andrew's monotone chain convex hull.
/// Returns (hull_vertices_ccw, orientation_test_count).
fn monotone_chain(input: &[Pos2]) -> (Vec<Pos2>, usize) {
    if input.len() < 3 {
        return (input.to_vec(), 0);
    }

    let mut pts: Vec<Pos2> = input.to_vec();
    pts.sort_by(|a, b| {
        a.x.partial_cmp(&b.x)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.y.partial_cmp(&b.y).unwrap_or(std::cmp::Ordering::Equal))
    });

    let mut tests = 0usize;

    let mut lower: Vec<Pos2> = Vec::new();
    for &p in &pts {
        while lower.len() >= 2 {
            let a = lower[lower.len() - 2];
            let b = lower[lower.len() - 1];
            tests += 1;
            if orient2d_naive(a, b, p) <= 0.0 {
                lower.pop();
            } else {
                break;
            }
        }
        lower.push(p);
    }

    let mut upper: Vec<Pos2> = Vec::new();
    for &p in pts.iter().rev() {
        while upper.len() >= 2 {
            let a = upper[upper.len() - 2];
            let b = upper[upper.len() - 1];
            tests += 1;
            if orient2d_naive(a, b, p) <= 0.0 {
                upper.pop();
            } else {
                break;
            }
        }
        upper.push(p);
    }

    lower.pop();
    upper.pop();
    lower.extend(upper);
    (lower, tests)
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
}
