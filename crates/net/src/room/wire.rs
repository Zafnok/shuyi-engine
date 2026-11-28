//! The room's messages and their bytes: fixed, little-endian and written
//! by hand, never serde's, so the wire stays the same whatever a library
//! does. [`Frame`]s go down, from the room to a client; [`ClientMsg`]s go
//! up. Each message is one byte naming it, then its fields:
//!
//! - a step's inputs are [`TickInput::encode`]'s bytes, and a seat's
//!   input [`SeatInput::encode`]'s;
//! - a seat is a `u8`, `0xFF` for watching (in a place) or a draw (as a
//!   winner);
//! - a roster is a `u8` count of seats, then a `u8` whose bit `n` is set
//!   when seat `n` is a human's;
//! - wins are a `u8` count, then each seat's as a `u32`;
//! - a catch-up's steps are a `u32` count, then each step's inputs.
//!
//! Decoding never panics. It refuses an unknown first byte, bytes that end
//! early or go on past the message, a seat past the game's
//! [`SEATS`](RoomGame::SEATS), and a catch-up longer than its
//! [`MAX_ROUND_STEPS`](RoomGame::MAX_ROUND_STEPS), each with its
//! [`WireError`].
//!
//! ```
//! use shuyi_core::tick::{ActionId, SeatInput};
//! use shuyi_net::room::wire::{ClientMsg, WireError};
//!
//! let msg = ClientMsg::Input(SeatInput::new(vec![ActionId(1)], vec![], vec![]));
//! let bytes = msg.encode();
//! assert_eq!(ClientMsg::decode(&bytes), Ok(msg));
//! assert_eq!(ClientMsg::decode(&[0x80]), Err(WireError::BadInput));
//! assert_eq!(ClientMsg::decode(&[0x7F]), Err(WireError::UnknownTag(0x7F)));
//! ```

use core::fmt;

use shuyi_core::tick::{SeatInput, TickInput};

use super::{Checked, Joined, RoomGame, SeatKind};

/// The version of these messages: a client and a server of other
/// versions don't talk.
pub const PROTOCOL: u16 = 1;

/// The first byte of each message.
mod tag {
    pub const STEP: u8 = 0x01;
    pub const JOINED: u8 = 0x02;
    pub const ROSTER: u8 = 0x03;
    pub const ROUND_OVER: u8 = 0x04;
    pub const ROUND_START: u8 = 0x05;
    pub const CLOSING: u8 = 0x06;
    pub const INPUT: u8 = 0x80;
    pub const ALIVE: u8 = 0x81;
    pub const RESYNC: u8 = 0x82;
    pub const LEAVE: u8 = 0x83;
}

/// A place's byte for watching, and a winner's for a draw.
const NONE: u8 = 0xFF;
/// The byte of a seat past 254: no game's seats reach it.
const PAST: u8 = 0xFE;

/// Who rides each seat, by seat number: what every client is told.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct Roster {
    /// Each seat's rider.
    pub seats: Vec<SeatKind>,
}

impl Roster {
    /// How many seats are humans'.
    pub fn humans(&self) -> usize {
        self.seats.iter().filter(|s| **s == SeatKind::Human).count()
    }
}

/// A round's start: what a client builds its sim from, with the wins so
/// far.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct RoundStart {
    /// The round, the first 1.
    pub round: u32,
    /// The seed the round's sim is built from
    /// ([`RoomGame::new_round`]).
    pub seed: u64,
    /// Each seat's rounds won so far, by seat number.
    pub wins: Vec<u32>,
}

/// A message from the room to a client.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Frame {
    /// A step of the round: every seat's input, which the client steps its
    /// sim with.
    #[non_exhaustive]
    Step {
        /// The step's number in the round, the first 1.
        step: u32,
        /// Every seat's input.
        inputs: TickInput,
        /// The sim's hash after the step, every
        /// [`HASH_EVERY`](super::HASH_EVERY) steps.
        hash: Option<u64>,
    },
    /// The catch-up, on joining and on a [`ClientMsg::Resync`]: the
    /// round's start and every step's inputs since, which bring a client's
    /// sim to the room's.
    #[non_exhaustive]
    Joined {
        /// Where the client is.
        place: Joined,
        /// Who rides each seat.
        roster: Roster,
        /// The round's start.
        round_start: RoundStart,
        /// Each step's inputs since the round began, oldest first.
        steps: Vec<TickInput>,
    },
    /// Who rides each seat changed, from the next step.
    #[non_exhaustive]
    Roster {
        /// Where the client is now: a watcher is seated when a round
        /// starts.
        place: Joined,
        /// Who rides each seat.
        roster: Roster,
    },
    /// The round ended.
    #[non_exhaustive]
    RoundOver {
        /// The seat that won, or none for a draw.
        winner: Option<usize>,
        /// Each seat's rounds won, this one counted.
        wins: Vec<u32>,
    },
    /// A new round starts: its steps follow.
    RoundStart(RoundStart),
    /// The server is shutting down.
    Closing,
}

/// A message from a client to the room.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ClientMsg {
    /// The player's actions changed: what it pressed, holds and released.
    Input(SeatInput),
    /// Nothing changed, but the player is still there.
    Alive,
    /// The client's sim differs from the room's: send a catch-up.
    Resync,
    /// The player leaves the room.
    Leave,
}

/// Bytes that are no message.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum WireError {
    /// The bytes end before the message does.
    Short,
    /// Bytes go on after the message.
    Overlong {
        /// How many bytes are left over.
        extra: usize,
    },
    /// The first byte names no message.
    UnknownTag(u8),
    /// A seat at or past the game's seats.
    NoSuchSeat {
        /// The seat read.
        seat: usize,
        /// The game's seats.
        seats: usize,
    },
    /// More seats than the game has.
    TooManySeats {
        /// How many were read.
        count: usize,
        /// The game's seats.
        seats: usize,
    },
    /// A catch-up longer than the game's longest round.
    CatchUp {
        /// Its steps.
        steps: u32,
        /// The game's [`MAX_ROUND_STEPS`](RoomGame::MAX_ROUND_STEPS).
        max: u32,
    },
    /// A seat's input that isn't in its byte form, or is cut short.
    BadInput,
    /// A field's byte that isn't one of its values: a flag past 1, or a
    /// roster's human past its seats.
    BadField(&'static str),
}

impl fmt::Display for WireError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Short => f.write_str("the message ends early"),
            Self::Overlong { extra } => write!(f, "{extra} bytes follow the message"),
            Self::UnknownTag(tag) => write!(f, "no message starts with byte {tag:#04x}"),
            Self::NoSuchSeat { seat, seats } => {
                write!(f, "seat {seat} is past the game's {seats} seats")
            }
            Self::TooManySeats { count, seats } => {
                write!(f, "{count} seats are more than the game's {seats}")
            }
            Self::CatchUp { steps, max } => {
                write!(
                    f,
                    "a catch-up of {steps} steps is longer than a round's {max}"
                )
            }
            Self::BadInput => f.write_str("a seat's input is not in its byte form"),
            Self::BadField(field) => write!(f, "the {field} holds no allowed value"),
        }
    }
}

impl core::error::Error for WireError {}

/// A seat's input cut to what its byte form holds: each list's first 255
/// actions. The room steps its sim with what it sends, so both agree.
pub(crate) fn fit(mut seat: SeatInput) -> SeatInput {
    let most = usize::from(u8::MAX);
    seat.pressed.truncate(most);
    seat.held.truncate(most);
    seat.released.truncate(most);
    seat
}

impl Frame {
    /// Its bytes. A seat's list of more than 255 actions is cut to its
    /// first 255, as the room cuts what it steps with.
    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::new();
        match self {
            Self::Step { step, inputs, hash } => {
                out.push(tag::STEP);
                put_u32(&mut out, *step);
                match hash {
                    Some(hash) => {
                        out.push(1);
                        out.extend_from_slice(&hash.to_le_bytes());
                    }
                    None => out.push(0),
                }
                put_inputs(&mut out, inputs);
            }
            Self::Joined {
                place,
                roster,
                round_start,
                steps,
            } => {
                out.push(tag::JOINED);
                put_place(&mut out, *place);
                put_roster(&mut out, roster);
                put_round_start(&mut out, round_start);
                put_u32(&mut out, u32::try_from(steps.len()).unwrap_or(u32::MAX));
                for inputs in steps {
                    put_inputs(&mut out, inputs);
                }
            }
            Self::Roster { place, roster } => {
                out.push(tag::ROSTER);
                put_place(&mut out, *place);
                put_roster(&mut out, roster);
            }
            Self::RoundOver { winner, wins } => {
                out.push(tag::ROUND_OVER);
                out.push(winner.map_or(NONE, seat_byte));
                put_wins(&mut out, wins);
            }
            Self::RoundStart(start) => {
                out.push(tag::ROUND_START);
                put_round_start(&mut out, start);
            }
            Self::Closing => out.push(tag::CLOSING),
        }
        out
    }

    /// The frame `bytes` hold, for game `G`: its seats and its longest
    /// round bound what is read.
    pub fn decode<G: RoomGame>(bytes: &[u8]) -> Result<Self, WireError> {
        let () = Checked::<G>::FITS;
        let mut read = Reader {
            rest: bytes,
            seats: G::SEATS,
        };
        let frame = match read.u8()? {
            tag::STEP => {
                let step = read.u32()?;
                let hash = match read.u8()? {
                    0 => None,
                    1 => Some(read.u64()?),
                    _ => return Err(WireError::BadField("hash flag")),
                };
                let inputs = read.inputs()?;
                Self::Step { step, inputs, hash }
            }
            tag::JOINED => {
                let place = read.place()?;
                let roster = read.roster()?;
                let round_start = read.round_start()?;
                let steps = read.u32()?;
                if steps > G::MAX_ROUND_STEPS {
                    return Err(WireError::CatchUp {
                        steps,
                        max: G::MAX_ROUND_STEPS,
                    });
                }
                // Each step's inputs take a byte at least.
                if usize::try_from(steps).map_or(true, |n| n > read.rest.len()) {
                    return Err(WireError::Short);
                }
                let steps = (0..steps)
                    .map(|_| read.inputs())
                    .collect::<Result<_, _>>()?;
                Self::Joined {
                    place,
                    roster,
                    round_start,
                    steps,
                }
            }
            tag::ROSTER => Self::Roster {
                place: read.place()?,
                roster: read.roster()?,
            },
            tag::ROUND_OVER => {
                let winner = match read.u8()? {
                    NONE => None,
                    seat => Some(read.check_seat(seat)?),
                };
                Self::RoundOver {
                    winner,
                    wins: read.wins()?,
                }
            }
            tag::ROUND_START => Self::RoundStart(read.round_start()?),
            tag::CLOSING => Self::Closing,
            other => return Err(WireError::UnknownTag(other)),
        };
        read.finish()?;
        Ok(frame)
    }
}

impl ClientMsg {
    /// Its bytes. A list of more than 255 actions is cut to its first 255.
    pub fn encode(&self) -> Vec<u8> {
        match self {
            Self::Input(seat) => {
                let mut out = vec![tag::INPUT];
                if seat.encode(&mut out).is_err() {
                    out.truncate(1);
                    // Each list fits now.
                    let _ = fit(seat.clone()).encode(&mut out);
                }
                out
            }
            Self::Alive => vec![tag::ALIVE],
            Self::Resync => vec![tag::RESYNC],
            Self::Leave => vec![tag::LEAVE],
        }
    }

    /// The message `bytes` hold.
    pub fn decode(bytes: &[u8]) -> Result<Self, WireError> {
        let mut read = Reader {
            rest: bytes,
            seats: shuyi_core::tick::MAX_SEATS,
        };
        let msg = match read.u8()? {
            tag::INPUT => {
                let (seat, len) = SeatInput::decode(read.rest).ok_or(WireError::BadInput)?;
                read.rest = &read.rest[len..];
                Self::Input(seat)
            }
            tag::ALIVE => Self::Alive,
            tag::RESYNC => Self::Resync,
            tag::LEAVE => Self::Leave,
            other => return Err(WireError::UnknownTag(other)),
        };
        read.finish()?;
        Ok(msg)
    }
}

/// A seat's byte: past 254 it is [`PAST`], so a wrong seat stays wrong
/// rather than reading as watching.
fn seat_byte(seat: usize) -> u8 {
    u8::try_from(seat).map_or(PAST, |b| b.min(PAST))
}

fn put_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn put_place(out: &mut Vec<u8>, place: Joined) {
    out.push(place.seat().map_or(NONE, seat_byte));
}

fn put_roster(out: &mut Vec<u8>, roster: &Roster) {
    out.push(u8::try_from(roster.seats.len()).unwrap_or(u8::MAX));
    // Seats past the eighth have no bit: a roster that long is refused.
    let humans = roster
        .seats
        .iter()
        .take(8)
        .enumerate()
        .filter(|(_, kind)| **kind == SeatKind::Human)
        .map(|(seat, _)| 1_u8 << seat)
        .sum::<u8>();
    out.push(humans);
}

fn put_wins(out: &mut Vec<u8>, wins: &[u32]) {
    out.push(u8::try_from(wins.len()).unwrap_or(u8::MAX));
    for win in wins {
        put_u32(out, *win);
    }
}

fn put_round_start(out: &mut Vec<u8>, start: &RoundStart) {
    put_u32(out, start.round);
    out.extend_from_slice(&start.seed.to_le_bytes());
    put_wins(out, &start.wins);
}

fn put_inputs(out: &mut Vec<u8>, inputs: &TickInput) {
    if inputs.encode(out).is_err() {
        let fitted = TickInput::new(inputs.seats.iter().cloned().map(fit).collect());
        // Each list fits now.
        let _ = fitted.encode(out);
    }
}

/// Reads a message's fields from the front of its bytes.
struct Reader<'a> {
    rest: &'a [u8],
    /// The game's seats.
    seats: usize,
}

impl Reader<'_> {
    fn take<const N: usize>(&mut self) -> Result<[u8; N], WireError> {
        let (head, rest) = self.rest.split_first_chunk::<N>().ok_or(WireError::Short)?;
        self.rest = rest;
        Ok(*head)
    }

    fn u8(&mut self) -> Result<u8, WireError> {
        self.take::<1>().map(|[byte]| byte)
    }

    fn u32(&mut self) -> Result<u32, WireError> {
        self.take().map(u32::from_le_bytes)
    }

    fn u64(&mut self) -> Result<u64, WireError> {
        self.take().map(u64::from_le_bytes)
    }

    fn check_seat(&self, seat: u8) -> Result<usize, WireError> {
        let seat = usize::from(seat);
        if seat < self.seats {
            Ok(seat)
        } else {
            Err(WireError::NoSuchSeat {
                seat,
                seats: self.seats,
            })
        }
    }

    fn check_count(&self, count: u8) -> Result<usize, WireError> {
        let count = usize::from(count);
        if count <= self.seats {
            Ok(count)
        } else {
            Err(WireError::TooManySeats {
                count,
                seats: self.seats,
            })
        }
    }

    fn place(&mut self) -> Result<Joined, WireError> {
        match self.u8()? {
            NONE => Ok(Joined::Watching),
            seat => self.check_seat(seat).map(Joined::Seat),
        }
    }

    fn roster(&mut self) -> Result<Roster, WireError> {
        let count = self.u8()?;
        let count = self.check_count(count)?;
        let humans = self.u8()?;
        if u16::from(humans) >> count != 0 {
            return Err(WireError::BadField("roster"));
        }
        let seats = (0..count)
            .map(|seat| {
                if humans & (1 << seat) == 0 {
                    SeatKind::Bot
                } else {
                    SeatKind::Human
                }
            })
            .collect();
        Ok(Roster { seats })
    }

    fn wins(&mut self) -> Result<Vec<u32>, WireError> {
        let count = self.u8()?;
        let count = self.check_count(count)?;
        (0..count).map(|_| self.u32()).collect()
    }

    fn round_start(&mut self) -> Result<RoundStart, WireError> {
        Ok(RoundStart {
            round: self.u32()?,
            seed: self.u64()?,
            wins: self.wins()?,
        })
    }

    fn inputs(&mut self) -> Result<TickInput, WireError> {
        let &count = self.rest.first().ok_or(WireError::Short)?;
        self.check_count(count)?;
        let (inputs, len) = TickInput::decode(self.rest).ok_or(WireError::BadInput)?;
        self.rest = &self.rest[len..];
        Ok(inputs)
    }

    fn finish(self) -> Result<(), WireError> {
        if self.rest.is_empty() {
            Ok(())
        } else {
            Err(WireError::Overlong {
                extra: self.rest.len(),
            })
        }
    }
}

#[cfg(test)]
mod tests;
