//! The game's rules, pure and deterministic over the engine's
//! [`TurnRules`]: tic-tac-toe on a board of three by three cells. X goes
//! first; each turn the side to move marks an empty cell, and three marks
//! of one side in a row, a column or a diagonal win. A full board with no
//! such line is a draw. The same commands always play out the same, so a
//! record of a game is its commands.
#![cfg_attr(not(test), deny(clippy::float_arithmetic))]

use std::fmt;

use serde::{Deserialize, Serialize};
use shuyi_core::{Pos, Sides, TurnRules};

/// The board's side, in cells.
pub const SIZE: i32 = 3;

/// The eight lines of three: the rows, the columns and the diagonals, as
/// cell indexes ([`index`]).
const LINES: [[usize; 3]; 8] = [
    [0, 1, 2],
    [3, 4, 5],
    [6, 7, 8],
    [0, 3, 6],
    [1, 4, 7],
    [2, 5, 8],
    [0, 4, 8],
    [2, 4, 6],
];

/// A side, and the mark it puts down.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Mark {
    /// X, who goes first.
    X,
    /// O.
    O,
}

impl Mark {
    /// The other side.
    #[must_use]
    pub const fn other(self) -> Self {
        match self {
            Self::X => Self::O,
            Self::O => Self::X,
        }
    }
}

/// What a side asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Command {
    /// The side to move marks this cell.
    Place(Pos),
}

/// What happened.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    /// A cell was marked.
    Placed {
        /// The mark.
        mark: Mark,
        /// The cell.
        at: Pos,
    },
    /// The game ended.
    Ended(Outcome),
}

/// Why a command was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// The game is over.
    Over,
    /// That cell is off the board.
    Outside(Pos),
    /// That cell is marked already.
    Taken(Pos),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Over => f.write_str("the game is over"),
            Self::Outside(p) => write!(f, "({}, {}) is off the board", p.x, p.y),
            Self::Taken(p) => write!(f, "({}, {}) is marked already", p.x, p.y),
        }
    }
}

/// How the game ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Outcome {
    /// This side made a line on this turn.
    Won {
        /// The side.
        mark: Mark,
        /// The turn.
        turn: u32,
    },
    /// The board filled with no line, on this turn.
    Draw {
        /// The turn.
        turn: u32,
    },
}

/// The index of cell `pos` in [`Board::cells`], if it is on the board.
pub fn index(pos: Pos) -> Option<usize> {
    let on = (0..SIZE).contains(&pos.x) && (0..SIZE).contains(&pos.y);
    on.then(|| usize::try_from(pos.y * SIZE + pos.x).ok())
        .flatten()
}

/// A game being played: the cells, the side to move, the turn and, once it
/// is over, how it ended. It serializes for its state hash
/// (`shuyi_core::state_hash`), which a replay compares step by step.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Board {
    cells: [Option<Mark>; 9],
    to_move: Mark,
    turn: u32,
    outcome: Option<Outcome>,
}

impl Default for Board {
    fn default() -> Self {
        Self::new()
    }
}

impl Board {
    /// An empty board, X to move: turn 1.
    pub const fn new() -> Self {
        Self {
            cells: [None; 9],
            to_move: Mark::X,
            turn: 1,
            outcome: None,
        }
    }

    /// The mark on cell `pos`, if any.
    pub fn at(&self, pos: Pos) -> Option<Mark> {
        index(pos).and_then(|i| self.cells[i])
    }

    /// The side that made a line, if one did.
    fn line(&self) -> Option<Mark> {
        LINES.iter().find_map(|&[a, b, c]| {
            let mark = self.cells[a]?;
            (self.cells[b] == Some(mark) && self.cells[c] == Some(mark)).then_some(mark)
        })
    }
}

impl TurnRules for Board {
    type Command = Command;
    type Event = Event;
    type Error = Error;
    type Outcome = Outcome;

    fn check(&self, command: &Command) -> Result<(), Error> {
        let Command::Place(pos) = *command;
        if self.outcome.is_some() {
            return Err(Error::Over);
        }
        let i = index(pos).ok_or(Error::Outside(pos))?;
        if self.cells[i].is_some() {
            return Err(Error::Taken(pos));
        }
        Ok(())
    }

    fn apply(&mut self, command: &Command) -> Result<Vec<Event>, Error> {
        self.check(command)?;
        let Command::Place(at) = *command;
        let mark = self.to_move;
        if let Some(i) = index(at) {
            self.cells[i] = Some(mark);
        }
        let mut events = vec![Event::Placed { mark, at }];
        let ended = match self.line() {
            Some(mark) => Some(Outcome::Won {
                mark,
                turn: self.turn,
            }),
            None if self.cells.iter().all(Option::is_some) => {
                Some(Outcome::Draw { turn: self.turn })
            }
            None => None,
        };
        if let Some(outcome) = ended {
            self.outcome = Some(outcome);
            events.push(Event::Ended(outcome));
        } else {
            self.to_move = mark.other();
            self.turn += 1;
        }
        Ok(events)
    }

    /// Each empty cell, row by row.
    fn legal_commands(&self) -> Vec<Command> {
        (0..SIZE)
            .flat_map(|y| (0..SIZE).map(move |x| Command::Place(Pos::new(x, y))))
            .filter(|c| self.check(c).is_ok())
            .collect()
    }

    fn outcome(&self) -> Option<Outcome> {
        self.outcome
    }
}

impl Sides for Board {
    type Side = Mark;

    fn side_to_move(&self) -> Mark {
        self.to_move
    }

    fn turn(&self) -> u32 {
        self.turn
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The board after placing at each of `cells`, in order.
    fn played(cells: &[(i32, i32)]) -> Board {
        let mut board = Board::new();
        for &(x, y) in cells {
            board.apply(&Command::Place(Pos::new(x, y))).unwrap();
        }
        board
    }

    #[test]
    fn x_goes_first_and_the_sides_take_turns() {
        let mut board = Board::new();
        assert_eq!((board.side_to_move(), board.turn()), (Mark::X, 1));
        let events = board.apply(&Command::Place(Pos::new(1, 1))).unwrap();
        assert_eq!(
            events,
            [Event::Placed {
                mark: Mark::X,
                at: Pos::new(1, 1)
            }]
        );
        assert_eq!((board.side_to_move(), board.turn()), (Mark::O, 2));
        assert_eq!(board.at(Pos::new(1, 1)), Some(Mark::X));
        assert_eq!(board.legal_commands().len(), 8);
    }

    #[test]
    fn a_marked_cell_or_one_off_the_board_is_refused() {
        let mut board = played(&[(0, 0)]);
        let taken = Command::Place(Pos::new(0, 0));
        assert_eq!(board.apply(&taken), Err(Error::Taken(Pos::new(0, 0))));
        let off = Command::Place(Pos::new(3, 0));
        assert_eq!(board.check(&off), Err(Error::Outside(Pos::new(3, 0))));
        assert_eq!(
            Error::Taken(Pos::new(0, 0)).to_string(),
            "(0, 0) is marked already"
        );
        assert_eq!(
            Error::Outside(Pos::new(3, 0)).to_string(),
            "(3, 0) is off the board"
        );
        assert_eq!(index(Pos::new(-1, 0)), None);
    }

    #[test]
    fn three_in_a_line_wins_and_ends_the_game() {
        // X: the middle, the top-left, the bottom-right; O: the top row.
        let board = played(&[(1, 1), (1, 0), (0, 0), (2, 0), (2, 2)]);
        assert_eq!(
            board.outcome(),
            Some(Outcome::Won {
                mark: Mark::X,
                turn: 5
            })
        );
        assert_eq!(board.legal_commands(), []);
        assert_eq!(
            board.check(&Command::Place(Pos::new(0, 2))),
            Err(Error::Over)
        );
        assert_eq!(Error::Over.to_string(), "the game is over");
    }

    #[test]
    fn a_full_board_with_no_line_is_a_draw() {
        let board = played(&[
            (0, 0),
            (1, 1),
            (2, 2),
            (1, 0),
            (1, 2),
            (0, 2),
            (2, 0),
            (2, 1),
            (0, 1),
        ]);
        assert_eq!(board.outcome(), Some(Outcome::Draw { turn: 9 }));
        // The same commands, the same game.
        assert_eq!(
            board,
            played(&[
                (0, 0),
                (1, 1),
                (2, 2),
                (1, 0),
                (1, 2),
                (0, 2),
                (2, 0),
                (2, 1),
                (0, 1)
            ])
        );
    }
}
