//! A rollout bot over any [`Determinize`] game: flat Monte Carlo.
//!
//! [`Rollout`] tries each of a few good-looking commands many times on
//! copies of the game where the luck and the hidden things are redrawn
//! ([`Determinize`]), plays each copy on a little way with simple players
//! ([`Playout`]), and picks the command whose copies ended best
//! ([`Evaluate`]). It searches one level, its own next command, so it
//! suits games that branch too much for a tree of every side's replies.
//!
//! Everything is in integers: the scores, their means and the softmax that
//! picks (base 2, the temperature in points). It thinks in work units,
//! never time: a unit is one command applied to a copy. Its own randomness
//! comes from its seed, drawn only at a decision's start and its end, so
//! how the work is sliced ([`RolloutSearch::advance`]) never changes the
//! command (ADR-0027).

use alloc::vec::Vec;

use serde::{Deserialize, Serialize};

use super::Bot;
use super::Sides;
use super::determinize::{Determinize, Evaluate};
use crate::rng::SimRng;

/// The game's list of candidates, from the real state's
/// [`legal_commands`](crate::TurnRules::legal_commands): it may drop commands
/// (ones the game never wants tried) and puts the most promising first.
/// It returns only commands from the list it is given.
pub trait Order<R: Sides> {
    /// `commands` (the legal commands of `rules`), dropped and reordered.
    fn order(&mut self, rules: &R, commands: Vec<R::Command>) -> Vec<R::Command>;
}

impl<R: Sides, F: FnMut(&R, Vec<R::Command>) -> Vec<R::Command>> Order<R> for F {
    fn order(&mut self, rules: &R, commands: Vec<R::Command>) -> Vec<R::Command> {
        self(rules, commands)
    }
}

/// The legal commands as the game lists them.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Unordered;

impl<R: Sides> Order<R> for Unordered {
    fn order(&mut self, _: &R, commands: Vec<R::Command>) -> Vec<R::Command> {
        commands
    }
}

/// How a copy is played on after the candidate, for whichever side is to
/// move: a game matches on [`side_to_move`](Sides::side_to_move) to use
/// its baseline for one side and its enemy mind for another.
///
/// Its randomness, if any, comes from the copy (whose luck was redrawn),
/// so it needs no seed of its own. Every [`Bot`] is one.
pub trait Playout<R: Sides> {
    /// The next command in the copy `rules`, which is not over.
    fn command(&mut self, rules: &R) -> R::Command;
}

impl<R: Sides, B: Bot<R>> Playout<R> for B {
    fn command(&mut self, rules: &R) -> R::Command {
        self.choose(rules)
    }
}

/// When a playout stops, short of the game's end.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Horizon {
    /// After the side to move has changed this many times (the
    /// candidate's own change included).
    Sides(u32),
    /// After [`Sides::turn`] has gone up this many.
    Turns(u32),
}

/// How a [`Rollout`] bot thinks: data, so a persona's settings can be a
/// file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RolloutSettings {
    /// How many of the ordered commands are tried, from the first. 0
    /// counts as 1.
    pub candidates: u32,
    /// How many copies each candidate is played out on. 0 counts as 1.
    pub samples: u32,
    /// When a playout stops.
    pub horizon: Horizon,
    /// The most commands a playout applies after the candidate, so a
    /// playout player that never ends its side's move can't run forever.
    pub commands: u32,
    /// How far from the best the pick may stray, in points: a candidate
    /// this many points below the best is half as likely. 0 always picks
    /// the best.
    pub temperature: u32,
}

/// A computer player that plans on copies of the game: flat Monte Carlo
/// (the module's docs).
///
/// A decision does at most `candidates × samples × (commands + 1)` units
/// of work, a unit being one command applied to a copy.
#[derive(Debug, Clone)]
pub struct Rollout<O, P, E> {
    order: O,
    playout: P,
    eval: E,
    settings: RolloutSettings,
    rng: SimRng,
}

impl<O, P, E> Rollout<O, P, E> {
    /// A rollout bot whose candidates come from `order`, played out by
    /// `playout` and scored by `eval`, thinking as `settings` says. Its
    /// choices follow from `seed`.
    pub fn new(order: O, playout: P, eval: E, settings: RolloutSettings, seed: u64) -> Self {
        Self {
            order,
            playout,
            eval,
            settings,
            rng: SimRng::new(seed),
        }
    }

    /// How it thinks.
    pub fn settings(&self) -> RolloutSettings {
        self.settings
    }

    /// Starts a decision in `rules`, which is not over: its candidates are
    /// listed and its seeds drawn. [`RolloutSearch::advance`] does the
    /// work.
    ///
    /// # Panics
    ///
    /// If the game lists no legal command, or the order drops them all.
    pub fn search<R>(&mut self, rules: &R) -> RolloutSearch<'_, R, O, P, E>
    where
        R: Determinize,
        R::Command: Clone + PartialEq,
        O: Order<R>,
        P: Playout<R>,
        E: Evaluate<R>,
    {
        let legal = rules.legal_commands();
        let mut candidates = self.order.order(rules, legal.clone());
        candidates.truncate(at_least_one(self.settings.candidates));
        assert!(
            !candidates.is_empty(),
            "no command to choose from: the game lists none, or its order dropped them all"
        );
        let seeds = draw_seeds(&mut self.rng, self.settings.samples);
        RolloutSearch {
            root: rules.clone(),
            side: rules.side_to_move(),
            legal,
            sums: alloc::vec![0; candidates.len()],
            candidates,
            seeds,
            next: 0,
            playing: None,
            chosen: None,
            bot: self,
        }
    }
}

impl<R, O, P, E> Bot<R> for Rollout<O, P, E>
where
    R: Determinize,
    R::Command: Clone + PartialEq,
    O: Order<R>,
    P: Playout<R>,
    E: Evaluate<R>,
{
    /// The command whose playouts ended best (or, at a temperature, one
    /// picked by the softmax).
    ///
    /// # Panics
    ///
    /// As [`Rollout::search`] and [`RolloutSearch::advance`] do.
    fn choose(&mut self, rules: &R) -> R::Command {
        let mut search = self.search(rules);
        loop {
            if let Some(command) = search.advance(u32::MAX) {
                return command;
            }
        }
    }
}

/// A decision of a [`Rollout`] bot under way: its loop state in a value,
/// so the work can be split over many calls (frames, later).
pub struct RolloutSearch<'a, R: Sides, O, P, E> {
    bot: &'a mut Rollout<O, P, E>,
    /// The state the decision began in.
    root: R,
    side: R::Side,
    /// The root's legal commands, which every copy must list too.
    legal: Vec<R::Command>,
    candidates: Vec<R::Command>,
    seeds: Vec<u64>,
    /// Each candidate's sum of scores.
    sums: Vec<i64>,
    /// The next playout to start: candidate `next / seeds.len()`, on seed
    /// `next % seeds.len()`.
    next: usize,
    playing: Option<Playing<R>>,
    chosen: Option<R::Command>,
}

/// A playout under way.
struct Playing<R: Sides> {
    candidate: usize,
    copy: R,
    side: R::Side,
    /// How many times the side to move has changed since the root.
    changes: u32,
    /// The commands applied after the candidate (refused ones included).
    played: u32,
}

impl<R, O, P, E> RolloutSearch<'_, R, O, P, E>
where
    R: Determinize,
    R::Command: Clone + PartialEq,
    O: Order<R>,
    P: Playout<R>,
    E: Evaluate<R>,
{
    /// Does at most `units` units of work (commands applied to copies) and
    /// returns the chosen command once the decision is made, `None` while
    /// it is working. Once made, it returns the same command again.
    ///
    /// # Panics
    ///
    /// With a message naming `determinize` when a copy lists other
    /// commands than the real state, or refuses a candidate: a bug in the
    /// game's [`Determinize::determinize`].
    pub fn advance(&mut self, units: u32) -> Option<R::Command> {
        let mut left = units;
        loop {
            if let Some(chosen) = &self.chosen {
                return Some(chosen.clone());
            }
            let Some(mut playing) = self.playing.take() else {
                if self.next == self.candidates.len() * self.seeds.len() {
                    self.chosen = Some(self.pick());
                } else if left == 0 {
                    return None;
                } else {
                    left -= 1;
                    self.playing = Some(self.start());
                }
                continue;
            };
            if self.over(&playing) {
                self.finish(&playing);
            } else if left == 0 {
                self.playing = Some(playing);
                return None;
            } else {
                left -= 1;
                let command = self.bot.playout.command(&playing.copy);
                playing.played += 1;
                if playing.copy.apply(&command).is_ok() {
                    note_side(&mut playing);
                    self.playing = Some(playing);
                } else {
                    self.finish(&playing);
                }
            }
        }
    }

    /// The next playout: a copy from `determinize`, checked, with its
    /// candidate applied.
    fn start(&mut self) -> Playing<R> {
        let candidate = self.next / self.seeds.len();
        let seed = self.seeds[self.next % self.seeds.len()];
        self.next += 1;
        let mut copy = checked_copy(&self.root, self.side, &self.legal, seed);
        apply_candidate(&mut copy, &self.candidates[candidate], candidate, seed);
        let mut playing = Playing {
            candidate,
            copy,
            side: self.side,
            changes: 0,
            played: 0,
        };
        note_side(&mut playing);
        playing
    }

    /// Whether `playing` is done: the game ended, the horizon is reached,
    /// or it applied its `commands`.
    fn over(&self, playing: &Playing<R>) -> bool {
        let settings = self.bot.settings;
        let reached = match settings.horizon {
            Horizon::Sides(changes) => playing.changes >= changes,
            Horizon::Turns(turns) => playing.copy.turn().saturating_sub(self.root.turn()) >= turns,
        };
        reached || playing.played >= settings.commands || playing.copy.outcome().is_some()
    }

    /// Scores a finished playout into its candidate's sum.
    fn finish(&mut self, playing: &Playing<R>) {
        let score = self.bot.eval.score(&self.root, &playing.copy, self.side);
        let sum = &mut self.sums[playing.candidate];
        *sum = sum.saturating_add(i64::from(score));
    }

    /// The decision: each candidate's mean score, then the pick.
    fn pick(&mut self) -> R::Command {
        let samples = i64::try_from(self.seeds.len()).unwrap_or(i64::MAX);
        let means: Vec<i64> = self
            .sums
            .iter()
            .map(|sum| sum.div_euclid(samples))
            .collect();
        let temperature = self.bot.settings.temperature;
        let index = pick(&means, temperature, &mut self.bot.rng);
        self.candidates[index].clone()
    }
}

/// Counts a change of the side to move in `playing`.
fn note_side<R: Sides>(playing: &mut Playing<R>) {
    let side = playing.copy.side_to_move();
    if side != playing.side {
        playing.side = side;
        playing.changes = playing.changes.saturating_add(1);
    }
}

/// `samples` seeds for a decision's copies (at least 1), each two draws
/// from `rng`, high half first.
pub(super) fn draw_seeds(rng: &mut SimRng, samples: u32) -> Vec<u64> {
    (0..at_least_one(samples))
        .map(|_| {
            let high = u64::from(rng.next_u32());
            let low = u64::from(rng.next_u32());
            (high << 32) + low
        })
        .collect()
}

/// `root`'s copy for `side` from `seed`, checked to list `legal` (the
/// root's legal commands) with `side` to move.
///
/// # Panics
///
/// With a message naming `determinize` when it doesn't.
pub(super) fn checked_copy<R: Determinize>(
    root: &R,
    side: R::Side,
    legal: &[R::Command],
    seed: u64,
) -> R
where
    R::Command: PartialEq,
{
    let copy = root.determinize(side, seed);
    assert!(
        copy.side_to_move() == side && copy.legal_commands() == legal,
        "determinize changed the commands at the root (seed {seed}): a copy must list \
         the state's legal commands, in the same order"
    );
    copy
}

/// Applies the root's candidate number `index` to a root's `copy` made
/// from `seed`.
///
/// # Panics
///
/// With a message naming `determinize` when the copy refuses it.
pub(super) fn apply_candidate<R: Determinize>(
    copy: &mut R,
    candidate: &R::Command,
    index: usize,
    seed: u64,
) {
    let applied = copy.apply(candidate).is_ok();
    assert!(
        applied,
        "the copy from determinize refused candidate {index} (seed {seed}): a copy must \
         accept the state's legal commands"
    );
}

/// `n`, but at least 1, as a length.
pub(super) fn at_least_one(n: u32) -> usize {
    usize::try_from(n.max(1)).unwrap_or(usize::MAX)
}

/// The index `means` pick: at `temperature` 0 the best (the first among
/// equals); otherwise a base-2 softmax, a mean `temperature` points below
/// the best weighing half as much, and one draw from `rng`.
pub(super) fn pick(means: &[i64], temperature: u32, rng: &mut SimRng) -> usize {
    let best = means.iter().copied().max().unwrap_or(0);
    let best_index = means.iter().position(|&mean| mean == best).unwrap_or(0);
    if temperature == 0 {
        return best_index;
    }
    let weights: Vec<u64> = means
        .iter()
        .map(|&mean| {
            let below = (best - mean).saturating_mul(1 << 16) / i64::from(temperature);
            u64::from(exp2_q16(-below))
        })
        .collect();
    let shift = fit_shift(weights.iter().sum());
    let weights: Vec<u64> = weights.iter().map(|weight| weight >> shift).collect();
    let total = u32::try_from(weights.iter().sum::<u64>()).unwrap_or(u32::MAX);
    share(&weights, u64::from(rng.roll_below(total))).unwrap_or(best_index)
}

/// How many halvings bring `total` into a `u32`, for one draw.
fn fit_shift(total: u64) -> u32 {
    u64::BITS - (total >> 32).leading_zeros()
}

/// The index whose share of `weights`, laid end to end, holds `roll`;
/// `None` past their sum.
fn share(weights: &[u64], mut roll: u64) -> Option<usize> {
    for (index, &weight) in weights.iter().enumerate() {
        if roll < weight {
            return Some(index);
        }
        roll -= weight;
    }
    None
}

/// `2^(-k/16)` in 16.16 fixed point, for `k` in `0..16`.
const EXP2_FRACTIONS: [u32; 16] = [
    65536, 62757, 60097, 57549, 55109, 52773, 50535, 48393, 46341, 44376, 42495, 40693, 38968,
    37316, 35734, 34219,
];

/// `2^x` in 16.16 fixed point (`1 << 16` is 1), for `x` in 16.16 at most
/// 0; a positive `x` counts as 0. The whole part is a shift, the fraction
/// comes from a 16-entry table, read straight between its entries. More
/// than 32 halvings down is 0.
pub fn exp2_q16(x: i64) -> u32 {
    let down = x.min(0).unsigned_abs();
    let halvings = u32::try_from(down >> 16).unwrap_or(u32::MAX);
    let fraction = down & 0xFFFF;
    let entry = usize::try_from(fraction >> 12).unwrap_or(0);
    let high = u64::from(EXP2_FRACTIONS[entry]);
    let low = EXP2_FRACTIONS
        .get(entry + 1)
        .map_or(u64::from(EXP2_FRACTIONS[0]) >> 1, |&next| u64::from(next));
    let value = high - (((high - low) * (fraction & 0xFFF)) >> 12);
    let value = value.checked_shr(halvings).unwrap_or(0);
    u32::try_from(value).unwrap_or(0)
}

#[cfg(test)]
mod tests;
