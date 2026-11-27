//! The server's side of a room: [`Room`], what it writes each step
//! ([`Outbox`]) and its counts ([`RoomStats`]).

use std::collections::VecDeque;

use shuyi_core::SimRng;
use shuyi_core::tick::{ActionId, SeatInput, TickInput, TickSim};

use super::wire::{ClientMsg, Frame, Roster, RoundStart, fit};
use super::{Checked, Full, HASH_EVERY, Joined, RoomGame, RoundEnd, Seat, SeatKind};
use crate::PeerId;

/// The most press-and-release edges a seat keeps between two steps: a
/// quick double turn survives, a flood doesn't pile up.
const QUEUED: usize = 2;

/// What one [`Room::step`] asks the host to do: frames to send, in order,
/// and peers to close.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct Outbox {
    /// Each frame and the peer it goes to, in the order to send them.
    /// The host encodes each ([`Frame::encode`]) and sends it on the
    /// [`Reliable`](crate::Channel::Reliable) channel.
    pub frames: Vec<(PeerId, Frame)>,
    /// Peers the room let go, silent too long: the host closes them.
    pub drop: Vec<PeerId>,
}

impl Outbox {
    /// Nothing to send.
    pub fn new() -> Self {
        Self::default()
    }

    /// Forgets what was written, keeping the room for more.
    pub fn clear(&mut self) {
        self.frames.clear();
        self.drop.clear();
    }

    fn send(&mut self, peer: PeerId, frame: Frame) {
        self.frames.push((peer, frame));
    }
}

/// A room's counts, for a server's stats.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct RoomStats {
    /// Seats ridden by players.
    pub humans: usize,
    /// Seats ridden by the game's bot.
    pub bots: usize,
    /// Players waiting for the next round's seat.
    pub watchers: usize,
    /// Steps run in this round.
    pub step: u32,
    /// The round, the first 1.
    pub round: u32,
}

/// A seat's latch: the last held set, the edges waiting for their steps,
/// and the steps since its player was last heard from.
#[derive(Debug, Clone, Default)]
struct Latch {
    held: Vec<ActionId>,
    edges: VecDeque<(Vec<ActionId>, Vec<ActionId>)>,
    quiet: u32,
}

impl Latch {
    /// Takes an `Input`: its held set replaces the last, and its presses
    /// and releases wait for a step of their own, while fewer than
    /// [`QUEUED`] wait.
    fn latch(&mut self, input: SeatInput) {
        let input = fit(input);
        self.held = input.held;
        let edge = !(input.pressed.is_empty() && input.released.is_empty());
        if edge && self.edges.len() < QUEUED {
            self.edges.push_back((input.pressed, input.released));
        }
    }

    /// The seat's input for the next step: the held set and the oldest
    /// waiting edges.
    fn input(&mut self) -> SeatInput {
        let (pressed, released) = self.edges.pop_front().unwrap_or_default();
        SeatInput::new(pressed, self.held.clone(), released)
    }
}

/// A room on the server: seats ridden by peers or the game's bot, the real
/// sim, its rounds, and what every peer is sent. It does no I/O: a host
/// hands it what peers do ([`join`](Self::join), [`leave`](Self::leave),
/// [`receive`](Self::receive)) and calls [`step`](Self::step) at the
/// game's [`STEPS_PER_SECOND`](RoomGame::STEPS_PER_SECOND), sending what
/// each step writes.
///
/// - **Joining** takes the lowest bot seat whose rider is still in the
///   round ([`RoomGame::seat_alive`]), from the next step; with none, the
///   peer watches until the next round seats it. A joiner is sent the
///   catch-up ([`Frame::Joined`]) at the next step.
/// - **Leaving**, or [`5 * STEPS_PER_SECOND`](RoomGame::STEPS_PER_SECOND)
///   steps without an `Input` or an `Alive`, gives the seat back to the
///   bot at the next step. A silent peer, seated or watching, is listed
///   in [`Outbox::drop`].
/// - **Inputs:** a seat's last `held` set counts; its presses and releases
///   wait, two at most, and each waits for a step of its own, so two
///   quick presses between two steps are both played.
/// - **Rounds:** a round ends when [`RoomGame::round_winner`] says so, or
///   as a draw at [`MAX_ROUND_STEPS`](RoomGame::MAX_ROUND_STEPS); after
///   [`PAUSE_STEPS`](RoomGame::PAUSE_STEPS) steps the next starts from a
///   seed the room's rng draws, and seats the watchers.
///
/// ```
/// use shuyi_net::room::testing::Dots;
/// use shuyi_net::room::{Full, Joined, Outbox, Room, RoomGame};
/// use shuyi_net::PeerId;
///
/// let mut room = Room::new(Dots, 1);
/// for (peer, seat) in (10..).zip(0..Dots::SEATS) {
///     assert_eq!(room.join(PeerId(peer)), Ok(Joined::Seat(seat)));
/// }
/// assert_eq!(room.join(PeerId(99)), Err(Full));
/// room.leave(PeerId(12));
/// let mut out = Outbox::new();
/// room.step(&mut out);
/// assert_eq!(room.stats().humans, Dots::SEATS - 1);
/// assert_eq!(room.stats().bots, 1);
/// ```
pub struct Room<G: RoomGame> {
    game: G,
    rng: SimRng,
    sim: G::Sim,
    round: u32,
    /// This round's seed.
    seed: u64,
    /// Steps run in this round.
    step: u32,
    seats: Vec<Seat>,
    latches: Vec<Latch>,
    /// The watchers in the order they joined, each with its quiet steps.
    watchers: Vec<(PeerId, u32)>,
    wins: Vec<u32>,
    /// This round's steps' inputs, oldest first: the catch-up.
    log: Vec<TickInput>,
    /// Between rounds: the steps left before the next.
    pause: Option<u32>,
    /// Peers owed a catch-up at the next step.
    owed: Vec<PeerId>,
    roster_changed: bool,
    round_started: bool,
}

impl<G: RoomGame> Room<G> {
    /// A room of bots in round 1. Its rng, from `seed`, draws each round's
    /// seed and feeds the bots.
    pub fn new(game: G, seed: u64) -> Self {
        let () = Checked::<G>::FITS;
        let mut rng = SimRng::new(seed);
        let round_seed = draw_seed(&mut rng);
        let sim = game.new_round(round_seed, 1);
        Self {
            game,
            rng,
            sim,
            round: 1,
            seed: round_seed,
            step: 0,
            seats: vec![Seat::Bot; G::SEATS],
            latches: vec![Latch::default(); G::SEATS],
            watchers: Vec::new(),
            wins: vec![0; G::SEATS],
            log: Vec::new(),
            pause: None,
            owed: Vec::new(),
            roster_changed: false,
            round_started: false,
        }
    }

    /// The game.
    pub fn game(&self) -> &G {
        &self.game
    }

    /// The round's sim, as the last step left it.
    pub fn sim(&self) -> &G::Sim {
        &self.sim
    }

    /// Who rides each seat, by seat number.
    pub fn seats(&self) -> &[Seat] {
        &self.seats
    }

    /// Each seat's rounds won.
    pub fn wins(&self) -> &[u32] {
        &self.wins
    }

    /// Where `peer` is, if it is in the room.
    pub fn place(&self, peer: PeerId) -> Option<Joined> {
        if let Some(seat) = self.seat_of(peer) {
            Some(Joined::Seat(seat))
        } else {
            self.watchers
                .iter()
                .any(|(p, _)| *p == peer)
                .then_some(Joined::Watching)
        }
    }

    /// Its counts.
    pub fn stats(&self) -> RoomStats {
        let humans = self.humans();
        RoomStats {
            humans,
            bots: G::SEATS - humans,
            watchers: self.watchers.len(),
            step: self.step,
            round: self.round,
        }
    }

    /// Seats `peer` in the lowest bot seat whose rider is alive, or lets it
    /// watch until the next round when none is; it is sent the catch-up at
    /// the next step. A peer already in the room gets its place back.
    pub fn join(&mut self, peer: PeerId) -> Result<Joined, Full> {
        if let Some(place) = self.place(peer) {
            return Ok(place);
        }
        if self.humans() + self.watchers.len() >= G::SEATS {
            return Err(Full);
        }
        let free = (0..G::SEATS)
            .find(|&seat| self.seats[seat] == Seat::Bot && G::seat_alive(&self.sim, seat));
        let place = if let Some(seat) = free {
            self.sit(seat, peer, 0);
            Joined::Seat(seat)
        } else {
            self.watchers.push((peer, 0));
            Joined::Watching
        };
        self.owed.push(peer);
        Ok(place)
    }

    /// Takes `peer` out: its seat is the bot's from the next step. A peer
    /// not in the room is left alone.
    pub fn leave(&mut self, peer: PeerId) {
        if let Some(seat) = self.seat_of(peer) {
            self.seats[seat] = Seat::Bot;
            self.latches[seat] = Latch::default();
            self.roster_changed = true;
        }
        self.watchers.retain(|(p, _)| *p != peer);
        self.owed.retain(|p| *p != peer);
    }

    /// Takes a message from `peer`. Anything from it counts as heard; an
    /// `Input` from a watcher, and anything from a peer not in the room,
    /// is ignored.
    pub fn receive(&mut self, peer: PeerId, msg: ClientMsg) {
        let seat = self.seat_of(peer);
        if let Some(seat) = seat {
            self.latches[seat].quiet = 0;
        } else if let Some((_, quiet)) = self.watchers.iter_mut().find(|(p, _)| *p == peer) {
            *quiet = 0;
        } else {
            return;
        }
        match msg {
            ClientMsg::Input(input) => {
                if let Some(seat) = seat {
                    self.latches[seat].latch(input);
                }
            }
            ClientMsg::Resync => {
                if !self.owed.contains(&peer) {
                    self.owed.push(peer);
                }
            }
            ClientMsg::Leave => self.leave(peer),
            ClientMsg::Alive => {}
        }
    }

    /// Runs one step of room time: lets the silent go, counts down a pause
    /// or starts the next round, sends what changed (catch-ups, a round's
    /// start, the roster), and steps the sim, sending each peer the step's
    /// inputs and, at its end, the round's result.
    pub fn step(&mut self, out: &mut Outbox) {
        self.drop_silent(out);
        match self.pause {
            Some(0) => self.start_round(),
            Some(left) => self.pause = Some(left - 1),
            None => {}
        }
        self.send_news(out);
        if self.pause.is_none() {
            self.play(out);
        }
        for latch in &mut self.latches {
            latch.quiet = latch.quiet.saturating_add(1);
        }
        for (_, quiet) in &mut self.watchers {
            *quiet = quiet.saturating_add(1);
        }
    }

    fn humans(&self) -> usize {
        self.seats.iter().filter(|s| **s != Seat::Bot).count()
    }

    fn seat_of(&self, peer: PeerId) -> Option<usize> {
        self.seats.iter().position(|s| *s == Seat::Human(peer))
    }

    /// Seats `peer` in `seat` with a fresh latch.
    fn sit(&mut self, seat: usize, peer: PeerId, quiet: u32) {
        self.seats[seat] = Seat::Human(peer);
        self.latches[seat] = Latch {
            quiet,
            ..Latch::default()
        };
        self.roster_changed = true;
    }

    /// Every peer in the room and its place: the seated in seat order, then
    /// the watchers in the order they came.
    fn members(&self) -> Vec<(PeerId, Joined)> {
        let seated = self
            .seats
            .iter()
            .enumerate()
            .filter_map(|(n, seat)| match seat {
                Seat::Human(peer) => Some((*peer, Joined::Seat(n))),
                Seat::Bot => None,
            });
        let watching = self.watchers.iter().map(|(p, _)| (*p, Joined::Watching));
        seated.chain(watching).collect()
    }

    fn roster(&self) -> Roster {
        Roster {
            seats: self.seats.iter().map(|s| SeatKind::from(*s)).collect(),
        }
    }

    fn round_start(&self) -> RoundStart {
        RoundStart {
            round: self.round,
            seed: self.seed,
            wins: self.wins.clone(),
        }
    }

    /// Lets go of every peer silent for five seconds of steps.
    fn drop_silent(&mut self, out: &mut Outbox) {
        let limit = G::STEPS_PER_SECOND.saturating_mul(5);
        for seat in 0..G::SEATS {
            if let Seat::Human(peer) = self.seats[seat]
                && self.latches[seat].quiet >= limit
            {
                self.leave(peer);
                out.drop.push(peer);
            }
        }
        let (silent, kept) = core::mem::take(&mut self.watchers)
            .into_iter()
            .partition(|(_, quiet)| *quiet >= limit);
        self.watchers = kept;
        for (peer, _) in silent {
            self.owed.retain(|p| *p != peer);
            out.drop.push(peer);
        }
    }

    /// The next round: a new seed and sim, an empty log, and the watchers
    /// in the lowest bot seats.
    fn start_round(&mut self) {
        self.round = self.round.saturating_add(1);
        self.seed = draw_seed(&mut self.rng);
        self.sim = self.game.new_round(self.seed, self.round);
        self.step = 0;
        self.log.clear();
        self.pause = None;
        for latch in &mut self.latches {
            latch.edges.clear();
        }
        for (peer, quiet) in core::mem::take(&mut self.watchers) {
            match self.seats.iter().position(|s| *s == Seat::Bot) {
                Some(seat) => self.sit(seat, peer, quiet),
                // Humans and watchers never pass the seats.
                None => self.watchers.push((peer, quiet)),
            }
        }
        self.round_started = true;
    }

    /// Sends what changed since the last step: a catch-up to each peer owed
    /// one, and to the others the round's start and the roster.
    fn send_news(&mut self, out: &mut Outbox) {
        let owed = core::mem::take(&mut self.owed);
        let roster = self.roster();
        for (peer, place) in self.members() {
            if owed.contains(&peer) {
                let frame = Frame::Joined {
                    place,
                    roster: roster.clone(),
                    round_start: self.round_start(),
                    steps: self.log.clone(),
                };
                out.send(peer, frame);
                continue;
            }
            if self.round_started {
                out.send(peer, Frame::RoundStart(self.round_start()));
            }
            if self.roster_changed {
                let roster = roster.clone();
                out.send(peer, Frame::Roster { place, roster });
            }
        }
        self.round_started = false;
        self.roster_changed = false;
    }

    /// Steps the sim with every seat's input, sends it, and ends the round
    /// when it is over.
    fn play(&mut self, out: &mut Outbox) {
        let mut seats = Vec::with_capacity(G::SEATS);
        for seat in 0..G::SEATS {
            seats.push(match self.seats[seat] {
                Seat::Human(_) => self.latches[seat].input(),
                Seat::Bot => fit(G::bot_input(&self.sim, seat, &mut self.rng)),
            });
        }
        let inputs = TickInput::new(seats);
        self.sim.step(&inputs);
        self.step += 1;
        let hash = self
            .step
            .is_multiple_of(HASH_EVERY)
            .then(|| self.sim.hash());
        let members = self.members();
        for (peer, _) in &members {
            let frame = Frame::Step {
                step: self.step,
                inputs: inputs.clone(),
                hash,
            };
            out.send(*peer, frame);
        }
        self.log.push(inputs);
        let winner = match G::round_winner(&self.sim) {
            RoundEnd::Going if self.step < G::MAX_ROUND_STEPS => return,
            RoundEnd::Won(seat) if seat < G::SEATS => Some(seat),
            RoundEnd::Going | RoundEnd::Won(_) | RoundEnd::Draw => None,
        };
        if let Some(seat) = winner {
            self.wins[seat] = self.wins[seat].saturating_add(1);
        }
        for (peer, _) in members {
            let wins = self.wins.clone();
            out.send(peer, Frame::RoundOver { winner, wins });
        }
        self.pause = Some(G::PAUSE_STEPS);
    }
}

/// A round's seed: two draws, the first the high half.
fn draw_seed(rng: &mut SimRng) -> u64 {
    let high = u64::from(rng.next_u32());
    let low = u64::from(rng.next_u32());
    (high << 32).wrapping_add(low)
}

#[cfg(test)]
mod tests;
