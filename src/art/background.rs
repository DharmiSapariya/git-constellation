//! Sky gradient, horizon glow, nebula, dust and paper grain.

use super::layer_rng;
use crate::layout::anchors::SeededRng;
use crate::layout::{Rect, dist};
use crate::svg::{self, Stop, SvgBuilder, Units, fmt};
use crate::theme::Theme;

// The nebula, watercolor blobs and paper fibers live in `nebula.rs`; re-exported here.
pub use super::nebula::*;

// ---- Horizon glow (a radial gradient along the bottom edge) ----
/// Glow center, as shares of the canvas size, and radius as a share of the height.
pub const GLOW_CENTER: (f64, f64) = (0.5, 1.07);
pub const GLOW_RADIUS: f64 = 0.87;

// ---- Dust ----
pub const DUST_MIN_SPACING: f64 = 14.0;
pub const DUST_ATTEMPTS: usize = 4000;
pub const DUST_TINT_SHARE: f64 = 0.10;
/// A dot is rejected if closer than this many radii to an `avoid` entry.
pub const DUST_AVOID_RADII: f64 = 4.0;
/// Dust and Milky Way dots keep this far outside every label box that is drawn.
pub const DUST_LABEL_PAD: f64 = 4.0;

// ---- Grain ----
pub const GRAIN_OPACITY: f64 = 0.06;
pub const GRAIN_FREQUENCY: f64 = 0.8;

/// The sky gradient, the horizon glow and (for paper) the vignette.
pub fn draw_sky(b: &mut SvgBuilder, theme: &Theme) {
    let (w, h) = (b.width(), b.height());
    let sky = b.new_id("sky");
    b.linear_gradient(
        &sky,
        (0.0, 0.0),
        (0.0, 1.0),
        Units::ObjectBoundingBox,
        &[
            Stop::new(0.0, theme.sky_top, 1.0),
            Stop::new(0.55, theme.sky_mid, 1.0),
            Stop::new(1.0, theme.sky_bottom, 1.0),
        ],
    );
    let glow = (theme.horizon_glow.1 > 0.0).then(|| {
        let id = b.new_id("glow");
        b.radial_gradient(
            &id,
            (w * GLOW_CENTER.0, h * GLOW_CENTER.1),
            h * GLOW_RADIUS,
            Units::UserSpaceOnUse,
            &[
                Stop::new(0.0, theme.horizon_glow.0, theme.horizon_glow.1),
                Stop::new(1.0, theme.horizon_glow.0, 0.0),
            ],
        );
        id
    });
    let vignette = theme.vignette.map(|color| {
        let id = b.new_id("vig");
        b.radial_gradient(
            &id,
            (w / 2.0, h / 2.0),
            w.hypot(h) / 2.0,
            Units::UserSpaceOnUse,
            &[
                Stop::new(0.0, color, 0.0),
                Stop::new(0.55, color, 0.0),
                Stop::new(1.0, color, 0.9),
            ],
        );
        id
    });
    b.group(&[("id", "bg".to_string())], |g| {
        g.add(&svg::rect(
            0.0,
            0.0,
            w,
            h,
            &[("fill", format!("url(#{sky})"))],
        ));
        for id in glow.iter().chain(vignette.iter()) {
            g.add(&svg::rect(
                0.0,
                0.0,
                w,
                h,
                &[("fill", format!("url(#{id})"))],
            ));
        }
    });
}

/// One speck of dust.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DustDot {
    pub x: f64,
    pub y: f64,
    pub r: f64,
    /// Index into the theme's four dust opacities.
    pub bucket: usize,
    pub tint: bool,
}

/// Scatter up to `count` dust dots by dart throwing: each must be 14 px from
/// every other dot, `4 * radius` from every `avoid` entry `(x, y, radius)`, and
/// outside every `avoid_rects` box (label boxes) inflated by `DUST_LABEL_PAD`.
/// Gives up after 4000 throws, so a crowded canvas simply gets fewer dots.
pub fn dust_dots(
    seed: u64,
    width: f64,
    height: f64,
    count: usize,
    avoid: &[(f64, f64, f64)],
    avoid_rects: &[Rect],
) -> Vec<DustDot> {
    let mut rng: SeededRng = layer_rng(seed, super::DUST);
    let mut dots: Vec<DustDot> = Vec::new();
    for _ in 0..DUST_ATTEMPTS {
        if dots.len() >= count {
            break;
        }
        let (x, y) = (rng.range(0.0, width), rng.range(0.0, height));
        let u = rng.unit();
        let r = 0.5 + 0.7 * u * u;
        let bucket = rng.below(4);
        let tint = rng.unit() < DUST_TINT_SHARE;
        let clear_of_dust = dots
            .iter()
            .all(|d| dist((x, y), (d.x, d.y)) >= DUST_MIN_SPACING);
        let clear_of_stars = avoid
            .iter()
            .all(|&(ax, ay, ar)| dist((x, y), (ax, ay)) >= DUST_AVOID_RADII * ar);
        let clear_of_labels = avoid_rects
            .iter()
            .all(|r| !r.inflate(DUST_LABEL_PAD).contains((x, y)));
        if clear_of_dust && clear_of_stars && clear_of_labels {
            dots.push(DustDot {
                x,
                y,
                r,
                bucket,
                tint,
            });
        }
    }
    dots
}

/// Dust: one `<g>` per opacity bucket and color, each dot just `cx cy r`.
pub fn draw_dust(
    b: &mut SvgBuilder,
    theme: &Theme,
    seed: u64,
    count: usize,
    avoid: &[(f64, f64, f64)],
    avoid_rects: &[Rect],
) {
    let dots = dust_dots(seed, b.width(), b.height(), count, avoid, avoid_rects);
    b.group(&[("id", "dust".to_string())], |g| {
        for (bucket, opacity) in theme.dust_opacities.iter().enumerate() {
            for tint in [false, true] {
                let members: Vec<&DustDot> = dots
                    .iter()
                    .filter(|d| d.bucket == bucket && d.tint == tint)
                    .collect();
                if members.is_empty() {
                    continue;
                }
                let color = if tint {
                    theme.dust_tint
                } else {
                    theme.dust_color
                };
                g.group(
                    &[("fill", color.to_string()), ("opacity", fmt(*opacity))],
                    |inner| {
                        for d in members {
                            inner.add(&svg::circle(d.x, d.y, d.r, &[]));
                        }
                    },
                );
            }
        }
    });
}

/// Optional film grain: one turbulence filter at 6% opacity.
pub fn draw_grain(b: &mut SvgBuilder, seed: u64) {
    let mut rng = layer_rng(seed, super::GRAIN);
    let noise_seed = (rng.unit() * 10_000.0) as u32;
    let id = b.new_id("grain");
    b.add_def(&format!(
        "<filter id=\"{id}\" x=\"0\" y=\"0\" width=\"100%\" height=\"100%\"><feTurbulence type=\"fractalNoise\" baseFrequency=\"{}\" numOctaves=\"2\" seed=\"{noise_seed}\" stitchTiles=\"stitch\"/><feColorMatrix type=\"saturate\" values=\"0\"/></filter>",
        fmt(GRAIN_FREQUENCY)
    ));
    let (w, h) = (b.width(), b.height());
    b.group(&[("id", "grain".to_string())], |g| {
        g.add(&svg::rect(
            0.0,
            0.0,
            w,
            h,
            &[
                ("filter", format!("url(#{id})")),
                ("opacity", fmt(GRAIN_OPACITY)),
            ],
        ));
    });
}
