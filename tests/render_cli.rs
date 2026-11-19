//! `constel render`: the real binary, its files, flags and error messages.

use std::path::PathBuf;
use std::process::{Command, Output};

const BIN: &str = env!("CARGO_BIN_EXE_constel");

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn fixture(name: &str) -> String {
    root()
        .join("tests")
        .join("fixtures")
        .join(format!("{name}.json"))
        .display()
        .to_string()
}

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("constel-cli-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp folder");
    dir.join(name)
}

fn run(args: &[&str]) -> Output {
    Command::new(BIN)
        .args(args)
        .current_dir(root())
        .output()
        .expect("the binary runs")
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).to_string()
}

/// Render and return the file's bytes, asserting success.
fn render(extra: &[&str], out: &str) -> Vec<u8> {
    let path = scratch(out);
    let mut args = vec![
        "render",
        "--from-file",
        "FIXTURE",
        "--out",
        path.to_str().unwrap(),
    ];
    args.extend_from_slice(extra);
    let fixture_path = fixture("small");
    let args: Vec<&str> = args
        .iter()
        .map(|a| {
            if *a == "FIXTURE" {
                fixture_path.as_str()
            } else {
                *a
            }
        })
        .collect();
    let result = run(&args);
    assert!(result.status.success(), "{}", text(&result.stderr));
    assert!(
        text(&result.stdout).starts_with("wrote "),
        "{}",
        text(&result.stdout)
    );
    std::fs::read(&path).expect("the output file exists")
}

fn png_size(png: &[u8]) -> (u32, u32) {
    assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
    let read = |at: usize| u32::from_be_bytes([png[at], png[at + 1], png[at + 2], png[at + 3]]);
    (read(16), read(20))
}

#[test]
fn svg_output_is_a_valid_sky() {
    let svg = text(&render(&["--format", "svg"], "a.svg"));
    assert!(svg.starts_with("<svg") && svg.contains("viewBox=\"0 0 1200 600\""));
    usvg::Tree::from_str(&svg, &usvg::Options::default()).expect("valid SVG");
    assert!(svg.contains("<g id=\"milkyway\">") && svg.contains("<g id=\"dust\">"));
}

#[test]
fn png_output_has_the_right_size_at_scale_one_and_two() {
    assert_eq!(
        png_size(&render(&["--format", "png"], "a.png")),
        (1200, 600)
    );
    assert_eq!(
        png_size(&render(&["--format", "png", "--scale", "2"], "b.png")),
        (2400, 1200)
    );
    assert_eq!(
        png_size(&render(&["--format", "png", "--scale", "0.5"], "c.png")),
        (600, 300)
    );
}

#[test]
fn svg_ignores_the_scale() {
    assert_eq!(
        render(&["--format", "svg", "--scale", "3"], "s1.svg"),
        render(&["--format", "svg"], "s2.svg")
    );
}

#[test]
fn the_default_output_is_deterministic_and_the_seed_changes_it() {
    assert_eq!(render(&[], "d1.svg"), render(&[], "d2.svg"));
    assert_eq!(
        render(&["--seed", "7"], "e1.svg"),
        render(&["--seed", "7"], "e2.svg")
    );
    assert_ne!(
        render(&["--seed", "7"], "f1.svg"),
        render(&["--seed", "8"], "f2.svg")
    );
}

#[test]
fn every_theme_is_reachable_from_the_command_line() {
    let expected = [
        ("midnight", "#0b1026"),
        ("dusk", "#1b1233"),
        ("aurora", "#07141f"),
        ("sakura", "#1a1030"),
        ("paper", "#f6efe2"),
        ("mono", "#0a0a0a"),
    ];
    for (theme, color) in expected {
        let svg = text(&render(&["--theme", theme], &format!("{theme}.svg")));
        assert!(svg.contains(color), "{theme} should use {color}");
    }
}

#[test]
fn time_of_day_chooses_paper_for_day_only_when_no_theme_is_given() {
    let svg = |extra: &[&str], name: &str| text(&render(extra, name));
    assert!(
        svg(&["--time-of-day", "day"], "t1.svg").contains("#f6efe2"),
        "day means paper"
    );
    assert!(
        svg(&["--time-of-day", "day", "--theme", "dusk"], "t2.svg").contains("#1b1233"),
        "an explicit theme wins"
    );
    assert!(
        svg(&["--time-of-day", "night"], "t3.svg").contains("#0b1026"),
        "night is the default midnight sky"
    );
    assert!(svg(&["--time-of-day", "auto"], "t4.svg").contains("#0b1026"));
}

#[test]
fn grain_is_added_only_when_asked() {
    assert!(!text(&render(&[], "g1.svg")).contains("feTurbulence"));
    assert!(text(&render(&["--paper-grain"], "g2.svg")).contains("feTurbulence"));
}

#[test]
fn a_config_file_sets_the_theme_and_a_flag_overrides_it() {
    let config = scratch("theme.toml");
    std::fs::write(&config, "theme = \"aurora\"\n").unwrap();
    let path = config.to_str().unwrap();
    assert!(
        text(&render(&["--config", path], "c1.svg")).contains("#07141f"),
        "from the file"
    );
    assert!(
        text(&render(&["--config", path, "--theme", "sakura"], "c2.svg")).contains("#1a1030"),
        "the flag wins"
    );
}

#[test]
fn render_has_no_background_only_flag_any_more() {
    let help = text(&run(&["render", "--help"]).stdout);
    assert!(help.contains("--from-file") && help.contains("--format") && help.contains("--scale"));
    assert!(help.contains("--paper-grain"));
    assert!(
        !help.contains("background-only"),
        "the temporary flag is gone"
    );
    let result = run(&[
        "render",
        "--from-file",
        &fixture("small"),
        "--background-only",
    ]);
    assert_eq!(result.status.code(), Some(2), "clap's own usage error");
    assert!(text(&result.stderr).contains("--background-only"));
}

fn one_line_error(result: &Output) -> String {
    assert!(!result.status.success(), "expected a failure");
    let err = text(&result.stderr);
    assert!(err.starts_with("error: "), "{err}");
    assert_eq!(err.trim_end().lines().count(), 1, "one line only: {err}");
    err
}

#[test]
fn a_missing_input_file_is_a_friendly_one_line_error() {
    let result = run(&["render", "--from-file", "no/such/file.json"]);
    assert_eq!(result.status.code(), Some(1));
    let err = one_line_error(&result);
    assert!(
        err.contains("no/such/file.json") && err.contains("could not read"),
        "{err}"
    );
}

#[test]
fn a_broken_input_file_is_a_friendly_error() {
    let bad = scratch("bad.json");
    std::fs::write(&bad, "{ \"username\": 5 }").unwrap();
    let result = run(&["render", "--from-file", bad.to_str().unwrap()]);
    assert!(one_line_error(&result).contains("activity JSON"));
}

#[test]
fn render_draws_the_full_sky_without_any_extra_flag() {
    let out = scratch("full-sky.svg");
    let result = run(&[
        "render",
        "--from-file",
        &fixture("small"),
        "--out",
        out.to_str().unwrap(),
    ]);
    assert!(result.status.success(), "{}", text(&result.stderr));
    let svg = std::fs::read_to_string(&out).expect("written");
    for layer in ["stars", "lines", "labels", "hud"] {
        assert!(svg.contains(&format!("<g id=\"{layer}\"")), "{layer}");
    }
}

#[test]
fn bad_scales_and_formats_are_refused() {
    let small = fixture("small");
    for scale in ["0", "-2", "nan", "inf"] {
        let out = scratch("never.png");
        let result = run(&[
            "render",
            "--from-file",
            &small,
            "--format",
            "png",
            "--scale",
            scale,
            "--out",
            out.to_str().unwrap(),
        ]);
        assert!(!result.status.success(), "scale {scale}");
        assert!(
            text(&result.stderr).contains("scale") || text(&result.stderr).contains("error"),
            "{scale}"
        );
    }
    let huge = run(&[
        "render",
        "--from-file",
        &small,
        "--format",
        "png",
        "--scale",
        "100",
        "--out",
        scratch("huge.png").to_str().unwrap(),
    ]);
    assert!(one_line_error(&huge).contains("8000"));
    let gif = run(&["render", "--from-file", &small, "--format", "gif"]);
    assert_eq!(gif.status.code(), Some(2), "clap's own usage error");
    assert!(text(&gif.stderr).contains("unknown format"));
}

#[test]
fn a_missing_required_flag_is_a_usage_error() {
    let result = run(&["render"]);
    assert_eq!(result.status.code(), Some(2));
    assert!(text(&result.stderr).contains("--from-file"));
}

#[test]
fn an_unwritable_output_path_is_a_friendly_error() {
    let result = run(&[
        "render",
        "--from-file",
        &fixture("small"),
        "--out",
        "/no/such/folder/x.svg",
    ]);
    assert!(one_line_error(&result).contains("could not write"));
}

#[test]
fn other_commands_still_say_they_are_not_built() {
    for command in ["stats", "card", "timelapse", "preview", "init"] {
        let result = run(&[command]);
        assert!(result.status.success(), "{command}");
        assert!(
            text(&result.stdout).contains("isn't built yet"),
            "{command}"
        );
    }
}
