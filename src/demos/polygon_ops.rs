use eframe::egui::{self, Color32, CursorIcon, Pos2, Rect, Sense, Stroke};
use i_overlay::core::fill_rule::FillRule;
use i_overlay::core::overlay_rule::OverlayRule;
use i_overlay::float::single::SingleFloatOverlay;

use crate::canvas;
use crate::theme;

const HIT_RADIUS: f32 = 8.0;

pub struct PolygonOpsDemo {
    a: Vec<Pos2>,
    b: Vec<Pos2>,
    op: OverlayRule,
    mode: EditMode,
    drag: Option<DragTarget>,
    last_rect: Option<Rect>,
    cache: Option<Cache>,
    vers_a: u64,
    vers_b: u64,
}

#[derive(Clone, Copy)]
enum DragTarget {
    Vertex(Side, usize),
    Body(Side),
}

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum EditMode {
    DragOnly,
    EditA,
    EditB,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Side {
    A,
    B,
}

struct Cache {
    op: OverlayRule,
    vers_a: u64,
    vers_b: u64,
    result: Vec<Vec<Vec<Pos2>>>,
    vertex_count: usize,
    area: f32,
    ms: f32,
    euler: EulerCounts,
}

/// Topology readout for the overlay result, treated as a planar subdivision.
/// For a set of disjoint simple polygons (outer contours) each of which may
/// contain holes, with vertices V, edges E, and faces F (including the single
/// unbounded face), Euler's formula for a planar graph with C connected
/// components gives `V − E + F = 1 + C`. We report all four and flag when
/// the invariant breaks.
#[derive(Default, Clone, Copy)]
pub struct EulerCounts {
    pub v: usize,
    pub e: usize,
    pub f: usize,
    pub components: usize,
}

impl EulerCounts {
    pub fn chi(self) -> i64 {
        self.v as i64 - self.e as i64 + self.f as i64
    }
    pub fn expected_chi(self) -> i64 {
        1 + self.components as i64
    }
}

impl Default for PolygonOpsDemo {
    fn default() -> Self {
        Self {
            a: pentagon(Pos2::new(360.0, 320.0), 130.0),
            b: rectangle(Pos2::new(500.0, 380.0), 220.0, 160.0),
            op: OverlayRule::Union,
            mode: EditMode::DragOnly,
            drag: None,
            last_rect: None,
            cache: None,
            vers_a: 1,
            vers_b: 1,
        }
    }
}

impl PolygonOpsDemo {
    pub fn mode_mut(&mut self) -> &mut EditMode {
        &mut self.mode
    }
    pub fn op_mut(&mut self) -> &mut OverlayRule {
        &mut self.op
    }

    pub fn metrics(&self) -> (usize, usize, usize, f32, f32) {
        let (vcount, area, ms) = match &self.cache {
            Some(c) => (c.vertex_count, c.area, c.ms),
            None => (0, 0.0, 0.0),
        };
        (self.a.len(), self.b.len(), vcount, area, ms)
    }

    pub fn euler(&self) -> EulerCounts {
        self.cache
            .as_ref()
            .map(|c| c.euler)
            .unwrap_or_default()
    }

    pub fn clear(&mut self) {
        self.a = pentagon(
            self.last_rect.map(|r| r.center()).unwrap_or(Pos2::new(360.0, 320.0)),
            130.0,
        );
        self.b = rectangle(
            self.last_rect
                .map(|r| r.center() + egui::vec2(140.0, 60.0))
                .unwrap_or(Pos2::new(500.0, 380.0)),
            220.0,
            160.0,
        );
        self.vers_a = self.vers_a.wrapping_add(1);
        self.vers_b = self.vers_b.wrapping_add(1);
        self.cache = None;
    }

    pub fn preset_a(&mut self, preset: Preset) {
        self.a = preset.build(self.last_rect_center() + egui::vec2(-70.0, 0.0));
        self.vers_a = self.vers_a.wrapping_add(1);
    }

    pub fn preset_b(&mut self, preset: Preset) {
        self.b = preset.build(self.last_rect_center() + egui::vec2(70.0, 0.0));
        self.vers_b = self.vers_b.wrapping_add(1);
    }

    fn last_rect_center(&self) -> Pos2 {
        self.last_rect
            .map(|r| r.center())
            .unwrap_or(Pos2::new(500.0, 380.0))
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) {
        let size = ui.available_size();
        let (response, painter) = ui.allocate_painter(size, Sense::click_and_drag());
        let rect = painter.clip_rect();
        self.last_rect = Some(rect);
        canvas::paint_grid(&painter, rect);

        let hover = response.hover_pos();

        if response.secondary_clicked() {
            if let Some(pos) = hover {
                if let Some((side, idx)) = self.nearest_vertex(pos) {
                    let poly = self.polygon_mut(side);
                    if poly.len() > 3 {
                        poly.remove(idx);
                        self.bump_version(side);
                    }
                }
            }
        }

        if response.drag_started() {
            if let Some(pos) = hover {
                self.drag = self
                    .nearest_vertex(pos)
                    .map(|(s, i)| DragTarget::Vertex(s, i))
                    .or_else(|| self.polygon_under(pos).map(DragTarget::Body));
            }
        }
        if response.dragged() {
            if let (Some(target), Some(pos)) = (self.drag, hover) {
                match target {
                    DragTarget::Vertex(side, idx) => {
                        let poly = self.polygon_mut(side);
                        if idx < poly.len() {
                            poly[idx] = pos;
                            self.bump_version(side);
                        }
                    }
                    DragTarget::Body(side) => {
                        let delta = response.drag_delta();
                        for p in self.polygon_mut(side).iter_mut() {
                            *p += delta;
                        }
                        self.bump_version(side);
                    }
                }
            }
        }
        if response.drag_stopped() {
            self.drag = None;
        }

        if response.clicked() {
            if let Some(pos) = hover {
                if self.nearest_vertex(pos).is_none() {
                    match self.mode {
                        EditMode::EditA => {
                            self.a.push(pos);
                            self.vers_a = self.vers_a.wrapping_add(1);
                        }
                        EditMode::EditB => {
                            self.b.push(pos);
                            self.vers_b = self.vers_b.wrapping_add(1);
                        }
                        EditMode::DragOnly => {}
                    }
                }
            }
        }

        if self.drag.is_some() {
            ui.ctx().set_cursor_icon(CursorIcon::Grabbing);
        } else if let Some(pos) = hover {
            if self.nearest_vertex(pos).is_some() || self.polygon_under(pos).is_some() {
                ui.ctx().set_cursor_icon(CursorIcon::Grab);
            }
        }

        self.recompute_if_dirty();

        paint_polygon(&painter, &self.a, fill_a(), stroke_a());
        paint_polygon(&painter, &self.b, fill_b(), stroke_b());
        if let Some(c) = &self.cache {
            let outer_fill = theme::OK.linear_multiply(0.9);
            let outer_stroke = Stroke::new(2.0, Color32::from_rgb(0x6f, 0x94, 0x4c));
            let hole_fill = theme::BG;
            let hole_stroke = Stroke::new(1.25, outer_stroke.color);
            for shape in &c.result {
                for (i, contour) in shape.iter().enumerate() {
                    let (fill, stroke) = if i == 0 {
                        (outer_fill, outer_stroke)
                    } else {
                        (hole_fill, hole_stroke)
                    };
                    paint_polygon(&painter, contour, fill, stroke);
                }
            }
        }
        paint_vertex_handles(&painter, &self.a, Side::A, hover, self.drag);
        paint_vertex_handles(&painter, &self.b, Side::B, hover, self.drag);
    }

    fn polygon_mut(&mut self, side: Side) -> &mut Vec<Pos2> {
        match side {
            Side::A => &mut self.a,
            Side::B => &mut self.b,
        }
    }

    fn bump_version(&mut self, side: Side) {
        match side {
            Side::A => self.vers_a = self.vers_a.wrapping_add(1),
            Side::B => self.vers_b = self.vers_b.wrapping_add(1),
        }
    }

    fn polygon_under(&self, pos: Pos2) -> Option<Side> {
        if point_in_polygon(pos, &self.a) {
            Some(Side::A)
        } else if point_in_polygon(pos, &self.b) {
            Some(Side::B)
        } else {
            None
        }
    }

    fn nearest_vertex(&self, pos: Pos2) -> Option<(Side, usize)> {
        let r2 = HIT_RADIUS * HIT_RADIUS;
        let mut best: Option<(Side, usize, f32)> = None;
        for (i, &p) in self.a.iter().enumerate() {
            let d2 = (p - pos).length_sq();
            if d2 <= r2 && best.is_none_or(|(_, _, bd)| d2 < bd) {
                best = Some((Side::A, i, d2));
            }
        }
        for (i, &p) in self.b.iter().enumerate() {
            let d2 = (p - pos).length_sq();
            if d2 <= r2 && best.is_none_or(|(_, _, bd)| d2 < bd) {
                best = Some((Side::B, i, d2));
            }
        }
        best.map(|(s, i, _)| (s, i))
    }

    fn recompute_if_dirty(&mut self) {
        let dirty = self
            .cache
            .as_ref()
            .map(|c| c.op != self.op || c.vers_a != self.vers_a || c.vers_b != self.vers_b)
            .unwrap_or(true);
        if !dirty {
            return;
        }
        if self.a.len() < 3 || self.b.len() < 3 {
            self.cache = None;
            return;
        }
        let t0 = web_time::Instant::now();
        let subj: Vec<[f32; 2]> = self.a.iter().map(|p| [p.x, p.y]).collect();
        let clip: Vec<[f32; 2]> = self.b.iter().map(|p| [p.x, p.y]).collect();
        let raw = subj.overlay(&clip, self.op, FillRule::EvenOdd);
        let ms = t0.elapsed().as_secs_f32() * 1000.0;

        let mut result: Vec<Vec<Vec<Pos2>>> = Vec::new();
        let mut vcount = 0usize;
        let mut area = 0.0f32;
        let mut ring_count = 0usize;
        for shape in raw {
            let mut shape_out: Vec<Vec<Pos2>> = Vec::new();
            for (i, contour) in shape.into_iter().enumerate() {
                vcount += contour.len();
                ring_count += 1;
                let pts: Vec<Pos2> = contour.into_iter().map(|p| Pos2::new(p[0], p[1])).collect();
                let s = signed_area(&pts);
                if i == 0 {
                    area += s.abs();
                } else {
                    area -= s.abs();
                }
                shape_out.push(pts);
            }
            result.push(shape_out);
        }

        // Every contour is a closed cycle of length k contributing k vertices
        // and k edges. Faces: one per ring (outer = filled, hole = unfilled)
        // plus the single unbounded face. Components = number of disjoint
        // shapes (each outer ring with its holes is one component).
        let euler = EulerCounts {
            v: vcount,
            e: vcount,
            f: ring_count + 1,
            components: result.len(),
        };

        self.cache = Some(Cache {
            op: self.op,
            vers_a: self.vers_a,
            vers_b: self.vers_b,
            result,
            vertex_count: vcount,
            area,
            ms,
            euler,
        });
    }
}

#[derive(Clone, Copy)]
pub enum Preset {
    Pentagon,
    Star,
    LShape,
    Rectangle,
}

impl Preset {
    fn build(self, center: Pos2) -> Vec<Pos2> {
        let scale = 120.0;
        match self {
            Preset::Pentagon => pentagon(center, scale),
            Preset::Star => star(center, scale, scale * 0.45, 5),
            Preset::LShape => l_shape(center, scale),
            Preset::Rectangle => rectangle(center, scale * 1.6, scale * 1.1),
        }
    }
}

fn pentagon(center: Pos2, radius: f32) -> Vec<Pos2> {
    polygon_regular(center, radius, 5, -std::f32::consts::FRAC_PI_2)
}

fn rectangle(center: Pos2, w: f32, h: f32) -> Vec<Pos2> {
    let hw = w * 0.5;
    let hh = h * 0.5;
    vec![
        center + egui::vec2(-hw, -hh),
        center + egui::vec2(hw, -hh),
        center + egui::vec2(hw, hh),
        center + egui::vec2(-hw, hh),
    ]
}

fn star(center: Pos2, outer: f32, inner: f32, points: usize) -> Vec<Pos2> {
    let mut out = Vec::with_capacity(points * 2);
    for i in 0..(points * 2) {
        let theta = -std::f32::consts::FRAC_PI_2
            + (i as f32) * std::f32::consts::PI / (points as f32);
        let r = if i % 2 == 0 { outer } else { inner };
        out.push(center + egui::vec2(r * theta.cos(), r * theta.sin()));
    }
    out
}

fn l_shape(center: Pos2, scale: f32) -> Vec<Pos2> {
    let s = scale;
    vec![
        center + egui::vec2(-s, -s),
        center + egui::vec2(0.4 * s, -s),
        center + egui::vec2(0.4 * s, 0.0),
        center + egui::vec2(s, 0.0),
        center + egui::vec2(s, s),
        center + egui::vec2(-s, s),
    ]
}

fn polygon_regular(center: Pos2, radius: f32, n: usize, start_angle: f32) -> Vec<Pos2> {
    (0..n)
        .map(|i| {
            let theta = start_angle + (i as f32) * std::f32::consts::TAU / (n as f32);
            center + egui::vec2(radius * theta.cos(), radius * theta.sin())
        })
        .collect()
}

pub(crate) fn signed_area(poly: &[Pos2]) -> f32 {
    if poly.len() < 3 {
        return 0.0;
    }
    let mut sum = 0.0;
    for i in 0..poly.len() {
        let a = poly[i];
        let b = poly[(i + 1) % poly.len()];
        sum += a.x * b.y - b.x * a.y;
    }
    sum * 0.5
}

fn fill_a() -> Color32 {
    theme::ACCENT.linear_multiply(0.22)
}
fn fill_b() -> Color32 {
    theme::ORANGE.linear_multiply(0.22)
}
fn stroke_a() -> Stroke {
    Stroke::new(1.5, theme::ACCENT.linear_multiply(0.85))
}
fn stroke_b() -> Stroke {
    Stroke::new(1.5, theme::ORANGE.linear_multiply(0.85))
}

pub(crate) fn paint_polygon(painter: &egui::Painter, poly: &[Pos2], fill: Color32, stroke: Stroke) {
    if poly.len() < 3 {
        return;
    }
    if fill.a() > 0 {
        let tris = triangulate(poly);
        if !tris.is_empty() {
            let mut mesh = egui::Mesh::default();
            for &p in poly {
                mesh.vertices.push(egui::epaint::Vertex {
                    pos: p,
                    uv: egui::epaint::WHITE_UV,
                    color: fill,
                });
            }
            for [a, b, c] in tris {
                mesh.indices.push(a as u32);
                mesh.indices.push(b as u32);
                mesh.indices.push(c as u32);
            }
            painter.add(egui::Shape::mesh(mesh));
        }
    }
    if stroke.width > 0.0 {
        let pts: Vec<Pos2> = poly.iter().copied().chain(std::iter::once(poly[0])).collect();
        painter.add(egui::Shape::line(pts, stroke));
    }
}

fn paint_vertex_handles(
    painter: &egui::Painter,
    poly: &[Pos2],
    side: Side,
    hover: Option<Pos2>,
    drag: Option<DragTarget>,
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
        let dragged = matches!(drag, Some(DragTarget::Vertex(s, idx)) if s == side && idx == i);
        let color = if hovered || dragged { theme::WARN } else { accent };
        canvas::paint_point(painter, p, color);
    }
}

pub(crate) fn point_in_polygon(p: Pos2, poly: &[Pos2]) -> bool {
    if poly.len() < 3 {
        return false;
    }
    let mut inside = false;
    let mut j = poly.len() - 1;
    for i in 0..poly.len() {
        let pi = poly[i];
        let pj = poly[j];
        let crosses = (pi.y > p.y) != (pj.y > p.y)
            && p.x < (pj.x - pi.x) * (p.y - pi.y) / (pj.y - pi.y + f32::EPSILON) + pi.x;
        if crosses {
            inside = !inside;
        }
        j = i;
    }
    inside
}

pub(crate) fn triangulate(poly: &[Pos2]) -> Vec<[usize; 3]> {
    let n = poly.len();
    if n < 3 {
        return Vec::new();
    }
    let mut verts: Vec<usize> = (0..n).collect();
    // Ear-clip expects CCW winding under the same sign convention as `orient`:
    // orient(a,b,c) > 0 for convex corners iff the polygon is CCW. That matches
    // signed_area > 0. Reverse when the input is CW (signed_area < 0), which is
    // what i_overlay emits for outer contours.
    if signed_area(poly) < 0.0 {
        verts.reverse();
    }
    let mut tris: Vec<[usize; 3]> = Vec::new();
    let mut guard = 0usize;
    let max_guard = n * n + 10;
    while verts.len() > 3 && guard < max_guard {
        guard += 1;
        let m = verts.len();
        let mut clipped = false;
        for i in 0..m {
            let ia = verts[(i + m - 1) % m];
            let ib = verts[i];
            let ic = verts[(i + 1) % m];
            let a = poly[ia];
            let b = poly[ib];
            let c = poly[ic];
            if orient(a, b, c) <= 0.0 {
                continue;
            }
            let mut any = false;
            for &j in &verts {
                if j == ia || j == ib || j == ic {
                    continue;
                }
                if point_in_triangle(poly[j], a, b, c) {
                    any = true;
                    break;
                }
            }
            if any {
                continue;
            }
            tris.push([ia, ib, ic]);
            verts.remove(i);
            clipped = true;
            break;
        }
        if !clipped {
            break;
        }
    }
    if verts.len() == 3 {
        tris.push([verts[0], verts[1], verts[2]]);
    }
    tris
}

fn orient(a: Pos2, b: Pos2, c: Pos2) -> f32 {
    (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x)
}

fn point_in_triangle(p: Pos2, a: Pos2, b: Pos2, c: Pos2) -> bool {
    let d1 = orient(p, a, b);
    let d2 = orient(p, b, c);
    let d3 = orient(p, c, a);
    let has_neg = d1 < 0.0 || d2 < 0.0 || d3 < 0.0;
    let has_pos = d1 > 0.0 || d2 > 0.0 || d3 > 0.0;
    !(has_neg && has_pos)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn triangulate_square() {
        let poly = rectangle(Pos2::new(0.0, 0.0), 10.0, 10.0);
        let tris = triangulate(&poly);
        assert_eq!(tris.len(), 2);
    }

    #[test]
    fn triangulate_l_shape() {
        let poly = l_shape(Pos2::new(0.0, 0.0), 10.0);
        let tris = triangulate(&poly);
        assert_eq!(tris.len(), 4);
    }

    #[test]
    fn union_area_equals_sum_minus_intersection() {
        let mut demo = PolygonOpsDemo::default();
        demo.last_rect = Some(Rect::from_min_size(
            Pos2::ZERO,
            egui::vec2(1000.0, 700.0),
        ));
        demo.a = rectangle(Pos2::new(200.0, 200.0), 200.0, 200.0);
        demo.b = rectangle(Pos2::new(300.0, 300.0), 200.0, 200.0);
        demo.vers_a = demo.vers_a.wrapping_add(1);
        demo.vers_b = demo.vers_b.wrapping_add(1);
        demo.op = OverlayRule::Union;
        demo.recompute_if_dirty();
        let union_area = demo.cache.as_ref().unwrap().area;
        demo.op = OverlayRule::Intersect;
        demo.recompute_if_dirty();
        let intersect_area = demo.cache.as_ref().unwrap().area;
        let expected = 200.0 * 200.0 + 200.0 * 200.0 - 100.0 * 100.0;
        assert!((union_area - expected).abs() < 1.0);
        assert!((intersect_area - 100.0 * 100.0).abs() < 1.0);
    }
}
