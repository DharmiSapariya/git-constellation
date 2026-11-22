//! Milestone 5: how stars are drawn (tiers, halos, sparkles, flares, rings, sharing).

mod common;

use std::collections::BTreeSet;

use common::{THEME_NAMES, options_for, planned};
use git_constellation::art::star_parts::{
    FLARE_LENGTH_MULT, FLARE_OPACITY, FLARE_THICKNESS, PAPER_FLARE_OPACITY, RING_FOUNDING_OPACITY,
    RING_FRESH_DASH, flare_half_lengths, flare_markup, ring_markup,
};
use git_constellation::art::stars::{
    SPARKLE_ROTATION_MAX_DEG, rotations, sparkle_path, star_palette,
};
use git_constellation::art::{ArtOptions, sky_svg};
use git_constellation::layout::geometry::{FOUNDING_RING, FRESH_RING, Rect, star_radius};
use git_constellation::model::{LabelMode, ThemeName, Tier};
use git_constellation::svg::fmt;
use git_constellation::theme::{Theme, luma, parse_hex};

fn render(fixture: &str, name: ThemeName) -> (git_constellation::SkyModel, String) {
    let options = options_for(name);
    let (model, layout) = planned(fixture, &options);
    let theme = Theme::for_name(name);
    let svg = sky_svg(
        &model,
        &layout,
        &theme,
        &ArtOptions::default(),
        LabelMode::Major,
    );
    (model, svg)
}

/// Each star is written on one line that starts with `<g id="star-`.
fn star_lines(svg: &str) -> Vec<&str> {
    svg.lines()
        .filter(|line| line.starts_with("<g id=\"star-"))
        .collect()
}

#[test]
fn star_radii_are_the_spec_values() {
    let radii: Vec<f64> = [Tier::One, Tier::Two, Tier::Three, Tier::Four, Tier::Five]
        .iter()
        .map(|t| star_radius(*t))
        .collect();
    assert_eq!(radii, vec![2.2, 3.2, 4.6, 6.4, 8.6]);
}

#[test]
fn the_sparkle_is_four_concave_curves() {
    assert_eq!(
        sparkle_path(10.0),
        "M 0 -21 Q 2 -2 21 0 Q 2 2 0 21 Q -2 2 -21 0 Q -2 -2 0 -21 Z"
    );
}

#[test]
fn every_star_is_one_group_with_a_stable_id() {
    for name in THEME_NAMES {
        let (model, svg) = render("medium", name);
        let lines = star_lines(&svg);
        assert_eq!(lines.len(), model.stars.len(), "{name:?}");
        for (i, line) in lines.iter().enumerate() {
            assert!(
                line.starts_with(&format!("<g id=\"star-{i}\">")),
                "{name:?} {i}"
            );
        }
    }
}

#[test]
fn tiers_decide_the_body_shape_and_every_star_has_a_halo_and_a_core() {
    let (model, svg) = render("medium", ThemeName::Midnight);
    for (star, line) in model.stars.iter().zip(star_lines(&svg)) {
        assert_eq!(line.matches("url(#halo-o").count(), 1);
        assert_eq!(line.matches("url(#halo-i").count(), 1);
        let sparkle = line.contains("<path d=\"M 0 -");
        let should = star.tier.number() >= 3 && !star.borrowed;
        assert_eq!(
            sparkle,
            should,
            "{} (tier {})",
            star.repo,
            star.tier.number()
        );
        if !star.borrowed {
            assert!(
                line.contains("opacity=\"0.95\""),
                "{} has a core",
                star.repo
            );
        }
    }
}

#[test]
fn only_calm_tier_five_stars_get_two_shared_flare_arms() {
    for name in THEME_NAMES {
        let (model, svg) = render("large", name);
        let flared = model
            .stars
            .iter()
            .filter(|s| s.tier == Tier::Five && !s.dormant && !s.borrowed)
            .count();
        let dark = Theme::for_name(name).is_dark;
        let rects = svg.matches("<rect").count();
        let flare_rects = svg.matches("fill=\"url(#flare").count();
        let gradients = svg.matches("<linearGradient id=\"flare").count();
        if dark {
            assert_eq!(flare_rects, 2 * flared, "{name:?}");
            assert_eq!(svg.matches("rotate(90 ").count(), flared, "{name:?}");
            assert_eq!(
                gradients,
                usize::from(flared > 0),
                "{name:?}: one shared gradient"
            );
        } else {
            assert_eq!(
                (flare_rects, gradients),
                (0, 0),
                "{name:?}: paper uses ink strokes"
            );
        }
        assert!(rects >= flare_rects);
    }
}

#[test]
fn dormant_stars_are_dimmer_and_have_no_flare() {
    for name in THEME_NAMES {
        let (model, svg) = render("large", name);
        for (star, line) in model.stars.iter().zip(star_lines(&svg)) {
            if star.dormant {
                assert!(line.contains("<g opacity=\"0.55\">"), "{}", star.repo);
                assert!(!line.contains("<rect"), "{}", star.repo);
            } else {
                assert!(!line.contains("<g opacity=\"0.55\">"), "{}", star.repo);
            }
        }
    }
}

#[test]
fn rings_belong_to_founding_fresh_and_borrowed_stars_only() {
    for fixture in ["small", "medium", "large"] {
        let (model, svg) = render(fixture, ThemeName::Midnight);
        assert!(
            model.stars.iter().filter(|s| s.founding).count() <= 1,
            "{fixture}"
        );
        for (star, line) in model.stars.iter().zip(star_lines(&svg)) {
            let fresh_ring = !star.founding && star.fresh && !star.dormant;
            let expected = usize::from(star.founding || fresh_ring) + usize::from(star.borrowed);
            assert_eq!(
                line.matches("fill=\"none\"").count(),
                expected,
                "{}",
                star.repo
            );
            // Only the fresh ring is dotted; the founding ring is solid.
            assert_eq!(
                line.contains(&format!("stroke-dasharray=\"{RING_FRESH_DASH}\"")),
                fresh_ring,
                "{}",
                star.repo
            );
        }
    }
}

#[test]
fn rings_are_drawn_at_the_layouts_radii() {
    let (model, svg) = render("large", ThemeName::Midnight);
    for (star, line) in model.stars.iter().zip(star_lines(&svg)) {
        let r = star_radius(star.tier);
        if star.founding {
            assert!(
                line.contains(&format!("r=\"{}\"", fmt(FOUNDING_RING * r))),
                "{}",
                star.repo
            );
        } else if star.fresh && !star.dormant {
            assert!(
                line.contains(&format!("r=\"{}\"", fmt(FRESH_RING * r))),
                "{}",
                star.repo
            );
        }
    }
    assert_eq!((FOUNDING_RING, FRESH_RING), (3.0, 2.5));
}

#[test]
fn a_ring_is_one_circle_and_founding_wins_over_fresh() {
    let (model, _) = render("medium", ThemeName::Midnight);
    let theme = Theme::for_name(ThemeName::Midnight);
    let mut star = model.stars[0].clone();
    star.dormant = false;
    let palette = star_palette(&theme, &star);
    let at = (100.0, 100.0);
    let r = star_radius(star.tier);

    (star.founding, star.fresh) = (true, true);
    let both = ring_markup(&theme, &star, at, &palette);
    assert_eq!(
        both.matches("<circle").count(),
        1,
        "one ring, not a bullseye"
    );
    assert!(!both.contains("stroke-dasharray"));
    assert!(both.contains(&format!("r=\"{}\"", fmt(FOUNDING_RING * r))));
    assert!(both.contains(&format!(
        "stroke-opacity=\"{}\"",
        fmt(RING_FOUNDING_OPACITY)
    )));

    (star.founding, star.fresh) = (false, true);
    let fresh = ring_markup(&theme, &star, at, &palette);
    assert_eq!(fresh.matches("<circle").count(), 1);
    assert!(fresh.contains(&format!("stroke-dasharray=\"{RING_FRESH_DASH}\"")));
    assert!(fresh.contains(&format!("r=\"{}\"", fmt(FRESH_RING * r))));
    assert!(
        FRESH_RING * r > 2.1 * r,
        "the dotted ring sits outside the sparkle tips"
    );

    star.dormant = true;
    assert!(
        ring_markup(&theme, &star, at, &palette).is_empty(),
        "dormant stars have no fresh ring"
    );
    (star.founding, star.fresh) = (false, false);
    assert!(ring_markup(&theme, &star, at, &palette).is_empty());
}

#[test]
fn the_flare_is_bright_and_thick_enough_to_see() {
    assert_eq!(
        (
            FLARE_OPACITY,
            FLARE_THICKNESS,
            FLARE_LENGTH_MULT,
            PAPER_FLARE_OPACITY
        ),
        (0.85, 1.3, 5.0, 0.7)
    );
}

#[test]
fn flare_arms_stop_three_pixels_before_a_label() {
    let at = (100.0, 100.0);
    let half = 21.5;
    let none = flare_half_lengths(at, half, FLARE_THICKNESS, &[]);
    assert_eq!(none, (half, half));
    // A label on the right, on the arm's line: the horizontal arm ends 3 px before it.
    let right = Rect {
        x0: 118.0,
        y0: 94.0,
        x1: 170.0,
        y1: 106.0,
    };
    assert_eq!(
        flare_half_lengths(at, half, FLARE_THICKNESS, &[right]),
        (15.0, half)
    );
    // A label above, over the vertical arm's line.
    let above = Rect {
        x0: 90.0,
        y0: 60.0,
        x1: 130.0,
        y1: 80.0,
    };
    assert_eq!(
        flare_half_lengths(at, half, FLARE_THICKNESS, &[above]),
        (half, 17.0)
    );
    // A label that does not lie on either arm's line changes nothing.
    let aside = Rect {
        x0: 118.0,
        y0: 120.0,
        x1: 170.0,
        y1: 134.0,
    };
    assert_eq!(
        flare_half_lengths(at, half, FLARE_THICKNESS, &[aside]),
        (half, half)
    );
    // A label sitting on the star leaves no arm at all.
    let on_star = Rect {
        x0: 90.0,
        y0: 90.0,
        x1: 110.0,
        y1: 110.0,
    };
    assert_eq!(
        flare_half_lengths(at, half, FLARE_THICKNESS, &[on_star]),
        (0.0, 0.0)
    );
}

#[test]
fn a_shortened_flare_is_drawn_shorter_and_a_free_one_is_not() {
    let (model, _) = render("large", ThemeName::Midnight);
    let theme = Theme::for_name(ThemeName::Midnight);
    let mut star = model.stars[0].clone();
    star.tier = Tier::Five;
    (star.dormant, star.borrowed) = (false, false);
    let at = (100.0, 100.0);
    let full = flare_markup(&theme, &star, at, Some("flare1"), &[]);
    assert_eq!(full.matches("<rect").count(), 2);
    assert!(full.contains(&format!("width=\"{}\"", fmt(FLARE_LENGTH_MULT * 8.6))));
    let label = Rect {
        x0: 118.0,
        y0: 94.0,
        x1: 170.0,
        y1: 106.0,
    };
    let cut = flare_markup(&theme, &star, at, Some("flare1"), &[label]);
    assert!(cut.contains("width=\"30\""), "{cut}");
    star.tier = Tier::Four;
    assert!(flare_markup(&theme, &star, at, Some("flare1"), &[]).is_empty());
}

#[test]
fn rotations_are_seeded_and_small() {
    let turns = rotations(42, 200);
    assert_eq!(turns.len(), 200);
    assert!(
        turns
            .iter()
            .all(|t| (-SPARKLE_ROTATION_MAX_DEG..=SPARKLE_ROTATION_MAX_DEG).contains(t))
    );
    assert_eq!(turns, rotations(42, 200));
    assert_ne!(turns, rotations(43, 200));
    assert!(
        turns.windows(2).any(|w| w[0] != w[1]),
        "no two sparkles stamped alike"
    );
    assert_eq!(
        &turns[..50],
        &rotations(42, 50)[..],
        "a star's turn does not depend on the count"
    );
}

#[test]
fn the_same_sky_is_the_same_bytes() {
    for name in THEME_NAMES {
        assert_eq!(
            render("medium", name).1,
            render("medium", name).1,
            "{name:?}"
        );
    }
}

#[test]
fn star_gradients_are_shared_per_color_not_per_star() {
    for name in THEME_NAMES {
        let theme = Theme::for_name(name);
        let mut counts = Vec::new();
        for fixture in ["medium", "large"] {
            let (model, svg) = render(fixture, name);
            let colors: BTreeSet<String> = model
                .stars
                .iter()
                .map(|s| star_palette(&theme, s).glow)
                .collect();
            let gradients = svg
                .split("<radialGradient id=\"")
                .skip(1)
                .filter(|rest| {
                    ["halo-o", "halo-i", "body", "wash"]
                        .iter()
                        .any(|prefix| rest.starts_with(prefix))
                })
                .count();
            assert!(
                gradients <= 3 * colors.len(),
                "{name:?} {fixture}: {gradients} gradients for {} colors",
                colors.len()
            );
            counts.push(gradients);
        }
        assert!(
            counts[1] <= 60,
            "{name:?}: large has {} gradients",
            counts[1]
        );
    }
}

#[test]
fn palettes_cool_dormant_stars_and_ink_the_paper_ones() {
    let (model, _) = render("medium", ThemeName::Midnight);
    let mut star = model.stars[0].clone();
    star.dormant = false;
    let dark = Theme::for_name(ThemeName::Midnight);
    let awake = star_palette(&dark, &star);
    assert_eq!(awake.glow, awake.body, "dark themes use one color");
    star.dormant = true;
    assert_ne!(
        star_palette(&dark, &star).glow,
        awake.glow,
        "dormant stars are cooled"
    );
    star.dormant = false;
    let paper = Theme::for_name(ThemeName::Paper);
    let inked = star_palette(&paper, &star);
    let (glow, body) = (
        parse_hex(&inked.glow).unwrap(),
        parse_hex(&inked.body).unwrap(),
    );
    assert!(
        luma(body) < luma(glow),
        "paper bodies are darkened toward the ink"
    );
}

#[test]
fn paper_stars_are_washes_with_inked_bodies() {
    let (model, svg) = render("medium", ThemeName::Paper);
    assert!(svg.contains("<radialGradient id=\"wash"));
    assert!(!svg.contains("<radialGradient id=\"halo-o"));
    assert!(!svg.contains("<radialGradient id=\"body"));
    let theme = Theme::for_name(ThemeName::Paper);
    for (star, line) in model.stars.iter().zip(star_lines(&svg)) {
        if !star.borrowed {
            assert!(
                line.contains(&format!("stroke=\"{}\"", theme.ink)),
                "{}",
                star.repo
            );
        }
    }
}

#[test]
fn every_sky_parses_even_with_odd_repo_names() {
    for name in THEME_NAMES {
        let (_, svg) = render("edge", name);
        usvg::Tree::from_str(&svg, &usvg::Options::default()).expect("valid SVG");
    }
}

#[test]
fn paper_inks_small_stars_darker_and_grays_dormant_ones() {
    use git_constellation::theme::mix;
    let (model, _) = render("medium", ThemeName::Paper);
    let paper = Theme::for_name(ThemeName::Paper);
    let ink = parse_hex(paper.ink).unwrap();
    let mut star = model.stars[0].clone();
    star.dormant = false;
    let language = parse_hex(paper.star_color(star.language)).unwrap();

    star.tier = Tier::One;
    assert_eq!(
        star_palette(&paper, &star).body,
        mix(language, ink, 0.30).to_hex()
    );
    star.tier = Tier::Two;
    assert_eq!(
        star_palette(&paper, &star).body,
        mix(language, ink, 0.30).to_hex()
    );
    star.tier = Tier::Three;
    assert_eq!(
        star_palette(&paper, &star).body,
        mix(language, ink, 0.18).to_hex()
    );

    // Dormant on paper: grayed ink, never the pale tint.
    star.dormant = true;
    let gray = mix(language, ink, 0.60).to_hex();
    for tier in [Tier::One, Tier::Three, Tier::Five] {
        star.tier = tier;
        let palette = star_palette(&paper, &star);
        assert_eq!(
            (palette.glow.clone(), palette.body),
            (gray.clone(), gray.clone())
        );
    }

    // Dark themes keep their cool tint and are not inked.
    let dark = Theme::for_name(ThemeName::Midnight);
    star.tier = Tier::One;
    let cooled = star_palette(&dark, &star);
    assert_eq!(cooled.glow, cooled.body);
    assert_ne!(cooled.glow, gray);
}

#[test]
fn paper_outlines_are_thinner_on_the_smallest_stars() {
    let (model, svg) = render("large", ThemeName::Paper);
    for (star, line) in model.stars.iter().zip(star_lines(&svg)) {
        if star.borrowed {
            continue;
        }
        let small = star.tier.number() <= 2;
        // The body's outline is 0.7 px for small stars and 0.8 px for the others, both at
        // opacity 0.7 (the dotted fresh ring is also 0.7 px wide, but at opacity 0.35).
        let thin = line.contains("stroke-width=\"0.7\" stroke-opacity=\"0.7\"");
        let normal = line.contains("stroke-width=\"0.8\" stroke-opacity=\"0.7\"");
        assert_eq!((thin, normal), (small, !small), "{}", star.repo);
    }
}
