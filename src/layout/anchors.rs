//! Seeded randomness, the Halton sequence, and constellation anchor placement.

use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

use super::{CANVAS_HEIGHT, CANVAS_WIDTH, Region, dist, safe_rect};
use crate::SkyError;
use crate::model::Point;
use crate::rules::fnv1a_64;

/// Candidate anchor points tried per constellation before shrinking.
pub const ANCHOR_CANDIDATES: usize = 400;
/// Required gap between two constellation circles.
pub const ANCHOR_GAP: f64 = 30.0;
/// Radii shrink by this factor when placement fails.
pub const ANCHOR_SHRINK: f64 = 0.92;
/// How many times radii may shrink.
pub const ANCHOR_SHRINK_TRIES: i32 = 6;
/// The brightest constellation starts in this share of the width (2.7, 5.1).
pub const FIRST_ANCHOR_X: (f64, f64) = (0.25, 0.45);
/// ...and in this share of the height, so the brightest group sits mid-sky.
pub const FIRST_ANCHOR_Y: (f64, f64) = (0.30, 0.70);
/// An anchor stays this share of its radius away from reserved areas.
pub const ANCHOR_RESERVED_PAD: f64 = 0.4;
/// ...and this share of its radius away from the safe-area edge.
pub const ANCHOR_EDGE_PAD: f64 = 0.5;

pub use super::sparse::{
    SMALL_SKY_ANCHOR_GAP, SMALL_SKY_JITTER, SMALL_SKY_MAX_STARS, SMALL_SKY_SLOTS,
};

/// A small seeded random stream. Every layer of the layout gets its own
/// stream, so adding randomness in one layer never shifts another (4.9).
pub struct SeededRng(ChaCha8Rng);

impl SeededRng {
    /// The stream for one named layer (and index, and layout attempt).
    pub fn for_layer(seed: u64, layer: &str, index: u64, attempt: u32) -> Self {
        let mut bytes = Vec::with_capacity(layer.len() + 20);
        bytes.extend_from_slice(&seed.to_le_bytes());
        bytes.extend_from_slice(layer.as_bytes());
        bytes.extend_from_slice(&index.to_le_bytes());
        bytes.extend_from_slice(&attempt.to_le_bytes());
        SeededRng(ChaCha8Rng::seed_from_u64(fnv1a_64(&bytes)))
    }

    /// A float in `[0, 1)` built from the top 53 bits of one 64-bit draw.
    /// Hand-made so no library's float conversion can change our output.
    pub fn unit(&mut self) -> f64 {
        (self.0.next_u64() >> 11) as f64 / 9_007_199_254_740_992.0
    }

    pub fn range(&mut self, low: f64, high: f64) -> f64 {
        low + (high - low) * self.unit()
    }

    /// A standard normal number (Box-Muller transform).
    pub fn normal(&mut self) -> f64 {
        let u1 = 1.0 - self.unit(); // in (0, 1], so ln() is finite
        let u2 = self.unit();
        (-2.0 * u1.ln()).sqrt() * (std::f64::consts::TAU * u2).cos()
    }

    /// An index in `0..n` (0 when `n` is 0).
    pub fn below(&mut self, n: usize) -> usize {
        ((self.unit() * n as f64) as usize).min(n.saturating_sub(1))
    }
}

/// The `index`-th point of the Halton sequence in the given base, in `[0, 1)`.
pub fn halton(mut index: u64, base: u64) -> f64 {
    let (mut fraction, mut result) = (1.0, 0.0);
    while index > 0 {
        fraction /= base as f64;
        result += fraction * (index % base) as f64;
        index /= base;
    }
    result
}

/// Estimated constellation radius: `clamp(70 + 22 * sqrt(n), 90, 200)`.
pub fn radius_estimate(star_count: usize) -> f64 {
    (70.0 + 22.0 * (star_count as f64).sqrt()).clamp(90.0, 200.0)
}

/// Place one anchor per constellation. `sizes` is the star count of each
/// constellation, brightest first. Returns `(anchor, base_radius)` for each.
pub fn place_anchors(
    sizes: &[usize],
    small_sky: bool,
    seed: u64,
    attempt: u32,
    regions: &[Region],
) -> Result<Vec<(Point, f64)>, SkyError> {
    let mut salt = Vec::new();
    salt.extend_from_slice(&seed.to_le_bytes());
    salt.extend_from_slice(b"halton");
    salt.extend_from_slice(&attempt.to_le_bytes());
    let offset = 1 + fnv1a_64(&salt) % 4096;

    let base: Vec<f64> = sizes.iter().map(|&n| radius_estimate(n)).collect();
    let shrunk = |shrink: i32| -> Vec<f64> {
        base.iter()
            .map(|r| r * ANCHOR_SHRINK.powi(shrink))
            .collect()
    };
    // A small sky tries its composed slots at every size first: three or four groups only fit
    // the slots a little smaller, and a random placement that fits at full size would
    // scatter them across the canvas.
    if small_sky {
        for shrink in 0..=ANCHOR_SHRINK_TRIES {
            if let Some(anchors) = slot_place(&shrunk(shrink), seed, attempt, regions) {
                return Ok(anchors);
            }
        }
    }
    for shrink in 0..=ANCHOR_SHRINK_TRIES {
        if let Some(anchors) = try_place(&shrunk(shrink), offset, regions) {
            return Ok(anchors);
        }
    }
    Err(SkyError::Layout(format!(
        "crowded: could not place {} constellations without overlap",
        sizes.len()
    )))
}

/// Put anchors in the staggered slots (with jitter), if they all fit.
fn slot_place(
    radii: &[f64],
    seed: u64,
    attempt: u32,
    regions: &[Region],
) -> Option<Vec<(Point, f64)>> {
    let slots = SMALL_SKY_SLOTS.get(radii.len().checked_sub(1)?)?;
    let mut rng = SeededRng::for_layer(seed, "slots", 0, attempt);
    let safe = safe_rect();
    let mut accepted: Vec<(Point, f64)> = Vec::new();
    for (&(fx, fy), &radius) in slots.iter().zip(radii) {
        let point = (
            fx * CANVAS_WIDTH + rng.range(-SMALL_SKY_JITTER, SMALL_SKY_JITTER),
            fy * CANVAS_HEIGHT + rng.range(-SMALL_SKY_JITTER, SMALL_SKY_JITTER),
        );
        let inside = safe.inflate(-ANCHOR_EDGE_PAD * radius).contains(point);
        let clear = !regions
            .iter()
            .any(|r| r.blocks_point(point, ANCHOR_RESERVED_PAD * radius));
        let apart = accepted.iter().all(|&(other, other_radius)| {
            dist(point, other) >= radius + other_radius + SMALL_SKY_ANCHOR_GAP
        });
        if !(inside && clear && apart) {
            return None;
        }
        accepted.push((point, radius));
    }
    Some(accepted)
}

fn try_place(radii: &[f64], offset: u64, regions: &[Region]) -> Option<Vec<(Point, f64)>> {
    let safe = safe_rect();
    let mut accepted: Vec<(Point, f64)> = Vec::new();
    let mut next = offset;
    for (ci, &radius) in radii.iter().enumerate() {
        let edge = ANCHOR_EDGE_PAD * radius;
        let (mut x0, mut x1) = (safe.x0 + edge, safe.x1 - edge);
        if ci == 0 {
            x0 = x0.max(FIRST_ANCHOR_X.0 * CANVAS_WIDTH);
            x1 = x1.min(FIRST_ANCHOR_X.1 * CANVAS_WIDTH);
        }
        let (mut y0, mut y1) = (safe.y0 + edge, safe.y1 - edge);
        if ci == 0 {
            y0 = y0.max(FIRST_ANCHOR_Y.0 * CANVAS_HEIGHT);
            y1 = y1.min(FIRST_ANCHOR_Y.1 * CANVAS_HEIGHT);
        }
        if x0 >= x1 || y0 >= y1 {
            return None;
        }

        let mut found = None;
        for _ in 0..ANCHOR_CANDIDATES {
            let point = (
                x0 + halton(next, 2) * (x1 - x0),
                y0 + halton(next, 3) * (y1 - y0),
            );
            next += 1;
            let clear_of_regions = !regions
                .iter()
                .any(|r| r.blocks_point(point, ANCHOR_RESERVED_PAD * radius));
            let clear_of_others = accepted.iter().all(|&(other, other_radius)| {
                dist(point, other) >= radius + other_radius + ANCHOR_GAP
            });
            if clear_of_regions && clear_of_others {
                found = Some((point, radius));
                break;
            }
        }
        accepted.push(found?);
    }
    debug_assert!(
        accepted
            .iter()
            .all(|(p, _)| p.0 <= CANVAS_WIDTH && p.1 <= CANVAS_HEIGHT)
    );
    Some(accepted)
}
