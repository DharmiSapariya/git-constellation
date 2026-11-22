//! The last layout step: scale the picture so its constellations cover a good share of the
//! canvas, then nudge it toward the center.

use super::sparse::{
    GATHER_ITERATIONS, GATHER_MIN_GAP, GATHER_STEP, GATHER_TARGET_GAP, SMALL_SKY_MAX_STARS,
};
use super::{
    BALANCE_X_TOLERANCE, BALANCE_Y_TOLERANCE, CANVAS_HEIGHT, CANVAS_WIDTH, FIT_STEPS, Layout,
    MIN_SCALE_DOWN, NUDGE_TARGET_SHARE, bounding_box, centroid, coverage_points, dist,
    sparse::coverage_for, star_area, transformed,
};
use crate::lint;
use crate::model::{Point, SkyModel};

/// The distance between the closest stars of every two constellations.
fn pair_gaps(layout: &Layout) -> Vec<f64> {
    let mut gaps = Vec::new();
    for (a, ca) in layout.constellations.iter().enumerate() {
        for cb in layout.constellations.iter().skip(a + 1) {
            let mut closest = f64::INFINITY;
            for &i in &ca.stars {
                for &j in &cb.stars {
                    closest = closest.min(dist(layout.positions[i], layout.positions[j]));
                }
            }
            if closest.is_finite() {
                gaps.push(closest);
            }
        }
    }
    gaps
}

/// Scale only the constellations of a small sky, around their own center, leaving loose stars
/// where they are (scaling everything around the canvas center pushes a faraway loose star
/// out of the margin and blocks the whole scale-up).
fn scaled_constellations(layout: &Layout, scale: f64) -> Layout {
    let Some(center) = centroid(&coverage_points(layout)) else {
        return layout.clone();
    };
    let map = |p: Point| {
        (
            center.0 + (p.0 - center.0) * scale,
            center.1 + (p.1 - center.1) * scale,
        )
    };
    let mut next = layout.clone();
    for placed in &mut next.constellations {
        for &i in &placed.stars {
            next.positions[i] = map(layout.positions[i]);
        }
        placed.anchor = map(placed.anchor);
        placed.base_radius *= scale;
        placed.radius *= scale;
    }
    next.scale = layout.scale * scale;
    next
}

/// Move every constellation of a small sky a little toward the middle of the others, as a rigid
/// group, while the geometry rules hold, until the most distant pair is close enough.
/// Constellations never come closer than `GATHER_MIN_GAP`.
fn gather(model: &SkyModel, layout: Layout) -> Layout {
    let mut current = layout;
    if model.stars.len() > SMALL_SKY_MAX_STARS || current.constellations.len() < 2 {
        return current;
    }
    for _ in 0..GATHER_ITERATIONS {
        let gaps = pair_gaps(&current);
        let worst = gaps.iter().copied().fold(0.0, f64::max);
        if worst <= GATHER_TARGET_GAP {
            break;
        }
        let centers: Vec<Point> = current
            .constellations
            .iter()
            .map(|c| {
                let points: Vec<Point> = c.stars.iter().map(|&i| current.positions[i]).collect();
                centroid(&points).unwrap_or(c.anchor)
            })
            .collect();
        let n = centers.len() as f64;
        let middle = (
            centers.iter().map(|c| c.0).sum::<f64>() / n,
            centers.iter().map(|c| c.1).sum::<f64>() / n,
        );
        let mut candidate = current.clone();
        for (placed, center) in candidate.constellations.iter_mut().zip(&centers) {
            let shift = (
                (middle.0 - center.0) * GATHER_STEP,
                (middle.1 - center.1) * GATHER_STEP,
            );
            for &i in &placed.stars {
                let p = &mut candidate.positions[i];
                *p = (p.0 + shift.0, p.1 + shift.1);
            }
            placed.anchor = (placed.anchor.0 + shift.0, placed.anchor.1 + shift.1);
        }
        let closest = pair_gaps(&candidate)
            .into_iter()
            .fold(f64::INFINITY, f64::min);
        if closest >= GATHER_MIN_GAP && fit_ok(model, &candidate) {
            current = candidate;
        } else {
            break;
        }
    }
    current
}

/// A fit move is acceptable when the geometry rules still hold and every star
/// is still inside the padded star area (not just the bare 40 px margin).
pub(super) fn fit_ok(model: &SkyModel, layout: &Layout) -> bool {
    layout.positions.iter().all(|&p| star_area().contains(p))
        && lint::geometry_failures(model, layout).is_empty()
}

/// Scale the layout toward the coverage range, then nudge it toward balance.
/// Every change is re-validated; if a change breaks a rule, a smaller change
/// (or none) is used.
pub(super) fn fit(model: &SkyModel, layout: Layout) -> Layout {
    let mut current = gather(model, layout);
    let (cover_w, cover_h, max_up) = coverage_for(model.stars.len());
    if let Some(bounds) = bounding_box(&coverage_points(&current)) {
        let (w, h) = (bounds.width().max(1.0), bounds.height().max(1.0));
        let too_small = w < cover_w.0 * CANVAS_WIDTH || h < cover_h.0 * CANVAS_HEIGHT;
        let too_big = w > cover_w.1 * CANVAS_WIDTH || h > cover_h.1 * CANVAS_HEIGHT;
        let wanted = if too_small && !too_big {
            Some(
                (cover_w.0 * CANVAS_WIDTH / w)
                    .max(cover_h.0 * CANVAS_HEIGHT / h)
                    .min(max_up),
            )
        } else if too_big && !too_small {
            Some(
                (cover_w.1 * CANVAS_WIDTH / w)
                    .min(cover_h.1 * CANVAS_HEIGHT / h)
                    .max(MIN_SCALE_DOWN),
            )
        } else {
            None
        };
        if let Some(wanted) = wanted {
            for step in 0..FIT_STEPS {
                let share = 1.0 - step as f64 / FIT_STEPS as f64;
                let factor = 1.0 + (wanted - 1.0) * share;
                let candidate = if model.stars.len() <= SMALL_SKY_MAX_STARS {
                    scaled_constellations(&current, factor)
                } else {
                    transformed(&current, factor, (0.0, 0.0))
                };
                if fit_ok(model, &candidate) {
                    current = candidate;
                    break;
                }
            }
        }
    }

    if let Some(c) = centroid(&current.positions) {
        let pull = |offset: f64, tolerance: f64| {
            let excess = offset.abs() - NUDGE_TARGET_SHARE * tolerance;
            if excess > 0.0 {
                -offset.signum() * excess
            } else {
                0.0
            }
        };
        let want = (
            pull(c.0 - CANVAS_WIDTH / 2.0, BALANCE_X_TOLERANCE * CANVAS_WIDTH),
            pull(
                c.1 - CANVAS_HEIGHT / 2.0,
                BALANCE_Y_TOLERANCE * CANVAS_HEIGHT,
            ),
        );
        for share in [1.0, 0.75, 0.5, 0.25] {
            let candidate = transformed(&current, 1.0, (want.0 * share, want.1 * share));
            if fit_ok(model, &candidate) {
                current = candidate;
                break;
            }
        }
    }
    current
}
