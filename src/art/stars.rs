//! Stars: the halo, body, core, flare and rings of every repo, drawn from the layout's positions.

use std::collections::BTreeMap;

use crate::layout::geometry::star_radius;
use crate::model::{Point, SkyModel, Star};
use crate::svg::{self, Stop, SvgBuilder, Units, fmt};
use crate::theme::{Rgb, Theme, mix, parse_hex};

use super::layer_rng;
use super::star_parts::{body_markup, flare_gradient, flare_markup, ring_markup, wants_flare};

// The flare, ring, borrowed and paper constants live next to the code that uses them.
pub use super::star_parts::*;

// ---- Look and feel: every number that shapes a star, in one place ----
/// Outer halo: radius in units of the star radius, and opacity at its center.
pub const HALO_OUTER_MULT: f64 = 3.4;
pub const HALO_OUTER_OPACITY: f64 = 0.28;
/// Inner halo, the same two numbers.
pub const HALO_INNER_MULT: f64 = 1.8;
pub const HALO_INNER_OPACITY: f64 = 0.35;
/// A halo fades in two steps: at this share of its radius it has this share of its center opacity.
pub const HALO_MID: (f64, f64) = (0.45, 0.4);
/// Sparkle tip length and the concave waist, in units of the star radius.
pub const SPARKLE_TIP: f64 = 2.1;
pub const SPARKLE_WAIST: f64 = 0.2;
/// Every sparkle is turned by a seeded angle within plus or minus this many degrees.
pub const SPARKLE_ROTATION_MAX_DEG: f64 = 9.0;
/// Dormant stars: body opacity, halo factor, and the cool tint they are mixed toward.
pub const DORMANT_BODY_OPACITY: f64 = 0.55;
pub const DORMANT_HALO_FACTOR: f64 = 0.5;
pub const DORMANT_TINT: &str = "#9aa3d0";
pub const DORMANT_TINT_MIX: f64 = 0.45;
/// Paper theme: watercolor wash size and opacity, and how far the body is darkened.
pub const WASH_MULT: f64 = 2.4;
pub const WASH_OPACITY: f64 = 0.28;
pub const PAPER_BODY_DARKEN: f64 = 0.18;

/// The colors one star is drawn with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StarPalette {
    /// The glow color (the language color, cooled if the star is dormant).
    pub glow: String,
    /// The body color: the glow color on dark themes, inked a little darker on paper.
    pub body: String,
}

fn parse(hex: &str) -> Rgb {
    parse_hex(hex).unwrap_or(Rgb {
        r: 255,
        g: 255,
        b: 255,
    })
}

/// The effective colors of a star in a theme.
pub fn star_palette(theme: &Theme, star: &Star) -> StarPalette {
    let mut glow = parse(theme.star_color(star.language));
    if star.dormant {
        glow = mix(glow, parse(DORMANT_TINT), DORMANT_TINT_MIX);
    }
    let body = if theme.is_dark {
        glow
    } else {
        mix(glow, parse(theme.ink), PAPER_BODY_DARKEN)
    };
    StarPalette {
        glow: glow.to_hex(),
        body: body.to_hex(),
    }
}

/// One seeded turn per star, in degrees. Every star draws one, so a star's turn
/// never depends on the tiers of the stars before it.
pub fn rotations(seed: u64, count: usize) -> Vec<f64> {
    let mut rng = layer_rng(seed, super::STARS);
    (0..count)
        .map(|_| rng.range(-SPARKLE_ROTATION_MAX_DEG, SPARKLE_ROTATION_MAX_DEG))
        .collect()
}

/// The path of a four-point sparkle with concave rounded sides, centered on (0, 0).
pub fn sparkle_path(r: f64) -> String {
    let (l, c) = (SPARKLE_TIP * r, SPARKLE_WAIST * r);
    let (pl, nl, pc, nc) = (fmt(l), fmt(-l), fmt(c), fmt(-c));
    format!(
        "M 0 {nl} Q {pc} {nc} {pl} 0 Q {pc} {pc} 0 {pl} Q {nc} {pc} {nl} 0 Q {nc} {nc} 0 {nl} Z"
    )
}

/// Gradients are shared: one per (kind, color, dormant) for the whole sky.
pub(super) type GradientCache = BTreeMap<(&'static str, String, bool), String>;

/// A shared radial gradient from `center` color at `opacity` to nothing at the edge.
fn halo_gradient(
    b: &mut SvgBuilder,
    cache: &mut GradientCache,
    prefix: &'static str,
    color: &str,
    opacity: f64,
    dormant: bool,
) -> String {
    let key = (prefix, color.to_string(), dormant);
    if let Some(id) = cache.get(&key) {
        return id.clone();
    }
    let strength = if dormant {
        opacity * DORMANT_HALO_FACTOR
    } else {
        opacity
    };
    let id = b.new_id(prefix);
    b.radial_gradient(
        &id,
        (0.5, 0.5),
        0.5,
        Units::ObjectBoundingBox,
        &[
            Stop::new(0.0, color, strength),
            Stop::new(HALO_MID.0, color, strength * HALO_MID.1),
            Stop::new(1.0, color, 0.0),
        ],
    );
    cache.insert(key, id.clone());
    id
}

/// A shared body gradient: white at the middle, the star's color at the edge.
pub(super) fn body_gradient(b: &mut SvgBuilder, cache: &mut GradientCache, color: &str) -> String {
    let key = ("body", color.to_string(), false);
    if let Some(id) = cache.get(&key) {
        return id.clone();
    }
    let id = b.new_id("body");
    b.radial_gradient(
        &id,
        (0.5, 0.5),
        0.5,
        Units::ObjectBoundingBox,
        &[Stop::new(0.0, "#ffffff", 1.0), Stop::new(1.0, color, 1.0)],
    );
    cache.insert(key, id.clone());
    id
}

/// Everything for one star, as one `<g id="star-N">`.
#[allow(clippy::too_many_arguments)]
fn star_group(
    b: &mut SvgBuilder,
    cache: &mut GradientCache,
    theme: &Theme,
    star: &Star,
    index: usize,
    at: Point,
    rotation: f64,
    flare_id: Option<&str>,
) -> String {
    let (x, y) = at;
    let r = star_radius(star.tier);
    let palette = star_palette(theme, star);
    let mut inner = String::new();

    // 1. Halo (a watercolor wash on paper).
    if theme.is_dark {
        let outer = halo_gradient(
            b,
            cache,
            "halo-o",
            &palette.glow,
            HALO_OUTER_OPACITY,
            star.dormant,
        );
        let inner_id = halo_gradient(
            b,
            cache,
            "halo-i",
            &palette.glow,
            HALO_INNER_OPACITY,
            star.dormant,
        );
        inner.push_str(&svg::circle(
            x,
            y,
            HALO_OUTER_MULT * r,
            &[("fill", format!("url(#{outer})"))],
        ));
        inner.push_str(&svg::circle(
            x,
            y,
            HALO_INNER_MULT * r,
            &[("fill", format!("url(#{inner_id})"))],
        ));
    } else {
        let wash = halo_gradient(b, cache, "wash", &palette.glow, WASH_OPACITY, star.dormant);
        inner.push_str(&svg::circle(
            x,
            y,
            WASH_MULT * r,
            &[("fill", format!("url(#{wash})"))],
        ));
    }

    // 2 and 3. Body and core (dimmed together when dormant).
    let body = body_markup(b, cache, theme, star, at, rotation, &palette);
    if star.dormant {
        inner.push_str(&svg::tag(
            "g",
            &[("opacity", fmt(DORMANT_BODY_OPACITY))],
            Some(&body),
        ));
    } else {
        inner.push_str(&body);
    }

    // 4. Flare of the calm tier 5 stars, then 5 and 6. Rings.
    inner.push_str(&flare_markup(theme, star, at, flare_id));
    inner.push_str(&ring_markup(theme, star, at, &palette));
    svg::tag("g", &[("id", format!("star-{index}"))], Some(&inner))
}

/// Draw the `stars` layer. `positions[i]` belongs to `model.stars[i]`.
pub fn draw_stars(b: &mut SvgBuilder, theme: &Theme, model: &SkyModel, positions: &[Point]) {
    let turns = rotations(model.seed, model.stars.len());
    let mut cache = GradientCache::new();
    let needs_flare = theme.is_dark && model.stars.iter().any(wants_flare);
    let flare_id = needs_flare.then(|| flare_gradient(b));
    b.group(&[("id", "stars".to_string())], |g| {
        for (i, star) in model.stars.iter().enumerate() {
            let Some(&at) = positions.get(i) else {
                continue;
            };
            let turn = turns.get(i).copied().unwrap_or(0.0);
            let markup = star_group(g, &mut cache, theme, star, i, at, turn, flare_id.as_deref());
            g.add(&markup);
        }
    });
}
