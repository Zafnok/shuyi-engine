//! `replay`: a record of a game re-run by the game's own rules, with a
//! state hash per step ([`shuyi_core::state_hash`]). One record is run
//! twice, which finds luck or state that a run doesn't own; two records
//! are run side by side. The answer says the first step where the two
//! runs' hashes differ, and passes if there is none.
//!
//! What a record is belongs to the game (for a turn game: its seed and
//! its commands, ADR-0003 §2). [`turns`] re-runs one over any
//! [`TurnRules`], and [`ticks`] a [`TickRecord`] over any [`TickSim`]
//! (ADR-0148).
//!
//! A record stamped with what made it ([`ReplayRun::made`]) is checked
//! against this build's stamp ([`GameTools::made`]) before anything is
//! compared: one made by another engine, another build of the game or
//! other content is refused, since its runs may differ with no desync,
//! unless the input says [`any_stamp`](ReplayInput::any_stamp).
//!
//! A panic in the rules ends the run at its step ([`ReplayStep::panic`])
//! instead of the program. With [`crash`](ReplayInput::crash), the record
//! is a crash report's ([`shuyi_platform::crash::CrashReport::record`]),
//! run once, and the answer says whether the replay panics where the
//! report did ([`CrashReplay`]).

use std::any::Any;
use std::cell::{Cell, RefCell};
use std::fmt::{Debug, Display, Write as _};
use std::fs;
use std::panic::{AssertUnwindSafe, PanicHookInfo, catch_unwind};
use std::path::Path;
use std::sync::Once;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use shuyi_assets::keymap::ActionSet;
use shuyi_core::tick::{ActionId, SeatInput, TickInput, TickRecord, TickSim};
use shuyi_core::{Made, StateHash, TurnRules, state_hash};
use shuyi_ui::tick::ActionIds;

use super::codes::{self, Code};
use super::{Answer, Op};
use crate::game::{GameTools, code_of};

/// What `replay` re-runs.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
// An input: built with `ReplayInput::new` (or `default()`) and its fields
// assigned, it may grow (ADR-0039).
#[non_exhaustive]
pub struct ReplayInput {
    /// The record's file, from the project's root.
    pub record: String,
    /// A second record to run beside it; without one, the record is run
    /// twice.
    #[serde(default)]
    pub other: Option<String>,
    /// Run a record stamped by another build or with other content too
    /// (`--any-stamp`), which is otherwise refused; its runs may then
    /// differ for that reason alone.
    #[serde(default)]
    pub any_stamp: bool,
    /// `record` names a crash report (`--crash`): the game's record it
    /// holds is run once, and passes if it plays to its end without a
    /// panic. Takes no [`other`](Self::other).
    #[serde(default)]
    pub crash: bool,
}

impl ReplayInput {
    /// The record in the file `record`, run twice (no
    /// [`other`](Self::other)), refused if stamped by another build.
    pub fn new(record: impl Into<String>) -> Self {
        Self {
            record: record.into(),
            other: None,
            any_stamp: false,
            crash: false,
        }
    }
}

/// A panic: its message and where it happened.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
// `#[non_exhaustive]` (ADR-0039, kind 3): it may gain fields.
#[non_exhaustive]
pub struct PanicAt {
    /// The panic's message: its payload when that is text.
    pub message: String,
    /// Where it panicked: `file:line:column`, if known.
    pub location: Option<String>,
}

impl PanicAt {
    /// A panic with `message` at `location`.
    pub fn new(message: impl Into<String>, location: Option<String>) -> Self {
        Self {
            message: message.into(),
            location,
        }
    }
}

/// What `replay --crash` found of a crash report's panic.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
// `#[non_exhaustive]` (ADR-0039, kind 3): an answer gains fields.
#[non_exhaustive]
pub struct CrashReplay {
    /// The panic the report holds.
    pub reported: PanicAt,
    /// The panic the replay ran into, if it did.
    pub replayed: Option<PanicAt>,
    /// The step it panicked at, if it did.
    pub step: Option<u32>,
    /// Whether the two panics are at the same place ([`same_place`]).
    pub same_place: bool,
}

impl CrashReplay {
    /// The report's panic `reported`, and the replay's `replayed` at
    /// `step`; [`same_place`](Self::same_place) is worked out.
    pub fn new(reported: PanicAt, replayed: Option<PanicAt>, step: Option<u32>) -> Self {
        let place = |panic: Option<&PanicAt>| panic.and_then(|p| p.location.clone());
        let same = match (place(Some(&reported)), place(replayed.as_ref())) {
            (Some(a), Some(b)) => same_place(&a, &b),
            _ => false,
        };
        Self {
            reported,
            replayed,
            step,
            same_place: same,
        }
    }
}

/// A record handed to the game: its file's name and its text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Record {
    /// The file, as given.
    pub file: String,
    /// Its text.
    pub text: String,
}

/// One run of a record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReplayRun {
    /// The record's file.
    pub record: String,
    /// Step 0 (the state it starts in), then one step per command.
    pub steps: Vec<ReplayStep>,
    /// How the game ended, in the game's words, if it did.
    pub outcome: Option<String>,
    /// What made the record, if it says: the game's `replay` passes on
    /// its record's stamp ([`turns`] leaves it `None`).
    #[serde(default)]
    pub made: Option<Made>,
}

/// One step of a run: the state after a command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
// `#[non_exhaustive]` (ADR-0039, kind 3): an answer gains fields, so a
// game reads its fields and never builds it by literal or matches it
// without `..`.
#[non_exhaustive]
pub struct ReplayStep {
    /// The step: 0 before any command, then the command's place from 1.
    pub step: u32,
    /// The command, in the game's words; none at step 0.
    pub command: Option<String>,
    /// The state's hash after it (sixteen hex digits).
    pub hash: String,
    /// What it did, in the game's words.
    pub events: Vec<String>,
    /// Why the rules refused it, if they did (the state didn't change).
    pub refused: Option<String>,
    /// The rules' panic while applying it, if they panicked: the run
    /// stops here, its hash the state as the panic left it.
    #[serde(default)]
    pub panic: Option<PanicAt>,
}

impl ReplayStep {
    /// Step `step`, after `command`, with the state's `hash`: nothing
    /// happened, nothing refused, no panic.
    pub fn new(step: u32, command: Option<String>, hash: impl Into<String>) -> Self {
        Self {
            step,
            command,
            hash: hash.into(),
            events: Vec::new(),
            refused: None,
            panic: None,
        }
    }
}

/// Where two runs part.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Difference {
    /// The first step whose hashes differ, or that only one run has.
    pub step: u32,
    /// Each run's hash at that step; none for a run that has no such
    /// step.
    pub hashes: Vec<Option<String>>,
}

/// What `replay` found.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
// `#[non_exhaustive]` (ADR-0039, kind 3): an answer gains fields, so a
// game reads its fields and never builds it by literal or matches it
// without `..`.
#[non_exhaustive]
pub struct ReplayOutput {
    /// The two runs.
    pub runs: Vec<ReplayRun>,
    /// The first step where they differ, if they do.
    pub first_difference: Option<Difference>,
    /// With `--crash`: the report's panic and the replay's.
    #[serde(default)]
    pub crash: Option<CrashReplay>,
}

thread_local! {
    /// Whether a step of [`turns`] runs on this thread: its panic is
    /// caught, so the hook prints nothing.
    static QUIET: Cell<bool> = const { Cell::new(false) };
    /// Where the last quiet panic on this thread happened.
    static LAST_PLACE: RefCell<Option<String>> = const { RefCell::new(None) };
}

/// A panic hook.
type Hook = Box<dyn Fn(&PanicHookInfo<'_>) + Sync + Send + 'static>;

/// The hook that keeps a quiet panic's place ([`QUIET`]) and passes
/// every other panic to `before`.
fn quiet(before: Hook) -> Hook {
    Box::new(move |info| {
        if QUIET.try_with(Cell::get).unwrap_or(false) {
            let place =
                (info.location()).map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()));
            let _ = LAST_PLACE.try_with(|last| *last.borrow_mut() = place);
        } else {
            before(info);
        }
    })
}

/// Installs [`quiet`] over the hook that was there, once.
fn quiet_hook() {
    static HOOK: Once = Once::new();
    HOOK.call_once(|| std::panic::set_hook(quiet(std::panic::take_hook())));
}

/// `f`'s value, or the panic it ended in, caught quietly.
fn caught<T>(f: impl FnOnce() -> T) -> Result<T, PanicAt> {
    quiet_hook();
    LAST_PLACE.with(|last| last.borrow_mut().take());
    let was = QUIET.replace(true);
    let result = catch_unwind(AssertUnwindSafe(f));
    QUIET.set(was);
    result.map_err(|payload| {
        PanicAt::new(
            payload_text(payload.as_ref()),
            LAST_PLACE.with(|last| last.borrow_mut().take()),
        )
    })
}

/// A panic's payload as text: a `&str` or a `String`, else a word for it.
fn payload_text(payload: &(dyn Any + Send)) -> String {
    payload
        .downcast_ref::<&str>()
        .map(|s| (*s).to_owned())
        .or_else(|| payload.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "a panic with no text".to_owned())
}

/// Whether two places, `file:line:column`, are the same: the same line and
/// column, and files that agree in the last three parts of their paths
/// (all of the shorter's when it has fewer), `\` read as `/`. So a place
/// from a Windows build, or from one whose paths were remapped, matches
/// one from Linux.
pub fn same_place(a: &str, b: &str) -> bool {
    let split = |place: &str| {
        let mut parts = place.rsplitn(3, ':');
        let (column, line) = (parts.next()?.to_owned(), parts.next()?.to_owned());
        let file = parts.next()?.replace('\\', "/");
        let path: Vec<String> = file
            .split('/')
            .filter(|p| !p.is_empty())
            .map(str::to_owned)
            .collect();
        Some((path, line, column))
    };
    let (Some((a, line_a, column_a)), Some((b, line_b, column_b))) = (split(a), split(b)) else {
        return false;
    };
    let n = a.len().min(b.len()).min(3);
    n > 0 && line_a == line_b && column_a == column_b && a[a.len() - n..] == b[b.len() - n..]
}

/// Re-runs `commands` from `initial` by the rules, hashing the state
/// before them and after each: the run of a turn game's record `file`. A
/// refused command changes nothing and says why. A panic in the rules
/// stops the run at its step ([`ReplayStep::panic`]), with no outcome.
pub fn turns<R>(file: &str, initial: R, commands: &[R::Command]) -> Result<ReplayRun, String>
where
    R: TurnRules + Serialize,
    R::Command: Debug,
    R::Event: Debug,
    R::Error: Display,
    R::Outcome: Debug,
{
    let mut state = initial;
    let hash = |state: &R| {
        state_hash(state)
            .map(|h| h.to_string())
            .map_err(|e| e.to_string())
    };
    let mut steps = vec![ReplayStep::new(0, None, hash(&state)?)];
    let mut panicked = false;
    for (step, command) in (1..).zip(commands) {
        let mut done = ReplayStep::new(step, Some(format!("{command:?}")), String::new());
        match caught(|| state.apply(command)) {
            Ok(Ok(events)) => done.events = events.iter().map(|e| format!("{e:?}")).collect(),
            Ok(Err(e)) => done.refused = Some(e.to_string()),
            Err(panic) => done.panic = Some(panic),
        }
        done.hash = hash(&state)?;
        panicked = done.panic.is_some();
        steps.push(done);
        if panicked {
            break;
        }
    }
    let outcome = if panicked { None } else { state.outcome() };
    Ok(ReplayRun {
        record: file.to_owned(),
        steps,
        outcome: outcome.map(|o| format!("{o:?}")),
        made: None,
    })
}

/// One seat's input of a step in words, its actions named as `A` names
/// them: `pressed Up`, `released Up`, `held Right`, joined by `; `. `None`
/// when it did nothing.
fn seat_in_words<A: ActionSet>(seat: &SeatInput) -> Option<String> {
    let name = |id: &ActionId| {
        A::from_id(*id).map_or_else(|| format!("#{}", id.0), |a| a.name().to_owned())
    };
    let parts: Vec<String> = [
        ("pressed", &seat.pressed),
        ("released", &seat.released),
        ("held", &seat.held),
    ]
    .into_iter()
    .filter(|(_, ids)| !ids.is_empty())
    .map(|(what, ids)| {
        let names: Vec<String> = ids.iter().map(name).collect();
        format!("{what} {}", names.join(", "))
    })
    .collect();
    (!parts.is_empty()).then(|| parts.join("; "))
}

/// A step's input in words (`pressed Up; held Up`): seat 0's as it is, each
/// other seat's after its number (`seat 2: pressed Up`), joined by ` · `.
/// `None` when no seat did anything.
pub fn input_in_words<A: ActionSet>(input: &TickInput) -> Option<String> {
    let seats: Vec<String> = (0..)
        .zip(&input.seats)
        .filter_map(|(n, seat)| {
            let words = seat_in_words::<A>(seat)?;
            Some(if n == 0 {
                words
            } else {
                format!("seat {n}: {words}")
            })
        })
        .collect();
    (!seats.is_empty()).then(|| seats.join(" · "))
}

/// Re-runs `record` from `initial`, the state the caller built from the
/// record's seed, by the rules of a tick game (ADR-0148), hashing the state
/// before the first step and after each: the run of the record `file`.
/// Each step's `command` is its input in words, its actions named as `A`
/// names them ([`input_in_words`]), none for a step with no input; its
/// `hash` is [`TickSim::hash`] in sixteen hex digits; a step is never
/// refused. `outcome` says how the game ended, in its words, if it has;
/// the run carries the record's stamp, which `replay` checks.
pub fn ticks<A, S>(
    file: &str,
    initial: S,
    record: &TickRecord,
    outcome: impl Fn(&S) -> Option<String>,
) -> ReplayRun
where
    A: ActionSet,
    S: TickSim,
    S::Event: Debug,
{
    let mut sim = initial;
    let hash = |sim: &S| StateHash(sim.hash()).to_string();
    let mut steps = vec![ReplayStep::new(0, None, hash(&sim))];
    for (step, input) in (1..).zip(record.inputs()) {
        let events = sim.step(input);
        let mut done = ReplayStep::new(step, input_in_words::<A>(input), hash(&sim));
        done.events = events.iter().map(|e| format!("{e:?}")).collect();
        steps.push(done);
    }
    ReplayRun {
        record: file.to_owned(),
        steps,
        outcome: outcome(&sim),
        made: Some(record.made().clone()),
    }
}

/// The first step where `a` and `b` differ, if they do.
pub fn first_difference(a: &ReplayRun, b: &ReplayRun) -> Option<Difference> {
    let longest = a.steps.len().max(b.steps.len());
    (0..longest).find_map(|i| {
        let hashes = [a, b].map(|run| run.steps.get(i).map(|s| s.hash.clone()));
        (hashes[0] != hashes[1]).then(|| Difference {
            step: u32::try_from(i).unwrap_or(u32::MAX),
            hashes: hashes.to_vec(),
        })
    })
}

/// The record at `path` under `root`.
fn read(root: &Path, path: &str) -> Result<Record, String> {
    let text = fs::read_to_string(root.join(path)).map_err(|e| format!("{path}: {e}"))?;
    Ok(Record {
        file: path.to_owned(),
        text,
    })
}

/// The step of `run` the rules panicked at, if they did.
fn panicked(run: &ReplayRun) -> Option<(u32, &PanicAt)> {
    (run.steps.iter()).find_map(|s| s.panic.as_ref().map(|p| (s.step, p)))
}

/// The limit of a run whose rules refused commands, if they did.
fn refusals(run: &ReplayRun) -> Option<String> {
    let refused = run.steps.iter().filter(|s| s.refused.is_some()).count();
    (refused > 0).then(|| {
        format!(
            "the rules refused {refused} command(s) of {}: the record doesn't play as it was made",
            run.record
        )
    })
}

/// `replay --crash`: the record in the crash report `input.record`, under
/// `root`, run once by `game`, and its panic set beside the report's.
fn crash(game: &dyn GameTools, root: &Path, input: &ReplayInput) -> Answer<ReplayOutput> {
    let failed = |code, e: String| Answer::failed(Op::Replay, code, e);
    if input.other.is_some() {
        let e = "--crash replays one crash report, with no second record";
        return failed(codes::WRONG_ARGUMENTS, e.to_owned());
    }
    let report = match super::crash::read_report(root, &input.record) {
        Ok(report) => report,
        Err((code, e)) => return failed(code, e),
    };
    let Some(text) = report.record else {
        return failed(
            codes::NO_CRASH_RECORD,
            format!(
                "{} holds no record of the game's play: a web report, a panic before a stage \
                 was open, or a game whose tools give no crash_record",
                input.record
            ),
        );
    };
    let record = Record {
        file: input.record.clone(),
        text,
    };
    let run = match game.replay(&record) {
        Ok(run) => run,
        Err(e) => return failed(code_of(&e), e),
    };
    let made = game.made();
    let mut limits = match stamps(made.as_ref(), std::slice::from_ref(&run), input.any_stamp) {
        Ok(limits) => limits,
        Err(e) => return failed(codes::OTHER_STAMP, e),
    };
    limits.extend(refusals(&run));
    let replayed = panicked(&run);
    let crash = CrashReplay::new(
        PanicAt::new(report.message, report.location),
        replayed.map(|(_, panic)| panic.clone()),
        replayed.map(|(step, _)| step),
    );
    let ok = crash.replayed.is_none();
    if ok {
        limits.push(
            "only the rules were replayed: a panic in a screen, a skin or the platform does not \
             run here"
                .to_owned(),
        );
    }
    let output = ReplayOutput {
        runs: vec![run],
        first_difference: None,
        crash: Some(crash),
    };
    Answer::new(Op::Replay, ok, output, limits)
}

/// `replay`: `input`'s records, under `root`, re-run by `game`.
pub fn replay(game: &dyn GameTools, root: &Path, input: &ReplayInput) -> Answer<ReplayOutput> {
    if input.crash {
        return crash(game, root, input);
    }
    let runs = (|| {
        let read = |path| read(root, path).map_err(|e| (codes::FILE, e));
        let first = read(&input.record)?;
        let second = match &input.other {
            Some(other) => read(other)?,
            None => first.clone(),
        };
        let replay = |record| game.replay(record).map_err(|e| (code_of(&e), e));
        Ok::<_, (Code, String)>([replay(&first)?, replay(&second)?])
    })();
    let runs = match runs {
        Ok(runs) => runs,
        Err((code, e)) => return Answer::failed(Op::Replay, code, e),
    };
    // A record run twice is checked once.
    let records = if input.other.is_some() {
        &runs[..]
    } else {
        &runs[..1]
    };
    let mut limits = match stamps(game.made().as_ref(), records, input.any_stamp) {
        Ok(limits) => limits,
        Err(e) => return Answer::failed(Op::Replay, codes::OTHER_STAMP, e),
    };
    let difference = first_difference(&runs[0], &runs[1]);
    if input.other.is_none() {
        limits.push(
            "the record ran twice in one process: what differs from one machine or build to \
             another needs a second record made there"
                .to_owned(),
        );
    }
    limits.extend(refusals(&runs[0]));
    let panic = panicked(&runs[0]);
    if let Some((step, panic)) = panic {
        limits.push(format!(
            "the rules panicked at step {step} of {}: {}",
            runs[0].record, panic.message
        ));
    }
    let ok = difference.is_none() && panic.is_none();
    let output = ReplayOutput {
        runs: runs.to_vec(),
        first_difference: difference,
        crash: None,
    };
    Answer::new(Op::Replay, ok, output, limits)
}

/// The limits the stamps of `runs` put on what they show, checked against
/// this build's, `now`; the refusal of the first stamped by another build,
/// unless `any_stamp`.
fn stamps(now: Option<&Made>, runs: &[ReplayRun], any_stamp: bool) -> Result<Vec<String>, String> {
    let Some(now) = now else {
        return Ok(vec![
            "the game's tools give no stamp of this build (GameTools::made), so no record was \
             checked against it: a difference may come from another build or other content"
                .to_owned(),
        ]);
    };
    let mut limits = Vec::new();
    for run in runs {
        let record = &run.record;
        match run.made.as_ref().map(|made| made.check(now)) {
            None => limits.push(format!(
                "{record} has no stamp: a difference may come from another build or other content"
            )),
            Some(Ok(())) => {}
            Some(Err(mismatch)) if any_stamp => limits.push(format!(
                "{record} was {mismatch}: a difference may come from that, not a desync"
            )),
            Some(Err(mismatch)) => {
                return Err(format!(
                    "{record} was {mismatch}: make it again with this build, or replay it with \
                     --any-stamp"
                ));
            }
        }
    }
    Ok(limits)
}

/// `replay`'s usage, with `command` as the tool's name.
pub fn usage(command: &str) -> String {
    format!(
        "usage: {command} replay <record> [<other>] [--any-stamp] [--json]\n       {command} \
         replay --crash <report> [--any-stamp] [--json]"
    )
}

/// A panic for people: its message, then where.
fn panic_text(panic: &PanicAt) -> String {
    match &panic.location {
        Some(place) => format!("{} at {place}", panic.message),
        None => panic.message.clone(),
    }
}

/// What `replay` found, for people.
pub fn text(output: &ReplayOutput) -> String {
    let mut out = String::new();
    let Some(first) = output.runs.first() else {
        return out;
    };
    let _ = writeln!(
        out,
        "replay: {} ({} step(s)), outcome {}",
        first.record,
        first.steps.len().saturating_sub(1),
        first.outcome.as_deref().unwrap_or("none")
    );
    for (i, run) in output.runs.iter().enumerate() {
        if i > 0 && run.record == first.record {
            continue;
        }
        match &run.made {
            Some(made) => {
                let _ = writeln!(out, "  {} was made with {made}", run.record);
            }
            None => {
                let _ = writeln!(out, "  {} has no stamp", run.record);
            }
        }
    }
    for step in &first.steps {
        let command = step.command.as_deref().unwrap_or("start");
        let _ = write!(out, "  {:>4} {} {command}", step.step, step.hash);
        if let Some(panic) = &step.panic {
            let _ = write!(out, " (panicked: {})", panic_text(panic));
        } else if let Some(refused) = &step.refused {
            let _ = write!(out, " (refused: {refused})");
        } else if !step.events.is_empty() {
            let _ = write!(out, " → {}", step.events.join(", "));
        }
        out.push('\n');
    }
    match (&output.crash, &output.first_difference) {
        (Some(crash), _) => {
            let _ = writeln!(out, "the report: {}", panic_text(&crash.reported));
            match (&crash.replayed, crash.step) {
                (Some(panic), Some(step)) => {
                    let place = if crash.same_place {
                        "the same place"
                    } else {
                        "another place"
                    };
                    let _ = writeln!(
                        out,
                        "the replay panicked at step {step}, at {place}: {}",
                        panic_text(panic)
                    );
                }
                _ => out.push_str("the replay played to its end without a panic\n"),
            }
        }
        (None, None) => out.push_str("the two runs agree at every step\n"),
        (None, Some(d)) => {
            let shown: Vec<&str> = d
                .hashes
                .iter()
                .map(|h| h.as_deref().unwrap_or("(none)"))
                .collect();
            let _ = writeln!(
                out,
                "the runs differ first at step {}: {}",
                d.step,
                shown.join(" vs ")
            );
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use serde::Serialize;
    use shuyi_core::TurnRules;

    use super::*;
    use crate::contract::fake::{BRITTLE_LINE, Brittle, Fake, stamp};
    use shuyi_core::ContentHash;

    /// A counter that ends at 3; an `Add` of more than 1 is refused.
    #[derive(Debug, Clone, PartialEq, Eq, Serialize)]
    struct Counter(u32);

    impl TurnRules for Counter {
        type Command = u32;
        type Event = u32;
        type Error = String;
        type Outcome = &'static str;

        fn check(&self, add: &u32) -> Result<(), String> {
            if *add > 1 {
                Err(format!("{add} is too many"))
            } else {
                Ok(())
            }
        }

        fn apply(&mut self, add: &u32) -> Result<Vec<u32>, String> {
            self.check(add)?;
            self.0 += add;
            Ok(vec![self.0])
        }

        fn legal_commands(&self) -> Vec<u32> {
            vec![0, 1]
        }

        fn outcome(&self) -> Option<&'static str> {
            (self.0 >= 3).then_some("Done")
        }
    }

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("shuyi-replay-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn turns_hashes_the_state_before_and_after_each_command() {
        let run = turns("r.ron", Counter(0), &[1, 5, 1, 1]).unwrap();
        assert_eq!(run.record, "r.ron");
        assert_eq!(run.steps.len(), 5);
        let hash = |n: u32| state_hash(&Counter(n)).unwrap().to_string();
        assert_eq!(run.steps[0].hash, hash(0));
        assert_eq!(
            (run.steps[0].step, run.steps[0].command.as_deref()),
            (0, None)
        );
        assert_eq!(run.steps[1].command.as_deref(), Some("1"));
        assert_eq!(run.steps[1].events, ["1"]);
        assert_eq!(run.steps[1].hash, hash(1));
        // Refused: the state, and its hash, stay.
        assert_eq!(run.steps[2].refused.as_deref(), Some("5 is too many"));
        let got = &run.steps[2].events;
        assert!(got.is_empty(), "{got:?}");
        assert_eq!(run.steps[2].hash, hash(1));
        assert_eq!(run.steps[4].step, 4);
        assert_eq!(run.steps[4].hash, hash(3));
        assert_eq!(run.outcome.as_deref(), Some("\"Done\""));
        assert_eq!(turns("r", Counter(0), &[]).unwrap().outcome, None);
    }

    /// A tick game that counts the steps its first action was held, and
    /// says so each step it is.
    #[derive(Debug, Default)]
    struct Held(u64);

    impl TickSim for Held {
        type Event = &'static str;

        fn step(&mut self, input: &TickInput) -> Vec<&'static str> {
            let held = input.is_held(0, ActionId(0));
            self.0 += u64::from(held);
            if held { vec!["held"] } else { Vec::new() }
        }

        fn hash(&self) -> u64 {
            self.0
        }
    }

    #[test]
    fn ticks_hashes_the_state_before_the_first_step_and_after_each() {
        use shuyi_assets::keymap::UiAction;

        let made = stamp("0.1.0", "snake", ContentHash(7));
        let mut record = TickRecord::new(made.clone(), 1);
        // Up (action 0) pressed and held, still held, then let go; another
        // seat presses Confirm in the last step.
        let up = UiAction::Up.id();
        record.push(TickInput::one(SeatInput::new(vec![up], vec![up], vec![])));
        record.push(TickInput::one(SeatInput::new(vec![], vec![up], vec![])));
        let confirm = UiAction::Confirm.id();
        record.push(TickInput::new(vec![
            SeatInput::new(vec![], vec![], vec![up]),
            SeatInput::new(vec![confirm], vec![], vec![]),
        ]));
        record.push(TickInput::default());
        let run = ticks::<UiAction, _>("r.ron", Held(0), &record, |held| {
            (held.0 >= 2).then(|| format!("held {} steps", held.0))
        });
        assert_eq!(run.record, "r.ron");
        let shown: Vec<(u32, Option<&str>, &str)> = (run.steps.iter())
            .map(|s| (s.step, s.command.as_deref(), s.hash.as_str()))
            .collect();
        assert_eq!(
            shown,
            [
                (0, None, "0000000000000000"),
                (1, Some("pressed Up; held Up"), "0000000000000001"),
                (2, Some("held Up"), "0000000000000002"),
                (
                    3,
                    Some("released Up · seat 1: pressed Confirm"),
                    "0000000000000002"
                ),
                (4, None, "0000000000000002"),
            ]
        );
        assert_eq!(run.steps[1].events, ["\"held\""]);
        let found = &run.steps[3].events;
        assert!(found.is_empty(), "{found:?}");
        assert!(run.steps.iter().all(|s| s.refused.is_none()));
        assert_eq!(run.outcome.as_deref(), Some("held 2 steps"));
        // It carries the record's stamp, which `replay` checks.
        assert_eq!(run.made, Some(made));
        // An id past the game's actions is named by its number.
        let stray = TickInput::one(SeatInput::new(vec![ActionId(99)], vec![], vec![]));
        assert_eq!(
            input_in_words::<UiAction>(&stray).as_deref(),
            Some("pressed #99")
        );
    }

    fn run_of(hashes: &[&str]) -> ReplayRun {
        ReplayRun {
            record: "r".into(),
            steps: (0..)
                .zip(hashes)
                .map(|(step, hash)| ReplayStep::new(step, None, *hash))
                .collect(),
            outcome: None,
            made: None,
        }
    }

    #[test]
    fn the_first_difference_is_the_first_step_that_differs_or_only_one_has() {
        let a = run_of(&["a", "b", "c"]);
        assert_eq!(first_difference(&a, &a), None);
        let b = run_of(&["a", "x", "c"]);
        assert_eq!(
            first_difference(&a, &b),
            Some(Difference {
                step: 1,
                hashes: vec![Some("b".into()), Some("x".into())]
            })
        );
        let short = run_of(&["a", "b"]);
        assert_eq!(
            first_difference(&a, &short),
            Some(Difference {
                step: 2,
                hashes: vec![Some("c".into()), None]
            })
        );
        assert_eq!(first_difference(&short, &a).unwrap().hashes[0], None);
    }

    #[test]
    fn replay_runs_one_record_twice_or_two_side_by_side() {
        let dir = scratch("runs");
        fs::write(dir.join("a.ron"), "h0\nh1\nh2").unwrap();
        fs::write(dir.join("b.ron"), "h0\nhx").unwrap();
        fs::write(dir.join("c.ron"), "h0\nrefused").unwrap();
        let input = |record: &str, other: Option<&str>| {
            let mut input = ReplayInput::new(record);
            input.other = other.map(Into::into);
            input
        };
        let answer = replay(&Fake::default(), &dir, &input("a.ron", None));
        assert!(answer.ok, "{answer:?}");
        assert_eq!(answer.op, Op::Replay);
        assert_eq!(answer.limits.len(), 2);
        assert!(answer.limits[0].starts_with("the game's tools give no stamp of this build"));
        assert!(answer.limits[1].starts_with("the record ran twice in one process"));
        let out = answer.result.unwrap();
        assert_eq!(out.runs.len(), 2);
        assert_eq!(out.runs[0], out.runs[1]);
        let answer = replay(&Fake::default(), &dir, &input("a.ron", Some("b.ron")));
        assert!(!answer.ok);
        assert_eq!(answer.limits.len(), 1, "{:?}", answer.limits);
        let out = answer.result.unwrap();
        assert_eq!(out.runs[1].record, "b.ron");
        assert_eq!(out.first_difference.unwrap().step, 1);
        let refused = replay(&Fake::default(), &dir, &input("c.ron", Some("c.ron")));
        assert_eq!(
            refused.limits[1..],
            ["the rules refused 1 command(s) of c.ron: the record doesn't play as it was made"]
        );
        // What can't be read or run says why.
        let missing = replay(&Fake::default(), &dir, &input("a.ron", Some("no.ron")));
        assert_eq!(missing.code.as_deref(), Some("SH3004"));
        assert_eq!(missing.exit_code(), 1);
        assert!(missing.error.unwrap().starts_with("no.ron: "));
        let game = Fake {
            replay: Err("bad record".into()),
            ..Fake::default()
        };
        let bad = replay(&game, &dir, &input("a.ron", None));
        assert_eq!(bad.code.as_deref(), Some("SH3002"));
        assert_eq!(bad.error.as_deref(), Some("bad record"));
        let unsupported = Fake {
            replay: Err(crate::game::unsupported("replay", "replay")),
            ..Fake::default()
        };
        let answer = replay(&unsupported, &dir, &input("a.ron", None));
        assert_eq!(answer.code.as_deref(), Some("SH3001"));
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_record_stamped_by_another_build_is_refused_unless_any_stamp() {
        let dir = scratch("stamps");
        let now = stamp("0.2.0", "demo", ContentHash(2));
        let game = Fake {
            made: Some(now.clone()),
            ..Fake::default()
        };
        fs::write(
            dir.join("same.ron"),
            "made 0.2.0 demo 0000000000000002\nh0\nh1",
        )
        .unwrap();
        fs::write(
            dir.join("other.ron"),
            "made 0.2.0 demo 0000000000000001\nh0\nh1",
        )
        .unwrap();
        fs::write(dir.join("none.ron"), "h0\nh1").unwrap();
        // Stamped by this build: it runs, with its stamp, and nothing to say.
        let same = replay(&game, &dir, &ReplayInput::new("same.ron"));
        assert!(same.ok, "{same:?}");
        assert_eq!(same.limits.len(), 1, "{:?}", same.limits);
        let out = same.result.unwrap();
        assert_eq!(out.runs[0].made.as_ref(), Some(&now));
        assert_eq!(out.runs[0].steps.len(), 2);
        // Other content: refused before anything is compared, naming the
        // record and both stamps' contents.
        let mut input = ReplayInput::new("same.ron");
        input.other = Some("other.ron".into());
        let other = replay(&game, &dir, &input);
        assert!(!other.ok);
        assert_eq!(other.code.as_deref(), Some("SH3007"));
        assert_eq!(other.exit_code(), 1);
        assert_eq!(
            other.error.as_deref(),
            Some(
                "other.ron was made with content 0000000000000001; this build: content \
                 0000000000000002: make it again with this build, or replay it with --any-stamp"
            )
        );
        assert!(other.result.is_none());
        // With `any_stamp`, it runs and says why a difference may show.
        input.any_stamp = true;
        let anyway = replay(&game, &dir, &input);
        assert!(anyway.ok, "{anyway:?}");
        assert_eq!(
            anyway.limits,
            [
                "other.ron was made with content 0000000000000001; this build: content \
              0000000000000002: a difference may come from that, not a desync"
            ]
        );
        // A record with no stamp runs, and says so.
        let none = replay(&game, &dir, &ReplayInput::new("none.ron"));
        assert!(none.ok, "{none:?}");
        assert_eq!(
            none.limits[0],
            "none.ron has no stamp: a difference may come from another build or other content"
        );
        // A game that gives no stamp checks none.
        let unstamped = replay(&Fake::default(), &dir, &ReplayInput::new("other.ron"));
        assert!(unstamped.ok, "{unstamped:?}");
        let got = &unstamped.limits[0];
        assert!(got.contains("GameTools::made"), "{got}");
        // The stamp goes out as text, and reads back.
        let json = serde_json::to_value(&out.runs[0]).unwrap();
        assert_eq!(
            json["made"],
            serde_json::json!({"engine": "0.2.0", "game": "demo", "content": "0000000000000002"})
        );
        let back: ReplayRun = serde_json::from_value(json).unwrap();
        assert_eq!(back, out.runs[0]);
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn what_replay_found_reads_for_people() {
        let mut run = run_of(&["h0", "h1", "h2"]);
        run.outcome = Some("Won".into());
        run.steps[1].command = Some("Move".into());
        run.steps[1].events = vec!["Moved".into(), "Reached".into()];
        run.steps[2].command = Some("Stay".into());
        run.steps[2].refused = Some("over".into());
        let agreed = ReplayOutput {
            runs: vec![run.clone(), run.clone()],
            first_difference: None,
            crash: None,
        };
        assert_eq!(
            text(&agreed),
            "replay: r (2 step(s)), outcome Won\n  r has no stamp\n     0 h0 start\n     1 h1 \
             Move → Moved, Reached\n     2 h2 Stay (refused: over)\nthe two runs agree at every \
             step\n"
        );
        let mut stamped = run_of(&["h0"]);
        stamped.made = Some(stamp("0.1.0", "demo", ContentHash(1)));
        let mut other = run_of(&["h0"]);
        other.record = "o".into();
        let two = ReplayOutput {
            runs: vec![stamped, other],
            first_difference: None,
            crash: None,
        };
        assert_eq!(
            text(&two),
            "replay: r (0 step(s)), outcome none\n  r was made with engine 0.1.0, game \"demo\", \
             content 0000000000000001\n  o has no stamp\n     0 h0 start\nthe two runs agree at \
             every step\n"
        );
        let parted = ReplayOutput {
            runs: vec![run_of(&["a"])],
            first_difference: Some(Difference {
                step: 1,
                hashes: vec![Some("b".into()), None],
            }),
            crash: None,
        };
        assert_eq!(
            text(&parted),
            "replay: r (0 step(s)), outcome none\n  r has no stamp\n     0 a start\nthe runs \
             differ first at step 1: b vs (none)\n"
        );
        let none = ReplayOutput {
            runs: Vec::new(),
            first_difference: None,
            crash: None,
        };
        assert_eq!(text(&none), "");
    }

    #[test]
    fn a_new_input_runs_its_record_twice() {
        let literal = ReplayInput {
            record: "a.ron".to_owned(),
            other: None,
            any_stamp: false,
            crash: false,
        };
        assert_eq!(ReplayInput::new("a.ron"), literal);
    }

    /// Where [`Brittle`]'s panic is, as this build compiled it.
    fn brittle_place(location: Option<&str>) -> bool {
        let tail = format!("fake.rs:{BRITTLE_LINE}:13");
        location.is_some_and(|place| place.ends_with(&tail))
    }

    #[test]
    fn turns_stops_at_a_panic_with_its_message_and_place() {
        let run = turns("r.ron", Brittle(0, false), &[1, 1, 9, 1]).unwrap();
        // Steps 0 to 3: the fourth command never runs.
        assert_eq!(run.steps.len(), 4);
        let panic = run.steps[3].panic.as_ref().unwrap();
        assert_eq!(panic.message, "nine is too many");
        assert!(brittle_place(panic.location.as_deref()), "{panic:?}");
        // The hash is the state as the panic left it, with no outcome.
        let left = state_hash(&Brittle(3, false)).unwrap().to_string();
        assert_eq!(run.steps[3].hash, left);
        assert_eq!(run.steps[3].command.as_deref(), Some("9"));
        assert!(run.steps[3].events.is_empty() && run.steps[3].refused.is_none());
        assert_eq!(run.outcome, None);
        assert!(run.steps[..3].iter().all(|s| s.panic.is_none()));
        // Fixed, it plays on to its outcome.
        let fixed = turns("r.ron", Brittle(0, true), &[1, 1, 9, 1]).unwrap();
        assert_eq!(fixed.steps.len(), 5);
        assert_eq!(fixed.outcome.as_deref(), Some("12"));
        // A payload that is a `String`, or isn't text.
        let owned = caught(|| std::panic::panic_any(String::from("owned")));
        assert_eq!(owned.unwrap_err().message, "owned");
        let number = caught(|| std::panic::panic_any(7_u8)).unwrap_err();
        assert_eq!(number.message, "a panic with no text");
        assert!(number.location.unwrap().contains("replay.rs:"));
    }

    #[test]
    fn the_quiet_hook_passes_other_threads_panics_on() {
        use std::sync::{Arc, Mutex};

        quiet_hook();
        let passed: Arc<Mutex<Vec<String>>> = Arc::default();
        let seen = Arc::clone(&passed);
        let installed = std::panic::take_hook();
        std::panic::set_hook(quiet(Box::new(move |info| {
            let name = std::thread::current().name().map(str::to_owned);
            seen.lock().unwrap().push(name.unwrap_or_default());
            let _ = info;
        })));
        let elsewhere = std::thread::Builder::new()
            .name("elsewhere".into())
            .spawn(|| panic!("not a step"))
            .unwrap();
        assert!(elsewhere.join().is_err());
        let here = caught(|| panic!("a step"));
        std::panic::set_hook(installed);
        assert_eq!(here.unwrap_err().message, "a step");
        let passed = passed.lock().unwrap();
        assert!(passed.iter().any(|n| n == "elsewhere"), "{passed:?}");
        let this = std::thread::current().name().unwrap_or_default().to_owned();
        assert!(!passed.contains(&this), "{passed:?}");
    }

    #[test]
    fn same_place_reads_paths_by_their_last_three_parts() {
        let remapped = "/cargo/git/checkouts/x/1/src/rules.rs:9:5";
        assert!(same_place(r"src\rules.rs:9:5", remapped));
        assert!(same_place(remapped, r"src\rules.rs:9:5"));
        assert!(!same_place("src/rules.rs:10:5", remapped));
        assert!(!same_place("src/rules.rs:9:6", remapped));
        assert!(same_place(
            r"C:\Users\me\game\crates\rules\src\lib.rs:3:1",
            "/home/ci/game/crates/rules/src/lib.rs:3:1"
        ));
        // Three parts are compared, not just the file's name.
        assert!(!same_place("a/src/rules.rs:9:5", "b/lib/rules.rs:9:5"));
        assert!(same_place("x/a/src/rules.rs:9:5", "y/a/src/rules.rs:9:5"));
        assert!(!same_place("rules.rs", "rules.rs"));
        assert!(!same_place(":9:5", ":9:5"));
    }

    #[test]
    fn replay_answers_a_panic_with_its_step_instead_of_ending() {
        let dir = scratch("panics");
        fs::write(dir.join("p.ron"), "turns 1,1,9,1").unwrap();
        let answer = replay(&Fake::default(), &dir, &ReplayInput::new("p.ron"));
        assert!(!answer.ok);
        assert_eq!(answer.code, None);
        assert_eq!(
            answer.limits.last().map(String::as_str),
            Some("the rules panicked at step 3 of p.ron: nine is too many")
        );
        let out = answer.result.unwrap();
        assert_eq!(out.first_difference, None);
        assert!(out.crash.is_none());
        let shown = text(&out);
        assert!(
            shown.contains("     3 ") && shown.contains(" 9 (panicked: nine is too many at "),
            "{shown}"
        );
        fs::remove_dir_all(&dir).unwrap();
    }

    /// A crash report of `message` at `location`, holding `record`.
    fn report(location: &str, record: Option<&str>) -> String {
        let mut report = shuyi_platform::crash::CrashReport::new(1, "nine is too many");
        report.location = Some(location.to_owned());
        report.set_record(record.map(str::to_owned));
        report.to_text()
    }

    #[test]
    fn replay_crash_replays_a_reports_record_to_its_panic() {
        let dir = scratch("crash");
        let crash_of = |file: &str| {
            let mut input = ReplayInput::new(file);
            input.crash = true;
            input
        };
        // The place the replay gives, as the report would have it.
        let place = turns("r", Brittle(0, false), &[1, 1, 9]).unwrap().steps[3]
            .panic
            .clone()
            .unwrap()
            .location
            .unwrap();
        fs::write(dir.join("c.ron"), report(&place, Some("turns 1,1,9,1"))).unwrap();
        let answer = replay(&Fake::default(), &dir, &crash_of("c.ron"));
        assert!(!answer.ok, "{answer:?}");
        assert_eq!(answer.code, None);
        let out = answer.result.unwrap();
        assert_eq!((out.runs.len(), out.first_difference.as_ref()), (1, None));
        assert_eq!(out.runs[0].record, "c.ron");
        let crash = out.crash.clone().unwrap();
        assert_eq!((crash.step, crash.same_place), (Some(3), true));
        assert_eq!(
            crash.reported,
            PanicAt::new("nine is too many", Some(place.clone()))
        );
        assert_eq!(crash.replayed.as_ref().unwrap().message, "nine is too many");
        // Run once: no "ran twice" line, and no rules-only line either.
        assert!(
            answer
                .limits
                .iter()
                .all(|l| !l.contains("ran twice") && !l.contains("only the rules")),
            "{:?}",
            answer.limits
        );
        let shown = text(&out);
        assert!(
            shown.contains(&format!("the report: nine is too many at {place}\n"))
                && shown.contains("the replay panicked at step 3, at the same place: "),
            "{shown}"
        );
        // Another line in the report: not the same place.
        let line = format!(":{BRITTLE_LINE}:");
        let elsewhere = place.replace(&line, &format!(":{}:", BRITTLE_LINE + 1));
        fs::write(dir.join("d.ron"), report(&elsewhere, Some("turns 1,1,9"))).unwrap();
        let answer = replay(&Fake::default(), &dir, &crash_of("d.ron"));
        let crash = answer.result.unwrap().crash.unwrap();
        assert_eq!((crash.step, crash.same_place), (Some(3), false));
        // The rules fixed: it plays to its end, and says what it didn't run.
        let fixed = Fake {
            fixed: true,
            ..Fake::default()
        };
        let answer = replay(&fixed, &dir, &crash_of("c.ron"));
        assert!(answer.ok, "{answer:?}");
        assert_eq!(
            answer.limits.last().map(String::as_str),
            Some(
                "only the rules were replayed: a panic in a screen, a skin or the platform does \
                 not run here"
            )
        );
        let out = answer.result.unwrap();
        let crash = out.crash.clone().unwrap();
        assert_eq!(
            (crash.replayed, crash.step, crash.same_place),
            (None, None, false)
        );
        assert!(text(&out).ends_with("the replay played to its end without a panic\n"));
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn replay_crash_refuses_what_it_cant_replay() {
        let dir = scratch("crash-refused");
        let crash_of = |file: &str, other: Option<&str>| {
            let mut input = ReplayInput::new(file);
            input.crash = true;
            input.other = other.map(str::to_owned);
            replay(&Fake::default(), &dir, &input)
        };
        fs::write(dir.join("bare.ron"), report("src/a.rs:1:1", None)).unwrap();
        let bare = crash_of("bare.ron", None);
        assert_eq!(bare.code.as_deref(), Some("SH3019"));
        assert!(
            (bare.error.unwrap()).starts_with("bare.ron holds no record of the game's play: a web")
        );
        let two = crash_of("bare.ron", Some("bare.ron"));
        assert_eq!((two.code.as_deref(), two.exit_code()), (Some("SH0001"), 2));
        fs::write(dir.join("r.ron"), "h0\nh1").unwrap();
        let record = crash_of("r.ron", None);
        assert_eq!(record.code.as_deref(), Some("SH3018"));
        assert_eq!(crash_of("nope.ron", None).code.as_deref(), Some("SH3004"));
        // The game's own error, and a stamp of other content, as `replay`.
        fs::write(dir.join("bad.ron"), report("src/a.rs:1:1", Some("turns x"))).unwrap();
        assert_eq!(crash_of("bad.ron", None).code.as_deref(), Some("SH3002"));
        let stamped = "made 0.2.0 demo 0000000000000001\nh0";
        fs::write(dir.join("old.ron"), report("src/a.rs:1:1", Some(stamped))).unwrap();
        let game = Fake {
            made: Some(stamp("0.2.0", "demo", ContentHash(2))),
            ..Fake::default()
        };
        let mut input = ReplayInput::new("old.ron");
        input.crash = true;
        assert_eq!(replay(&game, &dir, &input).code.as_deref(), Some("SH3007"));
        input.any_stamp = true;
        assert!(replay(&game, &dir, &input).ok);
        fs::remove_dir_all(&dir).unwrap();
    }
}
