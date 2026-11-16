//! The six fixtures (10.4) run through the whole rules pipeline.

mod common;

use std::collections::BTreeSet;

use common::{FIXTURES, date, fixture_text, load_fixture};
use git_constellation::model::{Achievement, LanguageKey, Mood, Phase, Tier};
use git_constellation::rules::seed_for_username;
use git_constellation::{Activity, Options, SkyModel, build_sky};

fn sky(name: &str) -> SkyModel {
    build_sky(&load_fixture(name), &Options::default()).expect("fixture builds")
}

#[test]
fn every_fixture_is_small_and_parses() {
    for name in FIXTURES {
        let text = fixture_text(name);
        assert!(text.len() < 200 * 1024, "{name} is {} bytes", text.len());
        let activity = Activity::from_json(&text).expect("parses");
        assert_eq!(activity.days.len(), 365, "{name} has a full year");
    }
}

#[test]
fn activity_and_sky_round_trip_through_json() {
    for name in FIXTURES {
        let activity = load_fixture(name);
        let json = serde_json::to_string(&activity).unwrap();
        assert_eq!(
            Activity::from_json(&json).unwrap(),
            activity,
            "{name} activity"
        );

        let model = sky(name);
        let json = serde_json::to_string(&model).unwrap();
        assert_eq!(
            serde_json::from_str::<SkyModel>(&json).unwrap(),
            model,
            "{name} model"
        );
    }
}

#[test]
fn bad_json_is_a_friendly_error() {
    let error = Activity::from_json("{ \"username\": 5 }")
        .unwrap_err()
        .to_string();
    assert!(
        error.contains("could not read the activity JSON"),
        "{error}"
    );
}

#[test]
fn build_sky_is_deterministic() {
    for name in FIXTURES {
        assert_eq!(sky(name), sky(name), "{name}");
    }
}

#[test]
fn repo_order_in_the_input_does_not_change_the_model() {
    for name in FIXTURES {
        let mut activity = load_fixture(name);
        let forward = build_sky(&activity, &Options::default()).unwrap();
        activity.repos.reverse();
        let backward = build_sky(&activity, &Options::default()).unwrap();
        assert_eq!(forward, backward, "{name}");
    }
}

#[test]
fn the_model_is_internally_consistent_for_every_fixture() {
    for name in FIXTURES {
        let model = sky(name);
        assert!(model.stars.len() <= 60, "{name}");
        assert!(model.dust_count <= 260, "{name}");
        assert!(model.constellations.len() <= 7, "{name}");
        assert!(
            model.shooting_stars <= 6 && model.sprites.len() <= 3,
            "{name}"
        );
        // Every star is in exactly one constellation or in the wanderers.
        let mut seen = BTreeSet::new();
        for constellation in &model.constellations {
            assert!(constellation.stars.len() >= 3, "{name}");
            for &i in &constellation.stars {
                assert!(seen.insert(i), "{name}: star {i} listed twice");
            }
        }
        for &i in &model.wanderers {
            assert!(seen.insert(i), "{name}: star {i} listed twice");
        }
        assert_eq!(seen.len(), model.stars.len(), "{name}");
        assert!(
            model.stars.windows(2).all(|w| w[0].score >= w[1].score),
            "{name}: sorted"
        );
        assert!(
            model.stars.iter().filter(|s| s.founding).count() <= 1,
            "{name}"
        );
        let names: BTreeSet<&str> = model
            .constellations
            .iter()
            .map(|c| c.name.as_str())
            .collect();
        assert_eq!(
            names.len(),
            model.constellations.len(),
            "{name}: unique names"
        );
        assert!(model.stars.iter().all(|s| s.score.is_finite()), "{name}");
    }
}

#[test]
fn empty_is_a_young_quiet_sky() {
    let model = sky("empty");
    assert!(model.stars.is_empty() && model.constellations.is_empty() && model.is_young());
    assert_eq!(
        (model.moon.phase, model.moon.mood),
        (Phase::New, Mood::Sleepy)
    );
    assert_eq!(model.milkyway.len(), 53);
    assert!(
        model
            .milkyway
            .iter()
            .all(|w| w.total == 0 && w.density == 0.0)
    );
    assert!(model.sprites.is_empty());
}

#[test]
fn tiny_is_one_star_and_a_crescent_moon() {
    let model = sky("tiny");
    assert_eq!(model.stars.len(), 1);
    assert!(model.is_young() && model.constellations.is_empty());
    // 1 commit + pushed today: 0.6*ln(2) + 0.8 = 1.22, so tier 2 by the formula.
    assert_eq!(model.stars[0].tier, Tier::Two);
    assert!(model.stars[0].fresh && model.stars[0].founding);
    assert_eq!(
        (model.current_streak, model.moon.phase, model.moon.mood),
        (1, Phase::Crescent, Mood::Content)
    );
}

#[test]
fn small_has_two_constellations_and_a_three_day_streak() {
    let model = sky("small");
    assert_eq!(model.stars.len(), 6);
    let languages: BTreeSet<LanguageKey> =
        model.constellations.iter().map(|c| c.language).collect();
    assert_eq!(
        languages,
        BTreeSet::from([LanguageKey::Python, LanguageKey::JsTs])
    );
    assert!(model.wanderers.is_empty() && !model.is_young());
    // Expected values were computed independently from the JSON with Python.
    assert_eq!((model.current_streak, model.longest_streak), (3, 4));
    assert_eq!(
        (model.moon.phase, model.moon.mood),
        (Phase::Crescent, Mood::Happy)
    );
}

#[test]
fn medium_matches_its_design() {
    let model = sky("medium");
    assert_eq!(model.stars.len(), 22, "24 repos minus 2 forks");
    assert_eq!(model.constellations.len(), 5);
    assert_eq!(
        model.wanderers.len(),
        2,
        "one unlabeled repo and one lone Java repo"
    );
    let found: Vec<&str> = model
        .stars
        .iter()
        .filter(|s| s.founding)
        .map(|s| s.repo.as_str())
        .collect();
    assert_eq!(found, ["old-homework"]);
    assert!(
        model
            .stars
            .iter()
            .any(|s| s.repo == "scrapebook" && s.dormant)
    );
    assert_eq!((model.current_streak, model.longest_streak), (12, 12));
    assert_eq!(
        (model.moon.phase, model.moon.mood),
        (Phase::Gibbous, Mood::Delighted)
    );
    assert_eq!(model.shooting_stars, 4);
    assert_eq!(
        model.sprites,
        [
            Achievement::Beacon,
            Achievement::Collaborator,
            Achievement::Cartographer
        ]
    );
    let names: BTreeSet<&str> = model
        .constellations
        .iter()
        .map(|c| c.name.as_str())
        .collect();
    for expected in [
        "The Little Crab",
        "The Gentle Serpent",
        "The Lantern",
        "The Gopher",
        "The Telescope",
    ] {
        assert!(names.contains(expected), "missing {expected}");
    }
}

#[test]
fn medium_with_forks_adds_borrowed_stars() {
    let options = Options {
        include_forks: true,
        ..Options::default()
    };
    let model = build_sky(&load_fixture("medium"), &options).unwrap();
    assert_eq!(model.stars.len(), 24);
    assert_eq!(model.stars.iter().filter(|s| s.borrowed).count(), 2);
    assert!(
        model
            .stars
            .iter()
            .filter(|s| s.borrowed)
            .all(|s| s.tier == Tier::One)
    );
}

#[test]
fn large_hits_the_cap_and_overflows() {
    let model = sky("large");
    assert_eq!(model.stars.len(), 60);
    assert_eq!(model.overflow_count, 142 - 60, "142 non-fork repos, 60 fit");
    assert_eq!(model.dust_count, 260);
    assert!((5..=7).contains(&model.constellations.len()));
    assert_eq!((model.current_streak, model.moon.phase), (45, Phase::Full));
    assert_eq!(model.shooting_stars, 6, "9 merged PRs, capped at 6");
    assert_eq!(
        model.sprites,
        [
            Achievement::Beacon,
            Achievement::Marathon,
            Achievement::Collaborator
        ]
    );
    assert_eq!(model.stars[0].tier, Tier::Five);
}

#[test]
fn edge_handles_unicode_ties_and_the_leap_day() {
    let model = sky("edge");
    // The seed ignores the username's case.
    assert_eq!(model.seed, seed_for_username("edge-tester"));
    // Unicode and very long names survive untouched.
    assert!(model.stars.iter().any(|s| s.repo == "日本語-notes"));
    assert!(model.stars.iter().any(|s| s.repo.chars().count() > 80));
    // Five Go repos have identical scores; byte order of names breaks the tie.
    let tied: Vec<&str> = model
        .stars
        .iter()
        .filter(|s| s.language == LanguageKey::Go)
        .map(|s| s.repo.as_str())
        .collect();
    assert_eq!(tied, ["Beta", "alpha", "alpha-2", "mid-tie", "zeta"]);
    // A 5-day streak that crosses 2024-02-29.
    assert_eq!(
        (model.current_streak, model.longest_streak, model.moon.phase),
        (5, 5, Phase::Half)
    );
    // The oldest non-fork repo (the long-named one) is the founding star.
    let founding: Vec<&str> = model
        .stars
        .iter()
        .filter(|s| s.founding)
        .map(|s| s.repo.as_str())
        .collect();
    assert_eq!(founding.len(), 1);
    assert!(founding[0].starts_with("a-very-long-repository-name"));
}

#[test]
fn the_date_option_moves_today() {
    let activity = load_fixture("small");
    let earlier = Options {
        date: Some(date(2026, 11, 13)),
        ..Options::default()
    };
    let model = build_sky(&activity, &earlier).unwrap();
    // Days 12 and 13 are still active; the 14th is "in the future" and ignored.
    assert_eq!(model.current_streak, 2);
}

#[test]
fn the_seed_option_overrides_the_username_seed_only() {
    let activity = load_fixture("medium");
    let plain = build_sky(&activity, &Options::default()).unwrap();
    let seeded = build_sky(
        &activity,
        &Options {
            seed: Some(7),
            ..Options::default()
        },
    )
    .unwrap();
    assert_eq!(plain.seed, seed_for_username("mira-codes"));
    assert_eq!(seeded.seed, 7);
    assert_eq!(plain.stars, seeded.stars);
}

#[test]
fn an_empty_username_is_rejected_kindly() {
    let mut activity = load_fixture("tiny");
    activity.username = "  ".to_string();
    let error = build_sky(&activity, &Options::default())
        .unwrap_err()
        .to_string();
    assert!(error.contains("username"), "{error}");
}

#[test]
fn the_milky_way_has_one_entry_per_week() {
    let model = sky("medium");
    assert_eq!(model.milkyway.len(), 53);
    let busiest = model.milkyway.iter().map(|w| w.density).fold(0.0, f64::max);
    assert!(
        (busiest - 1.0).abs() < 1e-12,
        "the busiest week has density 1"
    );
    assert!(
        model
            .milkyway
            .iter()
            .all(|w| (0.0..=1.0).contains(&w.density))
    );
    assert!(
        model
            .milkyway
            .windows(2)
            .all(|w| w[0].week_start < w[1].week_start)
    );
}

/// Star counts per tier, tier 1 first.
fn tier_histogram(model: &SkyModel) -> [usize; 5] {
    let mut counts = [0; 5];
    for star in &model.stars {
        counts[usize::from(star.tier.number()) - 1] += 1;
    }
    counts
}

#[test]
fn the_tier_mix_is_pinned_for_every_fixture() {
    // Pinned so a rule change that makes the sky "all the same size" is noticed.
    let expected = [
        ("empty", [0, 0, 0, 0, 0]),
        ("tiny", [0, 1, 0, 0, 0]),
        ("small", [1, 0, 3, 1, 1]),
        ("medium", [3, 2, 12, 4, 1]),
        ("large", [0, 0, 44, 12, 4]),
        ("edge", [0, 1, 9, 2, 1]),
    ];
    for (name, histogram) in expected {
        assert_eq!(tier_histogram(&sky(name)), histogram, "{name}");
    }
}

#[test]
fn flagship_tiers_stay_rare_in_every_fixture_and_every_mode() {
    let modes = [
        Options::default(),
        Options {
            include_forks: true,
            ..Options::default()
        },
    ];
    for options in &modes {
        for name in FIXTURES {
            let model = build_sky(&load_fixture(name), options).unwrap();
            let n = model.stars.len();
            let [_, _, _, fours, fives] = tier_histogram(&model);
            if n > 0 {
                assert!(
                    fives <= (n * 8 / 100).max(1),
                    "{name}: {fives} tier-5 of {n}"
                );
                assert!(
                    fours <= (n * 20 / 100).max(1),
                    "{name}: {fours} tier-4 of {n}"
                );
            }
        }
    }
}

#[test]
fn tiers_never_get_brighter_down_the_score_order() {
    for name in FIXTURES {
        let model = sky(name);
        let tiers: Vec<u8> = model
            .stars
            .iter()
            .filter(|s| !s.borrowed)
            .map(|s| s.tier.number())
            .collect();
        assert!(tiers.windows(2).all(|w| w[0] >= w[1]), "{name}: {tiers:?}");
    }
}
