//! Activity, Repo, SkyModel and the enums they use.

use std::str::FromStr;

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use crate::SkyError;
use crate::rules::YOUNG_SKY_BELOW_STARS;

/// A point on the canvas, as `(x, y)` in SVG pixels.
pub type Point = (f64, f64);

// ---------------------------------------------------------------------------
// Input data (what we fetch from GitHub, or load from a saved JSON file)
// ---------------------------------------------------------------------------

/// Contributions on one calendar day.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DayCount {
    pub date: NaiveDate,
    pub count: u32,
}

/// One public repository.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Repo {
    pub name: String,
    pub created: NaiveDate,
    pub last_pushed: NaiveDate,
    pub stars: u32,
    pub forks: u32,
    pub is_fork: bool,
    pub is_archived: bool,
    pub primary_language: Option<String>,
    // `#[serde(default)]` means "if the JSON has no such key, use the empty value".
    #[serde(default)]
    pub languages: Vec<String>,
    #[serde(default)]
    pub topics: Vec<String>,
    pub commits_last_year: u32,
    #[serde(default)]
    pub open_issues: u32,
    /// Per-day commit counts, only needed for the timelapse.
    #[serde(default)]
    pub daily_commits: Option<Vec<DayCount>>,
}

/// Everything we know about one account. All computation works from this.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Activity {
    pub username: String,
    pub account_created: NaiveDate,
    pub fetched_on: NaiveDate,
    pub days: Vec<DayCount>,
    pub merged_prs_30d: u32,
    pub closed_issues_30d: u32,
    pub repos: Vec<Repo>,
    #[serde(default)]
    pub private_contribution_count: Option<u32>,
}

impl Activity {
    /// Parse an activity file. Never touches the network.
    ///
    /// ```
    /// use git_constellation::Activity;
    /// assert!(Activity::from_json("not json").is_err());
    /// ```
    pub fn from_json(json: &str) -> Result<Self, SkyError> {
        // The `?` operator returns early on error, converting it with `From`.
        Ok(serde_json::from_str(json)?)
    }
}

// ---------------------------------------------------------------------------
// Small enums
// ---------------------------------------------------------------------------

/// Brightness class of a star, 1 (tiny) to 5 (flagship).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Tier {
    One = 1,
    Two,
    Three,
    Four,
    Five,
}

impl Tier {
    /// The tier as a plain number, 1 to 5.
    pub fn number(self) -> u8 {
        self as u8
    }
}

/// Moon phase, driven by the current streak.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Phase {
    New,
    Crescent,
    Half,
    Gibbous,
    Full,
}

/// Moon mood, driven by active days in the last week.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Mood {
    Sleepy,
    Content,
    Happy,
    Delighted,
}

/// The moon mascot's state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Moon {
    pub phase: Phase,
    pub mood: Mood,
}

/// Achievements, in priority order (earlier variants win when only three fit).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Achievement {
    Beacon,
    Marathon,
    Collaborator,
    Cartographer,
    StreakKeeper,
}

/// Which labels to draw.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LabelMode {
    #[serde(rename = "none")]
    Off,
    #[serde(rename = "major")]
    Major,
    #[serde(rename = "all")]
    All,
}

impl FromStr for LabelMode {
    type Err = SkyError;

    fn from_str(text: &str) -> Result<Self, SkyError> {
        match text.trim().to_lowercase().as_str() {
            "none" => Ok(LabelMode::Off),
            "major" => Ok(LabelMode::Major),
            "all" => Ok(LabelMode::All),
            other => Err(SkyError::InvalidOption(format!(
                "unknown label mode `{other}`. Choose one of: none, major, all."
            ))),
        }
    }
}

/// The six built-in themes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThemeName {
    Midnight,
    Dusk,
    Aurora,
    Sakura,
    Paper,
    Mono,
}

/// Only a label: every sky is a night sky, and `paper` is the day version.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TimeLabel {
    Night,
    Day,
}

impl ThemeName {
    /// All theme names, as typed on the command line.
    pub const NAMES: [&'static str; 6] = ["midnight", "dusk", "aurora", "sakura", "paper", "mono"];

    pub fn time_label(self) -> TimeLabel {
        match self {
            ThemeName::Paper => TimeLabel::Day,
            _ => TimeLabel::Night,
        }
    }
}

impl FromStr for ThemeName {
    type Err = SkyError;

    fn from_str(text: &str) -> Result<Self, SkyError> {
        match text.trim().to_lowercase().as_str() {
            "midnight" => Ok(ThemeName::Midnight),
            "dusk" => Ok(ThemeName::Dusk),
            "aurora" => Ok(ThemeName::Aurora),
            "sakura" => Ok(ThemeName::Sakura),
            "paper" => Ok(ThemeName::Paper),
            "mono" => Ok(ThemeName::Mono),
            other => Err(SkyError::InvalidOption(format!(
                "unknown theme `{other}`. Choose one of: {}.",
                ThemeName::NAMES.join(", ")
            ))),
        }
    }
}

// ---------------------------------------------------------------------------
// Languages
// ---------------------------------------------------------------------------

/// Language group. Repos in one group share a color and a constellation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum LanguageKey {
    Rust,
    Python,
    JsTs,
    Go,
    JavaKotlin,
    CFamily,
    HtmlCss,
    Shell,
    Jupyter,
    Other,
}

/// Which lowercase language names belong to which group (data, easy to edit).
pub const LANGUAGE_GROUPS: &[(LanguageKey, &[&str])] = &[
    (LanguageKey::Rust, &["rust"]),
    (LanguageKey::Python, &["python"]),
    (LanguageKey::JsTs, &["javascript", "typescript"]),
    (LanguageKey::Go, &["go"]),
    (LanguageKey::JavaKotlin, &["java", "kotlin"]),
    (LanguageKey::CFamily, &["c", "c++", "c#"]),
    (LanguageKey::HtmlCss, &["html", "css"]),
    (LanguageKey::Shell, &["shell", "makefile"]),
    (LanguageKey::Jupyter, &["jupyter notebook", "jupyter", "r"]),
    (LanguageKey::Other, &["other"]),
];

impl LanguageKey {
    /// Exact lookup: `None` for a language we do not know. Case-insensitive.
    pub fn from_name_strict(name: &str) -> Option<LanguageKey> {
        let wanted = name.trim().to_lowercase();
        LANGUAGE_GROUPS
            .iter()
            .find(|(_, aliases)| aliases.contains(&wanted.as_str()))
            .map(|(key, _)| *key)
    }

    /// Lenient lookup used for repos: anything unknown, or no language, is `Other`.
    pub fn from_language(language: Option<&str>) -> LanguageKey {
        language
            .and_then(LanguageKey::from_name_strict)
            .unwrap_or(LanguageKey::Other)
    }

    /// Every language name we accept, for friendly error messages.
    pub fn known_names() -> Vec<&'static str> {
        LANGUAGE_GROUPS
            .iter()
            .flat_map(|(_, aliases)| aliases.iter().copied())
            .collect()
    }
}

// ---------------------------------------------------------------------------
// The sky model (what exists, before we know where it goes)
// ---------------------------------------------------------------------------

/// One repo as a star.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Star {
    pub repo: String,
    pub language: LanguageKey,
    pub tier: Tier,
    pub score: f64,
    pub dormant: bool,
    pub founding: bool,
    pub fresh: bool,
    /// A fork, drawn as a hollow "borrowed star".
    pub borrowed: bool,
    pub orbit_moons: u8,
    /// Filled in by the layout engine.
    pub position: Option<Point>,
    pub constellation: Option<usize>,
}

/// A named group of stars joined by lines.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Constellation {
    pub name: String,
    pub language: LanguageKey,
    /// Indices into `SkyModel::stars`.
    pub stars: Vec<usize>,
    /// Pairs of star indices; filled in by the layout engine.
    pub edges: Vec<(usize, usize)>,
    pub anchor: Option<Point>,
    pub radius: Option<f64>,
}

/// One week of the Milky Way band.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WeekDensity {
    pub week_start: NaiveDate,
    pub total: u32,
    /// `total` divided by the busiest week, so 0.0 to 1.0.
    pub density: f64,
}

/// Everything that exists in the sky. No positions yet, and no colors.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SkyModel {
    pub username: String,
    pub seed: u64,
    pub stars: Vec<Star>,
    pub constellations: Vec<Constellation>,
    /// Indices of stars that belong to no constellation.
    pub wanderers: Vec<usize>,
    pub dust_count: usize,
    pub milkyway: Vec<WeekDensity>,
    pub moon: Moon,
    pub current_streak: u32,
    pub longest_streak: u32,
    pub shooting_stars: u32,
    pub sprites: Vec<Achievement>,
    pub overflow_count: usize,
}

impl SkyModel {
    /// Fewer than three stars: draw the gentle "a young sky" cluster instead.
    pub fn is_young(&self) -> bool {
        self.stars.len() < YOUNG_SKY_BELOW_STARS
    }
}
