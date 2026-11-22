//! Composition of small skies: are the constellations close enough to feel like one picture?

use super::{Check, check};
use crate::layout::{CANVAS_HEIGHT, CANVAS_WIDTH, Layout, coverage_points, dist};
use crate::model::{Point, SkyModel};

/// Skies with this many stars or fewer are "sparse": the targets below apply to them.
pub const SPARSE_MAX_STARS: usize = 12;
/// In a sparse sky with at most this many constellations, every two constellations should have
/// stars at most `SPARSE_MAX_GAP` apart (their closest pair).
pub const SPARSE_MAX_CONSTELLATIONS: usize = 3;
pub const SPARSE_MAX_GAP: f64 = 300.0;
/// The box around a sparse sky's constellation stars should cover at least this share of the width.
pub const SPARSE_MIN_WIDTH_SHARE: f64 = 0.45;

/// Numbers that describe how a sky is composed.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Composition {
    pub stars: usize,
    pub constellations: usize,
    /// The largest, over all pairs of constellations, of the distance between their closest stars.
    pub worst_gap: Option<f64>,
    /// Width of the box around the constellation stars, as a share of the canvas width.
    pub width_share: Option<f64>,
    /// Height of that box as a share of the canvas height.
    pub height_share: Option<f64>,
    /// How far the star centroid is from the canvas center, as shares of width and height.
    pub offset: (f64, f64),
}

fn spread(points: &[Point], value: fn(&Point) -> f64) -> f64 {
    let high = points.iter().map(value).fold(f64::MIN, f64::max);
    let low = points.iter().map(value).fold(f64::MAX, f64::min);
    high - low
}

/// Measure a sky's composition.
pub fn measure(model: &SkyModel, layout: &Layout) -> Composition {
    let mut worst_gap: Option<f64> = None;
    for (a, ca) in layout.constellations.iter().enumerate() {
        for cb in layout.constellations.iter().skip(a + 1) {
            let mut closest = f64::INFINITY;
            for &i in &ca.stars {
                for &j in &cb.stars {
                    closest = closest.min(dist(layout.positions[i], layout.positions[j]));
                }
            }
            if closest.is_finite() {
                worst_gap = Some(worst_gap.map_or(closest, |w| w.max(closest)));
            }
        }
    }
    let points = coverage_points(layout);
    let (width_share, height_share) = if points.len() >= 2 {
        (
            Some(spread(&points, |p| p.0) / CANVAS_WIDTH),
            Some(spread(&points, |p| p.1) / CANVAS_HEIGHT),
        )
    } else {
        (None, None)
    };
    let all = &layout.positions;
    let offset = if all.is_empty() {
        (0.0, 0.0)
    } else {
        let n = all.len() as f64;
        let (cx, cy) = (
            all.iter().map(|p| p.0).sum::<f64>() / n,
            all.iter().map(|p| p.1).sum::<f64>() / n,
        );
        (
            (cx - CANVAS_WIDTH / 2.0).abs() / CANVAS_WIDTH,
            (cy - CANVAS_HEIGHT / 2.0).abs() / CANVAS_HEIGHT,
        )
    };
    Composition {
        stars: model.stars.len(),
        constellations: layout.constellations.len(),
        worst_gap,
        width_share,
        height_share,
        offset,
    }
}

/// The two soft composition checks. They only judge sparse skies; for bigger skies they say
/// "not applicable" and pass.
pub fn checks(model: &SkyModel, layout: &Layout) -> Vec<Check> {
    let c = measure(model, layout);
    let sparse = c.stars <= SPARSE_MAX_STARS && c.constellations >= 1;
    let gap_name = "sparse sky: constellations close together";
    let width_name = "sparse sky: width covered";
    if !sparse {
        let note = format!("{} stars: not a sparse sky", c.stars);
        return vec![
            check(gap_name, true, note.clone()),
            check(width_name, true, note),
        ];
    }
    let gap = match c.worst_gap {
        Some(gap) if c.constellations <= SPARSE_MAX_CONSTELLATIONS => check(
            gap_name,
            gap <= SPARSE_MAX_GAP,
            format!("the most distant pair is {gap:.0}px apart (want at most {SPARSE_MAX_GAP:.0})"),
        ),
        _ => check(
            gap_name,
            true,
            "one constellation, or more than three".to_string(),
        ),
    };
    let width = match c.width_share {
        // One group cannot be spread wide: a 5-star constellation 45% of the canvas wide would
        // need 135 px lines. The width target is about how several groups fill the canvas.
        _ if c.constellations < 2 => check(
            width_name,
            true,
            "one constellation: the width target does not apply".to_string(),
        ),
        Some(share) => check(
            width_name,
            share >= SPARSE_MIN_WIDTH_SHARE,
            format!(
                "{:.0}% of the width (want at least {:.0}%)",
                share * 100.0,
                SPARSE_MIN_WIDTH_SHARE * 100.0
            ),
        ),
        None => check(width_name, true, "fewer than two stars".to_string()),
    };
    vec![gap, width]
}
