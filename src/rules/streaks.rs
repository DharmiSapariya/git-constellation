//! Streak helpers (4.8): pure functions over the contribution calendar.

use std::collections::BTreeMap;

use chrono::{Days, NaiveDate};

use crate::model::DayCount;

/// Sum counts per date. A `BTreeMap` iterates in date order, whatever the input order.
fn counts_by_date(days: &[DayCount]) -> BTreeMap<NaiveDate, u32> {
    let mut counts = BTreeMap::new();
    for day in days {
        let slot = counts.entry(day.date).or_insert(0u32);
        *slot = slot.saturating_add(day.count);
    }
    counts
}

/// Consecutive active days ending today. A zero today does not break the
/// streak if yesterday was active.
pub fn current_streak(days: &[DayCount], today: NaiveDate) -> u32 {
    let counts = counts_by_date(days);
    let is_active = |date: NaiveDate| counts.get(&date).is_some_and(|count| *count > 0);

    let mut cursor = if is_active(today) {
        Some(today)
    } else {
        today.pred_opt().filter(|yesterday| is_active(*yesterday))
    };
    let mut streak = 0;
    while let Some(date) = cursor.filter(|date| is_active(*date)) {
        streak += 1;
        cursor = date.pred_opt();
    }
    streak
}

/// The longest run of consecutive active days anywhere in the calendar.
pub fn longest_streak(days: &[DayCount]) -> u32 {
    let mut best = 0;
    let mut run = 0;
    let mut previous: Option<NaiveDate> = None;
    for (date, count) in counts_by_date(days) {
        if count == 0 {
            run = 0;
            previous = None;
            continue;
        }
        run = match previous {
            Some(prev) if prev.succ_opt() == Some(date) => run + 1,
            _ => 1,
        };
        previous = Some(date);
        best = best.max(run);
    }
    best
}

/// Distinct active days in the `n` days ending today (today included).
pub fn active_days_in_last(days: &[DayCount], today: NaiveDate, n: u32) -> u32 {
    if n == 0 {
        return 0;
    }
    let start = today
        .checked_sub_days(Days::new(u64::from(n - 1)))
        .unwrap_or(NaiveDate::MIN);
    let active = counts_by_date(days)
        .range(start..=today)
        .filter(|(_, count)| **count > 0)
        .count();
    u32::try_from(active).unwrap_or(u32::MAX)
}
