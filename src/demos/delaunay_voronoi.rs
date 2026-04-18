use eframe::egui::{self, Color32, Pos2, Rect, Stroke};
use spade::handles::VoronoiVertex;
use spade::{DelaunayTriangulation, Point2, Triangulation};
use web_time::Instant;

use crate::canvas;
use crate::theme;
use crate::ui::point_editor::{seeded_points, PointEditor};

const INITIAL_SEED: u64 = 0x2BD1_F3C7;

pub struct DelaunayVoronoiDemo {
    editor: PointEditor,
    seed: u64,
    last_rect: Option<Rect>,
    show_delaunay: bool,
    show_voronoi: bool,
    show_circumcircle: bool,
    cache: Option<Cache>,
    triangles: usize,
    last_ms: f32,
    euler: Euler,
    focus: Option<Focus>,
}

#[derive(Clone, Copy)]
pub struct Focus {
    pub degree: usize,
    pub cell_area: f32,
    pub nearest_dist: f32,
    pub is_hull: bool,
}

#[derive(Default, Clone, Copy)]
pub struct Euler {
    pub v: usize,
    pub e: usize,
    pub f: usize,
}

impl Euler {
    pub fn characteristic(self) -> i64 {
        self.v as i64 - self.e as i64 + self.f as i64
    }
}

struct Cache {
    version: u64,
    triangulation: DelaunayTriangulation<Point2<f32>>,
}

impl Default for DelaunayVoronoiDemo {
    fn default() -> Self {
        Self {
            editor: PointEditor::default(),
            seed: INITIAL_SEED,
            last_rect: None,
            show_delaunay: true,
            show_voronoi: true,
            show_circumcircle: true,
            cache: None,
            triangles: 0,
            last_ms: 0.0,
            euler: Euler::default(),
            focus: None,
        }
    }
}

impl DelaunayVoronoiDemo {
    pub fn metrics(&self) -> (usize, usize, f32) {
        (self.editor.len(), self.triangles, self.last_ms)
    }

    pub fn euler(&self) -> Euler {
        self.euler
    }

    pub fn focus(&self) -> Option<Focus> {
        self.focus
    }

    pub fn seed(&self) -> u64 {
        self.seed
    }

    pub fn clear(&mut self) {
        self.editor.clear();
        self.cache = None;
        self.triangles = 0;
        self.last_ms = 0.0;
        self.euler = Euler::default();
    }

    pub fn random_into_last_rect(&mut self, n: usize) {
        if let Some(r) = self.last_rect {
            self.editor.set(seeded_points(r, n, self.seed));
            self.seed = self
                .seed
                .wrapping_mul(0x5851_F42D_4C95_7F2D)
                .wrapping_add(0x14057B7E_F767_814F);
            self.cache = None;
        }
    }

    pub fn show_delaunay_mut(&mut self) -> &mut bool {
        &mut self.show_delaunay
    }
    pub fn show_voronoi_mut(&mut self) -> &mut bool {
        &mut self.show_voronoi
    }
    pub fn show_circumcircle_mut(&mut self) -> &mut bool {
        &mut self.show_circumcircle
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) {
        let frame = self.editor.run(ui);
        self.last_rect = Some(frame.rect);
        canvas::paint_grid(&frame.painter, frame.rect);

        if self.editor.len() < 3 {
            canvas::paint_empty_state(
                &frame.painter,
                frame.rect,
                "Add at least 3 sites",
                "click add · drag move · right-click delete · R random",
            );
            self.editor
                .paint(&frame.painter, theme::FG, frame.response.hover_pos());
            self.triangles = 0;
            self.last_ms = 0.0;
            return;
        }

        let version = self.editor.version();
        let need_rebuild = self
            .cache
            .as_ref()
            .map(|c| c.version != version)
            .unwrap_or(true);

        if need_rebuild {
            let t0 = Instant::now();
            let mut t = DelaunayTriangulation::<Point2<f32>>::new();
            for &p in self.editor.points() {
                let _ = t.insert(Point2::new(p.x, p.y));
            }
            self.last_ms = t0.elapsed().as_secs_f32() * 1000.0;
            self.cache = Some(Cache {
                version,
                triangulation: t,
            });
        }

        let cache = self.cache.as_ref().unwrap();
        let t = &cache.triangulation;
        self.triangles = t.num_inner_faces();
        self.euler = Euler {
            v: t.num_vertices(),
            e: t.num_undirected_edges(),
            f: t.num_all_faces(),
        };

        let viewport = frame.rect;
        let hover = frame.response.hover_pos();
        let hover_vertex = hover.and_then(|h| nearest_vertex(t, h, 14.0));

        if self.show_voronoi {
            paint_voronoi_cells(&frame.painter, t, viewport);
        }
        if self.show_voronoi {
            paint_voronoi_edges(&frame.painter, t, viewport);
        }
        if self.show_delaunay {
            paint_delaunay_edges(&frame.painter, t);
        }

        self.focus = hover_vertex.map(|v| compute_focus(t, v, viewport));

        if let Some(v) = hover_vertex {
            highlight_cell(&frame.painter, t, v, viewport);
            if self.show_circumcircle {
                paint_incident_circumcircles(&frame.painter, t, v, viewport);
            }
        }

        self.editor
            .paint(&frame.painter, theme::FG, frame.response.hover_pos());
    }
}

fn compute_focus(
    t: &DelaunayTriangulation<Point2<f32>>,
    fv: spade::handles::FixedVertexHandle,
    viewport: Rect,
) -> Focus {
    let vertex = t.vertex(fv);
    let degree = vertex.out_edges().count();
    let is_hull = vertex
        .out_edges()
        .any(|e| e.face().as_inner().is_none());

    let far_scale = (viewport.width() + viewport.height()) * 4.0;
    let polygon = voronoi_cell_polygon(vertex, far_scale);
    let clip = rect_to_poly(viewport);
    let clipped = sutherland_hodgman(&polygon, &clip);
    let cell_area = polygon_abs_area(&clipped);

    let pos = vertex.position();
    let mut nearest = f32::INFINITY;
    for e in vertex.out_edges() {
        let np = e.to().position();
        let dx = np.x - pos.x;
        let dy = np.y - pos.y;
        let d = (dx * dx + dy * dy).sqrt();
        if d < nearest {
            nearest = d;
        }
    }
    Focus {
        degree,
        cell_area,
        nearest_dist: if nearest.is_finite() { nearest } else { 0.0 },
        is_hull,
    }
}

fn polygon_abs_area(poly: &[Pos2]) -> f32 {
    if poly.len() < 3 {
        return 0.0;
    }
    let mut sum = 0.0;
    for i in 0..poly.len() {
        let a = poly[i];
        let b = poly[(i + 1) % poly.len()];
        sum += a.x * b.y - b.x * a.y;
    }
    (sum * 0.5).abs()
}

fn nearest_vertex(
    t: &DelaunayTriangulation<Point2<f32>>,
    pos: Pos2,
    radius: f32,
) -> Option<spade::handles::FixedVertexHandle> {
    let r2 = radius * radius;
    let mut best: Option<(spade::handles::FixedVertexHandle, f32)> = None;
    for v in t.vertices() {
        let p = v.position();
        let dx = p.x - pos.x;
        let dy = p.y - pos.y;
        let d2 = dx * dx + dy * dy;
        if d2 <= r2 && best.map_or(true, |(_, bd)| d2 < bd) {
            best = Some((v.fix(), d2));
        }
    }
    best.map(|(h, _)| h)
}

fn cell_colors() -> [Color32; 5] {
    [
        theme::ACCENT,
        theme::OK,
        theme::VIOLET,
        theme::ORANGE,
        theme::WARN,
    ]
}

fn paint_voronoi_cells(
    painter: &egui::Painter,
    t: &DelaunayTriangulation<Point2<f32>>,
    viewport: Rect,
) {
    let palette = cell_colors();
    let far_scale = (viewport.width() + viewport.height()) * 4.0;
    let clip = rect_to_poly(viewport);

    for (i, vertex) in t.vertices().enumerate() {
        let polygon = voronoi_cell_polygon(vertex, far_scale);
        if polygon.len() < 3 {
            continue;
        }
        let clipped = sutherland_hodgman(&polygon, &clip);
        if clipped.len() < 3 {
            continue;
        }
        let color = palette[i % palette.len()].linear_multiply(0.22);
        painter.add(egui::Shape::convex_polygon(
            clipped,
            color,
            Stroke::NONE,
        ));
    }
}

fn paint_voronoi_edges(
    painter: &egui::Painter,
    t: &DelaunayTriangulation<Point2<f32>>,
    viewport: Rect,
) {
    let stroke = Stroke::new(1.25, theme::FG.linear_multiply(0.45));
    let far_scale = (viewport.width() + viewport.height()) * 4.0;
    let clip = rect_to_poly(viewport);

    for edge in t.undirected_voronoi_edges() {
        let [a, b] = edge.vertices();
        let pa = voronoi_vertex_pos(a, far_scale);
        let pb = voronoi_vertex_pos(b, far_scale);
        let (Some(pa), Some(pb)) = (pa, pb) else {
            continue;
        };
        if let Some((ca, cb)) = clip_segment_to_poly(pa, pb, &clip) {
            painter.line_segment([ca, cb], stroke);
        }
    }
}

fn paint_delaunay_edges(painter: &egui::Painter, t: &DelaunayTriangulation<Point2<f32>>) {
    let stroke = Stroke::new(1.1, theme::ACCENT.linear_multiply(0.75));
    for edge in t.undirected_edges() {
        let [a, b] = edge.vertices();
        let pa = Pos2::new(a.position().x, a.position().y);
        let pb = Pos2::new(b.position().x, b.position().y);
        painter.line_segment([pa, pb], stroke);
    }
}

fn highlight_cell(
    painter: &egui::Painter,
    t: &DelaunayTriangulation<Point2<f32>>,
    fv: spade::handles::FixedVertexHandle,
    viewport: Rect,
) {
    let vertex = t.vertex(fv);
    let far_scale = (viewport.width() + viewport.height()) * 4.0;
    let polygon = voronoi_cell_polygon(vertex, far_scale);
    if polygon.len() < 3 {
        return;
    }
    let clip = rect_to_poly(viewport);
    let clipped = sutherland_hodgman(&polygon, &clip);
    if clipped.len() < 3 {
        return;
    }
    painter.add(egui::Shape::convex_polygon(
        clipped.clone(),
        theme::WARN.linear_multiply(0.18),
        Stroke::new(1.75, theme::WARN),
    ));
}

fn paint_incident_circumcircles(
    painter: &egui::Painter,
    t: &DelaunayTriangulation<Point2<f32>>,
    fv: spade::handles::FixedVertexHandle,
    viewport: Rect,
) {
    let vertex = t.vertex(fv);
    let stroke = Stroke::new(1.0, theme::FG_DIM.linear_multiply(0.8));
    for edge in vertex.out_edges() {
        if let Some(face) = edge.face().as_inner() {
            let (c, r2) = face.circumcircle();
            let center = Pos2::new(c.x, c.y);
            let radius = r2.max(0.0).sqrt();
            if !viewport
                .expand(radius + 20.0)
                .contains(center)
            {
                continue;
            }
            painter.circle_stroke(center, radius, stroke);
        }
    }
}

/// Build a polygon for a Voronoi cell by walking the Delaunay vertex's outgoing
/// edges in CCW order. For an interior vertex every `edge.face()` is an inner
/// triangle and the cell polygon is just those circumcenters. For a hull vertex
/// exactly one outgoing edge (V→A, the backward hull edge) has the outer face
/// on its left; the cell then opens out through that slot. We restart the walk
/// at V→B (the forward hull edge = CCW-next), emit far(V→B, right normal),
/// walk through the interior triangles emitting circumcenters, and close with
/// far(V→A, left normal).
fn voronoi_cell_polygon<'a>(
    vertex: spade::handles::VertexHandle<'a, Point2<f32>>,
    far_scale: f32,
) -> Vec<Pos2> {
    let out_edges: Vec<_> = vertex.out_edges().collect();
    let n = out_edges.len();
    if n == 0 {
        return Vec::new();
    }
    let outer_idx = out_edges
        .iter()
        .position(|e| e.face().as_inner().is_none());

    let mut poly: Vec<Pos2> = Vec::new();
    match outer_idx {
        None => {
            for e in &out_edges {
                if let Some(f) = e.face().as_inner() {
                    let c = f.circumcenter();
                    poly.push(Pos2::new(c.x, c.y));
                }
            }
        }
        Some(i) => {
            let v_to_a = out_edges[i];
            let v_to_b = out_edges[(i + 1) % n];
            poly.push(far_along_edge_normal(v_to_b, NormalSide::Right, far_scale));
            let mut k = (i + 1) % n;
            while k != i {
                if let Some(f) = out_edges[k].face().as_inner() {
                    let c = f.circumcenter();
                    poly.push(Pos2::new(c.x, c.y));
                }
                k = (k + 1) % n;
            }
            poly.push(far_along_edge_normal(v_to_a, NormalSide::Left, far_scale));
        }
    }
    poly
}

#[derive(Clone, Copy)]
enum NormalSide {
    Left,
    Right,
}

fn far_along_edge_normal<'a>(
    edge: spade::handles::DirectedEdgeHandle<'a, Point2<f32>, (), (), ()>,
    side: NormalSide,
    far_scale: f32,
) -> Pos2 {
    let a = edge.from().position();
    let b = edge.to().position();
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    let len = (dx * dx + dy * dy).sqrt().max(1e-6);
    let (nx, ny) = match side {
        NormalSide::Left => (-dy / len, dx / len),
        NormalSide::Right => (dy / len, -dx / len),
    };
    let mid = Pos2::new((a.x + b.x) * 0.5, (a.y + b.y) * 0.5);
    mid + egui::vec2(nx, ny) * far_scale
}

fn voronoi_vertex_pos<'a>(
    v: VoronoiVertex<'a, Point2<f32>, (), (), ()>,
    far_scale: f32,
) -> Option<Pos2> {
    match v {
        VoronoiVertex::Inner(f) => {
            let c = f.circumcenter();
            Some(Pos2::new(c.x, c.y))
        }
        VoronoiVertex::Outer(dv_edge) => outer_vertex_pos(dv_edge, far_scale),
    }
}

fn outer_vertex_pos<'a>(
    dv_edge: spade::handles::DirectedVoronoiEdge<'a, Point2<f32>, (), (), ()>,
    far_scale: f32,
) -> Option<Pos2> {
    // Anchor the ray from the nearest known inner vertex if any, otherwise the
    // midpoint of the dual Delaunay edge. Direction is the dual edge's normal
    // rotated outward (Point2 dual rotated 90 CW points outside a CCW hull).
    let de = dv_edge.as_delaunay_edge();
    let a = de.from().position();
    let b = de.to().position();
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    let len = (dx * dx + dy * dy).sqrt().max(1e-6);
    let nx = dy / len;
    let ny = -dx / len;
    let mid = Pos2::new((a.x + b.x) * 0.5, (a.y + b.y) * 0.5);
    Some(mid + egui::vec2(nx, ny) * far_scale)
}

fn rect_to_poly(r: Rect) -> Vec<Pos2> {
    vec![
        Pos2::new(r.min.x, r.min.y),
        Pos2::new(r.max.x, r.min.y),
        Pos2::new(r.max.x, r.max.y),
        Pos2::new(r.min.x, r.max.y),
    ]
}

/// Sutherland-Hodgman: clip `subject` polygon against a convex `clip` polygon.
/// Both polygons must be CCW-oriented; for our viewport rect that is the case.
fn sutherland_hodgman(subject: &[Pos2], clip: &[Pos2]) -> Vec<Pos2> {
    let mut output = subject.to_vec();
    if output.is_empty() {
        return output;
    }
    for i in 0..clip.len() {
        let a = clip[i];
        let b = clip[(i + 1) % clip.len()];
        let input = std::mem::take(&mut output);
        if input.is_empty() {
            break;
        }
        let mut prev = input[input.len() - 1];
        let mut prev_inside = is_left(a, b, prev);
        for &cur in &input {
            let cur_inside = is_left(a, b, cur);
            if cur_inside {
                if !prev_inside {
                    if let Some(ip) = intersect(a, b, prev, cur) {
                        output.push(ip);
                    }
                }
                output.push(cur);
            } else if prev_inside {
                if let Some(ip) = intersect(a, b, prev, cur) {
                    output.push(ip);
                }
            }
            prev = cur;
            prev_inside = cur_inside;
        }
    }
    output
}

fn clip_segment_to_poly(a: Pos2, b: Pos2, clip: &[Pos2]) -> Option<(Pos2, Pos2)> {
    // Liang-Barsky-lite: parametric clip against each clip edge (assumed convex CCW).
    let mut t0 = 0.0f32;
    let mut t1 = 1.0f32;
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    for i in 0..clip.len() {
        let c0 = clip[i];
        let c1 = clip[(i + 1) % clip.len()];
        // Inside half-plane: cross((c1-c0), (p-c0)) >= 0.
        let ex = c1.x - c0.x;
        let ey = c1.y - c0.y;
        let nx = -ey;
        let ny = ex;
        // f(p) = n · (p - c0). Inside if f >= 0.
        let f_a = nx * (a.x - c0.x) + ny * (a.y - c0.y);
        let denom = -(nx * dx + ny * dy);
        let num = f_a;
        if denom.abs() < 1e-9 {
            if num < 0.0 {
                return None;
            }
            continue;
        }
        let t = num / denom;
        if denom > 0.0 {
            if t > t1 {
                return None;
            }
            if t > t0 {
                t0 = t;
            }
        } else {
            if t < t0 {
                return None;
            }
            if t < t1 {
                t1 = t;
            }
        }
    }
    if t0 > t1 {
        return None;
    }
    Some((
        Pos2::new(a.x + dx * t0, a.y + dy * t0),
        Pos2::new(a.x + dx * t1, a.y + dy * t1),
    ))
}

fn is_left(a: Pos2, b: Pos2, p: Pos2) -> bool {
    (b.x - a.x) * (p.y - a.y) - (b.y - a.y) * (p.x - a.x) >= 0.0
}

fn intersect(a: Pos2, b: Pos2, p: Pos2, q: Pos2) -> Option<Pos2> {
    let r = egui::vec2(b.x - a.x, b.y - a.y);
    let s = egui::vec2(q.x - p.x, q.y - p.y);
    let denom = r.x * s.y - r.y * s.x;
    if denom.abs() < 1e-9 {
        return None;
    }
    let qp = egui::vec2(p.x - a.x, p.y - a.y);
    let t = (qp.x * s.y - qp.y * s.x) / denom;
    Some(Pos2::new(a.x + r.x * t, a.y + r.y * t))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sh_clips_polygon_to_rect() {
        let subject = vec![
            Pos2::new(-100.0, 50.0),
            Pos2::new(100.0, 50.0),
            Pos2::new(100.0, 150.0),
            Pos2::new(-100.0, 150.0),
        ];
        let clip = vec![
            Pos2::new(0.0, 0.0),
            Pos2::new(200.0, 0.0),
            Pos2::new(200.0, 200.0),
            Pos2::new(0.0, 200.0),
        ];
        let out = sutherland_hodgman(&subject, &clip);
        assert_eq!(out.len(), 4);
        for p in &out {
            assert!(p.x >= 0.0 && p.x <= 200.0);
            assert!(p.y >= 0.0 && p.y <= 200.0);
        }
    }

    #[test]
    fn segment_clip_outside_returns_none() {
        let clip = vec![
            Pos2::new(0.0, 0.0),
            Pos2::new(100.0, 0.0),
            Pos2::new(100.0, 100.0),
            Pos2::new(0.0, 100.0),
        ];
        assert!(clip_segment_to_poly(
            Pos2::new(-10.0, -10.0),
            Pos2::new(-5.0, -5.0),
            &clip
        )
        .is_none());
    }
}
