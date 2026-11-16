//! Config (8.6): defaults, a valid file, strict keys, friendly errors, CLI wins.

use git_constellation::config::{Overrides, load_config_file, parse_config};
use git_constellation::model::{LabelMode, LanguageKey, ThemeName};
use git_constellation::{Options, SkyError};

/// The sample from the spec, 8.6.
const SPEC_SAMPLE: &str = r##"
theme = "midnight"
width = 1200
height = 600
include_forks = false
max_stars = 60
labels = "major"
faces = true
paper_grain = false
exclude = ["old-experiment"]
include = []
[names]
Rust = "Ferris's Cove"
[colors]
Rust = "#ffb38a"
"##;

fn build(text: &str, overrides: &Overrides) -> Result<Options, SkyError> {
    let config = parse_config(text, "constellation.toml")?;
    Options::build(Some(("constellation.toml".to_string(), config)), overrides)
}

fn message(result: Result<Options, SkyError>) -> String {
    result
        .expect_err("this config should be rejected")
        .to_string()
}

#[test]
fn defaults_are_the_documented_ones() {
    let options = Options::build(None, &Overrides::default()).unwrap();
    assert_eq!(options, Options::default());
    assert_eq!(options.theme, ThemeName::Midnight);
    assert_eq!((options.width, options.height), (1200, 600));
    assert_eq!((options.max_stars, options.labels), (60, LabelMode::Major));
    assert!(
        options.faces && !options.include_forks && !options.paper_grain && !options.static_mode
    );
}

#[test]
fn the_spec_sample_parses() {
    let options = build(SPEC_SAMPLE, &Overrides::default()).unwrap();
    assert_eq!(options.exclude, ["old-experiment"]);
    assert_eq!(
        options.names.get(&LanguageKey::Rust).map(String::as_str),
        Some("Ferris's Cove")
    );
    assert_eq!(
        options.colors.get(&LanguageKey::Rust).map(String::as_str),
        Some("#ffb38a")
    );
}

#[test]
fn an_empty_file_changes_nothing() {
    assert_eq!(
        build("", &Overrides::default()).unwrap(),
        Options::default()
    );
}

#[test]
fn file_values_replace_defaults() {
    let text =
        "theme = \"dusk\"\nmax_stars = 10\nlabels = \"all\"\nfaces = false\ninclude_forks = true\n";
    let options = build(text, &Overrides::default()).unwrap();
    assert_eq!(options.theme, ThemeName::Dusk);
    assert_eq!(options.max_stars, 10);
    assert_eq!(options.labels, LabelMode::All);
    assert!(!options.faces && options.include_forks);
}

#[test]
fn language_keys_in_names_and_colors_are_case_insensitive() {
    let text = "[names]\njavascript = \"Lamp\"\n[colors]\nTYPESCRIPT = \"#FFE08A\"\n";
    let options = build(text, &Overrides::default()).unwrap();
    assert_eq!(
        options.names.get(&LanguageKey::JsTs).map(String::as_str),
        Some("Lamp")
    );
    assert!(options.colors.contains_key(&LanguageKey::JsTs));
}

#[test]
fn command_line_flags_beat_the_file() {
    let text = "theme = \"dusk\"\nlabels = \"all\"\nfaces = true\ninclude_forks = false\n";
    let overrides = Overrides {
        theme: Some(ThemeName::Paper),
        labels: Some(LabelMode::Off),
        faces: Some(false),
        include_forks: Some(true),
        seed: Some(99),
        static_mode: true,
        ..Overrides::default()
    };
    let options = build(text, &overrides).unwrap();
    assert_eq!(options.theme, ThemeName::Paper);
    assert_eq!(options.labels, LabelMode::Off);
    assert!(!options.faces && options.include_forks && options.static_mode);
    assert_eq!(options.seed, Some(99));
}

#[test]
fn flags_that_were_not_given_leave_the_file_alone() {
    let options = build(
        "theme = \"aurora\"\ninclude_forks = true\n",
        &Overrides::default(),
    )
    .unwrap();
    assert_eq!(options.theme, ThemeName::Aurora);
    assert!(options.include_forks);
}

#[test]
fn an_unknown_key_is_rejected_naming_the_file_line_and_key() {
    let text = "theme = \"dusk\"\nthemee = \"dusk\"\n";
    let error = message(build(text, &Overrides::default()));
    assert!(error.contains("constellation.toml"), "{error}");
    assert!(error.contains("line 2"), "{error}");
    assert!(error.contains("themee"), "{error}");
}

#[test]
fn a_bad_theme_value_names_the_file_and_the_choices() {
    let error = message(build("theme = \"neon\"\n", &Overrides::default()));
    assert!(
        error.contains("constellation.toml") && error.contains("line 1"),
        "{error}"
    );
    assert!(
        error.contains("neon") && error.contains("midnight"),
        "{error}"
    );
}

#[test]
fn a_wrong_type_is_rejected_with_a_line_number() {
    let error = message(build(
        "faces = true\nmax_stars = \"ten\"\n",
        &Overrides::default(),
    ));
    assert!(error.contains("line 2"), "{error}");
}

#[test]
fn broken_toml_is_rejected_not_panicked_on() {
    let error = message(build("theme = \n", &Overrides::default()));
    assert!(error.contains("constellation.toml"), "{error}");
}

#[test]
fn bad_colors_are_rejected_naming_the_table_and_key() {
    for bad in ["red", "#fff", "ffb38a", "#gggggg", "#ffb38a00"] {
        let text = format!("[colors]\nRust = \"{bad}\"\n");
        let error = message(build(&text, &Overrides::default()));
        assert!(error.contains("[colors] Rust"), "{bad}: {error}");
        assert!(error.contains("hex color"), "{bad}: {error}");
    }
}

#[test]
fn unknown_languages_in_names_are_rejected_with_the_known_list() {
    let error = message(build(
        "[names]\nFortran = \"The Fern\"\n",
        &Overrides::default(),
    ));
    assert!(
        error.contains("[names]") && error.contains("Fortran"),
        "{error}"
    );
    assert!(error.contains("rust"), "{error}");
}

#[test]
fn empty_names_are_rejected() {
    let error = message(build("[names]\nRust = \"  \"\n", &Overrides::default()));
    assert!(
        error.contains("[names] Rust") && error.contains("empty"),
        "{error}"
    );
}

#[test]
fn out_of_range_numbers_are_rejected() {
    for text in [
        "max_stars = 0",
        "max_stars = 61",
        "width = 100",
        "width = 5000",
        "height = 10",
    ] {
        let error = message(build(text, &Overrides::default()));
        assert!(error.contains("out of range"), "{text}: {error}");
        assert!(error.contains("constellation.toml"), "{text}: {error}");
    }
    assert!(
        build(
            "max_stars = 1\nwidth = 200\nheight = 4000",
            &Overrides::default()
        )
        .is_ok()
    );
    assert!(build("max_stars = 60", &Overrides::default()).is_ok());
}

#[test]
fn a_missing_file_is_a_friendly_error_naming_the_path() {
    let error = load_config_file(std::path::Path::new("definitely/not/here.toml"))
        .expect_err("file does not exist")
        .to_string();
    assert!(
        error.contains("definitely/not/here.toml") && error.contains("could not read"),
        "{error}"
    );
}

#[test]
fn resolve_reads_an_explicit_config_file_and_requires_it_to_exist() {
    let path = std::env::temp_dir().join(format!("constel-test-{}.toml", std::process::id()));
    std::fs::write(&path, "theme = \"sakura\"\n").unwrap();
    let overrides = Overrides {
        config: Some(path.clone()),
        ..Overrides::default()
    };
    let options = Options::resolve(&overrides).unwrap();
    assert_eq!(options.theme, ThemeName::Sakura);
    std::fs::remove_file(&path).unwrap();

    // The file is gone now, and an explicit --config must not silently fall back.
    assert!(Options::resolve(&overrides).is_err());
}

#[test]
fn file_config_round_trips_through_toml() {
    let config = parse_config(SPEC_SAMPLE, "constellation.toml").unwrap();
    let text = toml::to_string(&config).unwrap();
    assert_eq!(parse_config(&text, "again.toml").unwrap(), config);
}
