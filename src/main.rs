//! Thin CLI entry point. Anything testable lives in the library.

use std::process::ExitCode;

use anyhow::Result;
use clap::Parser;
use git_constellation::Options;
use git_constellation::cli::{Cli, not_built_yet};

// `anyhow::Result` is allowed in the binary only; the library uses typed errors.
fn run() -> Result<()> {
    let Cli { global, command } = Cli::parse();
    // Resolve settings first so a bad config file is reported in friendly words.
    // Later milestones pass these options to the commands.
    let _options = Options::resolve(&global.into_overrides())?;
    println!("{}", not_built_yet(&command));
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            // One tidy line instead of Rust's default debug dump of the error.
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}
