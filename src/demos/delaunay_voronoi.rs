use eframe::egui::{self, Color32, Pos2, Rect, Stroke};
use spade::handles::VoronoiVertex;
use spade::{DelaunayTriangulation, Point2, Triangulation};
use web_time::Instant;

use crate::canvas;
use crate::geometry::power_diagram::compute_power_cell;
use crate::theme;
use crate::ui::point_editor::{next_seed, seeded_points, PointEditor, HIT_RADIUS};

const INITIAL_SEED: u64 = 0x2BD1_F3C7;
const WEIGHT_CLAMP: f32 = 1500.0;
const WEIGHT_PER_SCROLL: f32 = 4.0;

pub struct DelaunayVoronoiDemo {
    editor: PointEditor,
    seed: u64,
    last_rect: Option<Rect>,
    show_delaunay: bool,
    show_voronoi: bool,
    show_circumcircle: bool,
    show_all_circumcircles: bool,
    show_power: bool,
    weights: Vec<f32>,
    weight_seed: u64,
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
            show_all_circumcircles: false,
            show_power: false,
            weights: Vec::new(),
            weight_seed: 0x9E37_79B9_u64,
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
        self.weights.clear();
        self.cache = None;
        self.triangles = 0;
        self.last_ms = 0.0;
        self.euler = Euler::default();
    }

    pub fn random_into_last_rect(&mut self, n: usize) {
        if let Some(r) = self.last_rect {
            self.editor.set(seeded_points(r, n, self.seed));
            self.seed = next_seed(self.seed);
            self.weights = vec![0.0; n];
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
    pub fn show_all_circumcircles_mut(&mut self) -> &mut bool {
        &mut self.show_all_circumcircles
    }
    pub fn show_power_mut(&mut self) -> &mut bool {
        &mut self.show_power
    }
    pub fn randomize_weights(&mut self) {
        let n = self.editor.len();
        if self.weights.len() != n {
            self.weights = vec![0.0; n];
        }
        for w in &mut self.weights {
            self.weight_seed ^= self.weight_seed << 13;
            self.weight_seed ^= self.weight_seed >> 7;
            self.weight_seed ^= self.weight_seed << 17;
            // Map to roughly [-300, 300] px² — visible without dominating.
            let r = (self.weight_seed as f32 / u64::MAX as f32) - 0.5;
            *w = r * 600.0;
        }
    }
    pub fn reset_weights(&mut self) {
        for w in &mut self.weights {
            *w = 0.0;
        }
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) {
        let frame = self.editor.run(ui);
        self.last_rect = Some(frame.rect);
        canvas::paint_grid(&frame.painter, frame.rect);

        // Sync weights vector to the current point count. PointEditor mutates
        // independently (add / drag / right-click delete), so we reconcile each
        // frame: extend with 0.0 for new points, truncate to drop deleted ones.
        let n_pts = self.editor.len();
        if self.weights.len() < n_pts {
            self.weights.resize(n_pts, 0.0);
        } else if self.weights.len() > n_pts {
            self.weights.truncate(n_pts);
        }

        // Scroll-wheel weight editing: gated by hover over the canvas + presence
        // of a site under the cursor, so it never steals page scroll on wasm.
        if self.show_power && frame.response.hovered() {
            let scroll = ui.input(|i| i.raw_scroll_delta.y);
            if scroll != 0.0 {
                if let Some(pos) = frame.response.hover_pos() {
                    if let Some(idx) = self.editor.nearest_within(pos, HIT_RADIUS) {
                        let w = (self.weights[idx] + scroll * WEIGHT_PER_SCROLL)
                            .clamp(-WEIGHT_CLAMP, WEIGHT_CLAMP);
                        self.weights[idx] = w;
                        ui.ctx().request_repaint();
                    }
                }
            }
        }

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

        let ctx = ui.ctx();
        // Voronoi cells and power cells are mutually exclusive: turning power on
        // suppresses the standard Voronoi-cell layer (their fills would compete).
        let voronoi_effective = self.show_voronoi && !self.show_power;
        let voronoi_a = ctx.animate_bool(egui::Id::new("dv_show_voronoi"), voronoi_effective);
        let power_a = ctx.animate_bool(egui::Id::new("dv_show_power"), self.show_power);
        let delaunay_a = ctx.animate_bool(egui::Id::new("dv_show_delaunay"), self.show_delaunay);
        let circle_a = ctx.animate_bool(egui::Id::new("dv_show_circumcircle"), self.show_circumcircle);
        let all_circles_a = ctx.animate_bool(
            egui::Id::new("dv_show_all_circumcircles"),
            self.show_all_circumcircles,
        );

        if voronoi_a > 0.01 {
            paint_voronoi_cells(&frame.painter, t, viewport, voronoi_a);
            paint_voronoi_edges(&frame.painter, t, viewport, voronoi_a);
        }
        if power_a > 0.01 {
            paint_power_cells(
                &frame.painter,
                self.editor.points(),
                &self.weights,
                viewport,
                power_a,
            );
        }
        if all_circles_a > 0.01 {
            paint_all_circumcircles(&frame.painter, t, viewport, all_circles_a);
        }
        if delaunay_a > 0.01 {
            paint_delaunay_edges(&frame.painter, t, delaunay_a);
        }

        self.focus = hover_vertex.map(|v| compute_focus(t, v, viewport));

        if let Some(v) = hover_vertex {
            highlight_cell(&frame.painter, t, v, viewport);
            if circle_a > 0.01 {
                paint_incident_circumcircles(&frame.painter, t, v, viewport, circle_a);
            }
        }

        if self.show_power {
            paint_weight_indicators(&frame.painter, self.editor.points(), &self.weights);
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
        if d2 <= r2 && best.is_none_or(|(_, bd)| d2 < bd) {
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
    alpha: f32,
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
        let color = palette[i % palette.len()].linear_multiply(0.22 * alpha);
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
    alpha: f32,
) {
    let stroke = Stroke::new(1.25, theme::FG.linear_multiply(0.45 * alpha));
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

fn paint_delaunay_edges(painter: &egui::Painter, t: &DelaunayTriangulation<Point2<f32>>, alpha: f32) {
    let stroke = Stroke::new(1.1, theme::ACCENT.linear_multiply(0.75 * alpha));
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

/// Every inner Delaunay triangle's circumcircle, rendered faintly. Demonstrates
/// the empty-circumcircle property: for each triangle, no other site lies
/// inside the drawn circle. Off by default because it's visually busy for
/// large point sets.
fn paint_all_circumcircles(
    painter: &egui::Painter,
    t: &DelaunayTriangulation<Point2<f32>>,
    viewport: Rect,
    alpha: f32,
) {
    let stroke = Stroke::new(0.75, theme::VIOLET.linear_multiply(0.35 * alpha));
    for face in t.inner_faces() {
        let (c, r2) = face.circumcircle();
        let center = Pos2::new(c.x, c.y);
        let radius = r2.max(0.0).sqrt();
        if !viewport.expand(radius + 20.0).contains(center) {
            continue;
        }
        painter.circle_stroke(center, radius, stroke);
    }
}

fn paint_incident_circumcircles(
    painter: &egui::Painter,
    t: &DelaunayTriangulation<Point2<f32>>,
    fv: spade::handles::FixedVertexHandle,
    viewport: Rect,
    alpha: f32,
) {
    let vertex = t.vertex(fv);
    let stroke = Stroke::new(1.0, theme::FG_DIM.linear_multiply(0.8 * alpha));
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

/// Paint power-diagram cells (weighted Voronoi). Each cell is the intersection
/// of n−1 halfplanes (radical axes) clipped to the viewport. Colours match the
/// rotating Voronoi palette so toggling between the two views is visually
/// continuous; sites with empty power cells (dominated by a neighbour) just
/// emit nothing.
fn paint_power_cells(
    painter: &egui::Painter,
    sites: &[Pos2],
    weights: &[f32],
    viewport: Rect,
    alpha: f32,
) {
    let palette = cell_colors();
    let edge_stroke = Stroke::new(1.25, theme::FG.linear_multiply(0.45 * alpha));
    for i in 0..sites.len() {
        let cell = compute_power_cell(i, sites, weights, viewport);
        if cell.len() < 3 {
            continue;
        }
        let fill = palette[i % palette.len()].linear_multiply(0.22 * alpha);
        painter.add(egui::Shape::convex_polygon(cell.clone(), fill, Stroke::NONE));
        // Each radical-axis edge gets drawn twice (once per neighbouring cell).
        // Cheap at n ≤ 100 and avoids a separate edge-extraction pass.
        for k in 0..cell.len() {
            let a = cell[k];
            let b = cell[(k + 1) % cell.len()];
            painter.line_segment([a, b], edge_stroke);
        }
    }
}

/// Per-site weight indicator: a thin ring of radius √max(w, 0). Negative weights
/// render a faint cross instead, signalling "shrunk" without competing with the
/// site dot. Both visualisations stay below the site marker in z-order.
fn paint_weight_indicators(painter: &egui::Painter, sites: &[Pos2], weights: &[f32]) {
    let ring_stroke = Stroke::new(1.0, theme::ACCENT.linear_multiply(0.55));
    let cross_stroke = Stroke::new(0.75, theme::FG_DIM.linear_multiply(0.7));
    for (i, &p) in sites.iter().enumerate() {
        let w = weights.get(i).copied().unwrap_or(0.0);
        if w > 0.5 {
            painter.circle_stroke(p, w.sqrt(), ring_stroke);
        } else if w < -0.5 {
            let r = 5.0_f32;
            painter.line_segment([p + egui::vec2(-r, -r), p + egui::vec2(r, r)], cross_stroke);
            painter.line_segment([p + egui::vec2(-r, r), p + egui::vec2(r, -r)], cross_stroke);
        }
    }
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
