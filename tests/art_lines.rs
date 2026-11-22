//! Milestone 5: constellation lines (gaps, opacity rules, grouping).

mod common;

use common::{THEME_NAMES, options_for, planned};
use git_constellation::art::lines::{
    LINE_DORMANT_DASH, LINE_DORMANT_OPACITY, LINE_GAP_FOUNDING_MULT, LINE_GAP_MULT,
    LINE_SPINE_BOOST, line_gap, line_style, trimmed_segment,
};
use git_constellation::art::{ArtOptions, sky_svg, sky_svg_with_stats};
use git_constellation::layout::geometry::{FOUNDING_RING, dist, star_radius};
use git_constellation::model::{LabelMode, ThemeName, Tier};
use git_constellation::theme::Theme;

fn lines_layer(svg: &str) -> &str {
    let start = svg.find("<g id=\"lines\">").expect("a lines layer");
    let end = svg.find("<g id=\"stars\">").expect("a stars layer");
    &svg[start..end]
}

#[test]
fn a_segment_is_shortened_at_both_ends_or_not_drawn_at_all() {
    let (from, to) = trimmed_segment((0.0, 0.0), (100.0, 0.0), 10.0, 20.0).expect("long enough");
    assert!((from.0 - 10.0).abs() < 1e-9 && from.1.abs() < 1e-9);
    assert!((to.0 - 80.0).abs() < 1e-9 && to.1.abs() < 1e-9);
    assert!(trimmed_segment((0.0, 0.0), (30.0, 0.0), 10.0, 20.0).is_none());
    assert!(trimmed_segment((0.0, 0.0), (20.0, 0.0), 10.0, 20.0).is_none());
    assert!(trimmed_segment((5.0, 5.0), (5.0, 5.0), 0.0, 0.0).is_none());
}

#[test]
fn the_gap_is_wider_at_the_founding_star() {
    let (model, _) = planned("medium", &options_for(ThemeName::Midnight));
    let mut star = model.stars[0].clone();
    star.tier = Tier::Five;
    star.founding = false;
    assert!((line_gap(&star) - LINE_GAP_MULT * 8.6).abs() < 1e-9);
    star.founding = true;
    assert!((line_gap(&star) - 3.3 * 8.6).abs() < 1e-9);
    assert!((line_gap(&star) - LINE_GAP_FOUNDING_MULT * 8.6).abs() < 1e-9);
    assert!(
        line_gap(&star) > FOUNDING_RING * 8.6,
        "lines stop outside the ring"
    );
}

#[test]
fn opacity_rules_dormant_beats_spine_beats_default() {
    let (model, _) = planned("medium", &options_for(ThemeName::Midnight));
    let theme = Theme::for_name(ThemeName::Midnight);
    let mut a = model.stars[0].clone();
    let mut b = model.stars[1].clone();
    for star in [&mut a, &mut b] {
        star.dormant = false;
        star.tier = Tier::Two;
    }
    let plain = line_style(&theme, &a, &b);
    assert_eq!((plain.opacity, plain.dash), (theme.line.1, None));

    a.tier = Tier::Four;
    let spine = line_style(&theme, &a, &b);
    assert!((spine.opacity - (theme.line.1 + LINE_SPINE_BOOST)).abs() < 1e-9);
    assert!(
        (spine.opacity - 0.50).abs() < 1e-9,
        "0.38 + 0.12 on the dark themes"
    );
    assert_eq!(spine.dash, None);

    b.tier = Tier::Five;
    assert_eq!(
        line_style(&theme, &a, &b),
        spine,
        "two bright ends are still one spine"
    );

    b.dormant = true;
    let quiet = line_style(&theme, &a, &b);
    assert_eq!(quiet.opacity, LINE_DORMANT_OPACITY);
    assert_eq!(quiet.dash, Some(LINE_DORMANT_DASH));
    assert_eq!(LINE_DORMANT_DASH, "2 5");
}

#[test]
fn the_spine_opacity_never_passes_one() {
    let (model, _) = planned("medium", &options_for(ThemeName::Paper));
    let theme = Theme::for_name(ThemeName::Paper);
    let mut a = model.stars[0].clone();
    a.dormant = false;
    a.tier = Tier::Five;
    let b = a.clone();
    assert!(line_style(&theme, &a, &b).opacity <= 1.0);
}

#[test]
fn every_constellation_is_a_titled_group_and_every_drawn_edge_is_two_paths() {
    for name in THEME_NAMES {
        let options = options_for(name);
        let theme = Theme::for_name(name);
        for fixture in ["tiny", "small", "medium", "large", "edge"] {
            let (model, layout) = planned(fixture, &options);
            let (svg, skipped) = sky_svg_with_stats(
                &model,
                &layout,
                &theme,
                &ArtOptions::default(),
                LabelMode::Major,
            );
            let layer = lines_layer(&svg);
            assert_eq!(
                layer.matches("<g id=\"constellation-").count(),
                layout.constellations.len(),
                "{name:?} {fixture}"
            );
            assert_eq!(
                layer.matches("<title>").count(),
                layout.constellations.len(),
                "{name:?} {fixture}"
            );
            let edges: usize = layout.constellations.iter().map(|c| c.edges.len()).sum();
            assert_eq!(
                layer.matches("<path").count(),
                2 * (edges - skipped),
                "{name:?} {fixture}: a glow and a line per drawn edge"
            );
        }
    }
}

#[test]
fn lines_are_drawn_before_stars_and_dashed_only_toward_dormant_stars() {
    let options = options_for(ThemeName::Midnight);
    let theme = Theme::for_name(ThemeName::Midnight);
    for fixture in ["medium", "large"] {
        let (model, layout) = planned(fixture, &options);
        let svg = sky_svg(
            &model,
            &layout,
            &theme,
            &ArtOptions::default(),
            LabelMode::Major,
        );
        assert!(svg.find("<g id=\"lines\"").unwrap() < svg.find("<g id=\"stars\"").unwrap());
        let drawn_dormant = layout
            .constellations
            .iter()
            .flat_map(|c| c.edges.iter())
            .filter(|edge| {
                let (ai, bi) = **edge;
                let (a, b) = (&model.stars[ai], &model.stars[bi]);
                let drawable = trimmed_segment(
                    layout.positions[ai],
                    layout.positions[bi],
                    line_gap(a),
                    line_gap(b),
                )
                .is_some();
                drawable && (a.dormant || b.dormant)
            })
            .count();
        let dashed = lines_layer(&svg)
            .matches("stroke-dasharray=\"2 5\"")
            .count();
        assert_eq!(
            dashed, drawn_dormant,
            "{fixture}: dashes only toward dormant stars"
        );
    }
}

#[test]
fn drawn_edges_end_outside_every_ring() {
    let options = options_for(ThemeName::Midnight);
    let (model, layout) = planned("large", &options);
    for placed in &layout.constellations {
        for &(ai, bi) in &placed.edges {
            let (a, b) = (&model.stars[ai], &model.stars[bi]);
            let (pa, pb) = (layout.positions[ai], layout.positions[bi]);
            if let Some((from, to)) = trimmed_segment(pa, pb, line_gap(a), line_gap(b)) {
                assert!(dist(from, pa) >= line_gap(a) - 1e-9);
                assert!(dist(to, pb) >= line_gap(b) - 1e-9);
                if a.founding {
                    assert!(dist(from, pa) >= 3.3 * star_radius(a.tier) - 1e-9);
                    assert!(dist(from, pa) > FOUNDING_RING * star_radius(a.tier));
                }
                if b.founding {
                    assert!(dist(to, pb) >= 3.3 * star_radius(b.tier) - 1e-9);
                }
            }
        }
    }
}

#[test]
fn no_line_is_too_short_to_draw_on_any_fixture() {
    // The wider founding gap (3.3 r) must not swallow a short edge at the founding star.
    for name in THEME_NAMES {
        let options = options_for(name);
        let theme = Theme::for_name(name);
        for fixture in common::FIXTURES {
            let (model, layout) = planned(fixture, &options);
            let (_, skipped) = sky_svg_with_stats(
                &model,
                &layout,
                &theme,
                &ArtOptions::default(),
                LabelMode::Major,
            );
            assert_eq!(
                skipped, 0,
                "{name:?} {fixture}: {skipped} edges skipped as too short"
            );
        }
    }
}
