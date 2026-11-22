//! Shared helpers for the integration tests (not a test file itself).
#![allow(dead_code)] // each test file uses a different subset of these helpers

use std::fmt::Write as _;
use std::path::PathBuf;

use chrono::NaiveDate;
use git_constellation::model::{DayCount, Repo, ThemeName};
use git_constellation::{Activity, Layout, Options, SkyModel, plan_sky};

/// All six themes, for matrix tests.
pub const THEME_NAMES: [ThemeName; 6] = [
    ThemeName::Midnight,
    ThemeName::Dusk,
    ThemeName::Aurora,
    ThemeName::Sakura,
    ThemeName::Paper,
    ThemeName::Mono,
];

/// A fixture's model and layout, planned the way the real renderer plans them.
pub fn planned(fixture: &str, options: &Options) -> (SkyModel, Layout) {
    plan_sky(&load_fixture(fixture), options).unwrap_or_else(|e| panic!("{fixture}: {e}"))
}

/// The options for one theme, everything else default.
pub fn options_for(theme: ThemeName) -> Options {
    Options {
        theme,
        ..Options::default()
    }
}

/// The six fixture names from Section 10.4.
pub const FIXTURES: [&str; 6] = ["empty", "tiny", "small", "medium", "large", "edge"];

pub fn date(year: i32, month: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(year, month, day).expect("valid test date")
}

/// "Today" for hand-built test data.
pub fn today() -> NaiveDate {
    date(2026, 11, 14)
}

pub fn days_ago(n: u64) -> NaiveDate {
    today() - chrono::Days::new(n)
}

/// A quiet, old, unstarred Rust repo; tests change only the fields they care about.
pub fn repo(name: &str) -> Repo {
    Repo {
        name: name.to_string(),
        created: date(2023, 1, 1),
        last_pushed: date(2024, 1, 1),
        stars: 0,
        forks: 0,
        is_fork: false,
        is_archived: false,
        primary_language: Some("Rust".to_string()),
        languages: vec!["Rust".to_string()],
        topics: Vec::new(),
        commits_last_year: 0,
        open_issues: 0,
        daily_commits: None,
    }
}

/// A repo in a given language with a given star count (to control its score).
pub fn repo_in(name: &str, language: Option<&str>, stars: u32) -> Repo {
    Repo {
        primary_language: language.map(str::to_string),
        stars,
        ..repo(name)
    }
}

pub fn day(date: NaiveDate, count: u32) -> DayCount {
    DayCount { date, count }
}

/// Days ending `today`, one per `counts` entry, oldest first.
pub fn calendar(counts: &[u32]) -> Vec<DayCount> {
    let len = counts.len() as u64;
    counts
        .iter()
        .enumerate()
        .map(|(i, count)| day(today() - chrono::Days::new(len - 1 - i as u64), *count))
        .collect()
}

pub fn activity_with(repos: Vec<Repo>) -> Activity {
    Activity {
        username: "tester".to_string(),
        account_created: date(2022, 1, 1),
        fetched_on: today(),
        days: Vec::new(),
        merged_prs_30d: 0,
        closed_issues_30d: 0,
        repos,
        private_contribution_count: None,
    }
}

pub fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(format!("{name}.json"))
}

pub fn fixture_text(name: &str) -> String {
    std::fs::read_to_string(fixture_path(name)).expect("fixture file should exist")
}

pub fn load_fixture(name: &str) -> Activity {
    Activity::from_json(&fixture_text(name)).expect("fixture should parse")
}

/// `medium.json` reduced to the repos whose primary language is one of `languages`: a small
/// sky with (usually) one constellation per language.
pub fn medium_with_languages(languages: &[&str]) -> Activity {
    let mut activity = load_fixture("medium");
    activity.repos.retain(|repo| {
        repo.primary_language
            .as_deref()
            .is_some_and(|language| languages.contains(&language))
    });
    activity
}

/// The synthetic sparse skies used to check small-sky composition: label and language groups.
pub const SYNTHETIC: [(&str, &[&str]); 4] = [
    ("one-constellation", &["Rust"]),
    ("two-constellations", &["Rust", "Python"]),
    ("three-constellations", &["Rust", "Python", "TypeScript"]),
    (
        "four-constellations",
        &["Rust", "Python", "TypeScript", "Go"],
    ),
];

/// Where review images go: `PREVIEW_DIR` if set, otherwise `target/previews` (never committed).
pub fn review_dir() -> PathBuf {
    let dir = match std::env::var("PREVIEW_DIR") {
        Ok(dir) => PathBuf::from(dir),
        Err(_) => PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("previews"),
    };
    std::fs::create_dir_all(&dir).expect("review folder");
    dir
}

/// The linter's full report for every fixture on the midnight theme: the layout checks and the
/// checks on the drawn sky (contrast, rings, flares, star visibility, brightness).
pub fn lint_report_text() -> String {
    use git_constellation::art::ArtOptions;
    use git_constellation::lint::{Check, drawn_checks, lint};
    use git_constellation::model::{LabelMode, ThemeName};
    use git_constellation::theme::Theme;

    let list = |out: &mut String, title: &str, checks: &[Check]| {
        let _ = writeln!(out, "{title}");
        for c in checks {
            let _ = writeln!(
                out,
                "  [{}] {}: {}",
                if c.passed { "pass" } else { "warn" },
                c.name,
                c.detail
            );
        }
    };
    let mut out = String::from(
        "Linter report: every fixture, midnight theme, default options.\n\
         Layout checks first, then the checks on the drawn sky.\n\n",
    );
    let options = options_for(ThemeName::Midnight);
    let theme = Theme::for_name(ThemeName::Midnight);
    for fixture in FIXTURES {
        let (model, layout) = planned(fixture, &options);
        let _ = writeln!(
            out,
            "=== {fixture}: {} stars, {} constellations, {} loose, layout try #{}, scale {:.2}",
            model.stars.len(),
            layout.constellations.len(),
            layout.wanderers.len(),
            layout.attempt + 1,
            layout.scale
        );
        let report = lint(&model, &layout);
        list(&mut out, "HARD (layout)", &report.hard);
        list(&mut out, "SOFT (layout)", &report.soft);
        let drawn = drawn_checks(
            &model,
            &layout,
            &theme,
            &ArtOptions::default(),
            LabelMode::Major,
        )
        .expect("drawn checks run");
        list(&mut out, "HARD (drawn sky)", &drawn.hard);
        list(&mut out, "SOFT (drawn sky)", &drawn.soft);
        out.push('\n');
    }
    out
}
