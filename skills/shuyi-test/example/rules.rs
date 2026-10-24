//! The rules, command by command: commands in, the state and the events
//! out. No screen, no window, no frame: the rules alone, and a property
//! that holds for any seed and any steps.

use proptest::prelude::*;
use rules::{Command, Event, Outcome, Stage, parse_map};
use shuyi_core::{Dir, Pos, TurnRules};

/// A corridor to the goal; the wanderer is walled in, so it never moves.
const CORRIDOR: &str = "########\n#...>#.#\n########\n";

/// An open room: the wanderer can reach your piece.
const ROOM: &str = "#######\n#....>#\n#.....#\n#.....#\n#######\n";

/// The stage of the map of `rows`, your piece starting at `hero` and the
/// wanderer at `wanderer`, its luck seeded `seed`.
fn stage(rows: &str, hero: (i32, i32), wanderer: (i32, i32), seed: u64) -> Stage {
    let source = format!(
        "(name: \"test\", legend: {{ '.': \"floor\", '#': \"wall\", '>': \"goal\" }}, \
         features: {{ {hero:?}: Hero, {wanderer:?}: Wanderer }})\n---\n{rows}"
    );
    let map = parse_map("test.map", &source).unwrap_or_else(|e| panic!("{e}"));
    Stage::new(&map, seed).unwrap_or_else(|e| panic!("{e}"))
}

#[test]
fn three_steps_right_reach_the_goal_and_end_the_stage() {
    let mut s = stage(CORRIDOR, (1, 1), (6, 1), 0);
    s.apply(&Command::Step(Dir::Right)).unwrap();
    s.apply(&Command::Step(Dir::Right)).unwrap();
    let events = s.apply(&Command::Step(Dir::Right)).unwrap();
    assert_eq!(
        events,
        [
            Event::Stepped {
                from: Pos::new(3, 1),
                to: Pos::new(4, 1)
            },
            Event::Reached
        ]
    );
    assert_eq!(s.outcome(), Some(Outcome::Won { turn: 3 }));
    assert_eq!(s.legal_commands(), []);
}

#[test]
fn a_wall_refuses_a_step_and_changes_nothing() {
    let mut s = stage(CORRIDOR, (1, 1), (6, 1), 0);
    let before = s.clone();
    assert!(s.apply(&Command::Step(Dir::Up)).is_err());
    assert_eq!(s, before);
}

proptest! {
    /// Whatever the seed and the steps tried, the same seed and the same
    /// steps give the same stage, and your piece stands on an open tile.
    #[test]
    fn the_same_seed_and_steps_give_the_same_stage(
        seed in any::<u64>(),
        steps in prop::collection::vec(0..4_usize, 0..40),
    ) {
        let play = || {
            let mut s = stage(ROOM, (1, 1), (4, 3), seed);
            for &i in &steps {
                let command = Command::Step(Dir::ALL[i]);
                if s.check(&command).is_ok() {
                    s.apply(&command).unwrap();
                }
            }
            s
        };
        let s = play();
        prop_assert_eq!(&s, &play());
        prop_assert!(s.open(s.hero()));
    }
}
