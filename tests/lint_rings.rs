//! Milestone 5 clean-up (FIX 3 and FIX 4): labels stay outside rings, flares stop before labels.

mod common;

use common::{FIXTURES, THEME_NAMES, planned};
use git_constellation::Options;
use git_constellation::art::labels::drawn_labels;
use git_constellation::art::{ArtOptions, sky_svg};
use git_constellation::layout::geometry::star_radius;
use git_constellation::layout::labels::{Label, LabelKind};
use git_constellation::lint::{flare_boxes, ring_flare_checks, ring_radius};
use git_constellation::model::{LabelMode, ThemeName, Tier};
use git_constellation::theme::Theme;

fn sky(
    fixture: &str,
    name: ThemeName,
    mode: LabelMode,
) -> (
    git_constellation::SkyModel,
    git_constellation::Layout,
    String,
) {
    let options = Options {
        theme: name,
        labels: mode,
        ..Options::default()
    };
    let (model, layout) = planned(fixture, &options);
    let svg = sky_svg(
        &model,
        &layout,
        &Theme::for_name(name),
        &ArtOptions::default(),
        mode,
    );
    (model, layout, svg)
}

fn fake_label(rect: git_constellation::layout::Rect) -> Label {
    Label {
        kind: LabelKind::Caption,
        text: "intruder".to_string(),
        rect,
        font_size: 10.5,
    }
}

#[test]
fn rings_and_flares_are_clear_of_labels_on_every_fixture_theme_and_mode() {
    for name in THEME_NAMES {
        for fixture in FIXTURES {
            for mode in [LabelMode::Major, LabelMode::All] {
                let (model, layout, svg) = sky(fixture, name, mode);
                for check in ring_flare_checks(&svg, &model, &layout, mode) {
                    assert!(
                        check.passed,
                        "{name:?} {fixture} {mode:?}: {} {}",
                        check.name, check.detail
                    );
                }
            }
        }
    }
}

#[test]
fn a_label_over_a_ring_fails_the_ring_check() {
    let (model, layout, svg) = sky("medium", ThemeName::Midnight, LabelMode::Major);
    let (i, star) = model
        .stars
        .iter()
        .enumerate()
        .find(|(_, s)| ring_radius(s).is_some())
        .expect("the medium sky has a ringed star");
    let r = ring_radius(star).unwrap();
    assert!(r > star_radius(star.tier), "a ring is wider than the star");
    let center = layout.positions[i];
    let mut bad = layout.clone();
    // A box that just touches the ring from the right.
    bad.labels.push(fake_label(git_constellation::layout::Rect {
        x0: center.0 + r - 2.0,
        y0: center.1 - 5.0,
        x1: center.0 + r + 40.0,
        y1: center.1 + 5.0,
    }));
    let checks = ring_flare_checks(&svg, &model, &bad, LabelMode::Major);
    let ring_check = checks
        .iter()
        .find(|c| c.name == "labels clear of rings")
        .unwrap();
    assert!(!ring_check.passed, "{}", ring_check.detail);
    assert!(ring_check.detail.contains("intruder"));
    // The same box outside the ring is fine.
    let mut fine = layout.clone();
    fine.labels
        .push(fake_label(git_constellation::layout::Rect {
            x0: center.0 + r + 1.0,
            y0: center.1 - 5.0,
            x1: center.0 + r + 40.0,
            y1: center.1 + 5.0,
        }));
    assert!(ring_flare_checks(&svg, &model, &fine, LabelMode::Major)[0].passed);
}

#[test]
fn a_label_over_a_flare_fails_the_flare_check() {
    for name in [ThemeName::Midnight, ThemeName::Paper] {
        let (model, layout, svg) = sky("medium", name, LabelMode::Major);
        let boxes = flare_boxes(&svg);
        assert!(
            !boxes.is_empty(),
            "{name:?}: the medium sky has a flared star"
        );
        let calm_tier_five = model
            .stars
            .iter()
            .filter(|s| s.tier == Tier::Five && !s.dormant && !s.borrowed)
            .count();
        assert!(boxes.len() >= calm_tier_five, "{name:?}");
        let mut bad = layout.clone();
        let b = boxes[0];
        bad.labels.push(fake_label(git_constellation::layout::Rect {
            x0: b.x0 - 1.0,
            y0: b.y0 - 1.0,
            x1: b.x1 + 1.0,
            y1: b.y1 + 1.0,
        }));
        let checks = ring_flare_checks(&svg, &model, &bad, LabelMode::Major);
        let flare = checks
            .iter()
            .find(|c| c.name == "flares clear of labels")
            .unwrap();
        assert!(!flare.passed, "{name:?}: {}", flare.detail);
    }
}

#[test]
fn no_label_is_checked_when_none_are_drawn() {
    let (model, layout, svg) = sky("medium", ThemeName::Midnight, LabelMode::Off);
    assert!(drawn_labels(&layout.labels, LabelMode::Off).is_empty());
    let checks = ring_flare_checks(&svg, &model, &layout, LabelMode::Off);
    assert!(checks.iter().all(|c| c.passed));
}
