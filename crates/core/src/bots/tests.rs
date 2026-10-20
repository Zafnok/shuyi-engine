//! Tests of the play loop over a toy game: tic-tac-toe.

use alloc::format;
use alloc::string::ToString;
use alloc::vec;
use alloc::vec::Vec;

use super::*;

/// A mark, and the side that places it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mark {
    X,
    O,
}

/// How a game of tic-tac-toe ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum End {
    Won(Mark),
    Draw,
}

/// Why a move was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Refused {
    Taken,
    OffTheBoard,
    Over,
}

impl fmt::Display for Refused {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Refused::Taken => "the square is taken",
            Refused::OffTheBoard => "no such square",
            Refused::Over => "the game is over",
        })
    }
}

/// The board, squares 0 to 8 row by row. X moves first; a turn is one
/// move of each.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct TicTacToe {
    squares: [Option<Mark>; 9],
}

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

impl TicTacToe {
    fn moves(&self) -> usize {
        self.squares.iter().flatten().count()
    }
}

impl TurnRules for TicTacToe {
    /// The square to mark.
    type Command = usize;
    /// The square marked, and by whom.
    type Event = (usize, Mark);
    type Error = Refused;
    type Outcome = End;

    fn check(&self, square: &usize) -> Result<(), Refused> {
        if self.outcome().is_some() {
            Err(Refused::Over)
        } else if *square >= 9 {
            Err(Refused::OffTheBoard)
        } else if self.squares[*square].is_some() {
            Err(Refused::Taken)
        } else {
            Ok(())
        }
    }

    fn apply(&mut self, square: &usize) -> Result<Vec<(usize, Mark)>, Refused> {
        self.check(square)?;
        let mark = self.side_to_move();
        self.squares[*square] = Some(mark);
        Ok(vec![(*square, mark)])
    }

    fn legal_commands(&self) -> Vec<usize> {
        if self.outcome().is_some() {
            return vec![];
        }
        (0..9).filter(|&s| self.squares[s].is_none()).collect()
    }

    fn outcome(&self) -> Option<End> {
        let won = LINES.iter().find_map(|line| {
            let mark = self.squares[line[0]]?;
            line.iter()
                .all(|&s| self.squares[s] == Some(mark))
                .then_some(mark)
        });
        match won {
            Some(mark) => Some(End::Won(mark)),
            None if self.moves() == 9 => Some(End::Draw),
            None => None,
        }
    }
}

impl Sides for TicTacToe {
    type Side = Mark;

    fn side_to_move(&self) -> Mark {
        if self.moves().is_multiple_of(2) {
            Mark::X
        } else {
            Mark::O
        }
    }

    fn turn(&self) -> u32 {
        u32::try_from(self.moves() / 2 + 1).unwrap_or(u32::MAX)
    }
}

/// Plays a fresh game with a random bot on each side.
fn random_game(seed: u64) -> (Report<TicTacToe>, TicTacToe) {
    let mut game = TicTacToe::default();
    let mut x = RandomBot::new(seed);
    let mut o = RandomBot::new(seed.wrapping_add(1000));
    let report = play(&mut game, &mut [(Mark::X, &mut x), (Mark::O, &mut o)], 10).unwrap();
    (report, game)
}

/// Plays the squares it is given, in order, and remembers whose move it
/// was asked for.
struct Scripted {
    squares: Vec<usize>,
    asked: Vec<Mark>,
}

impl Scripted {
    fn new(squares: &[usize]) -> Self {
        Self {
            squares: squares.iter().rev().copied().collect(),
            asked: Vec::new(),
        }
    }
}

impl Bot<TicTacToe> for Scripted {
    fn choose(&mut self, game: &TicTacToe) -> usize {
        self.asked.push(game.side_to_move());
        self.squares.pop().expect("the script ran out")
    }
}

#[test]
fn two_random_bots_play_to_the_end() {
    for seed in 0..100 {
        let (report, game) = random_game(seed);
        assert!(report.outcome.is_some(), "seed {seed}: {report:?}");
        assert_eq!(report.outcome, game.outcome());
        assert!((5..=9).contains(&report.commands.len()), "seed {seed}");
        assert_eq!(report.turns, game.turn());
        // The commands rebuild the game.
        let mut again = TicTacToe::default();
        for square in &report.commands {
            again.apply(square).unwrap();
        }
        assert_eq!(again, game);
    }
}

#[test]
fn a_play_is_the_same_for_the_same_seed() {
    for seed in 0..20 {
        assert_eq!(random_game(seed), random_game(seed), "seed {seed}");
    }
    // And the seeds matter: the games are not all the same.
    let first = random_game(0).0.commands;
    assert!((1..20).any(|seed| random_game(seed).0.commands != first));
}

#[test]
fn random_bots_reach_every_ending() {
    let endings: Vec<_> = (0..200).filter_map(|s| random_game(s).0.outcome).collect();
    for end in [End::Won(Mark::X), End::Won(Mark::O), End::Draw] {
        assert!(endings.contains(&end), "{end:?} never happened");
    }
}

#[test]
fn each_side_is_asked_by_its_own_bot() {
    let mut game = TicTacToe::default();
    let mut x = Scripted::new(&[0, 1, 2]);
    let mut o = Scripted::new(&[4, 8]);
    let report = play(&mut game, &mut [(Mark::O, &mut o), (Mark::X, &mut x)], 10).unwrap();
    assert_eq!(report.outcome, Some(End::Won(Mark::X)));
    assert_eq!(report.commands, [0, 4, 1, 8, 2]);
    assert_eq!(x.asked, [Mark::X; 3]);
    assert_eq!(o.asked, [Mark::O; 2]);
}

#[test]
fn the_observer_sees_every_applied_command() {
    let mut game = TicTacToe::default();
    let mut x = Scripted::new(&[0, 1, 2]);
    let mut o = Scripted::new(&[4, 8]);
    let mut seen = Vec::new();
    let report = play_observed(
        &mut game,
        &mut [(Mark::X, &mut x), (Mark::O, &mut o)],
        10,
        |after, square, events| seen.push((after.moves(), *square, events.to_vec())),
    )
    .unwrap();
    assert_eq!(seen.len(), report.commands.len());
    assert_eq!(seen[0], (1, 0, vec![(0, Mark::X)]));
    assert_eq!(seen[1], (2, 4, vec![(4, Mark::O)]));
}

#[test]
fn the_limit_stops_a_play_at_the_turn_past_it() {
    let mut game = TicTacToe::default();
    let mut x = RandomBot::new(1);
    let mut o = RandomBot::new(2);
    let report = play(&mut game, &mut [(Mark::X, &mut x), (Mark::O, &mut o)], 2).unwrap();
    // Turns 1 and 2 are played in full (two marks each: nobody can have
    // won); turn 3 only starts.
    assert_eq!(report.outcome, None);
    assert_eq!(report.commands.len(), 4);
    assert_eq!(report.turns, 3);
}

#[test]
fn a_limit_of_zero_or_a_game_already_over_plays_nothing() {
    let mut x = RandomBot::new(1);
    let mut o = RandomBot::new(2);
    let mut game = TicTacToe::default();
    let report = play(&mut game, &mut [(Mark::X, &mut x), (Mark::O, &mut o)], 0).unwrap();
    assert_eq!((report.outcome, report.turns), (None, 1));
    assert!(report.commands.is_empty());
    assert_eq!(game, TicTacToe::default());

    let (_, mut over) = random_game(3);
    let before = over.clone();
    let report = play(&mut over, &mut [(Mark::X, &mut x), (Mark::O, &mut o)], 10).unwrap();
    assert_eq!(report.outcome, before.outcome());
    assert!(report.commands.is_empty());
    assert_eq!(over, before);
}

#[test]
fn a_refused_command_ends_the_play_with_who_issued_it() {
    let mut game = TicTacToe::default();
    let mut x = Scripted::new(&[0, 1]);
    let mut o = Scripted::new(&[0]);
    let error = play(&mut game, &mut [(Mark::X, &mut x), (Mark::O, &mut o)], 10).unwrap_err();
    assert_eq!(
        (error.command, error.side, error.turn, error.error),
        (0, Mark::O, 1, Refused::Taken)
    );
    assert_eq!(
        error.to_string(),
        "turn 1, O to move: 0 refused: the square is taken"
    );
    // The game keeps what was applied before it.
    assert_eq!(game.moves(), 1);
}

#[test]
#[should_panic(expected = "no bot plays side O")]
fn a_side_without_a_bot_panics() {
    let mut game = TicTacToe::default();
    let mut x = RandomBot::new(1);
    let _ = play(&mut game, &mut [(Mark::X, &mut x)], 10);
}

#[test]
#[should_panic(expected = "no legal command")]
fn a_random_bot_with_nothing_legal_panics() {
    let (_, over) = random_game(3);
    let _ = Bot::<TicTacToe>::choose(&mut RandomBot::new(1), &over);
}

#[test]
fn reports_compare_print_and_clone_by_their_fields() {
    let (report, _) = random_game(4);
    assert_eq!(report.clone(), report);
    let other_outcome = Report {
        outcome: None,
        ..report.clone()
    };
    let other_turns = Report {
        turns: report.turns + 1,
        ..report.clone()
    };
    let other_commands = Report {
        commands: vec![],
        ..report.clone()
    };
    for other in [other_outcome, other_turns, other_commands] {
        assert_ne!(other, report);
    }
    let text = format!("{report:?}");
    assert!(text.starts_with("Report { outcome: Some("), "{text}");
    assert!(text.contains(&format!("turns: {}", report.turns)), "{text}");

    let error: PlayError<TicTacToe> = PlayError {
        command: 9,
        side: Mark::X,
        turn: 2,
        error: Refused::OffTheBoard,
    };
    assert_eq!(
        format!("{error:?}"),
        "PlayError { command: 9, side: X, turn: 2, error: OffTheBoard }"
    );
}
