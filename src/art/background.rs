//! Sky gradient, horizon glow, nebula, dust and paper grain.

use super::layer_rng;
use crate::layout::anchors::SeededRng;
use crate::layout::{HUD_RECT, MOON_CENTER, dist};
use crate::svg::{self, Stop, SvgBuilder, Units, fmt};
use crate::theme::{Backdrop, Theme};

// ---- Horizon glow (a radial gradient along the bottom edge) ----
/// Glow center, as shares of the canvas size, and radius as a share of the height.
pub const GLOW_CENTER: (f64, f64) = (0.5, 1.07);
pub const GLOW_RADIUS: f64 = 0.87;

// ---- Nebula ----
pub const NEBULA_COUNT: usize = 5;
pub const NEBULA_RX: (f64, f64) = (190.0, 350.0);
pub const NEBULA_RY: (f64, f64) = (100.0, 190.0);
pub const NEBULA_ROTATION_DEG: f64 = 30.0;
/// Centers are re-rolled (up to this many times) if they land too near the HUD or the moon.
pub const NEBULA_TRIES: usize = 50;
pub const NEBULA_HUD_CLEARANCE: f64 = 60.0;
pub const NEBULA_MOON_CLEARANCE: f64 = 130.0;
/// Centers may sit this far outside the canvas.
pub const NEBULA_OVERSHOOT: f64 = 100.0;
/// Watercolor blobs (paper theme).
pub const WATERCOLOR_COUNT: usize = 3;

// ---- Dust ----
pub const DUST_MIN_SPACING: f64 = 14.0;
pub const DUST_ATTEMPTS: usize = 4000;
pub const DUST_TINT_SHARE: f64 = 0.10;
/// A dot is rejected if closer than this many radii to an `avoid` entry.
pub const DUST_AVOID_RADII: f64 = 4.0;

// ---- Grain ----
pub const GRAIN_OPACITY: f64 = 0.06;
pub const GRAIN_FREQUENCY: f64 = 0.8;

/// A canvas size, and how reserved areas scale with it (they are defined for 1200 x 600).
fn scale_of(width: f64, height: f64) -> (f64, f64) {
    (width / 1200.0, height / 600.0)
}

/// Distance from a point to the (scaled) title rectangle.
fn hud_distance(p: (f64, f64), width: f64, height: f64) -> f64 {
    let (sx, sy) = scale_of(width, height);
    let r = HUD_RECT;
    let dx = (r.x0 * sx - p.0).max(0.0).max(p.0 - r.x1 * sx);
    let dy = (r.y0 * sy - p.1).max(0.0).max(p.1 - r.y1 * sy);
    dx.hypot(dy)
}

fn moon_distance(p: (f64, f64), width: f64, height: f64) -> f64 {
    let (sx, sy) = scale_of(width, height);
    dist(p, (MOON_CENTER.0 * sx, MOON_CENTER.1 * sy))
}

/// One soft cloud of color.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cloud {
    pub cx: f64,
    pub cy: f64,
    pub rx: f64,
    pub ry: f64,
    pub angle_deg: f64,
    /// Which of the theme's three cloud colors.
    pub color: usize,
}

/// Where the nebulae (or watercolor blobs) go. Seeded; each center is re-rolled
/// if it is within 60 px of the title or 130 px of the moon, and falls back to
/// a safe spot in the middle if every try fails.
pub fn clouds(seed: u64, width: f64, height: f64, count: usize) -> Vec<Cloud> {
    let mut rng = layer_rng(seed, super::NEBULA);
    let (sx, sy) = scale_of(width, height);
    (0..count)
        .map(|i| {
            let rx = rng.range(NEBULA_RX.0, NEBULA_RX.1) * sx;
            let ry = rng.range(NEBULA_RY.0, NEBULA_RY.1) * sy;
            let angle_deg = rng.range(-NEBULA_ROTATION_DEG, NEBULA_ROTATION_DEG);
            let mut center = (width * 0.5, height * 0.55);
            for _ in 0..NEBULA_TRIES {
                let candidate = (
                    rng.range(-NEBULA_OVERSHOOT, width + NEBULA_OVERSHOOT),
                    rng.range(-NEBULA_OVERSHOOT, height + NEBULA_OVERSHOOT),
                );
                let near_hud =
                    hud_distance(candidate, width, height) < NEBULA_HUD_CLEARANCE * sx.min(sy);
                let near_moon =
                    moon_distance(candidate, width, height) < NEBULA_MOON_CLEARANCE * sx.min(sy);
                if !near_hud && !near_moon {
                    center = candidate;
                    break;
                }
            }
            Cloud {
                cx: center.0,
                cy: center.1,
                rx,
                ry,
                angle_deg,
                color: i % 3,
            }
        })
        .collect()
}

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

/// Nebula clouds on dark themes, watercolor blobs on paper, nothing on mono.
pub fn draw_nebula(b: &mut SvgBuilder, theme: &Theme, seed: u64) {
    let count = match theme.backdrop {
        Backdrop::Nebula => NEBULA_COUNT,
        Backdrop::Watercolor => WATERCOLOR_COUNT,
        Backdrop::Plain => return,
    };
    // One shared gradient per cloud color: the color fades to nothing at the
    // edge, and each ellipse sets its own strength with `opacity`.
    let stops = |color: &'static str| match theme.backdrop {
        Backdrop::Watercolor => vec![
            Stop::new(0.0, color, 1.0),
            Stop::new(0.55, color, 0.75),
            Stop::new(1.0, color, 0.0),
        ],
        _ => vec![Stop::new(0.0, color, 1.0), Stop::new(1.0, color, 0.0)],
    };
    let ids: Vec<String> = theme
        .nebula
        .iter()
        .map(|(color, _)| {
            let id = b.new_id("neb");
            b.radial_gradient(
                &id,
                (0.5, 0.5),
                0.5,
                Units::ObjectBoundingBox,
                &stops(color),
            );
            id
        })
        .collect();
    let list = clouds(seed, b.width(), b.height(), count);
    b.group(&[("id", "nebula".to_string())], |g| {
        for cloud in &list {
            g.add(&svg::ellipse(
                cloud.cx,
                cloud.cy,
                cloud.rx,
                cloud.ry,
                &[
                    ("fill", format!("url(#{})", ids[cloud.color])),
                    ("opacity", fmt(theme.nebula[cloud.color].1)),
                    (
                        "transform",
                        format!(
                            "rotate({} {} {})",
                            fmt(cloud.angle_deg),
                            fmt(cloud.cx),
                            fmt(cloud.cy)
                        ),
                    ),
                ],
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
/// every other dot, and `4 * radius` from every `avoid` entry `(x, y, radius)`.
/// Gives up after 4000 throws, so a crowded canvas simply gets fewer dots.
pub fn dust_dots(
    seed: u64,
    width: f64,
    height: f64,
    count: usize,
    avoid: &[(f64, f64, f64)],
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
        if clear_of_dust && clear_of_stars {
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
) {
    let dots = dust_dots(seed, b.width(), b.height(), count, avoid);
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
