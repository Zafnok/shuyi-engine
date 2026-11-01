//! `cargo xtask sound-parity`: the first gate on leaving quad-snd (ticket
//! 2573, ADR-0066). `tools/sound-parity/README.md` says how to run it and
//! what each stage needs.
//!
//! - `corpus` writes the test signals into `--out`, each checked against its
//!   hash in `tools/sound-parity/corpus.ron`, and with `--oracle` what a
//!   decoder apart from the backend makes of each Ogg file.
//! - `gate` reads what `capture.py` recorded and writes `matrix.json`: one
//!   cell per platform and check, and exits non-zero unless every mandatory
//!   one passed. Missing, stale or silent evidence is a failure, never a skip.
//! - `port-ready <platform>` reads a matrix and exits non-zero unless that
//!   platform has real evidence: what a port's output work must pass first.
//! - `give-up [--apply]` takes the whole replacement out, on Nick's word alone
//!   ([`give_up`], ADR-0068).

mod compare;
mod corpus;
mod give_up;
#[cfg(test)]
mod tests;

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Deserialize;
use serde_json::{Value, json};

use self::compare::{Expect, Fault, Log, Recording, Tolerance};
use self::corpus::sha256;

const TOOL: &str = "tools/sound-parity";
/// A matrix's own version: a reader refuses any other.
const SCHEMA: u64 = 1;
/// How many interleaved pairs per workload the performance verdict needs.
const MIN_PAIRS: usize = 9;

/// `tools/sound-parity/corpus.ron`: each file's hash and how it was made.
#[derive(Debug, Deserialize)]
struct Manifest {
    /// Each `.wav` by name: its SHA-256.
    wav: BTreeMap<String, String>,
    /// Each `.ogg` by name.
    ogg: BTreeMap<String, OggFile>,
    /// How much the lossy encoding may differ from its source, in dB of
    /// signal to noise: a different thing from decoded equality.
    lossy_min_snr_db: f64,
    /// The tools that made the Ogg files and the decoded hashes.
    encoder: String,
    decoder: String,
}

#[derive(Debug, Deserialize)]
struct OggFile {
    sha256: String,
    /// The SHA-256 of what the pinned decoder makes of it, as raw 16-bit.
    decoded_sha256: String,
}

/// `tools/sound-parity/budget.ron`.
#[derive(Debug, Deserialize)]
pub(crate) struct Budget {
    version: u64,
    correctness: Tolerance,
    /// Each platform's verdicts, by metric.
    performance: BTreeMap<String, BTreeMap<String, Limit>>,
    /// Which platforms a parity claim needs, and who establishes the others.
    platforms: BTreeMap<String, Contract>,
}

/// How far an equal-work ratio of a metric may go.
#[derive(Debug, Clone, Copy, Deserialize)]
pub(crate) struct Limit {
    /// The largest median ratio that isn't a slowdown.
    pub(crate) max_ratio: f64,
    /// The largest spread of the ratios (their interquartile range) that
    /// still says anything: past it the verdict is inconclusive.
    pub(crate) max_spread: f64,
    /// Whether the injected slowdown moves it, so that it must be caught:
    /// memory and latency it doesn't touch.
    pub(crate) slows: bool,
}

#[derive(Debug, Clone, Deserialize)]
enum Contract {
    /// Its capture runs in CI: every cell of it is mandatory.
    Captured,
    /// Nothing captures it yet: this ticket has to establish that before
    /// that port's output work starts.
    Blocked(String),
}

/// The part of `scenarios.ron` the gate reads.
#[derive(Debug, Deserialize)]
struct Plan {
    scenarios: Vec<Scenario>,
}

#[derive(Debug, Deserialize)]
struct Scenario {
    name: String,
    /// The backends it holds on; `None` is every backend.
    backends: Option<Vec<String>>,
    steps: Vec<Step>,
    end: u32,
    expect: Vec<Expect>,
}

#[derive(Debug, Deserialize)]
struct Step {
    at: u32,
}

/// One run's meta file, written by `capture.py`.
#[derive(Debug, Deserialize)]
struct Meta {
    revision: String,
    backend: String,
    corpus_sha256: String,
    scenarios_sha256: String,
    /// The tools, the machine, the build's time and size: saved in the matrix.
    #[serde(default)]
    environment: Value,
}

/// Options of `corpus`.
struct CorpusOptions {
    out: PathBuf,
    oracle: bool,
}

/// Runs `cargo xtask sound-parity <args>`: what failed, if anything.
pub(crate) fn run(root: &Path, args: &[String]) -> Vec<String> {
    let strs: Vec<&str> = args.iter().map(String::as_str).collect();
    match strs.as_slice() {
        ["corpus", "--out", out] => write_corpus(
            root,
            &CorpusOptions {
                out: PathBuf::from(out),
                oracle: false,
            },
        ),
        ["corpus", "--out", out, "--oracle"] => write_corpus(
            root,
            &CorpusOptions {
                out: PathBuf::from(out),
                oracle: true,
            },
        ),
        ["gate", "--dir", dir, "--revision", revision] => gate(root, Path::new(dir), revision)
            .err()
            .unwrap_or_default(),
        ["port-ready", platform, "--matrix", matrix] => {
            match fs::read_to_string(matrix).map_err(|e| format!("{matrix}: {e}")) {
                Ok(text) => port_ready(&text, platform).err().into_iter().collect(),
                Err(e) => vec![e],
            }
        }
        ["give-up"] => give_up::run(root, false),
        ["give-up", "--apply"] => give_up::run(root, true),
        _ => vec![format!("sound-parity: don't know {args:?}")],
    }
}

fn read_ron<T: for<'a> Deserialize<'a>>(path: &Path) -> Result<T, String> {
    let text = fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    ron::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))
}

/// Writes every corpus file into `options.out/corpus`, and the decoded
/// expectations into `options.out/oracle`, proving each against its hash.
fn write_corpus(root: &Path, options: &CorpusOptions) -> Vec<String> {
    let tool = root.join(TOOL);
    let manifest: Manifest = match read_ron(&tool.join("corpus.ron")) {
        Ok(manifest) => manifest,
        Err(e) => return vec![e],
    };
    let dir = options.out.join("corpus");
    let oracle = options.out.join("oracle");
    if let Err(e) = fs::create_dir_all(&dir).and_then(|()| fs::create_dir_all(&oracle)) {
        return vec![format!("{}: {e}", options.out.display())];
    }
    let mut errors = Vec::new();
    let sources = corpus::sources();
    for (name, signal) in &sources {
        let bytes = corpus::wav(signal);
        let hash = sha256(&bytes);
        if manifest.wav.get(*name) != Some(&hash) {
            errors.push(format!("{name}: its hash is {hash}, not corpus.ron's"));
        }
        if let Err(e) = fs::write(dir.join(name), bytes) {
            errors.push(format!("{name}: {e}"));
        }
    }
    for (name, ogg) in &manifest.ogg {
        let path = tool.join("corpus").join(name);
        let Ok(bytes) = fs::read(&path) else {
            errors.push(format!("{}: can't read it", path.display()));
            continue;
        };
        if sha256(&bytes) != ogg.sha256 {
            errors.push(format!("{name}: not the file corpus.ron pinned"));
        }
        if let Err(e) = fs::write(dir.join(name), &bytes) {
            errors.push(format!("{name}: {e}"));
        }
        if options.oracle {
            let source = name.replace(".ogg", ".wav");
            let source = sources.iter().find(|(n, _)| *n == source).map(|(_, s)| s);
            errors.extend(decode_oracle(&dir, &oracle, name, ogg, source, &manifest));
        }
    }
    errors
}

/// Runs the pinned decoder on `name` and holds what it makes to the manifest:
/// equal to the hash, and near enough to the lossy file's source.
fn decode_oracle(
    dir: &Path,
    oracle: &Path,
    name: &str,
    ogg: &OggFile,
    source: Option<&corpus::Signal>,
    manifest: &Manifest,
) -> Option<String> {
    let out = oracle.join(format!("{name}.pcm"));
    let ran = Command::new("oggdec")
        .args([
            "--quiet",
            "--raw",
            "--bits=16",
            "--endianness=0",
            "--sign=1",
        ])
        .arg("--output")
        .arg(&out)
        .arg(dir.join(name))
        .status();
    if !ran.is_ok_and(|status| status.success()) {
        return Some(format!(
            "{name}: `oggdec` ({}) didn't run",
            manifest.decoder
        ));
    }
    let bytes = fs::read(&out).ok()?;
    if sha256(&bytes) != ogg.decoded_sha256 {
        return Some(format!(
            "{name}: decoded to {}, not corpus.ron's: not the pinned decoder ({})",
            sha256(&bytes),
            manifest.decoder
        ));
    }
    let source = source?;
    let decoded = corpus::samples(&bytes);
    let snr = snr_db(&source.samples, &decoded);
    (snr < manifest.lossy_min_snr_db).then(|| {
        format!(
            "{name}: {snr:.1} dB from its source, under {} ({})",
            manifest.lossy_min_snr_db, manifest.encoder
        )
    })
}

/// How far `decoded` is from `source`, in dB of signal to noise.
fn snr_db(source: &[i16], decoded: &[i16]) -> f64 {
    let (mut signal, mut noise) = (0.0, 0.0);
    for (a, b) in source.iter().zip(decoded) {
        signal += f64::from(*a).powi(2);
        noise += (f64::from(*a) - f64::from(*b)).powi(2);
    }
    if noise == 0.0 {
        f64::INFINITY
    } else {
        10.0 * (signal / noise).log10()
    }
}

/// One cell of the matrix.
fn cell(platform: &str, check: &str, mandatory: bool, status: &str, detail: &str) -> Value {
    json!({
        "platform": platform,
        "check": check,
        "mandatory": mandatory,
        "status": status,
        "detail": detail,
    })
}

/// A verdict as a cell: passed when there are no problems.
fn verdict(platform: &str, check: &str, problems: &[String]) -> Value {
    let status = if problems.is_empty() {
        "passed"
    } else {
        "failed"
    };
    cell(platform, check, true, status, &problems.join("; "))
}

/// Holds the evidence under `dir` to the budget and writes `dir/matrix.json`.
fn gate(root: &Path, dir: &Path, revision: &str) -> Result<(), Vec<String>> {
    let tool = root.join(TOOL);
    let one = |e| vec![e];
    let budget: Budget = read_ron(&tool.join("budget.ron")).map_err(one)?;
    let plan: Plan = read_ron(&tool.join("scenarios.ron")).map_err(one)?;
    let hash = |name: &str| {
        fs::read(tool.join(name))
            .map(|bytes| sha256(&bytes))
            .map_err(|e| vec![format!("{name}: {e}")])
    };
    let pins = Pins {
        revision: revision.to_owned(),
        corpus: hash("corpus.ron")?,
        scenarios: hash("scenarios.ron")?,
    };
    let mut cells = Vec::new();
    for (platform, contract) in &budget.platforms {
        match contract {
            Contract::Blocked(who) => {
                cells.push(cell(platform, "capture", false, "unsupported", who));
            }
            Contract::Captured => {
                cells.extend(platform_cells(dir, platform, &plan, &budget, &pins));
            }
        }
    }
    let matrix = json!({
        "schema": SCHEMA,
        "revision": revision,
        "budget_version": budget.version,
        "corpus_sha256": pins.corpus,
        "scenarios_sha256": pins.scenarios,
        "cells": cells,
    });
    let text = serde_json::to_string_pretty(&matrix).unwrap_or_default();
    fs::create_dir_all(dir).map_err(|e| vec![format!("{}: {e}", dir.display())])?;
    fs::write(dir.join("matrix.json"), &text).map_err(|e| vec![format!("matrix.json: {e}")])?;
    let failures = failures(&text);
    if failures.is_empty() {
        Ok(())
    } else {
        Err(failures)
    }
}

/// What a run's evidence must match to count.
struct Pins {
    revision: String,
    corpus: String,
    scenarios: String,
}

/// Every mandatory cell that didn't pass, one line each.
fn failures(matrix: &str) -> Vec<String> {
    let Ok(matrix) = serde_json::from_str::<Value>(matrix) else {
        return vec!["the matrix isn't JSON".to_owned()];
    };
    if matrix["schema"].as_u64() != Some(SCHEMA) {
        return vec![format!("the matrix's schema isn't {SCHEMA}")];
    }
    let Some(cells) = matrix["cells"].as_array() else {
        return vec!["the matrix has no cells".to_owned()];
    };
    if cells.is_empty() {
        return vec!["the matrix has no cells".to_owned()];
    }
    cells
        .iter()
        .filter(|cell| cell["mandatory"] != json!(false) && cell["status"] != "passed")
        .map(|cell| {
            format!(
                "{} {}: {}: {}",
                cell["platform"].as_str().unwrap_or("?"),
                cell["check"].as_str().unwrap_or("?"),
                cell["status"].as_str().unwrap_or("?"),
                cell["detail"].as_str().unwrap_or(""),
            )
        })
        .collect()
}

/// Whether `platform` has passed real evidence in a matrix: a port's prerequisite.
fn port_ready(matrix: &str, platform: &str) -> Result<(), String> {
    let matrix: Value = serde_json::from_str(matrix).map_err(|e| format!("the matrix: {e}"))?;
    if matrix["schema"].as_u64() != Some(SCHEMA) {
        return Err(format!("the matrix's schema isn't {SCHEMA}"));
    }
    let cells: Vec<&Value> = matrix["cells"]
        .as_array()
        .map(|cells| cells.iter().filter(|c| c["platform"] == platform).collect())
        .unwrap_or_default();
    let real = |cell: &&&Value| cell["status"] == "passed" && cell["check"] != "capture";
    if cells.iter().filter(real).count() == 0 {
        return Err(format!("{platform} has no passed capture evidence"));
    }
    let unmet: Vec<_> = cells
        .iter()
        .filter(|c| c["mandatory"] != json!(false) && c["status"] != "passed")
        .collect();
    if unmet.is_empty() {
        Ok(())
    } else {
        Err(format!("{platform} has {} unmet cells", unmet.len()))
    }
}

/// One platform's cells: stale evidence refused, every scenario in every run,
/// the runs against each other, the faults, and the performance.
fn platform_cells(
    dir: &Path,
    platform: &str,
    plan: &Plan,
    budget: &Budget,
    pins: &Pins,
) -> Vec<Value> {
    let tolerance = &budget.correctness;
    let base = dir.join(platform);
    let runs = run_dirs(&base);
    if runs.len() < 2 {
        let why = format!(
            "{} runs under {}, and it takes two",
            runs.len(),
            base.display()
        );
        return vec![cell(platform, "capture", true, "failed", &why)];
    }
    let mut backend = String::new();
    let mut environment = String::new();
    for run in &runs {
        let meta = read_meta(run);
        match meta.and_then(|m| stale(&m, pins).map(|()| m)) {
            Ok(meta) => {
                backend = meta.backend;
                environment = meta.environment.to_string();
            }
            Err(e) => {
                let why = format!("{}: {e}", run.display());
                return vec![cell(platform, "capture", true, "failed", &why)];
            }
        }
    }
    let oracle = |file: &str| oracle_samples(dir, file);
    let detail = format!("{backend}: {environment}");
    let mut cells = vec![cell(platform, "capture", true, "passed", &detail)];
    for scenario in &plan.scenarios {
        let holds = scenario
            .backends
            .as_ref()
            .is_none_or(|names| names.contains(&backend));
        if !holds {
            let name = format!("scenario:{}", scenario.name);
            cells.push(cell(platform, &name, false, "unsupported", &backend));
            continue;
        }
        cells.extend(scenario_cells(
            platform, scenario, &runs, &oracle, tolerance,
        ));
    }
    cells.extend(fault_cells(platform, plan, &runs[0], &oracle, tolerance));
    cells.extend(performance_cells(platform, &base, budget));
    cells
}

/// A platform's numbered runs, in order.
fn run_dirs(base: &Path) -> Vec<PathBuf> {
    let mut runs: Vec<PathBuf> = fs::read_dir(base)
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("run-"))
        })
        .collect();
    runs.sort();
    runs
}

fn read_meta(run: &Path) -> Result<Meta, String> {
    let text = fs::read_to_string(run.join("meta.json")).map_err(|e| format!("meta.json: {e}"))?;
    serde_json::from_str(&text).map_err(|e| format!("meta.json: {e}"))
}

/// Why a run's evidence no longer counts, if it doesn't.
fn stale(meta: &Meta, pins: &Pins) -> Result<(), String> {
    if meta.revision != pins.revision {
        return Err(format!(
            "stale: recorded at {}, not {}",
            meta.revision, pins.revision
        ));
    }
    if meta.corpus_sha256 != pins.corpus {
        return Err("stale: recorded with another corpus".to_owned());
    }
    if meta.scenarios_sha256 != pins.scenarios {
        return Err("stale: recorded with other scenarios".to_owned());
    }
    Ok(())
}

/// `file`'s expected samples: what the separate decoder made of an Ogg file,
/// the numbers themselves of a WAV.
fn oracle_samples(dir: &Path, file: &str) -> Option<Vec<i16>> {
    if Path::new(file).extension().is_some_and(|e| e == "ogg") {
        let bytes = fs::read(dir.join("oracle").join(format!("{file}.pcm"))).ok()?;
        return Some(corpus::samples(&bytes));
    }
    let sources = corpus::sources();
    let (_, signal) = sources.iter().find(|(name, _)| *name == file)?;
    Some(signal.samples.clone())
}

/// A scenario's recording and log in `run`.
fn read_run(run: &Path, scenario: &str) -> Result<(Recording, Log), String> {
    let pcm = run.join(format!("{scenario}.pcm"));
    let bytes = fs::read(&pcm).map_err(|e| format!("missing capture {}: {e}", pcm.display()))?;
    let text = fs::read_to_string(run.join(format!("{scenario}.jsonl")))
        .map_err(|e| format!("missing log of {scenario}: {e}"))?;
    Ok((Recording::new(&corpus::samples(&bytes)), parse_log(&text)?))
}

/// The probe's lines as a [`Log`]. A line that isn't JSON is a broken run.
fn parse_log(text: &str) -> Result<Log, String> {
    let mut log = Log::default();
    for line in text.lines() {
        let event: Value =
            serde_json::from_str(line).map_err(|e| format!("a malformed log line: {e}: {line}"))?;
        let text = event["text"].as_str().unwrap_or_default().to_owned();
        match event["event"].as_str() {
            Some("step") => log.steps.push((
                u32::try_from(event["at"].as_u64().unwrap_or(0)).unwrap_or(0),
                event["cue"].as_str().unwrap_or_default().to_owned(),
                event["secs"].as_f64().unwrap_or(0.0),
            )),
            Some("warn") => log.warnings.push(text),
            Some("failed") => log.failed = Some(text),
            Some("done") => log.done = true,
            _ => {}
        }
    }
    Ok(log)
}

/// A scenario against its expected signals in each run, and the runs against
/// each other.
fn scenario_cells(
    platform: &str,
    scenario: &Scenario,
    runs: &[PathBuf],
    oracle: &dyn Fn(&str) -> Option<Vec<i16>>,
    tolerance: &Tolerance,
) -> Vec<Value> {
    let name = &scenario.name;
    let mut problems = Vec::new();
    let mut recordings = Vec::new();
    for run in runs {
        match read_run(run, name) {
            Ok((recording, log)) => {
                problems.extend(compare::check(
                    &scenario.expect,
                    scenario.end,
                    &recording,
                    &log,
                    oracle,
                    tolerance,
                ));
                recordings.push(recording);
            }
            Err(e) => problems.push(e),
        }
    }
    let mut cells = vec![verdict(platform, &format!("scenario:{name}"), &problems)];
    let silent = scenario
        .expect
        .iter()
        .any(|e| matches!(e, Expect::NoSignal));
    if !silent {
        cells.push(repeat_cell(platform, scenario, &recordings, tolerance));
    }
    cells
}

/// The first run against each other, away from where commands land.
fn repeat_cell(
    platform: &str,
    scenario: &Scenario,
    recordings: &[Recording],
    tolerance: &Tolerance,
) -> Value {
    let check = format!("repeat:{}", scenario.name);
    // Where a command lands, or a sound ends by itself: runs may differ by a
    // frame there, so a window across one says nothing.
    let mut edges: Vec<u32> = scenario.steps.iter().map(|step| step.at).collect();
    edges.extend(scenario.expect.iter().filter_map(|expect| match expect {
        Expect::Rise { at } | Expect::Fall { at } => Some(*at),
        _ => None,
    }));
    let [first, rest @ ..] = recordings else {
        return cell(platform, &check, true, "failed", "no recordings");
    };
    if rest.is_empty() {
        return cell(platform, &check, true, "failed", "one recording");
    }
    let mut worst = 0.0f64;
    for other in rest {
        match compare::repeat_difference(first, other, &edges, scenario.end, tolerance) {
            Ok(difference) => worst = worst.max(difference),
            Err(e) => return cell(platform, &check, true, "failed", &e),
        }
    }
    let detail = format!("{worst:.3} dB apart at worst");
    let status = if worst > tolerance.repeat_db {
        "failed"
    } else {
        "passed"
    };
    cell(platform, &check, true, status, &detail)
}

/// The faults: each must be caught, on a real recording of its scenario.
const FAULTS: [(&str, &str, Fault); 7] = [
    ("silent", "onset", Fault::Silence),
    ("channel", "onset", Fault::SwapChannels),
    ("rate", "wav_22k", Fault::WrongRate(1.06)),
    ("voice", "crossfade", Fault::Drop(2400, 2950)),
    ("loop", "loop", Fault::Drop(520, 1500)),
    ("clipping", "gain", Fault::Clip),
    ("marker", "onset", Fault::MissingMarker),
];

/// Whether the checks catch every fault in `run`'s recordings.
fn fault_cells(
    platform: &str,
    plan: &Plan,
    run: &Path,
    oracle: &dyn Fn(&str) -> Option<Vec<i16>>,
    tolerance: &Tolerance,
) -> Vec<Value> {
    FAULTS
        .iter()
        .map(|(name, scenario_name, fault)| {
            let check = format!("fault:{name}");
            let scenario = plan.scenarios.iter().find(|s| s.name == *scenario_name);
            let (Some(scenario), Ok((recording, log))) = (scenario, read_run(run, scenario_name))
            else {
                return cell(platform, &check, true, "failed", "no recording to break");
            };
            let judge = |recording: &Recording| {
                compare::check(
                    &scenario.expect,
                    scenario.end,
                    recording,
                    &log,
                    oracle,
                    tolerance,
                )
            };
            // Breaking what is already broken proves nothing.
            if let Some(already) = judge(&recording).first() {
                let why = format!("the recording to break isn't good: {already}");
                return cell(platform, &check, true, "failed", &why);
            }
            let caught = judge(&compare::inject(&recording, *fault, tolerance));
            if caught.is_empty() {
                cell(platform, &check, true, "failed", "the fault wasn't caught")
            } else {
                cell(platform, &check, true, "passed", &caught[0])
            }
        })
        .collect()
}

/// What a metric's paired ratios say.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Ratios {
    pub(crate) median: f64,
    pub(crate) spread: f64,
    pub(crate) pairs: usize,
}

/// The median and interquartile range of `after[i] / before[i]`.
pub(crate) fn ratios(before: &[f64], after: &[f64]) -> Ratios {
    let mut all: Vec<f64> = before
        .iter()
        .zip(after)
        .filter(|(a, _)| **a > 0.0)
        .map(|(a, b)| b / a)
        .collect();
    all.sort_by(f64::total_cmp);
    let at = |numerator: usize| {
        all.get((all.len().saturating_sub(1)) * numerator / 4)
            .copied()
    };
    Ratios {
        median: at(2).unwrap_or(0.0),
        spread: at(3).unwrap_or(0.0) - at(1).unwrap_or(0.0),
        pairs: all.len(),
    }
}

/// `passed`, `failed` or `inconclusive`, as a metric's ratios are to its limit.
pub(crate) fn judge(ratios: &Ratios, limit: Limit) -> &'static str {
    if ratios.pairs < MIN_PAIRS || ratios.spread > limit.max_spread {
        "inconclusive"
    } else if ratios.median > limit.max_ratio {
        "failed"
    } else {
        "passed"
    }
}

/// The performance verdicts from `base/perf.jsonl`: old against old, which
/// must pass, and old against a slowed old, which must not.
fn performance_cells(platform: &str, base: &Path, budget: &Budget) -> Vec<Value> {
    let path = base.join("perf.jsonl");
    let rows: Vec<Value> = fs::read_to_string(&path)
        .unwrap_or_default()
        .lines()
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect();
    let Some(limits) = budget.performance.get(platform) else {
        return vec![cell(platform, "perf", true, "failed", "no budget")];
    };
    if rows.is_empty() {
        let why = format!("no samples in {}", path.display());
        return vec![cell(platform, "perf", true, "failed", &why)];
    }
    let side = |name: &str, metric: &str| -> Vec<f64> {
        rows.iter()
            .filter(|row| row["side"] == name)
            .filter_map(|row| row["metrics"][metric].as_f64())
            .collect()
    };
    // A count, not a ratio: a gap in either side is a gap.
    let underruns: f64 = ["a", "b"]
        .iter()
        .flat_map(|name| side(name, "underruns"))
        .sum();
    let status = if underruns == 0.0 && !side("a", "underruns").is_empty() {
        "passed"
    } else {
        "failed"
    };
    let detail = format!("{underruns} gaps while everything played");
    let mut cells = vec![cell(platform, "perf:underruns", true, status, &detail)];
    for (metric, limit) in limits {
        let before = side("a", metric);
        let same = ratios(&before, &side("b", metric));
        let status = judge(&same, *limit);
        let detail = format!(
            "median ratio {:.3}, spread {:.3}, {} pairs, median {:.2}",
            same.median,
            same.spread,
            same.pairs,
            median(&before),
        );
        cells.push(cell(
            platform,
            &format!("perf:{metric}"),
            true,
            status,
            &detail,
        ));
        if !limit.slows {
            continue;
        }
        let burned = ratios(&before, &side("burn", metric));
        // Noise is `perf:`'s to judge: a slowdown only has to show above the limit.
        let caught = burned.pairs >= MIN_PAIRS && burned.median > limit.max_ratio;
        let detail = format!(
            "with the injected slowdown: median ratio {:.3}, spread {:.3}",
            burned.median, burned.spread
        );
        let status = if caught { "passed" } else { "failed" };
        cells.push(cell(
            platform,
            &format!("slowdown:{metric}"),
            true,
            status,
            &detail,
        ));
    }
    cells
}

fn median(values: &[f64]) -> f64 {
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    sorted.get(sorted.len() / 2).copied().unwrap_or(0.0)
}
