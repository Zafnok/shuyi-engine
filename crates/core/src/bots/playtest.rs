//! A playtest runner: several named bots ([`Entrant`]s) each play a stage
//! K times from the same seeds, the game measures every try
//! ([`Measure`]), and data-file [`Check`]s say per bot whether it
//! succeeded about as often as the designer wants: ✓, ✗ too hard or
//! ⚠ too easy.
//!
//! [`run`] does it all. It is two halves, [`play_tries`] and [`judge`], so
//! a caller with threads can play seed ranges on threads of its own and
//! judge them once: the report never depends on how the seeds were split.
//!
//! Every number is an integer, so a report is the same on every machine:
//! a band compares `successes * 100` with `min * count`, and the median
//! of an even count is the lower middle value.

use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::fmt::{self, Write as _};
use core::ops::Range;

use serde::{Deserialize, Serialize};

use super::{Bot, PlayError, Report, Sides, play_observed};

/// The game's measures of one try, by name. A success or a loss is a
/// measure too (`"won"` 1 or 0), so every check reads measures only.
pub type Measures = BTreeMap<String, i64>;

/// A game's measures of a try, watching it command by command.
pub trait Measure<R: Sides> {
    /// Called for every applied command: the game after it, the command
    /// and the events it gave.
    fn observe(&mut self, rules: &R, command: &R::Command, events: &[R::Event]);

    /// The measures of the try, once it has stopped at `end`.
    fn finish(self: Box<Self>, report: &Report<R>, end: &R) -> Measures;
}

/// One named bot as the runner plays it, over the stage definition `S`
/// (the data a stage is built from).
pub trait Entrant<S, R: Sides> {
    /// Its name, which checks and the report use.
    fn name(&self) -> &str;

    /// The stage as this entrant plays it (a game with modes picks this
    /// entrant's), its luck from `seed`.
    fn start(&self, stage: &S, seed: u64) -> R;

    /// One bot per side for the try of `seed`.
    fn bots(&self, seed: u64) -> Vec<(R::Side, Box<dyn Bot<R>>)>;

    /// A fresh measure for a try.
    fn measure(&self) -> Box<dyn Measure<R>>;
}

/// K tries from seeds `seed` to `seed + count - 1` (those that fit in a
/// `u64`), each stopped at turn `turns` if it hasn't ended. Every entrant
/// plays the same seeds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Tries {
    /// How many tries.
    pub count: u32,
    /// The first try's seed.
    pub seed: u64,
    /// The turn a try stops at if it hasn't ended.
    pub turns: u32,
}

impl Tries {
    /// The seeds of the tries.
    pub fn seeds(&self) -> Range<u64> {
        self.seed..self.seed.saturating_add(u64::from(self.count))
    }
}

/// One try of one entrant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Tried {
    /// Its seed.
    pub seed: u64,
    /// How the game ended, as the outcome's `Debug` writes it; `None` if
    /// the turn limit stopped it.
    pub outcome: Option<String>,
    /// The turn it stopped on (past the limit if the limit stopped it).
    pub turns: u32,
    /// The game's measures of it.
    pub measures: Measures,
}

/// A measure within bounds, both ends inclusive. A try missing the
/// measure fails it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Condition {
    /// The measure's name.
    pub measure: String,
    /// Its least value, if it has one.
    #[serde(default)]
    pub min: Option<i64>,
    /// Its greatest value, if it has one.
    #[serde(default)]
    pub max: Option<i64>,
}

impl Condition {
    /// Whether `measures` meet it.
    pub fn met_by(&self, measures: &Measures) -> bool {
        measures.get(&self.measure).is_some_and(|&value| {
            self.min.is_none_or(|min| value >= min) && self.max.is_none_or(|max| value <= max)
        })
    }
}

/// Whether `measures` meet every one of `conditions`.
fn meets(conditions: &[Condition], measures: &Measures) -> bool {
    conditions.iter().all(|c| c.met_by(measures))
}

/// A share of tries in percent, both ends inclusive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Band {
    /// The least share, in percent.
    pub min: u32,
    /// The greatest share, in percent.
    pub max: u32,
}

/// Where a share of tries falls against a [`Band`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum RateVerdict {
    /// Under the band: too hard (✗).
    Below,
    /// In the band (✓).
    Within,
    /// Over the band: too easy (⚠).
    Above,
}

impl Band {
    /// Where `successes` of `count` tries fall, compared in integers.
    pub fn verdict(self, successes: u32, count: u32) -> RateVerdict {
        let share = u64::from(successes) * 100;
        if share < u64::from(self.min) * u64::from(count) {
            RateVerdict::Below
        } else if share > u64::from(self.max) * u64::from(count) {
            RateVerdict::Above
        } else {
            RateVerdict::Within
        }
    }
}

/// Which way a median should lean.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum Want {
    /// Strictly lower than the other entrant's.
    Lower,
    /// Strictly higher than the other entrant's.
    Higher,
}

/// How a median compares.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum MedianVerdict {
    /// It leans the way wanted (✓).
    Pass,
    /// It doesn't (✗).
    Fail,
    /// One side has no try to take a median of (–).
    Unknown,
}

/// A check of a run, kept as data.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum Check {
    /// The share of `entrant`'s tries that meet every `success` condition
    /// is within `band`. A share under `floor` percent blocks the run.
    Rate {
        /// The entrant checked.
        entrant: String,
        /// What a success is.
        success: Vec<Condition>,
        /// The share wanted.
        band: Band,
        /// The share under which the run is blocked, in percent.
        #[serde(default)]
        floor: Option<u32>,
    },
    /// The median of `measure` over `entrant`'s tries that meet `among`
    /// is lower or higher than the same median of entrant `than`.
    Median {
        /// The entrant checked.
        entrant: String,
        /// The measure.
        measure: String,
        /// Which tries count.
        #[serde(default)]
        among: Vec<Condition>,
        /// Which way it should lean.
        want: Want,
        /// The entrant compared with.
        than: String,
    },
}

impl Check {
    /// The entrants it names.
    fn entrants(&self) -> Vec<&str> {
        match self {
            Check::Rate { entrant, .. } => alloc::vec![entrant.as_str()],
            Check::Median { entrant, than, .. } => alloc::vec![entrant.as_str(), than.as_str()],
        }
    }
}

/// What a check found.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum Found {
    /// A [`Check::Rate`]'s numbers.
    Rate {
        /// The tries that succeeded.
        successes: u32,
        /// The tries.
        count: u32,
        /// Where the share falls.
        verdict: RateVerdict,
        /// Whether it is under the floor.
        blocked: bool,
    },
    /// A [`Check::Median`]'s numbers.
    Median {
        /// The entrant's median, if it has a try to take it of.
        median: Option<i64>,
        /// The other entrant's.
        than: Option<i64>,
        /// How they compare.
        verdict: MedianVerdict,
    },
}

/// A check and what it found.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Judged {
    /// The check.
    pub check: Check,
    /// What it found.
    pub found: Found,
}

/// One entrant's tries, in seed order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct EntrantTries {
    /// Its name.
    pub name: String,
    /// Its tries.
    pub tries: Vec<Tried>,
}

/// What a playtest found. Its `Display` is the table people read; a game
/// adds its own lines from the data.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct PlaytestReport {
    /// Every entrant's tries, in the order given.
    pub entrants: Vec<EntrantTries>,
    /// Every check, in the order given, and what it found.
    pub checks: Vec<Judged>,
    /// Whether a rate fell under its floor.
    pub blocked: bool,
}

/// A check names an entrant that isn't in the run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownEntrant {
    /// The check's place in the list, from 0.
    pub check: usize,
    /// The name.
    pub entrant: String,
}

impl fmt::Display for UnknownEntrant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "check {} names `{}`, who isn't in the run",
            self.check + 1,
            self.entrant
        )
    }
}

/// Why a playtest stopped.
pub enum PlaytestError<R: Sides> {
    /// A check names an entrant that isn't in the run.
    UnknownEntrant(UnknownEntrant),
    /// A bot's command was refused: a bug in a bot, never skipped.
    Play {
        /// The entrant whose bot it was.
        entrant: String,
        /// The refusal.
        error: PlayError<R>,
    },
}

impl<R: Sides> From<UnknownEntrant> for PlaytestError<R> {
    fn from(e: UnknownEntrant) -> Self {
        PlaytestError::UnknownEntrant(e)
    }
}

impl<R: Sides> fmt::Debug for PlaytestError<R>
where
    R::Command: fmt::Debug,
    R::Error: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PlaytestError::UnknownEntrant(e) => f.debug_tuple("UnknownEntrant").field(e).finish(),
            PlaytestError::Play { entrant, error } => f
                .debug_struct("Play")
                .field("entrant", entrant)
                .field("error", error)
                .finish(),
        }
    }
}

impl<R: Sides> fmt::Display for PlaytestError<R>
where
    R::Command: fmt::Debug,
    R::Error: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PlaytestError::UnknownEntrant(e) => e.fmt(f),
            PlaytestError::Play { entrant, error } => write!(f, "{entrant}: {error}"),
        }
    }
}

/// The first check that names an entrant not in `names`.
fn unknown_entrant(names: &[&str], checks: &[Check]) -> Option<UnknownEntrant> {
    checks.iter().enumerate().find_map(|(i, check)| {
        check
            .entrants()
            .into_iter()
            .find(|name| !names.contains(name))
            .map(|name| UnknownEntrant {
                check: i,
                entrant: name.to_string(),
            })
    })
}

/// `entrant` plays `stage` once from each of `seeds`, each try stopped at
/// turn `turns`: its tries in seed order.
pub fn play_tries<S, R: Sides>(
    stage: &S,
    entrant: &dyn Entrant<S, R>,
    seeds: Range<u64>,
    turns: u32,
) -> Result<Vec<Tried>, PlayError<R>>
where
    R::Command: Clone,
    R::Outcome: fmt::Debug,
{
    seeds
        .map(|seed| {
            let mut rules = entrant.start(stage, seed);
            let mut bots = entrant.bots(seed);
            let mut by_side: Vec<(R::Side, &mut dyn Bot<R>)> = bots
                .iter_mut()
                .map(|(side, bot)| (*side, bot.as_mut() as &mut dyn Bot<R>))
                .collect();
            let mut measure = entrant.measure();
            let report = play_observed(&mut rules, &mut by_side, turns, |r, c, e| {
                measure.observe(r, c, e);
            })?;
            Ok(Tried {
                seed,
                outcome: report.outcome.as_ref().map(|o| format!("{o:?}")),
                turns: report.turns,
                measures: measure.finish(&report, &rules),
            })
        })
        .collect()
}

/// The median of `values`: the lower middle one of an even count.
fn median(mut values: Vec<i64>) -> Option<i64> {
    values.sort_unstable();
    let middle = values.len().checked_sub(1)? / 2;
    values.get(middle).copied()
}

/// The median of `measure` over `tries` that meet `among` (a try missing
/// the measure doesn't count).
fn median_of(tries: &[Tried], measure: &str, among: &[Condition]) -> Option<i64> {
    median(
        tries
            .iter()
            .filter(|t| meets(among, &t.measures))
            .filter_map(|t| t.measures.get(measure).copied())
            .collect(),
    )
}

/// The tries of every entrant (its name and its tries, in any seed
/// order, in any number of parts) judged by `checks`.
pub fn judge(
    tries: Vec<(String, Vec<Tried>)>,
    checks: &[Check],
) -> Result<PlaytestReport, UnknownEntrant> {
    let mut entrants: Vec<EntrantTries> = Vec::new();
    for (name, mut part) in tries {
        if let Some(e) = entrants.iter_mut().find(|e| e.name == name) {
            e.tries.append(&mut part);
        } else {
            entrants.push(EntrantTries { name, tries: part });
        }
    }
    for e in &mut entrants {
        e.tries.sort_by_key(|t| t.seed);
    }
    let names: Vec<&str> = entrants.iter().map(|e| e.name.as_str()).collect();
    if let Some(e) = unknown_entrant(&names, checks) {
        return Err(e);
    }
    let of = |name: &str| -> &[Tried] {
        entrants
            .iter()
            .find(|e| e.name == name)
            .map_or(&[], |e| &e.tries)
    };
    let checks: Vec<Judged> = checks
        .iter()
        .map(|check| {
            let found = match check {
                Check::Rate {
                    entrant,
                    success,
                    band,
                    floor,
                } => {
                    let tries = of(entrant);
                    let count = u32::try_from(tries.len()).unwrap_or(u32::MAX);
                    let successes = tries.iter().filter(|t| meets(success, &t.measures)).count();
                    let successes = u32::try_from(successes).unwrap_or(u32::MAX);
                    let blocked = floor.is_some_and(|floor| {
                        u64::from(successes) * 100 < u64::from(floor) * u64::from(count)
                    });
                    Found::Rate {
                        successes,
                        count,
                        verdict: band.verdict(successes, count),
                        blocked,
                    }
                }
                Check::Median {
                    entrant,
                    measure,
                    among,
                    want,
                    than,
                } => {
                    let median = median_of(of(entrant), measure, among);
                    let other = median_of(of(than), measure, among);
                    let verdict = match (median, other, want) {
                        (Some(m), Some(o), Want::Lower) if m < o => MedianVerdict::Pass,
                        (Some(m), Some(o), Want::Higher) if m > o => MedianVerdict::Pass,
                        (Some(_), Some(_), _) => MedianVerdict::Fail,
                        _ => MedianVerdict::Unknown,
                    };
                    Found::Median {
                        median,
                        than: other,
                        verdict,
                    }
                }
            };
            Judged {
                check: check.clone(),
                found,
            }
        })
        .collect();
    let blocked = checks
        .iter()
        .any(|j| matches!(j.found, Found::Rate { blocked: true, .. }));
    Ok(PlaytestReport {
        entrants,
        checks,
        blocked,
    })
}

/// Every one of `entrants` plays `stage` as `tries` says, and `checks`
/// judge their tries. A check naming an entrant not in the run fails
/// before any try is played; a refused command stops the run.
pub fn run<S, R: Sides>(
    stage: &S,
    entrants: &[&dyn Entrant<S, R>],
    tries: &Tries,
    checks: &[Check],
) -> Result<PlaytestReport, PlaytestError<R>>
where
    R::Command: Clone,
    R::Outcome: fmt::Debug,
{
    let names: Vec<&str> = entrants.iter().map(|e| e.name()).collect();
    if let Some(e) = unknown_entrant(&names, checks) {
        return Err(e.into());
    }
    let mut played = Vec::new();
    for entrant in entrants {
        let name = entrant.name().to_string();
        match play_tries(stage, *entrant, tries.seeds(), tries.turns) {
            Ok(t) => played.push((name, t)),
            Err(error) => {
                return Err(PlaytestError::Play {
                    entrant: name,
                    error,
                });
            }
        }
    }
    Ok(judge(played, checks)?)
}

/// `n` or a dash.
fn or_dash(n: Option<i64>) -> String {
    n.map_or_else(|| "–".to_string(), |n| n.to_string())
}

impl PlaytestReport {
    /// A check's line: what it measured, and how it went.
    fn line(judged: &Judged, width: usize) -> (String, String) {
        match (&judged.check, &judged.found) {
            (
                Check::Rate {
                    entrant,
                    band,
                    floor,
                    ..
                },
                Found::Rate {
                    successes,
                    count,
                    verdict,
                    blocked,
                },
            ) => {
                let mark = match verdict {
                    RateVerdict::Within => "✓",
                    RateVerdict::Below => "✗ too hard",
                    RateVerdict::Above => "⚠ too easy",
                };
                let mut how = format!("band {}–{}%   {mark}", band.min, band.max);
                if *blocked {
                    // Writing to a `String` can't fail.
                    let _ = write!(how, " · blocked: under the {}% floor", floor.unwrap_or(0));
                }
                (
                    format!("{entrant:<width$} · {successes}/{count} succeed"),
                    how,
                )
            }
            (
                Check::Median {
                    entrant,
                    measure,
                    want,
                    than: other,
                    ..
                },
                Found::Median {
                    median,
                    than,
                    verdict,
                },
            ) => {
                let mark = match verdict {
                    MedianVerdict::Pass => "✓",
                    MedianVerdict::Fail => "✗",
                    MedianVerdict::Unknown => "–",
                };
                let want = match want {
                    Want::Lower => "lower",
                    Want::Higher => "higher",
                };
                (
                    format!(
                        "{entrant:<width$} · {measure} median {} ({other} {})",
                        or_dash(*median),
                        or_dash(*than)
                    ),
                    format!("{want} {mark}"),
                )
            }
            // A check with another check's numbers: only a hand-made report.
            (check, found) => (format!("{check:?}"), format!("{found:?}")),
        }
    }
}

/// The table: how many tries from which seeds, a line per check, then
/// every try of every entrant.
impl fmt::Display for PlaytestReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let first = self.entrants.first().map_or(&[][..], |e| &e.tries);
        match (first.first(), first.last()) {
            (Some(a), Some(b)) => {
                writeln!(f, "{} tries (seeds {}–{})", first.len(), a.seed, b.seed)?;
            }
            _ => writeln!(f, "0 tries")?,
        }
        let width = self
            .checks
            .iter()
            .map(|j| match &j.check {
                Check::Rate { entrant, .. } | Check::Median { entrant, .. } => {
                    entrant.chars().count()
                }
            })
            .max()
            .unwrap_or(0);
        let lines: Vec<(String, String)> =
            self.checks.iter().map(|j| Self::line(j, width)).collect();
        let left = lines
            .iter()
            .map(|(l, _)| l.chars().count())
            .max()
            .unwrap_or(0);
        if !lines.is_empty() {
            writeln!(f)?;
        }
        for (what, how) in &lines {
            let pad = left - what.chars().count() + 4;
            writeln!(f, "{what}{:pad$}{how}", "")?;
        }
        if self.entrants.iter().any(|e| !e.tries.is_empty()) {
            writeln!(f)?;
        }
        for entrant in &self.entrants {
            for t in &entrant.tries {
                write!(
                    f,
                    "try {} · {} · {} · turns {}",
                    t.seed,
                    entrant.name,
                    t.outcome.as_deref().unwrap_or("unfinished"),
                    t.turns
                )?;
                for (name, value) in &t.measures {
                    write!(f, " · {name} {value}")?;
                }
                writeln!(f)?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
