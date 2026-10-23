//! `autobalance`: the game's knobs change one of its stages until each of
//! its named bots (its entrants) succeeds as often as its checks' bands
//! want (`shuyi_core::autobalance_by`). The bots never change, only the
//! stage. The game gives the plan ([`GameTools::autobalance_plan`]): its
//! entrants and checks, as `playtest` has them, and the stage the search
//! may change, with its knobs ([`Tunable`](crate::game::Tunable)). This
//! runs the search, each setting's tries played on every core
//! ([`crate::pool`]) and judged once in seed order, so what it finds
//! never depends on the thread count. It
//! stops at the budget of evaluations or the time limit with the best
//! setting seen, says the changes in the knobs' words and, with `apply`,
//! has the game write the changed stage. The answer passes when every
//! rate is in its band.

use std::fmt::Write as _;
use std::time::{Duration, Instant};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use shuyi_core::bots::balance::{Change, distance};
use shuyi_core::bots::playtest::{PlaytestReport, Tried, Tries};
use shuyi_core::{BalanceSettings, Balanced, Stop};

use super::bots::{game_arg, options_text};
use super::codes::Code;
use super::playtest::{DEFAULT_TRIES, check_tries, checks_of, choose, played, seconds, threads};
use super::{Answer, Op, codes};
use crate::game::{GameArg, GameOption, GameTools, code_of};

/// The evaluations when the input doesn't say.
pub const DEFAULT_BUDGET: u32 = 100;

/// The most evaluations a search may make.
pub const MAX_BUDGET: u32 = 10_000;

/// What `autobalance` searches. A number left out is the default: the
/// game's for the tries ([`GameTools::playtest_defaults`]), the engine's
/// for the rest.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, default)]
pub struct AutobalanceInput {
    /// The stage, one of the game's; needed when it has several.
    pub stage: Option<String>,
    /// The entrants that play, by name or by a group's name; the game's
    /// default ones if empty.
    pub entrants: Vec<String>,
    /// How many tries of each entrant decide whether a stage is in its
    /// bands: the stage as authored and the one found play this many.
    pub tries: Option<u32>,
    /// The tries of each entrant in the search's own runs, from the same
    /// first seed: a fifth of `tries` (at least 1) if left out.
    pub quick: Option<u32>,
    /// The most evaluations (each a run of every entrant's tries), the
    /// stage as authored's among them: 100 if left out.
    pub budget: Option<u32>,
    /// Stop the search after this many seconds, with the best setting
    /// seen (whose run at `tries` still follows); no limit if left out.
    /// What it finds then depends on the machine.
    pub time: Option<u64>,
    /// The first try's seed; each next try takes the next.
    pub seed: Option<u64>,
    /// The turn a try stops at if it hasn't ended.
    pub turns: Option<u32>,
    /// The threads to play on; one a core if left out. What it finds never
    /// depends on it.
    pub threads: Option<u32>,
    /// Have the game write the changed stage to its files. This writes
    /// files of the game.
    pub apply: bool,
    /// The game's own options, in the order given.
    pub start: Vec<GameArg>,
}

impl AutobalanceInput {
    /// The tries it asks for, `defaults` filling in what it leaves out.
    pub fn tries(&self, defaults: &Tries) -> Tries {
        Tries::new(
            self.tries.unwrap_or(defaults.count),
            self.seed.unwrap_or(defaults.seed),
            self.turns.unwrap_or(defaults.turns),
        )
    }

    /// The search it asks for, `defaults` filling in the tries it leaves
    /// out.
    pub fn settings(&self, defaults: &Tries) -> BalanceSettings {
        let tries = self.tries(defaults);
        BalanceSettings::new(
            tries,
            self.quick.unwrap_or((tries.count / 5).max(1)),
            self.budget.unwrap_or(DEFAULT_BUDGET),
        )
    }
}

/// What `autobalance` found.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AutobalanceOutput {
    /// The stage, if the game names its stages.
    pub stage: Option<String>,
    /// The plan's title.
    pub title: String,
    /// The search's numbers, the defaults filled in.
    pub settings: BalanceSettings,
    /// Whether every rate of the stage found is in its band.
    pub in_bands: bool,
    /// Why the search stopped.
    pub stop: Stop,
    /// The evaluations it made.
    pub evaluations: u32,
    /// Every knob it moved, in the knobs' order, with the knob's words.
    pub changes: Vec<Change>,
    /// The stage as authored, at the full tries.
    pub before: PlaytestReport,
    /// The stage found, at the full tries (the stage as authored when
    /// nothing changed).
    pub after: PlaytestReport,
    /// With `apply`: the files the game wrote, none when nothing changed.
    pub written: Option<Vec<String>>,
    /// The search's wall time, in seconds: the one number that changes
    /// with the machine and the thread count.
    pub seconds: f64,
}

/// `autobalance`: `input`'s search of one of `game`'s stages, its tries
/// played on threads and judged.
pub fn autobalance(game: &dyn GameTools, input: &AutobalanceInput) -> Answer<AutobalanceOutput> {
    let failed = |code: Code, error: String| Answer::failed(Op::Autobalance, code, error);
    let settings = input.settings(&game.playtest_defaults());
    if let Err(e) = check_settings(&settings) {
        return failed(codes::WRONG_ARGUMENTS, e);
    }
    let threads = match threads(input.threads) {
        Ok(threads) => threads,
        Err(e) => return failed(codes::WRONG_ARGUMENTS, e),
    };
    let stage = match one_stage(&game.playtest_stages(), input.stage.as_deref()) {
        Ok(stage) => stage,
        Err(e) => return failed(codes::WRONG_ARGUMENTS, e),
    };
    let given = AutobalanceInput {
        tries: Some(settings.tries.count),
        quick: Some(settings.quick),
        budget: Some(settings.evaluations),
        seed: Some(settings.tries.seed),
        turns: Some(settings.tries.turns),
        ..input.clone()
    };
    let plan = match game.autobalance_plan(stage.as_deref(), &given) {
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
    let checks = match checks_of(&plan.checks, &plan.entrants, &chosen) {
        Ok(checks) => checks,
        Err(e) => return failed(codes::GAME_ERROR, e),
    };
    let started = Instant::now();
    let limit = input.time.map(Duration::from_secs);
    let mut should_stop = || limit.is_some_and(|limit| started.elapsed() >= limit);
    let mut evaluate = |one: &(dyn Fn(usize, u64, u32) -> Result<Tried, String> + Sync),
                        tries: &Tries| {
        played(&plan.entrants, &chosen, &checks, tries, threads, one).map(|p| p.report)
    };
    let searched = match plan
        .stage
        .search(&settings, &mut evaluate, &mut should_stop, input.apply)
    {
        Ok(searched) => searched,
        Err(e) => return failed(codes::GAME_ERROR, e),
    };
    let seconds = started.elapsed().as_secs_f64();
    let found = searched.balanced;
    let limits = limits(&found, input.time, settings.tries.turns);
    let (written, write_error) = match searched.written {
        None => (None, None),
        Some(Ok(files)) => (Some(files), None),
        Some(Err(e)) => (None, Some(e)),
    };
    let in_bands = distance(&found.after) == 0;
    let output = AutobalanceOutput {
        stage,
        title: plan.title,
        settings,
        in_bands,
        stop: found.stop,
        evaluations: found.evaluations,
        changes: found.changes,
        before: found.before,
        after: found.after,
        written,
        seconds,
    };
    let answer = Answer::new(
        Op::Autobalance,
        in_bands && write_error.is_none(),
        output,
        limits,
    );
    match write_error {
        Some(e) => {
            let mut answer = answer.with_code(codes::GAME_ERROR);
            answer.error = Some(format!("apply: the game couldn't write the stage: {e}"));
            answer
        }
        None => answer,
    }
}

/// What a search that ended as `found` could not determine: the setting
/// a time limit of `time` seconds cut short, and the tries of the stage
/// found that reached turn `turns` unfinished.
fn limits(found: &Balanced<()>, time: Option<u64>, turns: u32) -> Vec<String> {
    let mut limits = Vec::new();
    if let (Some(time), Stop::Stopped) = (time, found.stop) {
        limits.push(format!(
            "the time limit of {time} s ran out after {} evaluations: the best setting seen, \
             not what more time would find (it depends on the machine)",
            found.evaluations
        ));
    }
    let unfinished = found
        .after
        .entrants
        .iter()
        .flat_map(|e| &e.tries)
        .filter(|t| t.outcome.is_none())
        .count();
    if unfinished > 0 {
        limits.push(format!(
            "after: {unfinished} of the tries reached turn {turns} without an end: their \
             outcome is unknown"
        ));
    }
    limits
}

/// Whether `settings` can be searched: tries that can be played, quick
/// runs of 1 to that many, and 1 to [`MAX_BUDGET`] evaluations.
fn check_settings(settings: &BalanceSettings) -> Result<(), String> {
    check_tries(&settings.tries)?;
    let count = settings.tries.count;
    if settings.quick == 0 || settings.quick > count {
        return Err(format!(
            "quick: 1 to the tries ({count}), not {}",
            settings.quick
        ));
    }
    if settings.evaluations == 0 || settings.evaluations > MAX_BUDGET {
        return Err(format!(
            "budget: 1 to {MAX_BUDGET}, not {}",
            settings.evaluations
        ));
    }
    Ok(())
}

/// The stage `asked` for of the game's `stages` (`None`: the one unnamed
/// stage), or why it names none.
fn one_stage(stages: &[String], asked: Option<&str>) -> Result<Option<String>, String> {
    match (asked, stages) {
        (Some(stage), []) => Err(format!("stage: the game has no stages, not {stage}")),
        (Some(stage), _) if stages.iter().any(|s| s == stage) => Ok(Some(stage.to_owned())),
        (Some(stage), _) => Err(format!(
            "stage: no {stage}; the stages: {}",
            stages.join(", ")
        )),
        (None, []) => Ok(None),
        (None, [one]) => Ok(Some(one.clone())),
        (None, _) => Err(format!(
            "stage: the game has {} stages, name one: {}",
            stages.len(),
            stages.join(", ")
        )),
    }
}

/// `autobalance`'s own options, for its usage.
const OWN: [GameOption; 10] = [
    GameOption {
        flag: "--stage",
        value: Some("ID"),
        help: "the stage; needed when the game has several",
    },
    GameOption {
        flag: "--entrant",
        value: Some("NAME"),
        help: "an entrant or a group, again for more (the game's choice if none)",
    },
    GameOption {
        flag: "--tries",
        value: Some("N"),
        help: "the tries of each entrant that decide \"in band\", 1 to 10000",
    },
    GameOption {
        flag: "--quick",
        value: Some("N"),
        help: "the tries of the search's own runs (a fifth of --tries)",
    },
    GameOption {
        flag: "--budget",
        value: Some("N"),
        help: "the most evaluations, 1 to 10000 (100)",
    },
    GameOption {
        flag: "--time",
        value: Some("SECONDS"),
        help: "stop after that long with the best setting seen (no limit)",
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
        help: "the threads to play on (one a core; what it finds never changes)",
    },
    GameOption {
        flag: "--apply",
        value: None,
        help: "have the game write the changed stage to its files",
    },
];

/// `autobalance`'s usage, with the game's start `options`.
pub fn usage(command: &str, options: &[GameOption]) -> String {
    let mut out = format!(
        "usage: {command} autobalance [--stage ID] [--entrant NAME]... [--tries N] [--quick N]\n\
         \x20      [--budget N] [--time SECONDS] [--seed N] [--turns N] [--threads N] [--apply]\n\
         \x20      [options] [--json]\n\n\
         The game's knobs change its stage, never its bots, until each of its bots (its\n\
         entrants) succeeds as often as its checks' bands want: the smallest change the\n\
         search finds, in the knobs' words, with the reports before and after. Each\n\
         setting's tries run on every core. It stops at --budget evaluations or after\n\
         --time seconds with the best setting seen; --apply has the game write the\n\
         changed stage. The tries are the game's: {} from seed {} to turn {} unless it\n\
         says. It fails when a rate is still outside its band.\n\n",
        DEFAULT_TRIES.count, DEFAULT_TRIES.seed, DEFAULT_TRIES.turns
    );
    out.push_str(&options_text(&OWN));
    out.push_str(&options_text(options));
    out
}

/// Reads `autobalance`'s arguments (not `--json`).
pub fn parse_args(args: &[String], options: &[GameOption]) -> Result<AutobalanceInput, String> {
    let mut input = AutobalanceInput::default();
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
            "--entrant" => input.entrants.push(value()?),
            "--tries" => input.tries = Some(small(number(&value()?)?)?),
            "--quick" => input.quick = Some(small(number(&value()?)?)?),
            "--budget" => input.budget = Some(small(number(&value()?)?)?),
            "--time" => input.time = Some(number(&value()?)?),
            "--seed" => input.seed = Some(number(&value()?)?),
            "--turns" => input.turns = Some(small(number(&value()?)?)?),
            "--threads" => input.threads = Some(small(number(&value()?)?)?),
            "--apply" => input.apply = true,
            flag if flag.starts_with("--") => input.start.push(game_arg(flag, options, value)?),
            extra => return Err(format!("no <games> here, only flags: {extra}")),
        }
    }
    Ok(input)
}

/// What `autobalance` found, for people: the plan's title, why the search
/// stopped, the changes and the reports before and after, then what was
/// written and how long it took.
pub fn text(output: &AutobalanceOutput) -> String {
    let mut out = String::new();
    // Writing to a `String` can't fail.
    if !output.title.is_empty() {
        let _ = writeln!(out, "{}\n", output.title);
    }
    let found = Balanced {
        stage: (),
        changes: output.changes.clone(),
        before: output.before.clone(),
        after: output.after.clone(),
        stop: output.stop,
        evaluations: output.evaluations,
    };
    let _ = write!(out, "{found}");
    match output.written.as_deref() {
        None => {}
        Some([]) => out.push_str("\napply: nothing changed, nothing written\n"),
        Some(files) => {
            let _ = writeln!(out, "\napply: wrote {}", files.join(", "));
        }
    }
    let _ = writeln!(out, "\ntook {} s", seconds(output.seconds));
    out
}

#[cfg(test)]
mod tests;
