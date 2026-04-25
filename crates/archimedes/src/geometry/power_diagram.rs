//! Power diagrams (weighted Voronoi). Each site i has a weight w_i and the
//! power distance from a query point p is d²(p, s_i) − w_i. The power cell
//! of site i is the locus of points where i minimizes that distance:
//!
//!     cell_i = { p : d²(p, s_i) − w_i ≤ d²(p, s_j) − w_j  ∀j ≠ i }
//!
//! Each pairwise inequality reduces to a halfplane in p (the radical axis):
//!
//!     2 p · (s_j − s_i) ≤ |s_j|² − |s_i|² + (w_i − w_j)
//!
//! The cell is the intersection of n−1 such halfplanes with the viewport rect.
//! At all-zero weights this collapses to the standard Voronoi diagram.
//!
//! Reference: Aurenhammer (1987), *Power diagrams: properties, algorithms,
//! and applications*, SIAM J. Comput. 16(1).

use eframe::egui::{Pos2, Rect};

/// Compute the power cell of `sites[site_idx]` clipped to `viewport`.
/// Returns the cell polygon in CCW order (matching the viewport's orientation),
/// or an empty Vec if the site is dominated by a neighbor.
pub fn compute_power_cell(
    site_idx: usize,
    sites: &[Pos2],
    weights: &[f32],
    viewport: Rect,
) -> Vec<Pos2> {
    if sites.is_empty() || site_idx >= sites.len() || sites.len() != weights.len() {
        return Vec::new();
    }
    let mut poly = vec![
        Pos2::new(viewport.min.x, viewport.min.y),
        Pos2::new(viewport.max.x, viewport.min.y),
        Pos2::new(viewport.max.x, viewport.max.y),
        Pos2::new(viewport.min.x, viewport.max.y),
    ];
    let si = sites[site_idx];
    let wi = weights[site_idx];
    let si_sq = si.x * si.x + si.y * si.y;
    for (j, &sj) in sites.iter().enumerate() {
        if j == site_idx {
            continue;
        }
        let wj = weights[j];
        let sj_sq = sj.x * sj.x + sj.y * sj.y;
        // Inside the cell of i: 2 p · (s_j − s_i) ≤ (|s_j|² − |s_i|²) + (w_i − w_j).
        // Rewrite as n · p ≤ c with n = s_j − s_i, c = ((|s_j|² − |s_i|²) + (w_i − w_j)) / 2.
        let nx = sj.x - si.x;
        let ny = sj.y - si.y;
        let c = 0.5 * ((sj_sq - si_sq) + (wi - wj));
        poly = clip_halfplane(&poly, nx, ny, c);
        if poly.is_empty() {
            break;
        }
    }
    poly
}

/// Clip a polygon against the halfplane { p : nx·p.x + ny·p.y ≤ c }. The
/// classical inner step of Sutherland-Hodgman, specialised for halfplanes
/// (no need for a polygonal clip window).
fn clip_halfplane(poly: &[Pos2], nx: f32, ny: f32, c: f32) -> Vec<Pos2> {
    if poly.is_empty() {
        return Vec::new();
    }
    let inside = |p: Pos2| nx * p.x + ny * p.y <= c;
    let mut out = Vec::with_capacity(poly.len() + 1);
    let mut prev = poly[poly.len() - 1];
    let mut prev_in = inside(prev);
    for &cur in poly {
        let cur_in = inside(cur);
        if cur_in {
            if !prev_in {
                if let Some(ip) = intersect_line(prev, cur, nx, ny, c) {
                    out.push(ip);
                }
            }
            out.push(cur);
        } else if prev_in {
            if let Some(ip) = intersect_line(prev, cur, nx, ny, c) {
                out.push(ip);
            }
        }
        prev = cur;
        prev_in = cur_in;
    }
    out
}

fn intersect_line(a: Pos2, b: Pos2, nx: f32, ny: f32, c: f32) -> Option<Pos2> {
    let na = nx * a.x + ny * a.y;
    let nb = nx * b.x + ny * b.y;
    let denom = nb - na;
    if denom.abs() < 1e-9 {
        return None;
    }
    let t = (c - na) / denom;
    Some(Pos2::new(a.x + t * (b.x - a.x), a.y + t * (b.y - a.y)))
}

/// Absolute polygon area via the shoelace formula. Used by tests to assert
/// "this cell is full / empty"; the demo has its own copy upstream.
#[cfg(test)]
fn polygon_area(poly: &[Pos2]) -> f32 {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn vp() -> Rect {
        Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(400.0, 400.0))
    }

    /// At all-zero weights, power cells reduce to Voronoi cells. We verify by
    /// checking that every interior cell vertex (one not lying on the viewport
    /// boundary) is equidistant from the source site and at least one other
    /// site — the defining property of a Voronoi vertex.
    #[test]
    fn power_cells_equal_voronoi_at_zero_weights() {
        let sites = vec![
            Pos2::new(100.0, 100.0),
            Pos2::new(300.0, 100.0),
            Pos2::new(200.0, 300.0),
            Pos2::new(150.0, 200.0),
            Pos2::new(250.0, 200.0),
        ];
        let weights = vec![0.0_f32; sites.len()];
        let viewport = vp();

        for i in 0..sites.len() {
            let cell = compute_power_cell(i, &sites, &weights, viewport);
            assert!(cell.len() >= 3, "cell {i} too small: {}", cell.len());
            for &v in &cell {
                let on_boundary = (v.x - viewport.min.x).abs() < 0.5
                    || (v.x - viewport.max.x).abs() < 0.5
                    || (v.y - viewport.min.y).abs() < 0.5
                    || (v.y - viewport.max.y).abs() < 0.5;
                if on_boundary {
                    continue;
                }
                let di = (v - sites[i]).length();
                let mut min_other = f32::INFINITY;
                for (j, &sj) in sites.iter().enumerate() {
                    if j == i {
                        continue;
                    }
                    let d = (v - sj).length();
                    if d < min_other {
                        min_other = d;
                    }
                }
                assert!(
                    (di - min_other).abs() < 1.0,
                    "cell {i} vertex {:?}: d_i = {}, min_other = {}",
                    v,
                    di,
                    min_other
                );
            }
        }
    }

    /// One very heavy site engulfs the others: its cell should cover almost
    /// the entire viewport, and the dominated sites should yield empty cells.
    #[test]
    fn heavy_weight_dominates_neighbors() {
        let sites = vec![
            Pos2::new(200.0, 200.0),
            Pos2::new(100.0, 100.0),
            Pos2::new(300.0, 100.0),
            Pos2::new(200.0, 300.0),
        ];
        let weights = vec![100_000.0, 0.0, 0.0, 0.0];
        let viewport = vp();

        let heavy = compute_power_cell(0, &sites, &weights, viewport);
        let area = polygon_area(&heavy);
        let total = viewport.width() * viewport.height();
        assert!(
            area / total > 0.9,
            "heavy cell covers only {:.1}% of viewport",
            100.0 * area / total
        );

        for j in 1..sites.len() {
            let cell = compute_power_cell(j, &sites, &weights, viewport);
            assert!(
                polygon_area(&cell) < 100.0,
                "neighbour {j} survived domination with area {}",
                polygon_area(&cell)
            );
        }
    }

    /// A single very-negative weight makes site 0 dominated; its cell area
    /// should be effectively zero.
    #[test]
    fn power_cell_of_dominated_site_is_empty() {
        let sites = vec![
            Pos2::new(200.0, 200.0),
            Pos2::new(100.0, 100.0),
            Pos2::new(300.0, 100.0),
            Pos2::new(200.0, 300.0),
        ];
        let weights = vec![-100_000.0, 0.0, 0.0, 0.0];
        let viewport = vp();

        let cell = compute_power_cell(0, &sites, &weights, viewport);
        assert!(
            polygon_area(&cell) < 1.0,
            "dominated cell still has area {}",
            polygon_area(&cell)
        );
    }

    /// Sanity: a single site fills the whole viewport, regardless of weight.
    #[test]
    fn single_site_fills_viewport() {
        let sites = vec![Pos2::new(200.0, 200.0)];
        let weights = vec![0.0];
        let viewport = vp();
        let cell = compute_power_cell(0, &sites, &weights, viewport);
        let area = polygon_area(&cell);
        let total = viewport.width() * viewport.height();
        assert!((area - total).abs() < 1.0);
    }
}
