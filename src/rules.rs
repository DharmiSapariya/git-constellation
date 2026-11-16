//! Rules that turn data into sky decisions, and every tunable constant.

use chrono::NaiveDate;

use crate::config::Options;
use crate::model::{Achievement, Activity, LanguageKey, Mood, Phase, Repo, Star, Tier};

mod streaks;

pub use streaks::{active_days_in_last, current_streak, longest_streak};

// ---- Score (4.2) ----
/// Weight of the `ln(1 + stars)` term.
pub const STARS_WEIGHT: f64 = 1.0;
/// Weight of the `ln(1 + commits_last_year)` term.
pub const COMMITS_WEIGHT: f64 = 0.6;
/// Recency bonus for a push within `RECENT_HIGH_DAYS`.
pub const RECENT_HIGH_BONUS: f64 = 0.8;
/// Days since the last push that still earn the high bonus (inclusive).
pub const RECENT_HIGH_DAYS: i64 = 14;
/// Recency bonus for a push within `RECENT_LOW_DAYS`.
pub const RECENT_LOW_BONUS: f64 = 0.3;
/// Days since the last push that still earn the low bonus (inclusive).
pub const RECENT_LOW_DAYS: i64 = 60;

// ---- Tiers (4.2): inclusive lower bounds on the score; tier 1 is below tier 2's bound ----
pub const TIER_2_MIN: f64 = 1.0;
pub const TIER_3_MIN: f64 = 2.0;
pub const TIER_4_MIN: f64 = 3.2;
pub const TIER_5_MIN: f64 = 4.5;
/// At most this share of all stars may be tier 5 (but always at least one).
pub const TIER_5_MAX_SHARE: f64 = 0.08;
/// At most this share of all stars may be tier 4 (but always at least one).
pub const TIER_4_MAX_SHARE: f64 = 0.20;

// ---- Which repos become stars (4.3) ----
/// Hard cap on stars in one sky.
pub const MAX_STARS: usize = 60;

// ---- Constellations (4.4) ----
/// A language group needs at least this many stars to become a constellation.
pub const MIN_CONSTELLATION_STARS: usize = 3;
/// At most this many constellations; weaker groups become wanderers.
pub const MAX_CONSTELLATIONS: usize = 7;
/// Below this many stars in total the sky is "a young sky".
pub const YOUNG_SKY_BELOW_STARS: usize = 3;

// ---- Dormancy, freshness, orbit moons (4.5) ----
/// Dormant when the last push is more than this many days ago.
pub const DORMANT_AFTER_DAYS: i64 = 180;
/// Fresh when pushed today or yesterday (dates only, so "48 hours" is 1 day).
pub const FRESH_WITHIN_DAYS: i64 = 1;
/// One orbit moon per this many forks, rounded up.
pub const FORKS_PER_ORBIT_MOON: u32 = 3;
/// Never more than this many orbit moons on a star.
pub const MAX_ORBIT_MOONS: u8 = 3;
/// Orbit moons only appear on stars of this tier or brighter.
pub const ORBIT_MIN_TIER: Tier = Tier::Three;

// ---- Moon (4.6): streak lengths where the phase changes ----
pub const PHASE_CRESCENT_MIN_STREAK: u32 = 1;
pub const PHASE_HALF_MIN_STREAK: u32 = 4;
pub const PHASE_GIBBOUS_MIN_STREAK: u32 = 10;
pub const PHASE_FULL_MIN_STREAK: u32 = 30;
/// Mood looks at this many recent days.
pub const MOOD_WINDOW_DAYS: u32 = 7;
/// Active days (out of the window) where the mood changes.
pub const MOOD_CONTENT_MIN_DAYS: u32 = 1;
pub const MOOD_HAPPY_MIN_DAYS: u32 = 4;
pub const MOOD_DELIGHTED_MIN_DAYS: u32 = 7;

// ---- Shooting stars and achievements (4.7) ----
pub const MAX_SHOOTING_STARS: u32 = 6;
pub const MAX_SPRITES: usize = 3;
pub const BEACON_MIN_STARS: u32 = 50;
pub const MARATHON_MIN_STREAK: u32 = 30;
pub const CARTOGRAPHER_MIN_CONSTELLATIONS: usize = 5;
pub const STREAK_KEEPER_MIN_STREAK: u32 = 7;

// ---- Dust (4.3, 2.4) ----
/// Dust circles in every sky.
pub const BASE_DUST_COUNT: usize = 200;
/// Dust never exceeds this, even with overflow stars added.
pub const MAX_DUST_COUNT: usize = 260;

// ---- Seed (4.9): FNV-1a, 64-bit ----
const FNV_OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

// ===========================================================================
// Score, tier and star flags
// ===========================================================================

/// The three parts of a score, kept separate so `explain` can show them.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScoreBreakdown {
    pub stars_term: f64,
    pub commits_term: f64,
    pub recency_bonus: f64,
}

impl ScoreBreakdown {
    pub fn total(&self) -> f64 {
        self.stars_term + self.commits_term + self.recency_bonus
    }
}

/// Whole days from `date` to `today`, never negative.
pub fn days_since(today: NaiveDate, date: NaiveDate) -> i64 {
    (today - date).num_days().max(0)
}

/// Bonus for pushing recently.
pub fn recency_bonus(days_since_push: i64) -> f64 {
    if days_since_push <= RECENT_HIGH_DAYS {
        RECENT_HIGH_BONUS
    } else if days_since_push <= RECENT_LOW_DAYS {
        RECENT_LOW_BONUS
    } else {
        0.0
    }
}

/// Score a repo. `ln_1p(x)` is `ln(1 + x)`, computed accurately for small x.
pub fn score_breakdown(repo: &Repo, today: NaiveDate) -> ScoreBreakdown {
    ScoreBreakdown {
        stars_term: STARS_WEIGHT * f64::from(repo.stars).ln_1p(),
        commits_term: COMMITS_WEIGHT * f64::from(repo.commits_last_year).ln_1p(),
        recency_bonus: recency_bonus(days_since(today, repo.last_pushed)),
    }
}

/// Map a score to a tier using the inclusive lower bounds above.
pub fn tier_for_score(score: f64) -> Tier {
    if score >= TIER_5_MIN {
        Tier::Five
    } else if score >= TIER_4_MIN {
        Tier::Four
    } else if score >= TIER_3_MIN {
        Tier::Three
    } else if score >= TIER_2_MIN {
        Tier::Two
    } else {
        Tier::One
    }
}

/// How many stars may hold a tier: `floor(share * total)`, never below one.
/// The tiny epsilon stops float noise (like 0.2 * 15 = 3.0000000000000004)
/// from changing the answer.
pub fn tier_cap(share: f64, total: usize) -> usize {
    ((share * total as f64 + 1e-9).floor() as usize).max(1)
}

/// Keep the sky from being all flagship stars. `tiers` must be in score order
/// (best first). Extra tier-5 stars drop to tier 4, then extra tier-4 stars
/// (including the ones just demoted) drop to tier 3.
pub fn apply_tier_caps(tiers: &mut [Tier]) {
    let total = tiers.len();
    for (from, to, share) in [
        (Tier::Five, Tier::Four, TIER_5_MAX_SHARE),
        (Tier::Four, Tier::Three, TIER_4_MAX_SHARE),
    ] {
        let cap = tier_cap(share, total);
        let mut seen = 0;
        for tier in tiers.iter_mut().filter(|tier| **tier == from) {
            seen += 1;
            if seen > cap {
                *tier = to;
            }
        }
    }
}

pub fn is_dormant(repo: &Repo, today: NaiveDate) -> bool {
    repo.is_archived || days_since(today, repo.last_pushed) > DORMANT_AFTER_DAYS
}

pub fn is_fresh(repo: &Repo, today: NaiveDate) -> bool {
    days_since(today, repo.last_pushed) <= FRESH_WITHIN_DAYS
}

/// Tiny orbiting dots for forks: `min(3, ceil(forks / 3))`, tier 3 and up only.
pub fn orbit_moons(forks: u32, tier: Tier) -> u8 {
    if tier < ORBIT_MIN_TIER {
        return 0;
    }
    let wanted = forks.div_ceil(FORKS_PER_ORBIT_MOON);
    // `try_from` converts u32 to u8 and fails (instead of wrapping) if too big.
    u8::try_from(wanted)
        .unwrap_or(MAX_ORBIT_MOONS)
        .min(MAX_ORBIT_MOONS)
}

fn star_for_repo(repo: &Repo, today: NaiveDate) -> Star {
    let score = score_breakdown(repo, today).total();
    // Forks are "borrowed stars": always the smallest tier.
    let tier = if repo.is_fork {
        Tier::One
    } else {
        tier_for_score(score)
    };
    Star {
        repo: repo.name.clone(),
        language: LanguageKey::from_language(repo.primary_language.as_deref()),
        tier,
        score,
        dormant: is_dormant(repo, today),
        founding: false,
        fresh: is_fresh(repo, today),
        borrowed: repo.is_fork,
        orbit_moons: orbit_moons(repo.forks, tier),
        position: None,
        constellation: None,
    }
}

// ===========================================================================
// Choosing the stars (4.3)
// ===========================================================================

/// The chosen stars, best first, and how many eligible repos did not fit.
#[derive(Debug, Clone, PartialEq)]
pub struct Selection {
    pub stars: Vec<Star>,
    pub overflow_count: usize,
}

fn name_listed(list: &[String], name: &str) -> bool {
    list.iter().any(|item| item.eq_ignore_ascii_case(name))
}

/// Pick which repos become stars. Exclusions beat inclusions.
pub fn select_stars(activity: &Activity, options: &Options, today: NaiveDate) -> Selection {
    let mut eligible: Vec<(&Repo, Star)> = activity
        .repos
        .iter()
        .filter(|repo| !name_listed(&options.exclude, &repo.name))
        .filter(|repo| options.include.is_empty() || name_listed(&options.include, &repo.name))
        .filter(|repo| !repo.is_fork || options.include_forks)
        .map(|repo| (repo, star_for_repo(repo, today)))
        .collect();

    // Best score first; equal scores fall back to the name so the order is stable.
    eligible.sort_by(|a, b| {
        b.1.score
            .total_cmp(&a.1.score)
            .then_with(|| a.0.name.cmp(&b.0.name))
    });

    let cap = options.max_stars.min(MAX_STARS);
    let overflow_count = eligible.len().saturating_sub(cap);
    eligible.truncate(cap);

    // Brightness is relative: only a few stars per sky may be flagships.
    let mut tiers: Vec<Tier> = eligible.iter().map(|(_, star)| star.tier).collect();
    apply_tier_caps(&mut tiers);
    for ((repo, star), tier) in eligible.iter_mut().zip(tiers) {
        star.tier = tier;
        star.orbit_moons = orbit_moons(repo.forks, tier);
    }

    // The founding ("first light") star: oldest non-fork repo that made the cut.
    let founding = eligible
        .iter()
        .enumerate()
        .filter(|(_, (repo, _))| !repo.is_fork)
        .min_by(|(_, (a, _)), (_, (b, _))| {
            a.created.cmp(&b.created).then_with(|| a.name.cmp(&b.name))
        })
        .map(|(index, _)| index);

    let mut stars: Vec<Star> = eligible.into_iter().map(|(_, star)| star).collect();
    if let Some(index) = founding {
        stars[index].founding = true;
    }
    Selection {
        stars,
        overflow_count,
    }
}

/// Dust circles: the base amount plus one per overflow star, capped.
pub fn dust_count(overflow_count: usize) -> usize {
    (BASE_DUST_COUNT + overflow_count).min(MAX_DUST_COUNT)
}

// ===========================================================================
// Moon, shooting stars, achievements
// ===========================================================================

pub fn moon_phase(streak: u32) -> Phase {
    if streak >= PHASE_FULL_MIN_STREAK {
        Phase::Full
    } else if streak >= PHASE_GIBBOUS_MIN_STREAK {
        Phase::Gibbous
    } else if streak >= PHASE_HALF_MIN_STREAK {
        Phase::Half
    } else if streak >= PHASE_CRESCENT_MIN_STREAK {
        Phase::Crescent
    } else {
        Phase::New
    }
}

pub fn moon_mood(active_days_last_week: u32) -> Mood {
    if active_days_last_week >= MOOD_DELIGHTED_MIN_DAYS {
        Mood::Delighted
    } else if active_days_last_week >= MOOD_HAPPY_MIN_DAYS {
        Mood::Happy
    } else if active_days_last_week >= MOOD_CONTENT_MIN_DAYS {
        Mood::Content
    } else {
        Mood::Sleepy
    }
}

/// One shooting star per merged pull request in the last 30 days, capped.
pub fn shooting_star_count(merged_prs_30d: u32) -> u32 {
    merged_prs_30d.min(MAX_SHOOTING_STARS)
}

/// The numbers the achievement rules look at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AchievementFacts {
    pub max_repo_stars: u32,
    pub longest_streak: u32,
    pub current_streak: u32,
    pub merged_prs_30d: u32,
    pub constellations: usize,
}

/// Earned achievements in priority order, at most `MAX_SPRITES`.
pub fn earned_achievements(facts: &AchievementFacts) -> Vec<Achievement> {
    let checks = [
        (
            Achievement::Beacon,
            facts.max_repo_stars >= BEACON_MIN_STARS,
        ),
        (
            Achievement::Marathon,
            facts.longest_streak >= MARATHON_MIN_STREAK,
        ),
        (Achievement::Collaborator, facts.merged_prs_30d >= 1),
        (
            Achievement::Cartographer,
            facts.constellations >= CARTOGRAPHER_MIN_CONSTELLATIONS,
        ),
        (
            Achievement::StreakKeeper,
            facts.current_streak >= STREAK_KEEPER_MIN_STREAK,
        ),
    ];
    checks
        .into_iter()
        .filter(|(_, earned)| *earned)
        .map(|(achievement, _)| achievement)
        .take(MAX_SPRITES)
        .collect()
}

// ===========================================================================
// Seed
// ===========================================================================

/// 64-bit FNV-1a hash, written by hand because `DefaultHasher` may change
/// between Rust versions and our output must never change.
pub fn fnv1a_64(bytes: &[u8]) -> u64 {
    let mut hash = FNV_OFFSET_BASIS;
    for byte in bytes {
        hash ^= u64::from(*byte);
        // `wrapping_mul` overflows silently, which is exactly what FNV wants.
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

/// The default seed: FNV-1a of the lowercase username.
pub fn seed_for_username(username: &str) -> u64 {
    fnv1a_64(username.to_lowercase().as_bytes())
}
