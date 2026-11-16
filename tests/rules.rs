//! Rule functions (Section 4): score, tier, dormancy, moon, achievements, seed.

mod common;

use common::{date, days_ago, repo, today};
use git_constellation::model::{Achievement, LanguageKey, Mood, Phase, Tier};
use git_constellation::rules::*;

fn approx(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

#[test]
fn score_is_the_documented_formula() {
    let mut r = repo("x");
    r.stars = 7;
    r.commits_last_year = 10;
    r.last_pushed = days_ago(3);
    let parts = score_breakdown(&r, today());
    assert!(approx(parts.stars_term, 8f64.ln()));
    assert!(approx(parts.commits_term, 0.6 * 11f64.ln()));
    assert!(approx(parts.recency_bonus, 0.8));
    assert!(approx(parts.total(), 8f64.ln() + 0.6 * 11f64.ln() + 0.8));
}

#[test]
fn recency_bonus_boundaries_are_inclusive() {
    assert!(approx(recency_bonus(0), 0.8));
    assert!(approx(recency_bonus(14), 0.8));
    assert!(approx(recency_bonus(15), 0.3));
    assert!(approx(recency_bonus(60), 0.3));
    assert!(approx(recency_bonus(61), 0.0));
}

#[test]
fn a_push_dated_after_today_counts_as_today() {
    assert_eq!(days_since(today(), date(2026, 12, 25)), 0);
    assert_eq!(days_since(today(), date(2026, 11, 4)), 10);
}

#[test]
fn tier_boundaries_have_inclusive_lower_bounds() {
    let cases = [
        (0.0, Tier::One),
        (0.999, Tier::One),
        (1.0, Tier::Two),
        (1.999, Tier::Two),
        (2.0, Tier::Three),
        (3.199, Tier::Three),
        (3.2, Tier::Four),
        (4.499, Tier::Four),
        (4.5, Tier::Five),
        (99.0, Tier::Five),
    ];
    for (score, tier) in cases {
        assert_eq!(tier_for_score(score), tier, "score {score}");
    }
}

#[test]
fn a_never_pushed_unstarred_repo_is_tier_one() {
    // Created today and "pushed" today (the most generous case): score 0.8.
    let mut r = repo("empty-repo");
    r.created = today();
    r.last_pushed = today();
    assert_eq!(
        tier_for_score(score_breakdown(&r, today()).total()),
        Tier::One
    );
}

#[test]
fn dormancy_starts_after_180_days_or_when_archived() {
    let mut r = repo("old");
    r.last_pushed = days_ago(180);
    assert!(!is_dormant(&r, today()));
    r.last_pushed = days_ago(181);
    assert!(is_dormant(&r, today()));
    r.last_pushed = today();
    r.is_archived = true;
    assert!(is_dormant(&r, today()));
}

#[test]
fn freshness_is_today_or_yesterday() {
    let mut r = repo("new");
    for (ago, fresh) in [(0, true), (1, true), (2, false), (30, false)] {
        r.last_pushed = days_ago(ago);
        assert_eq!(is_fresh(&r, today()), fresh, "{ago} days ago");
    }
}

#[test]
fn orbit_moons_follow_ceil_forks_over_three_capped_at_three() {
    let cases = [
        (0, 0),
        (1, 1),
        (3, 1),
        (4, 2),
        (6, 2),
        (7, 3),
        (9, 3),
        (10, 3),
        (1000, 3),
    ];
    for (forks, moons) in cases {
        assert_eq!(orbit_moons(forks, Tier::Three), moons, "{forks} forks");
    }
    assert_eq!(orbit_moons(u32::MAX, Tier::Five), 3);
}

#[test]
fn orbit_moons_only_appear_on_tier_three_and_up() {
    assert_eq!(orbit_moons(30, Tier::One), 0);
    assert_eq!(orbit_moons(30, Tier::Two), 0);
    assert_eq!(orbit_moons(30, Tier::Four), 3);
}

#[test]
fn moon_phase_boundaries() {
    let cases = [
        (0, Phase::New),
        (1, Phase::Crescent),
        (3, Phase::Crescent),
        (4, Phase::Half),
        (9, Phase::Half),
        (10, Phase::Gibbous),
        (29, Phase::Gibbous),
        (30, Phase::Full),
        (1000, Phase::Full),
    ];
    for (streak, phase) in cases {
        assert_eq!(moon_phase(streak), phase, "streak {streak}");
    }
}

#[test]
fn moon_mood_boundaries() {
    let cases = [
        (0, Mood::Sleepy),
        (1, Mood::Content),
        (3, Mood::Content),
        (4, Mood::Happy),
        (6, Mood::Happy),
        (7, Mood::Delighted),
    ];
    for (days, mood) in cases {
        assert_eq!(moon_mood(days), mood, "{days} active days");
    }
}

#[test]
fn shooting_stars_are_capped_at_six() {
    let cases = [(0, 0), (3, 3), (6, 6), (7, 6), (100, 6)];
    for (prs, stars) in cases {
        assert_eq!(shooting_star_count(prs), stars);
    }
}

fn no_facts() -> AchievementFacts {
    AchievementFacts {
        max_repo_stars: 0,
        longest_streak: 0,
        current_streak: 0,
        merged_prs_30d: 0,
        constellations: 0,
    }
}

#[test]
fn each_achievement_has_an_exact_threshold() {
    let one = |facts: AchievementFacts| earned_achievements(&facts);
    assert!(one(no_facts()).is_empty());
    assert!(
        one(AchievementFacts {
            max_repo_stars: 49,
            ..no_facts()
        })
        .is_empty()
    );
    assert_eq!(
        one(AchievementFacts {
            max_repo_stars: 50,
            ..no_facts()
        }),
        [Achievement::Beacon]
    );
    assert!(
        one(AchievementFacts {
            longest_streak: 29,
            ..no_facts()
        })
        .is_empty()
    );
    assert_eq!(
        one(AchievementFacts {
            longest_streak: 30,
            ..no_facts()
        }),
        [Achievement::Marathon]
    );
    assert_eq!(
        one(AchievementFacts {
            merged_prs_30d: 1,
            ..no_facts()
        }),
        [Achievement::Collaborator]
    );
    assert!(
        one(AchievementFacts {
            constellations: 4,
            ..no_facts()
        })
        .is_empty()
    );
    assert_eq!(
        one(AchievementFacts {
            constellations: 5,
            ..no_facts()
        }),
        [Achievement::Cartographer]
    );
    assert!(
        one(AchievementFacts {
            current_streak: 6,
            ..no_facts()
        })
        .is_empty()
    );
    assert_eq!(
        one(AchievementFacts {
            current_streak: 7,
            ..no_facts()
        }),
        [Achievement::StreakKeeper]
    );
}

#[test]
fn achievements_keep_priority_order_and_stop_at_three() {
    let everything = AchievementFacts {
        max_repo_stars: 500,
        longest_streak: 99,
        current_streak: 99,
        merged_prs_30d: 9,
        constellations: 7,
    };
    assert_eq!(
        earned_achievements(&everything),
        [
            Achievement::Beacon,
            Achievement::Marathon,
            Achievement::Collaborator
        ]
    );
    let late_ones = AchievementFacts {
        constellations: 5,
        current_streak: 7,
        ..no_facts()
    };
    assert_eq!(
        earned_achievements(&late_ones),
        [Achievement::Cartographer, Achievement::StreakKeeper]
    );
}

#[test]
fn fnv1a_matches_the_published_test_vectors() {
    assert_eq!(fnv1a_64(b""), 0xcbf2_9ce4_8422_2325);
    assert_eq!(fnv1a_64(b"a"), 0xaf63_dc4c_8601_ec8c);
    assert_eq!(fnv1a_64(b"foobar"), 0x8594_4171_f739_67e8);
}

#[test]
fn the_seed_is_pinned_and_ignores_username_case() {
    // Reference value computed independently in Python, not by this code.
    assert_eq!(seed_for_username("octocat"), 0x3b73_d0f1_7996_7e56);
    assert_eq!(seed_for_username("OctoCat"), seed_for_username("octocat"));
    assert_eq!(seed_for_username("octocat"), 4_283_997_406_105_075_286);
}

#[test]
fn languages_map_to_groups_whatever_the_casing() {
    let cases = [
        (Some("Rust"), LanguageKey::Rust),
        (Some("rUsT"), LanguageKey::Rust),
        (Some("  Go "), LanguageKey::Go),
        (Some("TYPESCRIPT"), LanguageKey::JsTs),
        (Some("JavaScript"), LanguageKey::JsTs),
        (Some("Kotlin"), LanguageKey::JavaKotlin),
        (Some("C++"), LanguageKey::CFamily),
        (Some("c#"), LanguageKey::CFamily),
        (Some("CSS"), LanguageKey::HtmlCss),
        (Some("Makefile"), LanguageKey::Shell),
        (Some("Jupyter Notebook"), LanguageKey::Jupyter),
        (Some("R"), LanguageKey::Jupyter),
        (Some("Brainfuck"), LanguageKey::Other),
        (Some(""), LanguageKey::Other),
        (None, LanguageKey::Other),
    ];
    for (language, key) in cases {
        assert_eq!(LanguageKey::from_language(language), key, "{language:?}");
    }
}

#[test]
fn strict_lookup_rejects_unknown_languages() {
    assert_eq!(LanguageKey::from_name_strict("brainfuck"), None);
    assert_eq!(
        LanguageKey::from_name_strict("Other"),
        Some(LanguageKey::Other)
    );
}

#[test]
fn dust_count_adds_overflow_but_never_exceeds_the_cap() {
    assert_eq!(dust_count(0), 200);
    assert_eq!(dust_count(40), 240);
    assert_eq!(dust_count(60), 260);
    assert_eq!(dust_count(100_000), 260);
}

#[test]
fn tier_cap_is_floor_of_the_share_with_a_minimum_of_one() {
    assert_eq!(tier_cap(TIER_5_MAX_SHARE, 1), 1);
    assert_eq!(tier_cap(TIER_5_MAX_SHARE, 12), 1); // 0.96 floors to 0, minimum 1
    assert_eq!(tier_cap(TIER_5_MAX_SHARE, 25), 2); // 0.08 * 25 = 2.0 exactly
    assert_eq!(tier_cap(TIER_5_MAX_SHARE, 60), 4); // 4.8 floors to 4
    assert_eq!(tier_cap(TIER_4_MAX_SHARE, 15), 3); // 0.2 * 15 is 3.0000000000000004
    assert_eq!(tier_cap(TIER_4_MAX_SHARE, 22), 4);
    assert_eq!(tier_cap(TIER_4_MAX_SHARE, 60), 12);
}

#[test]
fn caps_demote_the_extras_one_tier_in_score_order() {
    // 22 stars, best first: eleven 5s, seven 4s, four 3s (the old medium fixture).
    let mut tiers = [
        vec![Tier::Five; 11],
        vec![Tier::Four; 7],
        vec![Tier::Three; 4],
    ]
    .concat();
    apply_tier_caps(&mut tiers);
    let count = |t: Tier| tiers.iter().filter(|x| **x == t).count();
    assert_eq!(
        (count(Tier::Five), count(Tier::Four), count(Tier::Three)),
        (1, 4, 17)
    );
    // The brightest star keeps tier 5; demotions happen from the back of the queue.
    assert_eq!(tiers[0], Tier::Five);
    assert_eq!(tiers[1], Tier::Four);
    assert_eq!(tiers[4], Tier::Four);
    assert_eq!(tiers[5], Tier::Three);
}

#[test]
fn caps_never_remove_the_only_flagship_and_never_invent_tiers() {
    let mut one = [Tier::Five];
    apply_tier_caps(&mut one);
    assert_eq!(one, [Tier::Five]);

    let mut none = [Tier::Three, Tier::Two, Tier::One];
    apply_tier_caps(&mut none);
    assert_eq!(none, [Tier::Three, Tier::Two, Tier::One]);

    let mut empty: [Tier; 0] = [];
    apply_tier_caps(&mut empty);
}
