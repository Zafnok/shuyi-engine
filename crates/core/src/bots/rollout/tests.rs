//! Tests of the rollout bot over the toy games: a dice race (luck in the
//! state) and Nim (no luck).

use alloc::vec;
use alloc::vec::Vec;
use core::cell::{Cell, RefCell};

use proptest::prelude::*;

use super::super::fixtures::{DiceRace, FINISH, Lie, Nim, Race, nim_perfect, won_or_lost};
use super::*;
use crate::turn::TurnRules;

/// Settings with these candidates, samples and horizon, 30 commands a
/// playout, at `temperature`.
fn settings(candidates: u32, samples: u32, horizon: Horizon, temperature: u32) -> RolloutSettings {
    RolloutSettings {
        candidates,
        samples,
        horizon,
        commands: 30,
        temperature,
    }
}

/// A score function of a toy game.
type Score<R> = fn(&R, &R, usize) -> i32;

/// A race's score for `side`: its outcome, or how far ahead it is.
fn race_lead(_: &DiceRace, end: &DiceRace, side: usize) -> i32 {
    match end.outcome() {
        Some(winner) if winner == side => 1000,
        Some(_) => -1000,
        None => {
            let lead = i64::from(end.squares[side]) - i64::from(end.squares[1 - side]);
            i32::try_from(lead).unwrap_or(0)
        }
    }
}

/// A race's playout player: steps when ahead, rolls when behind.
fn race_playout(race: &DiceRace) -> Race {
    let side = race.side_to_move();
    if race.squares[side] > race.squares[1 - side] {
        Race::Step
    } else {
        Race::Roll
    }
}

/// A rollout bot for the race, seeded with `seed`.
fn race_bot(
    seed: u64,
    temperature: u32,
) -> Rollout<Unordered, fn(&DiceRace) -> Race, Score<DiceRace>> {
    Rollout::new(
        Unordered,
        race_playout,
        race_lead,
        settings(2, 6, Horizon::Turns(3), temperature),
        seed,
    )
}

/// A rollout bot for Nim, its playouts played perfectly.
fn nim_bot(
    candidates: u32,
    samples: u32,
    horizon: Horizon,
    seed: u64,
) -> Rollout<Unordered, fn(&Nim) -> u32, Score<Nim>> {
    Rollout::new(
        Unordered,
        nim_perfect,
        won_or_lost::<Nim>,
        settings(candidates, samples, horizon, 0),
        seed,
    )
}

/// The command a search of `rules` reaches, `slices[i % len]` units at a
/// time.
fn sliced<R, O, P, E>(bot: &mut Rollout<O, P, E>, rules: &R, slices: &[u32]) -> R::Command
where
    R: Determinize,
    R::Command: Clone + PartialEq,
    O: Order<R>,
    P: Playout<R>,
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

#[test]
fn in_nim_it_takes_the_winning_amount() {
    for pile in (1..=20).filter(|pile| pile % 4 != 0) {
        let mut bot = nim_bot(3, 2, Horizon::Turns(20), 7);
        assert_eq!(bot.choose(&Nim::new(pile)), pile % 4, "pile {pile}");
    }
}

#[test]
fn the_same_seed_and_state_give_the_same_command() {
    for seed in 0..20 {
        let race = DiceRace::at([u32::try_from(seed).unwrap_or(0), 10], 1);
        let first = race_bot(seed, 40).choose(&race);
        assert_eq!(race_bot(seed, 40).choose(&race), first, "seed {seed}");
    }
}

#[test]
fn every_slicing_gives_the_same_command() {
    for pile in 1..=12 {
        let nim = Nim::new(pile);
        let whole = nim_bot(3, 2, Horizon::Sides(4), 3).choose(&nim);
        for units in [1, 7, 100, u32::MAX] {
            let mut bot = nim_bot(3, 2, Horizon::Sides(4), 3);
            assert_eq!(sliced(&mut bot, &nim, &[units]), whole, "pile {pile}");
        }
    }
}

#[test]
fn a_search_does_no_work_on_zero_units_and_keeps_its_answer() {
    let race = DiceRace::new(5);
    let mut bot = race_bot(1, 0);
    let mut search = bot.search(&race);
    assert_eq!(search.advance(0), None);
    assert_eq!(search.advance(1), None);
    let chosen = search.advance(u32::MAX);
    assert!(chosen.is_some());
    assert_eq!(search.advance(0), chosen);
}

#[test]
fn a_decision_takes_exactly_its_units() {
    // 3 candidates × 2 samples: the candidate alone, or it and 2 replies.
    for (horizon, units) in [(Horizon::Sides(0), 6), (Horizon::Sides(3), 18)] {
        let mut bot = nim_bot(3, 2, horizon, 0);
        let mut search = bot.search(&Nim::new(10));
        assert_eq!(search.advance(units - 1), None, "{horizon:?}");
        assert!(search.advance(1).is_some(), "{horizon:?}");
    }
}

#[test]
fn the_order_and_the_cap_choose_the_candidates() {
    // The more it takes, the better: the best candidate is the last tried.
    let taken =
        |start: &Nim, end: &Nim, _: usize| i32::try_from(start.pile - end.pile).unwrap_or(0);
    let reversed = |_: &Nim, mut commands: Vec<u32>| {
        commands.reverse();
        commands
    };
    let nim = Nim::new(10);
    let none = |_: &Nim| 1_u32;
    let cap = settings(2, 1, Horizon::Sides(0), 0);
    assert_eq!(Rollout::new(Unordered, none, taken, cap, 0).choose(&nim), 2);
    assert_eq!(Rollout::new(reversed, none, taken, cap, 0).choose(&nim), 3);
    // An order may drop commands.
    let odd = |_: &Nim, commands: Vec<u32>| -> Vec<u32> {
        commands.into_iter().filter(|c| c % 2 == 1).collect()
    };
    let all = settings(3, 1, Horizon::Sides(0), 0);
    assert_eq!(Rollout::new(odd, none, taken, all, 0).choose(&nim), 3);
}

#[test]
fn zero_candidates_or_samples_count_as_one() {
    let scored = Cell::new(0);
    let count = |_: &Nim, _: &Nim, _: usize| {
        scored.set(scored.get() + 1);
        0
    };
    let mut bot = Rollout::new(
        Unordered,
        nim_perfect,
        count,
        settings(0, 0, Horizon::Sides(2), 0),
        0,
    );
    assert_eq!(bot.choose(&Nim::new(10)), 1);
    assert_eq!(scored.get(), 1);
}

#[test]
fn ties_go_to_the_earlier_candidate() {
    let even = |_: &Nim, _: &Nim, _: usize| 5;
    let reversed = |_: &Nim, mut commands: Vec<u32>| {
        commands.reverse();
        commands
    };
    let all = settings(3, 2, Horizon::Sides(2), 0);
    let nim = Nim::new(10);
    assert_eq!(
        Rollout::new(Unordered, nim_perfect, even, all, 0).choose(&nim),
        1
    );
    assert_eq!(
        Rollout::new(reversed, nim_perfect, even, all, 0).choose(&nim),
        3
    );
}

/// How many playout commands one decision on a pile of `pile` applies,
/// with one candidate and one sample.
fn playout_commands(pile: u32, horizon: Horizon, commands: u32) -> u32 {
    let played = Cell::new(0);
    let counting = |nim: &Nim| {
        played.set(played.get() + 1);
        nim_perfect(nim)
    };
    let mut bot = Rollout::new(
        Unordered,
        counting,
        won_or_lost::<Nim>,
        RolloutSettings {
            commands,
            ..settings(1, 1, horizon, 0)
        },
        0,
    );
    bot.choose(&Nim::new(pile));
    played.get()
}

#[test]
fn each_horizon_stops_a_playout() {
    // Each take changes the side to move, the candidate's included.
    assert_eq!(playout_commands(100, Horizon::Sides(0), 50), 0);
    assert_eq!(playout_commands(100, Horizon::Sides(1), 50), 0);
    assert_eq!(playout_commands(100, Horizon::Sides(3), 50), 2);
    // Turn 1 is the candidate and one reply; turn 2 starts after it.
    assert_eq!(playout_commands(100, Horizon::Turns(0), 50), 0);
    assert_eq!(playout_commands(100, Horizon::Turns(1), 50), 1);
    assert_eq!(playout_commands(100, Horizon::Turns(2), 50), 3);
    // The game's end stops it first: 1 is taken from 3, then 2 is left.
    assert_eq!(playout_commands(3, Horizon::Turns(20), 50), 1);
}

#[test]
fn the_commands_cap_stops_a_playout() {
    assert_eq!(playout_commands(1000, Horizon::Turns(1000), 5), 5);
    assert_eq!(playout_commands(1000, Horizon::Sides(u32::MAX), 0), 0);
}

#[test]
fn a_refused_playout_command_ends_the_playout_where_it_is() {
    let asked = Cell::new(0);
    let refused = |_: &Nim| {
        asked.set(asked.get() + 1);
        9_u32
    };
    let ends = RefCell::new(Vec::new());
    let record = |_: &Nim, end: &Nim, _: usize| {
        ends.borrow_mut().push(end.pile);
        0
    };
    let mut bot = Rollout::new(
        Unordered,
        refused,
        record,
        settings(3, 1, Horizon::Turns(20), 0),
        0,
    );
    bot.choose(&Nim::new(10));
    // Asked once a playout, never again after the refusal.
    assert_eq!(asked.get(), 3);
    assert_eq!(*ends.borrow(), [9, 8, 7]);
}

#[test]
fn no_decision_does_more_than_its_units() {
    for (candidates, samples, horizon, commands) in [
        (3, 4, Horizon::Sides(u32::MAX), 6),
        (2, 3, Horizon::Turns(u32::MAX), 0),
        (8, 1, Horizon::Turns(4), 10),
        (1, 9, Horizon::Sides(5), 2),
    ] {
        let played = Cell::new(0_u32);
        let counting = |nim: &Nim| {
            played.set(played.get() + 1);
            nim_perfect(nim)
        };
        let decision = RolloutSettings {
            commands,
            ..settings(candidates, samples, horizon, 0)
        };
        let mut bot = Rollout::new(Unordered, counting, won_or_lost::<Nim>, decision, 0);
        bot.choose(&Nim::new(1000));
        let tried = candidates.min(3) * samples;
        assert!(played.get() + tried <= tried * (commands + 1));
    }
}

#[test]
fn exp2_q16_is_two_to_the_power() {
    // 0, -0.5, -1, -2.25, -31 and -33 in 16.16.
    for (x, expected) in [
        (0, 65536),
        (-32768, 46341),
        (-65536, 32768),
        (-147_456, 13777),
        (-31 * 65536, 0),
        (-33 * 65536, 0),
    ] {
        assert_eq!(exp2_q16(x), expected, "2^({x} / 65536)");
    }
    // Between the table's entries it is read straight (2^(-1/32) is
    // 64 132), and never rises.
    assert_eq!(exp2_q16(-2048), 64147);
    assert_eq!(exp2_q16(-15 * 4096 - 2048), 33494);
    let mut last = u32::MAX;
    for x in 0..=4 * 65536 {
        let value = exp2_q16(-x);
        assert!(value <= last, "2^-{x}");
        last = value;
    }
    assert_eq!(exp2_q16(5 * 65536), 65536);
    assert_eq!(exp2_q16(i64::MIN), 0);
}

#[test]
fn temperature_zero_always_picks_the_best_mean() {
    let mut rng = SimRng::new(0);
    for means in [vec![3, 9, 4], vec![-5, -1, -9], vec![7], vec![2, 2, 1]] {
        let best = means.iter().max().copied().unwrap_or(0);
        let first = means.iter().position(|&m| m == best).unwrap_or(0);
        assert_eq!(pick(&means, 0, &mut rng), first, "{means:?}");
    }
}

#[test]
fn a_candidate_temperature_points_below_is_half_as_likely() {
    // Taking 2 scores 100, taking 1 scores 50: 50 points apart.
    let taken =
        |start: &Nim, end: &Nim, _: usize| i32::try_from(start.pile - end.pile).unwrap_or(0) * 50;
    let two = settings(2, 1, Horizon::Sides(0), 50);
    let mut picked = [0_u32; 3];
    for seed in 0..1000 {
        let take = Rollout::new(Unordered, nim_perfect, taken, two, seed).choose(&Nim::new(10));
        picked[usize::try_from(take).unwrap_or(0)] += 1;
    }
    let (worse, best) = (picked[1], picked[2]);
    assert!(
        worse * 10 >= best * 4 && worse * 10 <= best * 6,
        "{worse} against {best}"
    );
    // Far below at a low temperature, a candidate weighs nothing.
    let mut rng = SimRng::new(1);
    for _ in 0..100 {
        assert_eq!(pick(&[0, 1000], 1, &mut rng), 1);
    }
}

#[test]
fn a_roll_falls_in_its_weights_share() {
    let weights = [0, 5, 3];
    let shares: Vec<_> = (0..9).map(|roll| share(&weights, roll)).collect();
    let (one, two) = (Some(1), Some(2));
    assert_eq!(shares, [one, one, one, one, one, two, two, two, None]);
}

#[test]
fn a_decisions_seeds_are_two_draws_each_high_half_first() {
    let mut rng = SimRng::new(11);
    let draws: Vec<u64> = (0..6).map(|_| u64::from(rng.next_u32())).collect();
    let mut bot = nim_bot(1, 3, Horizon::Sides(1), 11);
    let search = bot.search(&Nim::new(5));
    let seeds: Vec<u64> = draws.chunks(2).map(|d| d[0] * (1 << 32) + d[1]).collect();
    assert_eq!(search.seeds, seeds);
}

#[test]
fn weights_are_halved_only_as_far_as_one_draw_needs() {
    let totals = [
        0,
        u64::from(u32::MAX),
        1 << 32,
        (1 << 33) - 1,
        1 << 33,
        u64::MAX,
    ];
    let shifts: Vec<u32> = totals.into_iter().map(fit_shift).collect();
    assert_eq!(shifts, [0, 0, 1, 1, 2, 32]);
}

#[test]
fn many_equal_weights_still_fit_one_draw() {
    let means = vec![0; 70_000];
    let mut rng = SimRng::new(2);
    assert!(pick(&means, 10, &mut rng) < means.len());
}

#[test]
#[should_panic(expected = "determinize changed the commands at the root")]
fn a_determinize_that_changes_the_commands_panics() {
    let mut bot = nim_bot(3, 1, Horizon::Sides(2), 0);
    bot.choose(&Nim::lying(2, Lie::Pile));
}

#[test]
#[should_panic(expected = "the copy from determinize refused candidate 0")]
fn a_candidate_the_copy_refuses_panics() {
    let mut bot = nim_bot(3, 1, Horizon::Sides(2), 0);
    bot.choose(&Nim::lying(5, Lie::Refuses));
}

#[test]
#[should_panic(expected = "no command to choose from")]
fn a_game_with_nothing_legal_panics() {
    let mut bot = nim_bot(3, 1, Horizon::Sides(2), 0);
    bot.choose(&Nim::new(0));
}

#[test]
fn settings_are_data() {
    let persona = settings(8, 4, Horizon::Turns(3), 20);
    let text = ron::to_string(&persona).unwrap_or_default();
    assert_eq!(
        text,
        "(candidates:8,samples:4,horizon:Turns(3),commands:30,temperature:20)"
    );
    assert_eq!(ron::from_str::<RolloutSettings>(&text).ok(), Some(persona));
    assert_eq!(nim_bot(1, 2, Horizon::Sides(1), 0).settings().samples, 2);
}

#[test]
fn a_bot_plays_a_race_to_its_end() {
    let mut race = DiceRace::new(9);
    let mut a = race_bot(1, 0);
    let mut b = race_bot(2, 30);
    let report = super::super::play(&mut race, &mut [(0, &mut a), (1, &mut b)], 100)
        .unwrap_or_else(|_| panic!("a refused command"));
    assert!(report.outcome.is_some());
    assert!(race.squares.iter().any(|&square| square >= FINISH));
}

proptest! {
    #[test]
    fn the_luck_to_come_never_changes_the_command(
        squares in [0..FINISH, 0..FINISH],
        luck in any::<(u64, u64)>(),
        seed in any::<u64>(),
    ) {
        let first = race_bot(seed, 25).choose(&DiceRace::at(squares, luck.0));
        let second = race_bot(seed, 25).choose(&DiceRace::at(squares, luck.1));
        prop_assert_eq!(first, second);
    }

    #[test]
    fn slicing_never_changes_the_command(
        pile in 1..40_u32,
        squares in [0..FINISH, 0..FINISH],
        slices in proptest::collection::vec(1..20_u32, 1..8),
        seed in any::<u64>(),
    ) {
        let nim = Nim::new(pile);
        let mut nim_whole = nim_bot(3, 3, Horizon::Turns(4), seed);
        let mut nim_sliced = nim_bot(3, 3, Horizon::Turns(4), seed);
        prop_assert_eq!(sliced(&mut nim_sliced, &nim, &slices), nim_whole.choose(&nim));

        let race = DiceRace::at(squares, seed);
        let (mut whole, mut parts) = (race_bot(seed, 25), race_bot(seed, 25));
        prop_assert_eq!(sliced(&mut parts, &race, &slices), whole.choose(&race));
        // The bot's next decision is the same too: the slicing drew nothing.
        prop_assert_eq!(parts.choose(&race), whole.choose(&race));
    }
}
