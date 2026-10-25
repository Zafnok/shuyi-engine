//! A planner's candidate order for grid games: [`TacticsOrder`], on
//! [`ranked_order`](crate::bots::ranked_order)'s idea.
//!
//! A planner tries only the first few of hundreds of legal commands. Cut
//! at ten, the legal order holds one piece's attack in every flavour and
//! no piece's move, and a planner whose pieces never step towards the
//! goal or out of danger stalls (game ADR-0095). So a capped list gives
//! every piece its say before any piece's second, and keeps a step
//! towards the goal, a step to safety and the end of the turn. The game
//! tags each legal command with a [`Kind`] ([`OrderHooks::classify`]),
//! and the order is:
//!
//! 1. **strikes**: each piece's best strike on each target, best value
//!    first. Of a piece's strikes with the same way on the same target,
//!    only the best (from its best tile) is listed at all; the first per
//!    `(piece, target)` comes here, so every piece that can strike does
//!    before any piece's second way;
//! 2. **the game's pick** ([`OrderHooks::pick`], an AI's own command), if
//!    it is one of the commands given, isn't [`Kind::End`] or
//!    [`Kind::Drop`] and isn't listed yet; it is then left out of the
//!    steps below;
//! 3. **extras**: the game's own lists ([`Kind::Extra`], heals say), by
//!    group, then the best value first, one per `once`;
//! 4. **advances**: per piece, its [`Kind::Step`] whose destination is
//!    nearest the goals by walking ([`OrderHooks::distance`]), if nearer
//!    than where the piece stands;
//! 5. **holds**: per piece, its step whose destination no threat reaches
//!    ([`OrderHooks::safe`]), nearest the goals, if it isn't its advance;
//! 6. **the other ways of striking**: the strikes held back in step 1,
//!    best first;
//! 7. **the rest**: [`Kind::Other`] and the steps not listed yet, in the
//!    legal order;
//! 8. **the end of the turn** ([`Kind::End`]) last, in place of the last
//!    of the first `cap` commands, so the cap never cuts it.
//!
//! Commands tagged [`Kind::Drop`] are left out. Among equals the legal
//! order decides: the first strike of equal value, the first step of
//! equal distance, the pieces in the order their first listed command
//! comes. The order is at most `cap` commands; give the planner the same
//! cap as its `candidates`.

use alloc::vec::Vec;
use core::cmp::Reverse;

use crate::bots::order::keep_last;
use crate::bots::{Order, Sides};
use crate::grid::Pos;

/// What a legal command is to a [`TacticsOrder`]. `P` is the game's id of
/// a piece.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Kind<P> {
    /// `piece` strikes `target` (or a tile, `None`) in its way `way` (a
    /// weapon, an art, a spell: the game's number for it), worth `value`
    /// (the damage expected dealt, then minus the damage expected taken
    /// back, say: higher first).
    Strike {
        /// The piece striking.
        piece: P,
        /// The piece struck, if it strikes one.
        target: Option<P>,
        /// Its way of striking.
        way: u32,
        /// How good it looks: higher first.
        value: (i64, i64),
    },
    /// One of the game's own lists (heals say), after the game's pick: by
    /// `group`, lower first, then `value`, higher first. Of the commands
    /// with the same `once` (a piece, its target and its way), only the
    /// first by that order is listed; `None` is always listed.
    Extra {
        /// Its list: lower first.
        group: u8,
        /// How good it looks: higher first.
        value: (i64, i64),
        /// What it is one of.
        once: Option<(P, Option<P>, u32)>,
    },
    /// `piece` ends on `dest`, doing nothing else (it waits there): an
    /// advance, a hold or the rest.
    Step {
        /// The piece stepping.
        piece: P,
        /// Where it ends.
        dest: Pos,
    },
    /// The end of the turn, kept within the cap (the first one given; a
    /// second is left out).
    End,
    /// Never tried.
    Drop,
    /// Anything else: listed after the strikes held back.
    Other,
}

/// What a [`TacticsOrder`] asks the game, once per
/// [`order`](Order::order) call. The game's hooks hold the state's
/// [`Threats`](super::Threats) and [`GoalDistance`](super::GoalDistance),
/// built in [`prepare`](OrderHooks::prepare).
pub trait OrderHooks<R: Sides> {
    /// The game's id of a piece.
    type Piece: Eq + Copy;

    /// Called first: build this state's [`Threats`](super::Threats) and
    /// [`GoalDistance`](super::GoalDistance).
    fn prepare(&mut self, rules: &R);

    /// What `command` is, called once per legal command, in the legal
    /// order.
    fn classify(&mut self, rules: &R, command: &R::Command) -> Kind<Self::Piece>;

    /// The game's own pick (an AI's command), if it has one; it may also
    /// be a strike or a step, and is listed once.
    fn pick(&mut self, rules: &R) -> Option<R::Command>;

    /// Where `piece` stands now.
    fn at(&self, rules: &R, piece: Self::Piece) -> Option<Pos>;

    /// How far `piece` would walk from `from` to the nearest goal; `None`
    /// with no walk.
    fn distance(&mut self, rules: &R, piece: Self::Piece, from: Pos) -> Option<u32>;

    /// Whether no threat reaches `at`.
    fn safe(&self, rules: &R, at: Pos) -> bool;
}

/// A planner's candidate order for grid games, in eight steps (the
/// module's docs): every piece's best strike, the game's pick, its extras,
/// each piece's advance and hold, the other strikes, the rest, and the end
/// of the turn within `cap`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct TacticsOrder<H> {
    /// What the order asks the game.
    pub hooks: H,
    /// The most commands it lists; 0 counts as 1. The planner's
    /// `candidates`.
    pub cap: usize,
}

impl<H> TacticsOrder<H> {
    /// An order asking `hooks`, listing at most `cap` commands.
    pub const fn new(hooks: H, cap: usize) -> Self {
        Self { hooks, cap }
    }
}

impl<R, H> Order<R> for TacticsOrder<H>
where
    R: Sides,
    R::Command: PartialEq,
    H: OrderHooks<R>,
{
    fn order(&mut self, rules: &R, commands: Vec<R::Command>) -> Vec<R::Command> {
        let listed = listed(&mut self.hooks, rules, &commands);
        let mut commands: Vec<Option<R::Command>> = commands.into_iter().map(Some).collect();
        let mut take = |index: usize| commands[index].take();
        let order: Vec<R::Command> = listed.order.into_iter().filter_map(&mut take).collect();
        let end = listed.end.and_then(take);
        keep_last(order, end, self.cap)
    }
}

/// The order as indices of the commands given: every step but the last,
/// in order, and the end of the turn.
#[derive(Debug, PartialEq, Eq)]
struct Listed {
    order: Vec<usize>,
    end: Option<usize>,
}

/// The steps of the order (the module's docs), as indices of `commands`.
#[derive(Debug, PartialEq, Eq)]
struct Steps {
    strikes: Vec<usize>,
    pick: Option<usize>,
    extras: Vec<usize>,
    advances: Vec<usize>,
    holds: Vec<usize>,
    held: Vec<usize>,
    rest: Vec<usize>,
    end: Option<usize>,
}

/// The order of `commands` in `rules`, as [`steps`] lists it.
fn listed<R, H>(hooks: &mut H, rules: &R, commands: &[R::Command]) -> Listed
where
    R: Sides,
    R::Command: PartialEq,
    H: OrderHooks<R>,
{
    let steps = steps(hooks, rules, commands);
    let not_pick = |index: &usize| steps.pick != Some(*index);
    let mut order = steps.strikes;
    order.extend(steps.pick);
    for step in [
        steps.extras,
        steps.advances,
        steps.holds,
        steps.held,
        steps.rest,
    ] {
        order.extend(step.into_iter().filter(not_pick));
    }
    Listed {
        order,
        end: steps.end,
    }
}

/// Every step of the order of `commands` in `rules`, each in its order;
/// the pick may be in another step too.
fn steps<R, H>(hooks: &mut H, rules: &R, commands: &[R::Command]) -> Steps
where
    R: Sides,
    R::Command: PartialEq,
    H: OrderHooks<R>,
{
    hooks.prepare(rules);
    let kinds: Vec<Kind<H::Piece>> = commands
        .iter()
        .map(|command| hooks.classify(rules, command))
        .collect();
    let (strikes, held) = strikes(&kinds);
    let (advances, holds) = advances_and_holds(hooks, rules, &kinds);
    let pick = hooks
        .pick(rules)
        .and_then(|pick| commands.iter().position(|command| *command == pick))
        .filter(|&index| !matches!(kinds[index], Kind::End | Kind::Drop))
        .filter(|index| !strikes.contains(index));
    let mut end = None;
    let mut rest = Vec::new();
    for (index, kind) in kinds.iter().enumerate() {
        match kind {
            Kind::End if end.is_none() => end = Some(index),
            Kind::Other | Kind::Step { .. }
                if !advances.contains(&index) && !holds.contains(&index) =>
            {
                rest.push(index);
            }
            _ => {}
        }
    }
    Steps {
        strikes,
        pick,
        extras: extras(&kinds),
        advances,
        holds,
        held,
        rest,
        end,
    }
}

/// A strike's piece and the piece it strikes.
type Aim<P> = (P, Option<P>);

/// What an extra is one of: a piece, its target and its way.
type Once<P> = (P, Option<P>, u32);

/// A strike or an extra of the commands given: its index, what it is one
/// of, and its group and value.
struct Entry<O> {
    index: usize,
    of: O,
    rank: (u8, (i64, i64)),
}

/// The entries `entry` finds in `kinds`, by group, then the best value
/// first (a stable sort: ties keep the legal order).
fn ranked<P, O>(
    kinds: &[Kind<P>],
    entry: impl Fn(&Kind<P>) -> Option<(O, (u8, (i64, i64)))>,
) -> Vec<Entry<O>> {
    let mut entries: Vec<Entry<O>> = kinds
        .iter()
        .enumerate()
        .filter_map(|(index, kind)| entry(kind).map(|(of, rank)| Entry { index, of, rank }))
        .collect();
    entries.sort_by_key(|entry| (entry.rank.0, Reverse(entry.rank.1)));
    entries
}

/// The strikes of `kinds`, best first, one per piece, target and way: the
/// first of each piece and target (step 1), and the others (step 6).
fn strikes<P: Eq + Copy>(kinds: &[Kind<P>]) -> (Vec<usize>, Vec<usize>) {
    let strikes = ranked(kinds, |kind| match *kind {
        Kind::Strike {
            piece,
            target,
            way,
            value,
        } => Some((((piece, target), way), (0, value))),
        _ => None,
    });
    let mut ways: Vec<(Aim<P>, u32)> = Vec::new();
    let mut firsts = Vec::new();
    let mut held = Vec::new();
    for Entry { index, of, .. } in strikes {
        if ways.contains(&of) {
            continue;
        }
        if ways.iter().any(|&(aim, _)| aim == of.0) {
            held.push(index);
        } else {
            firsts.push(index);
        }
        ways.push(of);
    }
    (firsts, held)
}

/// The extras of `kinds`, by group, then the best value first, the first
/// of each `once` only (step 3).
fn extras<P: Eq + Copy>(kinds: &[Kind<P>]) -> Vec<usize> {
    let extras = ranked(kinds, |kind| match *kind {
        Kind::Extra { group, value, once } => Some((once, (group, value))),
        _ => None,
    });
    let mut seen: Vec<Once<P>> = Vec::new();
    let mut out = Vec::new();
    for Entry { index, of, .. } in extras {
        if let Some(once) = of {
            if seen.contains(&once) {
                continue;
            }
            seen.push(once);
        }
        out.push(index);
    }
    out
}

/// Each piece's advance and hold among the steps of `kinds` (steps 4 and
/// 5), in the order of the pieces' first: a piece's best as `(piece,
/// distance, index)`.
fn advances_and_holds<R, H>(
    hooks: &mut H,
    rules: &R,
    kinds: &[Kind<H::Piece>],
) -> (Vec<usize>, Vec<usize>)
where
    R: Sides,
    H: OrderHooks<R>,
{
    let mut here: Vec<(H::Piece, Option<u32>)> = Vec::new();
    let mut advances: Vec<(H::Piece, u32, usize)> = Vec::new();
    let mut holds: Vec<(H::Piece, u32, usize)> = Vec::new();
    for (index, kind) in kinds.iter().enumerate() {
        let Kind::Step { piece, dest } = *kind else {
            continue;
        };
        let Some(there) = hooks.distance(rules, piece, dest) else {
            continue;
        };
        let from = if let Some(&(_, from)) = here.iter().find(|(seen, _)| *seen == piece) {
            from
        } else {
            let from = hooks
                .at(rules, piece)
                .and_then(|at| hooks.distance(rules, piece, at));
            here.push((piece, from));
            from
        };
        if from.is_some_and(|from| there < from) {
            keep_nearest(&mut advances, piece, there, index);
        }
        if hooks.safe(rules, dest) {
            keep_nearest(&mut holds, piece, there, index);
        }
    }
    let advances: Vec<usize> = advances.into_iter().map(|(_, _, index)| index).collect();
    let holds = holds
        .into_iter()
        .map(|(_, _, index)| index)
        .filter(|index| !advances.contains(index))
        .collect();
    (advances, holds)
}

/// Keeps step `index` of `piece`, `distance` from the goals, in `best`
/// if it is the piece's nearest yet (the first among equals).
fn keep_nearest<P: Eq>(best: &mut Vec<(P, u32, usize)>, piece: P, distance: u32, index: usize) {
    match best.iter_mut().find(|(seen, _, _)| *seen == piece) {
        Some(nearest) if distance < nearest.1 => *nearest = (piece, distance, index),
        Some(_) => {}
        None => best.push((piece, distance, index)),
    }
}

#[cfg(test)]
mod tests;
