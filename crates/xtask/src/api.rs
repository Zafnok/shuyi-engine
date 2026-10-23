//! `cargo xtask api`: what a change does to the public API of the crates a
//! game uses (ticket 2010, ADR-0037).
//!
//! For each crate of [`API_CRATES`] it lists the public items of the head
//! (the working tree) and of the base (a `git worktree` at the base
//! revision) with `cargo-public-api`, diffs the two lists, and runs
//! `cargo-semver-checks` against the base. With `--pr-branch`, a semver
//! finding needs a `Breaking:` line in the branch's ticket.
//!
//! Every `git`, `cargo` and `rustup` call goes through [`Runner`], which the
//! tests fake.

use std::collections::HashSet;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::tickets::{breaking_lines, ticket_id_from_branch};

/// The crates a game made with `shuyi new` depends on: their public API is
/// what a game builds on. Not `xtask` (never shipped) and not the examples.
pub const API_CRATES: &[&str] = &[
    "shuyi-core",
    "shuyi-assets",
    "shuyi-ui",
    "shuyi-platform",
    "shuyi-app",
    "shuyi-cli",
];

/// The `cargo-semver-checks` this check runs, the one CI installs. It reads
/// the rustdoc JSON of the pinned toolchain (`rust-toolchain.toml`).
pub const SEMVER_CHECKS: &str = "0.51.0";

/// The `cargo-public-api` this check runs, the one CI installs.
pub const PUBLIC_API: &str = "0.52.0";

/// The nightly `cargo-public-api` builds rustdoc JSON with: the first 1.98
/// nightly (the workspace's `rust-version`), whose JSON is format 57, the
/// one `cargo-public-api` 0.52.0 reads.
pub const NIGHTLY: &str = "nightly-2026-05-23";

/// Where the base is checked out, under the repository's root.
const BASE_DIR: &str = "target/api-base";

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
}

/// The real commands.
pub struct System;

impl Runner for System {
    fn run(&self, dir: &Path, program: &str, args: &[&str]) -> Result<Ran, String> {
        let output = Command::new(program)
            .args(args)
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

/// What `cargo xtask api` found: the summary (empty when it couldn't
/// compare), the errors that fail it and the warnings that don't.
#[derive(Debug, Default)]
pub struct Outcome {
    pub summary: String,
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
    let errors = match &verdict {
        Some(Verdict::Fail(why)) => vec![why.clone()],
        _ => Vec::new(),
    };
    Outcome {
        summary: summary(&base, &crates, &warnings, verdict.as_ref()),
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
    // The base builds in the head's target folder, which has its
    // dependencies built already.
    let target_dir = root.join("target");
    let in_base = packages(&base_root, runner)?;
    API_CRATES
        .iter()
        .map(|&name| {
            let head = listing(root, name, &target_dir, runner)?;
            if !in_base.contains(name) {
                return Ok(CrateApi::new(name, None, &head, Vec::new()));
            }
            let base = listing(&base_root, name, &target_dir, runner)?;
            let findings = semver(root, name, &base_root, runner)?;
            Ok(CrateApi::new(name, Some(&base), &head, findings))
        })
        .collect()
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
    krate: &str,
    target_dir: &Path,
    runner: &dyn Runner,
) -> Result<Vec<String>, String> {
    let toolchain = format!("+{NIGHTLY}");
    let target_dir = target_dir.to_string_lossy();
    let ran = run_ok(
        runner,
        dir,
        "cargo",
        &[
            &toolchain,
            "public-api",
            "-p",
            krate,
            "--simplified",
            "--target-dir",
            &target_dir,
        ],
    )?;
    Ok(ran
        .stdout
        .lines()
        .map(str::trim_end)
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect())
}

/// `cargo-semver-checks`'s findings on `krate` against the base. Only the
/// lints that need a major version run (ADR-0037 §4).
fn semver(
    root: &Path,
    krate: &str,
    base_root: &Path,
    runner: &dyn Runner,
) -> Result<Vec<Finding>, String> {
    let base_root = base_root.to_string_lossy();
    let ran = runner.run(
        root,
        "cargo",
        &[
            "semver-checks",
            "check-release",
            "-p",
            krate,
            "--baseline-root",
            &base_root,
            "--release-type",
            "minor",
            "--color",
            "never",
        ],
    )?;
    parse_semver(krate, &ran)
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
             the public API (ADR-0037). Write a ticket for it (the `write-ticket` skill).\n\
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

/// The summary's Markdown: a table, a diff per changed crate, the warnings
/// and the rule's verdict.
pub fn summary(
    base: &str,
    crates: &[CrateApi],
    warnings: &[String],
    verdict: Option<&Verdict>,
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
