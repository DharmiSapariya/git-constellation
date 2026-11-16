//! constellation.toml loading and merging with CLI flags.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use crate::SkyError;
use crate::model::{LabelMode, LanguageKey, ThemeName};
use crate::rules::MAX_STARS;

/// The config file we look for when `--config` is not given.
pub const DEFAULT_CONFIG_FILE: &str = "constellation.toml";
/// Smallest and largest allowed canvas side, in pixels.
pub const MIN_CANVAS_SIDE: u32 = 200;
pub const MAX_CANVAS_SIDE: u32 = 4000;

/// The file as written. Every key is optional; unknown keys are rejected.
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FileConfig {
    pub theme: Option<ThemeName>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub include_forks: Option<bool>,
    pub max_stars: Option<usize>,
    pub labels: Option<LabelMode>,
    pub faces: Option<bool>,
    pub paper_grain: Option<bool>,
    pub exclude: Option<Vec<String>>,
    pub include: Option<Vec<String>>,
    pub names: Option<BTreeMap<String, String>>,
    pub colors: Option<BTreeMap<String, String>>,
}

/// Values given on the command line. They beat the config file.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Overrides {
    pub config: Option<PathBuf>,
    pub theme: Option<ThemeName>,
    pub date: Option<NaiveDate>,
    pub seed: Option<u64>,
    pub labels: Option<LabelMode>,
    pub include_forks: Option<bool>,
    pub faces: Option<bool>,
    pub static_mode: bool,
}

/// Final, validated settings: defaults, then the file, then the command line.
#[derive(Debug, Clone, PartialEq)]
pub struct Options {
    pub theme: ThemeName,
    pub width: u32,
    pub height: u32,
    pub include_forks: bool,
    pub max_stars: usize,
    pub labels: LabelMode,
    pub faces: bool,
    pub paper_grain: bool,
    pub exclude: Vec<String>,
    pub include: Vec<String>,
    pub names: BTreeMap<LanguageKey, String>,
    pub colors: BTreeMap<LanguageKey, String>,
    pub seed: Option<u64>,
    /// Replaces the activity's `fetched_on` as "today".
    pub date: Option<NaiveDate>,
    /// Render with no animation at all.
    pub static_mode: bool,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            theme: ThemeName::Midnight,
            width: 1200,
            height: 600,
            include_forks: false,
            max_stars: MAX_STARS,
            labels: LabelMode::Major,
            faces: true,
            paper_grain: false,
            exclude: Vec::new(),
            include: Vec::new(),
            names: BTreeMap::new(),
            colors: BTreeMap::new(),
            seed: None,
            date: None,
            static_mode: false,
        }
    }
}

fn config_error(file: &str, message: String) -> SkyError {
    SkyError::Config {
        file: file.to_string(),
        message,
    }
}

/// Parse config text. `file` is only used to name the file in error messages.
pub fn parse_config(source: &str, file: &str) -> Result<FileConfig, SkyError> {
    toml::from_str(source).map_err(|error| {
        // Turn the byte offset of the problem into a line number.
        let line = error
            .span()
            .and_then(|span| source.get(..span.start))
            .map(|before| before.matches('\n').count() + 1);
        let message = match line {
            Some(line) => format!("line {line}: {}", error.message().trim()),
            None => error.message().trim().to_string(),
        };
        config_error(file, message)
    })
}

/// Read and parse a config file from disk.
pub fn load_config_file(path: &Path) -> Result<FileConfig, SkyError> {
    let name = path.display().to_string();
    let text = std::fs::read_to_string(path).map_err(|error| {
        config_error(
            &name,
            format!("could not read the file ({error}). Check that the path is right."),
        )
    })?;
    parse_config(&text, &name)
}

fn check_color(value: &str) -> Result<(), String> {
    let digits = value.strip_prefix('#').unwrap_or("");
    if digits.len() == 6 && digits.chars().all(|c| c.is_ascii_hexdigit()) {
        Ok(())
    } else {
        Err("expected a hex color like #ffb38a".to_string())
    }
}

fn check_name(value: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        Err("the name must not be empty".to_string())
    } else {
        Ok(())
    }
}

/// Turn a `[names]` or `[colors]` table keyed by language text into one keyed
/// by `LanguageKey`, checking every key and value.
fn language_table(
    table: &str,
    raw: &BTreeMap<String, String>,
    file: &str,
    check_value: fn(&str) -> Result<(), String>,
) -> Result<BTreeMap<LanguageKey, String>, SkyError> {
    let mut out = BTreeMap::new();
    for (language, value) in raw {
        let key = LanguageKey::from_name_strict(language).ok_or_else(|| {
            config_error(
                file,
                format!(
                    "[{table}] has an unknown language `{language}`. Known languages: {}.",
                    LanguageKey::known_names().join(", ")
                ),
            )
        })?;
        check_value(value).map_err(|why| {
            config_error(file, format!("[{table}] {language} = \"{value}\": {why}"))
        })?;
        out.insert(key, value.clone());
    }
    Ok(out)
}

impl Options {
    /// Resolve settings the way the CLI does: `--config` if given (it must
    /// exist), else `constellation.toml` in the current folder if present.
    pub fn resolve(overrides: &Overrides) -> Result<Options, SkyError> {
        let default_path = Path::new(DEFAULT_CONFIG_FILE);
        let path = match &overrides.config {
            Some(path) => Some(path.as_path()),
            None if default_path.is_file() => Some(default_path),
            None => None,
        };
        let file = match path {
            Some(path) => Some((path.display().to_string(), load_config_file(path)?)),
            None => None,
        };
        Options::build(file, overrides)
    }

    /// Merge defaults, an optional parsed file (with its name), and overrides.
    pub fn build(
        file: Option<(String, FileConfig)>,
        overrides: &Overrides,
    ) -> Result<Options, SkyError> {
        let mut options = Options::default();
        if let Some((name, config)) = file {
            options.apply_file(&name, config)?;
        }
        options.apply_overrides(overrides);
        Ok(options)
    }

    fn apply_file(&mut self, file: &str, config: FileConfig) -> Result<(), SkyError> {
        let side_range = MIN_CANVAS_SIDE..=MAX_CANVAS_SIDE;
        for (key, value) in [("width", config.width), ("height", config.height)] {
            if let Some(side) = value.filter(|side| !side_range.contains(side)) {
                return Err(config_error(
                    file,
                    format!(
                        "`{key} = {side}` is out of range ({MIN_CANVAS_SIDE} to {MAX_CANVAS_SIDE})."
                    ),
                ));
            }
        }
        if let Some(max) = config
            .max_stars
            .filter(|max| !(1..=MAX_STARS).contains(max))
        {
            return Err(config_error(
                file,
                format!("`max_stars = {max}` is out of range (1 to {MAX_STARS})."),
            ));
        }

        self.theme = config.theme.unwrap_or(self.theme);
        self.width = config.width.unwrap_or(self.width);
        self.height = config.height.unwrap_or(self.height);
        self.include_forks = config.include_forks.unwrap_or(self.include_forks);
        self.max_stars = config.max_stars.unwrap_or(self.max_stars);
        self.labels = config.labels.unwrap_or(self.labels);
        self.faces = config.faces.unwrap_or(self.faces);
        self.paper_grain = config.paper_grain.unwrap_or(self.paper_grain);
        self.exclude = config.exclude.unwrap_or_default();
        self.include = config.include.unwrap_or_default();
        if let Some(raw) = &config.names {
            self.names = language_table("names", raw, file, check_name)?;
        }
        if let Some(raw) = &config.colors {
            self.colors = language_table("colors", raw, file, check_color)?;
        }
        Ok(())
    }

    fn apply_overrides(&mut self, overrides: &Overrides) {
        self.theme = overrides.theme.unwrap_or(self.theme);
        self.labels = overrides.labels.unwrap_or(self.labels);
        self.include_forks = overrides.include_forks.unwrap_or(self.include_forks);
        self.faces = overrides.faces.unwrap_or(self.faces);
        self.seed = overrides.seed.or(self.seed);
        self.date = overrides.date.or(self.date);
        self.static_mode = overrides.static_mode || self.static_mode;
    }
}
