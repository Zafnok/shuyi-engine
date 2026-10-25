//! Tests of the tactics order on the toy skirmish (two pieces a side, a
//! goal, a wall), and of its steps alone.

use alloc::vec;
use alloc::vec::Vec;

use proptest::prelude::*;

use super::*;
use crate::bots::fixtures::{GOAL_TILE, Skirm, Skirmish, SkirmishHooks, WALL};
use crate::bots::{Bot, PlanSettings, TurnPlan};
use crate::turn::TurnRules;

const A: u8 = 0;
const B: u8 = 1;
const X: u8 = 2;
const Y: u8 = 3;

fn p(x: i32, y: i32) -> Pos {
    Pos::new(x, y)
}

/// Side 0's `A` and `B`, side 1's `X` and `Y`, on these tiles.
fn skirmish(a: (i32, i32), b: (i32, i32), x: (i32, i32), y: (i32, i32)) -> Skirmish {
    Skirmish::new(&[(0, a), (0, b), (1, x), (1, y)])
}

/// `A` can strike `X` and `B` can strike `Y`, each with a sword or a bow.
fn strikes_on_both_sides() -> Skirmish {
    skirmish((0, 0), (4, 4), (0, 2), (6, 4))
}

/// The tactics order of `stage`'s legal commands, capped at `cap`, with
/// `pick` as the game's pick.
fn ordered(stage: &Skirmish, cap: usize, pick: Option<Skirm>) -> Vec<Skirm> {
    let mut hooks = SkirmishHooks::new();
    hooks.pick = pick;
    TacticsOrder::new(hooks, cap).order(stage, stage.legal_commands())
}

/// `stage`'s steps, as the commands they list, and the hooks after.
fn steps_of(stage: &Skirmish) -> (Steps, Vec<Skirm>, SkirmishHooks) {
    let mut hooks = SkirmishHooks::new();
    let legal = stage.legal_commands();
    let steps = steps(&mut hooks, stage, &legal);
    (steps, legal, hooks)
}

fn wait(piece: u8, (x, y): (i32, i32)) -> Skirm {
    Skirm::Wait {
        piece,
        dest: p(x, y),
    }
}

fn strike(piece: u8, target: u8, way: u32, (x, y): (i32, i32)) -> Skirm {
    Skirm::Strike {
        piece,
        target,
        way,
        from: p(x, y),
    }
}

/// The commands at `indices`.
fn at(legal: &[Skirm], indices: &[usize]) -> Vec<Skirm> {
    indices.iter().map(|&index| legal[index]).collect()
}

fn piece_of(command: Skirm) -> Option<u8> {
    match command {
        Skirm::Guard(piece) | Skirm::Taunt(piece) => Some(piece),
        Skirm::Wait { piece, .. } | Skirm::Strike { piece, .. } | Skirm::Heal { piece, .. } => {
            Some(piece)
        }
        Skirm::End => None,
    }
}

#[test]
fn capped_at_4_every_piece_strikes_then_one_advances_then_the_turn_ends() {
    let stage = strikes_on_both_sides();
    assert_eq!(
        ordered(&stage, 4, None),
        [
            strike(A, X, 0, (0, 1)),
            strike(B, Y, 0, (5, 4)),
            wait(A, (2, 0)),
            Skirm::End,
        ]
    );
}

#[test]
fn every_pieces_best_strike_comes_before_any_pieces_second_way() {
    let order = ordered(&strikes_on_both_sides(), usize::MAX, None);
    let position = |command: Skirm| order.iter().position(|&listed| listed == command);
    let swords = [strike(A, X, 0, (0, 1)), strike(B, Y, 0, (5, 4))];
    let bows = order
        .iter()
        .filter(|command| matches!(command, Skirm::Strike { way: 1, .. }))
        .count();
    // Each piece's bow once, from its first tile: the same way from
    // another tile is dropped.
    assert_eq!(bows, 2);
    assert_eq!(&order[..2], &swords);
    let second = order
        .iter()
        .position(|command| matches!(command, Skirm::Strike { way: 1, .. }))
        .unwrap();
    // After both advances.
    assert!(second > position(wait(B, (4, 2))).unwrap());
    assert_eq!(order[second], strike(A, X, 1, (0, 0)));
    assert_eq!(order[second + 1], strike(B, Y, 1, (5, 3)));
}

#[test]
fn a_hold_is_the_safe_tile_nearest_the_goal_by_walking_round_the_wall() {
    // X threatens the top-left, Y stands on the goal. A's advance, the top
    // row, is in X's reach; the bottom row is as near and safe.
    let stage = skirmish((2, 2), (5, 4), (0, 0), (6, 2));
    let (steps, legal, mut hooks) = steps_of(&stage);
    let hold = wait(A, (2, 4));
    let advance = wait(A, (2, 0));
    assert!(at(&legal, &steps.advances).contains(&advance));
    assert!(at(&legal, &steps.holds).contains(&hold));
    assert!(hooks.threats.safe(p(2, 4)));
    assert!(!hooks.threats.safe(p(2, 0)));

    // Of A's safe steps, it is the nearest by walking, and not the nearest
    // as the crow flies: that one is behind the wall.
    let safe: Vec<Pos> = legal
        .iter()
        .filter_map(|&command| match command {
            Skirm::Wait { piece: A, dest } if hooks.threats.safe(dest) => Some(dest),
            _ => None,
        })
        .collect();
    let walk = |hooks: &mut SkirmishHooks, dest: Pos| hooks.distance(&stage, A, dest).unwrap();
    let nearest = safe.iter().map(|&dest| walk(&mut hooks, dest)).min();
    assert_eq!(nearest, Some(walk(&mut hooks, p(2, 4))));
    let crow = safe
        .iter()
        .min_by_key(|&&dest| Pos::manhattan(dest, GOAL_TILE))
        .copied();
    assert_eq!(crow, Some(p(2, 3)));
    assert!(walk(&mut hooks, p(2, 3)) > walk(&mut hooks, p(2, 4)));
    assert!(WALL.contains(&p(3, 2)));
}

#[test]
fn the_rest_leaves_out_the_advances_and_the_holds() {
    let stage = skirmish((2, 2), (5, 4), (0, 0), (6, 2));
    let (steps, legal, _) = steps_of(&stage);
    let rest = at(&legal, &steps.rest);
    assert!(!steps.advances.is_empty() && !steps.holds.is_empty());
    for listed in at(&legal, &steps.advances)
        .into_iter()
        .chain(at(&legal, &steps.holds))
    {
        assert!(!rest.contains(&listed), "{listed:?}");
    }
    // Every other step and the guards are there, in the legal order.
    assert!(rest.contains(&wait(A, (2, 2))));
    assert!(rest.contains(&Skirm::Guard(A)));
    assert!(steps.rest.is_sorted());
}

#[test]
fn a_hold_that_is_the_advance_is_left_out() {
    // X now threatens the bottom-left: A's advance is its safe step
    // nearest the goal.
    let stage = skirmish((2, 2), (5, 4), (0, 4), (6, 2));
    let (steps, legal, _) = steps_of(&stage);
    assert!(at(&legal, &steps.advances).contains(&wait(A, (2, 0))));
    assert!(
        at(&legal, &steps.holds)
            .iter()
            .all(|&command| piece_of(command) != Some(A))
    );
    let order = ordered(&stage, usize::MAX, None);
    let advances = order
        .iter()
        .filter(|&&command| command == wait(A, (2, 0)))
        .count();
    assert_eq!(advances, 1);
}

#[test]
fn a_piece_with_no_step_nearer_the_goal_has_no_advance() {
    // B stands on the goal.
    let stage = skirmish((0, 0), (6, 2), (0, 4), (4, 4));
    let (steps, legal, _) = steps_of(&stage);
    let advances = at(&legal, &steps.advances);
    assert_eq!(advances, [wait(A, (2, 0))]);
}

#[test]
fn the_end_of_the_turn_is_within_every_cap() {
    let stage = strikes_on_both_sides();
    let commands = stage.legal_commands().len();
    let listed = ordered(&stage, usize::MAX, None).len();
    for cap in 0..=commands {
        let order = ordered(&stage, cap, None);
        assert_eq!(order.len(), cap.clamp(1, listed), "cap {cap}");
        assert_eq!(order.last(), Some(&Skirm::End), "cap {cap}");
    }
}

#[test]
fn others_come_in_the_legal_order_and_drops_never() {
    let order = ordered(&strikes_on_both_sides(), usize::MAX, None);
    assert!(
        !order
            .iter()
            .any(|command| matches!(command, Skirm::Taunt(_)))
    );
    let guard_a = order.iter().position(|&c| c == Skirm::Guard(A)).unwrap();
    let guard_b = order.iter().position(|&c| c == Skirm::Guard(B)).unwrap();
    let bows = order
        .iter()
        .rposition(|command| matches!(command, Skirm::Strike { way: 1, .. }))
        .unwrap();
    assert!(bows < guard_a && guard_a < guard_b);
    // The steps not listed yet come in the legal order after the guards
    // they follow there.
    assert_eq!(order[guard_a + 1], wait(A, (0, 0)));
}

#[test]
fn the_games_pick_comes_after_the_first_strikes_and_only_once() {
    let stage = strikes_on_both_sides();
    let plain = ordered(&stage, usize::MAX, None);
    for pick in [strike(A, X, 1, (0, 0)), wait(A, (2, 0)), Skirm::Guard(B)] {
        let order = ordered(&stage, usize::MAX, Some(pick));
        assert_eq!(order[2], pick);
        assert_eq!(order.iter().filter(|&&c| c == pick).count(), 1);
        let without: Vec<Skirm> = order.iter().copied().filter(|&c| c != pick).collect();
        let plain_without: Vec<Skirm> = plain.iter().copied().filter(|&c| c != pick).collect();
        assert_eq!(without, plain_without);
    }
}

#[test]
fn a_pick_already_listed_ending_dropped_or_not_given_changes_nothing() {
    let stage = strikes_on_both_sides();
    let plain = ordered(&stage, usize::MAX, None);
    for pick in [
        strike(A, X, 0, (0, 1)),
        // The same way from a worse tile: dropped by the order.
        strike(A, X, 1, (1, 1)),
        Skirm::End,
        Skirm::Taunt(A),
        wait(A, (6, 4)),
    ] {
        let order = ordered(&stage, usize::MAX, Some(pick));
        if pick == strike(A, X, 1, (1, 1)) {
            // A legal command the order left out is the game's to try.
            assert_eq!(order[2], pick);
            assert_eq!(&order[3..], &plain[2..]);
        } else {
            assert_eq!(order, plain, "{pick:?}");
        }
    }
}

#[test]
fn a_heal_is_listed_once_from_the_healers_own_tile_before_the_advances() {
    // B is wounded beside A.
    let mut stage = skirmish((1, 1), (1, 2), (6, 0), (6, 4));
    stage.pieces[usize::from(B)].hp = 5;
    let order = ordered(&stage, usize::MAX, None);
    let heals: Vec<Skirm> = order
        .iter()
        .copied()
        .filter(|command| matches!(command, Skirm::Heal { .. }))
        .collect();
    let own = Skirm::Heal {
        piece: A,
        target: B,
        from: p(1, 1),
    };
    assert_eq!(heals, [own]);
    assert_eq!(order[0], own);
    let legal_heals = stage
        .legal_commands()
        .iter()
        .filter(|command| matches!(command, Skirm::Heal { .. }))
        .count();
    assert!(legal_heals > 1);
}

#[test]
fn the_planner_with_the_order_strikes_first() {
    let settings = PlanSettings {
        width: 2,
        candidates: 4,
        depth: 2,
        samples: 1,
        temperature: 0,
    };
    // The damage dealt to the other side.
    let dealt = |start: &Skirmish, end: &Skirmish, side: usize| -> i32 {
        let hp = |stage: &Skirmish| -> i32 {
            stage
                .pieces
                .iter()
                .filter(|piece| piece.side != side)
                .map(|piece| piece.hp)
                .sum()
        };
        hp(start) - hp(end)
    };
    let order = TacticsOrder::new(SkirmishHooks::new(), 4);
    let mut planner = TurnPlan::new(order, dealt, settings, 7);
    let stage = strikes_on_both_sides();
    let chosen = planner.choose(&stage);
    assert!(
        [strike(A, X, 0, (0, 1)), strike(B, Y, 0, (5, 4))].contains(&chosen),
        "{chosen:?}"
    );
}

#[test]
fn the_order_is_pinned() {
    // Integers and lists only: the same on every target (native, wasm32).
    assert_eq!(
        ordered(&strikes_on_both_sides(), 10, None),
        [
            strike(A, X, 0, (0, 1)),
            strike(B, Y, 0, (5, 4)),
            wait(A, (2, 0)),
            wait(B, (4, 2)),
            strike(A, X, 1, (0, 0)),
            strike(B, Y, 1, (5, 3)),
            Skirm::Guard(A),
            wait(A, (0, 0)),
            wait(A, (1, 0)),
            Skirm::End,
        ]
    );
}

/// A strike of `piece` on `target` in `way`, worth `value`.
fn kind_strike(piece: u8, target: u8, way: u32, value: i64) -> Kind<u8> {
    Kind::Strike {
        piece,
        target: Some(target),
        way,
        value: (value, 0),
    }
}

#[test]
fn strikes_keep_one_per_way_and_target_and_hold_back_the_second_ways() {
    let kinds = [
        kind_strike(0, 5, 0, 1),
        kind_strike(0, 5, 0, 3),
        kind_strike(0, 5, 1, 2),
        kind_strike(1, 5, 0, 2),
        kind_strike(0, 6, 1, 0),
        Kind::Other,
        Kind::Strike {
            piece: 1,
            target: None,
            way: 2,
            value: (9, 0),
        },
    ];
    let (firsts, held) = strikes(&kinds);
    assert_eq!(firsts, [6, 1, 3, 4]);
    assert_eq!(held, [2]);
}

#[test]
fn extras_go_by_group_then_value_once_each() {
    let extra = |group: u8, value: i64, once: Option<(u8, Option<u8>, u32)>| Kind::Extra {
        group,
        value: (value, 0),
        once,
    };
    let kinds = [
        extra(1, 9, None),
        extra(0, 1, Some((0, Some(5), 0))),
        Kind::Other,
        extra(0, 3, Some((0, Some(5), 0))),
        extra(0, 2, None),
        extra(0, 2, None),
        extra(1, 0, Some((0, Some(5), 0))),
        extra(0, 0, Some((0, Some(6), 0))),
    ];
    assert_eq!(extras(&kinds), [3, 4, 5, 7, 0]);
}

#[test]
fn keep_nearest_keeps_each_pieces_first_nearest() {
    let mut best = Vec::new();
    keep_nearest(&mut best, 'a', 5, 0);
    keep_nearest(&mut best, 'b', 9, 1);
    keep_nearest(&mut best, 'a', 5, 2);
    keep_nearest(&mut best, 'a', 6, 3);
    assert_eq!(best, vec![('a', 5, 0), ('b', 9, 1)]);
    keep_nearest(&mut best, 'a', 4, 4);
    assert_eq!(best, vec![('a', 4, 4), ('b', 9, 1)]);
}

/// The skirmish's hooks, with a guard taken for an end of the turn.
struct GuardsEnd(SkirmishHooks);

impl OrderHooks<Skirmish> for GuardsEnd {
    type Piece = u8;

    fn prepare(&mut self, rules: &Skirmish) {
        self.0.prepare(rules);
    }

    fn classify(&mut self, rules: &Skirmish, command: &Skirm) -> Kind<u8> {
        match command {
            Skirm::Guard(_) => Kind::End,
            _ => self.0.classify(rules, command),
        }
    }

    fn pick(&mut self, rules: &Skirmish) -> Option<Skirm> {
        self.0.pick(rules)
    }

    fn at(&self, rules: &Skirmish, piece: u8) -> Option<Pos> {
        self.0.at(rules, piece)
    }

    fn distance(&mut self, rules: &Skirmish, piece: u8, from: Pos) -> Option<u32> {
        self.0.distance(rules, piece, from)
    }

    fn safe(&self, rules: &Skirmish, at: Pos) -> bool {
        self.0.safe(rules, at)
    }
}

#[test]
fn of_several_ends_the_first_is_kept_and_the_others_left_out() {
    let stage = strikes_on_both_sides();
    let mut order = TacticsOrder::new(GuardsEnd(SkirmishHooks::new()), usize::MAX);
    let order = order.order(&stage, stage.legal_commands());
    assert_eq!(order.last(), Some(&Skirm::Guard(A)));
    assert!(!order.contains(&Skirm::Guard(B)));
    assert!(!order.contains(&Skirm::End));
}

/// Four distinct tiles off the wall, from `seeds`.
fn tiles_from(seeds: [(i32, i32); 4]) -> Option<[(i32, i32); 4]> {
    let ok = seeds
        .iter()
        .enumerate()
        .all(|(i, &(x, y))| !WALL.contains(&p(x, y)) && !seeds[..i].contains(&(x, y)));
    ok.then_some(seeds)
}

proptest! {
    #[test]
    fn the_order_lists_given_commands_once_never_a_drop_and_the_end_within_the_cap(
        seeds in proptest::array::uniform4((0i32..7, 0i32..5)),
        hurt in proptest::array::uniform4(0i32..10),
        cap in 0usize..40,
        pick in proptest::option::of(any::<proptest::sample::Index>()),
    ) {
        let Some([a, b, x, y]) = tiles_from(seeds) else {
            return Ok(());
        };
        let mut stage = skirmish(a, b, x, y);
        for (piece, hurt) in stage.pieces.iter_mut().zip(hurt) {
            piece.hp -= hurt;
        }
        let legal = stage.legal_commands();
        let pick = pick.map(|index| legal[index.index(legal.len())]);
        let order = ordered(&stage, cap, pick);
        let full = ordered(&stage, usize::MAX, pick);

        prop_assert!(order.len() <= cap.max(1));
        for (i, command) in order.iter().enumerate() {
            prop_assert!(legal.contains(command));
            prop_assert!(!order[..i].contains(command));
            prop_assert!(!matches!(command, Skirm::Taunt(_)));
        }
        prop_assert_eq!(order.last(), Some(&Skirm::End));
        prop_assert_eq!(&order[..order.len() - 1], &full[..order.len() - 1]);
    }
}
