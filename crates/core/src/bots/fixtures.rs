//! Toy games for the bots' tests: a dice race (luck in the state) and Nim
//! (no luck, nothing hidden).

use alloc::vec;
use alloc::vec::Vec;

use super::{Determinize, Sides};
use crate::rng::SimRng;
use crate::turn::TurnRules;

/// The square a dice race is won on.
pub(crate) const FINISH: u32 = 20;

/// A dice race's command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Race {
    /// Steps 2.
    Step,
    /// Rolls 1 to 6 from the state's luck and moves that many.
    Roll,
}

/// Two sides take turns; the first to reach [`FINISH`] wins. A turn is
/// one move of each.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DiceRace {
    pub(crate) squares: [u32; 2],
    moves: u32,
    rng: SimRng,
}

impl DiceRace {
    /// A race from the start, its luck seeded with `seed`.
    pub(crate) fn new(seed: u64) -> Self {
        Self::at([0, 0], seed)
    }

    /// A race with the sides on `squares`, side 0 to move.
    pub(crate) fn at(squares: [u32; 2], seed: u64) -> Self {
        Self {
            squares,
            moves: 0,
            rng: SimRng::new(seed),
        }
    }
}

impl TurnRules for DiceRace {
    type Command = Race;
    /// The square the side moved to.
    type Event = u32;
    type Error = ();
    /// The side that won.
    type Outcome = usize;

    fn check(&self, _: &Race) -> Result<(), ()> {
        if self.outcome().is_some() {
            Err(())
        } else {
            Ok(())
        }
    }

    fn apply(&mut self, command: &Race) -> Result<Vec<u32>, ()> {
        self.check(command)?;
        let side = self.side_to_move();
        let steps = match command {
            Race::Step => 2,
            Race::Roll => self.rng.roll_below(6) + 1,
        };
        self.squares[side] += steps;
        self.moves += 1;
        Ok(vec![self.squares[side]])
    }

    fn legal_commands(&self) -> Vec<Race> {
        if self.outcome().is_some() {
            Vec::new()
        } else {
            vec![Race::Step, Race::Roll]
        }
    }

    fn outcome(&self) -> Option<usize> {
        self.squares.iter().position(|&square| square >= FINISH)
    }
}

impl Sides for DiceRace {
    type Side = usize;

    fn side_to_move(&self) -> usize {
        usize::try_from(self.moves % 2).unwrap_or(0)
    }

    fn turn(&self) -> u32 {
        self.moves / 2 + 1
    }
}

impl Determinize for DiceRace {
    /// The luck to come is redrawn; nothing else is hidden.
    fn determinize(&self, _: usize, seed: u64) -> Self {
        Self {
            rng: SimRng::new(seed),
            ..self.clone()
        }
    }
}

/// How a [`Nim`]'s `determinize` goes wrong, for the tests of the bot's
/// checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Lie {
    /// It doesn't.
    None,
    /// The copy has one less on the pile.
    Pile,
    /// The copy lists the right commands but refuses them all.
    Refuses,
}

/// Nim: take 1 to 3 from the pile; the last take wins. A turn is one
/// take of each.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Nim {
    pub(crate) pile: u32,
    moves: u32,
    lie: Lie,
    refuses: bool,
}

impl Nim {
    /// A pile of `pile`, side 0 to take.
    pub(crate) fn new(pile: u32) -> Self {
        Self::lying(pile, Lie::None)
    }

    /// [`Nim::new`] with a `determinize` that lies as `lie` says.
    pub(crate) fn lying(pile: u32, lie: Lie) -> Self {
        Self {
            pile,
            moves: 0,
            lie,
            refuses: false,
        }
    }
}

impl TurnRules for Nim {
    /// How many to take.
    type Command = u32;
    /// The pile left.
    type Event = u32;
    type Error = ();
    /// The side that took last.
    type Outcome = usize;

    fn check(&self, take: &u32) -> Result<(), ()> {
        if self.refuses || !(1..=3).contains(take) || *take > self.pile {
            Err(())
        } else {
            Ok(())
        }
    }

    fn apply(&mut self, take: &u32) -> Result<Vec<u32>, ()> {
        self.check(take)?;
        self.pile -= take;
        self.moves += 1;
        Ok(vec![self.pile])
    }

    fn legal_commands(&self) -> Vec<u32> {
        (1..=self.pile.min(3)).collect()
    }

    fn outcome(&self) -> Option<usize> {
        // The side that took last is the one not to move.
        (self.pile == 0 && self.moves > 0).then(|| 1 - self.side_to_move())
    }
}

impl Sides for Nim {
    type Side = usize;

    fn side_to_move(&self) -> usize {
        usize::try_from(self.moves % 2).unwrap_or(0)
    }

    fn turn(&self) -> u32 {
        self.moves / 2 + 1
    }
}

impl Determinize for Nim {
    /// No luck and nothing hidden: the state itself (unless it lies).
    fn determinize(&self, _: usize, _: u64) -> Self {
        match self.lie {
            Lie::None => self.clone(),
            Lie::Pile => Self {
                pile: self.pile - 1,
                ..self.clone()
            },
            Lie::Refuses => Self {
                refuses: true,
                ..self.clone()
            },
        }
    }
}

/// Nim's perfect player: takes `pile % 4` (1 when that is 0).
pub(crate) fn nim_perfect(nim: &Nim) -> u32 {
    match nim.pile % 4 {
        0 => 1,
        take => take,
    }
}

/// A finished game's score for `side`: 1 000 for a win, -1 000 for a
/// loss, 0 when unfinished.
pub(crate) fn won_or_lost<R: TurnRules<Outcome = usize> + Sides<Side = usize>>(
    _: &R,
    end: &R,
    side: usize,
) -> i32 {
    match end.outcome() {
        Some(winner) if winner == side => 1000,
        Some(_) => -1000,
        None => 0,
    }
}
