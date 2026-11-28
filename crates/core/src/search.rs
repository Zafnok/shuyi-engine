//! A level solver: a breadth-first search over any [`TurnRules`] game's
//! states, which proves a level can be won and gives its shortest line
//! ([`solve`], or a [`Solver`] stepped a slice at a time).
//!
//! A game calls it from its `check` to prove each level winnable, and the
//! line it finds doubles as a replay: its commands, applied to the start,
//! win.
//!
//! # Rules
//!
//! - **Breadth first**, so the line found is a shortest one in commands.
//! - **Children in [`legal_commands`](TurnRules::legal_commands) order,
//!   and the goal checked when a state is made**, so among shortest lines
//!   the first in that order wins, every time.
//! - **A state seen before is skipped.** Its [`state_hash`] finds it among
//!   the states kept, and full equality (`Eq`) confirms it, so a hash
//!   collision never drops a state.
//! - **`states` counts** the start and every distinct state made after
//!   it. The search keeps every state it made (the `Eq` check needs
//!   them), so [`Limits::states`] bounds its memory too. When a new
//!   distinct state would pass the limit, the answer is
//!   [`Solve::LimitReached`]. The start is always kept, so a limit of 0
//!   counts as 1.
//! - **A start whose outcome the goal accepts** is [`Solve::Solved`] with
//!   no commands. A finished state the goal refuses is a dead end, and so
//!   is a state with no listed commands.
//! - **A state [`Limits::depth`] commands from the start** that isn't
//!   finished is not expanded, and counts as cut. If any state was cut,
//!   the answer is [`Solve::LimitReached`], never [`Solve::Exhausted`].
//! - **[`Solve::Exhausted`] proves a level can't be won only if the game
//!   lists every command.** [`TurnRules`] lets a game leave some out, and
//!   such a game reads `Exhausted` as "not found among the listed
//!   commands".
//! - **Luck is part of the state.** The search plays the state as it
//!   stands, its [`SimRng`](crate::SimRng) included, so a line is a line
//!   for that seed. That is right for a puzzle level and its seed. It says
//!   nothing about other seeds.
//! - **Every command is the solver's.** It picks the commands of every
//!   side. For a game with an opponent, `Solved` means "some line of play
//!   reaches the goal", not "a side can force it".
//! - **A state that keeps its turn number** is never equal to a state of
//!   another turn, so the states kept grow with the depth, and a level
//!   whose pieces can walk forever never runs out of states. [`Limits`]
//!   bounds that, and the answer is then [`Solve::LimitReached`].
//! - **The game's own bugs are errors.** A command its `legal_commands`
//!   listed that `apply` refuses is [`SolveError::Refused`], and a state
//!   whose own `Serialize` fails is [`SolveError::Hash`].
//!
//! # Slices
//!
//! [`Solver::step`] expands up to a number of states and returns, so a
//! game can spread a search over frames. Its whole state lives in the
//! [`Solver`], and stepping by any slice sizes gives the same answer as
//! [`solve`].

use alloc::collections::{BTreeMap, VecDeque};
use alloc::vec::Vec;
use core::fmt;

use serde::Serialize;

use crate::hash::{HashError, StateHash, state_hash};
use crate::turn::TurnRules;

/// How far a search may go: the most distinct states kept, and the
/// longest line in commands. Build it with [`Limits::DEFAULT`] and its
/// `with_` methods (ADR-0039, ADR-0151).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct Limits {
    /// The most distinct states kept, the start included (0 counts as 1).
    pub states: u32,
    /// The longest line, in commands.
    pub depth: u32,
}

impl Limits {
    /// 100 000 states and no depth limit (`u32::MAX`).
    pub const DEFAULT: Self = Self {
        states: 100_000,
        depth: u32::MAX,
    };

    /// The same limits, keeping at most `states` distinct states.
    #[must_use]
    pub const fn with_states(mut self, states: u32) -> Self {
        self.states = states;
        self
    }

    /// The same limits, lines at most `depth` commands long.
    #[must_use]
    pub const fn with_depth(mut self, depth: u32) -> Self {
        self.depth = depth;
        self
    }
}

impl Default for Limits {
    /// [`Limits::DEFAULT`].
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// What a search found ([module docs](self)): a closed set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Solve<C> {
    /// A shortest line from the start to an outcome the goal accepts.
    Solved {
        /// The line's commands, in order.
        commands: Vec<C>,
        /// The distinct states made.
        states: u32,
    },
    /// Every line among the listed commands was tried, and none reaches
    /// the goal.
    Exhausted {
        /// The distinct states made.
        states: u32,
    },
    /// A limit stopped the search first. It is never a pass.
    LimitReached {
        /// The distinct states made.
        states: u32,
    },
}

impl<C> Solve<C> {
    /// The distinct states the search made.
    pub fn states(&self) -> u32 {
        match self {
            Self::Solved { states, .. }
            | Self::Exhausted { states }
            | Self::LimitReached { states } => *states,
        }
    }
}

/// Why a search failed: a bug in the game, not in its level.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum SolveError<C> {
    /// A state's own `Serialize` failed, so it could not be hashed.
    Hash(HashError),
    /// The game refused a command its own
    /// [`legal_commands`](TurnRules::legal_commands) listed.
    Refused {
        /// The line from the start that reaches the state, ending with
        /// the refused command: replaying it shows the game's error.
        line: Vec<C>,
    },
}

impl<C: fmt::Debug> fmt::Display for SolveError<C> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Hash(e) => e.fmt(f),
            Self::Refused { line } => write!(
                f,
                "the game refused a command its legal_commands listed, at the end of the line {line:?}"
            ),
        }
    }
}

impl<C: fmt::Debug> core::error::Error for SolveError<C> {}

/// A search's answer.
type Answer<C> = Result<Solve<C>, SolveError<C>>;

/// A state the search kept: the state, how deep it is, and the state and
/// command it was made from (none for the start).
#[derive(Debug, Clone)]
struct Node<R: TurnRules> {
    state: R,
    depth: u32,
    parent: Option<(u32, R::Command)>,
}

/// A breadth-first search over the states of a [`TurnRules`] game, run a
/// slice at a time with [`step`](Self::step) ([module docs](self)).
///
/// `goal` says which outcomes the search looks for.
pub struct Solver<R: TurnRules, G> {
    /// The start, until the first step keeps it.
    start: Option<R>,
    goal: G,
    limits: Limits,
    /// How a state is hashed: [`state_hash`] (a test sets another).
    hash: fn(&R) -> Result<StateHash, HashError>,
    /// Every distinct state made, the start first.
    nodes: Vec<Node<R>>,
    /// The kept states' indexes, by their hash.
    seen: BTreeMap<StateHash, Vec<u32>>,
    /// The states left to expand, in the order they were made.
    queue: VecDeque<u32>,
    /// Whether the depth limit cut a state.
    cut: bool,
    /// The answer, once there is one.
    answer: Option<Answer<R::Command>>,
}

impl<R: TurnRules, G> fmt::Debug for Solver<R, G> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Solver")
            .field("limits", &self.limits)
            .field("states", &self.nodes.len())
            .field("queued", &self.queue.len())
            .field("cut", &self.cut)
            .field("done", &self.answer.is_some())
            .finish_non_exhaustive()
    }
}

impl<R, G> Solver<R, G>
where
    R: TurnRules + Clone + Eq + Serialize,
    R::Command: Clone,
    G: Fn(&R::Outcome) -> bool,
{
    /// A search from `start` for an outcome `goal` accepts, within
    /// `limits`. It does no work: the first [`step`](Self::step) hashes
    /// and checks the start.
    pub fn new(start: R, goal: G, limits: Limits) -> Self {
        Self {
            start: Some(start),
            goal,
            limits,
            hash: state_hash::<R>,
            nodes: Vec::new(),
            seen: BTreeMap::new(),
            queue: VecDeque::new(),
            cut: false,
            answer: None,
        }
    }

    /// Expands up to `units` states, a unit being one state taken from the
    /// queue and expanded, and returns the answer once there is one, or
    /// `None` while the search isn't done. `step(0)` does nothing. Once it
    /// has answered, it returns the same answer again and expands nothing.
    /// A search expands each state it keeps at most once, and keeps at
    /// most [`Limits::states`], so `step(u32::MAX)` always answers.
    ///
    /// # Errors
    ///
    /// The game's bugs ([`SolveError`]), as the answer.
    pub fn step(
        &mut self,
        units: u32,
    ) -> Result<Option<Solve<R::Command>>, SolveError<R::Command>> {
        if self.answer.is_none() && units > 0 {
            self.answer = self.run(units);
        }
        self.answer.clone().transpose()
    }

    /// The distinct states made so far.
    pub fn states(&self) -> u32 {
        count(&self.nodes)
    }

    /// Up to `units` expansions, the start kept first: the answer, or
    /// `None` while the search isn't done.
    fn run(&mut self, units: u32) -> Option<Answer<R::Command>> {
        if let Some(start) = self.start.take()
            && let Some(answer) = self.keep(start, None)
        {
            return Some(answer);
        }
        for _ in 0..units {
            let Some(at) = self.queue.pop_front() else {
                break;
            };
            if let Some(answer) = self.expand(at) {
                return Some(answer);
            }
        }
        if self.queue.is_empty() {
            let states = self.states();
            Some(Ok(if self.cut {
                Solve::LimitReached { states }
            } else {
                Solve::Exhausted { states }
            }))
        } else {
            None
        }
    }

    /// Makes each child of the state kept at `at`, in its listed order.
    fn expand(&mut self, at: u32) -> Option<Answer<R::Command>> {
        let node = &self.nodes[index(at)];
        let commands = node.state.legal_commands();
        let state = node.state.clone();
        for command in commands {
            let mut child = state.clone();
            if child.apply(&command).is_err() {
                let mut line = self.line(at);
                line.push(command);
                return Some(Err(SolveError::Refused { line }));
            }
            if let Some(answer) = self.keep(child, Some((at, command))) {
                return Some(answer);
            }
        }
        None
    }

    /// Keeps `state`, made from `parent`, unless it was seen before: the
    /// answer when it ends the search, else `None`.
    fn keep(&mut self, state: R, parent: Option<(u32, R::Command)>) -> Option<Answer<R::Command>> {
        let hash = match (self.hash)(&state) {
            Ok(hash) => hash,
            Err(e) => return Some(Err(SolveError::Hash(e))),
        };
        let nodes = &self.nodes;
        let same = self.seen.entry(hash).or_default();
        if same.iter().any(|&i| nodes[index(i)].state == state) {
            return None;
        }
        let states = count(nodes);
        if !nodes.is_empty() && states >= self.limits.states {
            return Some(Ok(Solve::LimitReached { states }));
        }
        let at = states;
        same.push(at);
        let depth = parent
            .as_ref()
            .map_or(0, |(p, _)| self.nodes[index(*p)].depth + 1);
        let outcome = state.outcome();
        self.nodes.push(Node {
            state,
            depth,
            parent,
        });
        match outcome {
            Some(outcome) if (self.goal)(&outcome) => Some(Ok(Solve::Solved {
                commands: self.line(at),
                states: self.states(),
            })),
            Some(_) => None,
            None if depth >= self.limits.depth => {
                self.cut = true;
                None
            }
            None => {
                self.queue.push_back(at);
                None
            }
        }
    }

    /// The commands from the start to the state kept at `at`.
    fn line(&self, mut at: u32) -> Vec<R::Command> {
        let mut line = Vec::new();
        while let Some((parent, command)) = &self.nodes[index(at)].parent {
            line.push(command.clone());
            at = *parent;
        }
        line.reverse();
        line
    }
}

/// How many states `nodes` holds, which [`Limits::states`] keeps within
/// `u32`.
fn count<T>(nodes: &[T]) -> u32 {
    u32::try_from(nodes.len()).unwrap_or(u32::MAX)
}

/// A kept state's index as a `usize`.
fn index(at: u32) -> usize {
    usize::try_from(at).unwrap_or(usize::MAX)
}

/// Searches from a clone of `start` for an outcome `goal` accepts, within
/// `limits`: a [`Solver`] stepped to its end ([module docs](self)).
///
/// # Errors
///
/// The game's bugs ([`SolveError`]).
pub fn solve<R, G>(start: &R, goal: G, limits: Limits) -> Answer<R::Command>
where
    R: TurnRules + Clone + Eq + Serialize,
    R::Command: Clone,
    G: Fn(&R::Outcome) -> bool,
{
    let mut solver = Solver::new(start.clone(), goal, limits);
    // One slice of `u32::MAX` units always answers ([`Solver::step`]), so
    // the arm after it is never taken; it answers as a limit would.
    let answer = solver.step(u32::MAX)?;
    Ok(answer.unwrap_or_else(|| Solve::LimitReached {
        states: solver.states(),
    }))
}

#[cfg(test)]
mod tests;
