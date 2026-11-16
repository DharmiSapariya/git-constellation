//! Shared helpers for the integration tests (not a test file itself).
#![allow(dead_code)] // each test file uses a different subset of these helpers

use std::path::PathBuf;

use chrono::NaiveDate;
use git_constellation::Activity;
use git_constellation::model::{DayCount, Repo};

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
