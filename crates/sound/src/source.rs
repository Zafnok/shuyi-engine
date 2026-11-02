//! A source of samples in bounded chunks.

use alloc::vec::Vec;

use crate::error::Error;
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
    fn read(&mut self, out: &mut [i16]) -> Result<usize, Error>;
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

    /// Says nothing of its length and goes on for ever.
    struct Endless(Channels);

    impl Source for Endless {
        fn format(&self) -> PcmFormat {
            format(self.0)
        }

        fn total_frames(&self) -> Option<u64> {
            None
        }

        fn read(&mut self, out: &mut [i16]) -> Result<usize, Error> {
            out.fill(7);
            Ok(out.len() / usize::from(self.0.count()))
        }
    }

    #[test]
    fn read_all_stops_at_the_limit_of_an_endless_source() {
        let limits = Limits::new(8_000, 10_000);
        for (channels, chunk) in [(Channels::Mono, 8192), (Channels::Stereo, 4096)] {
            let Err(Error::TooLong { frames, limit }) = read_all(&mut Endless(channels), &limits)
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
