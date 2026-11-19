//! The sky linter: quality checks that stop ugly output.
//!
//! Hard checks must pass. Soft checks report warnings with numbers. Checks
//! that need drawn output (contrast, file size, luminance) arrive with the art.

mod report;
mod sky;
mod soft;

pub use report::{Check, LintReport};
pub use sky::{
    DrawnChecks, MAX_SVG_BYTES, MIN_TEXT_CONTRAST, SOFT_SVG_BYTES, VISIBILITY_BIG_TIER,
    VISIBILITY_MARGIN, VISIBILITY_RING, Visibility, drawn_checks, has_non_finite_number,
    star_visibility, svg_hard_checks, text_contrast_check,
};
pub use soft::{
    CLIP_LEVEL, CONSTELLATION_GAP, DARK_MEAN_LUMA, LIGHT_MEAN_LUMA, MAX_CLIPPED_DARK,
    MAX_EMPTY_SHARE, MAX_HUES, NEAREST_NEIGHBOR_RANGE, PixelStats, largest_empty_share,
    pixel_checks, pixel_stats,
};

use crate::layout::labels::{LabelKind, Obstacles, name_belongs};
use crate::layout::lines::{edges_cross, point_segment_distance};
use crate::layout::{
    BALANCE_X_TOLERANCE, BALANCE_Y_TOLERANCE, CANVAS_HEIGHT, CANVAS_WIDTH, Layout, Rect,
    WANDERER_LINE_CLEARANCE, centroid, dist, min_dist, safe_rect, star_radius,
};
use crate::model::{Point, SkyModel};

/// Tolerance for float noise in distance checks.
const SLACK: f64 = 1e-6;

fn check(name: &'static str, passed: bool, detail: String) -> Check {
    Check {
        name,
        passed,
        detail,
    }
}

fn radii(model: &SkyModel) -> Vec<f64> {
    model.stars.iter().map(|s| star_radius(s.tier)).collect()
}

// ---------------------------------------------------------------------------
// Hard checks
// ---------------------------------------------------------------------------

fn star_spacing(model: &SkyModel, layout: &Layout) -> Check {
    let r = radii(model);
    let mut worst: Option<(f64, usize, usize, f64, f64)> = None; // slack, i, j, distance, needed
    for i in 0..layout.positions.len() {
        for j in (i + 1)..layout.positions.len() {
            let (d, need) = (
                dist(layout.positions[i], layout.positions[j]),
                min_dist(r[i], r[j]),
            );
            if worst.is_none_or(|w| d - need < w.0) {
                worst = Some((d - need, i, j, d, need));
            }
        }
    }
    match worst {
        Some((slack, i, j, d, need)) => check(
            "star spacing",
            slack >= -SLACK,
            format!(
                "closest pair is {} and {}: {d:.1}px apart, needs {need:.1}px",
                model.stars[i].repo, model.stars[j].repo
            ),
        ),
        None => check("star spacing", true, "fewer than two stars".to_string()),
    }
}

fn star_bounds(model: &SkyModel, layout: &Layout) -> Check {
    let r = radii(model);
    let safe = safe_rect();
    for (i, &p) in layout.positions.iter().enumerate() {
        let in_region = layout
            .regions
            .iter()
            .any(|region| region.blocks_point(p, r[i]));
        if !safe.contains(p) || in_region {
            return check(
                "stars inside the margin and outside reserved areas",
                false,
                format!("{} at ({:.0}, {:.0})", model.stars[i].repo, p.0, p.1),
            );
        }
    }
    check(
        "stars inside the margin and outside reserved areas",
        true,
        format!("{} stars checked", layout.positions.len()),
    )
}

fn crossings(layout: &Layout) -> Check {
    let edges: Vec<(usize, usize)> = layout
        .constellations
        .iter()
        .flat_map(|c| c.edges.clone())
        .collect();
    let mut bad = 0;
    for a in 0..edges.len() {
        for b in (a + 1)..edges.len() {
            if edges_cross(&layout.positions, edges[a], edges[b]) {
                bad += 1;
            }
        }
    }
    check(
        "line crossings",
        bad == 0,
        format!("{bad} crossings among {} lines", edges.len()),
    )
}

fn wanderer_clearance(layout: &Layout) -> Check {
    let mut closest = f64::INFINITY;
    for &w in &layout.wanderers {
        for c in &layout.constellations {
            for &(a, b) in &c.edges {
                closest = closest.min(point_segment_distance(
                    layout.positions[w],
                    layout.positions[a],
                    layout.positions[b],
                ));
            }
        }
    }
    let detail = if closest.is_finite() {
        format!("closest loose star is {closest:.1}px from a line")
    } else {
        "no loose stars near lines".to_string()
    };
    check(
        "loose stars clear of lines",
        closest >= WANDERER_LINE_CLEARANCE - SLACK,
        detail,
    )
}

fn connected(layout: &Layout) -> Check {
    for (ci, c) in layout.constellations.iter().enumerate() {
        // Flood fill from the first star along the edges.
        let mut reached: Vec<usize> = c.stars.iter().take(1).copied().collect();
        let mut grew = true;
        while grew {
            grew = false;
            for &(a, b) in &c.edges {
                for (from, to) in [(a, b), (b, a)] {
                    if reached.contains(&from) && !reached.contains(&to) {
                        reached.push(to);
                        grew = true;
                    }
                }
            }
        }
        if reached.len() != c.stars.len() {
            return check(
                "constellation lines connect every star",
                false,
                format!("constellation {ci} is split"),
            );
        }
    }
    check(
        "constellation lines connect every star",
        true,
        format!("{} constellations", layout.constellations.len()),
    )
}

fn label_overlaps(model: &SkyModel, layout: &Layout) -> Check {
    let obstacles = Obstacles::from_layout(model, layout);
    let rects: Vec<Rect> = layout.labels.iter().map(|l| l.rect).collect();
    let mut bad = 0;
    for (i, rect) in rects.iter().enumerate() {
        let others: Vec<Rect> = rects
            .iter()
            .enumerate()
            .filter(|(j, _)| *j != i)
            .map(|(_, r)| *r)
            .collect();
        bad += obstacles.conflicts(rect, &others);
    }
    check(
        "labels clear of stars, lines, labels, reserved areas and margin",
        bad == 0,
        format!(
            "{bad} conflicts among {} labels ({} left out)",
            rects.len(),
            layout.dropped_labels
        ),
    )
}

fn name_ownership(layout: &Layout) -> Check {
    let name = "constellation names sit nearer their own stars";
    for label in &layout.labels {
        let LabelKind::Constellation(ci) = label.kind else {
            continue;
        };
        let points = |c: &crate::layout::PlacedConstellation| -> Vec<Point> {
            c.stars.iter().map(|&i| layout.positions[i]).collect()
        };
        let own = points(&layout.constellations[ci]);
        let foreign: Vec<Point> = layout
            .constellations
            .iter()
            .enumerate()
            .filter(|(o, _)| *o != ci)
            .flat_map(|(_, c)| points(c))
            .collect();
        if !name_belongs(&label.rect, &own, &foreign) {
            return check(
                name,
                false,
                format!("the name of constellation {ci} is not clearly nearer its own stars"),
            );
        }
    }
    check(
        name,
        true,
        format!(
            "{} names checked",
            layout
                .labels
                .iter()
                .filter(|l| matches!(l.kind, LabelKind::Constellation(_)))
                .count()
        ),
    )
}

fn balance(layout: &Layout) -> Check {
    match centroid(&layout.positions) {
        None => check("balance", true, "no stars".to_string()),
        Some(c) => {
            let (dx, dy) = (
                (c.0 - CANVAS_WIDTH / 2.0).abs() / CANVAS_WIDTH,
                (c.1 - CANVAS_HEIGHT / 2.0).abs() / CANVAS_HEIGHT,
            );
            check(
                "balance (star centroid near the center)",
                dx <= BALANCE_X_TOLERANCE + SLACK && dy <= BALANCE_Y_TOLERANCE + SLACK,
                format!(
                    "off center by {:.1}% of width (max {:.0}%), {:.1}% of height (max {:.0}%)",
                    dx * 100.0,
                    BALANCE_X_TOLERANCE * 100.0,
                    dy * 100.0,
                    BALANCE_Y_TOLERANCE * 100.0
                ),
            )
        }
    }
}

fn finite(layout: &Layout) -> Check {
    let points = layout
        .positions
        .iter()
        .chain(layout.constellations.iter().map(|c| &c.anchor));
    let radii_ok = layout
        .constellations
        .iter()
        .all(|c| c.radius.is_finite() && c.base_radius.is_finite());
    let labels_ok = layout.labels.iter().all(|l| {
        [l.rect.x0, l.rect.y0, l.rect.x1, l.rect.y1]
            .iter()
            .all(|v| v.is_finite())
    });
    let points_ok = points
        .into_iter()
        .all(|p: &Point| p.0.is_finite() && p.1.is_finite());
    check(
        "no NaN or infinite numbers",
        radii_ok && labels_ok && points_ok && layout.scale.is_finite(),
        "positions, anchors, radii, labels".to_string(),
    )
}

fn all_placed(model: &SkyModel, layout: &Layout) -> Check {
    let ok = layout.positions.len() == model.stars.len();
    check(
        "every star has a position",
        ok,
        format!(
            "{} stars, {} positions",
            model.stars.len(),
            layout.positions.len()
        ),
    )
}

/// The checks about where stars and lines are (no labels). Cheap enough to run
/// while the layout is still being fitted.
fn geometry_checks(model: &SkyModel, layout: &Layout) -> Vec<Check> {
    let placed = all_placed(model, layout);
    if !placed.passed {
        // Without a position for every star, the other checks cannot even run.
        return vec![placed];
    }
    vec![
        placed,
        star_spacing(model, layout),
        star_bounds(model, layout),
        crossings(layout),
        wanderer_clearance(layout),
        connected(layout),
    ]
}

pub fn hard_checks(model: &SkyModel, layout: &Layout) -> Vec<Check> {
    let mut checks = geometry_checks(model, layout);
    if checks.iter().all(|c| c.passed) || layout.positions.len() == model.stars.len() {
        checks.extend([
            label_overlaps(model, layout),
            name_ownership(layout),
            balance(layout),
            finite(layout),
        ]);
    }
    checks
}

fn failures(checks: Vec<Check>) -> Vec<String> {
    checks
        .into_iter()
        .filter(|c| !c.passed)
        .map(|c| format!("{}: {}", c.name, c.detail))
        .collect()
}

/// Which hard checks fail (empty when the layout is clean).
pub fn hard_failures(model: &SkyModel, layout: &Layout) -> Vec<String> {
    failures(hard_checks(model, layout))
}

/// Which geometry-only hard checks fail.
pub fn geometry_failures(model: &SkyModel, layout: &Layout) -> Vec<String> {
    failures(geometry_checks(model, layout))
}

/// How many soft checks warn (used to pick the nicest of several clean layouts).
pub fn soft_warning_count(model: &SkyModel, layout: &Layout) -> usize {
    if layout.positions.len() != model.stars.len() {
        return 0;
    }
    soft::checks(model, layout)
        .iter()
        .filter(|c| !c.passed)
        .count()
}

pub fn lint(model: &SkyModel, layout: &Layout) -> LintReport {
    LintReport {
        hard: hard_checks(model, layout),
        soft: if layout.positions.len() == model.stars.len() {
            soft::checks(model, layout)
        } else {
            Vec::new() // nothing to measure without a position for every star
        },
        deferred: vec![
            "text contrast 4.5:1 (hard 5)",
            "SVG size cap and animation budget (hard 7)",
            "luminance of the finished picture (soft 3: run lint::pixel_checks on the rendered PNG)",
        ],
    }
}
