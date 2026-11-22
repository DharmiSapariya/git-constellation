//! Composition of small skies (FIX 6): numbers for every fixture and for synthetic skies.

mod common;

use common::{FIXTURES, SYNTHETIC, medium_with_languages, planned};
use git_constellation::lint::{Composition, measure_composition};
use git_constellation::{Options, plan_sky};

fn row(name: &str, c: &Composition) -> String {
    let show =
        |v: Option<f64>, scale: f64| v.map_or("  -".to_string(), |x| format!("{:3.0}", x * scale));
    format!(
        "{name:<22} stars {:>3}  const {}  worst gap {}px  width {}%  height {}%  off-center {:.1}% / {:.1}%",
        c.stars,
        c.constellations,
        show(c.worst_gap, 1.0),
        show(c.width_share, 100.0),
        show(c.height_share, 100.0),
        c.offset.0 * 100.0,
        c.offset.1 * 100.0
    )
}

/// The most any pair of constellations may be apart (closest stars), by sky, as measured after
/// FIX 6. Before: small 561, edge 463, two 454, three 531, four 637. The 300 px target applies to
/// sparse skies (12 stars or fewer, three constellations at most); the larger ones are pinned
/// near what the layout reaches, so they cannot quietly get worse again.
const GAP_LIMITS: [(&str, f64); 6] = [
    ("small", 300.0),
    ("edge", 300.0),
    ("two-constellations", 300.0),
    ("three-constellations", 300.0),
    ("four-constellations", 420.0),
    ("medium", 906.0),
];

fn sky_of(name: &str) -> (git_constellation::SkyModel, git_constellation::Layout) {
    let options = Options::default();
    if let Some((_, languages)) = SYNTHETIC.iter().find(|(n, _)| *n == name) {
        plan_sky(&medium_with_languages(languages), &options).expect("plans")
    } else {
        planned(name, &options)
    }
}

#[test]
fn small_skies_are_composed_close_to_the_middle() {
    for (name, limit) in GAP_LIMITS {
        let (model, layout) = sky_of(name);
        let c = measure_composition(&model, &layout);
        let gap = c.worst_gap.expect("two or more constellations");
        assert!(
            gap <= limit,
            "{name}: the most distant pair is {gap:.0}px apart (limit {limit})"
        );
        if name != "medium" {
            let width = c.width_share.unwrap();
            assert!(
                width >= 0.45 - 1e-9,
                "{name}: {:.0}% of the width",
                width * 100.0
            );
            assert!(
                c.offset.0 <= 0.12 && c.offset.1 <= 0.15,
                "{name}: off-center {:?}",
                c.offset
            );
        }
    }
}

#[test]
fn every_hard_check_still_passes_on_the_synthetic_skies() {
    use git_constellation::art::ArtOptions;
    use git_constellation::lint::{drawn_checks, lint};
    use git_constellation::model::{LabelMode, ThemeName};
    use git_constellation::theme::Theme;
    for (name, languages) in SYNTHETIC {
        let (model, layout) =
            plan_sky(&medium_with_languages(languages), &Options::default()).expect("plans");
        let report = lint(&model, &layout);
        assert!(
            report.hard.iter().all(|c| c.passed),
            "{name}: {:?}",
            report.hard.iter().filter(|c| !c.passed).collect::<Vec<_>>()
        );
        let drawn = drawn_checks(
            &model,
            &layout,
            &Theme::for_name(ThemeName::Midnight),
            &ArtOptions::default(),
            LabelMode::Major,
        )
        .expect("runs");
        assert!(
            drawn.hard.iter().all(|c| c.passed),
            "{name}: {:?}",
            drawn.hard.iter().filter(|c| !c.passed).collect::<Vec<_>>()
        );
    }
}

/// Prints the composition table. Run: cargo test --test composition -- --nocapture
#[test]
fn composition_table() {
    let options = Options::default();
    for fixture in FIXTURES {
        let (model, layout) = planned(fixture, &options);
        println!("{}", row(fixture, &measure_composition(&model, &layout)));
    }
    for (name, languages) in SYNTHETIC {
        let (model, layout) = plan_sky(&medium_with_languages(languages), &options).expect("plans");
        println!("{}", row(name, &measure_composition(&model, &layout)));
    }
}
