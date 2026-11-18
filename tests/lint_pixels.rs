//! The picture-based lint checks: brightness (gamma-encoded) and clipped pixels.

mod common;

use common::load_fixture;
use git_constellation::art::{ArtOptions, background_svg};
use git_constellation::export::svg_to_png;
use git_constellation::lint::{CLIP_LEVEL, pixel_checks, pixel_stats};
use git_constellation::model::ThemeName;
use git_constellation::theme::{MonoVariant, Theme};
use git_constellation::{Options, build_sky};

fn solid(r: u8, g: u8, b: u8) -> Vec<u8> {
    let mut pixmap = tiny_skia::Pixmap::new(20, 10).unwrap();
    pixmap.fill(tiny_skia::Color::from_rgba8(r, g, b, 255));
    pixmap.encode_png().unwrap()
}

/// Left half one color, right half another.
fn halves(left: (u8, u8, u8), right: (u8, u8, u8)) -> Vec<u8> {
    let mut pixmap = tiny_skia::Pixmap::new(20, 10).unwrap();
    for (i, px) in pixmap.pixels_mut().iter_mut().enumerate() {
        let (r, g, b) = if i % 20 < 10 { left } else { right };
        *px = tiny_skia::ColorU8::from_rgba(r, g, b, 255).premultiply();
    }
    pixmap.encode_png().unwrap()
}

#[test]
fn brightness_is_measured_on_encoded_values_not_linear_light() {
    let black = pixel_stats(&solid(0, 0, 0)).unwrap();
    assert_eq!((black.mean_luma, black.clipped), (0.0, 0.0));
    let white = pixel_stats(&solid(255, 255, 255)).unwrap();
    assert!((white.mean_luma - 1.0).abs() < 1e-9 && (white.clipped - 1.0).abs() < 1e-9);
    // Mid-gray: encoded 128/255 = 0.502. In linear light it would be about 0.22.
    let gray = pixel_stats(&solid(128, 128, 128)).unwrap();
    assert!(
        (gray.mean_luma - 128.0 / 255.0).abs() < 1e-9,
        "{}",
        gray.mean_luma
    );
    // The weights are 0.2126 / 0.7152 / 0.0722.
    assert!((pixel_stats(&solid(255, 0, 0)).unwrap().mean_luma - 0.2126).abs() < 1e-9);
    assert!((pixel_stats(&solid(0, 255, 0)).unwrap().mean_luma - 0.7152).abs() < 1e-9);
    assert!((pixel_stats(&solid(0, 0, 255)).unwrap().mean_luma - 0.0722).abs() < 1e-9);
    // A typical night sky color sits comfortably inside the dark range.
    assert!((pixel_stats(&solid(0x15, 0x1b, 0x3d)).unwrap().mean_luma - 0.1106).abs() < 0.001);
}

#[test]
fn the_mean_averages_all_pixels_and_clipping_counts_a_share() {
    let half = pixel_stats(&halves((0, 0, 0), (255, 255, 255))).unwrap();
    assert!((half.mean_luma - 0.5).abs() < 1e-9 && (half.clipped - 0.5).abs() < 1e-9);
    // Clipping needs all three channels at 253 or more.
    assert_eq!(CLIP_LEVEL, 253);
    assert_eq!(pixel_stats(&solid(253, 253, 253)).unwrap().clipped, 1.0);
    assert_eq!(pixel_stats(&solid(252, 255, 255)).unwrap().clipped, 0.0);
    assert_eq!(pixel_stats(&solid(255, 255, 200)).unwrap().clipped, 0.0);
}

#[test]
fn dark_themes_pass_inside_the_range_and_fail_outside_it() {
    let verdict = |png: Vec<u8>| pixel_checks(&png, true).unwrap();
    for (level, ok) in [
        (5u8, false),
        (10, false),
        (21, true),
        (40, true),
        (76, true),
        (90, false),
        (200, false),
    ] {
        let checks = verdict(solid(level, level, level));
        assert_eq!(checks[0].passed, ok, "gray {level}: {}", checks[0].detail);
    }
    assert!(
        verdict(solid(40, 40, 40))[0].detail.contains("0.157"),
        "the measured number is reported"
    );
}

#[test]
fn light_themes_use_the_light_range() {
    let verdict = |png: Vec<u8>| pixel_checks(&png, false).unwrap();
    for (level, ok) in [
        (150u8, false),
        (203, false),
        (205, true),
        (230, true),
        (242, true),
        (245, false),
        (250, false),
    ] {
        let checks = verdict(solid(level, level, level));
        assert_eq!(checks[0].passed, ok, "gray {level}: {}", checks[0].detail);
    }
}

#[test]
fn clipping_is_limited_for_dark_themes_only() {
    // 10% white on a dark sky: 10% > 4% fails; 3% passes; light themes are not limited.
    let mut pixmap = tiny_skia::Pixmap::new(100, 100).unwrap();
    pixmap.fill(tiny_skia::Color::from_rgba8(30, 30, 40, 255));
    let white = tiny_skia::ColorU8::from_rgba(255, 255, 255, 255).premultiply();
    for (i, px) in pixmap.pixels_mut().iter_mut().enumerate() {
        if i < 1000 {
            *px = white;
        }
    }
    let ten_percent = pixmap.encode_png().unwrap();
    assert!(!pixel_checks(&ten_percent, true).unwrap()[1].passed);
    assert!(
        pixel_checks(&ten_percent, false).unwrap()[1].passed,
        "light themes are not limited"
    );
    for (i, px) in pixmap.pixels_mut().iter_mut().enumerate() {
        *px = if i < 300 {
            white
        } else {
            tiny_skia::ColorU8::from_rgba(30, 30, 40, 255).premultiply()
        };
    }
    let three_percent = pixmap.encode_png().unwrap();
    assert!(pixel_checks(&three_percent, true).unwrap()[1].passed);
    assert!(
        pixel_checks(&three_percent, true).unwrap()[1]
            .detail
            .contains("3.00%")
    );
}

#[test]
fn not_a_png_is_an_error_not_a_panic() {
    assert!(pixel_stats(b"definitely not a png").is_err());
    assert!(pixel_checks(&[], true).is_err());
    let error = pixel_stats(b"nope").unwrap_err().to_string();
    assert!(error.contains("PNG"), "{error}");
}

fn rendered(theme: Theme, fixture: &str) -> Vec<u8> {
    let model = build_sky(&load_fixture(fixture), &Options::default()).unwrap();
    svg_to_png(
        &background_svg(&model, &theme, &ArtOptions::default(), &[]),
        0.5,
    )
    .unwrap()
}

#[test]
fn the_real_backgrounds_land_in_their_brightness_ranges() {
    let mut variants: Vec<(String, Theme)> = [
        ThemeName::Midnight,
        ThemeName::Dusk,
        ThemeName::Aurora,
        ThemeName::Sakura,
        ThemeName::Paper,
        ThemeName::Mono,
    ]
    .iter()
    .map(|n| (format!("{n:?}"), Theme::for_name(*n)))
    .collect();
    variants.push(("MonoLight".to_string(), Theme::mono(MonoVariant::Light)));
    let mut table = Vec::new();
    let mut brightness_misses = Vec::new();
    for (label, theme) in variants {
        for fixture in ["small", "medium", "empty"] {
            let png = rendered(theme.clone(), fixture);
            let stats = pixel_stats(&png).unwrap();
            let checks = pixel_checks(&png, theme.is_dark).unwrap();
            table.push(format!(
                "{label}/{fixture}: luma {:.3}, clipped {:.2}%",
                stats.mean_luma,
                stats.clipped * 100.0
            ));
            assert!(checks[1].passed, "{label}/{fixture}: {}", checks[1].detail);
            if !checks[0].passed {
                brightness_misses.push(label.clone());
            }
        }
    }
    println!("PIXELS\n{}", table.join("\n"));
    brightness_misses.sort();
    brightness_misses.dedup();
    // The one known soft warning: mono-light is pure white (0.98), over the 0.95
    // ceiling the spec also asks for, so the check rightly flags it.
    assert_eq!(
        brightness_misses,
        ["MonoLight"],
        "unexpected brightness verdicts:\n{}",
        table.join("\n")
    );
}
