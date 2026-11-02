//! Streams: what a worker hands the mixer, and what a voice reads it through.
//!
//! A worker owns a decoder and a [`Feeder`]; the mixer owns the [`Feed`], the
//! other end of a fixed ring of decoded frames. What the mixer holds of a stream
//! is that ring and a small buffer, however long the stream is.
//!
//! The contract between them:
//!
//! - The worker keeps the ring full ahead of the mix ([`Feeder::fill_from`]), and
//!   [`Feeder::close`]s it after the last frame.
//! - A read of an empty ring that isn't closed is [`Error::Underrun`]. The voice
//!   stays where it is and sounds nothing until frames come: an underrun delays a
//!   stream, and loses and repeats nothing. [`Mixer::underrun_frames`] counts it.
//! - The worker supplies a loop's frames too, as [`LoopedRegion`] does, so that the
//!   way back to the start of a region is worked out and in the ring before the
//!   mix reaches the end of it.
//!
//! [`Mixer::underrun_frames`]: super::Mixer::underrun_frames

use alloc::boxed::Box;
use core::sync::atomic::{AtomicBool, Ordering};

use super::queue::Ring;
use super::{Region, Shared};
use crate::error::Error;
use crate::pcm::{Channels, PcmFormat};
use crate::source::Source;

/// How many frames a voice holds of its stream.
const BUFFER_FRAMES: usize = 128;
/// How many samples a worker moves or discards at a time.
const CHUNK_SAMPLES: usize = 512;

/// A frame as one word of a ring: a mono frame has the sample twice.
fn frame_word(left: i16, right: i16) -> u32 {
    let [a, b] = left.to_le_bytes();
    let [c, d] = right.to_le_bytes();
    u32::from_le_bytes([a, b, c, d])
}

fn word_frame(word: u32) -> [i16; 2] {
    let [a, b, c, d] = word.to_le_bytes();
    [i16::from_le_bytes([a, b]), i16::from_le_bytes([c, d])]
}

/// A ring of `capacity` frames of `format` (rounded up to a power of two): the end
/// a worker writes, and the end the mixer reads.
pub fn feed(format: PcmFormat, capacity: usize) -> (Feeder, Feed) {
    let ring = Shared::new(Ring::new(capacity, 1));
    let closed = Shared::new(AtomicBool::new(false));
    (
        Feeder {
            ring: Shared::clone(&ring),
            closed: Shared::clone(&closed),
            format,
        },
        Feed {
            ring,
            closed,
            format,
        },
    )
}

/// The end of a [`feed`] a worker writes.
#[derive(Debug)]
pub struct Feeder {
    ring: Shared<Ring>,
    closed: Shared<AtomicBool>,
    format: PcmFormat,
}

impl Feeder {
    /// How many frames the ring has room for now.
    pub fn room(&self) -> usize {
        self.ring.room()
    }

    /// Writes as many whole frames of `samples` as there is room for, interleaved,
    /// and says how many that was.
    pub fn write(&mut self, samples: &[i16]) -> usize {
        let channels = usize::from(self.format.channels.count());
        let frames = (samples.len() / channels).min(self.ring.room());
        self.ring.write_words(frames, |frame| {
            let left = samples[frame * channels];
            frame_word(left, samples[frame * channels + channels - 1])
        });
        frames
    }

    /// Moves frames from `source` until the ring has no room or `source` ends,
    /// when it closes the ring, and says how many frames moved. This is the
    /// decoder's work, for a worker: it is never done in a mix.
    pub fn fill_from(&mut self, source: &mut impl Source) -> Result<usize, Error> {
        if source.format().channels != self.format.channels {
            return Err(Error::Channels(u32::from(source.format().channels.count())));
        }
        let channels = usize::from(self.format.channels.count());
        let mut chunk = [0; CHUNK_SAMPLES];
        let mut moved = 0;
        loop {
            let frames = self.ring.room().min(CHUNK_SAMPLES / channels);
            if frames == 0 {
                return Ok(moved);
            }
            let read = source.read(&mut chunk[..frames * channels])?;
            if read == 0 {
                self.close();
                return Ok(moved);
            }
            moved += self.write(&chunk[..read * channels]);
        }
    }

    /// There are no more frames than those written.
    pub fn close(&mut self) {
        self.closed.store(true, Ordering::Release);
    }
}

/// The end of a [`feed`] the mixer reads, a [`Source`] that never waits.
#[derive(Debug)]
pub struct Feed {
    ring: Shared<Ring>,
    closed: Shared<AtomicBool>,
    format: PcmFormat,
}

impl Source for Feed {
    fn format(&self) -> PcmFormat {
        self.format
    }

    fn total_frames(&self) -> Option<u64> {
        None
    }

    fn read(&mut self, out: &mut [i16]) -> Result<usize, Error> {
        let channels = usize::from(self.format.channels.count());
        let room = crate::source::room(self.format, out)?;
        // Before the ring: frames written before the close are there to see.
        let closed = self.closed.load(Ordering::Acquire);
        let frames = self.ring.read_words(room, |frame, word| {
            let [left, right] = word_frame(word);
            out[frame * channels] = left;
            out[frame * channels + channels - 1] = right;
        });
        if frames == 0 && !closed {
            return Err(Error::Underrun);
        }
        Ok(frames)
    }

    /// Nothing to do: the worker has put the frames a loop goes back to in the
    /// ring already, right after the last, so the next read gives them.
    fn rewind_to(&mut self, _frame: u64) -> Result<(), Error> {
        Ok(())
    }
}

/// A source that plays up to the end of a region, then from its start, for ever,
/// for a worker to [`fill_from`](Feeder::fill_from). `restart` gives the source
/// from its first frame again, however that is done (a seek, or a new decoder over
/// the same bytes), and the frames before the region are read and dropped: what
/// comes out is sample-exact at the join whatever the decoder's own pages are.
#[derive(Debug)]
pub struct LoopedRegion<S, F> {
    source: S,
    restart: F,
    region: Region,
    /// The next frame `source` gives.
    at: u64,
}

impl<S: Source, F: FnMut() -> Result<S, Error>> LoopedRegion<S, F> {
    /// `source`, from its first frame, looping `region`.
    pub const fn new(source: S, region: Region, restart: F) -> Self {
        Self {
            source,
            restart,
            region,
            at: 0,
        }
    }

    /// Goes back to the start of the region.
    fn back(&mut self) -> Result<(), Error> {
        self.source = (self.restart)()?;
        self.at = 0;
        let channels = usize::from(self.source.format().channels.count());
        let mut chunk = [0; CHUNK_SAMPLES];
        while self.at < self.region.start() {
            let left = self.region.start() - self.at;
            let frames = usize::try_from(left).map_or(CHUNK_SAMPLES / channels, |left| {
                left.min(CHUNK_SAMPLES / channels)
            });
            let read = self.source.read(&mut chunk[..frames * channels])?;
            if read == 0 {
                return Ok(());
            }
            self.at += read as u64;
        }
        Ok(())
    }
}

impl<S: Source, F: FnMut() -> Result<S, Error>> Source for LoopedRegion<S, F> {
    fn format(&self) -> PcmFormat {
        self.source.format()
    }

    fn total_frames(&self) -> Option<u64> {
        None
    }

    fn read(&mut self, out: &mut [i16]) -> Result<usize, Error> {
        let channels = usize::from(self.format().channels.count());
        let room = crate::source::room(self.format(), out)?;
        let mut went_back = false;
        loop {
            if self.at >= self.region.end() {
                self.back()?;
                went_back = true;
            }
            let frames = usize::try_from(self.region.end().saturating_sub(self.at))
                .map_or(room, |left| left.min(room));
            let read = if frames == 0 {
                0
            } else {
                self.source.read(&mut out[..frames * channels])?
            };
            if read > 0 {
                self.at += read as u64;
                return Ok(read);
            }
            // The file ends before the region does, which is where it comes round;
            // a second time with nothing read means there is nothing to loop.
            if went_back {
                return Ok(0);
            }
            self.back()?;
            went_back = true;
        }
    }
}

/// What a voice gets when it asks for its source's next frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Fetch {
    /// The frame, left then right.
    Frame([i16; 2]),
    /// It has ended.
    End,
    /// It has no frame yet, and will.
    Underrun,
}

/// A stream the mixer holds: its source and what a voice hasn't played of it.
pub(super) struct Slot {
    pub(super) source: Option<Box<dyn Source + Send>>,
    pub(super) generation: u32,
    /// A voice is playing it or has.
    pub(super) used: bool,
    /// A voice played it and is done with it: it waits to be taken away.
    pub(super) retired: bool,
    buffer: Box<[i16]>,
    /// The next sample of `buffer`.
    start: usize,
    /// The end of what `buffer` holds.
    end: usize,
    /// The next frame the source gives.
    read_at: u64,
    /// How many samples a frame of the source has.
    channels: usize,
}

impl Slot {
    pub(super) fn new() -> Self {
        Self {
            source: None,
            generation: 0,
            used: false,
            retired: false,
            buffer: alloc::vec![0; BUFFER_FRAMES * 2].into_boxed_slice(),
            start: 0,
            end: 0,
            read_at: 0,
            channels: 2,
        }
    }

    /// Holds `source`, ready to play from its start.
    pub(super) fn hold(&mut self, source: Box<dyn Source + Send>) {
        self.channels = match source.format().channels {
            Channels::Mono => 1,
            Channels::Stereo => 2,
        };
        self.source = Some(source);
        self.generation = self.generation.wrapping_add(1);
        self.used = false;
        self.retired = false;
        self.start = 0;
        self.end = 0;
        self.read_at = 0;
    }

    /// The source's next frame, with a loop's way back. No decoding: a source a
    /// mixer reads is one that has its frames ready.
    pub(super) fn next(&mut self, looped: bool, region: Option<Region>) -> Fetch {
        if self.start >= self.end
            && let Some(stop) = self.refill(looped, region)
        {
            return stop;
        }
        let frame = [
            self.buffer[self.start],
            self.buffer[self.start + self.channels - 1],
        ];
        self.start += self.channels;
        Fetch::Frame(frame)
    }

    /// Fills the buffer, or says why it can't be.
    fn refill(&mut self, looped: bool, region: Option<Region>) -> Option<Fetch> {
        let source = self.source.as_mut()?;
        let mut went_back = false;
        loop {
            if let Some(region) = region.filter(|region| self.read_at >= region.end()) {
                match source.rewind_to(region.start()) {
                    Ok(()) => self.read_at = region.start(),
                    Err(Error::Underrun) => return Some(Fetch::Underrun),
                    Err(_) => return Some(Fetch::End),
                }
                went_back = true;
            }
            let frames = region.map_or(BUFFER_FRAMES, |region| {
                usize::try_from(region.end().saturating_sub(self.read_at))
                    .map_or(BUFFER_FRAMES, |left| left.min(BUFFER_FRAMES))
            });
            match source.read(&mut self.buffer[..frames * self.channels]) {
                Ok(0) => {
                    // A source that ends before its region's end comes round there
                    // too, and one with nothing after the way back has ended.
                    if went_back || !(looped || region.is_some()) {
                        return Some(Fetch::End);
                    }
                    let to = region.map_or(0, Region::start);
                    match source.rewind_to(to) {
                        Ok(()) => self.read_at = to,
                        Err(Error::Underrun) => return Some(Fetch::Underrun),
                        Err(_) => return Some(Fetch::End),
                    }
                    went_back = true;
                }
                Ok(read) => {
                    self.read_at += read as u64;
                    self.start = 0;
                    self.end = read * self.channels;
                    return None;
                }
                Err(Error::Underrun) => return Some(Fetch::Underrun),
                Err(_) => return Some(Fetch::End),
            }
        }
    }
}
