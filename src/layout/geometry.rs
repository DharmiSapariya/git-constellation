//! Geometry shared by the layout and the linter: the canvas, reserved areas,
//! rectangles, distances and star sizes.

use serde::{Deserialize, Serialize};

use crate::model::{Point, Tier};

// ---- Canvas (2.1) ----
pub const CANVAS_WIDTH: f64 = 1200.0;
pub const CANVAS_HEIGHT: f64 = 600.0;
/// No star center, label or character may sit closer than this to an edge.
pub const SAFE_MARGIN: f64 = 40.0;

// ---- Stars (2.3, 5.2) ----
/// Star radius in pixels for tiers 1 to 5.
pub const STAR_RADII: [f64; 5] = [2.2, 3.2, 4.6, 6.4, 8.6];
/// Stars are kept this much farther in than the margin, so glows are not clipped.
pub const STAR_EDGE_PAD: f64 = 14.0;
/// Breathing room between two stars' edges (2.7).
pub const MIN_STAR_GAP: f64 = 28.0;

// ---- Reserved areas (5.1) ----
/// Top-left title and stats.
pub const HUD_RECT: Rect = Rect {
    x0: 24.0,
    y0: 24.0,
    x1: 330.0,
    y1: 82.0,
};
/// Where the moon character lives.
pub const MOON_CENTER: Point = (1090.0, 112.0);
pub const MOON_RESERVED_RADIUS: f64 = 90.0;
/// Bottom-left legend, only when labels are on.
pub const LEGEND_RECT: Rect = Rect {
    x0: 24.0,
    y0: 560.0,
    x1: 360.0,
    y1: 590.0,
};

pub fn dist(a: Point, b: Point) -> f64 {
    ((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2)).sqrt()
}

pub fn star_radius(tier: Tier) -> f64 {
    STAR_RADII[usize::from(tier.number()) - 1]
}

/// Closest allowed distance between two star centers: the spec's
/// `4 * max(r) + 14`, but never less than the 28 px breathing room (2.7).
pub fn min_dist(radius_a: f64, radius_b: f64) -> f64 {
    (4.0 * radius_a.max(radius_b) + 14.0).max(radius_a + radius_b + MIN_STAR_GAP)
}

/// An axis-aligned rectangle.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Rect {
    pub x0: f64,
    pub y0: f64,
    pub x1: f64,
    pub y1: f64,
}

impl Rect {
    pub fn width(&self) -> f64 {
        self.x1 - self.x0
    }
    pub fn height(&self) -> f64 {
        self.y1 - self.y0
    }
    pub fn inflate(&self, pad: f64) -> Rect {
        Rect {
            x0: self.x0 - pad,
            y0: self.y0 - pad,
            x1: self.x1 + pad,
            y1: self.y1 + pad,
        }
    }
    pub fn contains(&self, p: Point) -> bool {
        p.0 >= self.x0 && p.0 <= self.x1 && p.1 >= self.y0 && p.1 <= self.y1
    }
    pub fn contains_rect(&self, other: &Rect) -> bool {
        other.x0 >= self.x0 && other.x1 <= self.x1 && other.y0 >= self.y0 && other.y1 <= self.y1
    }
    /// True when the interiors overlap (touching edges do not count).
    pub fn overlaps(&self, other: &Rect) -> bool {
        self.x0 < other.x1 && other.x0 < self.x1 && self.y0 < other.y1 && other.y0 < self.y1
    }
    /// Distance from a point to the rectangle (0 when inside).
    pub fn distance_to_point(&self, p: Point) -> f64 {
        let dx = (self.x0 - p.0).max(0.0).max(p.0 - self.x1);
        let dy = (self.y0 - p.1).max(0.0).max(p.1 - self.y1);
        (dx * dx + dy * dy).sqrt()
    }
}

/// The area stars, labels and anchors must stay inside.
pub fn safe_rect() -> Rect {
    Rect {
        x0: SAFE_MARGIN,
        y0: SAFE_MARGIN,
        x1: CANVAS_WIDTH - SAFE_MARGIN,
        y1: CANVAS_HEIGHT - SAFE_MARGIN,
    }
}

/// Where star centers may go: the safe area, pulled in a little more.
pub fn star_area() -> Rect {
    safe_rect().inflate(-STAR_EDGE_PAD)
}

/// A reserved area where no star, label or anchor may go.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Region {
    Rect(Rect),
    Circle { center: Point, radius: f64 },
}

impl Region {
    /// Is a point (a star center, grown by `pad`) inside this region?
    pub fn blocks_point(&self, p: Point, pad: f64) -> bool {
        match self {
            Region::Rect(rect) => rect.inflate(pad).contains(p),
            Region::Circle { center, radius } => dist(p, *center) < radius + pad,
        }
    }

    pub fn overlaps_rect(&self, rect: &Rect) -> bool {
        match self {
            Region::Rect(own) => own.overlaps(rect),
            Region::Circle { center, radius } => rect.distance_to_point(*center) < *radius,
        }
    }
}

pub fn reserved_regions(show_legend: bool) -> Vec<Region> {
    let mut regions = vec![
        Region::Rect(HUD_RECT),
        Region::Circle {
            center: MOON_CENTER,
            radius: MOON_RESERVED_RADIUS,
        },
    ];
    if show_legend {
        regions.push(Region::Rect(LEGEND_RECT));
    }
    regions
}

/// Does the segment `ab` touch the rectangle? (Liang-Barsky clipping.)
pub fn segment_hits_rect(a: Point, b: Point, rect: &Rect) -> bool {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let (mut t0, mut t1) = (0.0_f64, 1.0_f64);
    for (p, q) in [
        (-dx, a.0 - rect.x0),
        (dx, rect.x1 - a.0),
        (-dy, a.1 - rect.y0),
        (dy, rect.y1 - a.1),
    ] {
        if p.abs() < 1e-12 {
            if q < 0.0 {
                return false;
            }
        } else {
            let t = q / p;
            if p < 0.0 {
                t0 = t0.max(t);
            } else {
                t1 = t1.min(t);
            }
            if t0 > t1 {
                return false;
            }
        }
    }
    true
}
