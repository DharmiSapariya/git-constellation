//! Milestone 5: every theme with every fixture draws a valid, safe, readable sky.

mod common;

use common::{FIXTURES, THEME_NAMES, options_for, planned};
use git_constellation::art::{ArtOptions, sky_svg};
use git_constellation::lint::{
    MAX_SVG_BYTES, drawn_checks, has_non_finite_number, svg_hard_checks, text_contrast_check,
};
use git_constellation::model::LabelMode;
use git_constellation::theme::Theme;

#[test]
fn every_theme_and_fixture_makes_a_safe_valid_svg() {
    for name in THEME_NAMES {
        let options = options_for(name);
        let theme = Theme::for_name(name);
        for fixture in FIXTURES {
            let (model, layout) = planned(fixture, &options);
            let svg = sky_svg(
                &model,
                &layout,
                &theme,
                &ArtOptions::default(),
                LabelMode::Major,
            );
            let what = format!("{name:?} {fixture}");
            assert!(svg.contains("viewBox=\"0 0 1200 600\""), "{what}");
            for banned in [
                "<script",
                "<image",
                "<foreignObject",
                "rgba(",
                "hsl(",
                "@import",
            ] {
                assert!(!svg.contains(banned), "{what}: {banned}");
            }
            assert_eq!(svg.matches("http://").count(), 1, "{what}: only the xmlns");
            assert_eq!(svg.matches("https://").count(), 0, "{what}");
            assert!(svg.len() < MAX_SVG_BYTES, "{what}: {} bytes", svg.len());
            usvg::Tree::from_str(&svg, &usvg::Options::default())
                .unwrap_or_else(|e| panic!("{what}: {e}"));
            for check in svg_hard_checks(&svg) {
                assert!(check.passed, "{what}: {} {}", check.name, check.detail);
            }
        }
    }
}

#[test]
fn the_layers_come_in_back_to_front_order() {
    for name in THEME_NAMES {
        let options = options_for(name);
        let theme = Theme::for_name(name);
        let (model, layout) = planned("medium", &options);
        let svg = sky_svg(
            &model,
            &layout,
            &theme,
            &ArtOptions::default(),
            LabelMode::Major,
        );
        let mut last = 0;
        for id in ["bg", "dust", "milkyway", "lines", "stars", "labels", "hud"] {
            let at = svg
                .find(&format!("<g id=\"{id}\""))
                .unwrap_or_else(|| panic!("{name:?}: no {id} layer"));
            assert!(at > last, "{name:?}: {id} is out of order");
            last = at;
        }
        if let Some(nebula) = svg.find("<g id=\"nebula\"") {
            assert!(
                nebula > svg.find("<g id=\"bg\"").unwrap()
                    && nebula < svg.find("<g id=\"dust\"").unwrap()
            );
        }
    }
}

#[test]
fn text_stays_readable_over_the_sky_in_every_theme() {
    let mut failures = Vec::new();
    for name in THEME_NAMES {
        let options = options_for(name);
        let theme = Theme::for_name(name);
        for fixture in FIXTURES {
            let (model, layout) = planned(fixture, &options);
            let check = text_contrast_check(
                &model,
                &layout,
                &theme,
                &ArtOptions::default(),
                LabelMode::Major,
            )
            .expect("renders");
            if !check.passed {
                failures.push(format!("{name:?} {fixture}: {}", check.detail));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn the_drawn_sky_checks_run_on_every_theme() {
    for name in THEME_NAMES {
        let options = options_for(name);
        let theme = Theme::for_name(name);
        for fixture in ["small", "medium", "edge"] {
            let (model, layout) = planned(fixture, &options);
            let report = drawn_checks(
                &model,
                &layout,
                &theme,
                &ArtOptions::default(),
                LabelMode::Major,
            )
            .expect("checks run");
            let what = format!("{name:?} {fixture}");
            for check in &report.hard {
                assert!(check.passed, "{what}: {} {}", check.name, check.detail);
            }
            for wanted in ["labels clear of rings", "flares clear of labels"] {
                assert!(
                    report.hard.iter().any(|c| c.name == wanted),
                    "{what}: no {wanted} check"
                );
            }
            println!("{what}");
            for check in &report.soft {
                println!(
                    "  [{}] {}: {}",
                    if check.passed { " ok " } else { "warn" },
                    check.name,
                    check.detail
                );
            }
            assert!(
                report.soft.iter().any(|c| c.name == "stars stand out"),
                "{what}"
            );
        }
    }
}

#[test]
fn non_finite_numbers_are_caught() {
    assert!(has_non_finite_number("<circle cx=\"NaN\" r=\"3\"/>"));
    assert!(has_non_finite_number(
        "<circle cx=\"10\" transform=\"rotate(inf 1 2)\"/>"
    ));
    assert!(!has_non_finite_number(
        "<g id=\"info\"><title>NaN and inf in text</title></g>"
    ));
}
