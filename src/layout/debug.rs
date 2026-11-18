//! Debug renderer: shows only composition, no art. Dark background, circles for
//! stars, straight lines, red label boxes, dashed gray reserved areas, safe margin.

use super::{CANVAS_HEIGHT, CANVAS_WIDTH, Layout, Region, SAFE_MARGIN, star_radius};
use crate::model::SkyModel;
use crate::svg::fmt as num;

/// Fill colors for constellations (loose stars are white).
const PALETTE: [&str; 7] = [
    "#ffd166", "#7ed6a5", "#6ec1ff", "#ff9bb3", "#c3a6ff", "#ffa94d", "#9be3e0",
];

/// Build the debug picture as SVG text.
pub fn debug_svg(model: &SkyModel, layout: &Layout) -> String {
    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {} {}\" width=\"{}\" height=\"{}\">\n",
        num(CANVAS_WIDTH),
        num(CANVAS_HEIGHT),
        num(CANVAS_WIDTH),
        num(CANVAS_HEIGHT)
    );
    svg.push_str("<rect width=\"100%\" height=\"100%\" fill=\"#0b1020\"/>\n");
    svg.push_str(&format!(
        "<rect x=\"{m}\" y=\"{m}\" width=\"{w}\" height=\"{h}\" fill=\"none\" stroke=\"#2f9e8f\" stroke-width=\"1\"/>\n",
        m = num(SAFE_MARGIN),
        w = num(CANVAS_WIDTH - 2.0 * SAFE_MARGIN),
        h = num(CANVAS_HEIGHT - 2.0 * SAFE_MARGIN)
    ));

    for region in &layout.regions {
        let dashed = "fill=\"none\" stroke=\"#8a8f98\" stroke-width=\"1\" stroke-dasharray=\"6 4\"";
        match region {
            Region::Rect(r) => svg.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" {dashed}/>\n",
                num(r.x0),
                num(r.y0),
                num(r.width()),
                num(r.height())
            )),
            Region::Circle { center, radius } => svg.push_str(&format!(
                "<circle cx=\"{}\" cy=\"{}\" r=\"{}\" {dashed}/>\n",
                num(center.0),
                num(center.1),
                num(*radius)
            )),
        }
    }

    // Faint constellation circles, to judge how much room each one took.
    for (ci, placed) in layout.constellations.iter().enumerate() {
        svg.push_str(&format!(
            "<circle cx=\"{}\" cy=\"{}\" r=\"{}\" fill=\"none\" stroke=\"{}\" stroke-opacity=\"0.25\" stroke-width=\"1\"/>\n",
            num(placed.anchor.0), num(placed.anchor.1), num(placed.radius), PALETTE[ci % PALETTE.len()]
        ));
    }

    for placed in &layout.constellations {
        for &(a, b) in &placed.edges {
            let (p, q) = (layout.positions[a], layout.positions[b]);
            svg.push_str(&format!(
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#cfd6ff\" stroke-opacity=\"0.8\" stroke-width=\"1\"/>\n",
                num(p.0), num(p.1), num(q.0), num(q.1)
            ));
        }
    }

    for (i, star) in model.stars.iter().enumerate() {
        let color = layout
            .constellations
            .iter()
            .position(|c| c.stars.contains(&i))
            .map_or("#ffffff", |ci| PALETTE[ci % PALETTE.len()]);
        let p = layout.positions[i];
        let ring = if star.founding {
            " stroke=\"#ffffff\" stroke-width=\"1.5\""
        } else {
            ""
        };
        svg.push_str(&format!(
            "<circle cx=\"{}\" cy=\"{}\" r=\"{}\" fill=\"{color}\"{ring}/>\n",
            num(p.0),
            num(p.1),
            num(star_radius(star.tier))
        ));
    }

    for label in &layout.labels {
        let width = if matches!(label.kind, super::labels::LabelKind::Constellation(_)) {
            "1.6"
        } else {
            "1"
        };
        svg.push_str(&format!(
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"none\" stroke=\"#ff4d4d\" stroke-width=\"{width}\"/>\n",
            num(label.rect.x0), num(label.rect.y0), num(label.rect.width()), num(label.rect.height())
        ));
    }
    svg.push_str("</svg>\n");
    svg
}
