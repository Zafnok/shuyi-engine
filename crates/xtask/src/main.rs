//! `cargo xtask`: this repository's own tools, never shipped (ticket 2002).
//!
//! - `cargo xtask ticket-lint [--pr-branch <branch>] [--game-repo <path>]`
//! - `cargo xtask check-links`
//! - `cargo xtask check-docs`
//! - `cargo xtask check-no-std`
//! - `cargo xtask check-layers`
//! - `cargo xtask check-modules`
//! - `cargo xtask check-pins`
//! - `cargo xtask check-toolchain`
//! - `cargo xtask api [--base <rev>] [--pr-branch <branch>] [--summary <file>] [--verdict <file>]`
//! - `cargo xtask skill-examples [<skill>...] [--shard <i>/<n>]`
//! - `cargo xtask compile-times [--json] [--budget <file>] [--summary <file>] [--dir <dir>]`
//! - `cargo xtask changelog [--site <content dir>] [--format keep-a-changelog]`
//! - `cargo xtask llms --public <dir> --base-url <url> --commit <sha>`
//! - `cargo xtask llms --full-only --out <dir> --base-url <url> --commit <sha>`

mod api;
mod changelog;
mod compile_times;
mod docs;
mod layers;
mod links;
mod llms;
mod modules;
mod no_std;
mod pins;
mod script;
mod skills;
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
  cargo xtask check-modules
      Check the modules against docs/modules.md and docs/modules.ron (ADR-0009):
      no default crate depends on one, each has a README, its metadata and a
      row, answers the concerns list, alternatives name each other, and each
      feature of the shuyi crate names a row.
  cargo xtask check-pins
      Check that every workflow's actions are pinned by commit SHA, one SHA per
      action, every template's workflows in .github/dependabot.yml, and the
      templates' requirements equal to the root's (ADR-0037).
  cargo xtask check-toolchain
      Check that the pinned Rust is the same everywhere (the bump-rust skill):
      the root's exact channel, the templates' channels, every workspace's
      rust-version, the members' inherited one and Dependabot's folders.
  cargo xtask api [--base <rev>] [--pr-branch <branch>] [--summary <file>] [--verdict <file>]
      Show the public API change of the crates a game uses, against <rev> (by
      default `git merge-base HEAD main`): the listing diff (cargo-public-api)
      and the breaks (cargo-semver-checks), appending the Markdown to <file>.
      With --pr-branch, a break needs a `Breaking:` line in the branch's
      ticket (ADR-0038). With --verdict, write `none`, `added` or `breaking`
      to <file>, for the PR's `api:` label.
  cargo xtask skill-examples [<skill>...] [--shard <i>/<n>]
      Run the worked example of each skill of the skill pack (skills/, ADR-0043)
      on a fresh `shuyi new` game in target/skill-examples/, or of the skills
      named, or of every <n>th from the <i>th: a verdict per step. The games
      build in this repository's target folder unless CARGO_TARGET_DIR is set.
  cargo xtask compile-times [--json] [--budget <file>] [--summary <file>] [--dir <dir>]
      Time a new `grid` game's clean build, its rebuild after a one-line change
      in main.rs and the same for `cargo check`, and count a new `blank` game's
      crates, on games made in <dir> (target/compile-times). With --budget,
      fail over its numbers (tools/compile-budget.ron); with --summary, append
      the table's Markdown to <file>.
  cargo xtask changelog [--site <content dir>] [--format keep-a-changelog]
      Write the changelog from tickets/done/ and the first-parent history
      (ticket 2804): Markdown on stdout; with --site, Zola pages (changelog.md
      and a post for each release, in news/generated/) in <content dir>; with
      --format keep-a-changelog, the shape of CHANGELOG.md. Needs the tags and
      the whole history (a full clone).
  cargo xtask llms --public <dir> --base-url <url> --commit <sha>
      After `zola build` into <dir> at <url>, write the docs for agents there
      (ticket 2808): a Markdown twin of each Learn page (its URL + index.md),
      llms.txt, llms-full.txt and the contract's schemas, and check them: each
      twin's page built, no shortcode left, every llms.txt link resolving, and
      llms-full.txt under its budget.
  cargo xtask llms --full-only --out <dir> --base-url <url> --commit <sha>
      Write llms-full.txt alone into <dir>, for a release to attach.";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let root = repo_root();
    match args.first().map(String::as_str) {
        Some("ticket-lint") => ticket_lint(&root, &args[1..]),
        Some("check-links") if args.len() == 1 => report("check-links", &links::run(&root), &[]),
        Some("check-docs") if args.len() == 1 => report("check-docs", &docs::run(&root), &[]),
        Some("check-no-std") if args.len() == 1 => report("check-no-std", &no_std::run(&root), &[]),
        Some("check-layers") if args.len() == 1 => report("check-layers", &layers::run(&root), &[]),
        Some("check-modules") if args.len() == 1 => {
            report("check-modules", &modules::run(&root), &[])
        }
        Some("check-pins") if args.len() == 1 => report("check-pins", &pins::run(&root), &[]),
        Some("check-toolchain") if args.len() == 1 => {
            report("check-toolchain", &toolchain::run(&root), &[])
        }
        Some("api") => api(&root, &args[1..]),
        Some("skill-examples") => skill_examples(&root, &args[1..]),
        Some("compile-times") => compile_times(&root, &args[1..]),
        Some("changelog") => changelog(&root, &args[1..]),
        Some("llms") => llms(&root, &args[1..]),
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
    errors.extend(api::write_summary(
        &outcome.summary,
        options.summary.as_deref(),
        &mut std::io::stdout().lock(),
    ));
    errors.extend(api::write_verdict(
        outcome.label,
        options.verdict.as_deref(),
    ));
    report("api", &errors, &outcome.warnings)
}

fn skill_examples(root: &Path, args: &[String]) -> ExitCode {
    let options = match skills::Options::parse(args) {
        Ok(options) => options,
        Err(e) => {
            eprintln!(
                "skill-examples: {e}

{USAGE}"
            );
            return ExitCode::FAILURE;
        }
    };
    // Games share this repository's target folder unless cargo is told
    // otherwise, so the engine builds once for all of them.
    let runner = script::System {
        target_dir: std::env::var_os("CARGO_TARGET_DIR")
            .is_none()
            .then(|| root.join("target")),
    };
    let outcome = skills::run(root, &skills::games_dir(root), &options, &runner);
    eprint!("{}", outcome.report);
    report("skill-examples", &outcome.errors, &[])
}

fn compile_times(root: &Path, args: &[String]) -> ExitCode {
    let Some(options) = compile_times::Options::parse(args) else {
        eprintln!("{USAGE}");
        return ExitCode::FAILURE;
    };
    // The budget is read first: a wrong one fails before minutes of builds.
    let budget = match &options.budget {
        Some(file) => match std::fs::read_to_string(file)
            .map_err(|e| e.to_string())
            .and_then(|text| compile_times::parse_budget(&text))
        {
            Ok(budget) => Some((file.display().to_string(), budget)),
            Err(e) => return report("compile-times", &[format!("{}: {e}", file.display())], &[]),
        },
        None => None,
    };
    let games = compile_times::games_dir(root, options.dir.as_deref());
    let maker = script::System { target_dir: None };
    let game = compile_times::Game {
        target_dir: games.join("timed").join("target"),
    };
    let times = match compile_times::run(root, &games, &maker, &game) {
        Ok(times) => times,
        Err(e) => return report("compile-times", &[e], &[]),
    };
    let table = times.table();
    let shown = if options.json {
        times.json().to_string()
    } else {
        table.clone()
    };
    // An answer that can't be printed is still in the summary.
    let _ = writeln!(std::io::stdout().lock(), "{shown}");
    let mut verdict = compile_times::Verdict::default();
    if let Some(file) = &options.summary {
        let written = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(file)
            .and_then(|mut f| f.write_all(format!("{table}\n").as_bytes()));
        if let Err(e) = written {
            verdict.errors.push(format!("{}: {e}", file.display()));
        }
    }
    if let Some((file, budget)) = &budget {
        let found = compile_times::check(&times, budget, file);
        verdict.errors.extend(found.errors);
        verdict.warnings.extend(found.warnings);
    }
    report("compile-times", &verdict.errors, &verdict.warnings)
}

fn changelog(root: &Path, args: &[String]) -> ExitCode {
    let Some(options) = changelog::Options::parse(args) else {
        eprintln!("{USAGE}");
        return ExitCode::FAILURE;
    };
    let git = changelog::system_git(root);
    match changelog::run(root, &options, &git) {
        Ok(changelog::Output::Stdout(text)) => {
            let _ = write!(std::io::stdout().lock(), "{text}");
            ExitCode::SUCCESS
        }
        Ok(changelog::Output::Pages(pages)) => {
            let written = options
                .site
                .as_deref()
                .map_or(Ok(()), |dir| changelog::write_pages(dir, &pages));
            report(
                "changelog",
                &written.err().into_iter().collect::<Vec<_>>(),
                &[],
            )
        }
        Err(errors) => report("changelog", &errors, &[]),
    }
}

fn llms(root: &Path, args: &[String]) -> ExitCode {
    let Some(options) = llms::Options::parse(args) else {
        eprintln!("{USAGE}");
        return ExitCode::FAILURE;
    };
    let date = llms::git_date(root);
    let env = llms::Env {
        date: &date,
        budget: llms::FULL_BUDGET_BYTES,
    };
    let outcome = llms::run(root, &options, &env);
    let mut stdout = std::io::stdout().lock();
    for note in &outcome.notes {
        // A note that can't be printed leaves the check's verdict as it is.
        let _ = writeln!(stdout, "{note}");
    }
    report("llms", &outcome.errors, &[])
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
