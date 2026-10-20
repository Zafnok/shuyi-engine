//! Tests of the history over a toy game.

use alloc::string::ToString;
use alloc::vec;

use proptest::prelude::*;

use super::*;
use crate::rng::SimRng;

/// A toy game: a running total that each command adds to, with a roll of
/// luck per add. Adding 0, or anything once the total has reached 10, is
/// refused.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Tally {
    total: u32,
    rolls: Vec<u8>,
    rng: SimRng,
}

/// What an add did: the new total and the roll.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Added(u32, u8);

/// Why an add was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Refused {
    Nothing,
    Over,
}

impl Tally {
    fn new(seed: u64) -> Self {
        Tally {
            total: 0,
            rolls: Vec::new(),
            rng: SimRng::new(seed),
        }
    }
}

impl TurnRules for Tally {
    type Command = u32;
    type Event = Added;
    type Error = Refused;
    type Outcome = u32;

    fn check(&self, add: &u32) -> Result<(), Refused> {
        if self.total >= 10 {
            Err(Refused::Over)
        } else if *add == 0 {
            Err(Refused::Nothing)
        } else {
            Ok(())
        }
    }

    fn apply(&mut self, add: &u32) -> Result<Vec<Added>, Refused> {
        self.check(add)?;
        self.total += add;
        let roll = self.rng.roll_percent();
        self.rolls.push(roll);
        Ok(vec![Added(self.total, roll)])
    }

    fn legal_commands(&self) -> Vec<u32> {
        if self.total >= 10 {
            vec![]
        } else {
            vec![1, 2, 3]
        }
    }

    fn outcome(&self) -> Option<u32> {
        (self.total >= 10).then_some(self.total)
    }
}

/// A history of `adds` from a fresh game, with the game after each point.
fn played(adds: &[u32]) -> (History<Tally>, Vec<Tally>) {
    let mut game = Tally::new(7);
    let mut history = History::new(game.clone());
    let mut points = vec![game.clone()];
    for add in adds {
        let _ = game.apply(add);
        history.push(*add);
        points.push(game.clone());
    }
    (history, points)
}

#[test]
fn a_new_history_has_its_first_state_and_no_commands() {
    let history = History::new(Tally::new(1));
    assert!(history.is_empty());
    assert_eq!(history.len(), 0);
    assert_eq!(history.commands(), &[] as &[u32]);
    assert_eq!(history.initial(), &Tally::new(1));
    assert_eq!(history.state_at(0), Tally::new(1));
    assert_eq!(history.state_at(3), Tally::new(1));
    assert!(history.replay().is_empty());
}

#[test]
fn point_n_is_the_game_after_its_first_n_commands() {
    let (history, points) = played(&[2, 3, 1]);
    assert!(!history.is_empty());
    assert_eq!(history.len(), 3);
    assert_eq!(history.commands(), &[2, 3, 1]);
    for (n, point) in points.iter().enumerate() {
        assert_eq!(&history.state_at(n), point, "point {n}");
    }
    assert_eq!(history.state_at(9), points[3], "past the end: all of them");
    assert_eq!(points[3].total, 6);
    assert_eq!(points[3].rolls.len(), 3, "the luck replays too");
}

#[test]
fn a_refused_command_replays_as_nothing() {
    let (history, points) = played(&[4, 0, 5]);
    assert_eq!(history.state_at(2), points[1]);
    assert_eq!(history.state_at(3).total, 9);
    let replayed = history.replay();
    assert_eq!(replayed[1].events, []);
    assert_eq!(replayed[1].command, 0);
}

#[test]
fn replay_lists_each_command_with_the_game_before_it_and_its_events() {
    let (history, points) = played(&[2, 3]);
    let replayed = history.replay();
    assert_eq!(replayed.len(), 2);
    for (i, r) in replayed.iter().enumerate() {
        assert_eq!(r.before, points[i]);
        assert_eq!(r.command, history.commands()[i]);
        let mut game = points[i].clone();
        assert_eq!(Ok(r.events.clone()), game.apply(&r.command));
    }
    assert_eq!(replayed[1].events, [Added(5, points[2].rolls[1])]);
}

#[test]
fn a_rewind_forgets_the_point_and_everything_after() {
    let (mut history, points) = played(&[1, 2, 3]);
    assert_eq!(history.rewind_to(1), Ok(points[1].clone()));
    assert_eq!(history.commands(), &[1]);
    assert_eq!(history.rewind_to(0), Ok(points[0].clone()));
    assert!(history.is_empty());
}

#[test]
fn a_rewind_to_no_earlier_point_is_refused_and_changes_nothing() {
    let (mut history, _) = played(&[1, 2]);
    assert_eq!(history.rewind_to(2), Err(NoSuchPoint(2)));
    assert_eq!(history.rewind_to(5), Err(NoSuchPoint(5)));
    assert_eq!(history.commands(), &[1, 2]);
    assert_eq!(
        History::new(Tally::new(1)).rewind_to(0),
        Err(NoSuchPoint(0))
    );
    assert_eq!(
        NoSuchPoint(4).to_string(),
        "no earlier point 4 to rewind to"
    );
}

#[test]
fn doing_the_same_after_a_rewind_gives_the_same_result() {
    let (mut history, points) = played(&[2, 2]);
    let mut game = history.rewind_to(1).unwrap();
    assert_eq!(game.apply(&2), Ok(vec![Added(4, points[2].rolls[1])]));
    assert_eq!(game, points[2]);
}

#[test]
fn a_history_comes_apart_and_back_together() {
    let (history, _) = played(&[3, 1]);
    let (initial, commands) = history.clone().into_parts();
    assert_eq!(initial, Tally::new(7));
    assert_eq!(commands, [3, 1]);
    assert_eq!(History::from_parts(initial, commands), history);
}

#[test]
fn the_first_state_can_be_changed() {
    let (mut history, _) = played(&[3]);
    history.initial_mut().total = 5;
    assert_eq!(history.initial().total, 5);
    assert_eq!(history.state_at(1).total, 8);
}

#[test]
fn a_history_is_saved_as_its_first_state_and_commands() {
    let (history, _) = played(&[2, 3]);
    let text = ron::to_string(&history).unwrap();
    assert!(
        text.starts_with("(initial:(total:0,rolls:[],rng:("),
        "{text}"
    );
    assert!(text.ends_with("),commands:[2,3])"), "{text}");
    assert_eq!(ron::from_str::<History<Tally>>(&text).unwrap(), history);
}

proptest! {
    #[test]
    fn every_point_replays_and_rewinds_to_the_game_as_it_was(
        adds in prop::collection::vec(0u32..4, 0..12),
        n in 0usize..14,
    ) {
        let (mut history, points) = played(&adds);
        for (i, point) in points.iter().enumerate() {
            prop_assert_eq!(&history.state_at(i), point);
        }
        if n < adds.len() {
            prop_assert_eq!(history.rewind_to(n), Ok(points[n].clone()));
            prop_assert_eq!(history.commands(), &adds[..n]);
        } else {
            prop_assert_eq!(history.rewind_to(n), Err(NoSuchPoint(n)));
            prop_assert_eq!(history.commands(), &adds[..]);
        }
    }
}
