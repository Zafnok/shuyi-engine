//! A search that plays a level through the step's rules, headless: every
//! command from every position reached, breadth first, so the first win it
//! finds is one of the fewest moves.

use std::collections::{BTreeSet, VecDeque};

use shuyi::core::TurnRules;

/// The most positions a search looks at before it gives up.
pub const LIMIT: usize = 200_000;

/// What a search found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Solved<C> {
    /// A win, by these commands, as few as any win takes.
    Won(Vec<C>),
    /// No win, from any position the rules reach.
    Never,
    /// Over [`LIMIT`] positions, and no win among them.
    TooBig,
}

/// Each position a search reached, with the one it came from and the
/// command that led from there; none for the start.
type Reached<R, C> = Vec<(R, Option<(usize, C)>)>;

/// The fewest commands that win from `start`, the positions told apart by
/// `key` (where the pieces are, not how many moves were made).
pub fn solve<R, K>(start: &R, key: impl Fn(&R) -> K) -> Solved<R::Command>
where
    R: TurnRules + Clone,
    R::Command: Clone,
    K: Ord,
{
    let mut reached: Reached<R, R::Command> = vec![(start.clone(), None)];
    let mut seen = BTreeSet::from([key(start)]);
    let mut queue = VecDeque::from([0]);
    while let Some(at) = queue.pop_front() {
        if reached[at].0.outcome().is_some() {
            return Solved::Won(path(&reached, at));
        }
        for command in reached[at].0.legal_commands() {
            let mut next = reached[at].0.clone();
            if next.apply(&command).is_err() || !seen.insert(key(&next)) {
                continue;
            }
            if reached.len() == LIMIT {
                return Solved::TooBig;
            }
            reached.push((next, Some((at, command))));
            queue.push_back(reached.len() - 1);
        }
    }
    Solved::Never
}

/// The commands that led to the position `at`.
fn path<R, C: Clone>(reached: &Reached<R, C>, mut at: usize) -> Vec<C> {
    let mut commands = Vec::new();
    while let Some((from, command)) = &reached[at].1 {
        commands.push(command.clone());
        at = *from;
    }
    commands.reverse();
    commands
}
