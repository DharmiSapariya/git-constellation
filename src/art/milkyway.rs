//! The contribution band (Milky Way): a glowing S-curve of soft clouds, dark dust
//! lanes and tiny stars, one segment per week, brighter where there were more contributions.

use super::layer_rng;
use crate::layout::dist;
use crate::model::{Point, WeekDensity};
use crate::svg::{self, Stop, SvgBuilder, Units, fmt};
use crate::theme::Theme;

/// The centerline: a cubic Bezier on the default 1200 x 600 canvas, running
/// from the lower left to the upper right (scaled if the canvas differs).
pub const CONTROL_POINTS: [Point; 4] = [
    (-60.0, 545.0),
    (330.0, 520.0),
    (800.0, 300.0),
    (1260.0, 215.0),
];
/// Band thickness is `MIN + SPAN * sin(pi * t) ^ POWER`: thin at the ends, wide in the middle.
pub const WIDTH_MIN: f64 = 40.0;
pub const WIDTH_SPAN: f64 = 130.0;
pub const WIDTH_POWER: f64 = 0.7;
/// One segment per week of a year.
pub const DEFAULT_SEGMENTS: usize = 53;
/// A quiet week still glows a little.
pub const MIN_DENSITY: f64 = 0.15;
/// Cloud opacity is `BASE + SPAN * density`. Kept low so overlapping clouds stay soft.
pub const CLOUD_OPACITY_BASE: f64 = 0.02;
pub const CLOUD_OPACITY_SPAN: f64 = 0.07;
/// Cloud size: `rx` is this many segment lengths, `ry` this share of the band width.
pub const CLOUD_RX_PER_LENGTH: f64 = 1.6;
pub const CLOUD_RY_SHARE: f64 = 0.55;
/// Clouds sit off the centerline by up to this share of the band width.
pub const CLOUD_OFFSET_SHARE: f64 = 0.15;
/// Stars per segment: `round(BASE + SPAN * density)`, but never more than `MAX_DOTS` in all.
pub const DOTS_BASE: f64 = 2.0;
pub const DOTS_SPAN: f64 = 20.0;
pub const MAX_DOTS: usize = 700;
pub const DOT_RADIUS: (f64, f64) = (0.4, 1.0);
pub const DOT_OPACITIES: [f64; 4] = [0.15, 0.3, 0.5, 0.7];

// ---- Dust lanes: dark streaks along the band that give it texture ----
/// How many lanes, and the random stream they use.
pub const LANE_COUNT: usize = 6;
pub const LANES: &str = "milkyway-lanes";
/// Lane size on the default canvas: `rx` along the band, `ry` across it.
pub const LANE_RX: f64 = 90.0;
pub const LANE_RY: f64 = 14.0;
/// A lane sits off the centerline by up to this share of the band width.
pub const LANE_OFFSET_SHARE: f64 = 0.25;
/// Each lane sits somewhere inside its own sixth of the band (a share of that sixth).
pub const LANE_SLOT: (f64, f64) = (0.15, 0.85);
/// Lane opacity on dark themes (sky_bottom color) and on light themes (ink color).
pub const LANE_OPACITY: f64 = 0.10;
pub const LANE_OPACITY_LIGHT: f64 = 0.04;

fn scaled_points(width: f64, height: f64) -> [Point; 4] {
    CONTROL_POINTS.map(|(x, y)| (x * width / 1200.0, y * height / 600.0))
}

/// A point on the cubic Bezier at `t` (0 to 1).
pub fn point_at(p: &[Point; 4], t: f64) -> Point {
    let u = 1.0 - t;
    let (a, b, c, d) = (u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t);
    (
        a * p[0].0 + b * p[1].0 + c * p[2].0 + d * p[3].0,
        a * p[0].1 + b * p[1].1 + c * p[2].1 + d * p[3].1,
    )
}

/// The direction of travel at `t` (not normalized).
pub fn tangent_at(p: &[Point; 4], t: f64) -> Point {
    let u = 1.0 - t;
    let (a, b, c) = (3.0 * u * u, 6.0 * u * t, 3.0 * t * t);
    (
        a * (p[1].0 - p[0].0) + b * (p[2].0 - p[1].0) + c * (p[3].0 - p[2].0),
        a * (p[1].1 - p[0].1) + b * (p[2].1 - p[1].1) + c * (p[3].1 - p[2].1),
    )
}

/// Band thickness at `t`, in pixels for the default canvas height (scaled by `height / 600`).
pub fn thickness(t: f64, height: f64) -> f64 {
    (WIDTH_MIN + WIDTH_SPAN * (std::f64::consts::PI * t).sin().max(0.0).powf(WIDTH_POWER)) * height
        / 600.0
}

/// The sampled centerline, for tests and drawing.
pub fn centerline(width: f64, height: f64, samples: usize) -> Vec<Point> {
    let p = scaled_points(width, height);
    (0..samples)
        .map(|i| point_at(&p, i as f64 / (samples.max(2) - 1) as f64))
        .collect()
}

/// Density of each segment: `max(0.15, week / busiest week)`. A year with no
/// contributions (or no weeks at all) gives every segment 0.15.
pub fn densities(weeks: &[WeekDensity]) -> Vec<f64> {
    if weeks.is_empty() {
        return vec![MIN_DENSITY; DEFAULT_SEGMENTS];
    }
    weeks
        .iter()
        .map(|w| w.density.clamp(0.0, 1.0).max(MIN_DENSITY))
        .collect()
}

/// Stars per segment, scaled down together if the total would pass the cap.
pub fn dot_counts(densities: &[f64]) -> Vec<usize> {
    let wanted: Vec<usize> = densities
        .iter()
        .map(|d| (DOTS_BASE + DOTS_SPAN * d).round() as usize)
        .collect();
    let total: usize = wanted.iter().sum();
    if total <= MAX_DOTS {
        return wanted;
    }
    wanted
        .iter()
        .map(|n| (*n as f64 * MAX_DOTS as f64 / total as f64).floor() as usize)
        .collect()
}

/// Opacity of a cloud in a segment of the given density, after the theme's strength.
pub fn cloud_opacity(density: f64, strength: f64) -> f64 {
    ((CLOUD_OPACITY_BASE + CLOUD_OPACITY_SPAN * density) * strength).clamp(0.0, 1.0)
}

/// One soft cloud along the band.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BandCloud {
    pub cx: f64,
    pub cy: f64,
    pub rx: f64,
    pub ry: f64,
    pub angle_deg: f64,
    /// Core color (true) or fringe color (false).
    pub core: bool,
    pub opacity: f64,
}

/// One tiny star in the band.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BandDot {
    pub x: f64,
    pub y: f64,
    pub r: f64,
    pub bucket: usize,
    pub core: bool,
}

/// Two clouds per segment (one core color, one fringe), set off the centerline a little.
pub fn band_clouds(
    seed: u64,
    width: f64,
    height: f64,
    densities: &[f64],
    strength: f64,
) -> Vec<BandCloud> {
    let p = scaled_points(width, height);
    let n = densities.len();
    let mut rng = layer_rng(seed, super::MILKYWAY);
    let mut clouds = Vec::with_capacity(n * 2);
    for (i, d) in densities.iter().enumerate() {
        let (t0, t1) = (i as f64 / n as f64, (i + 1) as f64 / n as f64);
        let tm = (t0 + t1) / 2.0;
        let (start, end, mid) = (point_at(&p, t0), point_at(&p, t1), point_at(&p, tm));
        let (tx, ty) = tangent_at(&p, tm);
        let length = dist(start, end);
        let band = thickness(tm, height);
        let norm = tx.hypot(ty).max(1e-9);
        let normal = (-ty / norm, tx / norm);
        for core in [true, false] {
            let offset = rng.range(-CLOUD_OFFSET_SHARE, CLOUD_OFFSET_SHARE) * band;
            clouds.push(BandCloud {
                cx: mid.0 + normal.0 * offset,
                cy: mid.1 + normal.1 * offset,
                rx: CLOUD_RX_PER_LENGTH * length,
                ry: CLOUD_RY_SHARE * band,
                angle_deg: ty.atan2(tx).to_degrees(),
                core,
                opacity: cloud_opacity(*d, strength),
            });
        }
    }
    clouds
}

/// One dark dust lane lying along the band.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BandLane {
    pub cx: f64,
    pub cy: f64,
    pub rx: f64,
    pub ry: f64,
    pub angle_deg: f64,
}

/// Six dark lanes spread along the band, each off the centerline a little and
/// turned to the direction of travel.
pub fn band_lanes(seed: u64, width: f64, height: f64) -> Vec<BandLane> {
    let p = scaled_points(width, height);
    let mut rng = layer_rng(seed, LANES);
    let mut lanes = Vec::with_capacity(LANE_COUNT);
    for i in 0..LANE_COUNT {
        let slot = rng.range(LANE_SLOT.0, LANE_SLOT.1);
        let t = (i as f64 + slot) / LANE_COUNT as f64;
        let center = point_at(&p, t);
        let (tx, ty) = tangent_at(&p, t);
        let norm = tx.hypot(ty).max(1e-9);
        let offset = rng.range(-LANE_OFFSET_SHARE, LANE_OFFSET_SHARE) * thickness(t, height);
        lanes.push(BandLane {
            cx: center.0 - ty / norm * offset,
            cy: center.1 + tx / norm * offset,
            rx: LANE_RX * width / 1200.0,
            ry: LANE_RY * height / 600.0,
            angle_deg: ty.atan2(tx).to_degrees(),
        });
    }
    lanes
}

/// Tiny stars scattered across the band, roughly Gaussian across its width.
pub fn band_dots(seed: u64, width: f64, height: f64, densities: &[f64]) -> Vec<BandDot> {
    let p = scaled_points(width, height);
    let n = densities.len();
    let counts = dot_counts(densities);
    let mut rng = layer_rng(seed, "milkyway-dots");
    let mut dots = Vec::new();
    for (i, count) in counts.iter().enumerate() {
        for _ in 0..*count {
            let t = (i as f64 + rng.unit()) / n as f64;
            let center = point_at(&p, t);
            let (tx, ty) = tangent_at(&p, t);
            let norm = tx.hypot(ty).max(1e-9);
            // Three uniforms add up to a bell shape; `across` runs from -1 to 1.
            let across = (rng.unit() + rng.unit() + rng.unit() - 1.5) / 1.5;
            let offset = across * thickness(t, height) / 2.0;
            let r = rng.range(DOT_RADIUS.0, DOT_RADIUS.1);
            let bucket = rng.below(4);
            dots.push(BandDot {
                x: center.0 - ty / norm * offset,
                y: center.1 + tx / norm * offset,
                r,
                bucket,
                core: across.abs() < 0.5,
            });
        }
    }
    dots
}

/// Draw the band: soft clouds first, then the dark dust lanes, then the tiny stars on top.
pub fn draw_milkyway(b: &mut SvgBuilder, theme: &Theme, weeks: &[WeekDensity], seed: u64) {
    let (w, h) = (b.width(), b.height());
    let density = densities(weeks);
    let gradient = |b: &mut SvgBuilder, color: &'static str| {
        let id = b.new_id("mw");
        b.radial_gradient(
            &id,
            (0.5, 0.5),
            0.5,
            Units::ObjectBoundingBox,
            &[Stop::new(0.0, color, 1.0), Stop::new(1.0, color, 0.0)],
        );
        id
    };
    let (core_id, fringe_id) = (
        gradient(b, theme.milkyway_core),
        gradient(b, theme.milkyway_fringe),
    );
    let clouds = band_clouds(seed, w, h, &density, theme.milkyway_strength);
    let lanes = band_lanes(seed, w, h);
    // Dark lanes use the sky's own darkest color; on light themes, the ink color.
    let (lane_color, lane_opacity) = if theme.is_dark {
        (theme.sky_bottom, LANE_OPACITY)
    } else {
        (theme.ink, LANE_OPACITY_LIGHT)
    };
    let lane_id = gradient(b, lane_color);
    let dots = band_dots(seed, w, h, &density);
    b.group(&[("id", "milkyway".to_string())], |g| {
        for c in &clouds {
            let fill = if c.core { &core_id } else { &fringe_id };
            g.add(&svg::ellipse(
                c.cx,
                c.cy,
                c.rx,
                c.ry,
                &[
                    ("fill", format!("url(#{fill})")),
                    ("opacity", fmt(c.opacity)),
                    (
                        "transform",
                        format!("rotate({} {} {})", fmt(c.angle_deg), fmt(c.cx), fmt(c.cy)),
                    ),
                ],
            ));
        }
        for lane in &lanes {
            g.add(&svg::ellipse(
                lane.cx,
                lane.cy,
                lane.rx,
                lane.ry,
                &[
                    ("fill", format!("url(#{lane_id})")),
                    ("opacity", fmt(lane_opacity)),
                    (
                        "transform",
                        format!(
                            "rotate({} {} {})",
                            fmt(lane.angle_deg),
                            fmt(lane.cx),
                            fmt(lane.cy)
                        ),
                    ),
                ],
            ));
        }
        for (bucket, opacity) in DOT_OPACITIES.iter().enumerate() {
            for core in [true, false] {
                let members: Vec<&BandDot> = dots
                    .iter()
                    .filter(|d| d.bucket == bucket && d.core == core)
                    .collect();
                if members.is_empty() {
                    continue;
                }
                let color = if core {
                    theme.milkyway_core
                } else {
                    theme.milkyway_fringe
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
