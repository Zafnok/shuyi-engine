//! The engine's bots playing the stage's rules (`shuyi_core::bots`): a
//! random bot for your piece against the foe's own mind, and against
//! another random bot, and a rollout bot that plans on copies of the
//! stage against the foe. Every command a bot chooses is one the rules
//! accept, every play ends, and the same seeds play the same game.

use hello_grid::Hello;
use hello_grid::rules::{Chase, Outcome, Side, Stage, hero_order, playout, score};
use shuyi_core::{
    Bot, Determinize, Horizon, RandomBot, Report, Rollout, RolloutSettings, Sides, TurnRules, play,
};

/// The stage of the game's map, seeded with `seed`.
fn stage(seed: u64) -> Stage {
    let game = Hello::load().unwrap_or_else(|e| panic!("{e:?}"));
    Stage::new(&game.stage_map, seed).unwrap_or_else(|e| panic!("{e}"))
}

/// A play of stage `seed` to its end (or turn 200), your piece moved by
/// a random bot of `seed`, the foe by `foe`.
fn play_with(seed: u64, foe: &mut dyn Bot<Stage>) -> Report<Stage> {
    play_hero(seed, &mut RandomBot::new(seed), foe)
}

/// A play of stage `seed` to its end (or turn 200), your piece moved by
/// `hero`, the foe by `foe`.
fn play_hero(seed: u64, hero: &mut dyn Bot<Stage>, foe: &mut dyn Bot<Stage>) -> Report<Stage> {
    let mut stage = stage(seed);
    let mut bots: [(Side, &mut dyn Bot<Stage>); 2] = [(Side::Hero, hero), (Side::Foe, foe)];
    play(&mut stage, &mut bots, 200).unwrap_or_else(|e| panic!("seed {seed}: {e}"))
}

/// A rollout bot for your piece: 8 candidates nearest the goal, each
/// played out on 4 copies for 3 turns, the best taken.
fn rollout_hero(seed: u64) -> impl Bot<Stage> {
    let settings = RolloutSettings::new(8, 4, Horizon::Turns(3), 12, 0);
    Rollout::new(hero_order, playout, score, settings, seed)
}

/// How many of stages 0 to 19 your piece wins against the chaser, moved
/// by the bot `hero` makes for each seed.
fn wins<B: Bot<Stage>>(hero: impl Fn(u64) -> B) -> usize {
    (0..20)
        .filter(|&seed| {
            let report = play_hero(seed, &mut hero(seed), &mut Chase);
            matches!(report.outcome, Some(Outcome::Won { .. }))
        })
        .count()
}

#[test]
fn a_random_bot_against_the_chaser_always_plays_to_an_end() {
    let mut won = 0;
    let mut caught = 0;
    for seed in 0..60 {
        let report = play_with(seed, &mut Chase);
        match report.outcome {
            Some(Outcome::Won { turn }) => {
                won += 1;
                assert_eq!(turn, report.turns, "seed {seed}");
            }
            Some(Outcome::Caught { turn }) => {
                caught += 1;
                assert_eq!(turn, report.turns, "seed {seed}");
            }
            None => panic!("seed {seed}: no end in 200 turns"),
        }
        // A command for each side's move: yours, then the foe's.
        assert!(!report.commands.is_empty(), "{:?}", report.commands);
    }
    // A random walker sometimes finds the goal, and is sometimes caught.
    assert!(won > 0 && caught > 0, "won {won}, caught {caught}");
}

#[test]
fn two_random_bots_play_only_legal_commands() {
    for seed in 0..30 {
        let report = play_with(seed, &mut RandomBot::new(seed + 1000));
        assert!(report.turns >= 1);
    }
}

#[test]
fn the_same_seeds_play_the_same_game() {
    for seed in [0, 7, 42] {
        assert_eq!(play_with(seed, &mut Chase), play_with(seed, &mut Chase));
    }
}

#[test]
fn a_rollout_hero_wins_more_than_a_random_one() {
    let planned = wins(rollout_hero);
    let random = wins(RandomBot::new);
    // Recorded in ticket 2125's completion notes.
    eprintln!("of stages 0 to 19, the rollout hero won {planned}, the random hero {random}");
    assert!(planned > random, "rollout {planned}, random {random}");
}

#[test]
fn a_rollout_hero_never_sees_the_luck_to_come() {
    let first = rollout_hero(3).choose(&stage(0));
    for seed in 1..20 {
        assert_eq!(
            rollout_hero(3).choose(&stage(seed)),
            first,
            "stage seed {seed}"
        );
    }
}

#[test]
fn the_foe_scores_the_negation_of_the_hero() {
    let start = stage(0);
    let mut end = start.clone();
    let open = score(&start, &end, Side::Hero);
    // A step that brings your piece nearer the goal scores better for it.
    let nearer = hero_order(&end, end.legal_commands())[0];
    end.apply(&nearer).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(end.side_to_move(), Side::Foe);
    let hero = score(&start, &end, Side::Hero);
    assert!(hero > open);
    assert_ne!(hero, 0);
    assert_eq!(score(&start, &end, Side::Foe), -hero);
}

#[test]
fn a_finished_stage_scores_its_outcome() {
    let mut seen = (false, false);
    for seed in 0..60 {
        let mut end = stage(seed);
        let mut hero = RandomBot::new(seed);
        let mut bots: [(Side, &mut dyn Bot<Stage>); 2] =
            [(Side::Hero, &mut hero), (Side::Foe, &mut Chase)];
        play(&mut end, &mut bots, 200).unwrap_or_else(|e| panic!("seed {seed}: {e}"));
        let hero = match end.outcome() {
            Some(Outcome::Won { .. }) => {
                seen.0 = true;
                1000
            }
            Some(Outcome::Caught { .. }) => {
                seen.1 = true;
                -1000
            }
            None => continue,
        };
        assert_eq!(score(&stage(seed), &end, Side::Hero), hero, "seed {seed}");
        assert_eq!(score(&stage(seed), &end, Side::Foe), -hero, "seed {seed}");
    }
    assert_eq!(seen, (true, true));
}

#[test]
fn a_determinized_stage_has_only_its_luck_redrawn() {
    // Two stages that differ only in their luck are the same copy.
    let copy = stage(1).determinize(Side::Hero, 99);
    assert_eq!(stage(2).determinize(Side::Hero, 99), copy);
    assert_eq!(copy, stage(99));
    assert_ne!(copy, stage(1));
}
