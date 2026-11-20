//! The sky's soft color clouds: nebulae on dark themes, watercolor blobs and a few fibers on paper.

use super::layer_rng;
use crate::layout::{HUD_RECT, MOON_CENTER, dist};
use crate::svg::{self, Stop, SvgBuilder, Units, fmt};
use crate::theme::{Backdrop, Theme};

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

// ---- Paper fibers: a few faint ink dashes, paper theme only ----
pub const FIBER_COUNT: usize = 4;
/// Dash length range in pixels, stroke width, and opacity.
pub const FIBER_LENGTH: (f64, f64) = (6.0, 14.0);
pub const FIBER_WIDTH: f64 = 0.6;
pub const FIBER_OPACITY: f64 = 0.08;

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

/// One paper-fiber dash, from `(x1, y1)` to `(x2, y2)`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Fiber {
    pub x1: f64,
    pub y1: f64,
    pub x2: f64,
    pub y2: f64,
}

/// Four seeded dashes at random places and angles.
pub fn paper_fibers(seed: u64, width: f64, height: f64) -> Vec<Fiber> {
    let mut rng = layer_rng(seed, super::FIBERS);
    let mut fibers = Vec::with_capacity(FIBER_COUNT);
    for _ in 0..FIBER_COUNT {
        let (x1, y1) = (rng.range(0.0, width), rng.range(0.0, height));
        let length = rng.range(FIBER_LENGTH.0, FIBER_LENGTH.1);
        let angle = rng.range(0.0, std::f64::consts::PI);
        fibers.push(Fiber {
            x1,
            y1,
            x2: x1 + length * angle.cos(),
            y2: y1 + length * angle.sin(),
        });
    }
    fibers
}

/// Nebula clouds on dark themes, watercolor blobs (and a few paper fibers) on
/// paper, nothing on mono.
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
    let fibers = if theme.backdrop == Backdrop::Watercolor {
        paper_fibers(seed, b.width(), b.height())
    } else {
        Vec::new()
    };
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
        for f in &fibers {
            let d = format!(
                "M {} {} L {} {}",
                fmt(f.x1),
                fmt(f.y1),
                fmt(f.x2),
                fmt(f.y2)
            );
            g.add(&svg::path(
                &d,
                &[
                    ("fill", "none".to_string()),
                    ("stroke", theme.ink.to_string()),
                    ("stroke-width", fmt(FIBER_WIDTH)),
                    ("stroke-linecap", "round".to_string()),
                    ("opacity", fmt(FIBER_OPACITY)),
                ],
            ));
        }
    });
}
