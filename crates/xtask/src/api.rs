//! `cargo xtask api`: what a change does to the public API of the engine's
//! crates (ticket 2010, ADR-0038).
//!
//! For each crate of [`API_CRATES`] it lists the public items of the head
//! (the working tree) and of the base (a `git worktree` at the base
//! revision) with `cargo-public-api`, diffs the two lists, and runs
//! `cargo-semver-checks` against the base. With `--pr-branch`, a semver
//! finding needs a `Breaking:` line in the branch's ticket. With
//! `--verdict`, it writes what the change did in one word (ticket 2030), for
//! CI's `api:` label on the PR. With `--pr-branch` too, a change to the
//! public API that changes no file of the book needs a `Book:` line in the
//! branch's ticket (ticket 2815).
//!
//! **What is checked** (ticket 2040) is one configuration per crate, on the
//! host, in a debug build:
//!
//! - `shuyi-alloc-count` with `global`, so its feature-gated module is
//!   listed too. A release build's `global` is not checked: the same items,
//!   which read nothing there.
//! - `shuyi-playdate` without `--cfg shuyi_playdate_sdk`: the SDK-free
//!   surface, `stack_high_water` and `main!` among it. Its doc-hidden
//!   `device` module is not checked, and of `main!` only that it exists: a
//!   change to its arms shows in neither tool.
//! - The other eight with each tool's own default: the listing has their
//!   default features, and the semver check the features its heuristic turns
//!   on, so a public item behind another feature is judged but not listed.
//!
//! No crate is checked in every combination of its features.
//!
//! Every `git`, `cargo` and `rustup` call goes through [`Runner`], which the
//! tests fake.

use std::collections::HashSet;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::layers::is_engine;
use crate::tickets::{book_lines, breaking_lines, ticket_id_from_branch};

/// The features a crate's public API is checked with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Features {
    /// Each tool's own default (the module's docs say what differs).
    ToolDefaults,
    /// Exactly these features, in the listing and the semver check alike, on
    /// the head and the base alike.
    Only(&'static [&'static str]),
}

impl Features {
    /// What `cargo-public-api` is given.
    fn listing_args(self) -> Vec<&'static str> {
        self.args("--no-default-features")
    }

    /// What `cargo-semver-checks` is given. Its `--features` holds for the
    /// baseline and the current version both.
    fn semver_args(self) -> Vec<&'static str> {
        self.args("--only-explicit-features")
    }

    fn args(self, only: &'static str) -> Vec<&'static str> {
        match self {
            Self::ToolDefaults => Vec::new(),
            Self::Only(features) => std::iter::once(only)
                .chain(features.iter().flat_map(|name| ["--features", *name]))
                .collect(),
        }
    }
}

/// One crate of the catalog, and how it is checked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ApiCrate {
    pub name: &'static str,
    pub features: Features,
}

const fn defaults(name: &'static str) -> ApiCrate {
    ApiCrate {
        name,
        features: Features::ToolDefaults,
    }
}

/// Every engine crate (a workspace member [`is_engine`] names), the one
/// catalog of what a game can build on: [`check_catalog`] fails a run on a
/// member left out. Not `xtask` (never shipped), the examples or the book.
/// Not read from `publish`: every crate is unpublished before the first
/// release.
pub const API_CRATES: &[ApiCrate] = &[
    defaults("shuyi-core"),
    defaults("shuyi-assets"),
    defaults("shuyi-ui"),
    defaults("shuyi-platform"),
    defaults("shuyi-app"),
    defaults("shuyi-cli"),
    defaults("shuyi"),
    defaults("shuyi-sound"),
    // The crate has no feature; none may join the check unchosen.
    ApiCrate {
        name: "shuyi-playdate",
        features: Features::Only(&[]),
    },
    // `global` turns `std` on, and adds the public `global` module.
    ApiCrate {
        name: "shuyi-alloc-count",
        features: Features::Only(&["global"]),
    },
];

/// The `cargo-semver-checks` this check runs, the one CI installs. It reads
/// the rustdoc JSON of the pinned toolchain (`rust-toolchain.toml`).
pub const SEMVER_CHECKS: &str = "0.51.0";

/// The `cargo-public-api` this check runs, the one CI installs.
pub const PUBLIC_API: &str = "0.52.0";

/// The nightly `cargo-public-api` builds rustdoc JSON with: the first 1.99
/// nightly (the workspace's `rust-version`). Its JSON is format 60, which
/// `cargo-public-api` 0.52.0 (built for format 57) still reads for the
/// engine's crates.
pub const NIGHTLY: &str = "nightly-2026-07-06";

/// Where the base is checked out, under the repository's root.
const BASE_DIR: &str = "target/api-base";
/// Where the base is built, beside its tree in the head's target folder
/// (which CI's cache keeps).
const BASE_TARGET_DIR: &str = "target/api-base-target";

/// The folders of the book: its words and its code. A change to the public
/// API that changes a file under one of them needs no `Book:` line.
pub const BOOK_DIRS: &[&str] = &["site/content/learn/", "site/snippets/"];

/// The most diff lines the summary shows for one crate.
const DIFF_LIMIT: usize = 200;

/// What a command did.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Ran {
    pub ok: bool,
    pub stdout: String,
    pub stderr: String,
}

/// Runs the commands this check needs. The tests fake it.
pub trait Runner {
    /// Runs `program` with `args` in `dir`. An error says it could not start.
    fn run(&self, dir: &Path, program: &str, args: &[&str]) -> Result<Ran, String>;

    /// Runs `program` with `args` in `dir`, with the variables of `env` set
    /// too (`cargo xtask golden`'s `SHUYI_BLESS`). A runner that can't set
    /// them says so.
    fn run_with_env(
        &self,
        dir: &Path,
        program: &str,
        args: &[&str],
        env: &[(&str, &str)],
    ) -> Result<Ran, String> {
        let _ = (dir, args, env);
        Err(format!("this runner can't set `{program}`'s environment"))
    }
}

/// The real commands.
pub struct System;

impl Runner for System {
    fn run(&self, dir: &Path, program: &str, args: &[&str]) -> Result<Ran, String> {
        self.run_with_env(dir, program, args, &[])
    }

    fn run_with_env(
        &self,
        dir: &Path,
        program: &str,
        args: &[&str],
        env: &[(&str, &str)],
    ) -> Result<Ran, String> {
        let output = Command::new(program)
            .args(args)
            .envs(env.iter().copied())
            .current_dir(dir)
            .output()
            .map_err(|e| format!("`{program}` did not run ({e})"))?;
        Ok(Ran {
            ok: output.status.success(),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        })
    }
}

/// `cargo xtask api`'s options.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Options {
    /// `--base <rev>`: the revision to compare with, or `git merge-base HEAD
    /// main`.
    pub base: Option<String>,
    /// `--pr-branch <branch>`: check the rule for this branch's ticket.
    pub pr_branch: Option<String>,
    /// `--summary <file>`: append the summary's Markdown to this file.
    pub summary: Option<PathBuf>,
    /// `--verdict <file>`: write the [`Label`]'s word to this file.
    pub verdict: Option<PathBuf>,
}

impl Options {
    /// The options in `args`, or `None` on anything else.
    pub fn parse(args: &[String]) -> Option<Self> {
        let mut options = Self::default();
        let mut rest = args.iter();
        while let Some(arg) = rest.next() {
            match (arg.as_str(), rest.next()) {
                ("--base", Some(value)) => options.base = Some(value.clone()),
                ("--pr-branch", Some(value)) => options.pr_branch = Some(value.clone()),
                ("--summary", Some(value)) => options.summary = Some(PathBuf::from(value)),
                ("--verdict", Some(value)) => options.verdict = Some(PathBuf::from(value)),
                _ => return None,
            }
        }
        Some(options)
    }
}

/// One lint `cargo-semver-checks` failed, and the items it failed in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    pub lint: String,
    pub items: Vec<String>,
}

/// What changed in one crate's public API.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct CrateApi {
    pub name: String,
    /// The base has no such crate: every item is added, and no semver check
    /// ran.
    pub new: bool,
    /// The base's listing lines the head doesn't have.
    pub removed: Vec<String>,
    /// The head's listing lines the base doesn't have.
    pub added: Vec<String>,
    pub findings: Vec<Finding>,
}

impl CrateApi {
    /// The change from the `base` listing (`None`: a crate new in the head)
    /// to the `head` listing. A changed line is one removed and one added.
    pub fn new(
        name: &str,
        base: Option<&[String]>,
        head: &[String],
        findings: Vec<Finding>,
    ) -> Self {
        let base_lines: HashSet<&String> = base.unwrap_or_default().iter().collect();
        let head_lines: HashSet<&String> = head.iter().collect();
        Self {
            name: name.to_string(),
            new: base.is_none(),
            removed: base
                .unwrap_or_default()
                .iter()
                .filter(|line| !head_lines.contains(line))
                .cloned()
                .collect(),
            added: head
                .iter()
                .filter(|line| !base_lines.contains(line))
                .cloned()
                .collect(),
            findings,
        }
    }

    /// The number of items the semver findings name.
    fn finding_items(&self) -> usize {
        self.findings.iter().map(|f| f.items.len()).sum()
    }
}

/// The rule's verdict: a reason to pass, or a reason to fail.
#[derive(Debug, PartialEq, Eq)]
pub enum Verdict {
    Pass(String),
    Fail(String),
}

/// What a change did to the public API, as the PR's `api:` label says it
/// to readers outside CI (ticket 2030, ADR-0038's *The verdict as a
/// label*).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Label {
    /// No crate's listing changed.
    None,
    /// A listing changed, and no semver finding.
    Added,
    /// A semver finding on some crate.
    Breaking,
}

impl Label {
    /// The word `--verdict` writes, and the label's after `api: `.
    pub fn word(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Added => "added",
            Self::Breaking => "breaking",
        }
    }
}

/// The label for `crates`, by ADR-0038's rule: a semver finding on any
/// crate breaks, a listing diff without one adds, and nothing is none.
pub fn verdict(crates: &[CrateApi]) -> Label {
    if crates.iter().any(|c| !c.findings.is_empty()) {
        Label::Breaking
    } else if crates
        .iter()
        .any(|c| !c.removed.is_empty() || !c.added.is_empty())
    {
        Label::Added
    } else {
        Label::None
    }
}

/// What `cargo xtask api` found: the summary (empty when it couldn't
/// compare), the label (none when it couldn't compare), the errors that
/// fail it and the warnings that don't.
#[derive(Debug, Default)]
pub struct Outcome {
    pub summary: String,
    pub label: Option<Label>,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

/// Compares the head in `root` with the base, and checks the rule when
/// `options` names a branch.
pub fn run(root: &Path, options: &Options, runner: &dyn Runner) -> Outcome {
    let missing = missing_tools(root, runner);
    if !missing.is_empty() {
        return Outcome {
            errors: missing,
            ..Outcome::default()
        };
    }
    let compared = base_revision(root, options, runner)
        .and_then(|base| compare(root, &base, runner).map(|crates| (base, crates)));
    let (base, crates) = match compared {
        Ok(compared) => compared,
        Err(error) => {
            return Outcome {
                errors: vec![error],
                ..Outcome::default()
            };
        }
    };
    let warnings = lost_lines(&crates);
    let verdict = options
        .pr_branch
        .as_deref()
        .map(|branch| rule(&crates, branch, &|id| done_ticket(root, id)));
    let book = options.pr_branch.as_deref().map(|branch| {
        changed_files(root, &base, runner).map_or_else(Verdict::Fail, |changed| {
            book_rule(&crates, branch, &changed, &|id| done_ticket(root, id))
        })
    });
    let errors = [&verdict, &book]
        .into_iter()
        .filter_map(|verdict| match verdict {
            Some(Verdict::Fail(why)) => Some(why.clone()),
            _ => None,
        })
        .collect();
    Outcome {
        summary: summary(&base, &crates, &warnings, verdict.as_ref(), book.as_ref()),
        label: Some(self::verdict(&crates)),
        errors,
        warnings,
    }
}

/// An error for each tool that is missing or at another version, with the
/// line that installs it.
pub fn missing_tools(root: &Path, runner: &dyn Runner) -> Vec<String> {
    let mut missing = Vec::new();
    for (subcommand, package, version) in [
        ("semver-checks", "cargo-semver-checks", SEMVER_CHECKS),
        ("public-api", "cargo-public-api", PUBLIC_API),
    ] {
        let install = format!("cargo install --locked {package}@{version}");
        match runner.run(root, "cargo", &[subcommand, "--version"]) {
            Ok(ran) if ran.ok && ran.stdout.trim() == format!("{package} {version}") => {}
            Ok(ran) if ran.ok => missing.push(format!(
                "this check needs {package} {version}, not `{}`; install it with:\n    {install}",
                ran.stdout.trim()
            )),
            _ => missing.push(format!(
                "{package} {version} is not installed; install it with:\n    {install}"
            )),
        }
    }
    let has_nightly = runner
        .run(root, "rustup", &["toolchain", "list"])
        .is_ok_and(|ran| ran.ok && ran.stdout.lines().any(is_the_nightly));
    if !has_nightly {
        missing.push(format!(
            "the {NIGHTLY} toolchain is not installed; install it with:\n    \
             rustup toolchain install {NIGHTLY} --profile minimal"
        ));
    }
    missing
}

/// Whether a line of `rustup toolchain list` is [`NIGHTLY`], for any host.
fn is_the_nightly(line: &str) -> bool {
    line.split_whitespace()
        .next()
        .is_some_and(|name| name == NIGHTLY || name.starts_with(&format!("{NIGHTLY}-")))
}

/// Runs a command that must succeed; the error names it and says why.
fn run_ok(runner: &dyn Runner, dir: &Path, program: &str, args: &[&str]) -> Result<Ran, String> {
    let ran = runner.run(dir, program, args)?;
    if ran.ok {
        Ok(ran)
    } else {
        Err(format!(
            "`{program} {}` failed:\n{}",
            args.join(" "),
            ran.stderr.trim_end()
        ))
    }
}

/// The commit of `--base`, or where the head left `main`.
fn base_revision(root: &Path, options: &Options, runner: &dyn Runner) -> Result<String, String> {
    let base = match &options.base {
        Some(base) => base.clone(),
        None => run_ok(runner, root, "git", &["merge-base", "HEAD", "main"])?
            .stdout
            .trim()
            .to_string(),
    };
    let commit = format!("{base}^{{commit}}");
    let ran = run_ok(runner, root, "git", &["rev-parse", "--verify", &commit])?;
    Ok(ran.stdout.trim().to_string())
}

/// Every crate's change from `base` to the head. The base's tree is removed
/// afterwards, also when the comparison fails.
fn compare(root: &Path, base: &str, runner: &dyn Runner) -> Result<Vec<CrateApi>, String> {
    let compared = add_worktree(root, base, runner).and_then(|()| compare_crates(root, runner));
    remove_worktree(root, runner);
    compared
}

/// Checks `base` out in [`BASE_DIR`], removing an old tree first.
fn add_worktree(root: &Path, base: &str, runner: &dyn Runner) -> Result<(), String> {
    remove_worktree(root, runner);
    let dir = root.join(BASE_DIR);
    if dir.exists() {
        // A folder git no longer knows as a worktree.
        fs::remove_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    run_ok(
        runner,
        root,
        "git",
        &["worktree", "add", "--detach", BASE_DIR, base],
    )
    .map(drop)
}

/// Removes the base's tree, if there is one.
fn remove_worktree(root: &Path, runner: &dyn Runner) {
    // With no tree to remove both fail, and nothing is left either way.
    let _ = runner.run(root, "git", &["worktree", "remove", "--force", BASE_DIR]);
    let _ = runner.run(root, "git", &["worktree", "prune"]);
}

/// Each of [`API_CRATES`]: its listing in the head and in the base, and its
/// semver findings.
fn compare_crates(root: &Path, runner: &dyn Runner) -> Result<Vec<CrateApi>, String> {
    let base_root = root.join(BASE_DIR);
    let target_dir = root.join("target");
    // The base builds in a target folder of its own. In the head's, its
    // engine crates would have the head's units' hashes (a workspace
    // crate's hash takes its path from the workspace's root, the same in
    // both), and sources older than the head's builds: the base would be
    // built on the head's crates.
    let base_target_dir = root.join(BASE_TARGET_DIR);
    check_catalog(&packages(root, runner)?, API_CRATES)?;
    let in_base = packages(&base_root, runner)?;
    API_CRATES
        .iter()
        .map(|krate| {
            let name = krate.name;
            let head = listing(root, krate, &target_dir, runner)?;
            if !in_base.contains(name) {
                return Ok(CrateApi::new(name, None, &head, Vec::new()));
            }
            let base = listing(&base_root, krate, &base_target_dir, runner)?;
            let findings = semver(root, krate, &base_root, runner)?;
            Ok(CrateApi::new(name, Some(&base), &head, findings))
        })
        .collect()
}

/// That `catalog` has each engine crate among the workspace's `members`, and
/// nothing else: a crate left out would change with no check at all.
pub fn check_catalog(members: &HashSet<String>, catalog: &[ApiCrate]) -> Result<(), String> {
    let mut unlisted: Vec<&str> = members
        .iter()
        .map(String::as_str)
        .filter(|name| is_engine(name) && !catalog.iter().any(|c| c.name == *name))
        .collect();
    unlisted.sort_unstable();
    let gone: Vec<&str> = catalog
        .iter()
        .map(|c| c.name)
        .filter(|name| !members.contains(*name))
        .collect();
    let mut problems = Vec::new();
    if !unlisted.is_empty() {
        problems.push(format!(
            "engine crate(s) not in `API_CRATES` (crates/xtask/src/api.rs): {}. Add each with \
             the features its public API is checked with",
            unlisted.join(", ")
        ));
    }
    if !gone.is_empty() {
        problems.push(format!(
            "`API_CRATES` (crates/xtask/src/api.rs) names what is no workspace member: {}",
            gone.join(", ")
        ));
    }
    if problems.is_empty() {
        Ok(())
    } else {
        Err(problems.join("\n"))
    }
}

/// The names of the workspace's packages in `dir`.
fn packages(dir: &Path, runner: &dyn Runner) -> Result<HashSet<String>, String> {
    let ran = run_ok(
        runner,
        dir,
        "cargo",
        &["metadata", "--no-deps", "--format-version", "1"],
    )?;
    let metadata: serde_json::Value = serde_json::from_str(&ran.stdout)
        .map_err(|e| format!("`cargo metadata` in {}: {e}", dir.display()))?;
    Ok(metadata["packages"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|package| package["name"].as_str())
        .map(str::to_string)
        .collect())
}

/// `krate`'s public items in the workspace in `dir`, one per line.
fn listing(
    dir: &Path,
    krate: &ApiCrate,
    target_dir: &Path,
    runner: &dyn Runner,
) -> Result<Vec<String>, String> {
    let toolchain = format!("+{NIGHTLY}");
    let target_dir = target_dir.to_string_lossy();
    let mut args = vec![
        toolchain.as_str(),
        "public-api",
        "-p",
        krate.name,
        "--simplified",
    ];
    args.extend(krate.features.listing_args());
    args.extend(["--target-dir", &target_dir]);
    let ran = run_ok(runner, dir, "cargo", &args)?;
    Ok(ran
        .stdout
        .lines()
        .map(str::trim_end)
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect())
}

/// `cargo-semver-checks`'s findings on `krate` against the base. Only the
/// lints that need a major version run (ADR-0038 §4).
fn semver(
    root: &Path,
    krate: &ApiCrate,
    base_root: &Path,
    runner: &dyn Runner,
) -> Result<Vec<Finding>, String> {
    let base_root = base_root.to_string_lossy();
    let mut args = vec![
        "semver-checks",
        "check-release",
        "-p",
        krate.name,
        "--baseline-root",
        &base_root,
        "--release-type",
        "minor",
        "--color",
        "never",
    ];
    args.extend(krate.features.semver_args());
    let ran = runner.run(root, "cargo", &args)?;
    parse_semver(krate.name, &ran)
}

/// The findings in what `cargo semver-checks` printed. A run that failed
/// without saying a major version is needed (a build error, a tool error)
/// is an error, never "no break".
pub fn parse_semver(krate: &str, ran: &Ran) -> Result<Vec<Finding>, String> {
    let findings = failures(&ran.stdout);
    let needs_major = ran.stderr.contains("semver requires new major version");
    match (ran.ok, findings.is_empty(), needs_major) {
        (true, true, false) | (false, false, true) => Ok(findings),
        _ => Err(format!(
            "`cargo semver-checks` failed on {krate} without naming a break:\n{}",
            ran.stderr.trim_end()
        )),
    }
}

/// Each `--- failure <lint>: … ---` block of `stdout`, with the items under
/// its `Failed in:`, each indented by two spaces. Warnings are not findings.
fn failures(stdout: &str) -> Vec<Finding> {
    let mut findings: Vec<Finding> = Vec::new();
    let mut in_failure = false;
    let mut in_items = false;
    for line in stdout.lines().map(str::trim_end) {
        if let Some(header) = line.strip_prefix("--- failure ") {
            let lint = header.split_once(':').map_or(header, |(lint, _)| lint);
            findings.push(Finding {
                lint: lint.to_string(),
                items: Vec::new(),
            });
            in_failure = true;
            in_items = false;
        } else if line.starts_with("--- ") {
            in_failure = false;
            in_items = false;
        } else if line == "Failed in:" {
            in_items = in_failure;
        } else if let (true, Some(item), Some(finding)) = (
            in_items,
            line.strip_prefix("  ")
                .filter(|item| !item.starts_with(' ')),
            findings.last_mut(),
        ) {
            finding.items.push(without_file(item).to_string());
        } else {
            in_items = false;
        }
    }
    findings
}

/// An item as a finding names it, without the file it was in.
fn without_file(item: &str) -> &str {
    [", previously in file ", " in file "]
        .iter()
        .find_map(|mark| item.split_once(mark).map(|(item, _)| item))
        .unwrap_or(item)
}

/// A warning for each crate whose listing lost lines with no semver finding:
/// a change `cargo-semver-checks` doesn't judge may still break a game.
pub fn lost_lines(crates: &[CrateApi]) -> Vec<String> {
    crates
        .iter()
        .filter(|c| c.findings.is_empty() && !c.removed.is_empty())
        .map(|c| {
            format!(
                "{}: {} removed line(s) cargo-semver-checks didn't call breaking: read them",
                c.name,
                c.removed.len()
            )
        })
        .collect()
}

/// The rule: a crate with a semver finding is a break, and a break needs a
/// `Breaking:` line naming the crate in the done ticket of `branch`.
/// `done_ticket` reads a ticket in `tickets/done/` by id.
pub fn rule(
    crates: &[CrateApi],
    branch: &str,
    done_ticket: &dyn Fn(&str) -> Option<String>,
) -> Verdict {
    let breaking: Vec<&CrateApi> = crates.iter().filter(|c| !c.findings.is_empty()).collect();
    if breaking.is_empty() {
        return Verdict::Pass("no semver finding: the public API doesn't break.".to_string());
    }
    let breaks = describe_breaks(&breaking);
    let Some(id) = ticket_id_from_branch(branch) else {
        return Verdict::Fail(format!(
            "branch `{branch}` names no ticket, and a change with no ticket never breaks \
             the public API (ADR-0038). Write a ticket for it (the `write-ticket` skill).\n\
             {breaks}"
        ));
    };
    let Some(body) = done_ticket(id) else {
        return Verdict::Fail(format!(
            "ticket {id} is not in tickets/done/, so it has no `Breaking:` line.\n{breaks}"
        ));
    };
    let named: HashSet<&str> = breaking_lines(&body)
        .into_iter()
        .flat_map(named_crates)
        .collect();
    let unnamed: Vec<&str> = breaking
        .iter()
        .map(|c| c.name.as_str())
        .filter(|name| !named.contains(name))
        .collect();
    if unnamed.is_empty() {
        let recorded: Vec<&str> = breaking.iter().map(|c| c.name.as_str()).collect();
        return Verdict::Pass(format!(
            "ticket {id} has a `Breaking:` line for {}.",
            recorded.join(", ")
        ));
    }
    let mut examples = String::new();
    for name in &unnamed {
        let _ = writeln!(
            examples,
            "    Breaking: {name}: <item>: <what a game changes>"
        );
    }
    Verdict::Fail(format!(
        "ticket {id}'s `## Completion notes` have no `Breaking:` line naming {}. \
         Add one for each break, as:\n\n{examples}\n{breaks}",
        unnamed.join(", ")
    ))
}

/// The files the head changed from `base`, the working tree's included.
fn changed_files(root: &Path, base: &str, runner: &dyn Runner) -> Result<Vec<String>, String> {
    let ran = run_ok(runner, root, "git", &["diff", "--name-only", base])?;
    Ok(ran
        .stdout
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect())
}

/// The book's rule (ticket 2815): a change to the public API (a listing
/// diff or a semver finding) that changes no file under [`BOOK_DIRS`] needs
/// a `Book:` line in the done ticket of `branch`, saying why the book needs
/// no change. A branch with no ticket of this repository is skipped: a
/// break there already fails [`rule`]. `changed` is the files the branch
/// changed; `done_ticket` reads a ticket in `tickets/done/` by id.
pub fn book_rule(
    crates: &[CrateApi],
    branch: &str,
    changed: &[String],
    done_ticket: &dyn Fn(&str) -> Option<String>,
) -> Verdict {
    if verdict(crates) == Label::None {
        return Verdict::Pass("the public API didn't change, so the book needn't.".to_string());
    }
    let in_book = changed
        .iter()
        .filter(|file| BOOK_DIRS.iter().any(|dir| file.starts_with(dir)))
        .count();
    if in_book > 0 {
        return Verdict::Pass(format!(
            "the book changed with the public API ({in_book} file(s) under {}).",
            BOOK_DIRS.join(" or ")
        ));
    }
    let Some(id) = ticket_id_from_branch(branch) else {
        return Verdict::Pass(format!(
            "branch `{branch}` names no ticket, so no `Book:` line is asked for."
        ));
    };
    let changed_crates: Vec<&str> = crates
        .iter()
        .filter(|c| !c.removed.is_empty() || !c.added.is_empty() || !c.findings.is_empty())
        .map(|c| c.name.as_str())
        .collect();
    let ways = format!(
        "The public API of {} changed, and no file under {} did. Either change the \
         chapter the docs map (docs/README.md) names for it, or say why the book \
         needs no change in a line of the ticket's `## Completion notes`, as:\n\n    \
         Book: none, <why>",
        changed_crates.join(", "),
        BOOK_DIRS.join(" or ")
    );
    let Some(body) = done_ticket(id) else {
        return Verdict::Fail(format!(
            "ticket {id} is not in tickets/done/, so it has no `Book:` line.\n{ways}"
        ));
    };
    match book_lines(&body).first() {
        Some(line) => Verdict::Pass(format!("ticket {id}'s notes say: {line}")),
        None => Verdict::Fail(format!(
            "ticket {id}'s `## Completion notes` have no `Book:` line.\n{ways}"
        )),
    }
}

/// The crates a `Breaking:` line names: the comma-separated list before its
/// next `:`.
fn named_crates(line: &str) -> Vec<&str> {
    let rest = line.strip_prefix("Breaking:").unwrap_or(line);
    let crates = rest.split_once(':').map_or(rest, |(crates, _)| crates);
    crates
        .split(',')
        .map(|name| name.trim().trim_matches('`'))
        .filter(|name| !name.is_empty())
        .collect()
}

/// The breaking crates, their lints and items, one line each.
fn describe_breaks(breaking: &[&CrateApi]) -> String {
    let mut text = String::from("The breaks:\n");
    for krate in breaking {
        for finding in &krate.findings {
            for item in &finding.items {
                let _ = writeln!(text, "  {}: {}: {item}", krate.name, finding.lint);
            }
        }
    }
    text
}

/// The summary's Markdown: a table, a diff per changed crate, the warnings,
/// the rule's verdict and the book's.
pub fn summary(
    base: &str,
    crates: &[CrateApi],
    warnings: &[String],
    verdict: Option<&Verdict>,
    book: Option<&Verdict>,
) -> String {
    let mut out = format!(
        "## Public API\n\nAgainst `{base}`.\n\n\
         | Crate | Items removed | Items added | Semver findings |\n\
         | ----- | ------------- | ----------- | --------------- |\n"
    );
    for krate in crates {
        let (name, findings) = if krate.new {
            (format!("`{}` (new)", krate.name), "not checked".to_string())
        } else {
            (
                format!("`{}`", krate.name),
                krate.finding_items().to_string(),
            )
        };
        let _ = writeln!(
            out,
            "| {name} | {} | {} | {findings} |",
            krate.removed.len(),
            krate.added.len()
        );
    }
    for warning in warnings {
        let _ = write!(out, "\n> **Warning:** {warning}.\n");
    }
    for krate in crates
        .iter()
        .filter(|c| !c.removed.is_empty() || !c.added.is_empty() || !c.findings.is_empty())
    {
        details(&mut out, krate);
    }
    out.push('\n');
    match verdict {
        Some(Verdict::Pass(why)) => {
            let _ = writeln!(out, "**Pass:** {why}");
        }
        Some(Verdict::Fail(why)) => {
            let _ = write!(out, "**Fail:**\n\n```text\n{}\n```\n", why.trim_end());
        }
        None => out.push_str("No `--pr-branch`: the rule was not checked.\n"),
    }
    match book {
        Some(Verdict::Pass(why)) => {
            let _ = writeln!(out, "\n**Book:** {why}");
        }
        Some(Verdict::Fail(why)) => {
            let _ = write!(
                out,
                "\n**Book, fail:**\n\n```text\n{}\n```\n",
                why.trim_end()
            );
        }
        None => {}
    }
    out
}

/// One crate's `<details>` block: its findings and its diff, cut at
/// [`DIFF_LIMIT`] lines.
fn details(out: &mut String, krate: &CrateApi) {
    let _ = write!(
        out,
        "\n<details><summary><code>{}</code>: {} removed, {} added</summary>\n\n",
        krate.name,
        krate.removed.len(),
        krate.added.len()
    );
    for finding in &krate.findings {
        let _ = writeln!(out, "- `{}`: {}", finding.lint, finding.items.join("; "));
    }
    if !krate.findings.is_empty() {
        out.push('\n');
    }
    let lines: Vec<String> = krate
        .removed
        .iter()
        .map(|line| format!("-{line}"))
        .chain(krate.added.iter().map(|line| format!("+{line}")))
        .collect();
    out.push_str("```diff\n");
    for line in lines.iter().take(DIFF_LIMIT) {
        let _ = writeln!(out, "{line}");
    }
    out.push_str("```\n");
    if lines.len() > DIFF_LIMIT {
        let _ = writeln!(out, "\nAnd {} more.", lines.len() - DIFF_LIMIT);
    }
    out.push_str("\n</details>\n");
}

/// Prints `summary` to `out` and appends it to `file` (CI's
/// `$GITHUB_STEP_SUMMARY`), creating the file if needed. An empty summary
/// (the comparison failed) writes nothing. The error says the file couldn't
/// be written.
pub fn write_summary(
    summary: &str,
    file: Option<&Path>,
    out: &mut dyn std::io::Write,
) -> Option<String> {
    if summary.is_empty() {
        return None;
    }
    // A summary that can't be printed is still in the file.
    let _ = writeln!(out, "{summary}");
    let path = file?;
    fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .and_then(|mut file| {
            std::io::Write::write_all(&mut file, format!("{summary}\n").as_bytes())
        })
        .err()
        .map(|e| format!("{}: {e}", path.display()))
}

/// Writes `label`'s word to `file` (CI's `api:` label reads it). No label
/// (the comparison failed) writes nothing. The error says the file couldn't
/// be written.
pub fn write_verdict(label: Option<Label>, file: Option<&Path>) -> Option<String> {
    let (label, path) = (label?, file?);
    fs::write(path, label.word())
        .err()
        .map(|e| format!("{}: {e}", path.display()))
}

/// The ticket `id` in `tickets/done/`, if it is there.
fn done_ticket(root: &Path, id: &str) -> Option<String> {
    let prefix = format!("{id}-");
    fs::read_dir(root.join("tickets/done"))
        .ok()?
        .filter_map(Result::ok)
        .find(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            name.starts_with(&prefix) && entry.path().extension().is_some_and(|ext| ext == "md")
        })
        .and_then(|entry| fs::read_to_string(entry.path()).ok())
}

#[cfg(test)]
mod tests;
