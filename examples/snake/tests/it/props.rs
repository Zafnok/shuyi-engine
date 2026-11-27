//! The rules under random input: 3 000 steps of any presses never panic,
//! the body never overlaps itself while the snake lives, it is never longer
//! than the board's floor, and a record replays to the same hashes twice.

use proptest::prelude::*;
use shuyi::core::prelude::{ActionId, SeatInput, TickInput, TickRecord, TickSim, tick::replay};
use snake::Own;
use snake::rules::Snake;

/// The game's own part, from its files.
fn own() -> Own {
    snake::game().data().unwrap_or_else(|e| panic!("{e:?}")).own
}

/// A step's input: each of the game's six actions pressed or not (`bits`).
fn input(bits: u8) -> TickInput {
    let pressed = (0..6)
        .filter(|i| bits & (1 << i) != 0)
        .map(ActionId)
        .collect();
    TickInput::one(SeatInput::new(pressed, Vec::new(), Vec::new()))
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(48))]

    #[test]
    fn random_input_keeps_the_body_whole_and_on_the_board(
        seed in any::<u64>(),
        // Mostly quiet steps: a press is one step in 8.
        presses in proptest::collection::vec(prop_oneof![7 => Just(0u8), 1 => any::<u8>()], 3_000),
    ) {
        let own = own();
        let mut snake = Snake::new(&own.board, own.tuning, seed).unwrap_or_else(|e| panic!("{e}"));
        let floor = snake.floor_tiles();
        for &bits in &presses {
            snake.step(&input(bits));
            prop_assert!(snake.length() <= floor);
            if snake.outcome().is_none() {
                let body = snake.body();
                for (i, a) in body.iter().enumerate() {
                    prop_assert!(!body[i + 1..].contains(a), "{body:?}");
                    prop_assert_eq!(snake.board().get(*a), Some(&snake::rules::FLOOR));
                }
            }
        }
    }

    #[test]
    fn a_record_replays_to_the_same_hashes_twice(
        seed in any::<u64>(),
        presses in proptest::collection::vec(prop_oneof![7 => Just(0u8), 1 => any::<u8>()], 600),
    ) {
        let own = own();
        let made = snake::made();
        let mut record = TickRecord::new(made.clone(), seed);
        for &bits in &presses {
            record.push(input(bits));
        }
        let play = || {
            let mut snake = Snake::new(&own.board, own.tuning, seed).unwrap_or_else(|e| panic!("{e}"));
            replay(&mut snake, &record, &made).unwrap_or_else(|e| panic!("{e}"))
        };
        let first = play();
        prop_assert_eq!(first.len(), presses.len());
        prop_assert_eq!(first, play());
    }
}
