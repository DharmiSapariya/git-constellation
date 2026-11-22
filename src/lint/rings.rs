//! Hard checks on what surrounds a star: labels must stay outside its ring, and its flare must
//! stop before every label.

use super::Check;
use crate::art::labels::drawn_labels;
use crate::layout::Layout;
use crate::layout::geometry::{FOUNDING_RING, FRESH_RING, Rect, star_radius};
use crate::model::{LabelMode, SkyModel};

/// Half the stroke width of a paper flare stroke, added around its segment.
const STROKE_HALF: f64 = 0.4;

/// The value of the number attribute ` name="..."` in a tag's text.
fn number_attribute(tag: &str, name: &str) -> Option<f64> {
    let needle = format!(" {name}=\"");
    let start = tag.find(&needle)? + needle.len();
    let end = tag[start..].find('"')?;
    tag[start..start + end].parse().ok()
}

/// The boxes covered by every flare in the SVG: the thin glowing rects on dark themes (turned
/// 90 degrees when they say so) and the ink strokes of paper stars.
pub fn flare_boxes(svg: &str) -> Vec<Rect> {
    let mut boxes = Vec::new();
    for chunk in svg.split("<rect ").skip(1) {
        // The split removed the space before the first attribute; put it back.
        let tag = format!(" {}", chunk.split('>').next().unwrap_or(""));
        if !tag.contains("fill=\"url(#flare") {
            continue;
        }
        let (Some(x), Some(y), Some(w), Some(h)) = (
            number_attribute(&tag, "x"),
            number_attribute(&tag, "y"),
            number_attribute(&tag, "width"),
            number_attribute(&tag, "height"),
        ) else {
            continue;
        };
        let (cx, cy) = (x + w / 2.0, y + h / 2.0);
        let (hw, hh) = if tag.contains("rotate(90 ") {
            (h / 2.0, w / 2.0)
        } else {
            (w / 2.0, h / 2.0)
        };
        boxes.push(Rect {
            x0: cx - hw,
            y0: cy - hh,
            x1: cx + hw,
            y1: cy + hh,
        });
    }
    // On paper the flare is a path; the stars layer has no other path without a curve.
    let layer_start = svg.find("<g id=\"stars\">").unwrap_or(0);
    for chunk in svg[layer_start..].split("<path d=\"").skip(1) {
        let d = chunk.split('"').next().unwrap_or("");
        if d.contains('Q') || d.contains('Z') {
            continue;
        }
        if !chunk.contains("stroke-linecap") {
            continue;
        }
        let numbers: Vec<f64> = d
            .split_whitespace()
            .filter_map(|t| t.parse().ok())
            .collect();
        for segment in numbers.chunks_exact(4) {
            boxes.push(Rect {
                x0: segment[0].min(segment[2]) - STROKE_HALF,
                y0: segment[1].min(segment[3]) - STROKE_HALF,
                x1: segment[0].max(segment[2]) + STROKE_HALF,
                y1: segment[1].max(segment[3]) + STROKE_HALF,
            });
        }
        // Only the first path after the stars layer opens can be a flare of a calm star; the
        // layer's later paths are checked the same way, so keep going.
    }
    boxes
}

/// The radius of the ring drawn around a star, if it has one.
pub fn ring_radius(star: &crate::model::Star) -> Option<f64> {
    let r = star_radius(star.tier);
    if star.founding {
        Some(FOUNDING_RING * r)
    } else if star.fresh && !star.dormant {
        Some(FRESH_RING * r)
    } else {
        None
    }
}

/// "labels clear of rings" and "flares clear of labels", for the labels that are drawn.
pub fn ring_flare_checks(
    svg: &str,
    model: &SkyModel,
    layout: &Layout,
    mode: LabelMode,
) -> Vec<Check> {
    let shown = drawn_labels(&layout.labels, mode);
    let mut touching_rings = Vec::new();
    for (star, center) in model.stars.iter().zip(&layout.positions) {
        let Some(radius) = ring_radius(star) else {
            continue;
        };
        for label in shown {
            if label.rect.distance_to_point(*center) < radius {
                touching_rings.push(format!("\"{}\" over the ring of {}", label.text, star.repo));
            }
        }
    }
    let flares = flare_boxes(svg);
    let mut touching_flares = Vec::new();
    for flare in &flares {
        for label in shown {
            if flare.overlaps(&label.rect) {
                touching_flares.push(format!("\"{}\"", label.text));
            }
        }
    }
    vec![
        Check {
            name: "labels clear of rings",
            passed: touching_rings.is_empty(),
            detail: if touching_rings.is_empty() {
                format!("{} labels, none touch a ring", shown.len())
            } else {
                touching_rings.join("; ")
            },
        },
        Check {
            name: "flares clear of labels",
            passed: touching_flares.is_empty(),
            detail: if touching_flares.is_empty() {
                format!("{} flare arms, none touch a label", flares.len())
            } else {
                format!("flare arms touch {}", touching_flares.join(", "))
            },
        },
    ]
}
