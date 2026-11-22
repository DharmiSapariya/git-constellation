//! Layout engine: turns a SkyModel into positions, lines and labels.
//!
//! Pure and seeded: the same model and options always give the same layout.
//! Steps (Section 5): anchors, grow shapes, relax, connect lines, wanderers,
//! balance and fit, labels. Nothing here draws a pixel.

pub mod anchors;
pub mod debug;
pub mod geometry;
pub mod labels;
pub mod lines;
pub mod shape;
pub mod sparse;
pub mod text;

mod fitting;

use serde::{Deserialize, Serialize};

pub use geometry::*;

use crate::SkyError;
use crate::config::Options;
use crate::lint;
use crate::model::{LabelMode, Point, SkyModel};
use anchors::SeededRng;
use fitting::fit;
use labels::Label;
use shape::Placement;

// ---- Balance and coverage (2.7, 5.6) ----
/// The star centroid may be this share of the width from the center.
pub const BALANCE_X_TOLERANCE: f64 = 0.12;
/// ...and this share of the height.
pub const BALANCE_Y_TOLERANCE: f64 = 0.15;
/// Nudging aims to land the centroid inside this share of the tolerance.
pub const NUDGE_TARGET_SHARE: f64 = 0.1;
/// Allowed width coverage of all stars' bounding box, as a share of the canvas.
pub const COVERAGE_WIDTH: (f64, f64) = (0.55, 0.85);
/// Allowed height coverage.
pub const COVERAGE_HEIGHT: (f64, f64) = (0.45, 0.80);
pub const MAX_SCALE_UP: f64 = 1.25;
pub const MIN_SCALE_DOWN: f64 = 0.85;
/// Intermediate scales tried between 1.0 and the wanted scale.
pub const FIT_STEPS: usize = 6;

/// Wanderers keep this far from any constellation line (5.5).
pub const WANDERER_LINE_CLEARANCE: f64 = 30.0;
/// Among the layouts that pass every hard check, keep the best of this many.
pub const BEST_OF: usize = 6;
/// Independent layout tries (each with its own random streams) before giving up.
pub const LAYOUT_ATTEMPTS: u32 = 24;

/// One constellation after layout.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlacedConstellation {
    /// Indices into `SkyModel::stars` that really made it into the shape.
    pub stars: Vec<usize>,
    /// Pairs of star indices.
    pub edges: Vec<(usize, usize)>,
    pub anchor: Point,
    /// Radius chosen when anchors were placed (after any shrinking).
    pub base_radius: f64,
    /// Radius after the shape had to grow to fit its stars.
    pub radius: f64,
}

/// The finished layout. `positions[i]` belongs to `SkyModel::stars[i]`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Layout {
    pub positions: Vec<Point>,
    /// Parallel to `SkyModel::constellations`.
    pub constellations: Vec<PlacedConstellation>,
    /// Stars with no constellation, including ones that did not fit a shape.
    pub wanderers: Vec<usize>,
    pub labels: Vec<Label>,
    pub regions: Vec<Region>,
    /// Uniform scale applied by the fit step (1.0 means untouched).
    pub scale: f64,
    /// Which independent try produced this layout (0 is the first).
    pub attempt: u32,
    /// Repo labels (and the like) that had no clean spot and were left out.
    pub dropped_labels: usize,
    /// Set when the sky was too crowded and the star cap had to be lowered.
    pub star_cap: Option<usize>,
}

impl Layout {
    /// Copy the result into the model's own position, edge and anchor fields.
    pub fn apply_to(&self, model: &mut SkyModel) {
        for (star, position) in model.stars.iter_mut().zip(&self.positions) {
            star.position = Some(*position);
            star.constellation = None;
        }
        for (ci, placed) in self.constellations.iter().enumerate() {
            for &si in &placed.stars {
                model.stars[si].constellation = Some(ci);
            }
            let target = &mut model.constellations[ci];
            target.stars = placed.stars.clone();
            target.edges = placed.edges.clone();
            target.anchor = Some(placed.anchor);
            target.radius = Some(placed.radius);
        }
        model.wanderers = self.wanderers.clone();
    }
}

/// Lay out a sky. Tries several independent seeds-of-the-seed. Layouts that
/// fail a hard lint check are thrown away; of the clean ones (up to `BEST_OF`),
/// the one with the fewest soft warnings wins, earliest first on a tie.
pub fn compute(model: &SkyModel, options: &Options) -> Result<Layout, SkyError> {
    let mut last = String::new();
    let mut best: Option<(usize, Layout)> = None;
    let mut clean = 0;
    for attempt in 0..LAYOUT_ATTEMPTS {
        match try_layout(model, options, attempt) {
            Ok(layout) => {
                let failures = lint::hard_failures(model, &layout);
                if !failures.is_empty() {
                    last = failures.join("; ");
                    continue;
                }
                let warnings = lint::soft_warning_count(model, &layout);
                clean += 1;
                if best.as_ref().is_none_or(|(w, _)| warnings < *w) {
                    best = Some((warnings, layout));
                }
                if warnings == 0 || clean == BEST_OF {
                    break;
                }
            }
            Err(error) => last = error.to_string(),
        }
    }
    match best {
        Some((_, layout)) => Ok(layout),
        None => Err(SkyError::Layout(format!(
            "crowded: no clean layout in {LAYOUT_ATTEMPTS} tries (last problem: {last})"
        ))),
    }
}

/// One independent layout try (no retries, no scoring). Used by `compute` and by tests.
pub fn try_layout(model: &SkyModel, options: &Options, attempt: u32) -> Result<Layout, SkyError> {
    let regions = reserved_regions(options.labels != LabelMode::Off);
    let radii: Vec<f64> = model.stars.iter().map(|s| star_radius(s.tier)).collect();

    let mut core = if model.is_young() {
        young_layout(model, &regions, attempt)
    } else {
        grown_layout(model, &regions, &radii, attempt)?
    };
    if !model.is_young() {
        core = fit(model, core);
    }
    let (labels, dropped) = labels::place_labels(model, &core, options.labels);
    core.labels = labels;
    core.dropped_labels = dropped;
    Ok(core)
}

/// A gentle centered row for a sky with fewer than three stars.
fn young_layout(model: &SkyModel, regions: &[Region], attempt: u32) -> Layout {
    let n = model.stars.len();
    let mut rng = SeededRng::for_layer(model.seed, "young", 0, attempt);
    let positions = (0..n)
        .map(|i| {
            let x = CANVAS_WIDTH / 2.0 + (i as f64 - (n as f64 - 1.0) / 2.0) * 90.0;
            (x, CANVAS_HEIGHT / 2.0 + 20.0 + rng.range(-14.0, 14.0))
        })
        .collect();
    Layout {
        positions,
        constellations: Vec::new(),
        wanderers: (0..n).collect(),
        labels: Vec::new(),
        regions: regions.to_vec(),
        scale: 1.0,
        attempt,
        dropped_labels: 0,
        star_cap: None,
    }
}

/// Steps 1 to 5: anchors, shapes, relaxation, lines and wanderers.
fn grown_layout(
    model: &SkyModel,
    regions: &[Region],
    radii: &[f64],
    attempt: u32,
) -> Result<Layout, SkyError> {
    let sizes: Vec<usize> = model.constellations.iter().map(|c| c.stars.len()).collect();
    let small_sky =
        model.stars.len() <= anchors::SMALL_SKY_MAX_STARS && (1..=4).contains(&sizes.len());
    let anchors = anchors::place_anchors(&sizes, small_sky, model.seed, attempt, regions)?;
    let mut placement = Placement::new(radii.to_vec(), regions.to_vec());
    let mut wanderers = model.wanderers.clone();
    let mut placed = Vec::new();

    for (ci, constellation) in model.constellations.iter().enumerate() {
        let (anchor, base_radius) = anchors[ci];
        let mut rng = SeededRng::for_layer(model.seed, "shape", ci as u64, attempt);
        let grown = shape::grow(
            &mut placement,
            &constellation.stars,
            anchor,
            base_radius,
            &mut rng,
            if small_sky {
                sparse::SMALL_SKY_STEP_MULT
            } else {
                1.0
            },
        );
        let locked: Vec<usize> = grown
            .placed
            .iter()
            .copied()
            .filter(|&i| model.stars[i].founding)
            .collect();
        shape::relax(&mut placement, &grown.placed, &locked, anchor, grown.radius);
        wanderers.extend(&grown.dropped);
        placed.push(PlacedConstellation {
            stars: grown.placed,
            edges: Vec::new(),
            anchor,
            base_radius,
            radius: grown.radius,
        });
    }

    let mut segments = Vec::new();
    for constellation in &mut placed {
        let points: Vec<Point> = constellation
            .stars
            .iter()
            .map(|&i| placement.at(i))
            .collect();
        let local_radii: Vec<f64> = constellation.stars.iter().map(|&i| radii[i]).collect();
        for (a, b) in lines::build_edges(&points, &local_radii, &segments) {
            constellation
                .edges
                .push((constellation.stars[a], constellation.stars[b]));
            segments.push((points[a], points[b]));
        }
    }

    wanderers.sort_unstable();
    wanderers.dedup();
    let all_edges: Vec<(usize, usize)> = placed.iter().flat_map(|c| c.edges.clone()).collect();
    let mut rng = SeededRng::for_layer(model.seed, "wanderers", 0, attempt);
    shape::scatter_wanderers(&mut placement, &wanderers, &all_edges, &mut rng)?;

    let positions = (0..model.stars.len()).map(|i| placement.at(i)).collect();
    Ok(Layout {
        positions,
        constellations: placed,
        wanderers,
        labels: Vec::new(),
        regions: regions.to_vec(),
        scale: 1.0,
        attempt,
        dropped_labels: 0,
        star_cap: None,
    })
}

// ---------------------------------------------------------------------------
// Step 6: balance and fit
// ---------------------------------------------------------------------------

fn bounding_box(points: &[Point]) -> Option<Rect> {
    let first = points.first()?;
    let mut bounds = Rect {
        x0: first.0,
        y0: first.1,
        x1: first.0,
        y1: first.1,
    };
    for p in points {
        bounds.x0 = bounds.x0.min(p.0);
        bounds.y0 = bounds.y0.min(p.1);
        bounds.x1 = bounds.x1.max(p.0);
        bounds.y1 = bounds.y1.max(p.1);
    }
    Some(bounds)
}

/// The stars that count for coverage (2.7): those in constellations, or every
/// star if there are no constellations.
pub fn coverage_points(layout: &Layout) -> Vec<Point> {
    let members: Vec<Point> = layout
        .constellations
        .iter()
        .flat_map(|c| c.stars.iter().map(|&i| layout.positions[i]))
        .collect();
    if members.is_empty() {
        layout.positions.clone()
    } else {
        members
    }
}

pub fn centroid(points: &[Point]) -> Option<Point> {
    if points.is_empty() {
        return None;
    }
    let n = points.len() as f64;
    Some((
        points.iter().map(|p| p.0).sum::<f64>() / n,
        points.iter().map(|p| p.1).sum::<f64>() / n,
    ))
}

/// Scale about the canvas center and/or shift the whole layout.
fn transformed(layout: &Layout, scale: f64, shift: Point) -> Layout {
    let center = (CANVAS_WIDTH / 2.0, CANVAS_HEIGHT / 2.0);
    let map = |p: Point| {
        (
            center.0 + (p.0 - center.0) * scale + shift.0,
            center.1 + (p.1 - center.1) * scale + shift.1,
        )
    };
    let mut next = layout.clone();
    next.positions = layout.positions.iter().map(|&p| map(p)).collect();
    for placed in &mut next.constellations {
        placed.anchor = map(placed.anchor);
        placed.base_radius *= scale;
        placed.radius *= scale;
    }
    next.scale = layout.scale * scale;
    next
}
