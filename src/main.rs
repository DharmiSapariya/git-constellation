//! Thin CLI entry point. Anything testable lives in the library.

use std::process::ExitCode;

use anyhow::Result;
use clap::Parser;
use git_constellation::Options;
use git_constellation::cli::{Cli, Command, not_built_yet};
use git_constellation::export::{RenderRequest, render_file};

// `anyhow::Result` is allowed in the binary only; the library uses typed errors.
fn run() -> Result<()> {
    let Cli { global, command } = Cli::parse();
    // Resolve settings first so a bad config file is reported in friendly words.
    let options = Options::resolve(&global.into_overrides())?;
    match command {
        Command::Render(args) => {
            let request = RenderRequest {
                from_file: &args.from_file,
                out: args.out.as_deref(),
                format: args.format,
                scale: args.scale,
                grain: args.paper_grain,
                background_only: args.background_only,
                options: &options,
            };
            let written = render_file(&request)?;
            println!("wrote {}", written.display());
        }
        other => println!("{}", not_built_yet(&other)),
    }
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
