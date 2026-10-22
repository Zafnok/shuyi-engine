//! A turn planner over any [`Determinize`] game: a beam search over its
//! own side's commands.
//!
//! [`TurnPlan`] is for games whose side issues several commands a turn
//! (each of its pieces acts, cards are played one after another). It
//! weighs short *sequences* of its own commands, so a first command that
//! only pays off with a second ("weaken it, then finish it"; a card that
//! draws, then the card it drew) is chosen for what the sequence does,
//! not judged alone. It plays each partial turn on copies of the game
//! where the luck and the hidden things are redrawn ([`Determinize`]),
//! scores where each copy got to ([`Evaluate`]), and keeps the best few
//! at each depth to extend by the game's next candidates ([`Order`]).
//! It never searches the other sides' replies: their danger is the
//! game's [`Evaluate`] to see, as a player would.
//!
//! Every partial turn is played on the same copies, one per seed. A copy
//! stops where it refuses a command (a piece fell there), where its
//! side's turn ends, or where the game ends, and is scored there. A
//! partial turn's candidates are the order of its first copy still
//! playing. A first command is worth the best partial turn that starts
//! with it, itself included.
//!
//! It returns the first command of the best plan and plans again for the
//! next command, keeping nothing between decisions but its random number
//! generator. Its
//! integers, its softmax and its work units are the rollout bot's: a unit
//! is one command applied to a copy, and its own randomness is drawn only
//! at a decision's start and its end, so how the work is sliced
//! ([`PlanSearch::advance`]) never changes the command (ADR-0027,
//! ADR-0029).

use alloc::vec::Vec;
use core::cmp::Reverse;

use serde::{Deserialize, Serialize};

use super::Bot;
use super::Sides;
use super::determinize::{Determinize, Evaluate};
use super::rollout::{Order, apply_candidate, at_least_one, checked_copy, draw_seeds, pick};
use crate::rng::SimRng;

/// How a [`TurnPlan`] bot thinks: data, so a persona's settings can be a
/// file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanSettings {
    /// How many partial turns are kept at each depth, to be extended at
    /// the next: the beam. 0 counts as 1.
    pub width: u32,
    /// How many of the ordered commands are tried from each partial
    /// turn, from the first. 0 counts as 1.
    pub candidates: u32,
    /// The most of its own commands a plan holds; the side's turn may end
    /// sooner. 0 counts as 1.
    pub depth: u32,
    /// How many copies each partial turn is played on. 0 counts as 1.
    pub samples: u32,
    /// How far from the best the first command's pick may stray, in
    /// points: a command this many points below the best is half as
    /// likely. 0 always picks the best.
    pub temperature: u32,
}

/// A computer player that plans its side's turn on copies of the game: a
/// beam search over its own commands (the module's docs).
///
/// A decision does at most `samples × candidates × (1 + width × (depth -
/// 1))` units of work, a unit being one command applied to a copy.
#[derive(Debug, Clone)]
pub struct TurnPlan<O, E> {
    order: O,
    eval: E,
    settings: PlanSettings,
    rng: SimRng,
}

impl<O, E> TurnPlan<O, E> {
    /// A turn planner whose candidates come from `order` and whose partial
    /// turns are scored by `eval`, thinking as `settings` says. Its choices
    /// follow from `seed`.
    pub fn new(order: O, eval: E, settings: PlanSettings, seed: u64) -> Self {
        Self {
            order,
            eval,
            settings,
            rng: SimRng::new(seed),
        }
    }

    /// How it thinks.
    pub fn settings(&self) -> PlanSettings {
        self.settings
    }

    /// Starts a decision in `rules`, which is not over: its first commands
    /// are listed, its seeds drawn and its copies made.
    /// [`PlanSearch::advance`] does the work.
    ///
    /// # Panics
    ///
    /// If the game lists no legal command, or the order drops them all;
    /// with a message naming `determinize` when a copy lists other
    /// commands than the real state.
    pub fn search<R>(&mut self, rules: &R) -> PlanSearch<'_, R, O, E>
    where
        R: Determinize,
        R::Command: Clone + PartialEq,
        O: Order<R>,
        E: Evaluate<R>,
    {
        let legal = rules.legal_commands();
        let mut firsts = self.order.order(rules, legal.clone());
        firsts.truncate(at_least_one(self.settings.candidates));
        assert!(
            !firsts.is_empty(),
            "no command to choose from: the game lists none, or its order dropped them all"
        );
        let side = rules.side_to_move();
        let seeds = draw_seeds(&mut self.rng, self.settings.samples);
        let copies = seeds
            .iter()
            .map(|&seed| Sample {
                state: checked_copy(rules, side, &legal, seed),
                playing: true,
            })
            .collect();
        PlanSearch {
            root: rules.clone(),
            side,
            seeds,
            values: alloc::vec![i64::MIN; firsts.len()],
            parents: alloc::vec![Partial {
                first: None,
                copies,
                candidates: firsts.clone(),
            }],
            firsts,
            children: Vec::new(),
            depth: 1,
            parent: 0,
            candidate: 0,
            building: None,
            chosen: None,
            bot: self,
        }
    }
}

impl<R, O, E> Bot<R> for TurnPlan<O, E>
where
    R: Determinize,
    R::Command: Clone + PartialEq,
    O: Order<R>,
    E: Evaluate<R>,
{
    /// The first command of the best plan (or, at a temperature, one
    /// picked by the softmax).
    ///
    /// # Panics
    ///
    /// As [`TurnPlan::search`] and [`PlanSearch::advance`] do.
    fn choose(&mut self, rules: &R) -> R::Command {
        let mut search = self.search(rules);
        loop {
            if let Some(command) = search.advance(u32::MAX) {
                return command;
            }
        }
    }
}

/// A decision of a [`TurnPlan`] bot under way: its loop state in a value,
/// so the work can be split over many calls (frames, later).
pub struct PlanSearch<'a, R: Sides, O, E> {
    bot: &'a mut TurnPlan<O, E>,
    /// The state the decision began in.
    root: R,
    side: R::Side,
    /// One per copy of each partial turn.
    seeds: Vec<u64>,
    /// The root's candidates: the plans' first commands.
    firsts: Vec<R::Command>,
    /// The best score of a partial turn starting with each first command.
    values: Vec<i64>,
    /// The partial turns being extended: at depth 1, the root alone.
    parents: Vec<Partial<R>>,
    /// The partial turns built at this depth that may be extended, with
    /// their scores.
    children: Vec<(i64, Partial<R>)>,
    /// How many commands the partial turns being built hold.
    depth: usize,
    /// The next partial turn to build: this parent's candidate.
    parent: usize,
    candidate: usize,
    building: Option<Building<R>>,
    chosen: Option<R::Command>,
}

/// A partial turn: its copies, played as far as its commands go.
struct Partial<R: Sides> {
    /// The index of its first command in the root's candidates; `None`
    /// for the root.
    first: Option<usize>,
    /// One per seed.
    copies: Vec<Sample<R>>,
    /// The commands it is extended by, once it is kept.
    candidates: Vec<R::Command>,
}

/// A copy of a partial turn.
struct Sample<R> {
    state: R,
    /// Whether its plan goes on: no command was refused, the turn is
    /// still its side's and the game isn't over.
    playing: bool,
}

/// A partial turn being built, one copy at a time.
struct Building<R: Sides> {
    first: usize,
    copies: Vec<Sample<R>>,
    /// The sum of its copies' scores.
    sum: i64,
}

impl<R, O, E> PlanSearch<'_, R, O, E>
where
    R: Determinize,
    R::Command: Clone + PartialEq,
    O: Order<R>,
    E: Evaluate<R>,
{
    /// Does at most `units` units of work (commands applied to copies) and
    /// returns the chosen command once the decision is made, `None` while
    /// it is working. Once made, it returns the same command again.
    ///
    /// # Panics
    ///
    /// With a message naming `determinize` when a copy of the root refuses
    /// one of its candidates: a bug in the game's
    /// [`Determinize::determinize`].
    pub fn advance(&mut self, units: u32) -> Option<R::Command> {
        let mut left = units;
        loop {
            if let Some(chosen) = &self.chosen {
                return Some(chosen.clone());
            }
            let Some(building) = &self.building else {
                self.start();
                continue;
            };
            let built = building.copies.len();
            if built == self.seeds.len() {
                self.finish();
            } else if !self.parents[self.parent].copies[built].playing {
                self.step();
            } else if left == 0 {
                return None;
            } else {
                left -= 1;
                self.step();
            }
        }
    }

    /// Starts the next partial turn at this depth; at the depth's end,
    /// keeps the best to extend, or makes the decision.
    fn start(&mut self) {
        while self
            .parents
            .get(self.parent)
            .is_some_and(|parent| self.candidate == parent.candidates.len())
        {
            self.parent += 1;
            self.candidate = 0;
        }
        if let Some(parent) = self.parents.get(self.parent) {
            self.building = Some(Building {
                first: parent.first.unwrap_or(self.candidate),
                copies: Vec::with_capacity(self.seeds.len()),
                sum: 0,
            });
        } else if self.children.is_empty() {
            self.chosen = Some(self.pick());
        } else {
            self.deepen();
        }
    }

    /// Builds the next copy of the partial turn under way: its parent's
    /// copy with the candidate applied, if that copy is still playing.
    fn step(&mut self) {
        let Some(building) = &mut self.building else {
            return;
        };
        let index = building.copies.len();
        let parent = &self.parents[self.parent];
        let from = &parent.copies[index];
        let mut state = from.state.clone();
        let mut playing = from.playing;
        if playing {
            let command = &parent.candidates[self.candidate];
            if parent.first.is_none() {
                apply_candidate(&mut state, command, self.candidate, self.seeds[index]);
            } else {
                playing = state.apply(command).is_ok();
            }
            playing = playing && state.side_to_move() == self.side && state.outcome().is_none();
        }
        let score = self.bot.eval.score(&self.root, &state, self.side);
        building.sum = building.sum.saturating_add(i64::from(score));
        building.copies.push(Sample { state, playing });
    }

    /// Scores the partial turn just built into its first command's value,
    /// and keeps it if it may be extended.
    fn finish(&mut self) {
        let Some(building) = self.building.take() else {
            return;
        };
        let samples = i64::try_from(self.seeds.len()).unwrap_or(i64::MAX);
        let mean = building.sum.div_euclid(samples);
        let value = &mut self.values[building.first];
        *value = (*value).max(mean);
        let deeper = self.depth < at_least_one(self.bot.settings.depth);
        if deeper && building.copies.iter().any(|copy| copy.playing) {
            let partial = Partial {
                first: Some(building.first),
                copies: building.copies,
                candidates: Vec::new(),
            };
            self.children.push((mean, partial));
        }
        self.candidate += 1;
    }

    /// Goes a depth deeper: the `width` best partial turns just built (the
    /// earlier among equals) become the parents, each with its candidates,
    /// the game's order of its first playing copy's legal commands.
    fn deepen(&mut self) {
        let mut children = core::mem::take(&mut self.children);
        children.sort_by_key(|&(score, _)| Reverse(score));
        children.truncate(at_least_one(self.bot.settings.width));
        let cap = at_least_one(self.bot.settings.candidates);
        self.parents.clear();
        for (_, mut partial) in children {
            if let Some(copy) = partial.copies.iter().find(|copy| copy.playing) {
                let legal = copy.state.legal_commands();
                partial.candidates = self.bot.order.order(&copy.state, legal);
                partial.candidates.truncate(cap);
            }
            self.parents.push(partial);
        }
        self.depth += 1;
        self.parent = 0;
        self.candidate = 0;
    }

    /// The decision: each first command's best value, then the pick.
    fn pick(&mut self) -> R::Command {
        let temperature = self.bot.settings.temperature;
        let index = pick(&self.values, temperature, &mut self.bot.rng);
        self.firsts[index].clone()
    }
}

#[cfg(test)]
mod tests;
