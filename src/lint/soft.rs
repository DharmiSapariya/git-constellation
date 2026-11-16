//! Soft lint checks: warnings with numbers (Section 10.3). They never fail a build.

use super::{Check, check};
use crate::layout::labels::WIDE_HALO;
use crate::layout::{
    CANVAS_HEIGHT, CANVAS_WIDTH, COVERAGE_HEIGHT, COVERAGE_WIDTH, Layout, coverage_points, dist,
    star_radius,
};
use crate::model::{LanguageKey, Point, SkyModel};

/// Average nearest-neighbor distance must stay between these.
pub const NEAREST_NEIGHBOR_RANGE: (f64, f64) = (38.0, 150.0);
/// At most this many distinct star hues.
pub const MAX_HUES: usize = 7;
/// The largest star-free rectangle may cover at most this share of the canvas.
pub const MAX_EMPTY_SHARE: f64 = 0.30;
/// Wanted gap between stars of different constellations (2.7).
pub const CONSTELLATION_GAP: f64 = 60.0;
/// Grid cell size for the empty-rectangle search.
const EMPTY_CELL: f64 = 10.0;

fn radii(model: &SkyModel) -> Vec<f64> {
    model.stars.iter().map(|s| star_radius(s.tier)).collect()
}

/// All soft checks, in report order.
pub fn checks(model: &SkyModel, layout: &Layout) -> Vec<Check> {
    vec![
        coverage(layout),
        nearest_neighbor(layout),
        hues(model),
        empty_rect(model, layout),
        constellation_gap(layout),
        labels_left_out(layout),
    ]
}

/// Width and height of the box around some points (none for fewer than two).
fn extent(points: &[Point]) -> Option<(f64, f64)> {
    if points.len() < 2 {
        return None;
    }
    let spread = |value: fn(&Point) -> f64| {
        let high = points.iter().map(value).fold(f64::MIN, f64::max);
        let low = points.iter().map(value).fold(f64::MAX, f64::min);
        high - low
    };
    Some((spread(|p| p.0), spread(|p| p.1)))
}

fn coverage(layout: &Layout) -> Check {
    let name = "coverage of the constellations' bounding box";
    let Some((w, h)) = extent(&coverage_points(layout)) else {
        return check(name, true, "fewer than two stars, not measured".to_string());
    };
    let (wf, hf) = (w / CANVAS_WIDTH, h / CANVAS_HEIGHT);
    let ok = (COVERAGE_WIDTH.0..=COVERAGE_WIDTH.1).contains(&wf)
        && (COVERAGE_HEIGHT.0..=COVERAGE_HEIGHT.1).contains(&hf);
    let detail = format!(
        "{:.0}% of width (want {:.0}-{:.0}%), {:.0}% of height (want {:.0}-{:.0}%)",
        wf * 100.0,
        COVERAGE_WIDTH.0 * 100.0,
        COVERAGE_WIDTH.1 * 100.0,
        hf * 100.0,
        COVERAGE_HEIGHT.0 * 100.0,
        COVERAGE_HEIGHT.1 * 100.0
    );
    check(name, ok, detail)
}

fn nearest_neighbor(layout: &Layout) -> Check {
    let name = "average nearest-neighbor distance";
    let p = &layout.positions;
    if p.len() < 2 {
        return check(name, true, "fewer than two stars".to_string());
    }
    let nearest = |i: usize| {
        (0..p.len())
            .filter(|&j| j != i)
            .map(|j| dist(p[i], p[j]))
            .fold(f64::INFINITY, f64::min)
    };
    let average = (0..p.len()).map(nearest).sum::<f64>() / p.len() as f64;
    let (low, high) = NEAREST_NEIGHBOR_RANGE;
    check(
        name,
        (low..=high).contains(&average),
        format!("{average:.1}px (want {low:.0}-{high:.0})"),
    )
}

fn hues(model: &SkyModel) -> Check {
    let mut keys: Vec<LanguageKey> = model.stars.iter().map(|s| s.language).collect();
    keys.sort();
    keys.dedup();
    check(
        "distinct star hues",
        keys.len() <= MAX_HUES,
        format!("{} (want at most {MAX_HUES})", keys.len()),
    )
}

/// Largest rectangle with no star glow in it, as a share of the canvas.
pub fn largest_empty_share(model: &SkyModel, layout: &Layout) -> f64 {
    let (cols, rows) = (
        (CANVAS_WIDTH / EMPTY_CELL) as usize,
        (CANVAS_HEIGHT / EMPTY_CELL) as usize,
    );
    let r = radii(model);
    let free = |row: usize, col: usize| {
        let c = (
            (col as f64 + 0.5) * EMPTY_CELL,
            (row as f64 + 0.5) * EMPTY_CELL,
        );
        layout
            .positions
            .iter()
            .enumerate()
            .all(|(i, &p)| dist(c, p) > WIDE_HALO * r[i])
    };
    // Classic "largest rectangle in a histogram", one row at a time.
    let mut heights = vec![0usize; cols];
    let mut best = 0;
    for row in 0..rows {
        for (col, height) in heights.iter_mut().enumerate() {
            *height = if free(row, col) { *height + 1 } else { 0 };
        }
        let mut stack: Vec<usize> = Vec::new();
        for col in 0..=cols {
            let h = if col < cols { heights[col] } else { 0 };
            while let Some(&top) = stack.last() {
                if heights[top] <= h {
                    break;
                }
                stack.pop();
                let left = stack.last().map_or(0, |&l| l + 1);
                best = best.max(heights[top] * (col - left));
            }
            stack.push(col);
        }
    }
    best as f64 / (cols * rows) as f64
}

fn empty_rect(model: &SkyModel, layout: &Layout) -> Check {
    let share = largest_empty_share(model, layout);
    check(
        "largest star-free rectangle (stars only, dust comes later)",
        share < MAX_EMPTY_SHARE,
        format!(
            "{:.0}% of the canvas (want under {:.0}%)",
            share * 100.0,
            MAX_EMPTY_SHARE * 100.0
        ),
    )
}

fn constellation_gap(layout: &Layout) -> Check {
    let name = "gap between constellations";
    let mut closest = f64::INFINITY;
    for (a, ca) in layout.constellations.iter().enumerate() {
        for cb in layout.constellations.iter().skip(a + 1) {
            for &i in &ca.stars {
                for &j in &cb.stars {
                    closest = closest.min(dist(layout.positions[i], layout.positions[j]));
                }
            }
        }
    }
    if closest.is_finite() {
        let detail = format!(
            "closest stars of two constellations are {closest:.0}px apart (want {CONSTELLATION_GAP:.0}+)"
        );
        check(name, closest >= CONSTELLATION_GAP, detail)
    } else {
        check(name, true, "fewer than two constellations".to_string())
    }
}

fn labels_left_out(layout: &Layout) -> Check {
    check(
        "labels left out for lack of room",
        layout.dropped_labels == 0,
        format!("{} left out", layout.dropped_labels),
    )
}
