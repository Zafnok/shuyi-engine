//! A client's side of a room: [`RoomClient`] follows the frames a
//! [`Room`](super::Room) sends, behind a jitter buffer, checks its hash
//! and asks for a catch-up when its sim differs ([`Advance`]).

use std::collections::VecDeque;

use shuyi_core::tick::{ActionId, SeatInput, TickSim};

use super::wire::{ClientMsg, Frame, Roster, RoundStart, WireError};
use super::{Checked, Joined, RoomGame};

/// The steps that wait before the buffer starts applying them, at the
/// start and after it ran dry.
const BUFFER: usize = 2;

/// The event of a step of game `G`'s sim.
type Event<G> = <<G as RoomGame>::Sim as TickSim>::Event;

/// What one [`RoomClient::advance`] did.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Advance {
    /// It applied this many steps: 1, or 2 when it was catching up.
    Stepped(u32),
    /// No step applied: the buffer fills, or the client waits for its
    /// catch-up.
    Waiting,
    /// The client's sim differs from the room's: these are the bytes of a
    /// [`ClientMsg::Resync`], for the game to send to the room on the
    /// [`Reliable`](crate::Channel::Reliable) channel. It applies nothing
    /// until the catch-up arrives.
    Resync(Vec<u8>),
    /// The server shuts down.
    Closed,
}

/// A client's copy of a room: it builds the round's sim and steps it with
/// the inputs each [`Frame::Step`] carries, so it stays equal to the
/// room's. It does no I/O: the game hands it each frame's bytes
/// ([`on_frame`](Self::on_frame)), calls [`advance`](Self::advance) once a
/// step of its own tick driver, and sends what [`input`](Self::input),
/// [`alive`](Self::alive) and [`leave`](Self::leave) give.
///
/// - **The catch-up** ([`Frame::Joined`]) rebuilds the sim at once from
///   the round's start and every step since; until the first, frames are
///   dropped.
/// - **The jitter buffer:** steps wait in arrival order. It starts
///   applying once 2 wait, and again after it ran dry; it applies one a
///   step, or two while more than 2 wait.
/// - **The check:** a step whose hash differs from the sim's after it, or
///   whose number isn't the next, stops it: that `advance` returns
///   [`Advance::Resync`], and the next ones [`Advance::Waiting`] until the
///   catch-up arrives.
///
/// ```
/// use shuyi_net::PeerId;
/// use shuyi_net::room::testing::Dots;
/// use shuyi_net::room::{Advance, Outbox, Room, RoomClient};
///
/// let mut room = Room::new(Dots, 7);
/// room.join(PeerId(1)).expect("a seat is free");
/// let mut client = RoomClient::new(Dots);
/// let mut out = Outbox::new();
/// for _ in 0..2 {
///     out.clear();
///     room.step(&mut out);
///     for (_, frame) in &out.frames {
///         client.on_frame(&frame.encode()).expect("the room's frames decode");
///     }
/// }
/// assert_eq!(client.my_seat(), Some(0));
/// assert_eq!(client.steps_behind(), 2);
/// assert_eq!(client.advance(), Advance::Stepped(1));
/// assert_eq!(client.steps_behind(), 1);
/// ```
pub struct RoomClient<G: RoomGame> {
    game: G,
    /// The round's sim, from the first catch-up on.
    sim: Option<G::Sim>,
    place: Option<Joined>,
    roster: Roster,
    wins: Vec<u32>,
    round: u32,
    /// The last step applied in this round.
    step: u32,
    /// The frames that wait, in arrival order.
    queue: VecDeque<Frame>,
    /// How many of them are steps.
    steps: usize,
    /// Whether the buffer filled and applies.
    flowing: bool,
    /// Stopped after a mismatch, until the catch-up.
    stopped: bool,
    closed: bool,
    events: Vec<Event<G>>,
    /// The held set last sent.
    held: Vec<ActionId>,
    /// The advances since anything was sent.
    quiet: u32,
}

impl<G: RoomGame> RoomClient<G> {
    /// A client of a room of `game`, with no sim until its catch-up.
    pub fn new(game: G) -> Self {
        let () = Checked::<G>::FITS;
        Self {
            game,
            sim: None,
            place: None,
            roster: Roster::default(),
            wins: Vec::new(),
            round: 0,
            step: 0,
            queue: VecDeque::new(),
            steps: 0,
            flowing: false,
            stopped: false,
            closed: false,
            events: Vec::new(),
            held: Vec::new(),
            quiet: 0,
        }
    }

    /// Takes a frame's bytes, as the room sent them. A catch-up rebuilds
    /// the sim at once; a step, a roster and a round's end or start wait
    /// for [`advance`](Self::advance); a `Closing` closes the client.
    /// Frames before the first catch-up, and while it waits for one after
    /// a mismatch, are dropped.
    pub fn on_frame(&mut self, bytes: &[u8]) -> Result<(), WireError> {
        let frame = Frame::decode::<G>(bytes)?;
        match frame {
            Frame::Joined {
                place,
                roster,
                round_start,
                steps,
            } => {
                let mut sim = self.game.new_round(round_start.seed, round_start.round);
                for inputs in &steps {
                    // Played before: their events are dropped.
                    sim.step(inputs);
                }
                self.sim = Some(sim);
                self.place = Some(place);
                self.roster = roster;
                self.round = round_start.round;
                self.wins = round_start.wins;
                self.step = u32::try_from(steps.len()).unwrap_or(u32::MAX);
                self.empty();
                self.stopped = false;
            }
            Frame::Closing => self.closed = true,
            _ if self.sim.is_none() || self.stopped => {}
            frame => {
                if matches!(frame, Frame::Step { .. }) {
                    self.steps += 1;
                }
                self.queue.push_back(frame);
            }
        }
        Ok(())
    }

    /// Runs one step of the game's tick: applies the frames that wait
    /// before the next step, then that step, or two while more than 2
    /// wait, then the frames after them up to the next step.
    pub fn advance(&mut self) -> Advance {
        self.events.clear();
        self.quiet = self.quiet.saturating_add(1);
        if self.closed {
            return Advance::Closed;
        }
        if self.stopped {
            return Advance::Waiting;
        }
        self.apply_news();
        if !self.flowing && self.steps >= BUFFER {
            self.flowing = true;
        }
        if !self.flowing || self.steps == 0 {
            self.flowing = false;
            return Advance::Waiting;
        }
        let take = if self.steps > BUFFER { 2 } else { 1 };
        for _ in 0..take {
            if !self.apply_step() {
                self.stopped = true;
                self.empty();
                self.quiet = 0;
                return Advance::Resync(ClientMsg::Resync.encode());
            }
            self.apply_news();
        }
        Advance::Stepped(take)
    }

    /// The round's sim, as the last step applied left it; none before the
    /// first catch-up.
    pub fn sim(&self) -> Option<&G::Sim> {
        self.sim.as_ref()
    }

    /// Where this client is in the room; none before the first catch-up.
    pub fn place(&self) -> Option<Joined> {
        self.place
    }

    /// The seat this client rides, if it has one.
    pub fn my_seat(&self) -> Option<usize> {
        self.place.and_then(Joined::seat)
    }

    /// Who rides each seat.
    pub fn roster(&self) -> &Roster {
        &self.roster
    }

    /// Each seat's rounds won.
    pub fn wins(&self) -> &[u32] {
        &self.wins
    }

    /// The round, the first 1; 0 before the first catch-up.
    pub fn round(&self) -> u32 {
        self.round
    }

    /// The steps that wait in the buffer.
    pub fn steps_behind(&self) -> usize {
        self.steps
    }

    /// What the steps the last [`advance`](Self::advance) applied made, in
    /// order: a screen plays each once. A catch-up's steps make none.
    pub fn events(&self) -> &[Event<G>] {
        &self.events
    }

    /// The bytes of an `Input` for `seat`, the player's actions this step,
    /// when it presses or releases an action or holds another set than the
    /// last one sent; else none. The same press on two steps running is
    /// sent twice.
    pub fn input(&mut self, seat: &SeatInput) -> Option<Vec<u8>> {
        let edge = !(seat.pressed.is_empty() && seat.released.is_empty());
        if !edge && seat.held == self.held {
            return None;
        }
        self.held.clone_from(&seat.held);
        self.quiet = 0;
        Some(ClientMsg::Input(seat.clone()).encode())
    }

    /// The bytes of an `Alive` when nothing was sent for
    /// [`STEPS_PER_SECOND`](RoomGame::STEPS_PER_SECOND) advances, so the
    /// room keeps a quiet player; else none.
    pub fn alive(&mut self) -> Option<Vec<u8>> {
        if self.quiet < G::STEPS_PER_SECOND {
            return None;
        }
        self.quiet = 0;
        Some(ClientMsg::Alive.encode())
    }

    /// The bytes of a `Leave`: the player leaves the room.
    pub fn leave(&mut self) -> Vec<u8> {
        self.quiet = 0;
        ClientMsg::Leave.encode()
    }

    /// Forgets the frames that wait.
    fn empty(&mut self) {
        self.queue.clear();
        self.steps = 0;
        self.flowing = false;
    }

    /// Applies the frames at the head of the queue up to the next step.
    fn apply_news(&mut self) {
        while let Some(frame) = self.queue.front() {
            if matches!(frame, Frame::Step { .. }) {
                return;
            }
            if let Some(frame) = self.queue.pop_front() {
                self.apply(frame);
            }
        }
    }

    /// Applies a frame that isn't a step.
    fn apply(&mut self, frame: Frame) {
        match frame {
            Frame::Roster { place, roster } => {
                self.place = Some(place);
                self.roster = roster;
            }
            Frame::RoundOver { wins, .. } => self.wins = wins,
            Frame::RoundStart(RoundStart { round, seed, wins }) => {
                self.sim = Some(self.game.new_round(seed, round));
                self.round = round;
                self.wins = wins;
                self.step = 0;
            }
            // A catch-up and a `Closing` never wait, and a step is
            // applied apart.
            Frame::Step { .. } | Frame::Joined { .. } | Frame::Closing => {}
        }
    }

    /// Applies the step at the head of the queue: false when its number
    /// isn't the next or its hash differs from the sim's after it.
    fn apply_step(&mut self) -> bool {
        let Some(Frame::Step { step, inputs, hash }) = self.queue.pop_front() else {
            return true;
        };
        self.steps -= 1;
        let Some(sim) = self.sim.as_mut() else {
            return true;
        };
        if step != self.step.wrapping_add(1) {
            return false;
        }
        self.events.extend(sim.step(&inputs));
        self.step = step;
        hash.is_none_or(|hash| hash == sim.hash())
    }
}

#[cfg(test)]
mod tests;
