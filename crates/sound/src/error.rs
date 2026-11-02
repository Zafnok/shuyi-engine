//! What can go wrong with decoded sound.

use alloc::string::String;

/// Why samples couldn't be made or read.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// A sound has one or two channels.
    #[error("{0} channels; a sound has one or two")]
    Channels(u32),
    /// A rate of zero, or over the limit.
    #[error("{rate} samples a second; it must be from 1 to {max}")]
    Rate {
        /// The rate asked for.
        rate: u32,
        /// The limit's largest.
        max: u32,
    },
    /// Samples that don't divide into frames.
    #[error("{samples} samples aren't whole frames of {channels} channels")]
    PartialFrame {
        /// How many samples there are.
        samples: u64,
        /// How many channels a frame has.
        channels: u16,
    },
    /// More frames than the limit lets one source yield.
    #[error("{frames} frames; the limit is {limit}")]
    TooLong {
        /// How many there are, or the most seen so far.
        frames: u64,
        /// The limit.
        limit: u64,
    },
    /// A [`read`](crate::Source::read) into room for less than one frame.
    #[error("room for {samples} samples can't hold a frame of {channels} channels")]
    NoRoom {
        /// How many samples the buffer holds.
        samples: usize,
        /// How many channels a frame has.
        channels: u16,
    },
    /// A valid file whose samples aren't one of the kinds read here.
    #[error("{bits}-bit {kind} samples aren't read here")]
    SampleKind {
        /// `integer` or `float`.
        kind: &'static str,
        /// The width of one.
        bits: u16,
    },
    /// A file that doesn't read.
    #[error("not a readable {format} file: {reason}")]
    Malformed {
        /// `WAV` or `Ogg Vorbis`.
        format: &'static str,
        /// What the library said.
        reason: String,
    },
}
