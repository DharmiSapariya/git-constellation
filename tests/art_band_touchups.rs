//! Milestone 5, sub-step A: the softer Milky Way, its dust lanes, the stronger paper theme.

mod common;

use common::load_fixture;
use git_constellation::art::background::{FIBER_COUNT, paper_fibers};
use git_constellation::art::milkyway::{
    LANE_COUNT, MIN_DENSITY, band_lanes, centerline, cloud_opacity, densities, dot_counts,
};
use git_constellation::art::{ArtOptions, background_svg};
use git_constellation::export::svg_to_png;
use git_constellation::model::ThemeName;
use git_constellation::theme::{MonoVariant, Rgb, Theme, luma, parse_hex};
use git_constellation::{Options, build_sky};

/// A pixel this close to the band's centerline counts as "in the band".
const BAND_REACH: f64 = 25.0;
/// Only the middle 60 percent of the band is measured; its ends are thin and faint.
const BAND_MEASURE: (f64, f64) = (0.2, 0.8);
/// The band may be at most this many times as bright as the sky's lightest color.
const MAX_BAND_RATIO: f64 = 1.35;

fn medium_svg(theme: &Theme) -> String {
    let model = build_sky(&load_fixture("medium"), &Options::default()).expect("builds");
    background_svg(&model, theme, &ArtOptions::default(), &[])
}

/// Mean gamma-encoded luma of the band's middle, divided by the luma of the
/// theme's `sky_bottom` (its lightest sky color).
fn band_ratio(theme: &Theme) -> f64 {
    let png = svg_to_png(&medium_svg(theme), 1.0).expect("renders");
    let pixmap = tiny_skia::Pixmap::decode_png(&png).expect("decodes");
    let (width, height) = (pixmap.width() as usize, pixmap.height() as usize);
    let line = centerline(1200.0, 600.0, 400);
    let from = (BAND_MEASURE.0 * line.len() as f64) as usize;
    let to = (BAND_MEASURE.1 * line.len() as f64) as usize;
    // Mark every pixel within reach of a centerline sample (a small box around each).
    let mut in_band = vec![false; width * height];
    let reach = BAND_REACH as i64;
    for c in &line[from..to] {
        for dy in -reach..=reach {
            for dx in -reach..=reach {
                let (x, y) = (c.0.round() as i64 + dx, c.1.round() as i64 + dy);
                let inside = x >= 0 && y >= 0 && (x as usize) < width && (y as usize) < height;
                if inside && ((dx * dx + dy * dy) as f64).sqrt() <= BAND_REACH {
                    in_band[y as usize * width + x as usize] = true;
                }
            }
        }
    }
    let (mut sum, mut count) = (0.0, 0usize);
    for (i, p) in pixmap.pixels().iter().enumerate() {
        if in_band[i] {
            sum += luma(Rgb {
                r: p.red(),
                g: p.green(),
                b: p.blue(),
            });
            count += 1;
        }
    }
    assert!(count > 1000, "the band was found ({count} pixels)");
    let bottom = luma(parse_hex(theme.sky_bottom).expect("hex"));
    (sum / count as f64) / bottom
}

#[test]
fn the_band_is_not_much_brighter_than_the_sky_on_dark_themes() {
    let themes = [
        ("midnight", Theme::for_name(ThemeName::Midnight)),
        ("dusk", Theme::for_name(ThemeName::Dusk)),
        ("aurora", Theme::for_name(ThemeName::Aurora)),
        ("sakura", Theme::for_name(ThemeName::Sakura)),
        ("mono-dark", Theme::mono(MonoVariant::Dark)),
    ];
    for (name, theme) in &themes {
        assert!(theme.is_dark, "{name}");
        let ratio = band_ratio(theme);
        println!("band brightness ratio {name}: {ratio:.3} (limit {MAX_BAND_RATIO})");
        assert!(
            ratio <= MAX_BAND_RATIO,
            "{name}: the band is {ratio:.2}x the lightest sky color"
        );
    }
}

#[test]
fn the_busiest_week_is_much_more_opaque_and_much_denser_than_the_quietest() {
    let opacity_ratio = cloud_opacity(1.0, 1.0) / cloud_opacity(MIN_DENSITY, 1.0);
    assert!(opacity_ratio >= 2.5, "{opacity_ratio}");
    let counts = dot_counts(&[1.0, MIN_DENSITY]);
    assert!(
        counts[0] >= 3 * counts[1],
        "busy {} vs quiet {}",
        counts[0],
        counts[1]
    );
    assert!(densities(&[]).iter().all(|d| *d == MIN_DENSITY));
}

#[test]
fn six_dust_lanes_follow_the_band_and_are_seeded() {
    let lanes = band_lanes(5, 1200.0, 600.0);
    assert_eq!(lanes.len(), LANE_COUNT);
    assert_eq!(LANE_COUNT, 6);
    let line = centerline(1200.0, 600.0, 400);
    for lane in &lanes {
        assert_eq!((lane.rx, lane.ry), (90.0, 14.0));
        assert!((-90.0..=90.0).contains(&lane.angle_deg));
        let nearest = line
            .iter()
            .map(|p| ((p.0 - lane.cx).powi(2) + (p.1 - lane.cy).powi(2)).sqrt())
            .fold(f64::INFINITY, f64::min);
        assert!(
            nearest < 0.25 * 170.0 + 8.0,
            "a lane is {nearest:.0}px off the band"
        );
    }
    assert_eq!(lanes, band_lanes(5, 1200.0, 600.0));
    assert_ne!(lanes, band_lanes(6, 1200.0, 600.0));
}

#[test]
fn the_lanes_are_drawn_once_each_in_the_band_layer() {
    let svg = medium_svg(&Theme::for_name(ThemeName::Midnight));
    let band = &svg[svg.find("<g id=\"milkyway\">").expect("layer")..];
    assert_eq!(band.matches("opacity=\"0.1\"").count(), LANE_COUNT);
}

#[test]
fn paper_has_stronger_blobs_a_visible_band_and_four_faint_fibers() {
    let paper = Theme::for_name(ThemeName::Paper);
    assert!(paper.nebula.iter().all(|(_, o)| (0.35..=0.50).contains(o)));
    assert!(
        paper.milkyway_strength >= 2.0,
        "the band must register on cream"
    );
    assert_eq!(paper.vignette, Some("#eadfca"));
    assert_eq!(paper.milkyway_core, "#cdb9ee");

    let fibers = paper_fibers(3, 1200.0, 600.0);
    assert_eq!(fibers.len(), FIBER_COUNT);
    assert_eq!(FIBER_COUNT, 4);
    assert_eq!(fibers, paper_fibers(3, 1200.0, 600.0));
    assert_ne!(fibers, paper_fibers(4, 1200.0, 600.0));
    for f in &fibers {
        let length = ((f.x2 - f.x1).powi(2) + (f.y2 - f.y1).powi(2)).sqrt();
        assert!((6.0 - 1e-9..=14.0 + 1e-9).contains(&length), "{length}");
    }

    let paper_svg = medium_svg(&paper);
    let nebula_start = paper_svg.find("<g id=\"nebula\">").expect("layer");
    let nebula_end = paper_svg.find("<g id=\"dust\">").expect("layer");
    let layer = &paper_svg[nebula_start..nebula_end];
    assert_eq!(layer.matches("stroke-width=\"0.6\"").count(), FIBER_COUNT);
    assert_eq!(layer.matches("opacity=\"0.08\"").count(), FIBER_COUNT);
    let night = medium_svg(&Theme::for_name(ThemeName::Midnight));
    assert!(
        !night.contains("stroke-width=\"0.6\""),
        "fibers are paper only"
    );
}
