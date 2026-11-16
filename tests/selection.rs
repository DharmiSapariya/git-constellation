//! Choosing stars (4.3), grouping them (4.4) and naming constellations.

mod common;

use common::{activity_with, date, repo, repo_in, today};
use git_constellation::model::{LanguageKey, Tier};
use git_constellation::names::{NAME_POOL, default_name, group_constellations, pick_name};
use git_constellation::rules::select_stars;
use git_constellation::{Options, build_sky};
use std::collections::{BTreeMap, BTreeSet};

fn names_of(stars: &[git_constellation::model::Star]) -> Vec<&str> {
    stars.iter().map(|s| s.repo.as_str()).collect()
}

#[test]
fn forks_are_left_out_by_default() {
    let mut fork = repo("a-fork");
    fork.is_fork = true;
    let activity = activity_with(vec![repo("mine"), fork]);
    let picked = select_stars(&activity, &Options::default(), today());
    assert_eq!(names_of(&picked.stars), ["mine"]);
}

#[test]
fn included_forks_become_borrowed_tier_one_stars() {
    let mut fork = repo("a-fork");
    fork.is_fork = true;
    fork.stars = 5000; // a huge score, but forks stay tier 1
    fork.forks = 99;
    let options = Options {
        include_forks: true,
        ..Options::default()
    };
    let picked = select_stars(&activity_with(vec![fork]), &options, today());
    let star = &picked.stars[0];
    assert!(star.borrowed);
    assert_eq!(star.tier, Tier::One);
    assert_eq!(star.orbit_moons, 0);
}

#[test]
fn archived_repos_are_included_but_dormant() {
    let mut old = repo("archived-one");
    old.is_archived = true;
    old.last_pushed = today();
    let picked = select_stars(&activity_with(vec![old]), &Options::default(), today());
    assert_eq!(picked.stars.len(), 1);
    assert!(picked.stars[0].dormant);
}

#[test]
fn stars_are_sorted_best_first() {
    let repos = vec![
        repo_in("low", None, 1),
        repo_in("high", None, 90),
        repo_in("mid", None, 9),
    ];
    let picked = select_stars(&activity_with(repos), &Options::default(), today());
    assert_eq!(names_of(&picked.stars), ["high", "mid", "low"]);
}

#[test]
fn the_cap_keeps_the_best_and_counts_the_overflow() {
    let repos: Vec<_> = (0..70)
        .map(|i| repo_in(&format!("repo-{i:02}"), None, i))
        .collect();
    let activity = activity_with(repos);

    let picked = select_stars(&activity, &Options::default(), today());
    assert_eq!(picked.stars.len(), 60);
    assert_eq!(picked.overflow_count, 10);
    assert_eq!(picked.stars[0].repo, "repo-69");

    let small = Options {
        max_stars: 5,
        ..Options::default()
    };
    let picked = select_stars(&activity, &small, today());
    assert_eq!((picked.stars.len(), picked.overflow_count), (5, 65));

    // Asking for more than 60 is clamped to 60.
    let greedy = Options {
        max_stars: 500,
        ..Options::default()
    };
    assert_eq!(select_stars(&activity, &greedy, today()).stars.len(), 60);
}

#[test]
fn equal_scores_are_ordered_by_name_whatever_the_input_order() {
    let forward = activity_with(vec![repo("c"), repo("a"), repo("b")]);
    let backward = activity_with(vec![repo("b"), repo("c"), repo("a")]);
    let options = Options::default();
    let one = select_stars(&forward, &options, today());
    let two = select_stars(&backward, &options, today());
    assert_eq!(names_of(&one.stars), ["a", "b", "c"]);
    assert_eq!(one, two);
}

#[test]
fn exclusions_beat_inclusions_and_ignore_case() {
    let activity = activity_with(vec![repo("a"), repo("b"), repo("c")]);
    let options = Options {
        include: vec!["a".into(), "B".into()],
        exclude: vec!["b".into()],
        ..Options::default()
    };
    let picked = select_stars(&activity, &options, today());
    assert_eq!(names_of(&picked.stars), ["a"]);
    assert_eq!(picked.overflow_count, 0);
}

#[test]
fn the_founding_star_is_the_oldest_non_fork_repo() {
    let mut oldest_fork = repo("ancient-fork");
    oldest_fork.is_fork = true;
    oldest_fork.created = date(2015, 1, 1);
    let mut oldest = repo("zz-first");
    oldest.created = date(2019, 5, 5);
    let mut tied = repo("aa-also-first");
    tied.created = date(2019, 5, 5);
    let newer = repo("newer");
    let options = Options {
        include_forks: true,
        ..Options::default()
    };
    let picked = select_stars(
        &activity_with(vec![oldest_fork, oldest, newer, tied]),
        &options,
        today(),
    );

    let founders: Vec<&str> = picked
        .stars
        .iter()
        .filter(|s| s.founding)
        .map(|s| s.repo.as_str())
        .collect();
    assert_eq!(founders, ["aa-also-first"]); // tie broken by name
}

#[test]
fn no_repos_means_no_founding_star() {
    let picked = select_stars(&activity_with(vec![]), &Options::default(), today());
    assert!(picked.stars.is_empty());
    assert_eq!(picked.overflow_count, 0);
}

fn grouped(repos: Vec<git_constellation::model::Repo>) -> git_constellation::SkyModel {
    build_sky(&activity_with(repos), &Options::default()).expect("sky builds")
}

#[test]
fn three_stars_make_a_constellation_and_fewer_become_wanderers() {
    let mut repos = Vec::new();
    for i in 0..3 {
        repos.push(repo_in(&format!("py-{i}"), Some("Python"), 5));
    }
    repos.push(repo_in("go-1", Some("Go"), 5));
    repos.push(repo_in("go-2", Some("Go"), 5));
    repos.push(repo_in("rs-1", Some("Rust"), 5));
    let sky = grouped(repos);

    assert_eq!(sky.constellations.len(), 1);
    assert_eq!(sky.constellations[0].language, LanguageKey::Python);
    assert_eq!(sky.constellations[0].name, "The Gentle Serpent");
    assert_eq!(sky.wanderers.len(), 3);
    assert!(
        sky.wanderers.windows(2).all(|w| w[0] < w[1]),
        "wanderers are sorted"
    );
    for &index in &sky.constellations[0].stars {
        assert_eq!(sky.stars[index].constellation, Some(0));
    }
    for &index in &sky.wanderers {
        assert_eq!(sky.stars[index].constellation, None);
    }
}

#[test]
fn the_other_group_needs_three_stars_and_gets_a_pool_name() {
    let two = grouped(vec![repo_in("a", None, 1), repo_in("b", None, 1)]);
    assert!(two.constellations.is_empty());
    assert_eq!(two.wanderers.len(), 2);

    let three = grouped(vec![
        repo_in("a", None, 1),
        repo_in("b", None, 1),
        repo_in("c", Some("Zig"), 1),
    ]);
    assert_eq!(three.constellations.len(), 1);
    assert!(NAME_POOL.contains(&three.constellations[0].name.as_str()));
}

#[test]
fn at_most_seven_constellations_and_the_strongest_survive() {
    // Nine groups of three. Group g has stars = 10 * g, so higher g is brighter.
    let languages = [
        "Rust",
        "Python",
        "Go",
        "TypeScript",
        "Java",
        "C",
        "HTML",
        "Shell",
        "Jupyter Notebook",
    ];
    let mut repos = Vec::new();
    for (g, language) in languages.iter().enumerate() {
        for i in 0..3 {
            repos.push(repo_in(
                &format!("{language}-{i}"),
                Some(language),
                10 * g as u32,
            ));
        }
    }
    let sky = grouped(repos);
    assert_eq!(sky.constellations.len(), 7);
    assert_eq!(sky.wanderers.len(), 6); // the two weakest groups, 3 stars each
    let kept: BTreeSet<LanguageKey> = sky.constellations.iter().map(|c| c.language).collect();
    assert!(!kept.contains(&LanguageKey::Rust) && !kept.contains(&LanguageKey::Python));
    // Brightest first.
    assert_eq!(sky.constellations[0].language, LanguageKey::Jupyter);
}

#[test]
fn custom_names_win_over_defaults() {
    let mut custom = BTreeMap::new();
    custom.insert(LanguageKey::Rust, "Ferris's Cove".to_string());
    let options = Options {
        names: custom,
        ..Options::default()
    };
    let repos: Vec<_> = (0..3)
        .map(|i| repo_in(&format!("r{i}"), Some("Rust"), 1))
        .collect();
    let sky = build_sky(&activity_with(repos), &options).expect("sky builds");
    assert_eq!(sky.constellations[0].name, "Ferris's Cove");
}

#[test]
fn a_clashing_name_falls_back_to_the_pool() {
    let lantern = default_name(LanguageKey::JsTs).expect("JS has a default name");
    let taken = vec![lantern.to_string()];
    let name = pick_name(LanguageKey::JsTs, 1, 7, &taken, None);
    assert_ne!(name, lantern);
    assert!(NAME_POOL.contains(&name.as_str()));
}

#[test]
fn pool_names_are_stable_distinct_and_plenty() {
    assert!(NAME_POOL.len() >= 40);
    let unique: BTreeSet<&&str> = NAME_POOL.iter().collect();
    assert_eq!(unique.len(), NAME_POOL.len(), "no duplicate names");
    assert!(NAME_POOL.iter().all(|name| name.starts_with("The ")));

    let a = pick_name(LanguageKey::Other, 3, 12345, &[], None);
    let b = pick_name(LanguageKey::Other, 3, 12345, &[], None);
    assert_eq!(a, b);
    let different_seeds: BTreeSet<String> = (0..10)
        .map(|seed| pick_name(LanguageKey::Other, 0, seed, &[], None))
        .collect();
    assert!(
        different_seeds.len() > 3,
        "the seed really changes the pick"
    );
}

#[test]
fn naming_never_repeats_within_one_sky() {
    // A custom name that equals another group's default forces a real clash:
    // Python is called "The Lantern", so the JavaScript group (default
    // "The Lantern") must fall back to a pool name.
    let mut custom = BTreeMap::new();
    custom.insert(LanguageKey::Python, "The Lantern".to_string());
    let options = Options {
        names: custom,
        ..Options::default()
    };
    let mut repos = Vec::new();
    for language in ["Python", "JavaScript"] {
        repos.extend((0..3).map(|i| repo_in(&format!("{language}-{i}"), Some(language), 3)));
    }
    let sky = build_sky(&activity_with(repos), &options).expect("sky builds");
    assert_eq!(sky.constellations.len(), 2);
    let names: BTreeSet<&str> = sky.constellations.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(names.len(), 2, "both names are different");
    assert!(names.contains("The Lantern"));
}

#[test]
fn grouping_works_directly_on_a_star_slice() {
    let mut stars = select_stars(
        &activity_with(
            (0..4)
                .map(|i| repo_in(&format!("g{i}"), Some("Go"), 2))
                .collect(),
        ),
        &Options::default(),
        today(),
    )
    .stars;
    let (constellations, wanderers) = group_constellations(&mut stars, 1, &BTreeMap::new());
    assert_eq!(constellations.len(), 1);
    assert_eq!(constellations[0].stars.len(), 4);
    assert!(wanderers.is_empty());
    assert!(constellations[0].edges.is_empty() && constellations[0].anchor.is_none());
}
