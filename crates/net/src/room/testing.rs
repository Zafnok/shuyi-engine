//! A tiny [`RoomGame`] for rooms' tests: [`Dots`], behind the `testing`
//! feature.
//!
//! Eight dots on a 16 by 16 grid, each moving one cell a step the way it
//! heads, and stopping at the walls. A press of [`UP`], [`RIGHT`],
//! [`DOWN`] or [`LEFT`] turns a dot; a press of [`OUT`] takes it out of
//! the round. The last dot in wins it, and a round with none left is a
//! draw. The bot turns at random and goes out at random, so a round of
//! bots ends in a few dozen steps.

use shuyi_core::tick::{ActionId, SeatInput, TickInput, TickSim};
use shuyi_core::{ContentHash, Made, SimRng};

use super::{RoomGame, RoundEnd};

/// The grid's side, in cells.
pub const SIDE: u8 = 16;

/// Turns a dot up: a press of action 0.
pub const UP: ActionId = ActionId(0);
/// Turns a dot right.
pub const RIGHT: ActionId = ActionId(1);
/// Turns a dot down.
pub const DOWN: ActionId = ActionId(2);
/// Turns a dot left.
pub const LEFT: ActionId = ActionId(3);
/// Takes a dot out of the round.
pub const OUT: ActionId = ActionId(4);

/// The four turns, by heading.
const TURNS: [ActionId; 4] = [UP, RIGHT, DOWN, LEFT];

/// One bot's chance in this, each step, of turning.
const BOT_TURN: u32 = 6;
/// One bot's chance in this, each step, of going out.
const BOT_OUT: u32 = 40;

/// The game: eight dots, a round of at most 600 steps (40 seconds at 15
/// steps a second), and a pause of 3 steps between rounds.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Dots;

/// One dot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Dot {
    /// Its column, from 0 at the left.
    pub x: u8,
    /// Its row, from 0 at the top.
    pub y: u8,
    /// Where it heads: 0 up, 1 right, 2 down, 3 left.
    pub heading: u8,
    /// Whether it is still in the round.
    pub alive: bool,
}

/// What a step of [`DotsSim`] did.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DotEvent {
    /// The seat's dot went out.
    Out(usize),
}

/// A round of [`Dots`].
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DotsSim {
    /// Each seat's dot.
    pub dots: [Dot; 8],
    /// Steps run.
    pub steps: u32,
}

impl TickSim for DotsSim {
    type Event = DotEvent;

    fn step(&mut self, input: &TickInput) -> Vec<DotEvent> {
        let mut events = Vec::new();
        for (seat, dot) in self.dots.iter_mut().enumerate() {
            let Some(seat_input) = input.seat(seat).filter(|_| dot.alive) else {
                continue;
            };
            if seat_input.is_pressed(OUT) {
                dot.alive = false;
                events.push(DotEvent::Out(seat));
                continue;
            }
            let turn = seat_input
                .pressed
                .iter()
                .rev()
                .find_map(|action| TURNS.iter().position(|t| t == action));
            if let Some(heading) = turn.and_then(|h| u8::try_from(h).ok()) {
                dot.heading = heading;
            }
            let last = SIDE - 1;
            match dot.heading {
                0 => dot.y = dot.y.saturating_sub(1),
                1 => dot.x = (dot.x + 1).min(last),
                2 => dot.y = (dot.y + 1).min(last),
                _ => dot.x = dot.x.saturating_sub(1),
            }
        }
        self.steps += 1;
        events
    }

    fn hash(&self) -> u64 {
        // FNV-1a over the state's bytes.
        let mut bytes = self.steps.to_le_bytes().to_vec();
        for dot in &self.dots {
            bytes.extend([dot.x, dot.y, dot.heading, u8::from(dot.alive)]);
        }
        bytes.iter().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
            (hash ^ u64::from(*byte)).wrapping_mul(0x0100_0000_01b3)
        })
    }
}

impl RoomGame for Dots {
    type Sim = DotsSim;

    const SEATS: usize = 8;
    const STEPS_PER_SECOND: u32 = 15;
    const MAX_ROUND_STEPS: u32 = 600;
    const PAUSE_STEPS: u32 = 3;

    fn made(&self) -> Made {
        Made::new("dots", ContentHash(0))
    }

    /// Each dot at a cell and a heading the seed picks.
    fn new_round(&self, seed: u64, round: u32) -> DotsSim {
        let mut rng = SimRng::new(seed ^ u64::from(round));
        let mut cell = || u8::try_from(rng.roll_below(u32::from(SIDE))).unwrap_or(0);
        let dots = core::array::from_fn(|_| Dot {
            x: cell(),
            y: cell(),
            heading: cell() % 4,
            alive: true,
        });
        DotsSim { dots, steps: 0 }
    }

    fn round_winner(sim: &DotsSim) -> RoundEnd {
        let mut alive = (0..Self::SEATS).filter(|&seat| Self::seat_alive(sim, seat));
        match (alive.next(), alive.next()) {
            (None, _) => RoundEnd::Draw,
            (Some(seat), None) => RoundEnd::Won(seat),
            (Some(_), Some(_)) => RoundEnd::Going,
        }
    }

    fn seat_alive(sim: &DotsSim, seat: usize) -> bool {
        sim.dots.get(seat).is_some_and(|dot| dot.alive)
    }

    fn bot_input(_sim: &DotsSim, _seat: usize, rng: &mut SimRng) -> SeatInput {
        let mut pressed = Vec::new();
        if rng.roll_below(BOT_TURN) == 0 {
            let turn = usize::try_from(rng.roll_below(4)).unwrap_or(0);
            pressed.push(TURNS[turn]);
        }
        if rng.roll_below(BOT_OUT) == 0 {
            pressed.push(OUT);
        }
        SeatInput::new(pressed, Vec::new(), Vec::new())
    }
}
