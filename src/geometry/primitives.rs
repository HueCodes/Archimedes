use eframe::egui::Pos2;

/// 2D orientation test using naive f32 cross product.
///
/// Returns:
/// - positive: counterclockwise (left turn from pq to pr)
/// - negative: clockwise (right turn)
/// - zero: collinear
pub fn orient2d_naive(p: Pos2, q: Pos2, r: Pos2) -> f32 {
    (q.x - p.x) * (r.y - p.y) - (q.y - p.y) * (r.x - p.x)
}

/// Shewchuk adaptive-precision orientation predicate. Exact on nearly-degenerate
/// inputs where the naive float version silently returns the wrong sign.
///
/// See: Shewchuk 1997, *Adaptive Precision Floating-Point Arithmetic and Fast
/// Robust Geometric Predicates*.
pub fn orient2d_robust(p: Pos2, q: Pos2, r: Pos2) -> f64 {
    robust::orient2d(
        robust::Coord { x: p.x as f64, y: p.y as f64 },
        robust::Coord { x: q.x as f64, y: q.y as f64 },
        robust::Coord { x: r.x as f64, y: r.y as f64 },
    )
}
