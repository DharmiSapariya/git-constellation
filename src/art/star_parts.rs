//! The parts of a star that are not its halo: body and core, flare cross, rings and the paper inking.

use crate::layout::geometry::{FOUNDING_RING, FRESH_RING, Rect, star_radius};
use crate::model::{Point, Star};
use crate::svg::{self, Stop, SvgBuilder, Units, fmt};
use crate::theme::Theme;

use super::stars::{GradientCache, PAPER_SMALL_MAX_TIER, StarPalette, body_gradient, sparkle_path};

/// The hot white core, as a share of the star radius, and its opacity.
pub const CORE_RADIUS_MULT: f64 = 0.38;
pub const CORE_OPACITY: f64 = 0.95;
/// The flare cross of the biggest stars: arm length (units of radius), peak opacity, thickness.
pub const FLARE_LENGTH_MULT: f64 = 5.0;
pub const FLARE_OPACITY: f64 = 0.85;
pub const FLARE_THICKNESS: f64 = 1.3;
/// A flare arm stops this far before a drawn label, and is left out below this half-length.
pub const FLARE_LABEL_GAP: f64 = 3.0;
pub const FLARE_MIN_HALF: f64 = 2.0;
/// Rings: radius in units of the star radius, stroke width, opacity.
pub const RING_FOUNDING_OPACITY: f64 = 0.55;
pub const RING_FRESH_OPACITY: f64 = 0.35;
/// Ring stroke widths: the founding ring is solid, the fresh ring is dotted.
pub const RING_STROKE: f64 = 0.8;
pub const RING_FRESH_STROKE: f64 = 0.7;
pub const RING_FRESH_DASH: &str = "1.2 2.4";
/// A borrowed (forked) star is a hollow ring this thick.
pub const BORROWED_STROKE: f64 = 1.0;
/// Paper theme: the ink outline of a body, and the opacity of its white core.
pub const PAPER_OUTLINE_WIDTH: f64 = 0.8;
/// Paper: outline width of the small stars (tier 1 and 2).
pub const PAPER_SMALL_OUTLINE_WIDTH: f64 = 0.7;
pub const PAPER_OUTLINE_OPACITY: f64 = 0.7;
pub const PAPER_CORE_OPACITY: f64 = 0.6;
/// Paper tier 5 gets two short ink strokes instead of a glowing flare.
pub const PAPER_FLARE_MULT: f64 = 2.6;
pub const PAPER_FLARE_OPACITY: f64 = 0.7;
/// Stroke width of the paper flare: thinner than the body outline so it reads as a glint.
pub const PAPER_FLARE_WIDTH: f64 = 0.6;
/// The smallest tier that is drawn as a four-point sparkle, and the tier that gets a flare.
pub const SPARKLE_MIN_TIER: u8 = 3;
pub const FLARE_TIER: u8 = 5;

/// The one gradient every flare arm shares: fades in and out along its length.
pub(super) fn flare_gradient(b: &mut SvgBuilder) -> String {
    let id = b.new_id("flare");
    b.linear_gradient(
        &id,
        (0.0, 0.0),
        (1.0, 0.0),
        Units::ObjectBoundingBox,
        &[
            Stop::new(0.0, "#ffffff", 0.0),
            Stop::new(0.5, "#ffffff", FLARE_OPACITY),
            Stop::new(1.0, "#ffffff", 0.0),
        ],
    );
    id
}

/// True when some star needs the flare gradient (a calm tier 5 on a dark theme).
pub(super) fn wants_flare(star: &Star) -> bool {
    star.tier.number() == FLARE_TIER && !star.dormant && !star.borrowed
}

fn ring(
    at: Point,
    radius: f64,
    color: &str,
    opacity: f64,
    stroke: f64,
    dash: Option<&str>,
) -> String {
    let mut attrs = vec![
        ("fill", "none".to_string()),
        ("stroke", color.to_string()),
        ("stroke-width", fmt(stroke)),
        ("stroke-opacity", fmt(opacity)),
    ];
    if let Some(dash) = dash {
        attrs.push(("stroke-dasharray", dash.to_string()));
    }
    svg::circle(at.0, at.1, radius, &attrs)
}

/// The body of one star (the part that dormant stars dim), plus its core.
pub(super) fn body_markup(
    b: &mut SvgBuilder,
    cache: &mut GradientCache,
    theme: &Theme,
    star: &Star,
    at: Point,
    rotation: f64,
    palette: &StarPalette,
) -> String {
    let (x, y) = at;
    let r = star_radius(star.tier);
    let mut out = String::new();
    if star.borrowed {
        out.push_str(&svg::circle(
            x,
            y,
            r,
            &[
                ("fill", "none".to_string()),
                ("stroke", palette.body.clone()),
                ("stroke-width", fmt(BORROWED_STROKE)),
            ],
        ));
        out.push_str(&svg::circle(
            x,
            y,
            CORE_RADIUS_MULT * r,
            &[("fill", palette.body.clone())],
        ));
        return out;
    }
    let outline_width = if star.tier.number() <= PAPER_SMALL_MAX_TIER {
        PAPER_SMALL_OUTLINE_WIDTH
    } else {
        PAPER_OUTLINE_WIDTH
    };
    let (fill, outline) = if theme.is_dark {
        let id = body_gradient(b, cache, &palette.glow);
        (format!("url(#{id})"), Vec::new())
    } else {
        (
            palette.body.clone(),
            vec![
                ("stroke", theme.ink.to_string()),
                ("stroke-width", fmt(outline_width)),
                ("stroke-opacity", fmt(PAPER_OUTLINE_OPACITY)),
                ("stroke-linejoin", "round".to_string()),
            ],
        )
    };
    let mut attrs: Vec<(&str, String)> = vec![("fill", fill)];
    attrs.extend(outline);
    if star.tier.number() >= SPARKLE_MIN_TIER {
        attrs.push((
            "transform",
            format!("translate({} {}) rotate({})", fmt(x), fmt(y), fmt(rotation)),
        ));
        out.push_str(&svg::path(&sparkle_path(r), &attrs));
    } else {
        out.push_str(&svg::circle(x, y, r, &attrs));
    }
    let core_opacity = if theme.is_dark {
        CORE_OPACITY
    } else {
        PAPER_CORE_OPACITY
    };
    out.push_str(&svg::circle(
        x,
        y,
        CORE_RADIUS_MULT * r,
        &[
            ("fill", "#ffffff".to_string()),
            ("opacity", fmt(core_opacity)),
        ],
    ));
    out
}

/// How far the two flare arms of a star at `at` may reach on each side (horizontal, vertical),
/// so that they stop `FLARE_LABEL_GAP` before any label box instead of touching it.
/// `half` is the full half-length; `thickness` is the arm's thickness.
pub fn flare_half_lengths(at: Point, half: f64, thickness: f64, labels: &[Rect]) -> (f64, f64) {
    let (x, y) = at;
    let t = thickness / 2.0;
    let (mut horizontal, mut vertical) = (half, half);
    for label in labels {
        let box_ = label.inflate(FLARE_LABEL_GAP);
        // The horizontal arm lies on the line `y`; a box it can run into spans that line.
        if box_.y0 <= y + t && box_.y1 >= y - t {
            horizontal = if box_.x0 > x {
                horizontal.min(box_.x0 - x)
            } else if box_.x1 < x {
                horizontal.min(x - box_.x1)
            } else {
                0.0
            };
        }
        if box_.x0 <= x + t && box_.x1 >= x - t {
            vertical = if box_.y0 > y {
                vertical.min(box_.y0 - y)
            } else if box_.y1 < y {
                vertical.min(y - box_.y1)
            } else {
                0.0
            };
        }
    }
    (horizontal.max(0.0), vertical.max(0.0))
}

/// The flare of a calm tier 5 star: two glowing arms on dark themes (`flare_id` is their
/// shared gradient), two short ink strokes on paper. Arms stop before any drawn label.
/// Empty for every other star.
pub fn flare_markup(
    theme: &Theme,
    star: &Star,
    at: Point,
    flare_id: Option<&str>,
    labels: &[Rect],
) -> String {
    let mut inner = String::new();
    if !wants_flare(star) {
        return inner;
    }
    let (x, y) = at;
    let r = star_radius(star.tier);
    if let Some(id) = flare_id {
        let (hx, hy) = flare_half_lengths(at, FLARE_LENGTH_MULT * r / 2.0, FLARE_THICKNESS, labels);
        for (turn, half) in [(false, hx), (true, hy)] {
            if half < FLARE_MIN_HALF {
                continue;
            }
            let mut attrs = vec![("fill", format!("url(#{id})"))];
            if turn {
                attrs.push(("transform", format!("rotate(90 {} {})", fmt(x), fmt(y))));
            }
            inner.push_str(&svg::rect(
                x - half,
                y - FLARE_THICKNESS / 2.0,
                2.0 * half,
                FLARE_THICKNESS,
                &attrs,
            ));
        }
    } else {
        let (hx, hy) = flare_half_lengths(at, PAPER_FLARE_MULT * r, PAPER_FLARE_WIDTH, labels);
        let mut d = String::new();
        if hx >= FLARE_MIN_HALF {
            d.push_str(&format!(
                "M {} {} L {} {}",
                fmt(x - hx),
                fmt(y),
                fmt(x + hx),
                fmt(y)
            ));
        }
        if hy >= FLARE_MIN_HALF {
            if !d.is_empty() {
                d.push(' ');
            }
            d.push_str(&format!(
                "M {} {} L {} {}",
                fmt(x),
                fmt(y - hy),
                fmt(x),
                fmt(y + hy)
            ));
        }
        if !d.is_empty() {
            inner.push_str(&svg::path(
                &d,
                &[
                    ("fill", "none".to_string()),
                    ("stroke", theme.ink.to_string()),
                    ("stroke-width", fmt(PAPER_FLARE_WIDTH)),
                    ("stroke-opacity", fmt(PAPER_FLARE_OPACITY)),
                    ("stroke-linecap", "round".to_string()),
                ],
            ));
        }
    }
    inner
}

/// A star's ring: the founding ring (solid, `FOUNDING_RING` radii) if it is the founding star,
/// otherwise the fresh ring (dotted, `FRESH_RING` radii) if it is fresh and awake, otherwise nothing.
/// A founding star that is also fresh gets only the founding ring.
pub fn ring_markup(theme: &Theme, star: &Star, at: Point, palette: &StarPalette) -> String {
    let r = star_radius(star.tier);
    let color = if theme.is_dark {
        &palette.glow
    } else {
        &palette.body
    };
    if star.founding {
        ring(
            at,
            FOUNDING_RING * r,
            color,
            RING_FOUNDING_OPACITY,
            RING_STROKE,
            None,
        )
    } else if star.fresh && !star.dormant {
        ring(
            at,
            FRESH_RING * r,
            color,
            RING_FRESH_OPACITY,
            RING_FRESH_STROKE,
            Some(RING_FRESH_DASH),
        )
    } else {
        String::new()
    }
}
