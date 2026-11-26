//! The byte form of a step's input: fixed, compact and little-endian, and
//! never serde's, so a wire stays the same whatever serde does. Its one
//! user so far is the room's wire (ticket 2614), which sends each step's
//! [`TickInput`] to every client.
//!
//! - A seat ([`SeatInput::encode`]) is one header byte whose bits 0, 1 and
//!   2 say whether `pressed`, `held` and `released` are non-empty (the
//!   other bits 0), then, for each non-empty list in that order, a `u8`
//!   count and its [`ActionId`]s as `u16` little-endian.
//! - A step ([`TickInput::encode`]) is a `u8` seat count, then each seat.
//!
//! One input has one form: an empty list is a clear header bit, never a
//! count of 0. So an idle step of eight seats is 9 bytes, and a press adds
//! 3.

use alloc::vec::Vec;
use core::fmt;

use super::{ActionId, MAX_SEATS, SeatInput, TickInput};

/// A seat's list of actions too long for its byte form: more than 255.
#[derive(Debug, Clone, PartialEq, Eq)]
// What it says may grow (which list, say; ADR-0039).
#[non_exhaustive]
pub struct TooManyActions {
    /// How many actions the list held.
    len: usize,
}

impl fmt::Display for TooManyActions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "a seat's list holds {} actions; its byte form holds at most {}",
            self.len,
            u8::MAX
        )
    }
}

impl core::error::Error for TooManyActions {}

impl SeatInput {
    /// Its three lists, in the order of their header bits.
    fn lists(&self) -> [&[ActionId]; 3] {
        [&self.pressed, &self.held, &self.released]
    }

    /// Writes its byte form at the end of `out` (the module's doc says the
    /// layout). A list of more than 255 actions writes nothing.
    ///
    /// ```
    /// use shuyi_core::tick::{ActionId, SeatInput};
    ///
    /// let mut out = Vec::new();
    /// let seat = SeatInput::new(vec![ActionId(2)], vec![ActionId(2)], vec![]);
    /// seat.encode(&mut out).unwrap();
    /// assert_eq!(out, [0b011, 1, 2, 0, 1, 2, 0]);
    /// assert_eq!(SeatInput::decode(&out), Some((seat, 7)));
    /// ```
    ///
    /// # Errors
    ///
    /// [`TooManyActions`] when a list holds more than 255 actions.
    pub fn encode(&self, out: &mut Vec<u8>) -> Result<(), TooManyActions> {
        let lists = self.lists();
        let mut header = 0_u8;
        let mut counts = [0_u8; 3];
        for (bit, list) in lists.iter().enumerate() {
            counts[bit] =
                u8::try_from(list.len()).map_err(|_| TooManyActions { len: list.len() })?;
            if !list.is_empty() {
                header |= 1 << bit;
            }
        }
        out.push(header);
        for (list, count) in lists.iter().zip(counts) {
            if count > 0 {
                out.push(count);
                for action in *list {
                    out.extend_from_slice(&action.0.to_le_bytes());
                }
            }
        }
        Ok(())
    }

    /// Reads a seat's byte form from the start of `bytes`: the seat and how
    /// many bytes it took. `None` on bytes too short, a reserved header bit
    /// set, or a list's count of 0.
    pub fn decode(bytes: &[u8]) -> Option<(Self, usize)> {
        let (&header, mut rest) = bytes.split_first()?;
        if header & !0b111 != 0 {
            return None;
        }
        let mut lists: [Vec<ActionId>; 3] = Default::default();
        for (bit, list) in lists.iter_mut().enumerate() {
            if header & (1 << bit) == 0 {
                continue;
            }
            let (&count, after) = rest.split_first()?;
            if count == 0 {
                return None;
            }
            let (ids, after) = after.split_at_checked(usize::from(count) * 2)?;
            *list = ids
                .as_chunks::<2>()
                .0
                .iter()
                .map(|&pair| ActionId(u16::from_le_bytes(pair)))
                .collect();
            rest = after;
        }
        let [pressed, held, released] = lists;
        Some((Self::new(pressed, held, released), bytes.len() - rest.len()))
    }
}

impl TickInput {
    /// Writes its byte form at the end of `out`: a `u8` seat count, then
    /// each seat's ([`SeatInput::encode`]). Like [`new`](Self::new), it
    /// keeps the first [`MAX_SEATS`] seats. A seat that can't be written
    /// leaves `out` as it was.
    ///
    /// ```
    /// use shuyi_core::tick::{ActionId, SeatInput, TickInput};
    ///
    /// let idle = TickInput::new(vec![SeatInput::default(); 8]);
    /// let mut out = Vec::new();
    /// idle.encode(&mut out).unwrap();
    /// assert_eq!(out.len(), 9);
    /// assert_eq!(TickInput::decode(&out), Some((idle, 9)));
    /// ```
    ///
    /// # Errors
    ///
    /// [`TooManyActions`] when a seat's list holds more than 255 actions.
    pub fn encode(&self, out: &mut Vec<u8>) -> Result<(), TooManyActions> {
        let start = out.len();
        let seats = &self.seats[..self.seats.len().min(MAX_SEATS)];
        // At most `MAX_SEATS`, so it fits.
        out.push(u8::try_from(seats.len()).unwrap_or(u8::MAX));
        for seat in seats {
            if let Err(e) = seat.encode(out) {
                out.truncate(start);
                return Err(e);
            }
        }
        Ok(())
    }

    /// Reads a step's byte form from the start of `bytes`: the step and how
    /// many bytes it took. `None` on a seat count over [`MAX_SEATS`] or a
    /// seat [`SeatInput::decode`] refuses.
    pub fn decode(bytes: &[u8]) -> Option<(Self, usize)> {
        let (&count, _) = bytes.split_first()?;
        let count = usize::from(count);
        if count > MAX_SEATS {
            return None;
        }
        let mut read = 1;
        let mut seats = Vec::with_capacity(count);
        for _ in 0..count {
            let (seat, len) = SeatInput::decode(&bytes[read..])?;
            seats.push(seat);
            read += len;
        }
        Some((Self { seats }, read))
    }
}

#[cfg(test)]
mod tests;
