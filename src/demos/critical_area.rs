use eframe::egui::{self, Color32, CursorIcon, Pos2, Rect, Sense, Stroke};
use i_overlay::core::fill_rule::FillRule;
use i_overlay::core::overlay_rule::OverlayRule;
use i_overlay::float::single::SingleFloatOverlay;

use crate::canvas;
use crate::demos::polygon_ops::{paint_polygon, point_in_polygon, signed_area};
use crate::theme;

const HIT_RADIUS: f32 = 8.0;
const ARC_SEGMENTS: usize = 16;

pub struct CriticalAreaDemo {
    wire_a: Vec<Pos2>,
    wire_b: Vec<Pos2>,
    radius: f32,
    drag: Option<Drag>,
    last_rect: Option<Rect>,
    cache: Option<Cache>,
    vers_a: u64,
    vers_b: u64,
}

#[derive(Clone, Copy)]
enum Drag {
    Vertex(Side, usize),
    Body(Side),
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Side {
    A,
    B,
}

struct Cache {
    radius: f32,
    vers_a: u64,
    vers_b: u64,
    dilated_a: Vec<Pos2>,
    dilated_b: Vec<Pos2>,
    critical: Vec<Vec<Pos2>>,
    area: f32,
    ms: f32,
}

impl Default for CriticalAreaDemo {
    fn default() -> Self {
        Self {
            wire_a: wire(Pos2::new(420.0, 340.0), 240.0, 36.0),
            wire_b: wire(Pos2::new(420.0, 430.0), 240.0, 36.0),
            radius: 22.0,
            drag: None,
            last_rect: None,
            cache: None,
            vers_a: 1,
            vers_b: 1,
        }
    }
}

impl CriticalAreaDemo {
    pub fn radius_mut(&mut self) -> &mut f32 {
        &mut self.radius
    }

    pub fn metrics(&self) -> (f32, f32, f32) {
        let (area, ms) = self
            .cache
            .as_ref()
            .map(|c| (c.area, c.ms))
            .unwrap_or((0.0, 0.0));
        (self.radius, area, ms)
    }

    pub fn reset(&mut self) {
        *self = Self::default();
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
                self.drag = self
                    .nearest_vertex(pos)
                    .map(|(s, i)| Drag::Vertex(s, i))
                    .or_else(|| self.polygon_under(pos).map(Drag::Body));
            }
        }
        if response.dragged() {
            if let (Some(target), Some(pos)) = (self.drag, hover) {
                match target {
                    Drag::Vertex(side, idx) => {
                        let poly = self.polygon_mut(side);
                        if idx < poly.len() {
                            poly[idx] = pos;
                            self.bump(side);
                        }
                    }
                    Drag::Body(side) => {
                        let delta = response.drag_delta();
                        for p in self.polygon_mut(side).iter_mut() {
                            *p += delta;
                        }
                        self.bump(side);
                    }
                }
            }
        }
        if response.drag_stopped() {
            self.drag = None;
        }

        if self.drag.is_some() {
            ui.ctx().set_cursor_icon(CursorIcon::Grabbing);
        } else if let Some(pos) = hover {
            if self.nearest_vertex(pos).is_some() || self.polygon_under(pos).is_some() {
                ui.ctx().set_cursor_icon(CursorIcon::Grab);
            }
        }

        self.recompute_if_dirty();

        paint_polygon(
            &painter,
            &self.wire_a,
            theme::ACCENT.linear_multiply(0.35),
            Stroke::new(1.5, theme::ACCENT),
        );
        paint_polygon(
            &painter,
            &self.wire_b,
            theme::ORANGE.linear_multiply(0.35),
            Stroke::new(1.5, theme::ORANGE),
        );

        if let Some(c) = &self.cache {
            let outline = Stroke::new(1.0, theme::FG_DIM.linear_multiply(0.9));
            paint_polygon_outline(&painter, &c.dilated_a, outline);
            paint_polygon_outline(&painter, &c.dilated_b, outline);

            let fill = theme::WARN.linear_multiply(0.55);
            let stroke = Stroke::new(1.75, theme::WARN);
            for contour in &c.critical {
                paint_polygon(&painter, contour, fill, stroke);
            }
        }

        paint_vertex_handles(&painter, &self.wire_a, Side::A, hover, self.drag);
        paint_vertex_handles(&painter, &self.wire_b, Side::B, hover, self.drag);
    }

    fn polygon_mut(&mut self, side: Side) -> &mut Vec<Pos2> {
        match side {
            Side::A => &mut self.wire_a,
            Side::B => &mut self.wire_b,
        }
    }

    fn bump(&mut self, side: Side) {
        match side {
            Side::A => self.vers_a = self.vers_a.wrapping_add(1),
            Side::B => self.vers_b = self.vers_b.wrapping_add(1),
        }
    }

    fn polygon_under(&self, pos: Pos2) -> Option<Side> {
        if point_in_polygon(pos, &self.wire_a) {
            Some(Side::A)
        } else if point_in_polygon(pos, &self.wire_b) {
            Some(Side::B)
        } else {
            None
        }
    }

    fn nearest_vertex(&self, pos: Pos2) -> Option<(Side, usize)> {
        let r2 = HIT_RADIUS * HIT_RADIUS;
        let mut best: Option<(Side, usize, f32)> = None;
        for (i, &p) in self.wire_a.iter().enumerate() {
            let d2 = (p - pos).length_sq();
            if d2 <= r2 && best.map_or(true, |(_, _, bd)| d2 < bd) {
                best = Some((Side::A, i, d2));
            }
        }
        for (i, &p) in self.wire_b.iter().enumerate() {
            let d2 = (p - pos).length_sq();
            if d2 <= r2 && best.map_or(true, |(_, _, bd)| d2 < bd) {
                best = Some((Side::B, i, d2));
            }
        }
        best.map(|(s, i, _)| (s, i))
    }

    fn recompute_if_dirty(&mut self) {
        let dirty = self
            .cache
            .as_ref()
            .map(|c| {
                (c.radius - self.radius).abs() > f32::EPSILON
                    || c.vers_a != self.vers_a
                    || c.vers_b != self.vers_b
            })
            .unwrap_or(true);
        if !dirty {
            return;
        }
        if self.wire_a.len() < 3 || self.wire_b.len() < 3 {
            self.cache = None;
            return;
        }
        let half_r = (self.radius * 0.5).max(0.0);
        let t0 = web_time::Instant::now();
        let dilated_a = dilate(&self.wire_a, half_r, ARC_SEGMENTS);
        let dilated_b = dilate(&self.wire_b, half_r, ARC_SEGMENTS);

        let subj: Vec<[f32; 2]> = dilated_a.iter().map(|p| [p.x, p.y]).collect();
        let clip: Vec<[f32; 2]> = dilated_b.iter().map(|p| [p.x, p.y]).collect();
        let raw = subj.overlay(&clip, OverlayRule::Intersect, FillRule::EvenOdd);
        let ms = t0.elapsed().as_secs_f32() * 1000.0;

        let mut critical: Vec<Vec<Pos2>> = Vec::new();
        let mut area = 0.0f32;
        for shape in raw {
            for (i, contour) in shape.into_iter().enumerate() {
                let pts: Vec<Pos2> = contour.into_iter().map(|p| Pos2::new(p[0], p[1])).collect();
                let s = signed_area(&pts).abs();
                if i == 0 {
                    area += s;
                } else {
                    area -= s;
                }
                critical.push(pts);
            }
        }

        self.cache = Some(Cache {
            radius: self.radius,
            vers_a: self.vers_a,
            vers_b: self.vers_b,
            dilated_a,
            dilated_b,
            critical,
            area,
            ms,
        });
    }
}

fn wire(center: Pos2, w: f32, h: f32) -> Vec<Pos2> {
    let hw = w * 0.5;
    let hh = h * 0.5;
    vec![
        center + egui::vec2(-hw, -hh),
        center + egui::vec2(hw, -hh),
        center + egui::vec2(hw, hh),
        center + egui::vec2(-hw, hh),
    ]
}

/// Outward round-join buffer of a simple polygon by `radius`. Returns a closed
/// polygon. For a convex CCW input this produces a single convex ring; for a
/// non-convex input the result may self-intersect (i_overlay resolves that
/// downstream via EvenOdd).
fn dilate(poly: &[Pos2], radius: f32, seg_per_corner: usize) -> Vec<Pos2> {
    if poly.len() < 3 {
        return poly.to_vec();
    }
    if radius <= 0.0 {
        return poly.to_vec();
    }
    // Normalize to CCW (signed_area > 0 under our orient convention).
    let mut pts: Vec<Pos2> = poly.to_vec();
    if signed_area(&pts) < 0.0 {
        pts.reverse();
    }
    let n = pts.len();
    let mut out: Vec<Pos2> = Vec::with_capacity(n * (seg_per_corner + 1));

    for i in 0..n {
        let prev = pts[(i + n - 1) % n];
        let curr = pts[i];
        let next = pts[(i + 1) % n];

        // Outward normals of edge (prev→curr) and edge (curr→next).
        let n_in = outward_normal(prev, curr);
        let n_out = outward_normal(curr, next);

        // Start angle = angle of n_in; end angle = angle of n_out.
        let a0 = n_in.y.atan2(n_in.x);
        let mut a1 = n_out.y.atan2(n_out.x);
        // Ensure we sweep in the CCW direction consistent with the polygon's
        // outward side (here: the math-CCW orientation of outward normal turning).
        let mut delta = a1 - a0;
        while delta < 0.0 {
            delta += std::f32::consts::TAU;
        }
        while delta > std::f32::consts::TAU {
            delta -= std::f32::consts::TAU;
        }
        // If delta > pi, this vertex is reflex (concave); in that case don't
        // draw an arc outward — collapse to a single offset point using the
        // averaged normal. This keeps convex corners rounded and reflex corners
        // sharp, which i_overlay resolves cleanly.
        if delta <= std::f32::consts::PI {
            let _ = &mut a1;
            let steps = seg_per_corner.max(1);
            for k in 0..=steps {
                let t = k as f32 / steps as f32;
                let ang = a0 + delta * t;
                out.push(Pos2::new(
                    curr.x + radius * ang.cos(),
                    curr.y + radius * ang.sin(),
                ));
            }
        } else {
            let avg = egui::vec2(n_in.x + n_out.x, n_in.y + n_out.y);
            let len = (avg.x * avg.x + avg.y * avg.y).sqrt().max(1e-6);
            let n = egui::vec2(avg.x / len, avg.y / len);
            out.push(curr + n * radius);
        }
    }

    out
}

/// Outward normal of directed edge a→b for a CCW (signed_area > 0) polygon.
/// With orient = (b.x-a.x)*(c.y-a.y) - (b.y-a.y)*(c.x-a.x), a CCW polygon has
/// its interior on the LEFT of a→b, so the outward (right) normal rotates the
/// direction 90° CW: (dy, -dx) / len.
fn outward_normal(a: Pos2, b: Pos2) -> egui::Vec2 {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    let len = (dx * dx + dy * dy).sqrt().max(1e-6);
    egui::vec2(dy / len, -dx / len)
}

fn paint_polygon_outline(painter: &egui::Painter, poly: &[Pos2], stroke: Stroke) {
    if poly.len() < 2 {
        return;
    }
    let pts: Vec<Pos2> = poly.iter().copied().chain(std::iter::once(poly[0])).collect();
    painter.add(egui::Shape::line(pts, stroke));
    let _ = Color32::TRANSPARENT;
}

fn paint_vertex_handles(
    painter: &egui::Painter,
    poly: &[Pos2],
    side: Side,
    hover: Option<Pos2>,
    drag: Option<Drag>,
) {
    let accent = match side {
        Side::A => theme::ACCENT,
        Side::B => theme::ORANGE,
    };
    let r2 = HIT_RADIUS * HIT_RADIUS;
    for (i, &p) in poly.iter().enumerate() {
        let hovered = hover
            .map(|h| (h - p).length_sq() <= r2)
            .unwrap_or(false);
        let dragged = matches!(drag, Some(Drag::Vertex(s, idx)) if s == side && idx == i);
        let color = if hovered || dragged { theme::WARN } else { accent };
        canvas::paint_point(painter, p, color);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dilate_square_grows_area() {
        let sq = vec![
            Pos2::new(0.0, 0.0),
            Pos2::new(10.0, 0.0),
            Pos2::new(10.0, 10.0),
            Pos2::new(0.0, 10.0),
        ];
        let out = dilate(&sq, 1.0, 8);
        // Vertex count approximately (segs_per_corner+1) * 4
        assert!(out.len() >= 4 * 9);
        // Bounding box expanded in both dims.
        let (minx, miny, maxx, maxy) =
            out.iter()
                .fold((f32::INFINITY, f32::INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY),
                      |(ax, ay, bx, by), p| (ax.min(p.x), ay.min(p.y), bx.max(p.x), by.max(p.y)));
        assert!(minx < 0.0 && miny < 0.0 && maxx > 10.0 && maxy > 10.0);
    }

    #[test]
    fn intersection_present_when_radius_covers_gap() {
        let mut demo = CriticalAreaDemo::default();
        demo.vers_a = demo.vers_a.wrapping_add(1);
        demo.vers_b = demo.vers_b.wrapping_add(1);
        demo.radius = 80.0; // well over the 54px gap between wires
        demo.recompute_if_dirty();
        assert!(demo.cache.as_ref().unwrap().area > 0.0);
    }

    #[test]
    fn no_intersection_at_zero_radius() {
        let mut demo = CriticalAreaDemo::default();
        demo.vers_a = demo.vers_a.wrapping_add(1);
        demo.vers_b = demo.vers_b.wrapping_add(1);
        demo.radius = 0.0;
        demo.recompute_if_dirty();
        assert!(demo.cache.as_ref().unwrap().area < 1.0);
    }
}
