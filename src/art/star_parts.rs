//! The parts of a star that are not its halo: body and core, flare cross, rings and the paper inking.

use crate::layout::geometry::star_radius;
use crate::model::{Point, Star};
use crate::svg::{self, Stop, SvgBuilder, Units, fmt};
use crate::theme::Theme;

use super::stars::{GradientCache, StarPalette, body_gradient, sparkle_path};

/// The hot white core, as a share of the star radius, and its opacity.
pub const CORE_RADIUS_MULT: f64 = 0.38;
pub const CORE_OPACITY: f64 = 0.95;
/// The flare cross of the biggest stars: arm length (units of radius), peak opacity, thickness.
pub const FLARE_LENGTH_MULT: f64 = 5.0;
pub const FLARE_OPACITY: f64 = 0.5;
pub const FLARE_THICKNESS: f64 = 1.0;
/// Rings: radius in units of the star radius, stroke width, opacity.
pub const RING_FOUNDING_MULT: f64 = 2.6;
pub const RING_FOUNDING_OPACITY: f64 = 0.5;
pub const RING_FRESH_MULT: f64 = 2.0;
pub const RING_FRESH_OPACITY: f64 = 0.4;
pub const RING_STROKE: f64 = 0.8;
/// A borrowed (forked) star is a hollow ring this thick.
pub const BORROWED_STROKE: f64 = 1.0;
/// Paper theme: the ink outline of a body, and the opacity of its white core.
pub const PAPER_OUTLINE_WIDTH: f64 = 0.8;
pub const PAPER_OUTLINE_OPACITY: f64 = 0.7;
pub const PAPER_CORE_OPACITY: f64 = 0.6;
/// Paper tier 5 gets two short ink strokes instead of a glowing flare.
pub const PAPER_FLARE_MULT: f64 = 2.6;
pub const PAPER_FLARE_OPACITY: f64 = 0.5;
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

fn ring(at: Point, radius: f64, color: &str, opacity: f64) -> String {
    svg::circle(
        at.0,
        at.1,
        radius,
        &[
            ("fill", "none".to_string()),
            ("stroke", color.to_string()),
            ("stroke-width", fmt(RING_STROKE)),
            ("stroke-opacity", fmt(opacity)),
        ],
    )
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
    let (fill, outline) = if theme.is_dark {
        let id = body_gradient(b, cache, &palette.glow);
        (format!("url(#{id})"), Vec::new())
    } else {
        (
            palette.body.clone(),
            vec![
                ("stroke", theme.ink.to_string()),
                ("stroke-width", fmt(PAPER_OUTLINE_WIDTH)),
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

/// The flare of a calm tier 5 star: two glowing arms on dark themes (`flare_id` is their
/// shared gradient), two short ink strokes on paper. Empty for every other star.
pub(super) fn flare_markup(
    theme: &Theme,
    star: &Star,
    at: Point,
    flare_id: Option<&str>,
) -> String {
    let mut inner = String::new();
    if !wants_flare(star) {
        return inner;
    }
    let (x, y) = at;
    let r = star_radius(star.tier);
    if let Some(id) = flare_id {
        let length = FLARE_LENGTH_MULT * r;
        for turn in [false, true] {
            let mut attrs = vec![("fill", format!("url(#{id})"))];
            if turn {
                attrs.push(("transform", format!("rotate(90 {} {})", fmt(x), fmt(y))));
            }
            inner.push_str(&svg::rect(
                x - length / 2.0,
                y - FLARE_THICKNESS / 2.0,
                length,
                FLARE_THICKNESS,
                &attrs,
            ));
        }
    } else {
        let reach = PAPER_FLARE_MULT * r;
        let d = format!(
            "M {} {} L {} {} M {} {} L {} {}",
            fmt(x - reach),
            fmt(y),
            fmt(x + reach),
            fmt(y),
            fmt(x),
            fmt(y - reach),
            fmt(x),
            fmt(y + reach)
        );
        inner.push_str(&svg::path(
            &d,
            &[
                ("fill", "none".to_string()),
                ("stroke", theme.ink.to_string()),
                ("stroke-width", fmt(PAPER_OUTLINE_WIDTH)),
                ("stroke-opacity", fmt(PAPER_FLARE_OPACITY)),
                ("stroke-linecap", "round".to_string()),
            ],
        ));
    }
    inner
}

/// The founding ring and the fresh ring of a star (either, both or neither).
pub(super) fn ring_markup(theme: &Theme, star: &Star, at: Point, palette: &StarPalette) -> String {
    let r = star_radius(star.tier);
    let mut inner = String::new();
    let ring_color = if theme.is_dark {
        &palette.glow
    } else {
        &palette.body
    };
    if star.founding {
        inner.push_str(&ring(
            at,
            RING_FOUNDING_MULT * r,
            ring_color,
            RING_FOUNDING_OPACITY,
        ));
    }
    if star.fresh && !star.dormant {
        inner.push_str(&ring(
            at,
            RING_FRESH_MULT * r,
            ring_color,
            RING_FRESH_OPACITY,
        ));
    }
    inner
}
