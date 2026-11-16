//! git-constellation: turn a GitHub account's public activity into a
//! hand-drawn-feeling night sky.
//!
//! Pipeline (each stage only knows about the one before it):
//! `fetch -> Activity -> SkyModel (rules) -> Layout -> art -> SVG -> PNG/GIF`.

// `pub mod x;` tells Rust to load `src/x.rs` (or `src/x/mod.rs`) as a module.
pub mod anim;
pub mod art;
pub mod card;
pub mod cli;
pub mod config;
pub mod export;
pub mod fetch;
pub mod layout;
pub mod lint;
pub mod model;
pub mod names;
pub mod preview;
pub mod rules;
pub mod stats;
pub mod svg;
pub mod theme;
pub mod timelapse;

pub use config::{Options, Overrides};
pub use layout::Layout;
pub use model::{Activity, SkyModel};

use model::{Moon, WeekDensity};
use rules::AchievementFacts;

/// Every error the library can return. Messages say what happened and what to do.
#[derive(Debug, thiserror::Error)]
pub enum SkyError {
    /// A problem in a config file; names the file and, where possible, the key.
    #[error("{file}: {message}")]
    Config { file: String, message: String },
    /// A bad value for a command-line option.
    #[error("{0}")]
    InvalidOption(String),
    /// The activity JSON could not be read.
    #[error(
        "could not read the activity JSON: {0}. Check that the file is valid JSON in the Activity format."
    )]
    Json(#[from] serde_json::Error),
    /// The activity parsed, but its contents cannot make a sky.
    #[error("the activity data cannot be used: {0}")]
    InvalidActivity(String),
    #[error("layout failed: {0}")]
    Layout(String),
    #[error("rendering failed: {0}")]
    Render(String),
    #[error("export failed: {0}")]
    Export(String),
}

/// Weekly contribution totals for the Milky Way, from the calendar (53 weeks for 365 days).
fn milkyway(days: &[model::DayCount]) -> Vec<WeekDensity> {
    let mut sorted = days.to_vec();
    sorted.sort_by_key(|day| day.date);
    let weeks: Vec<(chrono::NaiveDate, u32)> = sorted
        .chunks(7)
        .map(|week| (week[0].date, week.iter().map(|d| d.count).sum()))
        .collect();
    let busiest = weeks.iter().map(|(_, total)| *total).max().unwrap_or(0);
    weeks
        .into_iter()
        .map(|(week_start, total)| WeekDensity {
            week_start,
            total,
            density: if busiest == 0 {
                0.0
            } else {
                f64::from(total) / f64::from(busiest)
            },
        })
        .collect()
}

/// Decide *what* exists in the sky. Pure and deterministic: the same activity
/// and options always give the same model.
pub fn build_sky(activity: &Activity, options: &Options) -> Result<SkyModel, SkyError> {
    if activity.username.trim().is_empty() {
        return Err(SkyError::InvalidActivity(
            "the username is empty".to_string(),
        ));
    }
    let today = options.date.unwrap_or(activity.fetched_on);
    let seed = options
        .seed
        .unwrap_or_else(|| rules::seed_for_username(&activity.username));

    let selection = rules::select_stars(activity, options, today);
    let mut stars = selection.stars;
    let (constellations, wanderers) = names::group_constellations(&mut stars, seed, &options.names);

    let current_streak = rules::current_streak(&activity.days, today);
    let longest_streak = rules::longest_streak(&activity.days);
    let active_week = rules::active_days_in_last(&activity.days, today, rules::MOOD_WINDOW_DAYS);
    let facts = AchievementFacts {
        max_repo_stars: activity
            .repos
            .iter()
            .map(|repo| repo.stars)
            .max()
            .unwrap_or(0),
        longest_streak,
        current_streak,
        merged_prs_30d: activity.merged_prs_30d,
        constellations: constellations.len(),
    };

    Ok(SkyModel {
        username: activity.username.clone(),
        seed,
        stars,
        constellations,
        wanderers,
        dust_count: rules::dust_count(selection.overflow_count),
        milkyway: milkyway(&activity.days),
        moon: Moon {
            phase: rules::moon_phase(current_streak),
            mood: rules::moon_mood(active_week),
        },
        current_streak,
        longest_streak,
        shooting_stars: rules::shooting_star_count(activity.merged_prs_30d),
        sprites: rules::earned_achievements(&facts),
        overflow_count: selection.overflow_count,
    })
}

/// When a sky is too crowded to lay out, the star cap drops to this share
/// (4 of every 5) each try.
pub const CAP_FALLBACK_NUMERATOR: usize = 4;
pub const CAP_FALLBACK_DENOMINATOR: usize = 5;

/// The next, lower star cap to try, or `None` when it cannot go lower.
pub fn lower_star_cap(cap: usize) -> Option<usize> {
    if cap <= 1 {
        return None;
    }
    let reduced = cap * CAP_FALLBACK_NUMERATOR / CAP_FALLBACK_DENOMINATOR;
    Some(reduced.min(cap - 1).max(1))
}

/// Build the model and lay it out. If the sky is too crowded, lower the star
/// cap (the extra repos become dust) and try again, as Section 5.1 asks.
pub fn plan_sky(activity: &Activity, options: &Options) -> Result<(SkyModel, Layout), SkyError> {
    let mut cap = options.max_stars.min(rules::MAX_STARS);
    let mut lowered = false;
    loop {
        let trial = Options {
            max_stars: cap,
            ..options.clone()
        };
        let model = build_sky(activity, &trial)?;
        match layout::compute(&model, &trial) {
            Ok(mut layout) => {
                layout.star_cap = lowered.then_some(cap);
                return Ok((model, layout));
            }
            Err(error) => match lower_star_cap(cap) {
                Some(next) => {
                    cap = next;
                    lowered = true;
                }
                None => return Err(error),
            },
        }
    }
}
