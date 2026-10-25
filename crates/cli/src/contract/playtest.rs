//! `playtest`: the game's named bots (its entrants) each play its stage K
//! times from seeds `seed`, `seed + 1`, …, the game measures every try,
//! and its checks say per bot whether it succeeded as often as wanted
//! (`shuyi_core::bots::playtest`). The game gives the plan
//! ([`GameTools::playtest_plan`]): its stages, its entrants and their
//! groups, its checks and how one try is played. This plays the tries on
//! every core ([`crate::pool`]), times them, and judges them once in seed
//! order, so the report never depends on the thread count. The answer
//! passes unless a rate fell under its floor.
//!
//! Each entrant's line carries the plan's summary items ([`Summary`]),
//! and [`kept`] keeps every run in a history and says what the last run
//! of the same stage, entrants and options found.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::time::Duration;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use shuyi_core::bots::playtest::{Check, Found, PlaytestReport, Tried, Tries, judge};

use super::bots::{MAX_GAMES, game_arg, options_text};
use super::codes::Code;
use super::{Answer, Op, codes};
use crate::game::{GameArg, GameOption, GameTools, PlaytestChecks, PlaytestPlan, code_of};
use crate::pool;

mod history;
mod summary;

pub use summary::Summary;

/// Where the history is kept when the input doesn't say, from the
/// project's folder.
pub const DEFAULT_HISTORY: &str = "target/playtest-history";

/// The tries when neither the input nor the game says: 100 from seed 0,
/// each stopped at turn 200.
pub const DEFAULT_TRIES: Tries = Tries::new(100, 0, 200);

/// What `playtest` plays. A number left out is the game's default
/// ([`GameTools::playtest_defaults`]).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, default)]
// An input: built with `default()` and its fields assigned, it may grow
// (ADR-0039).
#[non_exhaustive]
pub struct PlaytestInput {
    /// The stage, one of the game's; needed when it has several, unless
    /// `every_stage`.
    pub stage: Option<String>,
    /// Every stage of the game, in its order, a report each.
    pub every_stage: bool,
    /// The entrants that play, by name or by a group's name; the game's
    /// default ones if empty.
    pub entrants: Vec<String>,
    /// How many tries of each entrant.
    pub tries: Option<u32>,
    /// The first try's seed; each next try takes the next.
    pub seed: Option<u64>,
    /// The turn a try stops at if it hasn't ended.
    pub turns: Option<u32>,
    /// The threads to play on; one a core if left out. The report never
    /// depends on it.
    pub threads: Option<u32>,
    /// The game's own options, in the order given.
    pub start: Vec<GameArg>,
    /// The folder the history is kept in, from the project's
    /// ([`DEFAULT_HISTORY`] if left out); only [`kept`] keeps one.
    pub history: Option<String>,
    /// Keep no history, and compare with none.
    pub no_history: bool,
}

impl PlaytestInput {
    /// The tries it asks for, `defaults` filling in what it leaves out.
    pub fn tries(&self, defaults: &Tries) -> Tries {
        Tries::new(
            self.tries.unwrap_or(defaults.count),
            self.seed.unwrap_or(defaults.seed),
            self.turns.unwrap_or(defaults.turns),
        )
    }
}

/// How one entrant's tries went, besides the report: its label, its
/// speed, its summary and the last run's headline.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct Played {
    /// Its name.
    pub name: String,
    /// Its label in the game's plan (a mode), if any.
    pub label: Option<String>,
    /// The wall time of a try, on average, in seconds: the one number
    /// that changes with the machine and the thread count.
    pub seconds_per_try: f64,
    /// The plan's summary items over its tries, in order: `won 83/100`.
    #[serde(default)]
    pub summary: Vec<String>,
    /// Its headline number in the last run, if it played there:
    /// `was 47/100 succeed` from its first rate check, else its first
    /// summary item, `was won 63/100`.
    #[serde(default)]
    pub was: Option<String>,
}

/// The last run before this one, from the history.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct Previous {
    /// When it ran: `2026-10-22 14:05 UTC`.
    pub date: String,
    /// The short hash of the commit the project was on, or `unknown`.
    pub commit: String,
}

/// What `playtest` found on one stage.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct PlaytestOutput {
    /// The stage, if the game names its stages.
    pub stage: Option<String>,
    /// The plan's title.
    pub title: String,
    /// The tries and the checks' verdicts.
    pub report: PlaytestReport,
    /// Every entrant that played, in report order.
    pub entrants: Vec<Played>,
    /// The last run of the same stage, entrants and game options, if the
    /// history has one.
    #[serde(default)]
    pub previous: Option<Previous>,
}

/// `playtest`: `input`'s tries of `game`'s plan, played on threads and
/// judged; a report a stage, one unless `every_stage`. It keeps no
/// history: [`kept`] does.
pub fn playtest(game: &dyn GameTools, input: &PlaytestInput) -> Answer<Vec<PlaytestOutput>> {
    let failed = |code: Code, error: String| Answer::failed(Op::Playtest, code, error);
    let tries = input.tries(&game.playtest_defaults());
    if let Err(e) = check_tries(&tries) {
        return failed(codes::WRONG_ARGUMENTS, e);
    }
    let threads = match threads(input.threads) {
        Ok(threads) => threads,
        Err(e) => return failed(codes::WRONG_ARGUMENTS, e),
    };
    let stages = match stages(
        &game.playtest_stages(),
        input.stage.as_deref(),
        input.every_stage,
    ) {
        Ok(stages) => stages,
        Err(e) => return failed(codes::WRONG_ARGUMENTS, e),
    };
    let given = PlaytestInput {
        tries: Some(tries.count),
        seed: Some(tries.seed),
        turns: Some(tries.turns),
        ..input.clone()
    };
    let mut outputs = Vec::new();
    let mut limits = Vec::new();
    for stage in stages {
        let plan = match game.playtest_plan(stage.as_deref(), &given) {
            Ok(plan) => plan,
            Err(e) => return failed(code_of(&e), e),
        };
        let chosen = match choose(
            &plan.entrants,
            &plan.groups,
            &plan.default_entrants,
            &input.entrants,
        ) {
            Ok(chosen) => chosen,
            Err((code, e)) => return failed(code, e),
        };
        let mut output = match play(&plan, &chosen, &tries, threads) {
            Ok(report) => report,
            Err(e) => return failed(codes::GAME_ERROR, e),
        };
        summarize(&plan, &mut output);
        let unfinished = output
            .report
            .entrants
            .iter()
            .flat_map(|e| &e.tries)
            .filter(|t| t.outcome.is_none())
            .count();
        if unfinished > 0 {
            let on = stage.as_ref().map(|s| format!("{s}: ")).unwrap_or_default();
            limits.push(format!(
                "{on}{unfinished} of the tries reached turn {} without an end: their outcome is unknown",
                tries.turns
            ));
        }
        outputs.push(PlaytestOutput {
            stage,
            title: plan.title,
            report: output.report,
            entrants: output.entrants,
            previous: None,
        });
    }
    let ok = outputs.iter().all(|o| !o.report.blocked);
    Answer::new(Op::Playtest, ok, outputs, limits)
}

/// Each entrant's summary items of `plan` over its tries.
fn summarize(plan: &PlaytestPlan, stage: &mut StagePlayed) {
    for (played, tries) in stage.entrants.iter_mut().zip(&stage.report.entrants) {
        let at = plan.entrants.iter().position(|(n, _)| *n == played.name);
        let items = at.and_then(|at| plan.summary.get(at));
        played.summary = items
            .into_iter()
            .flatten()
            .map(|item| item.text(&tries.tries))
            .collect();
    }
}

/// [`playtest`], each stage's run kept in the history under `root`
/// (unless `no_history`) and compared with the last one kept there: each
/// entrant's [`was`](Played::was) and the stage's
/// [`previous`](PlaytestOutput::previous). A history that can't be
/// written fails the answer, naming the file.
pub fn kept(
    game: &dyn GameTools,
    root: &Path,
    input: &PlaytestInput,
) -> Answer<Vec<PlaytestOutput>> {
    let mut answer = playtest(game, input);
    if input.no_history {
        return answer;
    }
    let Some(outputs) = answer.result.as_mut() else {
        return answer;
    };
    let dir = root.join(input.history.as_deref().unwrap_or(DEFAULT_HISTORY));
    let date = history::utc_stamp(history::now());
    let commit = history::git_commit(root);
    let options: Vec<String> = input.start.iter().map(history::option).collect();
    let mut unwritten = None;
    for output in outputs.iter_mut() {
        let names: Vec<String> = output.entrants.iter().map(|p| p.name.clone()).collect();
        let file = dir.join(history::file_name(
            output.stage.as_deref(),
            &names,
            &options,
        ));
        if let Some(last) = history::last(&file) {
            compare(output, &last);
        }
        let record = history::Record {
            date: date.clone(),
            commit: commit.clone(),
            title: output.title.clone(),
            stage: output.stage.clone(),
            entrants: names,
            options: options.clone(),
            output: output.clone(),
        };
        if let Err(e) = history::append(&file, &record) {
            unwritten.get_or_insert(format!("history {}: {e}", file.display()));
        }
    }
    if let Some(error) = unwritten {
        answer.ok = false;
        answer.error = Some(error);
        answer = answer.with_code(codes::FILE);
    }
    answer
}

/// `output` against the `last` run kept: its date and commit, and each
/// entrant's headline there.
fn compare(output: &mut PlaytestOutput, last: &history::Record) {
    output.previous = Some(Previous {
        date: last.date.clone(),
        commit: last.commit.clone(),
    });
    let was: Vec<Option<String>> = output
        .entrants
        .iter()
        .map(|p| was(output, &last.output, &p.name))
        .collect();
    for (played, was) in output.entrants.iter_mut().zip(was) {
        played.was = was;
    }
}

/// `name`'s headline in `previous`, if it played there: its first rate
/// check's successes if it has a rate check `now`, else its first summary
/// item.
fn was(now: &PlaytestOutput, previous: &PlaytestOutput, name: &str) -> Option<String> {
    let played = previous.entrants.iter().find(|p| p.name == name)?;
    let rate = |output: &PlaytestOutput| {
        output
            .report
            .checks
            .iter()
            .find_map(|j| match (&j.check, &j.found) {
                (
                    Check::Rate { entrant, .. },
                    Found::Rate {
                        successes, count, ..
                    },
                ) if entrant == name => Some((*successes, *count)),
                _ => None,
            })
    };
    if rate(now).is_some() {
        return rate(previous).map(|(s, n)| format!("was {s}/{n} succeed"));
    }
    played.summary.first().map(|item| format!("was {item}"))
}

/// Whether `tries` can be played: 1 to [`MAX_GAMES`] of them, each seed
/// a `u64`.
pub(crate) fn check_tries(tries: &Tries) -> Result<(), String> {
    if tries.count == 0 || tries.count > MAX_GAMES {
        return Err(format!("tries: 1 to {MAX_GAMES}, not {}", tries.count));
    }
    if tries.seed.checked_add(u64::from(tries.count)).is_none() {
        return Err("seed: too big for that many tries".to_owned());
    }
    Ok(())
}

/// The threads `asked` for, or one a core if `None`.
pub(crate) fn threads(asked: Option<u32>) -> Result<usize, String> {
    match asked {
        Some(0) => Err("threads: at least 1".to_owned()),
        Some(n) => Ok(usize::try_from(n).unwrap_or(usize::MAX)),
        None => Ok(pool::cores()),
    }
}

/// The stages to play of the game's `stages` (`None`: the one unnamed
/// stage): the one named `stage`, or each if `every_stage`; or why it
/// names none.
pub(crate) fn stages(
    stages: &[String],
    stage: Option<&str>,
    every_stage: bool,
) -> Result<Vec<Option<String>>, String> {
    let list = || stages.join(", ");
    match (stage, every_stage) {
        (Some(_), true) => Err("stage or every stage, not both".to_owned()),
        (Some(stage), false) if stages.is_empty() => {
            Err(format!("stage: the game has no stages, not {stage}"))
        }
        (Some(stage), false) if stages.iter().any(|s| s == stage) => {
            Ok(vec![Some(stage.to_owned())])
        }
        (Some(stage), false) => Err(format!("stage: no {stage}; the stages: {}", list())),
        (None, _) if stages.is_empty() => Ok(vec![None]),
        (None, false) if stages.len() > 1 => Err(format!(
            "stage: the game has {} stages, name one or play every stage: {}",
            stages.len(),
            list()
        )),
        (None, _) => Ok(stages.iter().cloned().map(Some).collect()),
    }
}

/// The indexes of a plan's `entrants` that `asked` names (by name or by
/// one of its `groups`, in report order), or its `defaults` if `asked` is
/// empty, or every one if it has none. A name that is neither is the
/// asker's error; a group or default naming no entrant is the game's.
pub(crate) fn choose(
    entrants: &[(String, Option<String>)],
    groups: &BTreeMap<String, Vec<String>>,
    defaults: &[String],
    asked: &[String],
) -> Result<Vec<usize>, (Code, String)> {
    let index = |name: &str| entrants.iter().position(|(n, _)| n == name);
    let (names, code): (&[String], Code) = if asked.is_empty() {
        (defaults, codes::GAME_ERROR)
    } else {
        (asked, codes::WRONG_ARGUMENTS)
    };
    if names.is_empty() {
        return Ok((0..entrants.len()).collect());
    }
    let mut chosen = BTreeSet::new();
    for name in names {
        if let Some(group) = groups.get(name) {
            for member in group {
                let e = index(member).ok_or_else(|| {
                    let error = format!("the group {name} names {member}, who isn't an entrant");
                    (codes::GAME_ERROR, error)
                })?;
                chosen.insert(e);
            }
        } else if let Some(e) = index(name) {
            chosen.insert(e);
        } else {
            let mut error = format!(
                "entrant: no {name}; the entrants: {}",
                entrants
                    .iter()
                    .map(|(n, _)| n.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            );
            if !groups.is_empty() {
                let groups: Vec<&str> = groups.keys().map(String::as_str).collect();
                let _ = write!(error, "; the groups: {}", groups.join(", "));
            }
            return Err((code, error));
        }
    }
    Ok(chosen.into_iter().collect())
}

/// One stage's report and speeds.
pub(crate) struct StagePlayed {
    /// The tries, judged.
    pub(crate) report: PlaytestReport,
    /// Each entrant's speed.
    pub(crate) entrants: Vec<Played>,
}

/// The `chosen` entrants of `plan` play `tries` on `threads` threads,
/// judged by the plan's checks over them.
fn play(
    plan: &PlaytestPlan,
    chosen: &[usize],
    tries: &Tries,
    threads: usize,
) -> Result<StagePlayed, String> {
    let checks = checks_of(&plan.checks, &plan.entrants, chosen)?;
    played(&plan.entrants, chosen, &checks, tries, threads, &*plan.play)
}

/// A plan's `checks` over the `chosen` of its `entrants`, or why they
/// can't judge them: a check names an entrant who doesn't play.
pub(crate) fn checks_of(
    checks: &PlaytestChecks,
    entrants: &[(String, Option<String>)],
    chosen: &[usize],
) -> Result<Vec<Check>, String> {
    let names: Vec<&str> = chosen.iter().map(|&e| entrants[e].0.as_str()).collect();
    let checks = checks(&names);
    // Judging no tries says so, before a try is played.
    let none = names
        .iter()
        .map(|n| ((*n).to_owned(), Vec::new()))
        .collect();
    judge(none, &checks).map_err(|e| format!("the game's checks: {e}"))?;
    Ok(checks)
}

/// The `chosen` of a plan's `entrants` play `tries` on `threads` threads,
/// each try by `play` (entrant index, seed, the turn it stops at), timed,
/// and `checks` judge them once in seed order.
pub(crate) fn played(
    entrants: &[(String, Option<String>)],
    chosen: &[usize],
    checks: &[Check],
    tries: &Tries,
    threads: usize,
    play: &(dyn Fn(usize, u64, u32) -> Result<Tried, String> + Sync),
) -> Result<StagePlayed, String> {
    let work: Vec<(usize, u64)> = chosen
        .iter()
        .flat_map(|&e| tries.seeds().map(move |seed| (e, seed)))
        .collect();
    let one = |&(e, seed): &(usize, u64)| {
        play(e, seed, tries.turns)
            .map_err(|error| format!("{}: try {seed}: {error}", entrants[e].0))
    };
    let done = pool::run(&work, threads, &one)?;
    let mut parts: Vec<(String, Vec<Tried>)> = chosen
        .iter()
        .map(|&e| (entrants[e].0.clone(), Vec::new()))
        .collect();
    let mut took = vec![Duration::ZERO; chosen.len()];
    for ((e, _), (tried, elapsed)) in work.iter().zip(done) {
        let at = chosen.iter().position(|c| c == e).unwrap_or_default();
        parts[at].1.push(tried);
        took[at] += elapsed;
    }
    let report = judge(parts, checks).map_err(|e| format!("the game's checks: {e}"))?;
    let entrants = chosen
        .iter()
        .zip(took)
        .map(|(&e, took)| Played {
            name: entrants[e].0.clone(),
            label: entrants[e].1.clone(),
            seconds_per_try: took.as_secs_f64() / f64::from(tries.count),
            summary: Vec::new(),
            was: None,
        })
        .collect();
    Ok(StagePlayed { report, entrants })
}

/// `playtest`'s own options, for its usage.
const OWN: [GameOption; 11] = [
    GameOption {
        flag: "--stage",
        value: Some("ID"),
        help: "the stage; needed when the game has several",
    },
    GameOption {
        flag: "--every-stage",
        value: None,
        help: "every stage, in order, a report each",
    },
    GameOption {
        flag: "--entrant",
        value: Some("NAME"),
        help: "an entrant or a group, again for more (the game's choice if none)",
    },
    GameOption {
        flag: "--tries",
        value: Some("N"),
        help: "the tries of each entrant, 1 to 10000",
    },
    GameOption {
        flag: "--seed",
        value: Some("N"),
        help: "the first try's seed; each next try takes the next",
    },
    GameOption {
        flag: "--turns",
        value: Some("N"),
        help: "the turn a try stops at if it hasn't ended",
    },
    GameOption {
        flag: "--threads",
        value: Some("N"),
        help: "the threads to play on (one a core; the report never changes)",
    },
    GameOption {
        flag: "--report",
        value: Some("FILE"),
        help: "also write the JSON answer to FILE",
    },
    GameOption {
        flag: "--history",
        value: Some("DIR"),
        help: "keep the history in DIR (target/playtest-history)",
    },
    GameOption {
        flag: "--no-history",
        value: None,
        help: "keep no history, and compare with none",
    },
    GameOption {
        flag: "--brief",
        value: None,
        help: "leave out the line per try",
    },
];

/// `playtest`'s usage, with the game's start `options`.
pub fn usage(command: &str, options: &[GameOption]) -> String {
    let mut out = format!(
        "usage: {command} playtest [--stage ID | --every-stage] [--entrant NAME]... [--tries N]\n\
         \x20      [--seed N] [--turns N] [--threads N] [--report FILE]\n\
         \x20      [--history DIR | --no-history] [--brief] [options] [--json]\n\n\
         Each of the game's bots (its entrants) plays --tries tries, seeded N, N + 1, …,\n\
         each stopped at turn --turns, and the game's checks judge them. A number not\n\
         given is the game's: {} tries from seed {} to turn {} unless it says. It fails\n\
         only when a bot succeeds less often than a check's floor. Each run is kept in\n\
         the history, a file per stage, entrants and options, and each bot's line says\n\
         what the last run there found (was …).\n\n",
        DEFAULT_TRIES.count, DEFAULT_TRIES.seed, DEFAULT_TRIES.turns
    );
    out.push_str(&options_text(&OWN));
    out.push_str(&options_text(options));
    out
}

/// What `playtest`'s arguments ask for: the input, the file to write the
/// JSON answer to, if any, and whether the text is brief.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct PlaytestArgs {
    /// The input.
    pub input: PlaytestInput,
    /// `--report`'s file, from the project's folder.
    pub report: Option<PathBuf>,
    /// `--brief`: the text without the line per try ([`brief`]).
    pub brief: bool,
}

/// Reads `playtest`'s arguments (not `--json`).
pub fn parse_args(args: &[String], options: &[GameOption]) -> Result<PlaytestArgs, String> {
    let mut out = PlaytestArgs::default();
    let input = &mut out.input;
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        let mut value = || {
            iter.next()
                .ok_or_else(|| format!("{arg} needs a value"))
                .cloned()
        };
        let number = |text: &str| {
            text.parse::<u64>()
                .map_err(|_| format!("{arg}: not a whole number: {text}"))
        };
        let small = |n: u64| u32::try_from(n).map_err(|_| format!("{arg}: too big"));
        match arg.as_str() {
            "--stage" => input.stage = Some(value()?),
            "--every-stage" => input.every_stage = true,
            "--entrant" => input.entrants.push(value()?),
            "--seed" => input.seed = Some(number(&value()?)?),
            "--tries" => input.tries = Some(small(number(&value()?)?)?),
            "--turns" => input.turns = Some(small(number(&value()?)?)?),
            "--threads" => input.threads = Some(small(number(&value()?)?)?),
            "--report" => out.report = Some(PathBuf::from(value()?)),
            "--history" => input.history = Some(value()?),
            "--no-history" => input.no_history = true,
            "--brief" => out.brief = true,
            flag if flag.starts_with("--") => input.start.push(game_arg(flag, options, value)?),
            extra => return Err(format!("no <games> here, only flags: {extra}")),
        }
    }
    Ok(out)
}

/// A try's wall time for people: two decimals, or under a hundredth.
pub(crate) fn seconds(s: f64) -> String {
    if s < 0.01 {
        "< 0.01".to_owned()
    } else {
        format!("{s:.2}")
    }
}

/// The width an entrant's line wraps past.
const LINE: usize = 100;

/// What `playtest` found, for people: for each stage, the plan's title,
/// the report's table, then a line per entrant with its label, its
/// summary, its speed and what it was in the last run, and that run's
/// date and commit.
pub fn text(outputs: &[PlaytestOutput]) -> String {
    render(outputs, false)
}

/// [`text`] without the table's line per try (`--brief`).
pub fn brief(outputs: &[PlaytestOutput]) -> String {
    render(outputs, true)
}

/// [`text`], or [`brief`] if `brief`.
fn render(outputs: &[PlaytestOutput], brief: bool) -> String {
    let stages: Vec<String> = outputs
        .iter()
        .map(|output| {
            let mut out = String::new();
            if !output.title.is_empty() {
                // Writing to a `String` can't fail.
                let _ = writeln!(out, "{}\n", output.title);
            }
            if brief {
                out.push_str(&output.report.brief());
            } else {
                out.push_str(&output.report.to_string());
            }
            out.push('\n');
            let width = output
                .entrants
                .iter()
                .map(|p| p.name.chars().count())
                .max()
                .unwrap_or(0);
            for played in &output.entrants {
                out.push_str(&entrant_line(played, width));
            }
            if let Some(previous) = &output.previous {
                let _ = writeln!(
                    out,
                    "previous {} · commit {}",
                    previous.date, previous.commit
                );
            }
            out
        })
        .collect();
    stages.join("\n")
}

/// `played`'s line, its name padded to `width`: its label, its summary
/// items, its speed and what it was, ` · ` between them, wrapped past
/// [`LINE`] characters onto lines indented under its first item.
fn entrant_line(played: &Played, width: usize) -> String {
    let mut head = format!("{:<width$} · ", played.name);
    if let Some(label) = &played.label {
        let _ = write!(head, "{label} · ");
    }
    let speed = format!("{} s per try", seconds(played.seconds_per_try));
    let items = played
        .summary
        .iter()
        .cloned()
        .chain([speed])
        .chain(played.was.clone());
    let indent = " ".repeat(head.chars().count());
    let mut out = String::new();
    let mut line = head;
    let mut first = true;
    for item in items {
        if first {
            line.push_str(&item);
            first = false;
        } else if line.chars().count() + 3 + item.chars().count() > LINE {
            out.push_str(&line);
            out.push('\n');
            line = format!("{indent}{item}");
        } else {
            let _ = write!(line, " · {item}");
        }
    }
    out.push_str(&line);
    out.push('\n');
    out
}

#[cfg(test)]
mod tests;
