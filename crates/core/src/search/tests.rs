//! Tests of the level solver on two small fixture games: a counter that
//! `+2` and `×3` commands move toward a target, and a maze.

use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;

use proptest::prelude::*;
use serde::ser::{Error as _, SerializeTuple, Serializer};

use super::*;
use crate::grid::{Dir, Grid, Pos};
use crate::path::reachable;

/// A counter's command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Op {
    /// Adds 2.
    Add2,
    /// Multiplies by 3.
    Times3,
}

/// How a counter ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum End {
    /// On the target.
    Hit,
    /// Past it.
    Over,
}

/// A counter that `+2` and `×3` move toward `target`; past it, the game
/// is lost.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Counter {
    value: u32,
    target: u32,
    /// `×3` listed before `+2`.
    times_first: bool,
    /// A value at which `apply` refuses what `legal_commands` lists.
    refuse_at: Option<u32>,
    /// A value whose `Serialize` fails.
    poison: Option<u32>,
}

impl Counter {
    /// A counter at `value`, toward `target`, `+2` listed first.
    fn new(value: u32, target: u32) -> Self {
        Self {
            value,
            target,
            times_first: false,
            refuse_at: None,
            poison: None,
        }
    }
}

impl Serialize for Counter {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        if self.poison == Some(self.value) {
            return Err(S::Error::custom("poisoned"));
        }
        let mut t = s.serialize_tuple(2)?;
        t.serialize_element(&self.value)?;
        t.serialize_element(&self.target)?;
        t.end()
    }
}

impl TurnRules for Counter {
    type Command = Op;
    type Event = u32;
    type Error = String;
    type Outcome = End;

    fn check(&self, _: &Op) -> Result<(), String> {
        if self.outcome().is_some() {
            Err("over".to_string())
        } else if self.refuse_at == Some(self.value) {
            Err("refused".to_string())
        } else {
            Ok(())
        }
    }

    fn apply(&mut self, op: &Op) -> Result<Vec<u32>, String> {
        self.check(op)?;
        self.value = match op {
            Op::Add2 => self.value + 2,
            Op::Times3 => self.value * 3,
        };
        Ok(vec![self.value])
    }

    /// Both, unless it is over: the bug `refuse_at` makes lists them still.
    fn legal_commands(&self) -> Vec<Op> {
        match (self.outcome(), self.times_first) {
            (Some(_), _) => Vec::new(),
            (None, false) => vec![Op::Add2, Op::Times3],
            (None, true) => vec![Op::Times3, Op::Add2],
        }
    }

    fn outcome(&self) -> Option<End> {
        match self.value.cmp(&self.target) {
            core::cmp::Ordering::Less => None,
            core::cmp::Ordering::Equal => Some(End::Hit),
            core::cmp::Ordering::Greater => Some(End::Over),
        }
    }
}

/// The counter's goal: on the target.
const HIT: fn(&End) -> bool = |end| *end == End::Hit;

/// A walker in a maze of walls, won on the goal tile.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct Maze {
    walls: Grid<bool>,
    at: Pos,
    goal: Pos,
}

impl Maze {
    /// Whether `pos` can be walked.
    fn open(&self, pos: Pos) -> bool {
        self.walls.get(pos) == Some(&false)
    }
}

impl TurnRules for Maze {
    type Command = Dir;
    type Event = Pos;
    type Error = ();
    type Outcome = ();

    fn check(&self, dir: &Dir) -> Result<(), ()> {
        if self.outcome().is_none() && self.open(self.at.step(*dir)) {
            Ok(())
        } else {
            Err(())
        }
    }

    fn apply(&mut self, dir: &Dir) -> Result<Vec<Pos>, ()> {
        self.check(dir)?;
        self.at = self.at.step(*dir);
        Ok(vec![self.at])
    }

    fn legal_commands(&self) -> Vec<Dir> {
        Dir::ALL
            .into_iter()
            .filter(|d| self.check(d).is_ok())
            .collect()
    }

    fn outcome(&self) -> Option<()> {
        (self.at == self.goal).then_some(())
    }
}

/// Any outcome.
const ANY: fn(&()) -> bool = |()| true;

/// A maze from rows: `#` a wall, `@` the start, `>` the goal.
fn maze(rows: &[&str]) -> Maze {
    let width = u16::try_from(rows[0].len()).unwrap();
    let height = u16::try_from(rows.len()).unwrap();
    let mut walls = Grid::filled(width, height, false);
    let (mut at, mut goal) = (Pos::new(0, 0), Pos::new(0, 0));
    for (y, row) in rows.iter().enumerate() {
        for (x, c) in row.chars().enumerate() {
            let pos = Pos::new(i32::try_from(x).unwrap(), i32::try_from(y).unwrap());
            match c {
                '#' => *walls.get_mut(pos).unwrap() = true,
                '@' => at = pos,
                '>' => goal = pos,
                _ => {}
            }
        }
    }
    Maze { walls, at, goal }
}

/// The answer of a solver of `start`, every state given one hash.
fn one_hash<R, G>(start: &R, goal: G, limits: Limits) -> Answer<R::Command>
where
    R: TurnRules + Clone + Eq + Serialize,
    R::Command: Clone,
    G: Fn(&R::Outcome) -> bool,
{
    let mut solver = Solver::new(start.clone(), goal, limits);
    solver.hash = |_| Ok(StateHash(0));
    loop {
        if let Some(answer) = solver.step(u32::MAX)? {
            return Ok(answer);
        }
    }
}

/// `solve`'s answer, checked equal to one with every state given one hash.
fn both<R, G>(start: &R, goal: G, limits: Limits) -> Answer<R::Command>
where
    R: TurnRules + Clone + Eq + Serialize,
    R::Command: Clone + fmt::Debug + PartialEq,
    G: Fn(&R::Outcome) -> bool + Copy,
{
    let answer = solve(start, goal, limits);
    assert_eq!(
        one_hash(start, goal, limits),
        answer,
        "one hash for every state"
    );
    answer
}

#[test]
fn a_shortest_line_is_found() {
    // 1 + 2 = 3, × 3 = 9.
    assert_eq!(
        both(&Counter::new(1, 9), HIT, Limits::DEFAULT),
        Ok(Solve::Solved {
            commands: vec![Op::Add2, Op::Times3],
            states: 4
        })
    );
    let m = maze(&["@.#", ".##", "..>"]);
    let Ok(Solve::Solved { commands, .. }) = both(&m, ANY, Limits::default()) else {
        panic!("not solved");
    };
    assert_eq!(commands, [Dir::Down, Dir::Down, Dir::Right, Dir::Right]);
}

#[test]
fn among_shortest_lines_the_first_listed_wins() {
    // 5 is 1 + 2 + 2 or 1 × 3 + 2; both make 3 first, so the first listed
    // command that makes 3 is in the line.
    let plus = Counter::new(1, 5);
    let Ok(Solve::Solved { commands, .. }) = both(&plus, HIT, Limits::DEFAULT) else {
        panic!("not solved");
    };
    assert_eq!(commands, [Op::Add2, Op::Add2]);
    let times = Counter {
        times_first: true,
        ..plus
    };
    let Ok(Solve::Solved { commands, .. }) = both(&times, HIT, Limits::DEFAULT) else {
        panic!("not solved");
    };
    assert_eq!(commands, [Op::Times3, Op::Add2]);
    // Two ways round a pillar: Right is listed before Down.
    let m = maze(&["@.", ".>"]);
    assert_eq!(
        both(&m, ANY, Limits::DEFAULT),
        Ok(Solve::Solved {
            commands: vec![Dir::Right, Dir::Down],
            states: 4
        })
    );
}

#[test]
fn a_start_already_won_needs_no_commands() {
    assert_eq!(
        both(&Counter::new(4, 4), HIT, Limits::DEFAULT.with_depth(0)),
        Ok(Solve::Solved {
            commands: vec![],
            states: 1
        })
    );
}

#[test]
fn a_goal_out_of_reach_is_exhausted_with_every_state_counted() {
    // 1, 3 (by + 2; × 3 makes it again), then 5 and 9, both over.
    assert_eq!(
        both(&Counter::new(1, 4), HIT, Limits::DEFAULT),
        Ok(Solve::Exhausted { states: 4 })
    );
    // A finished start the goal refuses is a dead end.
    assert_eq!(
        both(&Counter::new(5, 4), HIT, Limits::DEFAULT),
        Ok(Solve::Exhausted { states: 1 })
    );
    // Walled off: the walker's three tiles.
    let m = maze(&["@.#", ".##", "##>"]);
    assert_eq!(
        both(&m, ANY, Limits::DEFAULT),
        Ok(Solve::Exhausted { states: 3 })
    );
}

#[test]
fn a_goal_past_the_depth_limit_is_never_exhausted() {
    let c = Counter::new(1, 9);
    assert_eq!(
        both(&c, HIT, Limits::DEFAULT.with_depth(1)),
        Ok(Solve::LimitReached { states: 2 })
    );
    assert_eq!(
        both(&c, HIT, Limits::DEFAULT.with_depth(0)),
        Ok(Solve::LimitReached { states: 1 })
    );
    assert!(matches!(
        both(&c, HIT, Limits::DEFAULT.with_depth(2)),
        Ok(Solve::Solved { .. })
    ));
    // Every line ends before the limit: still exhausted.
    assert_eq!(
        both(&Counter::new(1, 4), HIT, Limits::DEFAULT.with_depth(2)),
        Ok(Solve::Exhausted { states: 4 })
    );
}

#[test]
fn the_state_limit_stops_the_search() {
    let c = Counter::new(1, 9);
    assert_eq!(
        both(&c, HIT, Limits::DEFAULT.with_states(2)),
        Ok(Solve::LimitReached { states: 2 })
    );
    // 0 counts as 1: the start is kept.
    assert_eq!(
        both(&c, HIT, Limits::DEFAULT.with_states(0)),
        Ok(Solve::LimitReached { states: 1 })
    );
    assert_eq!(
        both(&Counter::new(9, 9), HIT, Limits::DEFAULT.with_states(0)),
        Ok(Solve::Solved {
            commands: vec![],
            states: 1
        })
    );
    assert!(matches!(
        both(&c, HIT, Limits::DEFAULT.with_states(4)),
        Ok(Solve::Solved { states: 4, .. })
    ));
}

#[test]
fn a_listed_command_the_game_refuses_is_an_error_with_its_line() {
    let c = Counter {
        refuse_at: Some(3),
        ..Counter::new(1, 20)
    };
    let error = both(&c, HIT, Limits::DEFAULT).unwrap_err();
    assert_eq!(
        error,
        SolveError::Refused {
            line: vec![Op::Add2, Op::Add2]
        }
    );
    // Replaying the line shows the game's error.
    let mut replay = c.clone();
    replay.apply(&Op::Add2).unwrap();
    assert_eq!(replay.apply(&Op::Add2), Err("refused".to_string()));
    assert!(error.to_string().contains("[Add2, Add2]"), "{error}");
}

#[test]
fn a_state_that_fails_to_serialize_is_a_hash_error() {
    let c = Counter {
        poison: Some(9),
        ..Counter::new(1, 20)
    };
    let error = solve(&c, HIT, Limits::DEFAULT).unwrap_err();
    assert_eq!(error, SolveError::Hash(HashError("poisoned".to_string())));
    assert_eq!(error.to_string(), "the state could not be hashed: poisoned");
    let start = Counter {
        poison: Some(1),
        ..c
    };
    assert!(matches!(
        solve(&start, HIT, Limits::DEFAULT),
        Err(SolveError::Hash(_))
    ));
}

#[test]
fn step_zero_does_nothing_and_an_answer_is_given_again() {
    let mut solver = Solver::new(Counter::new(1, 9), HIT, Limits::DEFAULT);
    assert_eq!(solver.step(0), Ok(None));
    assert_eq!(solver.states(), 0);
    // The start is kept, and one state expanded.
    assert_eq!(solver.step(1), Ok(None));
    assert_eq!(solver.states(), 2);
    let answer = solver.step(1);
    assert_eq!(
        answer,
        Ok(Some(Solve::Solved {
            commands: vec![Op::Add2, Op::Times3],
            states: 4
        }))
    );
    assert_eq!(solver.step(u32::MAX), answer);
    assert_eq!(solver.step(0), answer);
    assert_eq!(solver.states(), 4);
    assert!(format!("{solver:?}").contains("done: true"));
    // An error is given again too.
    let c = Counter {
        refuse_at: Some(1),
        ..Counter::new(1, 9)
    };
    let mut solver = Solver::new(c, HIT, Limits::DEFAULT);
    let error = solver.step(5);
    assert!(matches!(error, Err(SolveError::Refused { .. })));
    assert_eq!(solver.step(5), error);
}

#[test]
fn every_answer_counts_its_states() {
    let answers: [Solve<Op>; 3] = [
        Solve::Solved {
            commands: vec![],
            states: 1,
        },
        Solve::Exhausted { states: 2 },
        Solve::LimitReached { states: 3 },
    ];
    let states: Vec<u32> = answers.iter().map(Solve::states).collect();
    assert_eq!(states, [1, 2, 3]);
}

/// A random maze up to 4 × 3: its walls, start and goal.
fn random_maze() -> impl Strategy<Value = Maze> {
    (2u16..=4, 2u16..=3)
        .prop_flat_map(|(w, h)| {
            let cells = usize::from(w * h);
            (
                Just((w, h)),
                proptest::collection::vec(proptest::bool::weighted(0.3), cells),
                0..cells,
                0..cells,
            )
        })
        .prop_map(|((w, h), walls, start, goal)| {
            let pos = |i: usize| {
                let w = usize::from(w);
                Pos::new(i32::try_from(i % w).unwrap(), i32::try_from(i / w).unwrap())
            };
            let mut grid = Grid::filled(w, h, false);
            for (i, wall) in walls.into_iter().enumerate() {
                *grid.get_mut(pos(i)).unwrap() = wall;
            }
            *grid.get_mut(pos(start)).unwrap() = false;
            *grid.get_mut(pos(goal)).unwrap() = false;
            Maze {
                walls: grid,
                at: pos(start),
                goal: pos(goal),
            }
        })
}

/// Whether any line of exactly `depth` commands, or shorter, from `m`
/// reaches the goal: every line tried.
fn brute_force(m: &Maze, depth: u32) -> bool {
    if m.outcome().is_some() {
        return true;
    }
    depth > 0
        && m.legal_commands().into_iter().any(|dir| {
            let mut next = m.clone();
            next.apply(&dir).unwrap();
            brute_force(&next, depth - 1)
        })
}

/// The answer of a solver of `m` stepped by `slices` in turn.
fn sliced(m: &Maze, slices: &[u32]) -> Answer<Dir> {
    let mut solver = Solver::new(m.clone(), ANY, Limits::DEFAULT);
    for &units in slices.iter().cycle() {
        if let Some(answer) = solver.step(units)? {
            return Ok(answer);
        }
    }
    unreachable!("the slices are never all 0")
}

proptest! {
    #[test]
    fn a_maze_is_solved_by_a_shortest_line(m in random_maze()) {
        let answer = both(&m, ANY, Limits::DEFAULT);
        prop_assert_eq!(solve(&m, ANY, Limits::DEFAULT), answer.clone());
        let walk = reachable(m.walls.width(), m.walls.height(), m.at, u32::MAX, |p| {
            m.open(p).then_some(1)
        });
        match answer {
            Ok(Solve::Solved { commands, .. }) => {
                let mut end = m.clone();
                for dir in &commands {
                    end.apply(dir).unwrap();
                }
                prop_assert_eq!(end.outcome(), Some(()));
                let length = u32::try_from(commands.len()).unwrap();
                prop_assert_eq!(walk.cost(m.goal), Some(length));
                if length > 0 {
                    prop_assert!(!brute_force(&m, length - 1));
                }
            }
            Ok(Solve::Exhausted { states }) => {
                prop_assert!(!walk.contains(m.goal));
                prop_assert_eq!(usize::try_from(states).unwrap(), walk.tiles().len());
            }
            other => prop_assert!(false, "{:?}", other),
        }
    }

    #[test]
    fn slicing_never_changes_the_answer(
        m in random_maze(),
        slices in proptest::collection::vec(1u32..4, 1..6),
    ) {
        let whole = solve(&m, ANY, Limits::DEFAULT);
        prop_assert_eq!(sliced(&m, &[1]), whole.clone());
        prop_assert_eq!(sliced(&m, &slices), whole.clone());
        prop_assert_eq!(sliced(&m, &[u32::MAX]), whole);
    }
}
