//! `playtest`: the game's named bots (its entrants) each play its stage K
//! times from seeds `seed`, `seed + 1`, …, the game measures every try,
//! and its checks say per bot whether it succeeded as often as wanted
//! (`shuyi_core::bots::playtest`). The game gives the plan
//! ([`GameTools::playtest_plan`]): its stages, its entrants and their
//! groups, its checks and how one try is played. This plays the tries on
//! every core ([`crate::pool`]), times them, and judges them once in seed
//! order, so the report never depends on the thread count. The answer
//! passes unless a rate fell under its floor.

use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::path::PathBuf;
use std::time::Duration;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use shuyi_core::bots::playtest::{PlaytestReport, Tried, Tries, judge};

use super::bots::{MAX_GAMES, game_arg, options_text};
use super::codes::Code;
use super::{Answer, Op, codes};
use crate::game::{GameArg, GameOption, GameTools, PlaytestPlan, code_of};
use crate::pool;

/// The tries when neither the input nor the game says: 100 from seed 0,
/// each stopped at turn 200.
pub const DEFAULT_TRIES: Tries = Tries {
    count: 100,
    seed: 0,
    turns: 200,
};

/// What `playtest` plays. A number left out is the game's default
/// ([`GameTools::playtest_defaults`]).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, default)]
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
}

impl PlaytestInput {
    /// The tries it asks for, `defaults` filling in what it leaves out.
    pub fn tries(&self, defaults: &Tries) -> Tries {
        Tries {
            count: self.tries.unwrap_or(defaults.count),
            seed: self.seed.unwrap_or(defaults.seed),
            turns: self.turns.unwrap_or(defaults.turns),
        }
    }
}

/// How one entrant's tries went, besides the report: its label and its
/// speed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Played {
    /// Its name.
    pub name: String,
    /// Its label in the game's plan (a mode), if any.
    pub label: Option<String>,
    /// The wall time of a try, on average, in seconds: the one number
    /// that changes with the machine and the thread count.
    pub seconds_per_try: f64,
}

/// What `playtest` found on one stage.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PlaytestOutput {
    /// The stage, if the game names its stages.
    pub stage: Option<String>,
    /// The plan's title.
    pub title: String,
    /// The tries and the checks' verdicts.
    pub report: PlaytestReport,
    /// Every entrant that played, in report order.
    pub entrants: Vec<Played>,
}

/// `playtest`: `input`'s tries of `game`'s plan, played on threads and
/// judged; a report a stage, one unless `every_stage`.
pub fn playtest(game: &dyn GameTools, input: &PlaytestInput) -> Answer<Vec<PlaytestOutput>> {
    let failed = |code: Code, error: String| Answer::failed(Op::Playtest, code, error);
    let tries = input.tries(&game.playtest_defaults());
    if tries.count == 0 || tries.count > MAX_GAMES {
        let error = format!("tries: 1 to {MAX_GAMES}, not {}", tries.count);
        return failed(codes::WRONG_ARGUMENTS, error);
    }
    if tries.seed.checked_add(u64::from(tries.count)).is_none() {
        let error = "seed: too big for that many tries".to_owned();
        return failed(codes::WRONG_ARGUMENTS, error);
    }
    let threads = match input.threads {
        Some(0) => return failed(codes::WRONG_ARGUMENTS, "threads: at least 1".to_owned()),
        Some(n) => usize::try_from(n).unwrap_or(usize::MAX),
        None => pool::cores(),
    };
    let stages = match stages(&game.playtest_stages(), input) {
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
        let chosen = match choose(&plan, &input.entrants) {
            Ok(chosen) => chosen,
            Err((code, e)) => return failed(code, e),
        };
        let output = match play(&plan, &chosen, &tries, threads) {
            Ok(report) => report,
            Err(e) => return failed(codes::GAME_ERROR, e),
        };
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
        });
    }
    let ok = outputs.iter().all(|o| !o.report.blocked);
    Answer::new(Op::Playtest, ok, outputs, limits)
}

/// The stages `input` plays of the game's `stages` (`None`: the one
/// unnamed stage), or why it names none.
fn stages(stages: &[String], input: &PlaytestInput) -> Result<Vec<Option<String>>, String> {
    let list = || stages.join(", ");
    match (&input.stage, input.every_stage) {
        (Some(_), true) => Err("stage or every stage, not both".to_owned()),
        (Some(stage), false) if stages.is_empty() => {
            Err(format!("stage: the game has no stages, not {stage}"))
        }
        (Some(stage), false) if stages.contains(stage) => Ok(vec![Some(stage.clone())]),
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

/// The indexes of the plan's entrants that `asked` names (by name or
/// group, in report order), or its default ones if `asked` is empty, or
/// every one if it has none. A name that is neither is the asker's error;
/// a group or default naming no entrant is the game's.
fn choose(plan: &PlaytestPlan, asked: &[String]) -> Result<Vec<usize>, (Code, String)> {
    let index = |name: &str| plan.entrants.iter().position(|(n, _)| n == name);
    let (names, code): (&[String], Code) = if asked.is_empty() {
        (&plan.default_entrants, codes::GAME_ERROR)
    } else {
        (asked, codes::WRONG_ARGUMENTS)
    };
    if names.is_empty() {
        return Ok((0..plan.entrants.len()).collect());
    }
    let mut chosen = BTreeSet::new();
    for name in names {
        if let Some(group) = plan.groups.get(name) {
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
                plan.entrants
                    .iter()
                    .map(|(n, _)| n.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            );
            if !plan.groups.is_empty() {
                let groups: Vec<&str> = plan.groups.keys().map(String::as_str).collect();
                let _ = write!(error, "; the groups: {}", groups.join(", "));
            }
            return Err((code, error));
        }
    }
    Ok(chosen.into_iter().collect())
}

/// One stage's report and speeds.
struct StagePlayed {
    report: PlaytestReport,
    entrants: Vec<Played>,
}

/// The `chosen` entrants of `plan` play `tries` on `threads` threads,
/// judged by the plan's checks over them.
fn play(
    plan: &PlaytestPlan,
    chosen: &[usize],
    tries: &Tries,
    threads: usize,
) -> Result<StagePlayed, String> {
    let names: Vec<&str> = chosen
        .iter()
        .map(|&e| plan.entrants[e].0.as_str())
        .collect();
    let checks = (plan.checks)(&names);
    // A check naming an entrant who doesn't play fails before a try is
    // played: judging no tries says so.
    let none = names
        .iter()
        .map(|n| ((*n).to_owned(), Vec::new()))
        .collect();
    judge(none, &checks).map_err(|e| format!("the game's checks: {e}"))?;
    let work: Vec<(usize, u64)> = chosen
        .iter()
        .flat_map(|&e| tries.seeds().map(move |seed| (e, seed)))
        .collect();
    let one = |&(e, seed): &(usize, u64)| {
        (plan.play)(e, seed, tries.turns)
            .map_err(|error| format!("{}: try {seed}: {error}", plan.entrants[e].0))
    };
    let done = pool::run(&work, threads, &one)?;
    let mut parts: Vec<(String, Vec<Tried>)> = names
        .iter()
        .map(|n| ((*n).to_owned(), Vec::new()))
        .collect();
    let mut took = vec![Duration::ZERO; chosen.len()];
    for ((e, _), (tried, elapsed)) in work.iter().zip(done) {
        let at = chosen.iter().position(|c| c == e).unwrap_or_default();
        parts[at].1.push(tried);
        took[at] += elapsed;
    }
    let report = judge(parts, &checks).map_err(|e| format!("the game's checks: {e}"))?;
    let entrants = chosen
        .iter()
        .zip(took)
        .map(|(&e, took)| Played {
            name: plan.entrants[e].0.clone(),
            label: plan.entrants[e].1.clone(),
            seconds_per_try: took.as_secs_f64() / f64::from(tries.count),
        })
        .collect();
    Ok(StagePlayed { report, entrants })
}

/// `playtest`'s own options, for its usage.
const OWN: [GameOption; 8] = [
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
];

/// `playtest`'s usage, with the game's start `options`.
pub fn usage(command: &str, options: &[GameOption]) -> String {
    let mut out = format!(
        "usage: {command} playtest [--stage ID | --every-stage] [--entrant NAME]... [--tries N]\n\
         \x20      [--seed N] [--turns N] [--threads N] [--report FILE] [options] [--json]\n\n\
         Each of the game's bots (its entrants) plays --tries tries, seeded N, N + 1, …,\n\
         each stopped at turn --turns, and the game's checks judge them. A number not\n\
         given is the game's: {} tries from seed {} to turn {} unless it says. It fails\n\
         only when a bot succeeds less often than a check's floor.\n\n",
        DEFAULT_TRIES.count, DEFAULT_TRIES.seed, DEFAULT_TRIES.turns
    );
    out.push_str(&options_text(&OWN));
    out.push_str(&options_text(options));
    out
}

/// What `playtest`'s arguments ask for: the input, and the file to write
/// the JSON answer to, if any.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PlaytestArgs {
    /// The input.
    pub input: PlaytestInput,
    /// `--report`'s file, from the project's folder.
    pub report: Option<PathBuf>,
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
            flag if flag.starts_with("--") => input.start.push(game_arg(flag, options, value)?),
            extra => return Err(format!("no <games> here, only flags: {extra}")),
        }
    }
    Ok(out)
}

/// A try's wall time for people: two decimals, or under a hundredth.
fn seconds(s: f64) -> String {
    if s < 0.01 {
        "< 0.01".to_owned()
    } else {
        format!("{s:.2}")
    }
}

/// What `playtest` found, for people: for each stage, the plan's title,
/// the report's table, then a line per entrant with its label and speed.
pub fn text(outputs: &[PlaytestOutput]) -> String {
    let stages: Vec<String> = outputs
        .iter()
        .map(|output| {
            let mut out = String::new();
            if !output.title.is_empty() {
                // Writing to a `String` can't fail.
                let _ = writeln!(out, "{}\n", output.title);
            }
            out.push_str(&output.report.to_string());
            out.push('\n');
            let width = output
                .entrants
                .iter()
                .map(|p| p.name.chars().count())
                .max()
                .unwrap_or(0);
            for played in &output.entrants {
                let _ = write!(out, "{:<width$} · ", played.name);
                if let Some(label) = &played.label {
                    let _ = write!(out, "{label} · ");
                }
                let _ = writeln!(out, "{} s per try", seconds(played.seconds_per_try));
            }
            out
        })
        .collect();
    stages.join("\n")
}

#[cfg(test)]
mod tests;
