//! Command-line definitions (clap). Parsing only; no behaviour lives here.

use std::path::PathBuf;

use chrono::NaiveDate;
use clap::{Args, Parser, Subcommand, ValueEnum};

use crate::config::Overrides;
use crate::export::Format;
use crate::model::{LabelMode, ThemeName};

/// Turn a GitHub account's public activity into a night-sky picture.
#[derive(Debug, Parser)]
// `version` makes `--version` print the version from Cargo.toml.
#[command(name = "constel", version, about)]
pub struct Cli {
    #[command(flatten)]
    pub global: GlobalArgs,
    #[command(subcommand)]
    pub command: Command,
}

/// Flags accepted by every subcommand, before or after its name.
#[derive(Debug, Default, Args)]
pub struct GlobalArgs {
    /// Config file to use (default: constellation.toml, if present)
    #[arg(long, global = true, value_name = "PATH")]
    pub config: Option<PathBuf>,
    /// Color theme: midnight, dusk, aurora, sakura, paper or mono
    #[arg(long, global = true, value_name = "THEME")]
    pub theme: Option<ThemeName>,
    /// Pretend today is this date
    #[arg(long, global = true, value_name = "YYYY-MM-DD", value_parser = parse_date)]
    pub date: Option<NaiveDate>,
    /// Seed for the layout (default: derived from the username)
    #[arg(long, global = true, value_name = "N")]
    pub seed: Option<u64>,
    /// Which labels to draw: none, major or all
    #[arg(long, global = true, value_name = "MODE")]
    pub labels: Option<LabelMode>,
    /// Draw forks too, as hollow "borrowed" stars
    #[arg(long, global = true)]
    pub include_forks: bool,
    /// night, day or auto: `day` picks the paper theme unless --theme says otherwise
    #[arg(long, global = true, value_name = "WHEN", value_enum)]
    pub time_of_day: Option<TimeOfDay>,
    /// Draw no faces on the moon and other characters
    #[arg(long, global = true)]
    pub no_faces: bool,
    /// Render with no animation at all
    // `static` is a Rust keyword, so the field is named differently.
    #[arg(long = "static", global = true)]
    pub static_mode: bool,
}

/// A hint for choosing a theme. Resolved here, in the CLI layer only.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum TimeOfDay {
    Auto,
    Night,
    Day,
}

impl GlobalArgs {
    /// Convert the flags into the overrides the config code understands. A
    /// flag that was not given stays `None` so it does not beat the config file.
    pub fn into_overrides(self) -> Overrides {
        Overrides {
            config: self.config,
            // `day` means the paper theme, but never overrides an explicit --theme.
            // Night skies (and auto) leave the theme alone.
            theme: self
                .theme
                .or((self.time_of_day == Some(TimeOfDay::Day)).then_some(ThemeName::Paper)),
            date: self.date,
            seed: self.seed,
            labels: self.labels,
            include_forks: self.include_forks.then_some(true),
            faces: self.no_faces.then_some(false),
            static_mode: self.static_mode,
        }
    }
}

fn parse_date(text: &str) -> Result<NaiveDate, String> {
    NaiveDate::parse_from_str(text, "%Y-%m-%d")
        .map_err(|_| format!("`{text}` is not a date. Use the form YYYY-MM-DD, like 2026-11-14."))
}

/// What `constel render` needs.
#[derive(Debug, Clone, PartialEq, Args)]
pub struct RenderArgs {
    /// The activity JSON file to draw
    #[arg(long, value_name = "ACTIVITY.json")]
    pub from_file: PathBuf,
    /// Where to write the picture (default: constellation.svg or .png)
    #[arg(long, value_name = "PATH")]
    pub out: Option<PathBuf>,
    /// svg or png
    #[arg(long, value_name = "FORMAT", default_value = "svg")]
    pub format: Format,
    /// Zoom for PNG output (2 doubles the size)
    #[arg(long, value_name = "N", default_value_t = 1.0)]
    pub scale: f32,
    /// Add film grain to the background
    #[arg(long)]
    pub paper_grain: bool,
    /// Draw only the sky (temporary: removed in Milestone 5)
    #[arg(long, hide = true)]
    pub background_only: bool,
}

/// Every subcommand `constel` will have once all milestones are done.
#[derive(Debug, PartialEq, Subcommand)]
pub enum Command {
    /// Render the sky as SVG or PNG
    Render(RenderArgs),
    /// Render a year of the sky forming as an animated GIF
    Timelapse,
    /// Render a 1200x630 social share card
    Card,
    /// Print a cozy terminal summary of the sky
    Stats,
    /// Explain why one repository's star looks the way it does
    Explain {
        /// Name of the repository to explain
        repo: String,
    },
    /// Serve the sky on localhost and reload it when files change
    Preview,
    /// Write a sample config and workflow into the current directory
    Init,
}

impl Command {
    /// The name the user types on the command line.
    pub fn name(&self) -> &'static str {
        match self {
            Command::Render(_) => "render",
            Command::Timelapse => "timelapse",
            Command::Card => "card",
            Command::Stats => "stats",
            Command::Explain { .. } => "explain",
            Command::Preview => "preview",
            Command::Init => "init",
        }
    }
}

/// The friendly message shown until a command is implemented.
pub fn not_built_yet(command: &Command) -> String {
    format!(
        "`constel {}` isn't built yet. It arrives in a later milestone of git-constellation.",
        command.name()
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;
    use clap::error::ErrorKind;

    fn parse(args: &[&str]) -> Result<Cli, clap::Error> {
        Cli::try_parse_from(std::iter::once("constel").chain(args.iter().copied()))
    }

    #[test]
    fn every_subcommand_parses() {
        let cases = [
            (
                vec!["render", "--from-file", "a.json"],
                Command::Render(RenderArgs {
                    from_file: PathBuf::from("a.json"),
                    out: None,
                    format: Format::Svg,
                    scale: 1.0,
                    paper_grain: false,
                    background_only: false,
                }),
            ),
            (vec!["timelapse"], Command::Timelapse),
            (vec!["card"], Command::Card),
            (vec!["stats"], Command::Stats),
            (
                vec!["explain", "my-repo"],
                Command::Explain {
                    repo: "my-repo".to_string(),
                },
            ),
            (vec!["preview"], Command::Preview),
            (vec!["init"], Command::Init),
        ];
        for (args, expected) in cases {
            let cli = parse(&args).expect("subcommand should parse");
            assert_eq!(cli.command, expected);
        }
    }

    #[test]
    fn global_flags_parse_before_and_after_the_subcommand() {
        let cli = parse(&[
            "--theme",
            "dusk",
            "render",
            "--from-file",
            "a.json",
            "--seed",
            "42",
            "--static",
        ])
        .unwrap();
        let overrides = cli.global.into_overrides();
        assert_eq!(overrides.theme, Some(ThemeName::Dusk));
        assert_eq!(overrides.seed, Some(42));
        assert!(overrides.static_mode);
    }

    #[test]
    fn all_global_flags_become_overrides() {
        let cli = parse(&[
            "stats",
            "--config",
            "my.toml",
            "--date",
            "2026-02-28",
            "--labels",
            "all",
            "--include-forks",
            "--no-faces",
        ])
        .unwrap();
        let overrides = cli.global.into_overrides();
        assert_eq!(overrides.config, Some(PathBuf::from("my.toml")));
        assert_eq!(overrides.date, NaiveDate::from_ymd_opt(2026, 2, 28));
        assert_eq!(overrides.labels, Some(LabelMode::All));
        assert_eq!(overrides.include_forks, Some(true));
        assert_eq!(overrides.faces, Some(false));
    }

    #[test]
    fn absent_flags_do_not_override_the_config_file() {
        let overrides = parse(&["render", "--from-file", "a.json"])
            .unwrap()
            .global
            .into_overrides();
        assert_eq!(overrides, Overrides::default());
    }

    #[test]
    fn time_of_day_day_picks_paper_unless_a_theme_is_given() {
        let day = |extra: &[&str]| {
            let mut args = vec!["render", "--from-file", "a.json", "--time-of-day", "day"];
            args.extend_from_slice(extra);
            parse(&args).unwrap().global.into_overrides().theme
        };
        assert_eq!(day(&[]), Some(ThemeName::Paper));
        assert_eq!(
            day(&["--theme", "dusk"]),
            Some(ThemeName::Dusk),
            "an explicit theme wins"
        );
    }

    #[test]
    fn time_of_day_night_and_auto_leave_the_theme_alone() {
        for when in ["night", "auto"] {
            let cli = parse(&["render", "--from-file", "a.json", "--time-of-day", when]).unwrap();
            assert_eq!(cli.global.into_overrides().theme, None, "{when}");
        }
        assert!(parse(&["render", "--from-file", "a.json", "--time-of-day", "noon"]).is_err());
    }

    #[test]
    fn bad_flag_values_give_friendly_errors() {
        let theme = parse(&["render", "--from-file", "a.json", "--theme", "neon"])
            .unwrap_err()
            .to_string();
        assert!(theme.contains("unknown theme `neon`") && theme.contains("midnight"));
        let date = parse(&["render", "--from-file", "a.json", "--date", "14/11/2026"])
            .unwrap_err()
            .to_string();
        assert!(date.contains("YYYY-MM-DD"));
        assert!(parse(&["render", "--from-file", "a.json", "--labels", "some"]).is_err());
    }

    #[test]
    fn explain_needs_a_repo_name() {
        assert!(parse(&["explain"]).is_err());
    }

    #[test]
    fn unknown_subcommand_is_rejected() {
        let err = parse(&["paint"]).expect_err("unknown command must fail");
        assert_eq!(err.kind(), ErrorKind::InvalidSubcommand);
    }

    #[test]
    fn version_flag_is_wired_up() {
        let err = parse(&["--version"]).expect_err("--version exits early");
        assert_eq!(err.kind(), ErrorKind::DisplayVersion);
    }

    #[test]
    fn clap_definition_is_internally_consistent() {
        Cli::command().debug_assert();
    }

    #[test]
    fn not_built_message_names_the_command() {
        let message = not_built_yet(&Command::Stats);
        assert!(message.contains("constel stats"));
        assert!(message.contains("isn't built yet"));
    }
}
