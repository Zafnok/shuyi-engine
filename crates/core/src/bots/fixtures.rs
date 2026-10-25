//! Toy games for the bots' tests: a dice race (luck in the state), Nim
//! (no luck, nothing hidden), counters (several commands a turn, a
//! combo, and luck) and a skirmish on a grid (two pieces a side, a goal,
//! a wall), with its hooks for the tactics order.

use alloc::vec;
use alloc::vec::Vec;

use super::{Determinize, Sides};
use crate::grid::{Pos, TileSet};
use crate::path::reachable;
use crate::rng::SimRng;
use crate::threat::{GoalDistance, Kind, OrderHooks, Threat, Threats, strike_area};
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

/// The most commands a side issues in one turn of [`Counters`].
pub(crate) const PER_TURN: u32 = 3;

/// What raising both counters in one turn of [`Counters`] earns.
pub(crate) const COMBO: i32 = 10;

/// The points a side of [`Counters`] wins on.
pub(crate) const GOAL: i32 = 30;

/// A [`Counters`] command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Counter {
    /// Raises counter 0 or 1, for a point.
    Raise(usize),
    /// Ends the side's turn.
    End,
}

/// Two sides take turns of up to [`PER_TURN`] commands. Raising a
/// counter costs a point; raising both in one turn earns [`COMBO`], so
/// the first raise only pays off with the second. With slips on, a raise
/// slips one time in four (the state's luck): its counter stays down,
/// and a slip of counter 0 knocks counter 1 over, which can't be raised
/// again that turn. The first side to [`GOAL`] points wins.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Counters {
    pub(crate) points: [i32; 2],
    /// The counters raised this turn.
    pub(crate) raised: [bool; 2],
    /// Whether counter 1 fell this turn.
    pub(crate) fallen: bool,
    /// The commands issued this turn.
    pub(crate) played: u32,
    /// The turns of either side that have ended.
    turns: u32,
    slips: bool,
    rng: SimRng,
}

impl Counters {
    /// Counters from the start, their luck seeded with `seed`, raises
    /// slipping.
    pub(crate) fn new(seed: u64) -> Self {
        Self::at([0, 0], true, seed)
    }

    /// Counters with the sides on `points`, side 0 to move, raises
    /// slipping when `slips` says.
    pub(crate) fn at(points: [i32; 2], slips: bool, seed: u64) -> Self {
        Self {
            points,
            raised: [false; 2],
            fallen: false,
            played: 0,
            turns: 0,
            slips,
            rng: SimRng::new(seed),
        }
    }

    fn end_turn(&mut self) {
        self.turns += 1;
        self.raised = [false; 2];
        self.fallen = false;
        self.played = 0;
    }
}

impl TurnRules for Counters {
    type Command = Counter;
    /// The side's points after the command.
    type Event = i32;
    type Error = ();
    /// The side that won.
    type Outcome = usize;

    fn check(&self, command: &Counter) -> Result<(), ()> {
        let refused = match *command {
            Counter::Raise(counter) => {
                counter > 1 || self.raised[counter] || (counter == 1 && self.fallen)
            }
            Counter::End => false,
        };
        if refused || self.outcome().is_some() {
            Err(())
        } else {
            Ok(())
        }
    }

    fn apply(&mut self, command: &Counter) -> Result<Vec<i32>, ()> {
        self.check(command)?;
        let side = self.side_to_move();
        if let Counter::Raise(counter) = *command {
            self.points[side] -= 1;
            if self.slips && self.rng.roll_below(4) == 0 {
                self.fallen |= counter == 0;
            } else {
                self.raised[counter] = true;
                if self.raised == [true, true] {
                    self.points[side] += COMBO;
                }
            }
            self.played += 1;
        }
        if *command == Counter::End || self.played == PER_TURN {
            self.end_turn();
        }
        Ok(vec![self.points[side]])
    }

    fn legal_commands(&self) -> Vec<Counter> {
        [Counter::Raise(0), Counter::Raise(1), Counter::End]
            .into_iter()
            .filter(|command| self.check(command).is_ok())
            .collect()
    }

    fn outcome(&self) -> Option<usize> {
        self.points.iter().position(|&points| points >= GOAL)
    }
}

impl Sides for Counters {
    type Side = usize;

    fn side_to_move(&self) -> usize {
        usize::try_from(self.turns % 2).unwrap_or(0)
    }

    fn turn(&self) -> u32 {
        self.turns / 2 + 1
    }
}

impl Determinize for Counters {
    /// The luck to come is redrawn; nothing else is hidden.
    fn determinize(&self, _: usize, seed: u64) -> Self {
        Self {
            rng: SimRng::new(seed),
            ..self.clone()
        }
    }
}

/// How far ahead `side` of [`Counters`] is, in points.
pub(crate) fn points_ahead(_: &Counters, end: &Counters, side: usize) -> i32 {
    end.points[side] - end.points[1 - side]
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

/// A [`Skirmish`]'s width.
pub(crate) const WIDTH: u16 = 7;
/// A [`Skirmish`]'s height.
pub(crate) const HEIGHT: u16 = 5;
/// The wall down the middle of a [`Skirmish`], open at the top and the
/// bottom rows.
pub(crate) const WALL: [Pos; 3] = [Pos::new(3, 1), Pos::new(3, 2), Pos::new(3, 3)];
/// The tile side 0's pieces head for.
pub(crate) const GOAL_TILE: Pos = Pos::new(6, 2);
/// A piece's health, full.
pub(crate) const MAX_HP: i32 = 10;
/// How far a piece walks a turn.
pub(crate) const MOVES: u32 = 2;
/// Each way of striking: its range and its damage. Way 0, a sword,
/// strikes at 1 and is struck back; way 1, a bow, strikes at 2.
pub(crate) const WAYS: [(u32, i32); 2] = [(1, 3), (2, 2)];
/// What a heal gives back.
pub(crate) const HEAL: i32 = 3;

/// A [`Skirmish`] piece.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Piece {
    pub(crate) id: u8,
    pub(crate) side: usize,
    pub(crate) at: Pos,
    pub(crate) hp: i32,
    acted: bool,
}

/// A [`Skirmish`] command. Each but `End` is a piece's act for the turn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Skirm {
    /// Braces where it stands.
    Guard(u8),
    /// Shouts where it stands: the tactics order drops it.
    Taunt(u8),
    /// Walks to `dest` and waits.
    Wait { piece: u8, dest: Pos },
    /// Walks to `from` and strikes `target` in way `way`.
    Strike {
        piece: u8,
        target: u8,
        way: u32,
        from: Pos,
    },
    /// Walks to `from` and heals `target`, next to it.
    Heal { piece: u8, target: u8, from: Pos },
    /// Ends the side's turn.
    End,
}

/// Two sides take turns on a [`WIDTH`] × [`HEIGHT`] grid with a [`WALL`]:
/// each piece walks up to [`MOVES`] tiles round the wall and the other
/// pieces, then waits, strikes or heals. A side with no piece standing
/// loses. No luck, nothing hidden.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Skirmish {
    pub(crate) pieces: Vec<Piece>,
    turns: u32,
}

impl Skirmish {
    /// Pieces numbered in order, each `(side, (x, y))`, full of health;
    /// side 0 to move.
    pub(crate) fn new(pieces: &[(usize, (i32, i32))]) -> Self {
        let pieces = pieces
            .iter()
            .zip(0..)
            .map(|(&(side, (x, y)), id)| Piece {
                id,
                side,
                at: Pos::new(x, y),
                hp: MAX_HP,
                acted: false,
            })
            .collect();
        Self { pieces, turns: 0 }
    }

    /// Piece `id`, standing or not.
    pub(crate) fn piece(&self, id: u8) -> Option<&Piece> {
        self.pieces.iter().find(|piece| piece.id == id)
    }

    /// The pieces standing.
    fn standing(&self) -> impl Iterator<Item = &Piece> + '_ {
        self.pieces.iter().filter(|piece| piece.hp > 0)
    }

    /// The tiles `piece` can stop on: round the wall, through no other
    /// piece standing, row by row.
    pub(crate) fn stops(&self, piece: &Piece) -> Vec<Pos> {
        let held = |tile: Pos| {
            self.standing()
                .any(|other| other.id != piece.id && other.at == tile)
        };
        let reach = reachable(WIDTH, HEIGHT, piece.at, MOVES, |tile| {
            (!WALL.contains(&tile) && !held(tile)).then_some(1)
        });
        tiles().filter(|&tile| reach.contains(tile)).collect()
    }

    /// The acts of `piece` from `dest`: waiting, each strike, each heal.
    fn acts_from(&self, piece: &Piece, dest: Pos, out: &mut Vec<Skirm>) {
        out.push(Skirm::Wait {
            piece: piece.id,
            dest,
        });
        for (way, &(range, _)) in (0..).zip(&WAYS) {
            for target in self.standing().filter(|other| other.side != piece.side) {
                if Pos::manhattan(dest, target.at) == range {
                    out.push(Skirm::Strike {
                        piece: piece.id,
                        target: target.id,
                        way,
                        from: dest,
                    });
                }
            }
        }
        for ally in self.standing().filter(|other| other.side == piece.side) {
            if ally.id != piece.id && ally.hp < MAX_HP && Pos::manhattan(dest, ally.at) == 1 {
                out.push(Skirm::Heal {
                    piece: piece.id,
                    target: ally.id,
                    from: dest,
                });
            }
        }
    }

    /// Moves piece `id` to `at`, its act done.
    fn act(&mut self, id: u8, at: Pos) {
        if let Some(piece) = self.pieces.iter_mut().find(|piece| piece.id == id) {
            piece.at = at;
            piece.acted = true;
        }
    }

    /// Takes `damage` off piece `id`.
    fn hurt(&mut self, id: u8, damage: i32) {
        if let Some(piece) = self.pieces.iter_mut().find(|piece| piece.id == id) {
            piece.hp = (piece.hp - damage).clamp(0, MAX_HP);
        }
    }
}

/// Every tile of a skirmish, row by row.
fn tiles() -> impl Iterator<Item = Pos> {
    (0..i32::from(HEIGHT)).flat_map(|y| (0..i32::from(WIDTH)).map(move |x| Pos::new(x, y)))
}

impl TurnRules for Skirmish {
    type Command = Skirm;
    type Event = ();
    type Error = ();
    /// The side that won.
    type Outcome = usize;

    fn check(&self, command: &Skirm) -> Result<(), ()> {
        if self.legal_commands().contains(command) {
            Ok(())
        } else {
            Err(())
        }
    }

    fn apply(&mut self, command: &Skirm) -> Result<Vec<()>, ()> {
        self.check(command)?;
        match *command {
            Skirm::Guard(id) | Skirm::Taunt(id) => {
                let at = self.piece(id).map_or(Pos::new(0, 0), |piece| piece.at);
                self.act(id, at);
            }
            Skirm::Wait { piece, dest } => self.act(piece, dest),
            Skirm::Strike {
                piece,
                target,
                way,
                from,
            } => {
                self.act(piece, from);
                let (_, damage) = WAYS[usize::try_from(way).unwrap_or(0)];
                self.hurt(target, damage);
                if way == 0 && self.piece(target).is_some_and(|target| target.hp > 0) {
                    self.hurt(piece, WAYS[0].1);
                }
            }
            Skirm::Heal {
                piece,
                target,
                from,
            } => {
                self.act(piece, from);
                self.hurt(target, -HEAL);
            }
            Skirm::End => {
                self.turns += 1;
                for piece in &mut self.pieces {
                    piece.acted = false;
                }
            }
        }
        Ok(vec![()])
    }

    fn legal_commands(&self) -> Vec<Skirm> {
        if self.outcome().is_some() {
            return Vec::new();
        }
        let side = self.side_to_move();
        let mut out = Vec::new();
        for piece in self
            .standing()
            .filter(|piece| piece.side == side && !piece.acted)
        {
            out.push(Skirm::Guard(piece.id));
            out.push(Skirm::Taunt(piece.id));
            for dest in self.stops(piece) {
                self.acts_from(piece, dest, &mut out);
            }
        }
        out.push(Skirm::End);
        out
    }

    fn outcome(&self) -> Option<usize> {
        (0..2).find(|&side| self.standing().all(|piece| piece.side == side))
    }
}

impl Sides for Skirmish {
    type Side = usize;

    fn side_to_move(&self) -> usize {
        usize::try_from(self.turns % 2).unwrap_or(0)
    }

    fn turn(&self) -> u32 {
        self.turns / 2 + 1
    }
}

impl Determinize for Skirmish {
    /// No luck and nothing hidden: the state itself.
    fn determinize(&self, _: usize, _: u64) -> Self {
        self.clone()
    }
}

/// A skirmish's hooks for the tactics order: the other side's threats,
/// walking distance to [`GOAL_TILE`] round the wall, and a pick the test
/// sets.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SkirmishHooks {
    pub(crate) threats: Threats<u8>,
    goals: GoalDistance<()>,
    /// What [`OrderHooks::pick`] answers.
    pub(crate) pick: Option<Skirm>,
}

impl SkirmishHooks {
    /// Hooks with no pick.
    pub(crate) fn new() -> Self {
        Self {
            threats: Threats::default(),
            goals: GoalDistance::new(WIDTH, HEIGHT, vec![GOAL_TILE]),
            pick: None,
        }
    }
}

impl OrderHooks<Skirmish> for SkirmishHooks {
    type Piece = u8;

    fn prepare(&mut self, rules: &Skirmish) {
        let side = rules.side_to_move();
        let ranges: Vec<(u32, u32)> = WAYS.iter().map(|&(range, _)| (range, range)).collect();
        self.threats = rules
            .standing()
            .filter(|piece| piece.side != side)
            .map(|piece| {
                let mut stops = TileSet::new(WIDTH, HEIGHT);
                for tile in rules.stops(piece) {
                    stops.insert(tile);
                }
                Threat::new(piece.id, strike_area(&stops, &ranges))
            })
            .collect();
        self.goals = GoalDistance::new(WIDTH, HEIGHT, vec![GOAL_TILE]);
    }

    fn classify(&mut self, rules: &Skirmish, command: &Skirm) -> Kind<u8> {
        match *command {
            Skirm::Guard(_) => Kind::Other,
            Skirm::Taunt(_) => Kind::Drop,
            Skirm::Wait { piece, dest } => Kind::Step { piece, dest },
            Skirm::Strike {
                piece, target, way, ..
            } => {
                let (_, damage) = WAYS[usize::try_from(way).unwrap_or(0)];
                let survives = rules.piece(target).is_some_and(|target| target.hp > damage);
                let back = if way == 0 && survives { WAYS[0].1 } else { 0 };
                Kind::Strike {
                    piece,
                    target: Some(target),
                    way,
                    value: (i64::from(damage) * 100, -i64::from(back) * 100),
                }
            }
            Skirm::Heal {
                piece,
                target,
                from,
            } => {
                let missing = rules.piece(target).map_or(0, |target| MAX_HP - target.hp);
                let stays = rules.piece(piece).is_some_and(|piece| piece.at == from);
                Kind::Extra {
                    group: 0,
                    value: (i64::from(missing), i64::from(stays)),
                    once: Some((piece, Some(target), 0)),
                }
            }
            Skirm::End => Kind::End,
        }
    }

    fn pick(&mut self, _: &Skirmish) -> Option<Skirm> {
        self.pick
    }

    fn at(&self, rules: &Skirmish, piece: u8) -> Option<Pos> {
        rules
            .piece(piece)
            .filter(|piece| piece.hp > 0)
            .map(|piece| piece.at)
    }

    fn distance(&mut self, _: &Skirmish, _: u8, from: Pos) -> Option<u32> {
        self.goals
            .distance((), from, |tile| (!WALL.contains(&tile)).then_some(1))
    }

    fn safe(&self, _: &Skirmish, at: Pos) -> bool {
        self.threats.safe(at)
    }
}
