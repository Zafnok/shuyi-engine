//! Chapter 6, rules as commands: the garden's rules, a state that takes
//! commands and answers with what happened ([`TurnRules`]), its luck
//! from a seeded RNG kept in the state, its history, and the hash that
//! says two runs played the same.

use core::fmt;

use serde::{Deserialize, Serialize};
use shuyi::core::bots::Sides;
use shuyi::core::{Countdown, Every, SimRng, TurnRules};

use crate::assets::Plan;

// ANCHOR: state
/// The garden: every plot, planted or not, where the player stands, the
/// seeds left, and its luck. Everything the rules read is here, so the
/// same state and the same command always give the same answer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Garden {
    /// Each plot: whether something grows in it.
    plots: Vec<bool>,
    /// The plot the player stands at.
    at: usize,
    /// The seeds left.
    seeds: u32,
    /// How often a seed grows, in percent.
    grow_percent: u8,
    /// The garden's luck: the same seed, the same luck.
    rng: SimRng,
    /// The commands applied so far.
    turn: u32,
}

/// What the player asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Command {
    /// Move one plot left.
    Left,
    /// Move one plot right.
    Right,
    /// Plant a seed in the plot here.
    Plant,
}

/// What happened.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Event {
    /// The player moved to this plot.
    Moved(usize),
    /// The seed grew in this plot.
    Grew(usize),
    /// The seed didn't grow, and is gone.
    Wilted(usize),
}

/// Why a command was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    /// There is no plot that way.
    Edge,
    /// Something already grows here.
    Planted,
    /// The game is over.
    Over,
}

/// How the game ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Outcome {
    /// Something grows in every plot.
    Won,
    /// The seeds ran out first.
    Lost,
}
// ANCHOR_END: state

impl Garden {
    /// A garden laid out by `plan`, its luck from `seed`.
    pub fn new(plan: &Plan, seed: u64) -> Self {
        Self {
            plots: vec![false; usize::from(plan.plots)],
            at: 0,
            seeds: plan.seeds,
            grow_percent: plan.grow_percent,
            rng: SimRng::new(seed),
            turn: 0,
        }
    }

    /// Each plot: whether something grows in it.
    pub fn plots(&self) -> &[bool] {
        &self.plots
    }

    /// The plot the player stands at.
    pub fn at(&self) -> usize {
        self.at
    }

    /// The seeds left.
    pub fn seeds(&self) -> u32 {
        self.seeds
    }
}

// ANCHOR: rules
impl TurnRules for Garden {
    type Command = Command;
    type Event = Event;
    type Error = Refusal;
    type Outcome = Outcome;

    fn check(&self, command: &Command) -> Result<(), Refusal> {
        if self.outcome().is_some() {
            return Err(Refusal::Over);
        }
        match command {
            Command::Left if self.at == 0 => Err(Refusal::Edge),
            Command::Right if self.at + 1 >= self.plots.len() => Err(Refusal::Edge),
            Command::Plant if self.plots[self.at] => Err(Refusal::Planted),
            _ => Ok(()),
        }
    }

    fn apply(&mut self, command: &Command) -> Result<Vec<Event>, Refusal> {
        // All or nothing: a refused command changes nothing.
        self.check(command)?;
        self.turn += 1;
        let event = match command {
            Command::Left => {
                self.at -= 1;
                Event::Moved(self.at)
            }
            Command::Right => {
                self.at += 1;
                Event::Moved(self.at)
            }
            Command::Plant => {
                self.seeds -= 1;
                // Luck comes from the state's RNG, never from outside.
                if self.rng.roll_percent() < self.grow_percent {
                    self.plots[self.at] = true;
                    Event::Grew(self.at)
                } else {
                    Event::Wilted(self.at)
                }
            }
        };
        Ok(vec![event])
    }

    fn legal_commands(&self) -> Vec<Command> {
        [Command::Left, Command::Right, Command::Plant]
            .into_iter()
            .filter(|command| self.check(command).is_ok())
            .collect()
    }

    fn outcome(&self) -> Option<Outcome> {
        if self.plots.iter().all(|&grows| grows) {
            Some(Outcome::Won)
        } else if self.seeds == 0 {
            Some(Outcome::Lost)
        } else {
            None
        }
    }
}
// ANCHOR_END: rules

// ANCHOR: sides
/// Who moves, for the bots: a garden has one player, so its side is `()`.
impl Sides for Garden {
    type Side = ();

    fn side_to_move(&self) {}

    fn turn(&self) -> u32 {
        self.turn
    }
}
// ANCHOR_END: sides

// ANCHOR: time
/// What the garden times, in its state: rain every few turns, and a watering
/// can that needs a wait between uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Seasons {
    rain: Every,
    can: Countdown,
}

impl Seasons {
    /// Rain every `rain` turns, and a can ready now, then every `can` turns.
    pub fn new(rain: u32, can: u32) -> Self {
        Self {
            rain: Every::new(rain),
            can: Countdown::new_ready(can),
        }
    }

    /// Where a turn ends: whether it rains.
    pub fn end_turn(&mut self) -> bool {
        self.can.advance(1);
        self.rain.advance(1) > 0
    }

    /// Uses the can if it's ready, and says whether it was.
    pub fn water(&mut self) -> bool {
        let ready = self.can.ready();
        if ready {
            self.can.restart();
        }
        ready
    }
}
// ANCHOR_END: time

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // check-text: not player text, what a replay's step says
        f.write_str(match self {
            Self::Edge => "no plot that way",
            Self::Planted => "something grows here already",
            Self::Over => "the game is over",
        })
    }
}

#[cfg(test)]
mod tests {
    use shuyi::core::{History, state_hash};

    use super::*;

    /// The book's plan: six plots, nine seeds, three in four grow.
    fn plan() -> Plan {
        Plan::new(6, 9, 75)
    }

    #[test]
    fn each_applied_command_is_a_turn_and_a_refused_one_isnt() {
        let mut garden = Garden::new(&plan(), 7);
        assert_eq!(garden.turn(), 0);
        garden.apply(&Command::Right).unwrap();
        garden.apply(&Command::Right).unwrap();
        assert!(garden.apply(&Command::Plant).is_ok());
        let _ = garden.apply(&Command::Left);
        assert_eq!(garden.turn(), 4);
        let mut edge = Garden::new(&plan(), 7);
        assert!(edge.apply(&Command::Left).is_err());
        assert_eq!(edge.turn(), 0);
    }

    #[test]
    fn a_seed_grows_only_on_a_roll_under_its_chance() {
        // The garden's first roll, from the same seed: a chance equal to it
        // isn't enough.
        let roll = SimRng::new(5).roll_percent();
        let mut garden = Garden::new(&Plan::new(2, 3, roll), 5);
        assert_eq!(garden.apply(&Command::Plant).unwrap(), [Event::Wilted(0)]);
        let mut garden = Garden::new(&Plan::new(2, 3, roll + 1), 5);
        assert_eq!(garden.apply(&Command::Plant).unwrap(), [Event::Grew(0)]);
    }

    // ANCHOR: test
    #[test]
    fn planting_at_the_edge_and_moving_past_it() {
        let mut garden = Garden::new(&plan(), 7);
        // A refused command changes nothing.
        assert_eq!(garden.apply(&Command::Left), Err(Refusal::Edge));
        assert_eq!(garden, Garden::new(&plan(), 7));
        let events = garden.apply(&Command::Plant).unwrap();
        assert!(matches!(events[..], [Event::Grew(0) | Event::Wilted(0)]));
        assert_eq!(garden.seeds(), 8);
    }
    // ANCHOR_END: test

    // ANCHOR: history
    #[test]
    fn a_history_rewinds_by_replaying_its_commands() {
        let mut history = History::new(Garden::new(&plan(), 7));
        for command in [Command::Plant, Command::Right, Command::Plant] {
            history.push(command);
        }
        let now = history.state_at(history.len());
        assert_eq!(now.at(), 1);
        // Back to the game after its first command: the rest is forgotten.
        let then = history.rewind_to(1).unwrap();
        assert_eq!(then.at(), 0);
        assert_eq!(history.commands(), [Command::Plant]);
    }
    // ANCHOR_END: history

    // ANCHOR: hash
    #[test]
    fn the_same_seed_and_commands_give_the_same_hash() {
        let play = |seed| {
            let mut garden = Garden::new(&plan(), seed);
            for command in [Command::Plant, Command::Right, Command::Plant] {
                garden.apply(&command).unwrap();
            }
            state_hash(&garden).unwrap()
        };
        assert_eq!(play(7), play(7));
    }
    // ANCHOR_END: hash

    #[test]
    fn it_rains_every_third_turn_and_the_can_waits_two() {
        let mut seasons = Seasons::new(3, 2);
        assert!(seasons.water());
        assert!(!seasons.water());
        let rained = [0; 6].map(|_| seasons.end_turn());
        assert_eq!(rained, [false, false, true, false, false, true]);
        assert!(seasons.water());
    }

    #[test]
    fn every_plot_grown_wins_and_no_seed_left_loses() {
        let mut garden = Garden::new(&Plan::new(1, 3, 100), 0);
        assert_eq!(garden.legal_commands(), [Command::Plant]);
        garden.apply(&Command::Plant).unwrap();
        assert_eq!(garden.outcome(), Some(Outcome::Won));
        assert_eq!(garden.legal_commands(), []);
        let mut garden = Garden::new(&Plan::new(2, 1, 0), 0);
        assert_eq!(garden.apply(&Command::Plant).unwrap(), [Event::Wilted(0)]);
        assert_eq!(garden.outcome(), Some(Outcome::Lost));
        assert_eq!(garden.apply(&Command::Right), Err(Refusal::Over));
        assert_eq!(Refusal::Over.to_string(), "the game is over");
    }
}
