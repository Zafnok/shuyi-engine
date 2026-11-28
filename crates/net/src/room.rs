//! Rooms: a server-authoritative room over a [`TickSim`], with bots in
//! empty seats and players dropping in and out (ADR-0155).
//!
//! A game gives the room its rules through [`RoomGame`]. The server's
//! side is [`Room`]: it seats peers, latches their inputs, asks the game's
//! bot for each empty seat's, steps the real sim, and writes each step's
//! resolved inputs to every peer, with the sim's hash every
//! [`HASH_EVERY`] steps. A client steps its own copy of the same
//! deterministic sim with exactly those inputs, so the copies stay equal,
//! and a joiner catches up from the round's first step. Rounds follow one
//! another without end, a short pause between them. A client's side is
//! [`RoomClient`]: it follows the frames behind a jitter buffer, checks
//! the hash, and asks for a catch-up when its sim differs.
//!
//! The room is pure: no I/O, no clock, no floats. A host (a server's
//! socket loop, or a test over [`MemoryNetwork`](crate::MemoryNetwork))
//! calls [`Room::step`] `STEPS_PER_SECOND` times a second, and sends what
//! it wrote, encoded by [`wire`], to each peer on the
//! [`Reliable`](crate::Channel::Reliable) channel:
//!
//! ```
//! use shuyi_net::room::{Joined, Outbox, Room, RoomGame};
//! use shuyi_net::room::testing::Dots;
//! use shuyi_net::room::wire::Frame;
//! use shuyi_net::PeerId;
//!
//! let mut room = Room::new(Dots, 7);
//! // The lowest bot seat whose dot is alive.
//! assert_eq!(room.join(PeerId(1)), Ok(Joined::Seat(0)));
//! let mut out = Outbox::new();
//! room.step(&mut out);
//! // The catch-up first, then the step's inputs.
//! assert!(matches!(out.frames[0], (PeerId(1), Frame::Joined { .. })));
//! assert!(matches!(out.frames[1], (PeerId(1), Frame::Step { step: 1, .. })));
//! let bytes = out.frames[1].1.encode();
//! assert_eq!(Frame::decode::<Dots>(&bytes).as_ref(), Ok(&out.frames[1].1));
//! assert_eq!(room.stats().bots, Dots::SEATS - 1);
//! ```

use core::fmt;

use shuyi_core::tick::{MAX_SEATS, SeatInput, TickSim};
use shuyi_core::{Made, SimRng};

mod client;
mod host;
#[cfg(any(test, feature = "testing"))]
pub mod testing;
pub mod wire;

pub use client::{Advance, RoomClient};
pub use host::{Outbox, Room, RoomStats};

/// Every this many steps of a round, a step's frame carries the sim's
/// hash, which each client checks against its own.
pub const HASH_EVERY: u32 = 15;

/// What a game gives a room: its sim, its seats and timings, and its bot.
///
/// Every function is pure: the room calls them on the server, and a client
/// calls [`new_round`](Self::new_round) to build its own copy of the sim.
/// The bot runs only on the server, whose rng feeds it; clients get its
/// inputs with the humans'.
pub trait RoomGame {
    /// The rules, stepped once a step with every seat's input.
    type Sim: TickSim;

    /// How many seats a room has, humans' and bots': at least 1, at most
    /// [`MAX_SEATS`]. A room of another number doesn't build.
    const SEATS: usize;

    /// How many times a second the host steps the room.
    const STEPS_PER_SECOND: u32;

    /// The longest round, in steps: a round that reaches it with no winner
    /// ends as [`RoundEnd::Draw`]. It bounds a joiner's catch-up.
    const MAX_ROUND_STEPS: u32;

    /// The steps between a round's end and the next round's start.
    const PAUSE_STEPS: u32;

    /// The build's stamp, which a handshake compares.
    fn made(&self) -> Made;

    /// A round's first state, from the room's `seed` for it, round
    /// `round` (the first is 1).
    fn new_round(&self, seed: u64, round: u32) -> Self::Sim;

    /// Whether the round goes on, and who won it if not.
    fn round_winner(sim: &Self::Sim) -> RoundEnd;

    /// Whether seat `seat` still plays this round: a joiner takes only a
    /// bot seat that does.
    fn seat_alive(sim: &Self::Sim, seat: usize) -> bool;

    /// What the game's bot does in seat `seat` this step, drawing on the
    /// room's `rng`.
    fn bot_input(sim: &Self::Sim, seat: usize, rng: &mut SimRng) -> SeatInput;
}

/// Checks a game's numbers where a room or a client is built: a room of
/// no seat, of more than [`MAX_SEATS`], or that never steps doesn't
/// compile.
pub(crate) struct Checked<G>(core::marker::PhantomData<G>);

impl<G: RoomGame> Checked<G> {
    /// The check, evaluated when it is named.
    pub(crate) const FITS: () = assert!(
        G::SEATS >= 1 && G::SEATS <= MAX_SEATS && G::STEPS_PER_SECOND >= 1,
        "a room has 1 to MAX_SEATS seats and steps at least once a second"
    );
}

/// How a round stands after a step.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum RoundEnd {
    /// It goes on.
    Going,
    /// The seat that won it.
    Won(usize),
    /// It ended with no winner.
    Draw,
}

/// Where a peer is in a room.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Joined {
    /// In this seat, controlled from the next step.
    Seat(usize),
    /// Watching, until the next round seats it.
    Watching,
}

impl Joined {
    /// The seat, if it has one.
    pub fn seat(self) -> Option<usize> {
        match self {
            Self::Seat(seat) => Some(seat),
            Self::Watching => None,
        }
    }
}

/// Who rides a seat, on the server.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Seat {
    /// The peer's inputs.
    Human(crate::PeerId),
    /// The game's bot.
    Bot,
}

/// Who rides a seat, as a client is told: no peer's id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum SeatKind {
    /// A player.
    Human,
    /// The game's bot.
    Bot,
}

impl From<Seat> for SeatKind {
    fn from(seat: Seat) -> Self {
        match seat {
            Seat::Human(_) => Self::Human,
            Seat::Bot => Self::Bot,
        }
    }
}

/// A join refused: humans and watchers already fill every seat.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Full;

impl fmt::Display for Full {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("the room is full: players hold every seat")
    }
}

impl core::error::Error for Full {}
