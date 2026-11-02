//! A source of samples in bounded chunks.

use alloc::vec::Vec;

use crate::error::Error;
use crate::mix::SharedPcm;
use crate::pcm::{Limits, Pcm, PcmFormat};

/// How many samples [`read_all`] reads at a time.
const CHUNK_SAMPLES: usize = 8192;

/// Something that yields whole frames on request, as a decoder working through
/// a file does. A caller decides how much it holds at once: a read never fills
/// more than the buffer it is given.
pub trait Source {
    /// The rate and channels of what it yields.
    fn format(&self) -> PcmFormat;

    /// How many frames it yields from its start, if that is known without
    /// decoding.
    fn total_frames(&self) -> Option<u64>;

    /// Fills `out` with the next whole frames, interleaved, and says how many
    /// frames that was. `0` means it has ended. Short of the end it gives at
    /// least one, and a buffer without room for one is [`Error::NoRoom`].
    ///
    /// A source that mustn't keep its reader waiting gives [`Error::Underrun`] when
    /// it has no frame ready yet.
    fn read(&mut self, out: &mut [i16]) -> Result<usize, Error>;

    /// Goes back, so that the next read gives frame `frame`, without waiting:
    /// what a mix asks at a loop's join. A source that can't says
    /// [`Error::NoRewind`], as this does unless it is written otherwise.
    fn rewind_to(&mut self, frame: u64) -> Result<(), Error> {
        let _ = frame;
        Err(Error::NoRewind)
    }
}

/// Whole frames `out` has room for, of `format`'s, or why it has none.
pub(crate) fn room(format: PcmFormat, out: &[i16]) -> Result<usize, Error> {
    let channels = format.channels.count();
    let frames = out.len() / usize::from(channels);
    if frames == 0 {
        return Err(Error::NoRoom {
            samples: out.len(),
            channels,
        });
    }
    Ok(frames)
}

/// Everything `source` still yields, as one [`Pcm`], within `limits`.
pub fn read_all(source: &mut impl Source, limits: &Limits) -> Result<Pcm, Error> {
    let format = source.format();
    let channels = usize::from(format.channels.count());
    if let Some(frames) = source.total_frames().filter(|f| *f > limits.max_frames) {
        return Err(Error::TooLong {
            frames,
            limit: limits.max_frames,
        });
    }
    let mut samples = Vec::new();
    let mut chunk = [0; CHUNK_SAMPLES];
    loop {
        let frames = source.read(&mut chunk)?;
        if frames == 0 {
            return Pcm::new(format, samples, limits);
        }
        samples.extend_from_slice(&chunk[..frames * channels]);
        let frames = samples.len() as u64 / channels as u64;
        if frames > limits.max_frames {
            return Err(Error::TooLong {
                frames,
                limit: limits.max_frames,
            });
        }
    }
}

/// A [`Pcm`]'s samples as a source.
#[derive(Debug, Clone)]
pub struct PcmSource<'a> {
    pcm: &'a Pcm,
    /// The next sample.
    at: usize,
}

impl<'a> PcmSource<'a> {
    /// `pcm`'s samples, from the start.
    pub const fn new(pcm: &'a Pcm) -> Self {
        Self { pcm, at: 0 }
    }
}

impl Source for PcmSource<'_> {
    fn format(&self) -> PcmFormat {
        self.pcm.format()
    }

    fn total_frames(&self) -> Option<u64> {
        Some(self.pcm.frames())
    }

    fn read(&mut self, out: &mut [i16]) -> Result<usize, Error> {
        let format = self.format();
        let channels = usize::from(format.channels.count());
        let room = room(format, out)?;
        let left = &self.pcm.samples()[self.at..];
        let frames = room.min(left.len() / channels);
        let count = frames * channels;
        out[..count].copy_from_slice(&left[..count]);
        self.at += count;
        Ok(frames)
    }

    /// Past the end, the next read gives nothing.
    fn rewind_to(&mut self, frame: u64) -> Result<(), Error> {
        self.at = sample_index(self.pcm, frame);
        Ok(())
    }
}

/// Where frame `frame` of `pcm` starts, or its end if it hasn't one.
fn sample_index(pcm: &Pcm, frame: u64) -> usize {
    let channels = u64::from(pcm.format().channels.count());
    let frame = frame.min(pcm.frames());
    usize::try_from(frame * channels).unwrap_or(pcm.samples().len())
}

/// A shared [`Pcm`]'s samples as a source that owns its share, so a voice can
/// stream it from anywhere.
#[derive(Debug, Clone)]
pub struct SharedSource {
    pcm: SharedPcm,
    /// The next sample.
    at: usize,
}

impl SharedSource {
    /// `pcm`'s samples, from the start.
    pub const fn new(pcm: SharedPcm) -> Self {
        Self { pcm, at: 0 }
    }
}

impl Source for SharedSource {
    fn format(&self) -> PcmFormat {
        self.pcm.format()
    }

    fn total_frames(&self) -> Option<u64> {
        Some(self.pcm.frames())
    }

    fn read(&mut self, out: &mut [i16]) -> Result<usize, Error> {
        let mut source = PcmSource {
            pcm: &self.pcm,
            at: self.at,
        };
        let frames = source.read(out)?;
        self.at = source.at;
        Ok(frames)
    }

    /// Past the end, the next read gives nothing.
    fn rewind_to(&mut self, frame: u64) -> Result<(), Error> {
        self.at = sample_index(&self.pcm, frame);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pcm::Channels;
    use proptest::prelude::*;

    fn format(channels: Channels) -> PcmFormat {
        PcmFormat::new(8_000, u32::from(channels.count()), &Limits::default())
            .unwrap_or_else(|e| panic!("{e}"))
    }

    fn pcm(channels: Channels, samples: Vec<i16>) -> Pcm {
        Pcm::new(format(channels), samples, &Limits::default()).unwrap_or_else(|e| panic!("{e}"))
    }

    #[test]
    fn a_read_gives_whole_frames_up_to_the_room() {
        let sound = pcm(Channels::Stereo, vec![1, 2, 3, 4, 5, 6]);
        let mut source = sound.source();
        assert_eq!(source.format(), sound.format());
        assert_eq!(source.total_frames(), Some(3));
        let mut out = [0; 5];
        assert_eq!(source.read(&mut out), Ok(2));
        assert_eq!(out, [1, 2, 3, 4, 0]);
        assert_eq!(source.read(&mut out), Ok(1));
        assert_eq!(out[..2], [5, 6]);
        assert_eq!(source.read(&mut out), Ok(0));
        assert_eq!(source.read(&mut out), Ok(0));
    }

    #[test]
    fn a_read_without_room_for_a_frame_says_so() {
        let sound = pcm(Channels::Stereo, vec![1, 2]);
        let mut one = [0; 1];
        assert_eq!(
            sound.source().read(&mut one),
            Err(Error::NoRoom {
                samples: 1,
                channels: 2
            })
        );
        assert_eq!(
            sound.source().read(&mut []),
            Err(Error::NoRoom {
                samples: 0,
                channels: 2
            })
        );
    }

    /// Says nothing of its length and has no end, as far as a reader can tell.
    /// A reader that ignores its limit fails here, before it fills the memory.
    struct Endless(Channels, u32);

    impl Source for Endless {
        fn format(&self) -> PcmFormat {
            format(self.0)
        }

        fn total_frames(&self) -> Option<u64> {
            None
        }

        fn read(&mut self, out: &mut [i16]) -> Result<usize, Error> {
            self.1 += 1;
            assert!(self.1 < 16, "read on long past the limit");
            out.fill(7);
            Ok(out.len() / usize::from(self.0.count()))
        }
    }

    #[test]
    fn a_shared_source_reads_and_goes_back_as_a_borrowed_one_does() {
        let sound = pcm(Channels::Stereo, vec![1, 2, 3, 4, 5, 6]);
        let mut shared = SharedSource::new(SharedPcm::new(sound.clone()));
        let mut borrowed = sound.source();
        assert_eq!(shared.format(), sound.format());
        assert_eq!(shared.total_frames(), Some(3));
        let (mut a, mut b) = ([0; 4], [0; 4]);
        assert_eq!(shared.read(&mut a), Ok(2));
        assert_eq!(borrowed.read(&mut b), Ok(2));
        assert_eq!(a, [1, 2, 3, 4]);
        assert_eq!(a, b);
        assert_eq!(shared.rewind_to(1), Ok(()));
        assert_eq!(borrowed.rewind_to(1), Ok(()));
        assert_eq!(shared.read(&mut a), Ok(2));
        assert_eq!(borrowed.read(&mut b), Ok(2));
        assert_eq!(a, [3, 4, 5, 6]);
        assert_eq!(a, b);
        assert_eq!(shared.rewind_to(9), Ok(()));
        assert_eq!(borrowed.rewind_to(9), Ok(()));
        assert_eq!(shared.read(&mut a), Ok(0));
        assert_eq!(borrowed.read(&mut b), Ok(0));
    }

    #[test]
    fn a_source_goes_back_only_if_it_says_it_can() {
        let mut endless = Endless(Channels::Mono, 0);
        assert_eq!(endless.rewind_to(0), Err(Error::NoRewind));
    }

    #[test]
    fn read_all_stops_at_the_limit_of_an_endless_source() {
        let limits = Limits::new(8_000, 10_000);
        for (channels, chunk) in [(Channels::Mono, 8192), (Channels::Stereo, 4096)] {
            let Err(Error::TooLong { frames, limit }) =
                read_all(&mut Endless(channels, 0), &limits)
            else {
                panic!("it should have stopped");
            };
            assert_eq!(limit, 10_000);
            // It counts frames, and goes no further than one chunk past.
            assert!(frames > 10_000 && frames <= 10_000 + chunk, "{frames}");
        }
    }

    #[test]
    fn read_all_refuses_a_known_length_over_the_limit_before_reading() {
        let sound = pcm(Channels::Mono, vec![0; 5]);
        assert_eq!(
            read_all(&mut sound.source(), &Limits::new(8_000, 4)),
            Err(Error::TooLong {
                frames: 5,
                limit: 4
            })
        );
    }

    #[test]
    fn read_all_takes_what_is_exactly_at_the_limit() {
        let sound = pcm(Channels::Mono, vec![3; 5]);
        assert_eq!(
            read_all(&mut sound.source(), &Limits::new(8_000, 5)).as_ref(),
            Ok(&sound)
        );
    }

    proptest! {
        /// However it is cut into chunks, what comes out is what went in.
        #[test]
        fn chunks_rebuild_the_whole(
            samples in proptest::collection::vec(any::<i16>(), 0..512),
            stereo: bool,
            room in 2usize..64,
        ) {
            let channels = if stereo { Channels::Stereo } else { Channels::Mono };
            let whole = samples.len() / usize::from(channels.count());
            let samples = samples[..whole * usize::from(channels.count())].to_vec();
            let sound = pcm(channels, samples.clone());
            let mut source = sound.source();
            let mut out = vec![0; room];
            let mut rebuilt = Vec::new();
            loop {
                let frames = source.read(&mut out).unwrap_or_else(|e| panic!("{e}"));
                if frames == 0 {
                    break;
                }
                prop_assert!(frames * usize::from(channels.count()) <= room);
                rebuilt.extend_from_slice(&out[..frames * usize::from(channels.count())]);
            }
            prop_assert_eq!(rebuilt, samples);
            let again = read_all(&mut sound.source(), &Limits::default());
            prop_assert_eq!(again.as_ref(), Ok(&sound));
        }
    }
}
