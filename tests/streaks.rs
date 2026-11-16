//! Streak helpers (Section 4.8): every edge case.

mod common;

use common::{calendar, date, day, today};
use git_constellation::rules::{active_days_in_last, current_streak, longest_streak};

#[test]
fn empty_input_is_zero_everywhere() {
    assert_eq!(current_streak(&[], today()), 0);
    assert_eq!(longest_streak(&[]), 0);
    assert_eq!(active_days_in_last(&[], today(), 7), 0);
}

#[test]
fn a_single_active_day_today_is_a_streak_of_one() {
    let days = [day(today(), 3)];
    assert_eq!(current_streak(&days, today()), 1);
    assert_eq!(longest_streak(&days), 1);
}

#[test]
fn a_single_old_day_is_not_a_current_streak() {
    let days = [day(date(2026, 11, 1), 3)];
    assert_eq!(current_streak(&days, today()), 0);
    assert_eq!(longest_streak(&days), 1);
}

#[test]
fn zero_today_keeps_the_streak_alive_if_yesterday_was_active() {
    // 4 active days (10th to 13th), then nothing today (the 14th).
    let days = calendar(&[1, 1, 1, 1, 0]);
    assert_eq!(current_streak(&days, today()), 4);
}

#[test]
fn two_quiet_days_end_the_streak() {
    let days = calendar(&[5, 5, 5, 0, 0]);
    assert_eq!(current_streak(&days, today()), 0);
    assert_eq!(longest_streak(&days), 3);
}

#[test]
fn a_zero_day_in_the_middle_ends_the_run() {
    let days = calendar(&[1, 1, 0, 1, 1, 1]);
    assert_eq!(current_streak(&days, today()), 3);
    assert_eq!(longest_streak(&days), 3);
}

#[test]
fn a_missing_date_counts_as_a_gap() {
    let days = [
        day(date(2026, 11, 14), 1),
        day(date(2026, 11, 13), 1),
        // the 12th is missing entirely
        day(date(2026, 11, 11), 1),
    ];
    assert_eq!(current_streak(&days, today()), 2);
    assert_eq!(longest_streak(&days), 2);
}

#[test]
fn longest_streak_finds_the_best_run() {
    let days = calendar(&[1, 1, 1, 0, 1, 1, 1, 1, 1, 0, 1, 1]);
    assert_eq!(longest_streak(&days), 5);
    assert_eq!(current_streak(&days, today()), 2);
}

#[test]
fn a_streak_crosses_a_leap_day() {
    let days = [
        day(date(2024, 2, 28), 1),
        day(date(2024, 2, 29), 2),
        day(date(2024, 3, 1), 1),
    ];
    assert_eq!(current_streak(&days, date(2024, 3, 1)), 3);
    assert_eq!(longest_streak(&days), 3);
}

#[test]
fn skipping_the_leap_day_breaks_the_streak() {
    let days = [day(date(2024, 2, 28), 1), day(date(2024, 3, 1), 1)];
    assert_eq!(current_streak(&days, date(2024, 3, 1)), 1);
    assert_eq!(longest_streak(&days), 1);
}

#[test]
fn in_a_non_leap_year_march_first_follows_february_28th() {
    let days = [day(date(2023, 2, 28), 1), day(date(2023, 3, 1), 1)];
    assert_eq!(current_streak(&days, date(2023, 3, 1)), 2);
}

#[test]
fn all_zero_counts_give_zero() {
    let days = calendar(&[0, 0, 0, 0]);
    assert_eq!(current_streak(&days, today()), 0);
    assert_eq!(longest_streak(&days), 0);
    assert_eq!(active_days_in_last(&days, today(), 7), 0);
}

#[test]
fn input_order_and_duplicate_dates_do_not_matter() {
    let mut days = calendar(&[2, 0, 4, 1]);
    // Today appears twice: 0 + 3 still means "active".
    days.push(day(today(), 0));
    days.push(day(today(), 3));
    days.reverse();
    assert_eq!(current_streak(&days, today()), 2);
    assert_eq!(longest_streak(&days), 2);
}

#[test]
fn days_after_today_are_ignored_for_the_current_streak() {
    let days = [day(date(2026, 11, 15), 9), day(date(2026, 11, 13), 1)];
    assert_eq!(current_streak(&days, today()), 1);
}

#[test]
fn active_days_window_includes_today_and_excludes_the_day_before_the_window() {
    // Oldest first, 8 days: the oldest day (a 9) is outside a 7-day window but inside an 8-day one.
    let days = calendar(&[9, 0, 1, 0, 1, 1, 0, 1]);
    assert_eq!(active_days_in_last(&days, today(), 7), 4);
    assert_eq!(active_days_in_last(&days, today(), 8), 5);
    assert_eq!(active_days_in_last(&days, today(), 1), 1);
    assert_eq!(active_days_in_last(&days, today(), 0), 0);
}

#[test]
fn active_days_counts_each_date_once() {
    let days = [day(today(), 2), day(today(), 5)];
    assert_eq!(active_days_in_last(&days, today(), 7), 1);
}
