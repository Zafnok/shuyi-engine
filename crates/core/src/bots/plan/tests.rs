//! Tests of the turn planner over the toy games: counters (several
//! commands a turn, a combo, and luck) and Nim (one command a turn).

use alloc::vec::Vec;
use core::cell::{Cell, RefCell};

use proptest::prelude::*;

use super::super::fixtures::{
    Counter, Counters, GOAL, Lie, Nim, PER_TURN, points_ahead, won_or_lost,
};
use super::super::rollout::{Horizon, Rollout, RolloutSettings, Unordered};
use super::super::{RandomBot, play};
use super::*;
use crate::turn::TurnRules;

/// Settings with this width, candidates, depth, samples and temperature.
fn settings(
    width: u32,
    candidates: u32,
    depth: u32,
    samples: u32,
    temperature: u32,
) -> PlanSettings {
    PlanSettings {
        width,
        candidates,
        depth,
        samples,
        temperature,
    }
}

/// A score function of a toy game.
type Score<R> = fn(&R, &R, usize) -> i32;

/// A turn planner for the counters, scoring the points ahead.
fn planner(settings: PlanSettings, seed: u64) -> TurnPlan<Unordered, Score<Counters>> {
    TurnPlan::new(Unordered, points_ahead, settings, seed)
}

/// A rollout bot for the counters with these candidates, samples and
/// temperature, whose playout ends the side's turn at once.
fn ending_rollout(
    candidates: u32,
    samples: u32,
    temperature: u32,
    seed: u64,
) -> Rollout<Unordered, fn(&Counters) -> Counter, Score<Counters>> {
    let settings = RolloutSettings {
        candidates,
        samples,
        horizon: Horizon::Sides(1),
        commands: 4,
        temperature,
    };
    Rollout::new(Unordered, |_| Counter::End, points_ahead, settings, seed)
}

/// Counters part-way through side 0's turn, its luck seeded with `luck`.
fn counters(points: [i32; 2], raised: [bool; 2], fallen: bool, played: u32, luck: u64) -> Counters {
    let mut state = Counters::at(points, true, luck);
    state.raised = raised;
    state.fallen = fallen;
    state.played = played;
    state
}

/// The command a search of `rules` reaches, `slices[i % len]` units at a
/// time.
fn sliced<R, O, E>(bot: &mut TurnPlan<O, E>, rules: &R, slices: &[u32]) -> R::Command
where
    R: Determinize,
    R::Command: Clone + PartialEq,
    O: Order<R>,
    E: Evaluate<R>,
{
    let mut search = bot.search(rules);
    let mut i = 0;
    loop {
        if let Some(command) = search.advance(slices[i % slices.len()]) {
            return command;
        }
        i += 1;
    }
}

/// The units a decision in `rules` takes, and its command.
fn units<R, O, E>(bot: &mut TurnPlan<O, E>, rules: &R) -> (u32, R::Command)
where
    R: Determinize,
    R::Command: Clone + PartialEq,
    O: Order<R>,
    E: Evaluate<R>,
{
    let mut search = bot.search(rules);
    let mut taken = 1;
    loop {
        if let Some(command) = search.advance(1) {
            return (taken, command);
        }
        taken += 1;
    }
}

/// The most units the documented bound lets a decision take.
fn bound(settings: PlanSettings) -> u32 {
    let [width, candidates, depth, samples] = [
        settings.width,
        settings.candidates,
        settings.depth,
        settings.samples,
    ]
    .map(|n| n.max(1));
    samples * candidates * (1 + width * (depth - 1))
}

#[test]
fn it_plays_the_first_step_of_a_combo_the_rollout_bot_refuses() {
    // A raise costs a point and pays only with the other one: alone, it
    // looks worse than ending the turn.
    for seed in 0..10 {
        let start = Counters::new(seed);
        let plan = planner(settings(2, 3, 3, 8, 0), seed).choose(&start);
        assert!(matches!(plan, Counter::Raise(_)), "seed {seed}: {plan:?}");
        // The same candidates, samples and most units: 8 × 3 × 5.
        let rollout = ending_rollout(3, 8, 0, seed).choose(&start);
        assert_eq!(rollout, Counter::End, "seed {seed}");
    }
    let sure = Counters::at([0, 0], false, 0);
    assert_eq!(
        planner(settings(1, 3, 2, 1, 0), 0).choose(&sure),
        Counter::Raise(0)
    );
    assert_eq!(
        planner(settings(1, 3, 1, 1, 0), 0).choose(&sure),
        Counter::End
    );
}

#[test]
fn a_decision_takes_exactly_its_units() {
    // Depth 1: raise 0, raise 1 and end, on 2 copies. Depth 2: each raise
    // by the other and by end. Depth 3: each combo by end, the only
    // command left.
    let sure = Counters::at([0, 0], false, 0);
    let mut bot = planner(settings(2, 3, 3, 2, 0), 0);
    assert_eq!(units(&mut bot, &sure), (6 + 8 + 4, Counter::Raise(0)));
    let mut search = bot.search(&sure);
    assert_eq!(search.advance(17), None);
    assert_eq!(search.advance(1), Some(Counter::Raise(0)));
}

#[test]
fn a_search_does_no_work_on_zero_units_and_keeps_its_answer() {
    let mut bot = planner(settings(2, 3, 3, 4, 0), 1);
    let mut search = bot.search(&Counters::new(5));
    assert_eq!(search.advance(0), None);
    assert_eq!(search.advance(1), None);
    let chosen = search.advance(u32::MAX);
    assert!(chosen.is_some());
    assert_eq!(search.advance(0), chosen);
}

#[test]
fn the_same_seed_and_state_give_the_same_command() {
    for seed in 0..20 {
        let start = counters([0, 3], [false; 2], false, 0, 1);
        let first = planner(settings(2, 3, 3, 4, 40), seed).choose(&start);
        assert_eq!(
            planner(settings(2, 3, 3, 4, 40), seed).choose(&start),
            first,
            "seed {seed}"
        );
    }
}

#[test]
fn every_slicing_gives_the_same_command() {
    for seed in 0..10 {
        let start = Counters::new(seed);
        let whole = planner(settings(2, 3, 3, 3, 30), seed).choose(&start);
        for slice in [1, 7, u32::MAX] {
            let mut bot = planner(settings(2, 3, 3, 3, 30), seed);
            assert_eq!(sliced(&mut bot, &start, &[slice]), whole, "seed {seed}");
        }
    }
}

/// What a score of the counters records of a copy.
type Seen = ([i32; 2], [bool; 2], bool, u32);

#[test]
fn a_refused_command_ends_the_plan_of_that_copy_where_it_stopped() {
    // Raise 0, then what the first copy can do next, then again. A slip
    // of counter 0 knocks counter 1 over: raising it is refused there.
    let mut refusals = 0;
    for seed in 0..30 {
        let seen = RefCell::new(Vec::<Seen>::new());
        let record = |_: &Counters, end: &Counters, side: usize| {
            seen.borrow_mut()
                .push((end.points, end.raised, end.fallen, end.played));
            points_ahead(end, end, side)
        };
        let decision = settings(1, 1, 3, 8, 0);
        TurnPlan::new(Unordered, record, decision, seed).choose(&Counters::new(seed));
        let seen = seen.into_inner();
        assert_eq!(seen.len(), 24, "seed {seed}");
        let (raise, rest) = seen.split_at(8);
        let (then, after) = rest.split_at(8);
        if raise[0].2 {
            // The first copy fell: it raises 0 again, which no copy refuses.
            continue;
        }
        for copy in 0..8 {
            if raise[copy].2 {
                refusals += 1;
                assert_eq!(then[copy], raise[copy], "seed {seed}, copy {copy}");
                assert_eq!(after[copy], raise[copy], "seed {seed}, copy {copy}");
            } else {
                assert_eq!(then[copy].3, 2, "seed {seed}, copy {copy}");
            }
        }
    }
    assert!(refusals > 0);
}

#[test]
fn a_turn_that_ends_after_one_command_is_not_extended() {
    let scored = Cell::new(0);
    let count = |start: &Nim, end: &Nim, side: usize| {
        scored.set(scored.get() + 1);
        won_or_lost(start, end, side)
    };
    let mut bot = TurnPlan::new(Unordered, count, settings(3, 3, 5, 2, 0), 0);
    assert_eq!(units(&mut bot, &Nim::new(10)).0, 6);
    assert_eq!(scored.get(), 6);
    // Taking all of a pile of 3 wins.
    assert_eq!(bot.choose(&Nim::new(3)), 3);
}

#[test]
fn the_beam_keeps_the_best_partial_turns_the_earlier_among_equals() {
    // Which states the order is asked about past the root.
    for (bonus, kept) in [(0, [true, false]), (2, [false, true])] {
        let asked = RefCell::new(Vec::new());
        let order = |rules: &Counters, commands: Vec<Counter>| {
            asked.borrow_mut().push(rules.raised);
            commands
        };
        let favour_one = move |start: &Counters, end: &Counters, side: usize| {
            points_ahead(start, end, side) + if end.raised[1] { bonus } else { 0 }
        };
        let mut bot = TurnPlan::new(order, favour_one, settings(1, 3, 2, 1, 0), 0);
        bot.choose(&Counters::at([0, 0], false, 0));
        assert_eq!(*asked.borrow(), [[false, false], kept], "bonus {bonus}");
    }
}

#[test]
fn the_order_of_each_partial_turn_chooses_its_candidates() {
    let reversed = |_: &Counters, mut commands: Vec<Counter>| {
        commands.reverse();
        commands
    };
    let sure = Counters::at([0, 0], false, 0);
    // Two of end, raise 1, raise 0: raise 1, then raise 0 completes it.
    let mut bot = TurnPlan::new(reversed, points_ahead, settings(2, 2, 2, 1, 0), 0);
    assert_eq!(bot.choose(&sure), Counter::Raise(1));
    // One candidate: end alone.
    let mut bot = TurnPlan::new(reversed, points_ahead, settings(2, 1, 2, 1, 0), 0);
    assert_eq!(bot.choose(&sure), Counter::End);
    // Past the root, an order that drops every raise leaves only end, and
    // no combo.
    let no_second_raise = |rules: &Counters, commands: Vec<Counter>| -> Vec<Counter> {
        if rules.raised == [false, false] {
            commands
        } else {
            commands
                .into_iter()
                .filter(|c| *c == Counter::End)
                .collect()
        }
    };
    let mut bot = TurnPlan::new(no_second_raise, points_ahead, settings(3, 3, 3, 1, 0), 0);
    assert_eq!(bot.choose(&sure), Counter::End);
}

#[test]
fn a_won_game_ends_the_plan() {
    // The combo reaches the goal; the copies stop there, and the order is
    // never asked about a won game.
    let near = Counters::at([GOAL - 8, 0], false, 0);
    let asked = RefCell::new(Vec::new());
    let order = |rules: &Counters, commands: Vec<Counter>| {
        asked.borrow_mut().push(rules.outcome());
        commands
    };
    let mut bot = TurnPlan::new(order, won_or_lost::<Counters>, settings(2, 3, 3, 1, 0), 0);
    assert_eq!(units(&mut bot, &near), (3 + 4, Counter::Raise(0)));
    assert_eq!(*asked.borrow(), [None; 3]);
}

#[test]
fn zero_width_candidates_depth_or_samples_count_as_one() {
    let sure = Counters::at([0, 0], false, 0);
    let decide = |settings: PlanSettings| {
        let scored = Cell::new(0);
        let count = |start: &Counters, end: &Counters, side: usize| {
            scored.set(scored.get() + 1);
            points_ahead(start, end, side)
        };
        let command = TurnPlan::new(Unordered, count, settings, 0).choose(&sure);
        (scored.get(), command)
    };
    let base = settings(2, 3, 3, 2, 0);
    for field in 0..4 {
        let with = |n: u32| {
            let mut changed = base;
            *[
                &mut changed.width,
                &mut changed.candidates,
                &mut changed.depth,
                &mut changed.samples,
            ][field] = n;
            changed
        };
        assert_eq!(decide(with(0)), decide(with(1)), "field {field}");
        assert_ne!(decide(with(1)).0, decide(base).0, "field {field}");
    }
    assert_eq!(decide(settings(0, 0, 0, 0, 0)), (1, Counter::Raise(0)));
}

#[test]
#[should_panic(expected = "determinize changed the commands at the root")]
fn a_determinize_that_changes_the_commands_panics() {
    let mut bot = TurnPlan::new(Unordered, won_or_lost::<Nim>, settings(1, 3, 2, 1, 0), 0);
    bot.choose(&Nim::lying(2, Lie::Pile));
}

#[test]
#[should_panic(expected = "the copy from determinize refused candidate 0")]
fn a_candidate_the_root_copy_refuses_panics() {
    let mut bot = TurnPlan::new(Unordered, won_or_lost::<Nim>, settings(1, 3, 2, 1, 0), 0);
    bot.choose(&Nim::lying(5, Lie::Refuses));
}

#[test]
#[should_panic(expected = "no command to choose from")]
fn a_game_with_nothing_legal_panics() {
    let mut bot = TurnPlan::new(Unordered, won_or_lost::<Nim>, settings(1, 3, 2, 1, 0), 0);
    bot.choose(&Nim::new(0));
}

#[test]
fn settings_are_data() {
    let persona = settings(4, 6, 3, 8, 20);
    let text = ron::to_string(&persona).unwrap_or_default();
    assert_eq!(
        text,
        "(width:4,candidates:6,depth:3,samples:8,temperature:20)"
    );
    assert_eq!(ron::from_str::<PlanSettings>(&text).ok(), Some(persona));
    assert_eq!(planner(persona, 0).settings(), persona);
}

#[test]
fn a_planner_wins_against_a_random_bot() {
    let mut game = Counters::new(4);
    let mut plans = planner(settings(2, 3, 3, 4, 0), 1);
    let mut random = RandomBot::new(2);
    let report = play(&mut game, &mut [(0, &mut plans), (1, &mut random)], 40)
        .unwrap_or_else(|_| panic!("a refused command"));
    assert_eq!(report.outcome, Some(0));
}

prop_compose! {
    /// Counters part-way through side 0's turn, before the game's end.
    fn any_counters()(
        points in [-20..GOAL, -20..GOAL],
        raised in any::<[bool; 2]>(),
        fallen in any::<bool>(),
        played in 0..PER_TURN,
        luck in any::<u64>(),
    ) -> Counters {
        counters(points, raised, fallen, played, luck)
    }
}

proptest! {
    #[test]
    fn one_command_deep_it_is_the_rollout_bot(
        start in any_counters(),
        candidates in 0..4_u32,
        samples in 0..6_u32,
        temperature in 0..60_u32,
        seed in any::<u64>(),
    ) {
        let one_deep = settings(1, candidates, 1, samples, temperature);
        let mut plans = planner(one_deep, seed);
        let mut rolls = ending_rollout(candidates, samples, temperature, seed);
        prop_assert_eq!(plans.choose(&start), rolls.choose(&start));
        // Their next decisions too: each drew as much.
        prop_assert_eq!(plans.choose(&start), rolls.choose(&start));
    }

    #[test]
    fn the_luck_to_come_never_changes_the_command(
        start in any_counters(),
        luck in any::<u64>(),
        seed in any::<u64>(),
    ) {
        let other = counters(start.points, start.raised, start.fallen, start.played, luck);
        let first = planner(settings(2, 3, 3, 4, 25), seed).choose(&start);
        let second = planner(settings(2, 3, 3, 4, 25), seed).choose(&other);
        prop_assert_eq!(first, second);
    }

    #[test]
    fn slicing_never_changes_the_command(
        start in any_counters(),
        slices in proptest::collection::vec(1..20_u32, 1..8),
        seed in any::<u64>(),
    ) {
        let (mut whole, mut parts) = (
            planner(settings(2, 3, 3, 3, 25), seed),
            planner(settings(2, 3, 3, 3, 25), seed),
        );
        prop_assert_eq!(sliced(&mut parts, &start, &slices), whole.choose(&start));
        // The bot's next decision is the same too: the slicing drew nothing.
        prop_assert_eq!(parts.choose(&start), whole.choose(&start));
    }

    #[test]
    fn it_picks_a_legal_command_within_its_bound(
        start in any_counters(),
        width in 0..4_u32,
        candidates in 0..4_u32,
        depth in 0..5_u32,
        samples in 0..4_u32,
        seed in any::<u64>(),
    ) {
        let decision = settings(width, candidates, depth, samples, 10);
        let (taken, command) = units(&mut planner(decision, seed), &start);
        prop_assert!(start.legal_commands().contains(&command));
        prop_assert!(taken <= bound(decision), "{} > {}", taken, bound(decision));
    }
}
