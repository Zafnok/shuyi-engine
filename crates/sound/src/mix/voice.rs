//! One playing sound: where it is, how loud, and its part of a block of the mix.

use super::resample::{Position, between};
use super::stream::{Fetch, Slot};
use super::{Params, RATE_MAX, RATE_MIN, Region, SharedPcm, UNITY};

/// The fractional bits a voice's volume carries while it fades.
const VOLUME_FRACTION: u32 = 16;

/// What a voice plays.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Origin {
    /// Nothing: a sound or a stream that wasn't there. It ends at once.
    Nothing,
    /// A sound of the bank, by index.
    Sound(usize),
    /// A stream slot, by index.
    Stream(usize),
}

/// What a voice's next frame after the current one is, as far as it knows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Next {
    /// Not asked for yet.
    Unknown,
    Frame([i16; 2]),
    /// The source has no more.
    End,
}

/// What a voice does at the end of what it plays.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Repeat {
    Once,
    /// From its first frame.
    Whole,
    /// From the start of this, at its end.
    Region(Region),
}

impl Repeat {
    const fn of(params: Params) -> Self {
        match params.region {
            Some(region) => Self::Region(region),
            None if params.looped => Self::Whole,
            None => Self::Once,
        }
    }

    const fn region(self) -> Option<Region> {
        match self {
            Self::Region(region) => Some(region),
            _ => None,
        }
    }
}

/// What a voice stopped for in a block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Stop {
    End,
    Underrun,
}

/// What the mixer's sounds and streams are to a voice.
pub(super) struct Sources<'a> {
    pub(super) sounds: &'a [Option<SharedPcm>],
    pub(super) streams: &'a mut [Slot],
}

/// One voice.
#[derive(Debug, Clone)]
pub(super) struct Voice {
    pub(super) generation: u32,
    pub(super) active: bool,
    pub(super) paused: bool,
    /// It ended and nobody has been told.
    pub(super) ended: bool,
    pub(super) origin: Origin,
    repeat: Repeat,
    position: Position,
    /// The frame a sound's next fetch gives. A stream's knows its own.
    next_frame: u64,
    /// How many source frames the position has passed and the voice hasn't.
    pending: u64,
    current: [i16; 2],
    next: Next,
    /// In 1/4096, with [`VOLUME_FRACTION`] bits more.
    volume: u32,
    /// Where a fade ends: it means nothing without one.
    target: u32,
    fade_step: i64,
    fade_left: u64,
    balance: (u16, u16),
}

impl Voice {
    pub(super) fn new() -> Self {
        Self {
            generation: 0,
            active: false,
            paused: false,
            ended: false,
            origin: Origin::Nothing,
            repeat: Repeat::Once,
            position: Position::new(1, 65_536, 1),
            next_frame: 0,
            pending: 0,
            current: [0; 2],
            next: Next::Unknown,
            volume: 0,
            target: 0,
            fade_step: 0,
            fade_left: 0,
            balance: (UNITY, UNITY),
        }
    }

    /// Starts `origin` from its first frame. `source_rate` is its rate.
    pub(super) fn start(
        &mut self,
        generation: u32,
        origin: Origin,
        source_rate: u32,
        output_rate: u32,
        params: Params,
    ) {
        let rate = params.rate.clamp(RATE_MIN, RATE_MAX);
        *self = Self {
            generation,
            active: true,
            origin,
            repeat: Repeat::of(params),
            position: Position::new(source_rate, rate, output_rate),
            // Its first fetch is the first frame.
            pending: 1,
            volume: u32::from(params.volume) << VOLUME_FRACTION,
            balance: params.balance,
            ..Self::new()
        };
    }

    /// The voice is over: its slot is free, and a stream's source waits to go.
    pub(super) fn finish(&mut self, streams: &mut [Slot]) {
        self.active = false;
        self.ended = true;
        if let Origin::Stream(index) = self.origin
            && let Some(slot) = streams.get_mut(index)
        {
            slot.retired = true;
        }
    }

    /// Its volume now, in 1/4096.
    pub(super) fn volume(&self) -> u16 {
        u16::try_from(self.volume >> VOLUME_FRACTION).unwrap_or(u16::MAX)
    }

    /// Jumps to `volume`, ending any fade.
    pub(super) fn set_volume(&mut self, volume: u16) {
        self.volume = u32::from(volume) << VOLUME_FRACTION;
        self.target = self.volume;
        self.fade_left = 0;
    }

    /// Goes to `volume` in a straight line over `frames` frames, ending on it
    /// exactly.
    pub(super) fn fade_to(&mut self, volume: u16, frames: u64) {
        if frames == 0 {
            self.set_volume(volume);
            return;
        }
        self.target = u32::from(volume) << VOLUME_FRACTION;
        let distance = i64::from(self.target) - i64::from(self.volume);
        self.fade_step = distance / i64::try_from(frames).unwrap_or(i64::MAX);
        self.fade_left = frames;
    }

    pub(super) const fn set_balance(&mut self, left: u16, right: u16) {
        self.balance = (left, right);
    }

    /// The left and right gains, in 1/2^32.
    fn gains(&self) -> (i64, i64) {
        let volume = i64::from(self.volume >> (VOLUME_FRACTION - 8));
        (
            volume * i64::from(self.balance.0),
            volume * i64::from(self.balance.1),
        )
    }

    /// One fade's frame goes by.
    fn fade(&mut self) {
        self.fade_left -= 1;
        self.volume = if self.fade_left == 0 {
            self.target
        } else {
            u32::try_from(i64::from(self.volume) + self.fade_step).unwrap_or(self.target)
        };
    }

    /// Its source's next frame.
    fn fetch(&mut self, sources: &mut Sources<'_>) -> Fetch {
        match self.origin {
            Origin::Nothing => Fetch::End,
            Origin::Sound(index) => {
                let Some(Some(pcm)) = sources.sounds.get(index) else {
                    return Fetch::End;
                };
                let channels = usize::from(pcm.format().channels.count());
                let samples = pcm.samples();
                let frames = (samples.len() / channels) as u64;
                let region = self.repeat.region();
                let end = region.map_or(frames, |r| r.end().min(frames));
                if self.next_frame >= end {
                    if self.repeat == Repeat::Once {
                        return Fetch::End;
                    }
                    self.next_frame = region.map_or(0, Region::start);
                    if self.next_frame >= end {
                        return Fetch::End;
                    }
                }
                // Under `frames`, which a slice's length gave.
                let at = usize::try_from(self.next_frame).unwrap_or(0) * channels;
                self.next_frame += 1;
                Fetch::Frame([samples[at], samples[at + channels - 1]])
            }
            Origin::Stream(index) => match sources.streams.get_mut(index) {
                Some(slot) => slot.next(self.repeat == Repeat::Whole, self.repeat.region()),
                None => Fetch::End,
            },
        }
    }

    /// Catches up with the position. A stop here leaves it to try again.
    fn settle(&mut self, sources: &mut Sources<'_>) -> Result<(), Stop> {
        while self.pending > 0 {
            let frame = match self.next {
                Next::Frame(frame) => frame,
                Next::End => return Err(Stop::End),
                Next::Unknown => match self.fetch(sources) {
                    Fetch::Frame(frame) => frame,
                    Fetch::End => return Err(Stop::End),
                    Fetch::Underrun => return Err(Stop::Underrun),
                },
            };
            self.current = frame;
            self.next = Next::Unknown;
            self.pending -= 1;
        }
        Ok(())
    }

    /// The frame at the position: the next one is only asked for when it is
    /// part of it. At a source's very end the last frame holds.
    fn frame(&mut self, sources: &mut Sources<'_>) -> Result<[i32; 2], Stop> {
        let fraction = self.position.fraction();
        if fraction == 0 {
            return Ok([i32::from(self.current[0]), i32::from(self.current[1])]);
        }
        if self.next == Next::Unknown {
            self.next = match self.fetch(sources) {
                Fetch::Frame(frame) => Next::Frame(frame),
                Fetch::End => Next::End,
                Fetch::Underrun => return Err(Stop::Underrun),
            };
        }
        let next = match self.next {
            Next::Frame(frame) => frame,
            _ => self.current,
        };
        Ok([
            between(self.current[0], next[0], fraction),
            between(self.current[1], next[1], fraction),
        ])
    }

    /// Adds this voice to a block of the mix (`channels` a frame), and says how
    /// many frames of it it had nothing for though it hasn't ended.
    pub(super) fn mix(
        &mut self,
        sources: &mut Sources<'_>,
        block: &mut [i32],
        channels: usize,
    ) -> u64 {
        let frames = block.len() / channels;
        let (mut left_gain, mut right_gain) = self.gains();
        for (done, out) in block.chunks_exact_mut(channels).enumerate() {
            let frame = self.settle(sources).and_then(|()| self.frame(sources));
            let [left, right] = match frame {
                Ok(frame) => frame,
                Err(Stop::End) => {
                    self.finish(sources.streams);
                    return 0;
                }
                Err(Stop::Underrun) => return (frames - done) as u64,
            };
            let (left, right) = (i64::from(left) * left_gain, i64::from(right) * right_gain);
            // Within `i32`: 16 bits of sample and 40 of gain, less the 32 shifted.
            if channels == 2 {
                let left = i32::try_from((left + (1 << 31)) >> 32).unwrap_or(0);
                let right = i32::try_from((right + (1 << 31)) >> 32).unwrap_or(0);
                out[0] = out[0].saturating_add(left);
                out[1] = out[1].saturating_add(right);
            } else {
                let mean = i32::try_from((left + right + (1 << 32)) >> 33).unwrap_or(0);
                out[0] = out[0].saturating_add(mean);
            }
            if self.fade_left > 0 {
                self.fade();
                (left_gain, right_gain) = self.gains();
            }
            self.pending = self.position.advance();
        }
        0
    }
}
