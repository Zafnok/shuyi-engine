//! `cargo xtask pr-size`: a PR's changed lines, by kind, against the size a
//! session finishes well (ticket 2053, ported from the game's ADR-0101 tool).
//!
//! It counts code, tests, docs and hand-written data, the lines added plus
//! removed, and leaves out what no one writes by hand: snapshots, golden
//! tables, the contract's schemas, lock files and generated files, which it
//! totals apart, as it does the tickets. Past [`WARN`] it warns. Past
//! [`LIMIT`], a PR passes only when its done ticket's completion notes have a
//! `Size:` line with the measured number and why it wasn't split.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::tickets::{self, Pr, PrKind};

/// Past this many counted lines, a PR warns.
pub const WARN: usize = 1_000;
/// Past this many counted lines, a PR needs a `Size:` line.
pub const LIMIT: usize = 1_500;

/// What the command was asked.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Options {
    pub base: Option<String>,
    pub hook: bool,
    pub ticket: Option<PathBuf>,
    pub pr_branch: Option<String>,
    pub pr_title: Option<String>,
}

impl Options {
    pub fn parse(args: &[String]) -> Option<Self> {
        let mut options = Options::default();
        let mut rest = args.iter();
        while let Some(arg) = rest.next() {
            match arg.as_str() {
                "--hook" => options.hook = true,
                "--base" => options.base = Some(rest.next()?.clone()),
                "--ticket" => options.ticket = Some(PathBuf::from(rest.next()?)),
                "--pr-branch" => options.pr_branch = Some(rest.next()?.clone()),
                "--pr-title" => options.pr_title = Some(rest.next()?.clone()),
                _ => return None,
            }
        }
        Some(options)
    }
}

/// Why a file's lines are left out of the count.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LeftOut {
    Snapshot,
    Golden,
    Schema,
    Lock,
    Generated,
}

impl LeftOut {
    const ALL: [LeftOut; 5] = [
        LeftOut::Snapshot,
        LeftOut::Golden,
        LeftOut::Schema,
        LeftOut::Lock,
        LeftOut::Generated,
    ];

    fn name(self) -> &'static str {
        match self {
            LeftOut::Snapshot => "snapshots",
            LeftOut::Golden => "golden tables",
            LeftOut::Schema => "schemas",
            LeftOut::Lock => "lock files",
            LeftOut::Generated => "generated",
        }
    }
}

/// What a changed file is, for the count.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Kind {
    Code,
    Test,
    Docs,
    Data,
    /// `tickets/`: counted apart.
    Tickets,
    LeftOut(LeftOut),
}

impl Kind {
    /// The kinds the PR's size is.
    const COUNTED: [Kind; 4] = [Kind::Code, Kind::Test, Kind::Docs, Kind::Data];

    fn name(self) -> &'static str {
        match self {
            Kind::Code => "code",
            Kind::Test => "tests",
            Kind::Docs => "docs",
            Kind::Data => "data",
            Kind::Tickets => "tickets",
            Kind::LeftOut(why) => why.name(),
        }
    }

    fn counted(self) -> bool {
        Kind::COUNTED.contains(&self)
    }
}

/// Which kind `path` (from the repository's root, `/`-separated) is.
pub fn classify(path: &str) -> Kind {
    let segments: Vec<&str> = path.split('/').collect();
    let name = segments.last().copied().unwrap_or(path);
    let dirs = &segments[..segments.len().saturating_sub(1)];
    let ext = Path::new(name).extension().and_then(|e| e.to_str());
    if ext == Some("snap") || dirs.contains(&"snapshots") {
        Kind::LeftOut(LeftOut::Snapshot)
    } else if dirs.contains(&"golden") && ext != Some("rs") {
        Kind::LeftOut(LeftOut::Golden)
    } else if name.ends_with(".schema.json") || path.starts_with("crates/cli/schemas/") {
        Kind::LeftOut(LeftOut::Schema)
    } else if name == "Cargo.lock" {
        Kind::LeftOut(LeftOut::Lock)
    } else if is_generated(path, name) {
        Kind::LeftOut(LeftOut::Generated)
    } else if path.starts_with("tickets/") {
        Kind::Tickets
    } else if ext == Some("md") || path.starts_with("docs/") || path.starts_with("site/") {
        Kind::Docs
    } else if dirs
        .iter()
        .any(|d| matches!(*d, "tests" | "benches" | "fuzz"))
        || name == "tests.rs"
        || name.ends_with("_tests.rs")
    {
        Kind::Test
    } else if matches!(
        ext,
        Some("ron" | "json" | "map" | "dlg" | "txt" | "csv" | "sfx" | "bdf")
    ) {
        Kind::Data
    } else {
        Kind::Code
    }
}

/// The files a tool writes (`Do not edit` at their top): the agents' skill
/// loaders (`scripts/sync-agent-skills.ps1`), a font's atlas (`cargo shuyi
/// font-atlas`) and a game's identity (`cargo shuyi app-info`, outside the
/// engine's own crates).
fn is_generated(path: &str, name: &str) -> bool {
    path.starts_with(".agents/skills/")
        || path.ends_with("/font/atlas.ron")
        || (name == "app_info.rs" && !path.starts_with("crates/"))
}

/// Each file's added plus removed lines from `git diff --numstat -z`;
/// binary files (`-`) are left out. A rename (`added\tremoved\t\0old\0new\0`)
/// counts under its new path.
pub fn numstat(output: &str) -> Vec<(String, usize)> {
    let mut files = Vec::new();
    let mut fields = output.split('\0');
    while let Some(entry) = fields.next() {
        let mut parts = entry.splitn(3, '\t');
        let (Some(added), Some(removed), Some(path)) = (parts.next(), parts.next(), parts.next())
        else {
            continue;
        };
        let path = if path.is_empty() {
            fields.next();
            fields.next().unwrap_or_default()
        } else {
            path
        };
        if let (Ok(added), Ok(removed)) = (added.parse::<usize>(), removed.parse::<usize>()) {
            files.push((path.to_owned(), added + removed));
        }
    }
    files
}

/// The lines of an untracked file, or `None` for a binary one (a NUL byte,
/// as git judges it).
pub fn untracked_lines(bytes: &[u8]) -> Option<usize> {
    if bytes.contains(&0) {
        return None;
    }
    let pieces = bytes.split(|&b| b == b'\n').count();
    Some(pieces - usize::from(bytes.is_empty() || bytes.ends_with(b"\n")))
}

/// `n` with a comma every three digits.
pub fn thousands(n: usize) -> String {
    let digits = n.to_string();
    let mut out = String::new();
    for (i, digit) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(digit);
    }
    out
}

/// A PR's size: its files by kind.
#[derive(Debug, Default)]
pub struct Size {
    pub files: Vec<(String, usize, Kind)>,
}

impl Size {
    pub fn of(files: Vec<(String, usize)>) -> Self {
        Size {
            files: files
                .into_iter()
                .map(|(path, lines)| {
                    let kind = classify(&path);
                    (path, lines, kind)
                })
                .collect(),
        }
    }

    fn total(&self, keep: impl Fn(Kind) -> bool) -> usize {
        self.files
            .iter()
            .filter(|(_, _, kind)| keep(*kind))
            .map(|(_, lines, _)| lines)
            .sum()
    }

    /// The lines the PR's size is.
    pub fn counted(&self) -> usize {
        self.total(Kind::counted)
    }

    /// The lines left out, generated or kept for a test.
    pub fn left_out(&self) -> usize {
        self.total(|k| matches!(k, Kind::LeftOut(_)))
    }

    /// The report: the counted total by kind, the left-out total by why,
    /// the tickets, and the largest counted files.
    pub fn report(&self, from: &str) -> String {
        let mut out = format!(
            "pr-size: {} counted lines (added + removed), {from} to the working tree\n",
            thousands(self.counted())
        );
        let by = |kinds: &mut dyn Iterator<Item = Kind>| {
            kinds
                .map(|k| format!("{} {}", k.name(), thousands(self.total(|f| f == k))))
                .collect::<Vec<_>>()
                .join(", ")
        };
        let _ = writeln!(out, "  counted: {}", by(&mut Kind::COUNTED.into_iter()));
        let _ = writeln!(
            out,
            "  left out: {} lines ({}), and binary files",
            thousands(self.left_out()),
            by(&mut LeftOut::ALL.into_iter().map(Kind::LeftOut))
        );
        let _ = writeln!(
            out,
            "  tickets: {} lines, counted apart",
            thousands(self.total(|k| k == Kind::Tickets))
        );
        let mut largest: Vec<_> = self.files.iter().filter(|f| f.2.counted()).collect();
        largest.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        for (path, lines, kind) in largest.into_iter().take(5) {
            let _ = writeln!(out, "{lines:>7}  {path} ({})", kind.name());
        }
        out
    }
}

/// How a PR's size reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    Fine,
    Warn,
    OverTheLimit,
    /// A bot's, a Sonar batch's or a ticket-only PR: its size is not a
    /// ticket's work.
    Exempt,
}

impl Verdict {
    pub fn of(counted: usize, kind: Option<&PrKind>) -> Self {
        if matches!(
            kind,
            Some(PrKind::Bot | PrKind::Sonar | PrKind::TicketsOnly(_) | PrKind::Post)
        ) {
            Verdict::Exempt
        } else if counted > LIMIT {
            Verdict::OverTheLimit
        } else if counted > WARN {
            Verdict::Warn
        } else {
            Verdict::Fine
        }
    }
}

/// Where the PR's ticket comes from, and so whether a missing `Size:` line
/// fails (CI) or only warns (a local run, the hook).
pub enum TicketSource {
    /// A ticket's file, read: its display path and body.
    File(String, String),
    /// No ticket to read, and why.
    Missing(String),
    /// A bot's, a Sonar batch's or a ticket-only PR.
    Exempt,
}

/// The problem with a PR of `counted` lines, if any, against its ticket's
/// `Size:` line. A number within a tenth of the measured one passes: a
/// merge or the line itself moves it.
pub fn size_problem(counted: usize, ticket: &TicketSource) -> Option<String> {
    if counted <= LIMIT {
        return None;
    }
    let lines = thousands(counted);
    let split = "Split what can wait into a new ticket (write-ticket), or write \
                 `Size: <number> counted lines, <why it wasn't split>` in the ticket's \
                 completion notes (work-ticket §3)";
    let (path, body) = match ticket {
        TicketSource::Exempt => return None,
        TicketSource::Missing(why) => {
            return Some(format!(
                "this PR is at {lines} counted lines, over the limit of {}: {why}. {split}",
                thousands(LIMIT)
            ));
        }
        TicketSource::File(path, body) => (path, body),
    };
    match tickets::size_line(body) {
        Err(e) => Some(format!("{path}: {e}")),
        Ok(None) => Some(format!(
            "this PR is at {lines} counted lines, over the limit of {}, and {path} has no \
             `Size:` line. {split}",
            thousands(LIMIT)
        )),
        Ok(Some(written)) if written.abs_diff(counted) * 10 > counted => Some(format!(
            "{path}: its `Size:` line says {}, but the PR is at {lines} counted lines: write \
             the measured number",
            thousands(written)
        )),
        Ok(Some(_)) => None,
    }
}

/// The warning for a PR of `counted` lines between [`WARN`] and [`LIMIT`].
fn warning(counted: usize) -> String {
    format!(
        "this PR is at {} counted lines, past the {} a ticket is planned at: note where \
         it would split cleanly before it grows further (work-ticket §3)",
        thousands(counted),
        thousands(WARN)
    )
}

/// What a run found: the report, and the errors and warnings for
/// `report` in `main.rs`.
#[derive(Debug, Default)]
pub struct Outcome {
    pub report: String,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

/// Judges a PR of `size`: past [`WARN`] a warning, and past [`LIMIT`] a
/// problem with its ticket's `Size:` line, an error when `strict` (CI with
/// the PR's title or an explicit ticket) and else a warning.
pub fn judge(
    size: &Size,
    kind: Option<&PrKind>,
    ticket: &TicketSource,
    strict: bool,
) -> (Vec<String>, Vec<String>) {
    let counted = size.counted();
    match Verdict::of(counted, kind) {
        Verdict::Fine | Verdict::Exempt => (Vec::new(), Vec::new()),
        Verdict::Warn => (Vec::new(), vec![warning(counted)]),
        Verdict::OverTheLimit => {
            let problem = size_problem(counted, ticket).into_iter().collect();
            if strict {
                (problem, Vec::new())
            } else {
                (Vec::new(), problem)
            }
        }
    }
}

fn git(repo_root: &Path, args: &[&str]) -> Result<String, String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(repo_root)
        .output()
        .map_err(|e| format!("git did not run ({e})"))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    } else {
        Err(format!(
            "git {}: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}

/// The changed files of `repo_root`'s working tree against `base`'s merge
/// base with `HEAD`, untracked files included, and the short hash counted
/// from.
pub fn changed_files(
    repo_root: &Path,
    base: &str,
) -> Result<(Vec<(String, usize)>, String), String> {
    let from = git(repo_root, &["merge-base", base, "HEAD"])?
        .trim()
        .to_owned();
    let diff = git(
        repo_root,
        &["diff", "--numstat", "-z", "-M", "--no-ext-diff", &from],
    )?;
    let mut files = numstat(&diff);
    let untracked = git(
        repo_root,
        &["ls-files", "--others", "--exclude-standard", "-z"],
    )?;
    for path in untracked.split('\0').filter(|p| !p.is_empty()) {
        if let Some(lines) = std::fs::read(repo_root.join(path))
            .ok()
            .and_then(|bytes| untracked_lines(&bytes))
        {
            files.push((path.to_owned(), lines));
        }
    }
    Ok((files, from.chars().take(10).collect()))
}

/// The top of the git working tree that holds `dir`.
pub fn repo_of(dir: &Path) -> Result<PathBuf, String> {
    Ok(PathBuf::from(
        git(dir, &["rev-parse", "--show-toplevel"])?.trim(),
    ))
}

/// The base a run counts from: `--base`, else `origin/main`, else `main`.
fn base_of(repo_root: &Path, base: Option<&str>) -> String {
    base.map_or_else(
        || {
            if git(repo_root, &["rev-parse", "--verify", "-q", "origin/main"]).is_ok() {
                "origin/main".to_owned()
            } else {
                "main".to_owned()
            }
        },
        str::to_owned,
    )
}

/// The ticket file of `id` in `tickets/done/`, or, when `open_too`, in
/// `tickets/open/` (a session measuring before it archives the ticket).
fn ticket_file(repo_root: &Path, id: &str, open_too: bool) -> TicketSource {
    let folders: &[&str] = if open_too {
        &["done", "open"]
    } else {
        &["done"]
    };
    for folder in folders {
        let dir = repo_root.join("tickets").join(folder);
        let Ok(files) = tickets::read_ticket_files(&dir) else {
            continue;
        };
        if let Some((name, body)) = files
            .into_iter()
            .find(|(name, _)| name.starts_with(&format!("{id}-")))
        {
            return TicketSource::File(format!("tickets/{folder}/{name}"), body);
        }
    }
    TicketSource::Missing(format!("ticket {id} is not in tickets/done/"))
}

/// The PR's kind and ticket, from `--ticket`, else `--pr-branch` and
/// `--pr-title` (CI), else the current branch (a local run). Returns
/// whether a missing `Size:` line fails.
fn pr_ticket(
    repo_root: &Path,
    options: &Options,
) -> Result<(Option<PrKind>, TicketSource, bool), String> {
    if let Some(file) = &options.ticket {
        let body = std::fs::read_to_string(repo_root.join(file))
            .map_err(|e| format!("{}: {e}", file.display()))?;
        return Ok((
            None,
            TicketSource::File(file.display().to_string(), body),
            true,
        ));
    }
    let (branch, title, strict) = match &options.pr_branch {
        Some(branch) => (branch.clone(), options.pr_title.as_deref(), true),
        None => (
            git(repo_root, &["rev-parse", "--abbrev-ref", "HEAD"])?
                .trim()
                .to_owned(),
            None,
            false,
        ),
    };
    let kind = Pr {
        branch: &branch,
        title,
    }
    .kind()?;
    let ticket = match &kind {
        PrKind::Work(id) => ticket_file(repo_root, id, !strict),
        PrKind::Bot | PrKind::Sonar | PrKind::TicketsOnly(_) | PrKind::Post => TicketSource::Exempt,
        PrKind::GameSmall(id) => TicketSource::Missing(format!(
            "game ticket {id}'s small change (`Game {id}: …`) has no ticket here to say why"
        )),
        PrKind::None => TicketSource::Missing(format!("branch `{branch}` names no ticket")),
    };
    Ok((Some(kind), ticket, strict))
}

/// Measures and judges `repo_root`'s PR.
pub fn run(repo_root: &Path, options: &Options) -> Outcome {
    let base = base_of(repo_root, options.base.as_deref());
    let (files, from) = match changed_files(repo_root, &base) {
        Ok(found) => found,
        Err(e) => {
            return Outcome {
                errors: vec![e],
                ..Outcome::default()
            };
        }
    };
    let size = Size::of(files);
    let mut outcome = Outcome {
        report: size.report(&from),
        ..Outcome::default()
    };
    match pr_ticket(repo_root, options) {
        Ok((kind, ticket, strict)) => {
            let (errors, warnings) = judge(&size, kind.as_ref(), &ticket, strict);
            outcome.errors = errors;
            outcome.warnings = warnings;
        }
        Err(e) => outcome.errors.push(e),
    }
    outcome
}

/// What a Claude Code `PreToolUse` hook is given on stdin, as far as
/// `pr-size` needs it: the session's directory and the shell command.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct HookInput {
    pub cwd: Option<PathBuf>,
    pub command: String,
}

/// Reads the hook's input; a missing field (or no JSON at all, from a
/// person running `--hook` by hand) is left empty.
pub fn hook_input(json: &str) -> HookInput {
    let value: serde_json::Value = serde_json::from_str(json).unwrap_or_default();
    HookInput {
        cwd: value["cwd"]
            .as_str()
            .filter(|cwd| !cwd.is_empty())
            .map(PathBuf::from),
        command: value["tool_input"]["command"]
            .as_str()
            .unwrap_or_default()
            .to_owned(),
    }
}

/// Whether a shell command runs `git commit`, `git push` or `gh pr
/// create`. The hooks' `if` rules also fire on commands Claude Code can't
/// analyze (a `$VAR`, a `$(...)`), so the hook checks for itself.
pub fn runs_a_pr_step(command: &str) -> bool {
    let words: Vec<&str> = command
        .split(|c: char| c.is_whitespace() || ";&|(){}".contains(c))
        .filter(|word| !word.is_empty())
        .collect();
    words
        .windows(2)
        .any(|pair| matches!(pair, ["git", "commit" | "push"]))
        || words
            .windows(3)
            .any(|three| three == ["gh", "pr", "create"])
}

/// A Claude Code `PreToolUse` hook's answer that gives the session
/// `message` and lets the command run.
fn hook_json(message: &str) -> String {
    serde_json::json!({
        "hookSpecificOutput": {
            "hookEventName": "PreToolUse",
            "additionalContext": format!("pr-size: {message}"),
        }
    })
    .to_string()
}

/// The hook's answer for its input `stdin`: nothing unless the command is a
/// PR step, else the warning for the repository of the session's `cwd` (the
/// hook process itself runs in the main checkout, not the worktree).
/// `fallback` is measured when the input names no directory. Any git error
/// is silence: the hook never gets in the way.
pub fn hook_answer(stdin: &str, fallback: &Path, base: Option<&str>) -> String {
    let input = hook_input(stdin);
    if !input.command.is_empty() && !runs_a_pr_step(&input.command) {
        return String::new();
    }
    let root = match &input.cwd {
        Some(cwd) => match repo_of(cwd) {
            Ok(top) => top,
            Err(_) => return String::new(),
        },
        None => fallback.to_path_buf(),
    };
    let options = Options {
        base: base.map(str::to_owned),
        ..Options::default()
    };
    let outcome = run(&root, &options);
    if !outcome.errors.is_empty() || outcome.warnings.is_empty() {
        return String::new();
    }
    hook_json(&outcome.warnings.join("\n")) + "\n"
}

#[cfg(test)]
mod tests;
