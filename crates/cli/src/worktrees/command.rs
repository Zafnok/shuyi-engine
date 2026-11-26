//! `shuyi trim-targets` and `shuyi seed-target`: [`trim`] and
//! [`seed`] on the worktrees of the game's git repository, the one its
//! project's root is in, with a game's roots ([`trim::GAME_ROOTS`]). Each
//! answers in text or, with `--json`, as `{ ok, result, code, error, help
//! }`, and each refusal carries a problem code.
//!
//! Like `repos`, they act on folders outside the game (other worktrees'
//! build output), so they are commands of the CLI only: never operations
//! of the contract, nor tools of the MCP server. They need no game, so
//! they run in `shuyi` and in the game's tool program alike.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde::Serialize;

use super::{seed, trim};
use crate::contract::codes::{self, Code};
use crate::project::{self, Project};

/// Why a command refused, or failed: its problem code and its message.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Refusal {
    code: Code,
    error: String,
}

impl Refusal {
    fn new(code: Code, error: impl Into<String>) -> Self {
        Self {
            code,
            error: error.into(),
        }
    }
}

/// The answer with `--json`: `result` when it ran, the error's `code`,
/// `error` and `help` when not.
#[derive(Debug, Serialize)]
struct Answer<T> {
    ok: bool,
    result: Option<T>,
    code: Option<String>,
    error: Option<String>,
    help: Option<String>,
}

/// What a command line asks.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Parsed<T> {
    /// Run it, answering in JSON when `json`.
    Run { what: T, json: bool },
    /// `--help`.
    Help,
}

/// `trim-targets`' usage line.
fn trim_usage(command: &str) -> String {
    format!(
        "usage: {command} trim-targets [--idle-days N] [--dry-run] [--stale-hashes] [--merged] \
         [--json]"
    )
}

/// `seed-target`'s usage line.
fn seed_usage(command: &str) -> String {
    format!("usage: {command} seed-target --into <worktree> [--json]")
}

/// `trim-targets --help`.
fn trim_help(command: &str) -> String {
    format!(
        "{}

Frees the disk the build output of this repository's idle worktrees takes,
and never removes, prunes or archives a worktree. In each worktree but
this one, idle (no build in {days} days) and with no build running, it
deletes the incremental cache of each profile folder of target/ (debug,
release, a triple's, the web build's): cargo still finds the build
fresh, and only the first edit there is slower.

  --idle-days N    idle after N days without a build ({days})
  --dry-run        say what would be deleted, with each target/'s size,
                   and delete nothing
  --stale-hashes   also delete the older hashes of a crate in deps/ that
                   no build read in N days
  --merged         also empty target/ of a worktree whose branch is
                   merged into origin/main, or whose pull request is (gh)
  --json           the answer as JSON",
        trim_usage(command),
        days = trim::DEFAULT_IDLE_DAYS
    )
}

/// `seed-target --help`.
fn seed_help(command: &str) -> String {
    format!(
        "{}

Run in the warm checkout (the first line of `git worktree list
--porcelain`), into a new worktree before its first build. It starts the
new worktree's target/debug/ from this one's: the third-party crates'
compiled output hard-linked, the rest copied, and the game's own crates
left to build fresh with the new worktree's paths. What the new target/
already has is kept. It refuses while a cargo build runs in either
worktree, and across volumes.

  --into <worktree>  the new worktree's folder, one `git worktree list`
                     lists
  --json             the answer as JSON",
        seed_usage(command)
    )
}

/// The options of `trim-targets [--idle-days N] [--dry-run]
/// [--stale-hashes] [--merged] [--json]`, with a game's roots.
fn parse_trim(args: &[String]) -> Result<Parsed<trim::Options>, String> {
    let mut options = trim::Options::new(trim::GAME_ROOTS.map(str::to_owned).to_vec());
    let mut json = false;
    let mut rest = args.iter();
    while let Some(arg) = rest.next() {
        match arg.as_str() {
            "--help" | "-h" => return Ok(Parsed::Help),
            "--idle-days" => {
                let days = rest.next().ok_or("--idle-days needs a number of days")?;
                options.idle_days = days
                    .parse()
                    .map_err(|_| format!("--idle-days {days}: not a number of days"))?;
            }
            "--dry-run" => options.dry_run = true,
            "--stale-hashes" => options.stale_hashes = true,
            "--merged" => options.merged = true,
            "--json" => json = true,
            other => return Err(format!("unknown argument {other}")),
        }
    }
    Ok(Parsed::Run {
        what: options,
        json,
    })
}

/// The worktree of `seed-target --into <worktree> [--json]`.
fn parse_seed(args: &[String]) -> Result<Parsed<PathBuf>, String> {
    let mut into = None;
    let mut json = false;
    let mut rest = args.iter();
    while let Some(arg) = rest.next() {
        match arg.as_str() {
            "--help" | "-h" => return Ok(Parsed::Help),
            "--into" => {
                let folder = rest
                    .next()
                    .ok_or("--into needs the new worktree's folder")?;
                if into.replace(PathBuf::from(folder)).is_some() {
                    return Err("--into names one worktree, not two".to_owned());
                }
            }
            "--json" => json = true,
            other => return Err(format!("unknown argument {other}")),
        }
    }
    let into = into.ok_or("--into <worktree> names the new worktree to seed")?;
    Ok(Parsed::Run { what: into, json })
}

/// The problem code of a seed's error.
fn seed_code(error: &seed::Error) -> Code {
    match error {
        seed::Error::NoWorktree(_) => codes::NOT_A_WORKTREE,
        seed::Error::SameWorktree(_) => codes::SAME_WORKTREE,
        seed::Error::NoDonorBuild(_) => codes::NO_DONOR_BUILD,
        seed::Error::Locked(_) => codes::BUILD_RUNNING,
        seed::Error::OtherVolume { .. } => codes::OTHER_VOLUME,
        _ => codes::FILE,
    }
}

/// The folder of the git repository's worktree `root` is in, when the
/// game at `root` is part of it: not when the repository ignores it (a
/// game made inside another repository's `target/`).
fn repository(root: &Path) -> Result<PathBuf, Refusal> {
    let git = |args: &[&str]| {
        Command::new("git")
            .current_dir(root)
            .args(args)
            .stderr(Stdio::null())
            .output()
            .map_err(|e| Refusal::new(codes::TOOLCHAIN, format!("git didn't start: {e}")))
    };
    let top = git(&["rev-parse", "--show-toplevel"])?;
    if !top.status.success() {
        return Err(Refusal::new(
            codes::NO_REPOSITORY,
            format!("{} is in no git repository", root.display()),
        ));
    }
    let top = PathBuf::from(String::from_utf8_lossy(&top.stdout).trim());
    if git(&["check-ignore", "-q", project::FILE])?
        .status
        .success()
    {
        return Err(Refusal::new(
            codes::NO_REPOSITORY,
            format!(
                "{} is inside the git repository at {}, which ignores it: the game has no \
                 repository of its own",
                root.display(),
                top.display()
            ),
        ));
    }
    Ok(top)
}

/// What `trim-targets` answers.
#[derive(Debug, Serialize)]
struct Trimmed {
    /// The worktree it ran in.
    repository: PathBuf,
    /// A line per worktree: what was deleted, or why it was left alone.
    worktrees: Vec<String>,
    /// The counts.
    #[serde(flatten)]
    report: trim::Report,
    /// The run in one line.
    summary: String,
}

/// What `seed-target` answers.
#[derive(Debug, Serialize)]
struct Seeded {
    /// The warm worktree, seeded from.
    donor: PathBuf,
    /// The new worktree, seeded.
    into: PathBuf,
    /// What was linked, copied, kept and left to rebuild.
    #[serde(flatten)]
    report: seed::Report,
}

/// Trims the idle worktrees of `root`'s repository.
fn trim_from(root: &Path, options: &trim::Options) -> Result<Trimmed, Refusal> {
    let repository = repository(root)?;
    let mut out = Vec::new();
    let report = trim::run(&repository, options, &mut out)
        .map_err(|e| Refusal::new(codes::NO_REPOSITORY, e))?;
    let summary = report.summary();
    let worktrees = String::from_utf8_lossy(&out)
        .lines()
        .filter(|line| *line != summary)
        .map(str::to_owned)
        .collect();
    Ok(Trimmed {
        repository,
        worktrees,
        report,
        summary,
    })
}

/// Seeds `into`, a worktree of `root`'s repository, from `root`'s.
fn seed_from(root: &Path, into: &Path) -> Result<Seeded, Refusal> {
    let donor = repository(root)?;
    let listed = trim::worktree_paths(&donor).map_err(|e| Refusal::new(codes::NO_REPOSITORY, e))?;
    let into_found = into.canonicalize().ok();
    let same =
        |worktree: &PathBuf| into_found.is_some() && worktree.canonicalize().ok() == into_found;
    if !listed.iter().any(same) {
        let names: Vec<String> = listed.iter().map(|w| w.display().to_string()).collect();
        return Err(Refusal::new(
            codes::NOT_A_WORKTREE,
            format!(
                "{} is no worktree of this repository (git worktree list lists {})",
                into.display(),
                names.join(", ")
            ),
        ));
    }
    let packages = seed::path_packages_of(&donor).map_err(|e| Refusal::new(codes::TOOLCHAIN, e))?;
    let report = seed::seed(&donor, into, &packages)
        .map_err(|e| Refusal::new(seed_code(&e), e.to_string()))?;
    let into = std::path::absolute(into).unwrap_or_else(|_| into.to_path_buf());
    Ok(Seeded {
        donor,
        into,
        report,
    })
}

/// Prints `outcome` of the command `name`, as JSON when `json`, else with
/// `text` for a result; its exit code.
fn answer<T: Serialize>(
    name: &str,
    json: bool,
    outcome: Result<T, Refusal>,
    text: impl Fn(&T) -> String,
) -> u8 {
    let exit = match &outcome {
        Ok(_) => 0,
        Err(refusal) => codes::exit_code(Some(refusal.code.0)),
    };
    if json {
        let answer = match outcome {
            Ok(result) => Answer {
                ok: true,
                result: Some(result),
                code: None,
                error: None,
                help: None,
            },
            Err(refusal) => Answer {
                ok: false,
                result: None,
                code: Some(refusal.code.0.to_owned()),
                error: Some(refusal.error),
                help: codes::help_of(refusal.code.0).map(str::to_owned),
            },
        };
        match serde_json::to_string_pretty(&answer) {
            Ok(text) => println!("{text}"),
            Err(e) => eprintln!("{name}: {e}"),
        }
    } else {
        match &outcome {
            Ok(result) => print!("{}", text(result)),
            Err(refusal) => eprint!(
                "{}",
                codes::said(
                    name,
                    refusal.code.0,
                    &refusal.error,
                    codes::help_of(refusal.code.0)
                )
            ),
        }
    }
    exit
}

/// A wrong command line of `name`: said, with its usage; exit code 2.
fn wrong(name: &str, args: &[String], error: &str, usage: &str) -> u8 {
    let json = args.iter().any(|arg| arg == "--json");
    let outcome: Result<(), Refusal> = Err(Refusal::new(
        codes::WRONG_ARGUMENTS,
        format!("{error}; {usage}"),
    ));
    answer(name, json, outcome, |()| String::new())
}

/// The command `name` (`trim-targets` or `seed-target`) from the command
/// line, on `project`'s repository, with no game: the exit code, or `None`
/// for another command.
pub(crate) fn main(project: &Project, name: &str, args: &[String]) -> Option<u8> {
    match name {
        "trim-targets" => Some(trim_main(project, args)),
        "seed-target" => Some(seed_main(project, args)),
        _ => None,
    }
}

/// `trim-targets` from the command line, on `project`'s repository: the
/// exit code.
fn trim_main(project: &Project, args: &[String]) -> u8 {
    let command = project.config.command();
    match parse_trim(args) {
        Ok(Parsed::Help) => {
            println!("{}", trim_help(command));
            0
        }
        Ok(Parsed::Run { what, json }) => answer(
            "trim-targets",
            json,
            trim_from(&project.root, &what),
            |done| {
                let mut text = done.worktrees.join("\n");
                text.push('\n');
                text.push_str(&done.summary);
                text.push('\n');
                text
            },
        ),
        Err(e) => wrong("trim-targets", args, &e, &trim_usage(command)),
    }
}

/// `seed-target` from the command line, from `project`'s worktree: the
/// exit code.
fn seed_main(project: &Project, args: &[String]) -> u8 {
    let command = project.config.command();
    match parse_seed(args) {
        Ok(Parsed::Help) => {
            println!("{}", seed_help(command));
            0
        }
        Ok(Parsed::Run { what, json }) => answer(
            "seed-target",
            json,
            seed_from(&project.root, &what),
            |done| {
                format!(
                    "seed-target: {} into {}: {}\n",
                    done.donor.display(),
                    done.into.display(),
                    done.report
                )
            },
        ),
        Err(e) => wrong("seed-target", args, &e, &seed_usage(command)),
    }
}

#[cfg(test)]
mod tests;
