//! The flaky gates' ledger, `tools/flaky.ron` (ticket 2064): each cell CI is
//! known to fail now and then without a change to blame, its open ticket, the
//! day it was seen and its measured rate.
//!
//! - `ticket-lint` holds each entry's ticket open: fixing a flake removes its
//!   entry ([`lint`]).
//! - `flaky rerun` judges a failed CI run (`.github/workflows/flaky-rerun.yml`):
//!   when every failed job failed on listed cells, CI is re-run **once**.
//!   A cell not listed gets no re-run.
//! - `flaky report` names, on the re-run attempt's `ci-result`, what the
//!   attempt before it failed on, as a warning: a re-run is never a pass with
//!   no trace.
//! - `flaky calibration` writes the table of a calibration run
//!   (`.github/workflows/calibrate.yml`): each run's jobs and failed cells,
//!   and each cell's rate.
//!
//! A failed job's facts come from `scripts/flaky-failures.sh`, as JSON: every
//! job of the attempt with its conclusion, and for a failed one its log and,
//! for a job the ledger's `matrices` name, its `matrix.json`.

use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::fmt::{self, Write as _};
use std::io::Write as _;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use serde_json::Value;

/// The ledger, from the repository's root.
pub const LEDGER: &str = "tools/flaky.ron";

/// The aggregator, which fails whenever another job does.
const AGGREGATOR: &str = "ci-result";

/// What the `run-gates` skill says to do about a re-run, named in warnings.
const RULE: &str = "the run-gates skill, *When CI fails but local passes*";

/// `tools/flaky.ron`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Ledger {
    /// The jobs whose failures are the cells of a `matrix.json`.
    #[serde(default)]
    pub matrices: Vec<MatrixJob>,
    pub entries: Vec<Entry>,
}

/// A job whose failures are the failed cells of a `matrix.json` it uploads.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MatrixJob {
    /// The job, as the jobs API names it.
    pub job: String,
    /// The artifact whose root holds its `matrix.json`.
    pub artifact: String,
    /// The platforms it judges from another job's recording.
    #[serde(default)]
    pub recorded: Vec<Recorded>,
}

/// Cells a job judges from what another job recorded. Re-running the judge
/// alone judges the same recording again, so a failed one re-runs the job
/// that records, and the judge after it.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Recorded {
    /// The platforms whose name starts with this.
    pub platforms: String,
    /// The job that records them.
    pub job: String,
}

/// One known flaky cell.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Entry {
    pub cell: Cell,
    /// The open ticket that fixes it.
    pub ticket: String,
    /// The day it was first seen, `YYYY-MM-DD`.
    pub seen: String,
    /// How often it failed when measured.
    pub rate: Rate,
    /// The runs that measured the rate.
    pub runs: Vec<String>,
}

/// What fails: a whole job, one test of a job, or a cell of a matrix job's
/// `matrix.json`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub enum Cell {
    /// Any failure of the job of that name (as the jobs API names it).
    Job(String),
    /// A test of that job that `cargo test` reports `FAILED`.
    Test { job: String, name: String },
    /// A mandatory cell of a matrix job's `matrix.json` that didn't pass.
    Matrix { platform: String, check: String },
}

impl fmt::Display for Cell {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Cell::Job(job) => write!(f, "job `{job}`"),
            Cell::Test { job, name } => write!(f, "test `{name}` of `{job}`"),
            Cell::Matrix { platform, check } => write!(f, "cell `{platform} {check}`"),
        }
    }
}

/// `failed` runs of `runs`.
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rate {
    pub failed: u32,
    pub runs: u32,
}

impl fmt::Display for Entry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} (ticket {}, {} in {}, seen {})",
            self.cell, self.ticket, self.rate.failed, self.rate.runs, self.seen
        )
    }
}

/// The ledger's text, read and checked: four-digit tickets, dates, a rate of
/// at least one failure in no more runs than measured, the runs named, and no
/// cell twice.
pub fn parse(text: &str) -> Result<Ledger, Vec<String>> {
    let ledger: Ledger = ron::from_str(text).map_err(|e| vec![format!("{LEDGER}: {e}")])?;
    let mut errors = Vec::new();
    for (i, entry) in ledger.entries.iter().enumerate() {
        let at = format!("{LEDGER}: {}", entry.cell);
        if entry.ticket.len() != 4 || !entry.ticket.bytes().all(|b| b.is_ascii_digit()) {
            errors.push(format!(
                "{at}: ticket `{}` is not a four-digit id",
                entry.ticket
            ));
        }
        if !is_date(&entry.seen) {
            errors.push(format!("{at}: seen `{}` is not YYYY-MM-DD", entry.seen));
        }
        if entry.rate.failed == 0 || entry.rate.failed > entry.rate.runs {
            errors.push(format!(
                "{at}: a rate of {} in {} runs is no measured flake",
                entry.rate.failed, entry.rate.runs
            ));
        }
        if entry.runs.is_empty() {
            errors.push(format!("{at}: no runs named beside its rate"));
        }
        if ledger.entries[..i].iter().any(|e| e.cell == entry.cell) {
            errors.push(format!("{at}: listed twice"));
        }
    }
    if errors.is_empty() {
        Ok(ledger)
    } else {
        Err(errors)
    }
}

fn is_date(text: &str) -> bool {
    let b = text.as_bytes();
    b.len() == 10
        && b[4] == b'-'
        && b[7] == b'-'
        && b.iter()
            .enumerate()
            .all(|(i, c)| i == 4 || i == 7 || c.is_ascii_digit())
}

/// `ticket-lint`'s check of the ledger in `repo_root`: it parses, and each
/// entry's ticket is open. A ticket in `done/` means the flake was fixed, so
/// its entry goes. No ledger is no error.
pub fn lint(repo_root: &Path, open: &HashSet<&str>, done: &HashSet<&str>) -> Vec<String> {
    let Ok(text) = std::fs::read_to_string(repo_root.join(LEDGER)) else {
        return Vec::new();
    };
    match parse(&text) {
        Ok(ledger) => check_tickets(&ledger, open, done),
        Err(errors) => errors,
    }
}

/// Each entry whose ticket isn't open.
pub fn check_tickets(ledger: &Ledger, open: &HashSet<&str>, done: &HashSet<&str>) -> Vec<String> {
    ledger
        .entries
        .iter()
        .filter(|e| !open.contains(e.ticket.as_str()))
        .map(|e| {
            if done.contains(e.ticket.as_str()) {
                format!(
                    "{LEDGER}: {} names ticket {}, which is done: a fixed flake's entry is removed",
                    e.cell, e.ticket
                )
            } else {
                format!(
                    "{LEDGER}: {} names ticket {}, which doesn't exist",
                    e.cell, e.ticket
                )
            }
        })
        .collect()
}

/// One job of a CI attempt, as `scripts/flaky-failures.sh` writes it.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JobRecord {
    pub name: String,
    pub id: u64,
    /// `success`, `failure`, `cancelled`, `skipped`… or null while running.
    pub conclusion: Option<String>,
    /// A failed job's log.
    #[serde(default)]
    pub log: Option<PathBuf>,
    /// A matrix job's `matrix.json`, when it failed and left one.
    #[serde(default)]
    pub matrix: Option<PathBuf>,
}

/// What a job's failure was, read from its log and matrix.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Job {
    pub name: String,
    pub id: u64,
    pub failed: bool,
    /// The tests its log reports `FAILED`.
    pub failed_tests: Vec<String>,
    /// A matrix job's mandatory cells that didn't pass, when it left a
    /// matrix: `(platform, check)`.
    pub matrix_cells: Option<Vec<(String, String)>>,
}

/// Reads `scripts/flaky-failures.sh`'s JSON and each failed job's files.
pub fn load_jobs(failures: &Path) -> Result<Vec<Job>, String> {
    let text =
        std::fs::read_to_string(failures).map_err(|e| format!("{}: {e}", failures.display()))?;
    let records: Vec<JobRecord> =
        serde_json::from_str(&text).map_err(|e| format!("{}: {e}", failures.display()))?;
    records
        .into_iter()
        .map(|r| {
            let failed_tests = match &r.log {
                Some(log) => failed_tests(
                    &std::fs::read_to_string(log).map_err(|e| format!("{}: {e}", log.display()))?,
                ),
                None => Vec::new(),
            };
            let matrix_cells = match &r.matrix {
                Some(matrix) => Some(matrix_cells(
                    &std::fs::read_to_string(matrix)
                        .map_err(|e| format!("{}: {e}", matrix.display()))?,
                )?),
                None => None,
            };
            Ok(Job {
                failed: r.conclusion.as_deref() == Some("failure"),
                name: r.name,
                id: r.id,
                failed_tests,
                matrix_cells,
            })
        })
        .collect()
}

/// The tests `cargo test` reports `FAILED` in a job's log, whose lines may
/// start with a timestamp: `… test a::b ... FAILED`. Each once, in order.
pub fn failed_tests(log: &str) -> Vec<String> {
    let mut seen = Vec::new();
    for line in log.lines() {
        let Some(rest) = line.trim_end().strip_suffix(" ... FAILED") else {
            continue;
        };
        let Some(at) = rest.rfind("test ") else {
            continue;
        };
        let name = &rest[at + "test ".len()..];
        if !name.is_empty()
            && !name.contains(char::is_whitespace)
            && !seen.iter().any(|s| s == name)
        {
            seen.push(name.to_string());
        }
    }
    seen
}

/// The mandatory cells that didn't pass, from a `matrix.json`.
pub fn matrix_cells(matrix: &str) -> Result<Vec<(String, String)>, String> {
    let value: Value = serde_json::from_str(matrix).map_err(|e| format!("matrix.json: {e}"))?;
    let cells = value["cells"]
        .as_array()
        .ok_or("matrix.json: no `cells` list")?;
    Ok(cells
        .iter()
        .filter(|c| c["mandatory"] != Value::Bool(false) && c["status"] != "passed")
        .map(|c| {
            (
                c["platform"].as_str().unwrap_or_default().to_string(),
                c["check"].as_str().unwrap_or_default().to_string(),
            )
        })
        .collect())
}

/// What to re-run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rerun {
    /// Nothing: no failure, one not listed, or this was the re-run.
    None,
    /// The failed jobs, with what depends on them.
    Failed,
    /// This job, with what depends on it.
    Job(u64),
}

impl fmt::Display for Rerun {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Rerun::None => f.write_str("none"),
            Rerun::Failed => f.write_str("failed"),
            Rerun::Job(id) => write!(f, "{id}"),
        }
    }
}

/// What a failed attempt's jobs were found to be.
#[derive(Debug, Default)]
pub struct Judgement<'a> {
    /// Each failed job a listed cell explains, with the entries.
    pub matched: Vec<(String, Vec<&'a Entry>)>,
    /// Each failed job none does, and why.
    pub unmatched: Vec<(String, String)>,
}

/// Matches each failed job of an attempt against the ledger. The aggregator
/// is left out: it fails whenever another job does.
pub fn judge<'a>(ledger: &'a Ledger, jobs: &[Job]) -> Judgement<'a> {
    let mut judgement = Judgement::default();
    for job in jobs.iter().filter(|j| j.failed && j.name != AGGREGATOR) {
        match explain(ledger, job) {
            Ok(entries) => judgement.matched.push((job.name.clone(), entries)),
            Err(why) => judgement.unmatched.push((job.name.clone(), why)),
        }
    }
    judgement
}

/// The entries that explain a failed job, or why none do.
fn explain<'a>(ledger: &'a Ledger, job: &Job) -> Result<Vec<&'a Entry>, String> {
    if let Some(entry) = ledger
        .entries
        .iter()
        .find(|e| matches!(&e.cell, Cell::Job(name) if *name == job.name))
    {
        return Ok(vec![entry]);
    }
    if ledger.matrices.iter().any(|m| m.job == job.name) {
        let cells = job
            .matrix_cells
            .as_deref()
            .filter(|cells| !cells.is_empty())
            .ok_or("no failed cell in its matrix: a step before its judge failed")?;
        return listed(
            cells.iter().map(|(platform, check)| Cell::Matrix {
                platform: platform.clone(),
                check: check.clone(),
            }),
            ledger,
        );
    }
    if job.failed_tests.is_empty() {
        return Err("no failed test in its log".to_string());
    }
    listed(
        job.failed_tests.iter().map(|name| Cell::Test {
            job: job.name.clone(),
            name: name.clone(),
        }),
        ledger,
    )
}

/// The entry of each cell, or the cells none lists.
fn listed(cells: impl Iterator<Item = Cell>, ledger: &Ledger) -> Result<Vec<&Entry>, String> {
    let mut found = Vec::new();
    let mut missing = Vec::new();
    for cell in cells {
        match ledger.entries.iter().find(|e| e.cell == cell) {
            Some(entry) => found.push(entry),
            None => missing.push(cell.to_string()),
        }
    }
    if missing.is_empty() {
        Ok(found)
    } else {
        Err(format!("{} not listed", missing.join(", ")))
    }
}

/// What to re-run after attempt `attempt` failed, and why, as warnings.
/// Only a first attempt whose every failed job a listed cell explains is
/// re-run. A cell judged from another job's recording re-runs that job, when
/// its judge is the only failed job; with others failed too, nothing is
/// re-run, as one call can't re-run both.
pub fn rerun(ledger: &Ledger, jobs: &[Job], attempt: u32) -> (Rerun, Vec<String>) {
    let judgement = judge(ledger, jobs);
    let mut notes = Vec::new();
    for (job, why) in &judgement.unmatched {
        notes.push(format!(
            "no re-run: `{job}` failed with {why} in {LEDGER}; see {RULE}"
        ));
    }
    if judgement.matched.is_empty() || !judgement.unmatched.is_empty() {
        return (Rerun::None, notes);
    }
    let named = describe(&judgement.matched);
    if attempt != 1 {
        notes.push(format!(
            "no re-run: attempt {attempt} is past the first, and CI re-runs once: {named}"
        ));
        return (Rerun::None, notes);
    }
    let recorders: BTreeSet<&str> = judgement
        .matched
        .iter()
        .flat_map(|(job, entries)| entries.iter().map(move |e| (job, e)))
        .filter_map(|(job, entry)| recorder(ledger, job, &entry.cell))
        .collect();
    let Some(&recorder) = recorders.first() else {
        notes.push(format!("re-running the failed jobs once: {named}"));
        return (Rerun::Failed, notes);
    };
    if recorders.len() > 1 || judgement.matched.len() > 1 {
        notes.push(format!(
            "no re-run: a cell re-runs the job that recorded it, `{recorder}`, and other jobs failed too: {named}"
        ));
        return (Rerun::None, notes);
    }
    if let Some(job) = jobs.iter().find(|j| j.name == recorder) {
        notes.push(format!(
            "re-running `{recorder}` once, which records anew for its judge: {named}"
        ));
        (Rerun::Job(job.id), notes)
    } else {
        notes.push(format!(
            "no re-run: no `{recorder}` job in the run: {named}"
        ));
        (Rerun::None, notes)
    }
}

/// The job that recorded a matrix cell of `job`, if another one did.
fn recorder<'a>(ledger: &'a Ledger, job: &str, cell: &Cell) -> Option<&'a str> {
    let Cell::Matrix { platform, .. } = cell else {
        return None;
    };
    ledger
        .matrices
        .iter()
        .filter(|m| m.job == job)
        .flat_map(|m| &m.recorded)
        .find(|r| platform.starts_with(&r.platforms))
        .map(|r| r.job.as_str())
}

/// The warnings a re-run attempt's `ci-result` shows about attempt
/// `attempt`, the one before it: what it failed on, listed or not.
pub fn report(ledger: &Ledger, jobs: &[Job], attempt: u32) -> Vec<String> {
    let judgement = judge(ledger, jobs);
    let mut warnings = Vec::new();
    if !judgement.matched.is_empty() {
        warnings.push(format!(
            "this is a re-run: attempt {attempt} failed on listed flakes ({LEDGER}): {}. Name the re-run and its link in the PR body ({RULE})",
            describe(&judgement.matched)
        ));
    }
    for (job, why) in &judgement.unmatched {
        warnings.push(format!(
            "this is a re-run: attempt {attempt}'s `{job}` failed with {why} in {LEDGER}. Name the re-run and its link in the PR body; a second sighting gets an entry and a ticket ({RULE})"
        ));
    }
    warnings
}

fn describe(matched: &[(String, Vec<&Entry>)]) -> String {
    matched
        .iter()
        .map(|(job, entries)| {
            let entries: Vec<String> = entries.iter().map(ToString::to_string).collect();
            format!("`{job}` on {}", entries.join("; "))
        })
        .collect::<Vec<_>>()
        .join("; ")
}

/// One job of a calibration run, as the jobs API lists it.
#[derive(Debug, Deserialize)]
pub struct CalibrationJob {
    pub name: String,
    pub conclusion: Option<String>,
}

/// A calibration run's table, in Markdown: for each run, each of its jobs'
/// conclusions and the cells that failed, then each cell's rate. Runs are
/// the jobs named `<job> (<n>) / <inner job>`; a run's cells are read from
/// `<dir>/<job>-run-<n>/matrix.json` when it has one.
pub fn calibration(
    job: &str,
    jobs: &[CalibrationJob],
    dir: &Path,
    revision: &str,
) -> Result<String, String> {
    // run -> inner job -> conclusion
    let mut runs: BTreeMap<u32, BTreeMap<String, String>> = BTreeMap::new();
    let mut inner_jobs = BTreeSet::new();
    let prefix = format!("{job} (");
    for j in jobs {
        let Some(rest) = j.name.strip_prefix(&prefix) else {
            continue;
        };
        let Some((n, inner)) = rest.split_once(") / ") else {
            continue;
        };
        let Ok(n) = n.parse::<u32>() else { continue };
        inner_jobs.insert(inner.to_string());
        runs.entry(n).or_default().insert(
            inner.to_string(),
            j.conclusion
                .clone()
                .unwrap_or_else(|| "unfinished".to_string()),
        );
    }
    if runs.is_empty() {
        return Err(format!("no job named `{job} (<n>) / …` in the run"));
    }
    let mut cells: BTreeMap<String, Vec<u32>> = BTreeMap::new();
    let mut out = format!(
        "## Calibration of `{job}`: {} runs of {revision}\n\n",
        runs.len()
    );
    out.push_str("| Run |");
    for inner in &inner_jobs {
        let _ = write!(out, " `{inner}` |");
    }
    out.push_str(" Failed cells |\n|---|");
    for _ in &inner_jobs {
        out.push_str("---|");
    }
    out.push_str("---|\n");
    let mut passed = 0;
    for (n, conclusions) in &runs {
        let mut row = format!("| {n} |");
        for inner in &inner_jobs {
            let _ = write!(
                row,
                " {} |",
                conclusions.get(inner).map_or("-", String::as_str)
            );
        }
        let matrix = dir.join(format!("{job}-run-{n}")).join("matrix.json");
        let failed = match std::fs::read_to_string(&matrix) {
            Ok(text) => {
                let failed =
                    matrix_cells(&text).map_err(|e| format!("{}: {e}", matrix.display()))?;
                for (platform, check) in &failed {
                    cells
                        .entry(format!("{platform} {check}"))
                        .or_default()
                        .push(*n);
                }
                if failed.is_empty() {
                    "none".to_string()
                } else {
                    failed
                        .iter()
                        .map(|(p, c)| format!("`{p} {c}`"))
                        .collect::<Vec<_>>()
                        .join(", ")
                }
            }
            Err(_) => "no matrix".to_string(),
        };
        if conclusions.values().all(|c| c == "success") {
            passed += 1;
        }
        let _ = writeln!(row, " {failed} |");
        out.push_str(&row);
    }
    let _ = writeln!(out, "\n{passed} of {} runs passed every job.", runs.len());
    if !cells.is_empty() {
        out.push_str("\n| Cell | Failed | Runs |\n|---|---|---|\n");
        for (cell, failed_in) in &cells {
            let list: Vec<String> = failed_in.iter().map(ToString::to_string).collect();
            let _ = writeln!(
                out,
                "| `{cell}` | {} in {} | {} |",
                failed_in.len(),
                runs.len(),
                list.join(", ")
            );
        }
    }
    Ok(out)
}

/// `cargo xtask flaky …`: the errors, the warnings, and what to write.
pub fn run(repo_root: &Path, args: &[String]) -> (Vec<String>, Vec<String>) {
    match command(repo_root, args) {
        Ok(warnings) => (Vec::new(), warnings),
        Err(errors) => (errors, Vec::new()),
    }
}

fn command(repo_root: &Path, args: &[String]) -> Result<Vec<String>, Vec<String>> {
    let (sub, rest) = args.split_first().ok_or_else(|| vec![USAGE.to_string()])?;
    let options = options(rest)?;
    let get = |key: &str| {
        options
            .get(key)
            .cloned()
            .ok_or_else(|| vec![format!("flaky {sub}: --{key} is required")])
    };
    match sub.as_str() {
        "rerun" | "report" => {
            let attempt: u32 = get("attempt")?
                .parse()
                .map_err(|e| vec![format!("--attempt: {e}")])?;
            let ledger = read_ledger(repo_root)?;
            let jobs = load_jobs(Path::new(&get("failures")?)).map_err(|e| vec![e])?;
            if sub == "report" {
                return Ok(report(&ledger, &jobs, attempt));
            }
            let (decision, notes) = rerun(&ledger, &jobs, attempt);
            std::fs::write(get("decision")?, format!("{decision}\n"))
                .map_err(|e| vec![format!("--decision: {e}")])?;
            Ok(notes)
        }
        "matrices" => {
            let mut stdout = std::io::stdout().lock();
            for matrix in &read_ledger(repo_root)?.matrices {
                writeln!(stdout, "{}\t{}", matrix.job, matrix.artifact)
                    .map_err(|e| vec![format!("stdout: {e}")])?;
            }
            Ok(Vec::new())
        }
        "calibration" => {
            let job = get("job")?;
            let text =
                std::fs::read_to_string(get("jobs")?).map_err(|e| vec![format!("--jobs: {e}")])?;
            let jobs: Vec<CalibrationJob> =
                serde_json::from_str(&text).map_err(|e| vec![format!("--jobs: {e}")])?;
            let table = calibration(&job, &jobs, Path::new(&get("dir")?), &get("revision")?)
                .map_err(|e| vec![e])?;
            std::fs::write(get("summary")?, table).map_err(|e| vec![format!("--summary: {e}")])?;
            Ok(Vec::new())
        }
        _ => Err(vec![USAGE.to_string()]),
    }
}

const USAGE: &str = "usage: cargo xtask flaky matrices | flaky rerun --failures <file> --attempt <n> --decision <file> | \
flaky report --failures <file> --attempt <n> | \
flaky calibration --job <name> --jobs <file> --dir <dir> --revision <sha> --summary <file>";

fn options(args: &[String]) -> Result<BTreeMap<String, String>, Vec<String>> {
    let mut options = BTreeMap::new();
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        let key = arg
            .strip_prefix("--")
            .ok_or_else(|| vec![format!("unexpected `{arg}`; {USAGE}")])?;
        let value = args
            .next()
            .ok_or_else(|| vec![format!("--{key} needs a value")])?;
        options.insert(key.to_string(), value.clone());
    }
    Ok(options)
}

fn read_ledger(repo_root: &Path) -> Result<Ledger, Vec<String>> {
    let text = std::fs::read_to_string(repo_root.join(LEDGER))
        .map_err(|e| vec![format!("{LEDGER}: {e}")])?;
    parse(&text)
}

#[cfg(test)]
mod tests;
