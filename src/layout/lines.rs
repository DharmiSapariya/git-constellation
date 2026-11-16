//! Lines: segment geometry, minimum spanning tree, extra edges, crossing tests.

use super::dist;
use crate::model::Point;

/// Extra edges must not make an angle sharper than this (degrees).
pub const MIN_EDGE_ANGLE_DEG: f64 = 20.0;
/// An extra edge may not pass this close to another star's edge.
pub const EDGE_STAR_CLEARANCE: f64 = 6.0;
/// Tolerance for "these points are on one line".
const EPS: f64 = 1e-9;

fn orient(a: Point, b: Point, c: Point) -> f64 {
    (b.0 - a.0) * (c.1 - a.1) - (b.1 - a.1) * (c.0 - a.0)
}

fn within_box(a: Point, b: Point, p: Point) -> bool {
    p.0 >= a.0.min(b.0) - EPS
        && p.0 <= a.0.max(b.0) + EPS
        && p.1 >= a.1.min(b.1) - EPS
        && p.1 <= a.1.max(b.1) + EPS
}

/// Do segments `ab` and `cd` intersect (crossing, or one touching the other)?
/// Callers handle segments that share an endpoint star themselves.
pub fn segments_cross(a: Point, b: Point, c: Point, d: Point) -> bool {
    let (o1, o2, o3, o4) = (
        orient(a, b, c),
        orient(a, b, d),
        orient(c, d, a),
        orient(c, d, b),
    );
    let proper = ((o1 > EPS && o2 < -EPS) || (o1 < -EPS && o2 > EPS))
        && ((o3 > EPS && o4 < -EPS) || (o3 < -EPS && o4 > EPS));
    proper
        || (o1.abs() <= EPS && within_box(a, b, c))
        || (o2.abs() <= EPS && within_box(a, b, d))
        || (o3.abs() <= EPS && within_box(c, d, a))
        || (o4.abs() <= EPS && within_box(c, d, b))
}

/// Distance from point `p` to the segment `ab`.
pub fn point_segment_distance(p: Point, a: Point, b: Point) -> f64 {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let length_sq = dx * dx + dy * dy;
    if length_sq < EPS {
        return dist(p, a);
    }
    let t = (((p.0 - a.0) * dx + (p.1 - a.1) * dy) / length_sq).clamp(0.0, 1.0);
    dist(p, (a.0 + t * dx, a.1 + t * dy))
}

/// Angle in degrees between the vectors `s->p` and `s->q`.
pub fn angle_at(s: Point, p: Point, q: Point) -> f64 {
    let (ux, uy, vx, vy) = (p.0 - s.0, p.1 - s.1, q.0 - s.0, q.1 - s.1);
    let denom = (ux.hypot(uy) * vx.hypot(vy)).max(EPS);
    ((ux * vx + uy * vy) / denom)
        .clamp(-1.0, 1.0)
        .acos()
        .to_degrees()
}

/// Do two edges (pairs of star indices) cross or overlap, given star positions?
/// Edges that only share an endpoint star are fine unless they lie on top of
/// each other.
pub fn edges_cross(positions: &[Point], e1: (usize, usize), e2: (usize, usize)) -> bool {
    let shared = [e1.0, e1.1].into_iter().find(|i| *i == e2.0 || *i == e2.1);
    match shared {
        None => segments_cross(
            positions[e1.0],
            positions[e1.1],
            positions[e2.0],
            positions[e2.1],
        ),
        Some(s) => {
            let other = |e: (usize, usize)| if e.0 == s { e.1 } else { e.0 };
            angle_at(positions[s], positions[other(e1)], positions[other(e2)]) < 1.0
        }
    }
}

/// Connect a constellation's stars: a minimum spanning tree (Prim, ties go to
/// the lower index) plus `floor(n / 4)` extra short edges that cross nothing.
/// `points` and `radii` are local to the constellation; `others` are segments
/// of constellations already finished. Returns local index pairs `(a, b)`, `a < b`.
pub fn build_edges(
    points: &[Point],
    radii: &[f64],
    others: &[(Point, Point)],
) -> Vec<(usize, usize)> {
    let n = points.len();
    if n < 2 {
        return Vec::new();
    }
    let mut edges = minimum_spanning_tree(points);

    let wanted = n / 4;
    let mut candidates: Vec<(f64, usize, usize)> = Vec::new();
    for i in 0..n {
        for j in (i + 1)..n {
            if !edges.contains(&(i, j)) {
                candidates.push((dist(points[i], points[j]), i, j));
            }
        }
    }
    candidates.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)).then(a.2.cmp(&b.2)));

    let mut added = 0;
    for (_, i, j) in candidates {
        if added == wanted {
            break;
        }
        if extra_edge_ok(points, radii, &edges, others, (i, j)) {
            edges.push((i, j));
            added += 1;
        }
    }
    edges
}

fn minimum_spanning_tree(points: &[Point]) -> Vec<(usize, usize)> {
    let n = points.len();
    let mut in_tree = vec![false; n];
    in_tree[0] = true;
    let mut edges = Vec::with_capacity(n - 1);
    for _ in 1..n {
        let mut best: Option<(f64, usize, usize)> = None;
        for u in (0..n).filter(|&u| in_tree[u]) {
            for v in (0..n).filter(|&v| !in_tree[v]) {
                let d = dist(points[u], points[v]);
                // Strict `<`, scanning low indices first: ties keep the lower pair.
                if best.is_none_or(|(best_d, _, _)| d < best_d) {
                    best = Some((d, u, v));
                }
            }
        }
        if let Some((_, u, v)) = best {
            in_tree[v] = true;
            edges.push((u.min(v), u.max(v)));
        }
    }
    edges
}

fn extra_edge_ok(
    points: &[Point],
    radii: &[f64],
    edges: &[(usize, usize)],
    others: &[(Point, Point)],
    (i, j): (usize, usize),
) -> bool {
    let (a, b) = (points[i], points[j]);
    if others.iter().any(|&(c, d)| segments_cross(a, b, c, d)) {
        return false;
    }
    for &(p, q) in edges {
        if edges_cross(points, (i, j), (p, q)) {
            return false;
        }
        // Sharper than 20 degrees where the two edges meet at a star.
        for (s, far_new, far_old) in [
            (i, j, if p == i { q } else { p }),
            (j, i, if p == j { q } else { p }),
        ] {
            if (p == s || q == s)
                && angle_at(points[s], points[far_new], points[far_old]) < MIN_EDGE_ANGLE_DEG
            {
                return false;
            }
        }
    }
    // Do not run through a third star.
    (0..points.len())
        .filter(|&k| k != i && k != j)
        .all(|k| point_segment_distance(points[k], a, b) >= radii[k] + EDGE_STAR_CLEARANCE)
}
