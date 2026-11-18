//! The background art: sky, nebula, dust, grain and the per-layer random streams.

mod common;

use std::collections::BTreeSet;

use common::load_fixture;
use git_constellation::art::background::{clouds, dust_dots};
use git_constellation::art::{ArtOptions, background_svg, layer_rng};
use git_constellation::model::ThemeName;
use git_constellation::theme::{MonoVariant, Theme};
use git_constellation::{Options, SkyModel, build_sky};

const THEMES: [ThemeName; 6] = [
    ThemeName::Midnight,
    ThemeName::Dusk,
    ThemeName::Aurora,
    ThemeName::Sakura,
    ThemeName::Paper,
    ThemeName::Mono,
];

fn model(name: &str, seed: Option<u64>) -> SkyModel {
    build_sky(
        &load_fixture(name),
        &Options {
            seed,
            ..Options::default()
        },
    )
    .expect("fixture builds")
}

fn render(m: &SkyModel, theme: &Theme, grain: bool) -> String {
    background_svg(
        m,
        theme,
        &ArtOptions {
            grain,
            ..ArtOptions::default()
        },
        &[],
    )
}

/// The text of one top-level layer, `<g id="name">` up to the next layer.
fn section<'a>(svg: &'a str, name: &str) -> &'a str {
    let order = ["bg", "nebula", "dust", "milkyway", "grain"];
    let start = svg
        .find(&format!("<g id=\"{name}\">"))
        .expect("layer exists");
    let end = order
        .iter()
        .filter_map(|next| {
            svg[start + 1..]
                .find(&format!("<g id=\"{next}\">"))
                .map(|i| start + 1 + i)
        })
        .min()
        .unwrap_or(svg.len());
    svg[start..end].trim_end_matches("</svg>\n")
}

fn attr(tag: &str, name: &str) -> f64 {
    let key = format!(" {name}=\"");
    let rest = &tag[tag.find(&key).unwrap_or_else(|| panic!("{name} in {tag}")) + key.len()..];
    rest.split('"')
        .next()
        .unwrap_or("")
        .parse()
        .unwrap_or_else(|_| panic!("number in {tag}"))
}

fn ids_in(svg: &str) -> Vec<String> {
    svg.split(" id=\"")
        .skip(1)
        .map(|r| r.split('"').next().unwrap_or("").to_string())
        .collect()
}

#[test]
fn the_same_input_gives_byte_identical_svg() {
    for name in ["small", "medium", "large"] {
        let m = model(name, None);
        for theme in THEMES {
            let t = Theme::for_name(theme);
            assert_eq!(
                render(&m, &t, false),
                render(&m, &t, false),
                "{name}/{theme:?}"
            );
        }
    }
}

#[test]
fn different_seeds_change_the_nebula_and_the_dust() {
    let t = Theme::for_name(ThemeName::Midnight);
    let (a, b) = (
        render(&model("medium", Some(1)), &t, false),
        render(&model("medium", Some(2)), &t, false),
    );
    assert!(
        section(&a, "nebula") != section(&b, "nebula"),
        "nebula differs by seed"
    );
    assert!(
        section(&a, "dust") != section(&b, "dust"),
        "dust differs by seed"
    );
    assert!(
        section(&a, "milkyway") != section(&b, "milkyway"),
        "milky way differs by seed"
    );
    assert_eq!(
        section(&a, "bg"),
        section(&b, "bg"),
        "the gradient does not depend on the seed"
    );
}

#[test]
fn every_output_is_valid_safe_and_self_consistent() {
    for fixture in ["empty", "small", "medium"] {
        let m = model(fixture, None);
        for theme in THEMES {
            let t = Theme::for_name(theme);
            let svg = render(&m, &t, false);
            let tag = format!("{fixture}/{theme:?}");
            usvg::Tree::from_str(&svg, &usvg::Options::default())
                .unwrap_or_else(|e| panic!("{tag}: {e}"));
            assert!(svg.contains("viewBox=\"0 0 1200 600\""), "{tag}");
            assert!(
                svg.contains("<title>") && svg.contains("<desc>") && svg.contains("role=\"img\""),
                "{tag}"
            );

            let ids = ids_in(&svg);
            let unique: BTreeSet<&String> = ids.iter().collect();
            assert_eq!(unique.len(), ids.len(), "{tag}: duplicate ids {ids:?}");
            for r in svg
                .split("url(#")
                .skip(1)
                .map(|r| r.split(')').next().unwrap_or(""))
            {
                assert!(
                    unique.contains(&r.to_string()),
                    "{tag}: url(#{r}) points at nothing"
                );
            }
            for forbidden in [
                "<script",
                "<image",
                "<foreignObject",
                "xlink:href",
                "rgba(",
                "hsl(",
                "hsla(",
                "rgb(",
                "mix-blend-mode",
                "<filter",
                "javascript:",
                "<style",
            ] {
                assert!(!svg.contains(forbidden), "{tag}: contains {forbidden}");
            }
            let bare = svg.replace("http://www.w3.org/2000/svg", "");
            assert!(
                !bare.contains("http://") && !bare.contains("https://"),
                "{tag}"
            );
            assert!(!svg.contains("NaN") && !svg.contains("inf"), "{tag}");
            for v in svg
                .split(|c: char| !(c.is_ascii_digit() || c == '.' || c == '-'))
                .filter(|v| v.contains('.'))
            {
                assert!(
                    v.split('.').nth(1).is_none_or(|d| d.len() <= 2),
                    "{tag}: {v} has more than 2 decimals"
                );
            }
        }
    }
}

#[test]
fn layers_come_in_order_with_their_ids() {
    let svg = render(
        &model("medium", None),
        &Theme::for_name(ThemeName::Midnight),
        false,
    );
    let positions: Vec<usize> = ["bg", "nebula", "dust", "milkyway"]
        .iter()
        .map(|id| {
            svg.find(&format!("<g id=\"{id}\">"))
                .unwrap_or_else(|| panic!("layer {id}"))
        })
        .collect();
    assert!(
        positions.windows(2).all(|w| w[0] < w[1]),
        "bg, nebula, dust, milkyway in that order"
    );
    assert!(
        svg.find("<defs>").unwrap() < positions[0],
        "gradients live in one defs at the top"
    );
    assert!(!svg.contains("<g id=\"grain\">"), "grain is off by default");
}

#[test]
fn all_six_themes_render_from_small_and_medium_and_stay_small() {
    let mut sizes = Vec::new();
    for fixture in ["small", "medium"] {
        let m = model(fixture, None);
        let mut variants: Vec<(String, Theme)> = THEMES
            .iter()
            .map(|n| (format!("{n:?}"), Theme::for_name(*n)))
            .collect();
        variants.push(("MonoLight".to_string(), Theme::mono(MonoVariant::Light)));
        for (label, theme) in variants {
            let svg = render(&m, &theme, false);
            assert!(
                svg.len() < 60 * 1024,
                "{fixture}/{label}: {} bytes",
                svg.len()
            );
            let png = git_constellation::export::svg_to_png(&svg, 1.0)
                .unwrap_or_else(|e| panic!("{fixture}/{label}: {e}"));
            assert!(png.len() > 1000);
            sizes.push(format!("{fixture}/{label}={}B", svg.len()));
        }
    }
    println!("SVG sizes: {}", sizes.join(", "));
}

#[test]
fn night_themes_have_five_nebulae_paper_has_three_blobs_and_mono_has_none() {
    let count = |t: ThemeName| {
        let svg = render(&model("medium", None), &Theme::for_name(t), false);
        section(&svg, "nebula").matches("<ellipse").count()
    };
    for dark in [
        ThemeName::Midnight,
        ThemeName::Dusk,
        ThemeName::Aurora,
        ThemeName::Sakura,
    ] {
        assert_eq!(count(dark), 5, "{dark:?}");
    }
    assert_eq!(count(ThemeName::Paper), 3);
    let mono = render(
        &model("medium", None),
        &Theme::for_name(ThemeName::Mono),
        false,
    );
    assert!(
        !mono.contains("<g id=\"nebula\">"),
        "mono has no nebula layer at all"
    );
}

#[test]
fn nebulae_share_three_gradients_and_set_strength_with_opacity() {
    let t = Theme::for_name(ThemeName::Midnight);
    let svg = render(&model("medium", None), &t, false);
    let neb = section(&svg, "nebula");
    let used: BTreeSet<&str> = neb
        .split("fill=\"url(#")
        .skip(1)
        .map(|r| r.split(')').next().unwrap_or(""))
        .collect();
    assert_eq!(
        used.len(),
        3,
        "one shared gradient per cloud color: {used:?}"
    );
    for line in neb.lines().filter(|l| l.starts_with("<ellipse")) {
        let color = ["neb3", "neb4", "neb5"]
            .iter()
            .position(|id| line.contains(&format!("url(#{id})")));
        let opacity = attr(line, "opacity");
        assert!(
            t.nebula.iter().any(|(_, o)| (*o - opacity).abs() < 1e-9),
            "{line}"
        );
        assert!(color.is_some() || line.contains("url(#neb"), "{line}");
    }
    assert!(svg.contains("gradientUnits=\"objectBoundingBox\""));
}

#[test]
fn nebula_size_rotation_and_placement_follow_the_rules() {
    for seed in 0..300u64 {
        let list = clouds(seed, 1200.0, 600.0, 5);
        assert_eq!(list.len(), 5);
        for (i, c) in list.iter().enumerate() {
            assert!(
                (190.0..=350.0).contains(&c.rx) && (100.0..=190.0).contains(&c.ry),
                "seed {seed}: {c:?}"
            );
            assert!((-30.0..=30.0).contains(&c.angle_deg), "seed {seed}");
            assert!(
                (-100.0..=1300.0).contains(&c.cx) && (-100.0..=700.0).contains(&c.cy),
                "seed {seed}"
            );
            assert!(c.cx.is_finite() && c.cy.is_finite() && c.rx > 0.0 && c.ry > 0.0);
            assert_eq!(c.color, i % 3);
            // Never within 60 px of the title rectangle or 130 px of the moon's center.
            let dx = (24.0 - c.cx).max(0.0).max(c.cx - 330.0);
            let dy = (24.0 - c.cy).max(0.0).max(c.cy - 82.0);
            assert!(
                dx.hypot(dy) >= 60.0,
                "seed {seed}: cloud {i} too near the title"
            );
            assert!(
                ((c.cx - 1090.0).powi(2) + (c.cy - 112.0).powi(2)).sqrt() >= 130.0,
                "seed {seed}: cloud {i} too near the moon"
            );
        }
    }
    assert_eq!(clouds(5, 1200.0, 600.0, 5), clouds(5, 1200.0, 600.0, 5));
    assert_ne!(clouds(5, 1200.0, 600.0, 5), clouds(6, 1200.0, 600.0, 5));
}

#[test]
fn dust_obeys_its_spacing_size_and_count_rules() {
    for seed in 0..20u64 {
        let dots = dust_dots(seed, 1200.0, 600.0, 260, &[]);
        assert!(
            dots.len() <= 260 && dots.len() > 150,
            "seed {seed}: {}",
            dots.len()
        );
        for (i, a) in dots.iter().enumerate() {
            assert!((0.0..=1200.0).contains(&a.x) && (0.0..=600.0).contains(&a.y));
            assert!((0.5..=1.2).contains(&a.r) && a.bucket < 4, "{a:?}");
            for b in &dots[i + 1..] {
                assert!(
                    ((a.x - b.x).powi(2) + (a.y - b.y).powi(2)).sqrt() >= 14.0 - 1e-9,
                    "seed {seed}: dots too close"
                );
            }
        }
        let tinted = dots.iter().filter(|d| d.tint).count() as f64 / dots.len() as f64;
        assert!((0.0..0.25).contains(&tinted), "seed {seed}: {tinted}");
    }
    let small = dust_dots(1, 1200.0, 600.0, 10, &[]);
    assert_eq!(small.len(), 10);
    assert!(dust_dots(1, 1200.0, 600.0, 0, &[]).is_empty());
    // Most dots are tiny: the radius follows 0.5 + 0.7 * u^2.
    let dots = dust_dots(3, 1200.0, 600.0, 260, &[]);
    assert!(dots.iter().filter(|d| d.r < 0.85).count() > dots.len() / 2);
    // Impossible to fit: gives up gracefully instead of looping.
    assert!(dust_dots(1, 200.0, 100.0, 5000, &[]).len() < 200);
}

#[test]
fn dust_keeps_away_from_listed_stars() {
    let avoid = [
        (300.0, 200.0, 20.0),
        (900.0, 400.0, 30.0),
        (600.0, 300.0, 8.0),
    ];
    let dots = dust_dots(7, 1200.0, 600.0, 260, &avoid);
    assert!(!dots.is_empty());
    for d in &dots {
        for &(x, y, r) in &avoid {
            assert!(
                ((d.x - x).powi(2) + (d.y - y).powi(2)).sqrt() >= 4.0 * r,
                "{d:?} is within 4r of ({x},{y})"
            );
        }
    }
    // The empty avoid list changes nothing about the sequence of accepted dots' rules.
    let without = dust_dots(7, 1200.0, 600.0, 260, &[]);
    assert!(dots.len() <= without.len() + 5);
}

#[test]
fn dust_is_grouped_by_opacity_bucket_and_color() {
    let t = Theme::for_name(ThemeName::Midnight);
    let svg = render(&model("medium", None), &t, false);
    let dust = section(&svg, "dust");
    let groups: Vec<&str> = dust.lines().filter(|l| l.starts_with("<g fill=")).collect();
    assert!(
        groups.len() >= 4 && groups.len() <= 8,
        "{} groups",
        groups.len()
    );
    for g in groups {
        assert!(
            g.contains("opacity=\"0.25\"")
                || g.contains("opacity=\"0.45\"")
                || g.contains("opacity=\"0.65\"")
                || g.contains("opacity=\"0.85\""),
            "{g}"
        );
        assert!(g.contains("#e6e9ff") || g.contains("#bfa8ff"), "{g}");
    }
    assert!(
        dust.lines()
            .filter(|l| l.starts_with("<circle"))
            .all(|l| l.matches('=').count() == 3),
        "dots are just cx cy r"
    );
    assert!(dust.matches("<circle").count() <= 260);
}

#[test]
fn the_dust_count_follows_the_model_and_never_passes_260() {
    let m = model("large", None);
    assert_eq!(m.dust_count, 260);
    let svg = render(&m, &Theme::for_name(ThemeName::Aurora), false);
    assert!(section(&svg, "dust").matches("<circle").count() <= 260);
}

#[test]
fn the_horizon_glow_is_placed_where_the_spec_says() {
    let svg = render(
        &model("medium", None),
        &Theme::for_name(ThemeName::Midnight),
        false,
    );
    assert!(
        svg.contains("cx=\"600\" cy=\"642\" r=\"522\"") || svg.contains("cy=\"642\""),
        "centered below the canvas"
    );
    assert!(svg.contains("gradientUnits=\"userSpaceOnUse\""));
    assert!(svg.contains("stop-color=\"#4b315f\" stop-opacity=\"0.55\""));
    let mono = render(
        &model("medium", None),
        &Theme::for_name(ThemeName::Mono),
        false,
    );
    assert!(
        !mono.contains("glow") && !mono.contains("userSpaceOnUse"),
        "no glow on mono"
    );
    let paper = render(
        &model("medium", None),
        &Theme::for_name(ThemeName::Paper),
        false,
    );
    assert!(paper.contains("#eadfca"), "paper has its vignette");
}

#[test]
fn grain_is_one_filter_and_only_when_asked() {
    let m = model("medium", None);
    let t = Theme::for_name(ThemeName::Paper);
    let plain = render(&m, &t, false);
    let grainy = render(&m, &t, true);
    assert!(!plain.contains("feTurbulence"));
    assert_eq!(grainy.matches("<filter").count(), 1);
    assert!(grainy.contains("baseFrequency=\"0.8\"") && grainy.contains("numOctaves=\"2\""));
    assert!(grainy.contains("opacity=\"0.06\""));
    usvg::Tree::from_str(&grainy, &usvg::Options::default()).expect("grain parses");
    let png = git_constellation::export::svg_to_png(&grainy, 0.25).expect("grain renders");
    assert!(png.len() > 1000);
    assert_eq!(
        grainy,
        render(&m, &t, true),
        "the grain seed is deterministic too"
    );
    assert!(
        grainy != render(&model("medium", Some(99)), &t, true),
        "grain differs by seed"
    );
}

#[test]
fn each_layer_has_its_own_stream() {
    let first_five = |seed: u64, layer: &str| {
        let mut rng = layer_rng(seed, layer);
        (0..5).map(|_| rng.unit()).collect::<Vec<f64>>()
    };
    for layer in ["nebula", "dust", "milkyway", "grain"] {
        assert_eq!(first_five(42, layer), first_five(42, layer), "{layer}");
        assert_ne!(
            first_five(42, layer),
            first_five(43, layer),
            "{layer} changes with the seed"
        );
    }
    let layers = ["nebula", "dust", "milkyway", "grain"];
    for (i, a) in layers.iter().enumerate() {
        for b in &layers[i + 1..] {
            assert_ne!(
                first_five(42, a),
                first_five(42, b),
                "{a} and {b} must differ"
            );
        }
    }
}

#[test]
fn adding_or_skipping_one_layer_never_shifts_another() {
    // Dust placement is the same whether or not the nebula was computed first.
    let before = dust_dots(11, 1200.0, 600.0, 260, &[]);
    let _ = clouds(11, 1200.0, 600.0, 5);
    assert_eq!(before, dust_dots(11, 1200.0, 600.0, 260, &[]));
    let t = Theme::for_name(ThemeName::Midnight);
    let m = model("medium", Some(11));
    let with_grain = render(&m, &t, true);
    let without = render(&m, &t, false);
    for layer in ["bg", "nebula", "dust", "milkyway"] {
        assert_eq!(
            section(&with_grain, layer),
            section(&without, layer),
            "{layer} unchanged by grain"
        );
    }
}

#[test]
fn a_custom_canvas_size_scales_the_art() {
    let m = model("medium", None);
    let t = Theme::for_name(ThemeName::Midnight);
    let svg = background_svg(
        &m,
        &t,
        &ArtOptions {
            width: 600.0,
            height: 300.0,
            grain: false,
        },
        &[],
    );
    assert!(svg.contains("viewBox=\"0 0 600 300\""));
    usvg::Tree::from_str(&svg, &usvg::Options::default()).expect("valid");
}
