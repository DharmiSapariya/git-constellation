//! Shapes: the growing walk, spring relaxation, and wanderer placement.

use super::anchors::SeededRng;
use super::lines::point_segment_distance;
use super::{Region, WANDERER_LINE_CLEARANCE, dist, min_dist, star_area};
use crate::SkyError;
use crate::model::Point;

// ---- Growing walk (5.2) ----
/// Candidate positions tried for each new star.
pub const WALK_CANDIDATES: usize = 40;
/// Chance the parent is the most recent star, then the one before it.
pub const PARENT_LAST: f64 = 0.70;
pub const PARENT_BEFORE_LAST: f64 = 0.20;
/// Spread of the turn angle, and its hard limit, in degrees.
pub const TURN_SIGMA_DEG: f64 = 55.0;
pub const TURN_LIMIT_DEG: f64 = 100.0;
/// Step length is `STEP_MIN + STEP_SPAN * u` for `u` in `[0, 1)`.
pub const STEP_MIN: f64 = 50.0;
pub const STEP_SPAN: f64 = 40.0;
/// When no candidate fits, the circle grows by this factor, up to 5 times.
pub const ENLARGE_FACTOR: f64 = 1.10;
pub const MAX_ENLARGEMENTS: u32 = 5;
/// Extra room kept between a star body and a reserved area.
pub const REGION_PAD: f64 = 4.0;

// ---- Relaxation (5.3) ----
pub const RELAX_ITERATIONS: usize = 30;
pub const RELAX_STEP: f64 = 0.15;
/// The step shrinks by this factor every iteration.
pub const RELAX_DECAY: f64 = 0.92;
/// Stars repel when closer than this multiple of their `min_dist`.
pub const RELAX_REPEL_FACTOR: f64 = 1.4;
/// Weak pull toward the constellation centroid.
pub const RELAX_PULL: f64 = 0.10;

// ---- Wanderers (5.5) ----
/// Closest two wanderers may be (or `4 * r`, if larger).
pub const WANDERER_MIN_DIST: f64 = 60.0;
/// Closest a wanderer may be to a constellation star (or `min_dist`, if larger).
pub const WANDERER_STAR_DIST: f64 = 40.0;
/// Darts thrown per batch; the best valid one in a batch is kept.
pub const WANDERER_DARTS: usize = 30;
pub const WANDERER_BATCHES: usize = 40;

/// Where every star currently is, plus what is in the way.
pub struct Placement {
    positions: Vec<Option<Point>>,
    radii: Vec<f64>,
    regions: Vec<Region>,
}

impl Placement {
    pub fn new(radii: Vec<f64>, regions: Vec<Region>) -> Self {
        Placement {
            positions: vec![None; radii.len()],
            radii,
            regions,
        }
    }

    /// Where star `i` is (the origin if it has not been placed).
    pub fn at(&self, i: usize) -> Point {
        self.positions[i].unwrap_or((0.0, 0.0))
    }

    /// Can star `who` sit at `p`? Inside the safe area, clear of reserved
    /// areas, and at least `min_dist` from every other placed star.
    pub fn position_ok(&self, who: usize, p: Point) -> bool {
        let r = self.radii[who];
        star_area().contains(p)
            && !self
                .regions
                .iter()
                .any(|region| region.blocks_point(p, r + REGION_PAD))
            && self
                .positions
                .iter()
                .enumerate()
                .all(|(other, pos)| match pos {
                    Some(q) if other != who => dist(p, *q) >= min_dist(r, self.radii[other]),
                    _ => true,
                })
    }
}

/// What happened to one constellation's stars.
pub struct Grown {
    pub placed: Vec<usize>,
    pub dropped: Vec<usize>,
    pub radius: f64,
}

/// Grow a stick-figure shape. `members` are star indices, brightest first.
pub fn grow(
    placement: &mut Placement,
    members: &[usize],
    anchor: Point,
    base_radius: f64,
    rng: &mut SeededRng,
    step_mult: f64,
) -> Grown {
    let mut grown = Grown {
        placed: Vec::new(),
        dropped: Vec::new(),
        radius: base_radius,
    };
    let mut incoming: Vec<f64> = Vec::new(); // arrival angle of each placed star
    let Some((&first, rest)) = members.split_first() else {
        return grown;
    };

    match find_start(placement, first, anchor) {
        Some(p) => {
            placement.positions[first] = Some(p);
            grown.placed.push(first);
            incoming.push(rng.range(0.0, std::f64::consts::TAU));
        }
        None => {
            grown.dropped.extend(members);
            return grown;
        }
    }

    for &star in rest {
        let mut enlargements = 0;
        loop {
            if let Some((p, angle)) =
                best_candidate(placement, star, &grown, &incoming, anchor, rng, step_mult)
            {
                placement.positions[star] = Some(p);
                grown.placed.push(star);
                incoming.push(angle);
                break;
            }
            if enlargements == MAX_ENLARGEMENTS {
                grown.dropped.push(star);
                break;
            }
            grown.radius *= ENLARGE_FACTOR;
            enlargements += 1;
        }
    }
    grown
}

/// The anchor itself, or the nearest free spot in rings around it.
fn find_start(placement: &Placement, star: usize, anchor: Point) -> Option<Point> {
    if placement.position_ok(star, anchor) {
        return Some(anchor);
    }
    for ring in 1..=12 {
        for k in 0..8 {
            let angle = k as f64 * std::f64::consts::FRAC_PI_4;
            let d = ring as f64 * 12.0;
            let p = (anchor.0 + d * angle.cos(), anchor.1 + d * angle.sin());
            if placement.position_ok(star, p) {
                return Some(p);
            }
        }
    }
    None
}

fn best_candidate(
    placement: &Placement,
    star: usize,
    grown: &Grown,
    incoming: &[f64],
    anchor: Point,
    rng: &mut SeededRng,
    step_mult: f64,
) -> Option<(Point, f64)> {
    let n = grown.placed.len();
    let roll = rng.unit();
    let parent_slot = if roll < PARENT_LAST {
        n - 1
    } else if roll < PARENT_LAST + PARENT_BEFORE_LAST {
        n.saturating_sub(2)
    } else if n > 2 {
        rng.below(n - 2)
    } else {
        n - 1
    };
    let parent = placement.at(grown.placed[parent_slot]);
    let turn_limit = TURN_LIMIT_DEG.to_radians();

    let mut best: Option<(f64, Point, f64)> = None;
    for _ in 0..WALK_CANDIDATES {
        // Draw both numbers every time so the random stream stays in step.
        let turn = (rng.normal() * TURN_SIGMA_DEG.to_radians()).clamp(-turn_limit, turn_limit);
        let step = (STEP_MIN + STEP_SPAN * rng.unit()) * step_mult;
        let angle = incoming[parent_slot] + turn;
        let p = (parent.0 + step * angle.cos(), parent.1 + step * angle.sin());
        if dist(p, anchor) > grown.radius || !placement.position_ok(star, p) {
            continue;
        }
        // Prefer the spot farthest from its own constellation's other stars.
        let spacing = grown
            .placed
            .iter()
            .map(|&other| dist(p, placement.at(other)))
            .fold(f64::INFINITY, f64::min);
        if best.is_none_or(|(best_spacing, _, _)| spacing > best_spacing) {
            best = Some((spacing, p, angle));
        }
    }
    best.map(|(_, p, angle)| (p, angle))
}

/// Mild spring relaxation. `locked` stars (the founding star) never move.
pub fn relax(
    placement: &mut Placement,
    members: &[usize],
    locked: &[usize],
    anchor: Point,
    radius: f64,
) {
    if members.len() < 2 {
        return;
    }
    let mut step = RELAX_STEP;
    for _ in 0..RELAX_ITERATIONS {
        let n = members.len() as f64;
        let center = (
            members.iter().map(|&i| placement.at(i).0).sum::<f64>() / n,
            members.iter().map(|&i| placement.at(i).1).sum::<f64>() / n,
        );
        for &star in members.iter().filter(|s| !locked.contains(s)) {
            let p = placement.at(star);
            let mut push = (RELAX_PULL * (center.0 - p.0), RELAX_PULL * (center.1 - p.1));
            for &other in members.iter().filter(|&&o| o != star) {
                let q = placement.at(other);
                let d = dist(p, q);
                let limit =
                    RELAX_REPEL_FACTOR * min_dist(placement.radii[star], placement.radii[other]);
                if d < limit && d > 1e-9 {
                    push.0 += (limit - d) * (p.0 - q.0) / d;
                    push.1 += (limit - d) * (p.1 - q.1) / d;
                }
            }
            let mut next = (p.0 + step * push.0, p.1 + step * push.1);
            let from_anchor = dist(next, anchor);
            if from_anchor > radius {
                // Clamp back onto the circle.
                next = (
                    anchor.0 + (next.0 - anchor.0) * radius / from_anchor,
                    anchor.1 + (next.1 - anchor.1) * radius / from_anchor,
                );
            }
            if placement.position_ok(star, next) {
                placement.positions[star] = Some(next);
            }
        }
        step *= RELAX_DECAY;
    }
}

/// Place loose stars in the leftover gaps with a seeded "best of 30 darts".
pub fn scatter_wanderers(
    placement: &mut Placement,
    wanderers: &[usize],
    edges: &[(usize, usize)],
    rng: &mut SeededRng,
) -> Result<(), SkyError> {
    let safe = star_area();
    let mut is_wanderer = vec![false; placement.radii.len()];
    for &w in wanderers {
        is_wanderer[w] = true;
    }
    for &star in wanderers {
        let r = placement.radii[star];
        let mut chosen = None;
        'batches: for _ in 0..WANDERER_BATCHES {
            let mut best: Option<(f64, Point)> = None;
            for _ in 0..WANDERER_DARTS {
                let p = (rng.range(safe.x0, safe.x1), rng.range(safe.y0, safe.y1));
                if !placement.position_ok(star, p) {
                    continue;
                }
                let mut nearest = f64::INFINITY;
                let mut fits = true;
                for (other, pos) in placement.positions.iter().enumerate() {
                    let Some(q) = pos else { continue };
                    if other == star {
                        continue;
                    }
                    let d = dist(p, *q);
                    let other_r = placement.radii[other];
                    let need = if is_wanderer[other] {
                        WANDERER_MIN_DIST.max(4.0 * r.max(other_r))
                    } else {
                        min_dist(r, other_r).max(WANDERER_STAR_DIST)
                    };
                    if d < need {
                        fits = false;
                        break;
                    }
                    nearest = nearest.min(d);
                }
                let clear_of_lines = edges.iter().all(|&(a, b)| {
                    point_segment_distance(p, placement.at(a), placement.at(b))
                        >= WANDERER_LINE_CLEARANCE + r
                });
                if fits && clear_of_lines && best.is_none_or(|(d, _)| nearest > d) {
                    best = Some((nearest, p));
                }
            }
            if let Some((_, p)) = best {
                chosen = Some(p);
                break 'batches;
            }
        }
        match chosen {
            Some(p) => placement.positions[star] = Some(p),
            None => {
                return Err(SkyError::Layout(
                    "crowded: no room left for a loose star".to_string(),
                ));
            }
        }
    }
    Ok(())
}
