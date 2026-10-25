//! A candidate order made of ranked groups: the few commands worth trying
//! first, out of hundreds of legal ones.
//!
//! A bot that plans ([`TurnPlan`](super::TurnPlan), [`Rollout`](super::Rollout))
//! tries only the first `candidates` commands of its [`Order`](super::Order).
//! The legal order cut at ten holds one piece's attack in every flavour
//! and nothing else, and a planner that never tries its other pieces, a
//! step towards the goal or the end of its turn stalls (game ADR-0095).
//! [`ranked_order`] is the generic cure: the game tags each legal command
//! with a [`Ranked`] group, value and key (or leaves it out), and gets
//! back
//!
//! 1. the groups in ascending order;
//! 2. in a group, the best value first (a stable sort: ties keep the
//!    game's order);
//! 3. in a group, one per key first: the first command of each key comes
//!    before any second command of a key already listed (a round-robin:
//!    all firsts, then all seconds…); a command with no key is never held
//!    back;
//! 4. the `last` command (the end of the turn), if any, kept within the
//!    cap: it takes the place of the last of the first `cap` commands, so
//!    the cap never cuts it.
//!
//! A deck-builder's order is this alone: damage plays best first, one per
//! target first, then draws, then the rest, ending the turn always kept.
//! A grid game's is [`TacticsOrder`](crate::threat::TacticsOrder), built on
//! it.
//!
//! ```
//! use shuyi_core::bots::{Ranked, ranked_order};
//!
//! let plays = vec![
//!     Ranked::new("strike a", 0, (5, 0), Some('a')),
//!     Ranked::new("draw", 1, (0, 0), None),
//!     Ranked::new("burn a", 0, (4, 0), Some('a')),
//!     Ranked::new("strike b", 0, (3, 0), Some('b')),
//! ];
//! let order = ranked_order(plays.clone(), Some("end turn"), 10);
//! assert_eq!(order, ["strike a", "strike b", "burn a", "draw", "end turn"]);
//! assert_eq!(ranked_order(plays, Some("end turn"), 3), ["strike a", "strike b", "end turn"]);
//! ```

use alloc::vec::Vec;
use core::cmp::Reverse;

/// One command of a candidate order, as the game ranks it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ranked<C, K> {
    /// The command.
    pub command: C,
    /// Its group: lower groups come first.
    pub group: u8,
    /// Its value in its group: higher comes first.
    pub value: (i64, i64),
    /// What it is one of, for one per key first (the target it strikes,
    /// say); `None` is never held back.
    pub key: Option<K>,
}

impl<C, K> Ranked<C, K> {
    /// `command` in `group`, worth `value`, one of `key`.
    pub const fn new(command: C, group: u8, value: (i64, i64), key: Option<K>) -> Self {
        Self {
            command,
            group,
            value,
            key,
        }
    }
}

/// The first `cap` commands of `items` in the order the module's docs
/// give: groups ascending, the best value first in a group, one per key
/// first, and `last`, if any, in place of the last of them. `cap` 0
/// counts as 1.
pub fn ranked_order<C, K: Eq>(mut items: Vec<Ranked<C, K>>, last: Option<C>, cap: usize) -> Vec<C> {
    items.sort_by_key(|item| (item.group, Reverse(item.value)));
    let rounds = rounds(&items);
    let mut ranked: Vec<(u8, usize, C)> = items
        .into_iter()
        .zip(rounds)
        .map(|(item, round)| (item.group, round, item.command))
        .collect();
    ranked.sort_by_key(|&(group, round, _)| (group, round));
    keep_last(
        ranked.into_iter().map(|(_, _, command)| command).collect(),
        last,
        cap,
    )
}

/// Each item's round in its group: how many items of its group and key
/// come before it (0 with no key).
fn rounds<C, K: Eq>(items: &[Ranked<C, K>]) -> Vec<usize> {
    let mut seen: Vec<(u8, &K, usize)> = Vec::new();
    items
        .iter()
        .map(|item| {
            let Some(key) = &item.key else {
                return 0;
            };
            if let Some(entry) = seen
                .iter_mut()
                .find(|(group, seen, _)| *group == item.group && *seen == key)
            {
                entry.2 += 1;
                entry.2
            } else {
                seen.push((item.group, key, 0));
                0
            }
        })
        .collect()
}

/// The first `cap` of `order` (0 counts as 1), with `last`, if any, in
/// place of the last of them.
pub(crate) fn keep_last<C>(mut order: Vec<C>, last: Option<C>, cap: usize) -> Vec<C> {
    let cap = cap.max(1);
    match last {
        Some(last) => {
            order.truncate(cap - 1);
            order.push(last);
        }
        None => order.truncate(cap),
    }
    order
}

#[cfg(test)]
mod tests;
