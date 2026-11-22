//! Constellation lines: faint threads between stars, stopping short of every star and ring.

use crate::layout::Layout;
use crate::layout::geometry::{FOUNDING_RING, dist, star_radius};
use crate::model::{Point, SkyModel, Star, Tier};
use crate::svg::{self, SvgBuilder, escape, fmt};
use crate::theme::Theme;

/// A line stops this many star radii from a star's center, and a bit more at the founding star,
/// whose ring sits at `FOUNDING_RING` radii.
pub const LINE_GAP_MULT: f64 = 2.4;
pub const LINE_GAP_FOUNDING_MULT: f64 = 3.3;
const _: () = assert!(
    LINE_GAP_FOUNDING_MULT > FOUNDING_RING,
    "lines must stop outside the ring"
);
/// The soft glow under each line: width and opacity.
pub const LINE_GLOW_WIDTH: f64 = 3.0;
pub const LINE_GLOW_OPACITY: f64 = 0.06;
/// The line itself.
pub const LINE_WIDTH: f64 = 1.1;
/// Lines touching a tier 4 or 5 star read a little stronger than the theme's usual line.
pub const LINE_SPINE_BOOST: f64 = 0.12;
/// Lines to a dormant star are dashed and quiet.
pub const LINE_DORMANT_OPACITY: f64 = 0.25;
pub const LINE_DORMANT_DASH: &str = "2 5";
/// The smallest tier that makes a line a "spine" line.
pub const SPINE_MIN_TIER: Tier = Tier::Four;

/// How one line is stroked.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LineStyle {
    pub opacity: f64,
    pub dash: Option<&'static str>,
}

/// The stroke of the line between two stars. Dormant wins over everything; then
/// a bright end makes the line a spine; otherwise the theme's own line opacity.
pub fn line_style(theme: &Theme, a: &Star, b: &Star) -> LineStyle {
    if a.dormant || b.dormant {
        LineStyle {
            opacity: LINE_DORMANT_OPACITY,
            dash: Some(LINE_DORMANT_DASH),
        }
    } else if a.tier >= SPINE_MIN_TIER || b.tier >= SPINE_MIN_TIER {
        LineStyle {
            opacity: (theme.line.1 + LINE_SPINE_BOOST).min(1.0),
            dash: None,
        }
    } else {
        LineStyle {
            opacity: theme.line.1,
            dash: None,
        }
    }
}

/// How far from its center a star's line stops.
pub fn line_gap(star: &Star) -> f64 {
    let mult = if star.founding {
        LINE_GAP_FOUNDING_MULT
    } else {
        LINE_GAP_MULT
    };
    mult * star_radius(star.tier)
}

/// The visible part of the line from `a` to `b`: shortened by a gap at each end,
/// or `None` when the two gaps leave nothing.
pub fn trimmed_segment(a: Point, b: Point, gap_a: f64, gap_b: f64) -> Option<(Point, Point)> {
    let length = dist(a, b);
    if length <= gap_a + gap_b || length <= 0.0 {
        return None;
    }
    let unit = ((b.0 - a.0) / length, (b.1 - a.1) / length);
    Some((
        (a.0 + unit.0 * gap_a, a.1 + unit.1 * gap_a),
        (b.0 - unit.0 * gap_b, b.1 - unit.1 * gap_b),
    ))
}

fn segment_path(from: Point, to: Point) -> String {
    format!(
        "M {} {} L {} {}",
        fmt(from.0),
        fmt(from.1),
        fmt(to.0),
        fmt(to.1)
    )
}

/// Draw the `lines` layer, one group per constellation. Returns how many edges
/// were too short to draw (the linter reports it).
pub fn draw_lines(b: &mut SvgBuilder, theme: &Theme, model: &SkyModel, layout: &Layout) -> usize {
    let mut skipped = 0;
    b.group(&[("id", "lines".to_string())], |g| {
        for (ci, placed) in layout.constellations.iter().enumerate() {
            let name = model.constellations.get(ci).map_or("", |c| c.name.as_str());
            let mut inner = svg::tag("title", &[], Some(&escape(name)));
            for &(ai, bi) in &placed.edges {
                let (Some(a), Some(z)) = (model.stars.get(ai), model.stars.get(bi)) else {
                    continue;
                };
                let (Some(&pa), Some(&pz)) = (layout.positions.get(ai), layout.positions.get(bi))
                else {
                    continue;
                };
                let Some((from, to)) = trimmed_segment(pa, pz, line_gap(a), line_gap(z)) else {
                    skipped += 1;
                    continue;
                };
                let d = segment_path(from, to);
                inner.push_str(&svg::path(
                    &d,
                    &[
                        ("fill", "none".to_string()),
                        ("stroke", theme.line.0.to_string()),
                        ("stroke-width", fmt(LINE_GLOW_WIDTH)),
                        ("stroke-opacity", fmt(LINE_GLOW_OPACITY)),
                        ("stroke-linecap", "round".to_string()),
                    ],
                ));
                let style = line_style(theme, a, z);
                let mut attrs = vec![
                    ("fill", "none".to_string()),
                    ("stroke", theme.line.0.to_string()),
                    ("stroke-width", fmt(LINE_WIDTH)),
                    ("stroke-opacity", fmt(style.opacity)),
                    ("stroke-linecap", "round".to_string()),
                ];
                if let Some(dash) = style.dash {
                    attrs.push(("stroke-dasharray", dash.to_string()));
                }
                inner.push_str(&svg::path(&d, &attrs));
            }
            g.add(&svg::tag(
                "g",
                &[("id", format!("constellation-{ci}"))],
                Some(&inner),
            ));
        }
    });
    skipped
}
