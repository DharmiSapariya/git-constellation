//! The debug picture: valid SVG, right element counts, a real PNG with real pixels.

mod common;

use std::path::PathBuf;

use common::{FIXTURES, load_fixture};
use git_constellation::export::svg_to_png;
use git_constellation::layout::debug::debug_svg;
use git_constellation::lint::lint;
use git_constellation::model::LabelMode;
use git_constellation::{Layout, Options, SkyModel, plan_sky};

fn planned(name: &str) -> (SkyModel, Layout) {
    let options = Options {
        labels: LabelMode::Major,
        ..Options::default()
    };
    plan_sky(&load_fixture(name), &options).expect("fixture lays out")
}

fn count(haystack: &str, needle: &str) -> usize {
    haystack.matches(needle).count()
}

fn png_size(bytes: &[u8]) -> (u32, u32) {
    assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n", "PNG signature");
    let read =
        |at: usize| u32::from_be_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]]);
    (read(16), read(20))
}

fn pixel(pixmap: &tiny_skia::Pixmap, x: f64, y: f64) -> (u8, u8, u8, u8) {
    let p = pixmap
        .pixel(x.round() as u32, y.round() as u32)
        .expect("pixel in range");
    (p.red(), p.green(), p.blue(), p.alpha())
}

#[test]
fn the_debug_svg_is_valid_and_complete_for_every_fixture() {
    for name in FIXTURES {
        let (model, layout) = planned(name);
        let svg = debug_svg(&model, &layout);
        usvg::Tree::from_str(&svg, &usvg::Options::default())
            .unwrap_or_else(|e| panic!("{name}: {e}"));
        assert!(svg.contains("viewBox=\"0 0 1200 600\""), "{name}");
        assert!(
            !svg.contains("NaN") && !svg.contains("inf"),
            "{name}: bad numbers"
        );
        assert!(
            !svg.contains("<script") && !svg.contains("<text"),
            "{name}: no scripts or text"
        );

        let edges: usize = layout.constellations.iter().map(|c| c.edges.len()).sum();
        assert_eq!(count(&svg, "<line "), edges, "{name}: one line per edge");
        assert_eq!(
            count(&svg, "#ff4d4d"),
            layout.labels.len(),
            "{name}: one red box per label"
        );
        // Circles: stars + one faint circle per constellation + the dashed moon area.
        let circles = model.stars.len() + layout.constellations.len() + 1;
        assert_eq!(count(&svg, "<circle "), circles, "{name}");
        assert_eq!(
            count(&svg, "stroke-dasharray"),
            layout.regions.len(),
            "{name}: reserved areas are dashed"
        );
    }
}

#[test]
fn numbers_in_the_svg_have_at_most_two_decimals() {
    let (model, layout) = planned("medium");
    let svg = debug_svg(&model, &layout);
    for attribute in ["cx=\"", "cy=\"", "x1=\"", "y2=\"", "width=\"", "r=\""] {
        for part in svg.split(attribute).skip(1) {
            let value = part.split('"').next().expect("closing quote");
            if let Some((_, decimals)) = value.split_once('.') {
                assert!(decimals.len() <= 2, "{attribute}{value}");
            }
        }
    }
}

#[test]
fn the_png_has_the_right_signature_and_size_at_both_scales() {
    let (model, layout) = planned("small");
    let svg = debug_svg(&model, &layout);
    assert_eq!(png_size(&svg_to_png(&svg, 1.0).unwrap()), (1200, 600));
    assert_eq!(png_size(&svg_to_png(&svg, 2.0).unwrap()), (2400, 1200));
}

#[test]
fn rendering_is_deterministic_byte_for_byte() {
    let (model, layout) = planned("medium");
    let svg = debug_svg(&model, &layout);
    assert_eq!(svg, debug_svg(&model, &layout));
    assert_eq!(
        svg_to_png(&svg, 1.0).unwrap(),
        svg_to_png(&svg, 1.0).unwrap()
    );
}

#[test]
fn the_pixels_show_a_dark_sky_with_bright_stars() {
    let (model, layout) = planned("medium");
    let png = svg_to_png(&debug_svg(&model, &layout), 1.0).unwrap();
    let pixmap = tiny_skia::Pixmap::decode_png(&png).expect("PNG decodes");
    assert_eq!((pixmap.width(), pixmap.height()), (1200, 600));
    // The background (#0b1020) in a corner away from everything.
    assert_eq!(pixel(&pixmap, 600.0, 20.0), (11, 16, 32, 255));
    // Each star's center is bright, not background.
    for (i, p) in layout.positions.iter().enumerate() {
        let (r, g, b, a) = pixel(&pixmap, p.0, p.1);
        assert_eq!(a, 255);
        assert!(
            u32::from(r) + u32::from(g) + u32::from(b) > 300,
            "star {i} center is {:?}",
            (r, g, b)
        );
    }
    // The dashed moon area is drawn: some gray pixel exists on its rim.
    let rim = pixel(&pixmap, 1090.0 + 90.0, 112.0);
    assert!(
        rim != (11, 16, 32, 255),
        "the moon area outline is drawn at {rim:?}"
    );
    // The safe margin outline (teal) is drawn along x = 40.
    let margin = pixel(&pixmap, 40.0, 300.0);
    assert!(margin.1 > margin.0, "teal margin line, got {margin:?}");
}

#[test]
fn a_broken_svg_is_a_friendly_error_not_a_panic() {
    let error = svg_to_png("this is not svg", 1.0).unwrap_err().to_string();
    assert!(
        error.contains("rendering failed") && error.contains("SVG"),
        "{error}"
    );
    assert!(
        svg_to_png(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"0\" height=\"0\"/>",
            1.0
        )
        .is_err()
    );
}

fn previews_dir() -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("debug-previews");
    std::fs::create_dir_all(&dir).expect("temp folder");
    dir
}

#[test]
fn previews_for_every_fixture_can_be_written() {
    for name in FIXTURES {
        let (model, layout) = planned(name);
        let png = svg_to_png(&debug_svg(&model, &layout), 1.0).unwrap();
        let path = previews_dir().join(format!("{name}.png"));
        std::fs::write(&path, &png).unwrap();
        assert!(std::fs::metadata(&path).unwrap().len() > 500, "{name}");
    }
}

/// Regenerate the review pictures and linter reports in docs/preview:
/// `cargo test --test debug_renderer -- --ignored`
#[test]
#[ignore = "writes files into docs/preview"]
fn regenerate_docs_previews() {
    let folder = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("docs")
        .join("preview");
    std::fs::create_dir_all(&folder).unwrap();
    let mut combined = String::new();
    for name in FIXTURES {
        let (model, layout) = planned(name);
        let png = svg_to_png(&debug_svg(&model, &layout), 1.0).unwrap();
        std::fs::write(folder.join(format!("debug-{name}.png")), png).unwrap();
        let report = lint(&model, &layout);
        combined.push_str(&format!(
            "=== {name}: {} stars, {} constellations, {} loose, layout try #{}, scale {:.2}{}\n{}\n",
            model.stars.len(),
            layout.constellations.len(),
            layout.wanderers.len(),
            layout.attempt + 1,
            layout.scale,
            layout.star_cap.map_or(String::new(), |c| format!(", star cap lowered to {c}")),
            report.to_text()
        ));
    }
    std::fs::write(folder.join("lint-report.txt"), combined).unwrap();
}
