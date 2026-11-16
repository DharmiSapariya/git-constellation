//! Label placement: text boxes that avoid stars, lines, each other and the edges.

use serde::{Deserialize, Serialize};

pub use super::text::*;

use super::{Layout, Rect, Region, dist, safe_rect, segment_hits_rect, star_radius};
use crate::model::{LabelMode, Point, SkyModel, Tier};

/// Gap between a star's edge and its label.
pub const LABEL_OFFSET: f64 = 10.0;
/// How much farther each retry pushes a label away.
pub const LABEL_RETRY_STEP: f64 = 7.0;
pub const LABEL_ATTEMPTS: usize = 3;
/// Search grid for a constellation name that found no tidy spot.
pub const SEARCH_STEP: f64 = 10.0;
pub const SEARCH_RADIUS: f64 = 260.0;
/// Star glow radii as multiples of the star radius: tight (a hard no-go) and wide (preferred clear).
pub const TIGHT_HALO: f64 = 1.8;
pub const WIDE_HALO: f64 = 3.4;
/// The founding star's ring (2.3) is `2.6 * r` wide.
pub const FOUNDING_RING: f64 = 2.6;
pub const CAPTION: &str = "a young sky";
pub const FIRST_LIGHT: &str = "first light";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LabelKind {
    /// The name of constellation `.0`.
    Constellation(usize),
    /// The repo name of star `.0`.
    Star(usize),
    /// The tiny "first light" note beside the founding star `.0`.
    FirstLight(usize),
    /// "a young sky".
    Caption,
}

/// A text label's reserved box. Drawing the text itself comes in a later milestone.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Label {
    pub kind: LabelKind,
    /// The text as it will be drawn (constellation names are uppercase).
    pub text: String,
    pub rect: Rect,
    pub font_size: f64,
}

fn text_rect(text: &str, font_size: f64, spaced: bool) -> (f64, f64) {
    (
        text_width(text, font_size, spaced) + 2.0 * BOX_PAD,
        font_size * LINE_HEIGHT + 2.0 * BOX_PAD,
    )
}

/// A constellation's name must be at most this share of the distance to the
/// nearest star of any other constellation, so no reader has to guess whose name it is.
pub const NAME_OWNER_RATIO: f64 = 0.8;

/// Is a constellation name clearly nearer its own stars than any other group's?
pub fn name_belongs(rect: &Rect, own: &[Point], foreign: &[Point]) -> bool {
    let nearest = |points: &[Point]| {
        points
            .iter()
            .map(|&p| rect.distance_to_point(p))
            .fold(f64::INFINITY, f64::min)
    };
    let theirs = nearest(foreign);
    theirs.is_infinite() || nearest(own) <= NAME_OWNER_RATIO * theirs
}

/// Everything a label must stay clear of.
pub struct Obstacles {
    /// Star centers with their hard keep-out radius.
    pub stars: Vec<(Point, f64)>,
    /// Star centers with the wide glow radius (a soft preference).
    pub wide: Vec<(Point, f64)>,
    pub segments: Vec<(Point, Point)>,
    pub regions: Vec<Region>,
}

impl Obstacles {
    pub fn from_layout(model: &SkyModel, layout: &Layout) -> Obstacles {
        let mut stars = Vec::new();
        let mut wide = Vec::new();
        for (i, star) in model.stars.iter().enumerate() {
            let r = star_radius(star.tier);
            let hard = if star.founding {
                FOUNDING_RING * r + 1.0
            } else {
                TIGHT_HALO * r
            };
            stars.push((layout.positions[i], hard));
            wide.push((layout.positions[i], WIDE_HALO * r));
        }
        let segments = layout
            .constellations
            .iter()
            .flat_map(|c| {
                c.edges
                    .iter()
                    .map(|&(a, b)| (layout.positions[a], layout.positions[b]))
            })
            .collect();
        Obstacles {
            stars,
            wide,
            segments,
            regions: layout.regions.clone(),
        }
    }

    /// How many hard rules this box breaks: stars, lines, other boxes,
    /// reserved areas and the safe margin. Zero means a clean spot.
    pub fn conflicts(&self, rect: &Rect, others: &[Rect]) -> usize {
        let mut count = 0;
        count += self
            .stars
            .iter()
            .filter(|(c, r)| rect.distance_to_point(*c) < *r)
            .count();
        count += self
            .segments
            .iter()
            .filter(|(a, b)| segment_hits_rect(*a, *b, rect))
            .count();
        count += others
            .iter()
            .filter(|o| o.inflate(1.0).overlaps(rect))
            .count();
        count += self
            .regions
            .iter()
            .filter(|region| region.overlaps_rect(rect))
            .count();
        count += usize::from(!safe_rect().contains_rect(rect));
        count
    }

    /// A soft score: how much the box sits in star glows. Lower is nicer.
    fn glow(&self, rect: &Rect) -> f64 {
        self.wide
            .iter()
            .map(|(c, r)| (r - rect.distance_to_point(*c)).max(0.0))
            .sum()
    }
}

fn box_around(x_left: f64, y_top: f64, w: f64, h: f64) -> Rect {
    Rect {
        x0: x_left,
        y0: y_top,
        x1: x_left + w,
        y1: y_top + h,
    }
}

/// Eight spots around a star: right, left, above, below, then the diagonals.
fn star_spots(p: Point, offset: f64, w: f64, h: f64) -> [Rect; 8] {
    let diagonal = offset * std::f64::consts::FRAC_1_SQRT_2;
    [
        box_around(p.0 + offset, p.1 - h / 2.0, w, h),
        box_around(p.0 - offset - w, p.1 - h / 2.0, w, h),
        box_around(p.0 - w / 2.0, p.1 - offset - h, w, h),
        box_around(p.0 - w / 2.0, p.1 + offset, w, h),
        box_around(p.0 + diagonal, p.1 - diagonal - h, w, h),
        box_around(p.0 - diagonal - w, p.1 - diagonal - h, w, h),
        box_around(p.0 + diagonal, p.1 + diagonal, w, h),
        box_around(p.0 - diagonal - w, p.1 + diagonal, w, h),
    ]
}

/// Pick the best of some candidate boxes: no hard conflicts, then least glow.
fn best_clean(
    obstacles: &Obstacles,
    others: &[Rect],
    spots: &[Rect],
    accept: &dyn Fn(&Rect) -> bool,
) -> Option<Rect> {
    spots
        .iter()
        .filter(|rect| obstacles.conflicts(rect, others) == 0 && accept(rect))
        .min_by(|a, b| obstacles.glow(a).total_cmp(&obstacles.glow(b)))
        .copied()
}

/// Constellation names (and the caption) float near a preferred point; if the
/// usual spots are taken, search outward on a grid for the nearest clean box.
fn place_floating(
    obstacles: &Obstacles,
    others: &[Rect],
    preferred: &[Rect],
    w: f64,
    h: f64,
    accept: &dyn Fn(&Rect) -> bool,
) -> Option<Rect> {
    if let Some(rect) = best_clean(obstacles, others, preferred, accept) {
        return Some(rect);
    }
    let center = preferred
        .first()
        .map(|r| ((r.x0 + r.x1) / 2.0, (r.y0 + r.y1) / 2.0))?;
    let steps = (SEARCH_RADIUS / SEARCH_STEP) as i32;
    let mut spots: Vec<(f64, Rect)> = Vec::new();
    for ix in -steps..=steps {
        for iy in -steps..=steps {
            let c = (
                center.0 + f64::from(ix) * SEARCH_STEP,
                center.1 + f64::from(iy) * SEARCH_STEP,
            );
            spots.push((
                dist(c, center),
                box_around(c.0 - w / 2.0, c.1 - h / 2.0, w, h),
            ));
        }
    }
    spots.sort_by(|a, b| a.0.total_cmp(&b.0));
    spots
        .into_iter()
        .map(|(_, r)| r)
        .find(|r| obstacles.conflicts(r, others) == 0 && accept(r))
}

/// Place every label. Returns the labels and how many were left out.
pub fn place_labels(model: &SkyModel, layout: &Layout, mode: LabelMode) -> (Vec<Label>, usize) {
    let obstacles = Obstacles::from_layout(model, layout);
    let mut labels: Vec<Label> = Vec::new();
    let mut boxes: Vec<Rect> = Vec::new();
    let mut dropped = 0;

    if model.is_young() {
        let low = layout
            .positions
            .iter()
            .map(|p| p.1)
            .fold(super::CANVAS_HEIGHT / 2.0, f64::max);
        let text = CAPTION.to_string();
        let (w, h) = text_rect(&text, CONSTELLATION_FONT, false);
        let spots: Vec<Rect> = [34.0, 44.0, 56.0]
            .iter()
            .map(|dy| box_around(super::CANVAS_WIDTH / 2.0 - w / 2.0, low + dy, w, h))
            .collect();
        match place_floating(&obstacles, &boxes, &spots, w, h, &|_| true) {
            Some(rect) => {
                boxes.push(rect);
                labels.push(Label {
                    kind: LabelKind::Caption,
                    text,
                    rect,
                    font_size: CONSTELLATION_FONT,
                });
            }
            None => dropped += 1,
        }
    }
    if mode == LabelMode::Off {
        return (labels, dropped);
    }

    for (ci, constellation) in model.constellations.iter().enumerate() {
        let placed = &layout.constellations[ci];
        if placed.stars.is_empty() {
            continue;
        }
        let text = constellation.name.to_uppercase();
        let (w, h) = text_rect(&text, CONSTELLATION_FONT, true);
        let points: Vec<Point> = placed.stars.iter().map(|&i| layout.positions[i]).collect();
        let bottom = placed
            .stars
            .iter()
            .map(|&i| layout.positions[i].1 + star_radius(model.stars[i].tier))
            .fold(f64::MIN, f64::max);
        let top = placed
            .stars
            .iter()
            .map(|&i| layout.positions[i].1 - star_radius(model.stars[i].tier))
            .fold(f64::MAX, f64::min);
        let cx = points.iter().map(|p| p.0).sum::<f64>() / points.len() as f64;
        let mut spots = Vec::new();
        for attempt in 0..LABEL_ATTEMPTS {
            let extra = LABEL_RETRY_STEP * attempt as f64;
            for shift in [0.0, -40.0, 40.0, -80.0, 80.0, -120.0, 120.0] {
                spots.push(box_around(
                    cx + shift - w / 2.0,
                    bottom + 14.0 + extra,
                    w,
                    h,
                ));
                spots.push(box_around(
                    cx + shift - w / 2.0,
                    top - 14.0 - extra - h,
                    w,
                    h,
                ));
            }
        }
        spots.push(box_around(
            placed.anchor.0 - w / 2.0,
            placed.anchor.1 + placed.radius - h,
            w,
            h,
        ));
        let own: Vec<Point> = placed.stars.iter().map(|&i| layout.positions[i]).collect();
        let foreign: Vec<Point> = layout
            .constellations
            .iter()
            .enumerate()
            .filter(|(other, _)| *other != ci)
            .flat_map(|(_, c)| c.stars.iter().map(|&i| layout.positions[i]))
            .collect();
        let belongs = |rect: &Rect| name_belongs(rect, &own, &foreign);
        match place_floating(&obstacles, &boxes, &spots, w, h, &belongs) {
            Some(rect) => {
                boxes.push(rect);
                labels.push(Label {
                    kind: LabelKind::Constellation(ci),
                    text,
                    rect,
                    font_size: CONSTELLATION_FONT,
                });
            }
            None => dropped += 1,
        }
    }

    for (i, star) in model.stars.iter().enumerate().filter(|(_, s)| s.founding) {
        let text = FIRST_LIGHT.to_string();
        let (w, h) = text_rect(&text, SMALL_FONT, false);
        let r = star_radius(star.tier);
        let found = (0..LABEL_ATTEMPTS).find_map(|attempt| {
            let offset = FOUNDING_RING * r + 6.0 + LABEL_RETRY_STEP * attempt as f64;
            best_clean(
                &obstacles,
                &boxes,
                &star_spots(layout.positions[i], offset, w, h),
                &|_| true,
            )
        });
        match found {
            Some(rect) => {
                boxes.push(rect);
                labels.push(Label {
                    kind: LabelKind::FirstLight(i),
                    text,
                    rect,
                    font_size: SMALL_FONT,
                });
            }
            None => dropped += 1,
        }
    }

    // Repo names: brightest stars first, so they get the best spots.
    for (i, star) in model.stars.iter().enumerate() {
        let wanted = match mode {
            LabelMode::All => true,
            LabelMode::Major => star.tier >= Tier::Four,
            LabelMode::Off => false,
        };
        if !wanted {
            continue;
        }
        let text = truncate_name(&star.repo);
        let (w, h) = text_rect(&text, REPO_FONT, false);
        let r = star_radius(star.tier);
        let found = (0..LABEL_ATTEMPTS).find_map(|attempt| {
            let offset = r + LABEL_OFFSET + LABEL_RETRY_STEP * attempt as f64;
            best_clean(
                &obstacles,
                &boxes,
                &star_spots(layout.positions[i], offset, w, h),
                &|_| true,
            )
        });
        match found {
            Some(rect) => {
                boxes.push(rect);
                labels.push(Label {
                    kind: LabelKind::Star(i),
                    text,
                    rect,
                    font_size: REPO_FONT,
                });
            }
            None => dropped += 1,
        }
    }
    (labels, dropped)
}
