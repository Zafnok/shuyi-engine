//! `cargo xtask`: this repository's own tools, never shipped (ticket 2002).
//!
//! - `cargo xtask ticket-lint [--pr-branch <branch>] [--game-repo <path>]`
//! - `cargo xtask check-links`
//! - `cargo xtask check-no-std`

mod links;
mod no_std;
mod tickets;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

const USAGE: &str = "\
usage:
  cargo xtask ticket-lint [--pr-branch <branch>] [--game-repo <path>]
      Check tickets/open and tickets/done (tickets/README.md). `game#NNNN` ids
      are checked against the game's checkout at <path>, or on GitHub with `gh`.
  cargo xtask check-links
      Check that every relative link in the Markdown files resolves.
  cargo xtask check-no-std
      Check that shuyi-core, shuyi-ui and shuyi-platform use no `std::` path
      outside `cfg(feature = \"std\")` (ADR-0008 §5).";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let root = repo_root();
    match args.first().map(String::as_str) {
        Some("ticket-lint") => ticket_lint(&root, &args[1..]),
        Some("check-links") if args.len() == 1 => report("check-links", &links::run(&root), &[]),
        Some("check-no-std") if args.len() == 1 => report("check-no-std", &no_std::run(&root), &[]),
        _ => {
            eprintln!("{USAGE}");
            ExitCode::FAILURE
        }
    }
}

/// The repository's root: `crates/xtask/../..`.
fn repo_root() -> PathBuf {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .and_then(Path::parent)
        .unwrap_or(manifest)
        .to_path_buf()
}

fn ticket_lint(root: &Path, args: &[String]) -> ExitCode {
    let mut pr_branch = None;
    let mut game_repo = None;
    let mut rest = args.iter();
    while let Some(arg) = rest.next() {
        match (arg.as_str(), rest.next()) {
            ("--pr-branch", Some(value)) => pr_branch = Some(value.as_str()),
            ("--game-repo", Some(value)) => game_repo = Some(PathBuf::from(value)),
            _ => {
                eprintln!("{USAGE}");
                return ExitCode::FAILURE;
            }
        }
    }
    let game = match &game_repo {
        Some(path) => tickets::GameSource::Checkout(path),
        None => tickets::GameSource::GitHub,
    };
    let found = tickets::run(root, pr_branch, &game);
    report("ticket-lint", &found.errors, &found.warnings)
}

fn report(check: &str, errors: &[String], warnings: &[String]) -> ExitCode {
    for warning in warnings {
        eprintln!("warning: {warning}");
    }
    for error in errors {
        eprintln!("error: {error}");
    }
    if errors.is_empty() {
        eprintln!("{check}: ok");
        ExitCode::SUCCESS
    } else {
        eprintln!("{check}: {} error(s)", errors.len());
        ExitCode::FAILURE
    }
}
