//! An autobalance search: the game's [`Knob`]s change a stage a little at
//! a time (one more enemy, a weaker trap, a later ambush), the
//! [`playtest`] runner replays it with the bots, and the search stops when
//! every rate check is inside its band. [`autobalance`] does it all and
//! says what it changed, in the knobs' own words.
//!
//! **The bots never change.** Only the stage does: the search takes the
//! entrants and the checks as they are and has no way to reach their
//! settings, since bots tuned until they pass would prove nothing.
//!
//! The search is a pattern search over the knobs' integer steps, scored
//! by [`distance`]. Every run plays the same seeds, so settings are
//! compared on the same luck, and the same inputs always give the same
//! changes. What it finds is the smallest change set *this search* found,
//! not one proved the smallest.

use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;
use core::fmt;
use core::ops::RangeInclusive;

use serde::{Deserialize, Serialize};

use super::Sides;
use super::playtest::{self, Check, Entrant, Found, PlaytestError, PlaytestReport, Tries};

/// One way the game lets the search change a stage definition `S`: a
/// bounded integer step, 0 being the stage as authored.
pub trait Knob<S> {
    /// Its name, which the change set uses.
    fn name(&self) -> &str;

    /// The steps it may take. It holds 0, the stage as authored.
    fn range(&self) -> RangeInclusive<i32>;

    /// The first step size the search tries: a coarse knob (every enemy's
    /// level) starts larger. The search halves it down to 1.
    fn stride(&self) -> u32 {
        1
    }

    /// `stage` with this knob moved `step`, or `None` when it can't apply
    /// there (no free tile for one more piece). Knobs apply in the order
    /// given, each to the result of the one before; a knob at 0 isn't
    /// applied.
    fn apply(&self, stage: &S, step: i32) -> Option<S>;

    /// The change of `step` in plain words, such as `"the foe trips 10%
    /// less often"`.
    fn describe(&self, step: i32) -> String;
}

/// How hard the search works.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct BalanceSettings {
    /// The full run, which decides "in band": the stage as authored and
    /// the stage found are both played this way.
    pub tries: Tries,
    /// The try count of the search's own runs, from the same first seed.
    pub quick: u32,
    /// The most runs of the playtest runner the search makes, the first
    /// run of the stage as authored among them.
    pub evaluations: u32,
}

/// How far `report`'s rates lie outside their bands: over every
/// [`Check::Rate`],
/// `max(0, min * count - successes * 100, successes * 100 - max * count)`,
/// summed. 0 exactly when every rate is inside its band.
/// A [`Check::Median`] doesn't count: it is about the bots, not the stage.
///
/// Distances of runs with different try counts don't compare.
pub fn distance(report: &PlaytestReport) -> u64 {
    report
        .checks
        .iter()
        .map(|judged| match (&judged.check, &judged.found) {
            (
                Check::Rate { band, .. },
                Found::Rate {
                    successes, count, ..
                },
            ) => {
                let share = u64::from(*successes) * 100;
                let min = u64::from(band.min) * u64::from(*count);
                let max = u64::from(band.max) * u64::from(*count);
                min.saturating_sub(share).max(share.saturating_sub(max))
            }
            _ => 0,
        })
        .fold(0, u64::saturating_add)
}

/// Why the search stopped.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum Stop {
    /// The stage as authored was in every band: nothing changed.
    AlreadyInBands,
    /// The change set puts every rate in its band. If the budget ran out
    /// while it was being pruned, it may be larger than it had to be.
    InBands,
    /// No knob's step brought the stage any nearer its bands: the change
    /// set is the nearest the search came.
    Stuck,
    /// The budget of evaluations ran out first: the change set is the
    /// best setting seen.
    Budget,
}

impl fmt::Display for Stop {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Stop::AlreadyInBands => "already in every band: nothing to change",
            Stop::InBands => "in every band",
            Stop::Stuck => "stuck: no knob brings it nearer its bands",
            Stop::Budget => "out of evaluations: the best setting seen",
        })
    }
}

/// One knob the search moved.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Change {
    /// The knob's name.
    pub knob: String,
    /// Its step.
    pub step: i32,
    /// The knob's words for the step.
    pub what: String,
}

/// What the search found. Its `Display` says why it stopped, the changes
/// one per line, then the reports before and after.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Balanced<S> {
    /// The changed stage (the stage as authored when nothing changed).
    pub stage: S,
    /// Every knob not at 0, in knob order.
    pub changes: Vec<Change>,
    /// The stage as authored, at the full try count.
    pub before: PlaytestReport,
    /// The changed stage, at the full try count.
    pub after: PlaytestReport,
    /// Why the search stopped.
    pub stop: Stop,
    /// The runs of the playtest runner it made.
    pub evaluations: u32,
}

impl<S> fmt::Display for Balanced<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{} · {} evaluations", self.stop, self.evaluations)?;
        if self.changes.is_empty() {
            writeln!(f, "no change")?;
        }
        for change in &self.changes {
            writeln!(f, "{} {:+}: {}", change.knob, change.step, change.what)?;
        }
        write!(f, "\nbefore: {}\nafter: {}", self.before, self.after)
    }
}

/// What one evaluation of a setting gave.
enum Eval {
    /// Its distance.
    Distance(u64),
    /// A knob can't apply there.
    Inapplicable,
    /// The budget doesn't allow the run.
    Budget,
}

/// What a round over the knobs found.
enum Move {
    /// A step that lowered the distance to this, taken.
    Better(u64),
    /// No step lowers it.
    None,
    /// The budget ran out.
    Budget,
}

/// A search in progress: what it was given, and every run it made.
struct Search<'a, S, R: Sides> {
    stage: &'a S,
    knobs: &'a [&'a dyn Knob<S>],
    entrants: &'a [&'a dyn Entrant<S, R>],
    checks: &'a [Check],
    settings: &'a BalanceSettings,
    /// The distance of every setting run, by its steps and try count.
    distances: BTreeMap<(Vec<i32>, u32), u64>,
    /// The report of every setting run at the full try count.
    reports: BTreeMap<Vec<i32>, PlaytestReport>,
    /// The runs made.
    used: u32,
}

impl<S: Clone, R: Sides> Search<'_, S, R>
where
    R::Command: Clone,
    R::Outcome: fmt::Debug,
{
    /// The full try count.
    fn full(&self) -> u32 {
        self.settings.tries.count
    }

    /// The stage with every knob moved as `point` says, or `None` if one
    /// can't apply.
    fn build(&self, point: &[i32]) -> Option<S> {
        let mut stage = self.stage.clone();
        for (knob, &step) in self.knobs.iter().zip(point) {
            if step != 0 {
                stage = knob.apply(&stage, step)?;
            }
        }
        Some(stage)
    }

    /// Records a run of `point` at `count` tries.
    fn record(&mut self, point: &[i32], count: u32, report: PlaytestReport) -> u64 {
        let d = distance(&report);
        self.distances.insert((point.to_vec(), count), d);
        if count == self.full() {
            self.reports.insert(point.to_vec(), report);
        }
        d
    }

    /// The distance of `point` at `count` tries, run unless it was run
    /// before. A run at fewer tries than the full count keeps one
    /// evaluation back, for the full run of the setting it ends on.
    fn eval(&mut self, point: &[i32], count: u32) -> Result<Eval, PlaytestError<R>> {
        if let Some(&d) = self.distances.get(&(point.to_vec(), count)) {
            return Ok(Eval::Distance(d));
        }
        let Some(stage) = self.build(point) else {
            return Ok(Eval::Inapplicable);
        };
        let kept = u32::from(count != self.full());
        if self.used.saturating_add(kept) >= self.settings.evaluations {
            return Ok(Eval::Budget);
        }
        let tries = Tries {
            count,
            ..self.settings.tries
        };
        let report = playtest::run(&stage, self.entrants, &tries, self.checks)?;
        self.used += 1;
        Ok(Eval::Distance(self.record(point, count, report)))
    }

    /// Whether `point` is in every band at the full try count.
    fn in_bands(&mut self, point: &[i32]) -> Result<bool, PlaytestError<R>> {
        let full = self.full();
        Ok(matches!(self.eval(point, full)?, Eval::Distance(0)))
    }

    /// One round over the knobs from `point`, which is `here` from its
    /// bands: in knob order, `+stride` then `-stride` (clamped to the
    /// knob's range), taking the first step that lowers the distance.
    fn round(
        &mut self,
        point: &mut Vec<i32>,
        strides: &[i32],
        count: u32,
        here: u64,
    ) -> Result<Move, PlaytestError<R>> {
        let knobs = self.knobs;
        for ((k, knob), &stride) in knobs.iter().enumerate().zip(strides) {
            let range = knob.range();
            if range.is_empty() {
                continue;
            }
            for stride in [stride, -stride] {
                let to = point[k]
                    .saturating_add(stride)
                    .clamp(*range.start(), *range.end());
                if to == point[k] {
                    continue;
                }
                let mut next = point.clone();
                next[k] = to;
                match self.eval(&next, count)? {
                    Eval::Distance(d) if d < here => {
                        *point = next;
                        return Ok(Move::Better(d));
                    }
                    Eval::Budget => return Ok(Move::Budget),
                    Eval::Distance(_) | Eval::Inapplicable => {}
                }
            }
        }
        Ok(Move::None)
    }

    /// The pattern search from every knob at 0, at the quick try count
    /// until a setting is in every band there, then at the full count:
    /// the setting it ends on and why. A setting in every band at the
    /// full count is pruned.
    ///
    /// Each pass of `level` halves the strides once more (a shift by
    /// `level`). A move lowers the distance, so a pass makes at most one
    /// move per evaluation; the passes and the moves are bounded loops,
    /// so the search ends whatever its comparisons say.
    fn pattern(&mut self) -> Result<(Vec<i32>, Stop), PlaytestError<R>> {
        let full = self.full();
        let mut count = self.settings.quick;
        let mut point = vec![0; self.knobs.len()];
        let bases: Vec<i32> = self
            .knobs
            .iter()
            .map(|k| i32::try_from(k.stride()).unwrap_or(i32::MAX))
            .collect();
        let Eval::Distance(mut here) = self.eval(&point, count)? else {
            return Ok((point, Stop::Budget));
        };
        for level in 0..u32::BITS {
            let strides: Vec<i32> = bases.iter().map(|&b| (b >> level).max(1)).collect();
            for _ in 0..=self.settings.evaluations {
                if here == 0 {
                    match self.eval(&point, full)? {
                        Eval::Distance(0) => return Ok((self.prune(point)?, Stop::InBands)),
                        Eval::Distance(d) => {
                            count = full;
                            here = d;
                        }
                        Eval::Inapplicable | Eval::Budget => return Ok((point, Stop::Budget)),
                    }
                }
                match self.round(&mut point, &strides, count, here)? {
                    Move::Better(d) => here = d,
                    Move::None => break,
                    Move::Budget => return Ok((point, Stop::Budget)),
                }
            }
            if strides.iter().all(|&s| s == 1) {
                break;
            }
        }
        Ok((point, Stop::Stuck))
    }

    /// `point`, in every band at the full count, with each changed knob
    /// in order tried at 0, then nearer 0 one step at a time, keeping
    /// each change that stays in every band.
    fn prune(&mut self, mut point: Vec<i32>) -> Result<Vec<i32>, PlaytestError<R>> {
        for k in 0..point.len() {
            let step = point[k];
            let mut trial = point.clone();
            trial[k] = 0;
            if self.in_bands(&trial)? {
                point = trial;
                continue;
            }
            for nearer in (1..step.unsigned_abs()).rev() {
                trial[k] = step.signum() * i32::try_from(nearer).unwrap_or(i32::MAX);
                if !self.in_bands(&trial)? {
                    break;
                }
                point.clone_from(&trial);
            }
        }
        Ok(point)
    }

    /// What the search found, ending on `point`. A setting no nearer its
    /// bands at the full count than the stage as authored is dropped.
    fn finish(
        mut self,
        point: &[i32],
        before: PlaytestReport,
        stop: Stop,
    ) -> Result<Balanced<S>, PlaytestError<R>> {
        let full = self.full();
        let origin = vec![0; self.knobs.len()];
        let found = match self.eval(point, full)? {
            Eval::Distance(d) if d < distance(&before) => self
                .build(point)
                .zip(self.reports.get(point).cloned())
                .map(|(stage, after)| (point.to_vec(), stage, after)),
            _ => None,
        };
        let (point, stage, after) =
            found.unwrap_or_else(|| (origin, self.stage.clone(), before.clone()));
        let changes = self
            .knobs
            .iter()
            .zip(&point)
            .filter(|&(_, &step)| step != 0)
            .map(|(knob, &step)| Change {
                knob: knob.name().to_string(),
                step,
                what: knob.describe(step),
            })
            .collect();
        Ok(Balanced {
            stage,
            changes,
            before,
            after,
            stop,
            evaluations: self.used,
        })
    }
}

/// Changes `stage` with `knobs` until every rate check of `checks` puts
/// every one of `entrants` inside its band, as `settings` says. The
/// entrants and the checks are never changed: only the stage is.
///
/// All deterministic:
/// 1. **Before:** the stage as authored plays at `settings.tries`. In
///    every band already, it stops ([`Stop::AlreadyInBands`]).
/// 2. **Pattern search** at `settings.quick` tries, from every knob at 0,
///    each knob's stride its [`Knob::stride`]: in knob order, `+stride`
///    then `-stride` (clamped to the range, skipping a step that doesn't
///    move or that a knob can't apply). The first step that lowers the
///    [`distance`] is taken and the knob order starts again. When none
///    does, every stride above 1 is halved; when every stride is 1, it
///    stops ([`Stop::Stuck`]).
/// 3. **Confirm:** a setting in every band at `quick` plays at `tries`.
///    In every band there too, it is pruned; if not, the pattern search
///    carries on at `tries` from there.
/// 4. **Prune:** each changed knob in order is tried at 0, then nearer 0
///    one step at a time, keeping each change that stays in every band
///    at `tries` ([`Stop::InBands`]). What is left is the smallest change
///    set this search found, not one proved the smallest.
/// 5. **Budget:** every run of the playtest runner is one evaluation, and
///    a setting is never run twice at one try count. When the
///    `evaluations` run out, it stops ([`Stop::Budget`]) with the best
///    setting seen, kept back one evaluation to play it at `tries`.
///
/// A setting that ends no nearer its bands at `tries` than the stage as
/// authored is dropped: the result is then the stage as authored.
///
/// A check naming an entrant not in the run fails before any try is
/// played; a refused command stops the search.
pub fn autobalance<S: Clone, R: Sides>(
    stage: &S,
    knobs: &[&dyn Knob<S>],
    entrants: &[&dyn Entrant<S, R>],
    checks: &[Check],
    settings: &BalanceSettings,
) -> Result<Balanced<S>, PlaytestError<R>>
where
    R::Command: Clone,
    R::Outcome: fmt::Debug,
{
    let mut search = Search {
        stage,
        knobs,
        entrants,
        checks,
        settings,
        distances: BTreeMap::new(),
        reports: BTreeMap::new(),
        used: 0,
    };
    let before = playtest::run(stage, entrants, &settings.tries, checks)?;
    search.used = 1;
    let origin = vec![0; knobs.len()];
    let (point, stop) = if search.record(&origin, settings.tries.count, before.clone()) == 0 {
        (origin, Stop::AlreadyInBands)
    } else {
        search.pattern()?
    };
    search.finish(&point, before, stop)
}

#[cfg(test)]
mod tests;
