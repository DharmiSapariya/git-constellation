//! Integration smoke test: the public library API can parse and describe a command.

use clap::Parser;
use git_constellation::cli::{Cli, not_built_yet};

#[test]
fn render_prints_a_friendly_placeholder() {
    let cli = Cli::try_parse_from(["constel", "render", "--from-file", "a.json"])
        .expect("render should parse");
    let message = not_built_yet(&cli.command);
    assert!(message.contains("constel render"));
}
