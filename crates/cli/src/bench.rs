//! `bench [--target <id>] [--scenario <name>] [--out <dir>] [--no-instructions]`:
//! plays the game's benchmark scenarios headless and compares what each
//! costs with the budgets of the targets it ships on (ADR-0008 §6,
//! ADR-0081). Wall time is never measured: the numbers are proxies that
//! come out the same on every run.
//!
//! A **scenario** is a `*.bench.ron` file anywhere in the game's folder
//! (not under `target`, `dist` or a hidden folder): the game's start
//! options, the steps to play, frames to run after them, and the frames
//! to measure ([`Scenario`]). It is a test input of the tool, never an
//! asset: no format reads it and no build ships it.
//!
//! For each scenario, `bench` measures, per frame:
//!
//! - the **instructions** it took, counted by Valgrind's Callgrind on
//!   Linux, which dumps its count at the end of each measured frame
//!   ([`callgrind`]);
//! - the **heap's peak**, from the program's heap counter
//!   (`shuyi_alloc_count::global`), as `run --perf` reads it;
//!
//! and, for each target with a package budget, the **package's size**
//! from the `export-report.json` that `export` left under `dist/export`.
//! A web package is measured by what the page loads before the game's
//! first frame: the files of the export's folder outside `web.music` and
//! `web.voice`, which the game fetches when it first plays a track or a
//! line. The whole zip's size is reported beside it.
//!
//! The game's tool program is built in an optimized profile of its own,
//! `shuyi-bench`: the release profile with debug assertions on, so the
//! heap counter counts and the frames are measured (`DEBUG_TOOLS`), and
//! with its symbols kept, so Callgrind finds the frame marker. Each
//! scenario plays once for the targets with a colour display, and once
//! more on each one-bit target, on the console the game shows there and
//! with each frame converted as the device converts it
//! ([`GameTools::session_on`]).
//!
//! It writes `bench-report.json` and `bench-report.md` (the table for a
//! pull request) to `--out` (`dist/bench` by default), prints the table,
//! and fails (exit 1) when a scenario or a package is over a budget,
//! naming the scenario, the frame, the target and both numbers.
//!
//! `bench-play <file> [--target <id>] --out <file>` is its inner half,
//! run in the game's tool program: it plays one scenario and writes what
//! each frame recorded ([`Played`]).

mod callgrind;
mod report;
#[cfg(test)]
mod tests;

use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::{Deserialize, Serialize};
use shuyi_assets::target::{Color, Status};
use shuyi_ui::perf::PERF_FRAMES;

use crate::contract::codes::{self, Code};
use crate::contract::export::{ExportOutput, REPORT};
use crate::contract::session::{Session, measure_with_a_fake_clock, measured_start, process_heap};
use crate::contract::target;
use crate::frame_png::Script;
use crate::game::{GameArg, GameTools, check_start, code_of, is_unsupported};
use crate::project::{FILE, Project};

pub(crate) use report::Report;
use report::{Budget, FrameCounts, Package, Run};

/// The end of a scenario's file name, by which `bench` finds it.
pub(crate) const SUFFIX: &str = ".bench.ron";

/// The scenario format this `shuyi` reads.
pub(crate) const FORMAT_VERSION: u32 = 1;

/// Where the reports go without `--out`, from the game's folder.
const OUT: &str = "dist/bench";

/// Where `export` leaves its packages and reports, from the game's folder.
const EXPORTS: &str = "dist/export";

/// The cargo profile the game's tool program is measured in.
const PROFILE: &str = "shuyi-bench";

/// The folders a search for scenarios skips, beside hidden ones.
const SKIPPED: [&str; 3] = ["target", "dist", "node_modules"];

/// A benchmark scenario: what a `*.bench.ron` file holds.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Scenario {
    /// The file's format: [`FORMAT_VERSION`].
    pub format_version: u32,
    /// What it plays, for the report.
    #[serde(default)]
    pub about: String,
    /// The game's start options, as `run`'s `start` holds them: its
    /// stage, its seed.
    #[serde(default)]
    pub start: Vec<GameArg>,
    /// The steps to play, in `run`'s script form.
    #[serde(default)]
    pub script: Script,
    /// Frames to run after the script with no input.
    #[serde(default)]
    pub frames: u32,
    /// The frames measured.
    #[serde(default)]
    pub measure: Measure,
}

/// Which of a scenario's frames are measured, by their number: the first
/// frame the game runs is 1.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
pub(crate) enum Measure {
    /// Every frame the game ran.
    #[default]
    Every,
    /// The frames from `first` to `last`, both measured.
    Frames {
        /// The first.
        first: u32,
        /// The last.
        last: u32,
    },
}

impl Measure {
    /// Whether frame `frame` is measured.
    fn has(self, frame: u32) -> bool {
        match self {
            Measure::Every => true,
            Measure::Frames { first, last } => (first..=last).contains(&frame),
        }
    }
}

impl Scenario {
    /// Reads `text`, the scenario file shown as `file`; a file that doesn't
    /// read is an error naming the file, the line and the column.
    pub(crate) fn read(file: &str, text: &str) -> Result<Self, String> {
        let options = ron::Options::default()
            .with_default_extension(ron::extensions::Extensions::IMPLICIT_SOME);
        let scenario: Self = options.from_str(text).map_err(|e| format!("{file}:{e}"))?;
        if scenario.format_version != FORMAT_VERSION {
            return Err(format!(
                "{file}: format_version {}: this shuyi reads {FORMAT_VERSION}",
                scenario.format_version
            ));
        }
        scenario
            .script
            .check()
            .map_err(|e| format!("{file}: script: {e}"))?;
        if let Measure::Frames { first, last } = scenario.measure
            && (first == 0 || first > last)
        {
            return Err(format!(
                "{file}: measure: frames {first} to {last}: the first is 1 or more, and the last \
                 no less"
            ));
        }
        Ok(scenario)
    }
}

/// A scenario's name: its file's, without [`SUFFIX`].
pub(crate) fn name_of(file: &str) -> &str {
    let base = file.rsplit('/').next().unwrap_or(file);
    base.strip_suffix(SUFFIX).unwrap_or(base)
}

/// Every scenario file under `root`, from it with `/` between folders, in
/// order: not under [`SKIPPED`] folders or hidden ones.
pub(crate) fn find(root: &Path) -> Vec<String> {
    let mut found = Vec::new();
    let mut dirs = vec![PathBuf::new()];
    while let Some(dir) = dirs.pop() {
        let Ok(entries) = fs::read_dir(root.join(&dir)) else {
            continue;
        };
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            let path = dir.join(&name);
            if entry.file_type().is_ok_and(|t| t.is_dir()) {
                if !name.starts_with('.') && !SKIPPED.contains(&name.as_str()) {
                    dirs.push(path);
                }
            } else if name.ends_with(SUFFIX) {
                found.push(path.to_string_lossy().replace('\\', "/"));
            }
        }
    }
    found.sort();
    found
}

/// `bench`'s arguments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Options {
    /// The one target to gate, if given.
    pub target: Option<String>,
    /// The one scenario to play, by name, if given.
    pub scenario: Option<String>,
    /// Where the reports go, from the game's folder.
    pub out: String,
    /// Whether to count instructions (under Valgrind).
    pub instructions: bool,
}

/// The usage text, with `command` as the tool's name.
pub(crate) fn usage(command: &str) -> String {
    format!(
        "usage: {command} bench [--target <id>] [--scenario <name>] [--out <dir>] \
         [--no-instructions]\n\n\
         Plays each *.bench.ron scenario of the game headless and compares its\n\
         instructions per frame, its heap's peak and the package's size with the\n\
         budgets of each supported target the game lists (or of --target alone).\n\
         Instructions are counted under Valgrind (Linux); --no-instructions\n\
         measures the rest without it. The reports go in --out ({OUT} by\n\
         default): bench-report.json and bench-report.md."
    )
}

/// Parses `bench`'s arguments.
pub(crate) fn parse_args(args: &[String]) -> Result<Options, String> {
    let mut options = Options {
        target: None,
        scenario: None,
        out: OUT.to_owned(),
        instructions: true,
    };
    let mut rest = args.iter();
    while let Some(arg) = rest.next() {
        let mut value = || {
            rest.next()
                .cloned()
                .ok_or_else(|| format!("{arg} needs a value"))
        };
        match arg.as_str() {
            "--target" => options.target = Some(value()?),
            "--scenario" => options.scenario = Some(value()?),
            "--out" => options.out = value()?,
            "--no-instructions" => options.instructions = false,
            other => return Err(format!("unknown argument: {other}")),
        }
    }
    Ok(options)
}

/// A failure: its problem code and what went wrong.
type Failed = (Code, String);

/// `bench` on `project` with `args`: its exit code.
pub(crate) fn main(project: &Project, args: &[String]) -> u8 {
    let command = project.config.command();
    let fail = |(code, e): Failed| {
        eprint!("{}", codes::said("bench", code.0, &e, None));
        codes::exit_code(Some(code.0))
    };
    let options = match parse_args(args) {
        Ok(options) => options,
        Err(e) => {
            eprint!(
                "{}",
                codes::said("bench", codes::WRONG_ARGUMENTS.0, &e, Some(&usage(command)))
            );
            return 2;
        }
    };
    let plan = match plan(project, &options) {
        Ok(plan) => plan,
        Err(failed) => return fail(failed),
    };
    let player = match Built::new(project, options.instructions) {
        Ok(player) => player,
        Err(failed) => return fail(failed),
    };
    let out = project.root.join(&options.out);
    let report = match measure(&plan, &player, &out.join("runs"), &project.root, command) {
        Ok(report) => report,
        Err(failed) => return fail(failed),
    };
    let table = report::markdown(&report);
    if let Err(e) = write(&out, &report, &table) {
        return fail((codes::FILE, e));
    }
    println!("{table}");
    println!("bench: the reports are in {}", options.out);
    u8::from(!report.over.is_empty())
}

/// Writes `report` and its `table` into `out`.
fn write(out: &Path, report: &Report, table: &str) -> Result<(), String> {
    fs::create_dir_all(out).map_err(|e| format!("creating {}: {e}", out.display()))?;
    let json = serde_json::to_string_pretty(report).map_err(|e| e.to_string())?;
    for (name, text) in [
        ("bench-report.json", json),
        ("bench-report.md", table.to_owned()),
    ] {
        let path = out.join(name);
        fs::write(&path, text).map_err(|e| format!("writing {}: {e}", path.display()))?;
    }
    Ok(())
}

/// What `bench` plays and against which budgets.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Plan {
    /// The targets gated, with their budgets and whether their display is
    /// one-bit.
    pub targets: Vec<(Budget, bool)>,
    /// The scenarios: each file, from the game's folder, and what it says.
    pub scenarios: Vec<(String, Scenario)>,
    /// What the targets chosen leave out.
    pub limits: Vec<String>,
    /// The folders of a web package the game fetches when it first plays
    /// a file from them (`web.music`, `web.voice`), as package paths: not
    /// counted in its size.
    pub later: Vec<String>,
}

/// The plan of `options` on `project`: its targets' budgets and its
/// scenarios, read.
pub(crate) fn plan(project: &Project, options: &Options) -> Result<Plan, Failed> {
    let mut limits = Vec::new();
    let ids: Vec<String> = match &options.target {
        Some(id) => vec![id.clone()],
        None => project.config.targets.keys().cloned().collect(),
    };
    if ids.is_empty() {
        return Err((
            codes::PROJECT_FILE,
            format!("{FILE} lists no targets (`targets`): --target names the one to gate"),
        ));
    }
    let mut targets = Vec::new();
    for id in ids {
        let found = target::profile(&project.config, &project.root, &id)?.profile;
        if found.status != Status::Supported {
            if options.target.is_none() {
                limits.push(format!("{id} isn't supported yet: not gated"));
                continue;
            }
            limits.push(format!("{id} isn't supported yet: gated as asked"));
        }
        let budgets = &found.budgets;
        let budget = Budget {
            target: id,
            frame_instructions: budgets.frame_instructions,
            peak_heap_bytes: budgets.peak_heap_bytes,
            package_bytes: budgets.package_bytes,
        };
        targets.push((budget, found.display.color == Color::OneBit));
    }
    let mut files = find(&project.root);
    if let Some(name) = &options.scenario {
        files.retain(|file| name_of(file) == name);
        if files.is_empty() {
            return Err((
                codes::WRONG_ARGUMENTS,
                format!("no scenario {name}{SUFFIX} in the game's folder"),
            ));
        }
    }
    if files.is_empty() {
        return Err((
            codes::FILE,
            format!("no *{SUFFIX} scenario in the game's folder"),
        ));
    }
    let mut scenarios = Vec::new();
    for file in files {
        let text = fs::read_to_string(project.root.join(&file))
            .map_err(|e| (codes::FILE, format!("{file}: {e}")))?;
        let scenario = Scenario::read(&file, &text).map_err(|e| (codes::FILE, e))?;
        scenarios.push((file, scenario));
    }
    let later = project
        .config
        .web
        .iter()
        .flat_map(|web| [&web.music, &web.voice])
        .flatten()
        .map(|folder| crate::web::package_path(folder))
        .filter(|path| !path.is_empty())
        .collect();
    Ok(Plan {
        targets,
        scenarios,
        limits,
        later,
    })
}

/// What one frame recorded, as `bench-play` writes it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PlayedFrame {
    /// Its number: the first frame the game runs is 1.
    pub frame: u32,
    /// The heap's peak at its end, where the heap is counted.
    pub peak_heap_bytes: Option<u64>,
}

/// What `bench-play` writes: each frame the scenario recorded, in order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Played {
    /// The frames.
    pub frames: Vec<PlayedFrame>,
    /// Whether the program's heap counter read the heap at each frame's
    /// end (which is also Callgrind's frame marker).
    pub counts_heap: bool,
    /// What the run can't promise.
    pub limits: Vec<String>,
}

/// How a scenario is played and measured.
pub(crate) trait Player {
    /// Plays scenario `file` (from the game's folder), on the one-bit
    /// target `target` if given, with `dir` for its working files; its
    /// frames, each with what was counted of it, and the instructions
    /// before its first.
    fn play(&self, file: &str, target: Option<&str>, dir: &Path) -> Result<Measured, String>;
}

/// What a [`Player`] measured of one scenario.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Measured {
    /// Each frame, in order.
    pub frames: Vec<FrameCounts>,
    /// The instructions from the program's start to its first frame, if
    /// counted.
    pub start_instructions: Option<u64>,
    /// What the run can't promise.
    pub limits: Vec<String>,
}

/// The game's tool program built for `bench`.
struct Built {
    /// The program.
    exe: PathBuf,
    /// The arguments it takes before its command.
    args: Vec<String>,
    /// The game's folder, where it runs.
    root: PathBuf,
    /// Whether it runs under Callgrind.
    instructions: bool,
}

impl Built {
    /// The tool program of `project`, built in the [`PROFILE`]; first, with
    /// `instructions`, Valgrind is found.
    fn new(project: &Project, instructions: bool) -> Result<Self, Failed> {
        let Some(program) = &project.config.game else {
            return Err((
                codes::PROJECT_FILE,
                format!("{FILE} names no game tool program (`game`): bench plays the game"),
            ));
        };
        if instructions {
            callgrind::check().map_err(|e| (codes::TOOLCHAIN, e))?;
        }
        let mut args: Vec<String> = program.cargo_build_args();
        let profile = [
            "--profile".to_owned(),
            PROFILE.to_owned(),
            "--message-format=json-render-diagnostics".to_owned(),
        ];
        let config = [
            format!("profile.{PROFILE}.inherits=\"release\""),
            format!("profile.{PROFILE}.debug-assertions=true"),
            format!("profile.{PROFILE}.strip=false"),
        ];
        let config = config.into_iter().flat_map(|c| ["--config".to_owned(), c]);
        args.splice(1..1, profile.into_iter().chain(config));
        eprintln!("bench: building the game's tool program (profile {PROFILE})");
        let output = Command::new(crate::cargo())
            .args(&args)
            .current_dir(&project.root)
            .stderr(std::process::Stdio::inherit())
            .output()
            .map_err(|e| (codes::TOOLCHAIN, format!("cannot run cargo: {e}")))?;
        if !output.status.success() {
            return Err((
                codes::GAME_PROGRAM,
                format!("the game's tool program didn't build ({})", output.status),
            ));
        }
        let stdout = String::from_utf8_lossy(&output.stdout);
        let exe = executable(&stdout).ok_or_else(|| {
            (
                codes::GAME_PROGRAM,
                "cargo built no program to run".to_owned(),
            )
        })?;
        Ok(Self {
            exe,
            args: program.args.clone(),
            root: project.root.clone(),
            instructions,
        })
    }
}

/// The program cargo built, from its JSON messages `stdout`: the last
/// artifact with an executable.
pub(crate) fn executable(stdout: &str) -> Option<PathBuf> {
    stdout
        .lines()
        .rev()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .filter(|message| message["reason"] == "compiler-artifact")
        .find_map(|message| message["executable"].as_str().map(PathBuf::from))
}

impl Player for Built {
    fn play(&self, file: &str, target: Option<&str>, dir: &Path) -> Result<Measured, String> {
        let _ = fs::remove_dir_all(dir);
        fs::create_dir_all(dir).map_err(|e| format!("creating {}: {e}", dir.display()))?;
        let played_file = dir.join("played.json");
        let mut command = if self.instructions {
            let mut valgrind = Command::new(callgrind::VALGRIND);
            valgrind.args(callgrind::args(dir)).arg(&self.exe);
            valgrind
        } else {
            Command::new(&self.exe)
        };
        let mut play: Vec<OsString> = vec!["bench-play".into(), file.into()];
        if let Some(target) = target {
            play.extend(["--target".into(), target.into()]);
        }
        play.extend(["--out".into(), played_file.clone().into_os_string()]);
        let output = command
            .args(&self.args)
            .args(play)
            .current_dir(&self.root)
            .output()
            .map_err(|e| format!("{file}: cannot run the game's tool program: {e}"))?;
        if !output.status.success() {
            let said = String::from_utf8_lossy(&output.stderr);
            let tail: Vec<&str> = said.lines().rev().take(20).collect();
            let tail: Vec<&str> = tail.into_iter().rev().collect();
            return Err(format!(
                "{file}: the game's tool program failed ({}):\n{}",
                output.status,
                tail.join("\n")
            ));
        }
        let text = fs::read_to_string(&played_file)
            .map_err(|e| format!("{file}: reading {}: {e}", played_file.display()))?;
        let played: Played =
            serde_json::from_str(&text).map_err(|e| format!("{file}: what it played: {e}"))?;
        let counted = if self.instructions {
            if !played.counts_heap {
                return Err(format!(
                    "{file}: the game's tool program doesn't count the heap (a game with its \
                     own allocator, `shuyi_no_heap_count`): bench can't tell its frames apart \
                     to count their instructions; --no-instructions measures the rest"
                ));
            }
            Some(callgrind::read(dir).map_err(|e| format!("{file}: {e}"))?)
        } else {
            None
        };
        combine(file, played, counted)
    }
}

/// `played`'s frames, each with its instructions from `counted` (the
/// counts Callgrind dumped: the start's, then one per frame), if counted.
pub(crate) fn combine(
    file: &str,
    played: Played,
    counted: Option<Vec<u64>>,
) -> Result<Measured, String> {
    let (start, counts) = match counted {
        Some(counts) => {
            let Some((start, frames)) = counts.split_first() else {
                return Err(format!("{file}: Callgrind dumped no count"));
            };
            if frames.len() != played.frames.len() {
                return Err(format!(
                    "{file}: Callgrind counted {} frame(s), and the game recorded {}",
                    frames.len(),
                    played.frames.len()
                ));
            }
            (Some(*start), Some(frames.to_vec()))
        }
        None => (None, None),
    };
    let frames = played
        .frames
        .iter()
        .enumerate()
        .map(|(i, played)| FrameCounts {
            frame: played.frame,
            instructions: counts.as_ref().map(|counts| counts[i]),
            peak_heap_bytes: played.peak_heap_bytes,
        })
        .collect();
    Ok(Measured {
        frames,
        start_instructions: start,
        limits: played.limits,
    })
}

/// Plays `plan` with `player`, each run's working files in a folder of
/// `runs`, and measures the packages under the game's folder `root`
/// (`command` is the tool's name, for what a limit says to run).
pub(crate) fn measure(
    plan: &Plan,
    player: &dyn Player,
    runs: &Path,
    root: &Path,
    command: &str,
) -> Result<Report, Failed> {
    let colour: Vec<String> = plan
        .targets
        .iter()
        .filter(|(_, one_bit)| !one_bit)
        .map(|(budget, _)| budget.target.clone())
        .collect();
    let one_bit = plan
        .targets
        .iter()
        .filter(|(_, one_bit)| *one_bit)
        .map(|(budget, _)| budget.target.clone());
    let mut kinds: Vec<(Option<String>, Vec<String>)> = Vec::new();
    if !colour.is_empty() {
        kinds.push((None, colour));
    }
    kinds.extend(one_bit.map(|id| (Some(id.clone()), vec![id])));
    let mut out = Vec::new();
    for (file, scenario) in &plan.scenarios {
        let name = name_of(file);
        for (target, on) in &kinds {
            let dir = runs.join(format!("{name}-{}", target.as_deref().unwrap_or("colour")));
            eprintln!(
                "bench: {name} on {}",
                target
                    .as_deref()
                    .map_or_else(|| on.join(", "), str::to_owned)
            );
            let measured = player
                .play(file, target.as_deref(), &dir)
                .map_err(|e| (codes::GAME_ERROR, e))?;
            out.push(run(file, scenario, on, target.is_some(), measured)?);
        }
    }
    let budgets: Vec<Budget> = plan.targets.iter().map(|(b, _)| b.clone()).collect();
    let (packages, mut limits) = packages(root, &budgets, &plan.later, command);
    limits.splice(0..0, plan.limits.iter().cloned());
    let over = report::over(&budgets, &out, &packages);
    Ok(Report {
        format_version: FORMAT_VERSION,
        targets: budgets,
        runs: out,
        packages,
        over,
        limits,
    })
}

/// The run of `scenario` (from `file`) on targets `on`, measured as
/// `measured` says, its frames the scenario measures.
fn run(
    file: &str,
    scenario: &Scenario,
    on: &[String],
    one_bit: bool,
    measured: Measured,
) -> Result<Run, Failed> {
    let all = measured.frames.len();
    let (first, last) = (
        measured.frames.first().map_or(0, |f| f.frame),
        measured.frames.last().map_or(0, |f| f.frame),
    );
    let frames: Vec<FrameCounts> = measured
        .frames
        .into_iter()
        .filter(|f| scenario.measure.has(f.frame))
        .collect();
    if frames.is_empty() {
        let ran = if all == 0 {
            "none was recorded: the game didn't load, or quit at once".to_owned()
        } else {
            format!("it recorded frames {first} to {last}")
        };
        return Err((
            codes::GAME_ERROR,
            format!("{file}: no frame it measures was played: {ran}"),
        ));
    }
    Ok(Run {
        scenario: name_of(file).to_owned(),
        file: file.to_owned(),
        about: scenario.about.clone(),
        on: on.to_vec(),
        one_bit,
        start_instructions: measured.start_instructions,
        instructions: report::spread(&frames),
        peak_heap: report::peak(&frames),
        frames,
        limits: measured.limits,
    })
}

/// The packages under `root`'s [`EXPORTS`] whose profile is one of
/// `budgets`' with a package budget, and what isn't measured: a target
/// with such a budget and no package (`command` is the tool's name). A
/// web package counts what loads at start: its folder but `later`'s
/// folders ([`at_start`]).
fn packages(
    root: &Path,
    budgets: &[Budget],
    later: &[String],
    command: &str,
) -> (Vec<Package>, Vec<String>) {
    let mut found = Vec::new();
    let mut limits = Vec::new();
    let mut dirs: Vec<PathBuf> = fs::read_dir(root.join(EXPORTS))
        .map(|entries| entries.flatten().map(|e| e.path()).collect())
        .unwrap_or_default();
    dirs.sort();
    for dir in dirs {
        let path = dir.join(REPORT);
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        let Ok(report) = serde_json::from_str::<ExportOutput>(&text) else {
            limits.push(format!(
                "{}: doesn't read as an export report",
                path.display()
            ));
            continue;
        };
        let gated = budgets
            .iter()
            .any(|b| b.target == report.profile && b.package_bytes.is_some());
        if let (true, Some(package)) = (gated, report.sizes.first()) {
            let mut measured = Package {
                target: report.profile.clone(),
                export: report.target.clone(),
                file: package.file.clone(),
                bytes: package.bytes,
                whole_bytes: None,
            };
            if report.target == "web" {
                let folder = Some(&report.folder)
                    .filter(|f| !f.is_empty())
                    .map(|f| dir.join(f));
                match folder.map_or_else(
                    || Err("the report names no folder".to_owned()),
                    |f| at_start(&f, later),
                ) {
                    Ok(bytes) => {
                        measured.whole_bytes = Some(package.bytes);
                        measured.bytes = bytes;
                    }
                    Err(e) => limits.push(format!(
                        "{}: what loads at start isn't measured, the whole {} is: {e}",
                        report.profile, package.file
                    )),
                }
            }
            found.push(measured);
        }
    }
    for budget in budgets.iter().filter(|b| b.package_bytes.is_some()) {
        if !found.iter().any(|p| p.target == budget.target) {
            limits.push(format!(
                "{}: no package measured: no {EXPORTS}/*/{REPORT} is for it \
                 (`{command} export <target>` makes one)",
                budget.target
            ));
        }
    }
    (found, limits)
}

/// The bytes of the web build in `folder` that the page loads before the
/// game's first frame: every file but those under `later`'s folders,
/// which the game fetches when it first plays one.
fn at_start(folder: &Path, later: &[String]) -> Result<u64, String> {
    if !folder.is_dir() {
        return Err(format!("no folder {}", folder.display()));
    }
    let entries = crate::export::archive::entries(folder, "", &|_| false)?;
    let mut bytes = 0;
    for entry in entries {
        let fetched_later = later.iter().any(|dir| {
            entry
                .name
                .strip_prefix(dir.as_str())
                .is_some_and(|rest| rest.starts_with('/'))
        });
        if !fetched_later {
            let meta = fs::metadata(&entry.path)
                .map_err(|e| format!("stat {}: {e}", entry.path.display()))?;
            bytes += meta.len();
        }
    }
    Ok(bytes)
}

/// `bench-play`'s arguments: the scenario file, the one-bit target and
/// where its frames go.
fn parse_play(args: &[String]) -> Result<(String, Option<String>, PathBuf), String> {
    let (mut file, mut target, mut out) = (None, None, None);
    let mut rest = args.iter();
    while let Some(arg) = rest.next() {
        let mut value = || {
            rest.next()
                .cloned()
                .ok_or_else(|| format!("{arg} needs a value"))
        };
        match arg.as_str() {
            "--target" => target = Some(value()?),
            "--out" => out = Some(PathBuf::from(value()?)),
            flag if flag.starts_with("--") => return Err(format!("unknown argument: {flag}")),
            _ if file.is_none() => file = Some(arg.clone()),
            extra => return Err(format!("one scenario only, not also {extra}")),
        }
    }
    let file = file.ok_or("missing <file>")?;
    let out = out.ok_or("missing --out <file>")?;
    Ok((file, target, out))
}

/// `bench-play <file> [--target <id>] --out <file>` on `project` with the
/// game's tools `game`: its exit code.
pub(crate) fn play_main(project: &Project, args: &[String], game: &dyn GameTools) -> u8 {
    let (file, target, out) = match parse_play(args) {
        Ok(parsed) => parsed,
        Err(e) => {
            let usage = format!(
                "usage: {} bench-play <file> [--target <id>] --out <file>",
                project.config.command()
            );
            eprint!(
                "{}",
                codes::said("bench-play", codes::WRONG_ARGUMENTS.0, &e, Some(&usage))
            );
            return 2;
        }
    };
    // Frames are measured by their instructions, never their time: a real
    // clock's readings could change the count from run to run.
    measure_with_a_fake_clock();
    let played = play(&project.root, &file, target.as_deref(), game).and_then(|played| {
        let json = serde_json::to_string(&played).map_err(|e| (codes::FILE, e.to_string()))?;
        fs::write(&out, json).map_err(|e| (codes::FILE, format!("writing {}: {e}", out.display())))
    });
    match played {
        Ok(()) => 0,
        Err((code, e)) => {
            eprint!("{}", codes::said("bench-play", code.0, &e, None));
            codes::exit_code(Some(code.0))
        }
    }
}

/// Plays scenario `file` (from `root`) in `game`, on the one-bit target
/// `target` if given, measured: each frame it recorded.
pub(crate) fn play(
    root: &Path,
    file: &str,
    target: Option<&str>,
    game: &dyn GameTools,
) -> Result<Played, Failed> {
    let text =
        fs::read_to_string(root.join(file)).map_err(|e| (codes::FILE, format!("{file}: {e}")))?;
    let scenario = Scenario::read(file, &text).map_err(|e| (codes::FILE, e))?;
    check_start(game.frame_options(), &scenario.start)
        .map_err(|e| (codes::FILE, format!("{file}: start: {e}")))?;
    let mut limits = Vec::new();
    let start = || -> Result<Box<dyn Session>, String> {
        let Some(target) = target else {
            return game.session(&scenario.start);
        };
        match game.session_on(target, &scenario.start) {
            Err(e) if is_unsupported(&e) => {
                limits.push(format!(
                    "{e}: its frames on {target} weren't converted for its display, and it ran \
                     on the console of its colour targets"
                ));
                game.session(&scenario.start)
            }
            started => started,
        }
    };
    let mut session = measured_start(start).map_err(|e| (code_of(&e), e))?;
    session.set_perf(true);
    let counts_heap = session.counts_process_heap();
    // The start ends here: Callgrind dumps what it took apart from the
    // first frame's count.
    let _ = process_heap();
    session.play(&scenario.script);
    session.idle(scenario.frames);
    let read = session.perf(0, u32::try_from(PERF_FRAMES).unwrap_or(u32::MAX));
    if read.dropped > 0 {
        return Err((
            codes::FILE,
            format!(
                "{file}: it ran more than the {PERF_FRAMES} measured frames bench keeps: \
                 make it shorter"
            ),
        ));
    }
    let frames = read
        .records
        .iter()
        .map(|record| PlayedFrame {
            frame: record.frame,
            peak_heap_bytes: record.heap.map(|heap| heap.peak_bytes),
        })
        .collect();
    Ok(Played {
        frames,
        counts_heap,
        limits,
    })
}
