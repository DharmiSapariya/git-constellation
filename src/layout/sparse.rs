//! Small skies: where their constellations sit and how far their picture may grow, so that a
//! beginner's handful of repos looks composed rather than scattered.

/// Skies with at most this many stars get hand-composed anchor slots instead of random ones
/// (random placement is a lottery with so few groups), and the relaxed coverage below.
pub const SMALL_SKY_MAX_STARS: usize = 16;
/// Slots exist for 1 to 4 constellations (as shares of width and height), brightest first.
/// They sit near the middle, close enough to read as one picture: two stagger diagonally,
/// three form a triangle, four a loose square.
pub const SMALL_SKY_SLOTS: [&[(f64, f64)]; 4] = [
    &[(0.44, 0.52)],
    &[(0.38, 0.40), (0.62, 0.62)],
    &[(0.38, 0.38), (0.62, 0.38), (0.50, 0.67)],
    &[(0.38, 0.37), (0.62, 0.37), (0.38, 0.69), (0.62, 0.69)],
];
/// Each slot is nudged by up to this many pixels, so seeds still differ.
pub const SMALL_SKY_JITTER: f64 = 20.0;
/// The gap kept between two constellations' circles in a small sky (a full sky keeps 30).
pub const SMALL_SKY_ANCHOR_GAP: f64 = 10.0;
/// Constellations of a small sky grow with longer steps, so three stars still make a shape.
pub const SMALL_SKY_STEP_MULT: f64 = 1.3;
/// A small sky may cover less of the canvas than a full one (as shares of width and height) and
/// may be scaled up further to reach that.
pub const SMALL_SKY_COVERAGE_WIDTH: (f64, f64) = (0.45, 0.85);
pub const SMALL_SKY_COVERAGE_HEIGHT: (f64, f64) = (0.38, 0.80);
pub const SMALL_SKY_MAX_SCALE_UP: f64 = 1.6;

/// The coverage ranges and the largest scale-up for a sky with this many stars.
pub fn coverage_for(star_count: usize) -> ((f64, f64), (f64, f64), f64) {
    if star_count <= SMALL_SKY_MAX_STARS {
        (
            SMALL_SKY_COVERAGE_WIDTH,
            SMALL_SKY_COVERAGE_HEIGHT,
            SMALL_SKY_MAX_SCALE_UP,
        )
    } else {
        (
            super::COVERAGE_WIDTH,
            super::COVERAGE_HEIGHT,
            super::MAX_SCALE_UP,
        )
    }
}

/// Gathering: in a small sky each constellation is moved (as a rigid group) toward the middle of
/// the others until the most distant pair of constellations has stars this close...
pub const GATHER_TARGET_GAP: f64 = 190.0;
/// ...but no two constellations ever get stars closer than this (names and lines need room).
pub const GATHER_MIN_GAP: f64 = 110.0;
/// Each try moves a constellation this share of the way to the middle, at most this many tries.
pub const GATHER_STEP: f64 = 0.12;
pub const GATHER_ITERATIONS: usize = 16;
