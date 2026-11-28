//! Quick play: a [`Director`] that seats each player who asks to play in
//! a room with a free seat, and opens rooms up to a cap.
//!
//! The player presses play; the director picks the room. What it does:
//!
//! - **A handshake first.** A client's first message is
//!   [`ClientMsg::Hello`], with its [`PROTOCOL`] and its build's
//!   [`Made`]. Another protocol, or anything else first, is refused with
//!   [`Refusal::Protocol`]; another build with [`Refusal::OtherBuild`],
//!   which carries the server's stamp so the client can say what differs.
//!   Either way the link is closed after the refusal. A client that sends
//!   no `Hello` within [`hello_timeout_s`](DirectorConfig::hello_timeout_s)
//!   is let go. Otherwise it is sent [`Frame::Welcome`].
//! - **Play seats the player.** After the welcome, [`ClientMsg::Play`]
//!   puts it in the open room with the most players that still takes one
//!   (ties to the lowest [`RoomId`]): in the lowest seat a bot rides whose
//!   rider is still in the round, or watching until the next round
//!   ([`Room::join`]). With no such room it opens one, up to
//!   [`rooms_max`](DirectorConfig::rooms_max); past that the answer is
//!   [`Refusal::Busy`], the link stays open, and the client may ask again.
//! - **Leaving** (a [`ClientMsg::Leave`]) gives the seat back to the bot;
//!   the player may send `Play` again. A disconnect, or five seconds of
//!   silence, does the same and forgets the player.
//! - **Rooms come and go.** [`rooms_warm`](DirectorConfig::rooms_warm)
//!   rooms of bots run from the start, so a player lands in a round
//!   already under way. A room left with no player closes after
//!   [`idle_close_s`](DirectorConfig::idle_close_s), unless it is one of
//!   the `rooms_warm` oldest open rooms, which stay. A room's id is never
//!   reused while the director runs, so a log line names one room.
//! - **A flood is cut off.** A client that sends more than
//!   [`msgs_per_second`](DirectorConfig::msgs_per_second) messages in a
//!   second is no longer read: its seat goes back to the bot, and its link
//!   is closed.
//!
//! What it doesn't do: no accounts, ratings or queues by skill; no lobby,
//! ready step, room code or invite; no names, chat or room browser; no
//! regions, and no choosing to watch. Players are strangers, matched by
//! free seats alone.
//!
//! Like [`Room`], the director is pure: no socket, no clock, no thread. A
//! server hands it what its transport saw ([`Director::on_event`]), calls
//! [`Director::step`] at the game's
//! [`STEPS_PER_SECOND`](RoomGame::STEPS_PER_SECOND), sends each frame it
//! wrote, encoded, on the [`Reliable`](crate::Channel::Reliable) channel,
//! and then closes each peer in [`Outbox::drop`]:
//!
//! ```
//! use shuyi_core::{ContentHash, Made};
//! use shuyi_net::director::{Director, DirectorConfig};
//! use shuyi_net::room::testing::Dots;
//! use shuyi_net::room::wire::{ClientMsg, Frame};
//! use shuyi_net::room::{Outbox, RoomGame};
//! use shuyi_net::{Channel, NetEvent, PeerId};
//!
//! let made = Made::new("dots", ContentHash(0));
//! let mut director = Director::new(DirectorConfig::new(), Dots, made.clone(), 7);
//! let peer = PeerId(1);
//! let say = |msg: ClientMsg| NetEvent::Message {
//!     peer,
//!     channel: Channel::Reliable,
//!     bytes: msg.encode(),
//! };
//! director.on_event(NetEvent::Connected(peer));
//! director.on_event(say(ClientMsg::hello(made)));
//! director.on_event(say(ClientMsg::Play));
//! let mut out = Outbox::new();
//! director.step(&mut out);
//! assert_eq!(out.frames[0], (peer, Frame::Welcome));
//! assert!(matches!(out.frames[1], (_, Frame::Joined { .. })));
//! // The warm room took it.
//! assert_eq!(director.stats().rooms, 1);
//! assert_eq!(director.stats().bots + director.stats().humans, Dots::SEATS);
//! ```

use std::collections::BTreeMap;

use core::fmt;

use shuyi_core::{Made, SimRng};

use crate::room::wire::{ClientMsg, Frame, PROTOCOL, Refusal};
use crate::room::{Outbox, Room, RoomGame};
use crate::{NetEvent, PeerId};

/// A player silent this many seconds is let go, in a room (the room's own
/// rule) or out of one.
const SILENT_S: u32 = 5;

/// A room's number, from 0, never reused while its director runs.
///
/// ```
/// use shuyi_net::director::RoomId;
///
/// assert_eq!(RoomId(3).to_string(), "room 3");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RoomId(pub u16);

impl fmt::Display for RoomId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "room {}", self.0)
    }
}

/// How a [`Director`] runs: its rooms, its time-outs and its limits.
/// Times are in seconds, counted in the room game's steps.
///
/// ```
/// use shuyi_net::director::DirectorConfig;
///
/// let config = DirectorConfig::new().with_rooms_max(4).with_rooms_warm(9);
/// assert_eq!(config.rooms_max(), 4);
/// // Never more warm rooms than rooms.
/// assert_eq!(config.rooms_warm(), 4);
/// assert_eq!(DirectorConfig::new().retry_after_s(), 10);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct DirectorConfig {
    rooms_max: u16,
    rooms_warm: u16,
    idle_close_s: u16,
    msgs_per_second: u16,
    hello_timeout_s: u16,
    retry_after_s: u16,
}

impl Default for DirectorConfig {
    fn default() -> Self {
        Self::new()
    }
}

impl DirectorConfig {
    /// At most 10 rooms, 1 kept warm; an empty room closes after 60
    /// seconds; 30 messages a second at most; 5 seconds for a `Hello`; a
    /// busy client asks again after 10 seconds.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            rooms_max: 10,
            rooms_warm: 1,
            idle_close_s: 60,
            msgs_per_second: 30,
            hello_timeout_s: 5,
            retry_after_s: 10,
        }
    }

    /// The most rooms open at once.
    #[must_use]
    pub const fn with_rooms_max(mut self, rooms: u16) -> Self {
        self.rooms_max = rooms;
        self
    }

    /// The rooms kept running with bots only, so a new player lands in a
    /// round under way: at most [`rooms_max`](Self::rooms_max).
    #[must_use]
    pub const fn with_rooms_warm(mut self, rooms: u16) -> Self {
        self.rooms_warm = rooms;
        self
    }

    /// How long a room with no player, past the warm ones, runs before it
    /// closes.
    #[must_use]
    pub const fn with_idle_close_s(mut self, seconds: u16) -> Self {
        self.idle_close_s = seconds;
        self
    }

    /// The most messages a client sends in a second; one more, and it is
    /// cut off.
    #[must_use]
    pub const fn with_msgs_per_second(mut self, msgs: u16) -> Self {
        self.msgs_per_second = msgs;
        self
    }

    /// How long a client has, from its connection, to send its `Hello`.
    #[must_use]
    pub const fn with_hello_timeout_s(mut self, seconds: u16) -> Self {
        self.hello_timeout_s = seconds;
        self
    }

    /// The wait a [`Refusal::Busy`] asks for.
    #[must_use]
    pub const fn with_retry_after_s(mut self, seconds: u16) -> Self {
        self.retry_after_s = seconds;
        self
    }

    /// The most rooms open at once.
    pub const fn rooms_max(&self) -> u16 {
        self.rooms_max
    }

    /// The rooms kept running with bots only, at most
    /// [`rooms_max`](Self::rooms_max).
    pub const fn rooms_warm(&self) -> u16 {
        if self.rooms_warm < self.rooms_max {
            self.rooms_warm
        } else {
            self.rooms_max
        }
    }

    /// How long a room with no player runs before it closes.
    pub const fn idle_close_s(&self) -> u16 {
        self.idle_close_s
    }

    /// The most messages a client sends in a second.
    pub const fn msgs_per_second(&self) -> u16 {
        self.msgs_per_second
    }

    /// How long a client has to send its `Hello`.
    pub const fn hello_timeout_s(&self) -> u16 {
        self.hello_timeout_s
    }

    /// The wait a [`Refusal::Busy`] asks for.
    pub const fn retry_after_s(&self) -> u16 {
        self.retry_after_s
    }
}

/// The clients a [`Director`] turned away or cut off, each kind counted
/// from its start.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct Refusals {
    /// [`Refusal::OtherBuild`]s.
    pub other_build: u64,
    /// [`Refusal::Busy`]s.
    pub busy: u64,
    /// [`Refusal::Protocol`]s.
    pub protocol: u64,
    /// Clients cut off for sending past
    /// [`msgs_per_second`](DirectorConfig::msgs_per_second).
    pub flood: u64,
}

/// A [`Director`]'s counts, for a server's stats page.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct DirectorStats {
    /// Open rooms.
    pub rooms: usize,
    /// Seats ridden by players, in every room.
    pub humans: usize,
    /// Seats ridden by bots, in every room.
    pub bots: usize,
    /// Players in a room waiting for its next round's seat.
    pub watchers: usize,
    /// Clients the director serves, in a room or not.
    pub peers: usize,
    /// Clients turned away or cut off.
    pub refused: Refusals,
}

/// Where a client is in the handshake.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Stage {
    /// Linked, no `Hello` yet.
    Connected,
    /// Welcomed, in no room.
    Greeted,
    /// In this room, seated or watching.
    In(RoomId),
}

/// A client the director serves.
#[derive(Debug, Clone, Copy)]
struct Client {
    stage: Stage,
    /// Steps since it was last heard from, or since it connected.
    quiet: u32,
    /// Messages in its current second.
    msgs: u32,
    /// Steps left in its current second; 0 when none runs.
    second_left: u32,
}

/// An open room and the steps it has run with no player.
struct OpenRoom<G: RoomGame> {
    room: Room<G>,
    idle: u32,
}

/// A room's places, as the seating policy reads them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RoomSeats {
    id: RoomId,
    /// Its players, seated or watching.
    players: usize,
    /// Its seats.
    seats: usize,
}

/// Where the seating policy puts a player.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Pick {
    /// In this open room.
    Room(RoomId),
    /// In a room opened for it.
    Open,
    /// Nowhere: every room is full and no more may open.
    Busy,
}

/// The seating policy: the room with the most players that still takes
/// one, ties to the lowest id; else a new room while fewer than
/// `rooms_max` are open; else none.
fn pick(rooms: &[RoomSeats], rooms_max: u16) -> Pick {
    let best = rooms
        .iter()
        .filter(|room| room.players < room.seats)
        .min_by_key(|room| (core::cmp::Reverse(room.players), room.id));
    match best {
        Some(room) => Pick::Room(room.id),
        None if rooms.len() < usize::from(rooms_max) => Pick::Open,
        None => Pick::Busy,
    }
}

/// Seconds as steps of game `G`.
fn steps<G: RoomGame>(seconds: u16) -> u32 {
    u32::from(seconds).saturating_mul(G::STEPS_PER_SECOND)
}

/// Quick play's server side: it greets clients, seats each that asks to
/// play in a room with a free seat, runs the rooms, and opens and closes
/// them, as the [module](self) says.
pub struct Director<G: RoomGame + Clone> {
    config: DirectorConfig,
    game: G,
    made: Made,
    /// Draws each new room's seed.
    rng: SimRng,
    rooms: BTreeMap<RoomId, OpenRoom<G>>,
    /// The next room's id; past `u16::MAX`, no room opens again.
    next_room: u32,
    clients: BTreeMap<PeerId, Client>,
    /// Frames written between steps, sent at the next.
    pending: Vec<(PeerId, Frame)>,
    /// Peers to close at the next step, after their frames.
    closing: Vec<PeerId>,
    refused: Refusals,
}

impl<G: RoomGame + Clone> Director<G> {
    /// A director of rooms of `game`, for clients of build `made`, its
    /// [`rooms_warm`](DirectorConfig::rooms_warm) rooms open with bots.
    /// Each room it opens takes its seed from an rng seeded with `seed`,
    /// so the same events give the same run.
    pub fn new(config: DirectorConfig, game: G, made: Made, seed: u64) -> Self {
        let mut director = Self {
            config,
            game,
            made,
            rng: SimRng::new(seed),
            rooms: BTreeMap::new(),
            next_room: 0,
            clients: BTreeMap::new(),
            pending: Vec::new(),
            closing: Vec::new(),
            refused: Refusals::default(),
        };
        for _ in 0..config.rooms_warm() {
            director.open();
        }
        director
    }

    /// How it runs.
    pub fn config(&self) -> DirectorConfig {
        self.config
    }

    /// The build it serves.
    pub fn made(&self) -> &Made {
        &self.made
    }

    /// Each open room, by id.
    pub fn rooms(&self) -> impl Iterator<Item = (RoomId, &Room<G>)> {
        self.rooms.iter().map(|(id, open)| (*id, &open.room))
    }

    /// The open room `id`.
    pub fn room(&self, id: RoomId) -> Option<&Room<G>> {
        self.rooms.get(&id).map(|open| &open.room)
    }

    /// The room `peer` is in, seated or watching.
    pub fn room_of(&self, peer: PeerId) -> Option<RoomId> {
        match self.clients.get(&peer)?.stage {
            Stage::In(id) => Some(id),
            Stage::Connected | Stage::Greeted => None,
        }
    }

    /// Its counts.
    pub fn stats(&self) -> DirectorStats {
        let mut stats = DirectorStats {
            rooms: self.rooms.len(),
            peers: self.clients.len(),
            refused: self.refused,
            ..DirectorStats::default()
        };
        for open in self.rooms.values() {
            let room = open.room.stats();
            stats.humans += room.humans;
            stats.bots += room.bots;
            stats.watchers += room.watchers;
        }
        stats
    }

    /// Takes what the transport saw: a client linked, a message from one,
    /// or one gone. What it answers is written at the next
    /// [`step`](Self::step).
    pub fn on_event(&mut self, event: NetEvent) {
        match event {
            NetEvent::Connected(peer) => {
                self.clients.entry(peer).or_insert(Client {
                    stage: Stage::Connected,
                    quiet: 0,
                    msgs: 0,
                    second_left: 0,
                });
            }
            NetEvent::Message { peer, bytes, .. } => self.message(peer, &bytes),
            NetEvent::Disconnected { peer, .. } => self.forget(peer),
        }
    }

    /// Runs one step: writes the answers since the last step and the peers
    /// to close, lets go of the silent, steps every open room once in id
    /// order, and closes the rooms idle too long. It adds to `out`; the
    /// caller clears it.
    pub fn step(&mut self, out: &mut Outbox) {
        out.frames.append(&mut self.pending);
        out.drop.append(&mut self.closing);
        self.drop_silent(out);
        for (id, open) in &mut self.rooms {
            let before = out.drop.len();
            open.room.step(out);
            for peer in &out.drop[before..] {
                // Let go by its room: silent too long.
                if self
                    .clients
                    .get(peer)
                    .is_some_and(|client| client.stage == Stage::In(*id))
                {
                    self.clients.remove(peer);
                }
            }
            let stats = open.room.stats();
            if stats.humans + stats.watchers == 0 {
                open.idle = open.idle.saturating_add(1);
            } else {
                open.idle = 0;
            }
        }
        self.close_idle();
    }

    /// Opens a room of bots, if an id is left.
    fn open(&mut self) -> Option<RoomId> {
        let id = RoomId(u16::try_from(self.next_room).ok()?);
        self.next_room += 1;
        let high = u64::from(self.rng.next_u32());
        let seed = (high << 32) | u64::from(self.rng.next_u32());
        let room = Room::new(self.game.clone(), seed);
        self.rooms.insert(id, OpenRoom { room, idle: 0 });
        Some(id)
    }

    /// Takes a client's message: counted against its budget, then read as
    /// its stage reads it.
    fn message(&mut self, peer: PeerId, bytes: &[u8]) {
        let budget = u32::from(self.config.msgs_per_second);
        let Some(client) = self.clients.get_mut(&peer) else {
            // Gone, refused or cut off: not read.
            return;
        };
        if client.second_left == 0 {
            client.second_left = G::STEPS_PER_SECOND;
            client.msgs = 0;
        }
        client.msgs += 1;
        if client.msgs > budget {
            self.refused.flood += 1;
            self.pending.retain(|(p, _)| *p != peer);
            self.forget(peer);
            self.closing.push(peer);
            return;
        }
        client.quiet = 0;
        let stage = client.stage;
        let msg = ClientMsg::decode(bytes);
        match (stage, msg) {
            (Stage::Connected, Ok(ClientMsg::Hello { protocol, made })) => {
                if protocol != PROTOCOL {
                    self.refuse(peer, Refusal::Protocol { ours: PROTOCOL });
                } else if made.check(&self.made).is_err() {
                    let theirs = self.made.clone();
                    self.refuse(peer, Refusal::OtherBuild { theirs });
                } else {
                    self.set_stage(peer, Stage::Greeted);
                    self.pending.push((peer, Frame::Welcome));
                }
            }
            (Stage::Connected, _) => self.refuse(peer, Refusal::Protocol { ours: PROTOCOL }),
            (Stage::Greeted, Ok(ClientMsg::Play)) => self.seat(peer),
            (Stage::In(id), Ok(ClientMsg::Leave)) => {
                if let Some(open) = self.rooms.get_mut(&id) {
                    open.room.leave(peer);
                }
                self.set_stage(peer, Stage::Greeted);
            }
            // A second `Hello` or `Play` is ignored.
            (Stage::In(_), Ok(ClientMsg::Hello { .. } | ClientMsg::Play)) => {}
            (Stage::In(id), Ok(msg)) => {
                if let Some(open) = self.rooms.get_mut(&id) {
                    open.room.receive(peer, msg);
                }
            }
            // Out of a room, only `Play` counts; bytes that are no
            // message are passed over, as a room passes over them.
            (Stage::Greeted | Stage::In(_), _) => {}
        }
    }

    fn set_stage(&mut self, peer: PeerId, stage: Stage) {
        if let Some(client) = self.clients.get_mut(&peer) {
            client.stage = stage;
        }
    }

    /// Answers `why` and closes the link after it.
    fn refuse(&mut self, peer: PeerId, why: Refusal) {
        match why {
            Refusal::OtherBuild { .. } => self.refused.other_build += 1,
            Refusal::Protocol { .. } => self.refused.protocol += 1,
            Refusal::Busy { .. } => self.refused.busy += 1,
        }
        self.pending.push((peer, Frame::Refused { why }));
        self.forget(peer);
        self.closing.push(peer);
    }

    /// Seats a welcomed client where the policy picks, or answers busy.
    fn seat(&mut self, peer: PeerId) {
        let seats: Vec<RoomSeats> = self
            .rooms
            .iter()
            .map(|(id, open)| {
                let stats = open.room.stats();
                RoomSeats {
                    id: *id,
                    players: stats.humans + stats.watchers,
                    seats: G::SEATS,
                }
            })
            .collect();
        let id = match pick(&seats, self.config.rooms_max) {
            Pick::Room(id) => Some(id),
            Pick::Open => self.open(),
            Pick::Busy => None,
        };
        let joined = id.filter(|id| {
            self.rooms
                .get_mut(id)
                .is_some_and(|open| open.room.join(peer).is_ok())
        });
        if let Some(id) = joined {
            self.set_stage(peer, Stage::In(id));
        } else {
            self.refused.busy += 1;
            let retry_after_s = self.config.retry_after_s;
            let why = Refusal::Busy { retry_after_s };
            self.pending.push((peer, Frame::Refused { why }));
        }
    }

    /// Takes `peer` out of its room, if it is in one, and forgets it.
    fn forget(&mut self, peer: PeerId) {
        if let Some(Stage::In(id)) = self.clients.remove(&peer).map(|c| c.stage)
            && let Some(open) = self.rooms.get_mut(&id)
        {
            open.room.leave(peer);
        }
    }

    /// Counts a step for every client out of a room, and lets go of each
    /// that sent no `Hello` in time, or nothing for five seconds after its
    /// welcome. A room keeps its own players' silence.
    fn drop_silent(&mut self, out: &mut Outbox) {
        let hello = steps::<G>(self.config.hello_timeout_s);
        let silent = G::STEPS_PER_SECOND.saturating_mul(SILENT_S);
        let mut gone = Vec::new();
        for (peer, client) in &mut self.clients {
            client.second_left = client.second_left.saturating_sub(1);
            let limit = match client.stage {
                Stage::Connected => hello,
                Stage::Greeted => silent,
                Stage::In(_) => continue,
            };
            client.quiet = client.quiet.saturating_add(1);
            if client.quiet >= limit {
                gone.push(*peer);
            }
        }
        for peer in gone {
            self.clients.remove(&peer);
            out.drop.push(peer);
        }
    }

    /// Closes each room idle for [`idle_close_s`](DirectorConfig::idle_close_s)
    /// past the warm ones: the [`rooms_warm`](DirectorConfig::rooms_warm)
    /// open rooms of the lowest ids never close, so the oldest rooms run on
    /// and fewer than the warm rooms are never open.
    fn close_idle(&mut self) {
        let limit = steps::<G>(self.config.idle_close_s);
        let warm = usize::from(self.config.rooms_warm());
        let idle: Vec<RoomId> = self
            .rooms
            .iter()
            .skip(warm)
            .filter(|(_, open)| open.idle >= limit)
            .map(|(id, _)| *id)
            .collect();
        for id in idle {
            self.rooms.remove(&id);
        }
    }
}

#[cfg(test)]
mod tests;
