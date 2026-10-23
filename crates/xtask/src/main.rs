//! `cargo xtask`: this repository's own tools, never shipped (ticket 2002).
//!
//! - `cargo xtask ticket-lint [--pr-branch <branch>] [--game-repo <path>]`
//! - `cargo xtask check-links`
//! - `cargo xtask check-docs`
//! - `cargo xtask check-no-std`
//! - `cargo xtask check-layers`
//! - `cargo xtask check-toolchain`
//! - `cargo xtask api [--base <rev>] [--pr-branch <branch>] [--summary <file>]`

mod api;
mod docs;
mod layers;
mod links;
mod no_std;
mod tickets;
mod toolchain;

use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const USAGE: &str = "\
usage:
  cargo xtask ticket-lint [--pr-branch <branch>] [--game-repo <path>]
      Check tickets/open and tickets/done (tickets/README.md). `game#NNNN` ids
      are checked against the game's checkout at <path>, or on GitHub with `gh`.
      Warns, without failing, on a clause in an open ticket that waits on a
      done one (\"if NNNN is done\"): settle it (the work-ticket skill, §5).
  cargo xtask check-links
      Check that every relative link in the Markdown files resolves.
  cargo xtask check-docs
      Check that the docs map (docs/README.md) links every Markdown file.
  cargo xtask check-no-std
      Check that shuyi-core, shuyi-ui and shuyi-platform use no `std::` path
      outside `cfg(feature = \"std\")` (ADR-0008 §5).
  cargo xtask check-layers
      Check the crates' layers against tools/layers.ron (ADR-0032): which
      engine crate depends on which, the outside crates of the pure ones,
      no device crate below them and the float ban in the simulations.
  cargo xtask check-toolchain
      Check that the pinned Rust is the same everywhere (the bump-rust skill):
      the root's exact channel, the templates' channels, every workspace's
      rust-version, the members' inherited one and Dependabot's folders.
  cargo xtask api [--base <rev>] [--pr-branch <branch>] [--summary <file>]
      Show the public API change of the crates a game uses, against <rev> (by
      default `git merge-base HEAD main`): the listing diff (cargo-public-api)
      and the breaks (cargo-semver-checks), appending the Markdown to <file>.
      With --pr-branch, a break needs a `Breaking:` line in the branch's
      ticket (ADR-0037).";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let root = repo_root();
    match args.first().map(String::as_str) {
        Some("ticket-lint") => ticket_lint(&root, &args[1..]),
        Some("check-links") if args.len() == 1 => report("check-links", &links::run(&root), &[]),
        Some("check-docs") if args.len() == 1 => report("check-docs", &docs::run(&root), &[]),
        Some("check-no-std") if args.len() == 1 => report("check-no-std", &no_std::run(&root), &[]),
        Some("check-layers") if args.len() == 1 => report("check-layers", &layers::run(&root), &[]),
        Some("check-toolchain") if args.len() == 1 => {
            report("check-toolchain", &toolchain::run(&root), &[])
        }
        Some("api") => api(&root, &args[1..]),
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

fn api(root: &Path, args: &[String]) -> ExitCode {
    let Some(options) = api::Options::parse(args) else {
        eprintln!("{USAGE}");
        return ExitCode::FAILURE;
    };
    let outcome = api::run(root, &options, &api::System);
    let mut errors = outcome.errors;
    if !outcome.summary.is_empty() {
        // A summary that can't be printed is still in the file.
        let _ = writeln!(std::io::stdout().lock(), "{}", outcome.summary);
        if let Some(path) = &options.summary
            && let Err(e) = append(path, &outcome.summary)
        {
            errors.push(format!("{}: {e}", path.display()));
        }
    }
    report("api", &errors, &outcome.warnings)
}

/// Appends `text` to the file at `path`, creating it if needed.
fn append(path: &Path, text: &str) -> std::io::Result<()> {
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    writeln!(file, "{text}")
}

/// Prints what a check found on stderr, and on GitHub Actions each warning on
/// stdout as a workflow command too, so it shows on the PR.
fn report(check: &str, errors: &[String], warnings: &[String]) -> ExitCode {
    let on_github = std::env::var("GITHUB_ACTIONS").is_ok_and(|v| v == "true");
    let mut stdout = std::io::stdout().lock();
    for warning in warnings {
        eprintln!("warning: {warning}");
        if on_github {
            // A warning that can't be annotated is still on stderr.
            let _ = writeln!(stdout, "{}", annotation(warning));
        }
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

/// `warning` as a GitHub workflow command: `::warning::` and the text, with
/// `%`, CR and LF escaped as workflow commands require.
fn annotation(warning: &str) -> String {
    let text = warning
        .replace('%', "%25")
        .replace('\r', "%0D")
        .replace('\n', "%0A");
    format!("::warning::{text}")
}

#[cfg(test)]
mod tests {
    use super::annotation;

    #[test]
    fn an_annotation_is_a_warning_command() {
        assert_eq!(annotation("plain text"), "::warning::plain text");
    }

    #[test]
    fn an_annotation_escapes_percent_cr_and_lf() {
        assert_eq!(
            annotation("100% done\r\nnext%0A"),
            "::warning::100%25 done%0D%0Anext%250A"
        );
    }
}
