//! From a mix to what a device takes: its sample format, its channels and its rate.
//! Plain code on every target, so that its tests run with no device. Nothing here
//! allocates after [`Boundary::new`], since it runs on the audio thread.

/// The fractional bits of a boundary's place between two frames.
const FRACTION_BITS: u32 = 16;

/// A sample a device can take, from one of the mix's.
pub trait DeviceSample: Copy {
    /// `sample` as this format: the same fraction of full scale.
    fn from_mix(sample: i16) -> Self;
}

impl DeviceSample for f32 {
    fn from_mix(sample: i16) -> Self {
        Self::from(sample) / 32_768.0
    }
}

impl DeviceSample for f64 {
    fn from_mix(sample: i16) -> Self {
        Self::from(sample) / 32_768.0
    }
}

impl DeviceSample for i16 {
    fn from_mix(sample: i16) -> Self {
        sample
    }
}

impl DeviceSample for i32 {
    fn from_mix(sample: i16) -> Self {
        Self::from(sample) << 16
    }
}

impl DeviceSample for u16 {
    fn from_mix(sample: i16) -> Self {
        sample.cast_unsigned() ^ 0x8000
    }
}

impl DeviceSample for u8 {
    fn from_mix(sample: i16) -> Self {
        (sample.cast_unsigned() >> 8).to_le_bytes()[0] ^ 0x80
    }
}

/// Fills a device's buffers from a mix of another format: its channels mapped, and
/// its rate converted in a straight line between frames when the device's differs.
/// When it doesn't, frames go through untouched.
#[derive(Debug)]
pub struct Boundary {
    /// The mix's channels, and the device's.
    from: usize,
    to: usize,
    /// One frame of the mix per this many device frames, in 16.16.
    step: u64,
    /// Where the device is between `previous` and the frame at `at`, in 16.16.
    place: u64,
    /// The last frame of the mix behind the device's place, and the one ahead.
    previous: [i16; 2],
    /// What came of the mix and hasn't been used.
    held: Box<[i16]>,
    at: usize,
    filled: usize,
}

impl Boundary {
    /// From a mix of `from` channels (one or two) at `from_rate`, asked for `frames`
    /// at a time, to a device of `to` channels at `to_rate`.
    pub fn new(from: usize, from_rate: u32, frames: usize, to: usize, to_rate: u32) -> Self {
        Self {
            from: from.clamp(1, 2),
            to: to.max(1),
            step: (u64::from(from_rate) << FRACTION_BITS) / u64::from(to_rate.max(1)),
            place: 0,
            previous: [0; 2],
            held: vec![0; frames.max(1) * from.clamp(1, 2)].into_boxed_slice(),
            at: 0,
            filled: 0,
        }
    }

    /// The next frame of the mix, left then right, asking `mix` for more when there
    /// is none. A mono mix has its one sample in both.
    fn next(&mut self, mix: &mut dyn FnMut(&mut [i16])) -> [i16; 2] {
        if self.at >= self.filled {
            mix(&mut self.held);
            self.at = 0;
            self.filled = self.held.len();
        }
        let frame = [self.held[self.at], self.held[self.at + self.from - 1]];
        self.at += self.from;
        frame
    }

    /// The frame at the device's place.
    fn frame(&mut self, mix: &mut dyn FnMut(&mut [i16])) -> [i16; 2] {
        let one = 1u64 << FRACTION_BITS;
        if self.step == one {
            return self.next(mix);
        }
        let mut ahead = self.next_held(mix);
        while self.place >= one {
            self.place -= one;
            self.previous = ahead;
            self.at += self.from;
            ahead = self.next_held(mix);
        }
        let between = |a: i16, b: i16| {
            let (a, b) = (i64::from(a), i64::from(b));
            let mixed = a + (b - a) * i64::try_from(self.place).unwrap_or(0) / (1 << FRACTION_BITS);
            i16::try_from(mixed).unwrap_or(0)
        };
        let frame = [
            between(self.previous[0], ahead[0]),
            between(self.previous[1], ahead[1]),
        ];
        self.place += self.step;
        frame
    }

    /// The frame ahead, left where it is.
    fn next_held(&mut self, mix: &mut dyn FnMut(&mut [i16])) -> [i16; 2] {
        if self.at >= self.filled {
            mix(&mut self.held);
            self.at = 0;
            self.filled = self.held.len();
        }
        [self.held[self.at], self.held[self.at + self.from - 1]]
    }

    /// Fills `out`, whole frames of the device's channels, from `mix`. Past two
    /// channels the rest are silence, and a mono device has the mean of the two.
    pub fn fill<T: DeviceSample>(&mut self, out: &mut [T], mix: &mut dyn FnMut(&mut [i16])) {
        let silence = T::from_mix(0);
        for device in out.chunks_exact_mut(self.to) {
            let [left, right] = self.frame(mix);
            match device {
                [only] => {
                    *only = T::from_mix(i16::midpoint(left, right));
                }
                [first, second, rest @ ..] => {
                    *first = T::from_mix(left);
                    *second = T::from_mix(right);
                    rest.fill(silence);
                }
                [] => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CHUNK_FRAMES: usize = 1024;

    /// A mix counting up on the left and down on the right, from 1.
    fn counting() -> impl FnMut(&mut [i16]) {
        let mut next = 1i16;
        move |out: &mut [i16]| {
            for frame in out.as_chunks_mut::<2>().0 {
                frame[0] = next;
                frame[1] = -next;
                next = next.wrapping_add(1);
            }
        }
    }

    #[test]
    fn a_float_device_gets_the_same_fraction_of_full_scale() {
        assert!((f32::from_mix(i16::MIN) + 1.0).abs() < f32::EPSILON);
        assert!(f32::from_mix(0).abs() < f32::EPSILON);
        assert!((f32::from_mix(16_384) - 0.5).abs() < f32::EPSILON);
        assert!((f32::from_mix(i16::MAX) - 32_767.0 / 32_768.0).abs() < f32::EPSILON);
    }

    #[test]
    fn every_sixteen_bit_sample_survives_a_float_device_and_back() {
        for sample in i16::MIN..=i16::MAX {
            let back = f32::from_mix(sample) * 32_768.0;
            assert!((back - f32::from(sample)).abs() < f32::EPSILON, "{sample}");
        }
    }

    #[test]
    fn integer_devices_keep_the_order_and_the_extremes() {
        assert_eq!(i16::from_mix(-5), -5);
        assert_eq!(i32::from_mix(i16::MIN), i32::MIN);
        assert_eq!(i32::from_mix(1), 1 << 16);
        assert_eq!(u16::from_mix(i16::MIN), 0);
        assert_eq!(u16::from_mix(0), 0x8000);
        assert_eq!(u16::from_mix(i16::MAX), u16::MAX);
        assert_eq!(u8::from_mix(i16::MIN), 0);
        assert_eq!(u8::from_mix(0), 0x80);
        assert_eq!(u8::from_mix(i16::MAX), u8::MAX);
        assert!((f64::from_mix(16_384) - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn at_the_same_rate_frames_go_through_untouched() {
        let mut boundary = Boundary::new(2, 48_000, CHUNK_FRAMES, 2, 48_000);
        let mut mix = counting();
        // Past the chunk the mix is asked for, in sizes that line up with nothing.
        let mut seen = Vec::new();
        for size in [7usize, 1024, 1, 2000] {
            let mut out = vec![0i16; size * 2];
            boundary.fill(&mut out, &mut mix);
            seen.extend(out);
        }
        for (index, frame) in seen.as_chunks::<2>().0.iter().enumerate() {
            let want = i16::try_from(index + 1).unwrap_or(0);
            assert_eq!(*frame, [want, -want]);
        }
    }

    #[test]
    fn a_mono_device_has_the_mean_and_a_surround_one_silence_past_two() {
        let mut mix = |out: &mut [i16]| {
            for frame in out.as_chunks_mut::<2>().0 {
                *frame = [1000, 3000];
            }
        };
        let mut mono = [0i16; 2];
        Boundary::new(2, 8000, CHUNK_FRAMES, 1, 8000).fill(&mut mono, &mut mix);
        assert_eq!(mono, [2000, 2000]);
        let mut surround = [9i16; 8];
        Boundary::new(2, 8000, CHUNK_FRAMES, 4, 8000).fill(&mut surround, &mut mix);
        assert_eq!(surround, [1000, 3000, 0, 0, 1000, 3000, 0, 0]);
    }

    #[test]
    fn a_mono_mix_sounds_on_both_channels() {
        let mut mix = |out: &mut [i16]| out.fill(500);
        let mut stereo = [0i16; 4];
        Boundary::new(1, 8000, CHUNK_FRAMES, 2, 8000).fill(&mut stereo, &mut mix);
        assert_eq!(stereo, [500; 4]);
    }

    #[test]
    fn a_partial_frame_at_the_end_of_a_buffer_is_left_alone() {
        let mut mix = counting();
        let mut out = [9i16; 3];
        Boundary::new(2, 8000, CHUNK_FRAMES, 2, 8000).fill(&mut out, &mut mix);
        assert_eq!(out, [1, -1, 9]);
    }

    #[test]
    fn another_rate_comes_out_in_a_straight_line_at_the_device_rate() {
        // A ramp stays a ramp: one mix frame per two device frames.
        let mut boundary = Boundary::new(2, 8000, CHUNK_FRAMES, 2, 16_000);
        let mut mix = |out: &mut [i16]| {
            for (index, frame) in out.as_chunks_mut::<2>().0.iter_mut().enumerate() {
                let value = i16::try_from(index * 100).unwrap_or(0);
                *frame = [value, value];
            }
        };
        let mut out = [0i16; 16];
        boundary.fill(&mut out, &mut mix);
        let left: Vec<i16> = out.iter().step_by(2).copied().collect();
        assert_eq!(left, [0, 0, 0, 50, 100, 150, 200, 250]);
    }

    #[test]
    fn a_higher_mix_rate_uses_the_frames_up_at_its_rate() {
        // Four seconds' worth of a 48 kHz mix for a 44.1 kHz device asks the mix for
        // what the rates say, within a chunk.
        let mut asked = 0usize;
        let mut mix = |out: &mut [i16]| {
            asked += out.len() / 2;
            out.fill(7);
        };
        let mut boundary = Boundary::new(2, 48_000, CHUNK_FRAMES, 2, 44_100);
        let mut out = vec![0i16; 44_100 * 2];
        boundary.fill(&mut out, &mut mix);
        assert!(
            (48_000..48_000 + CHUNK_FRAMES * 2).contains(&asked),
            "{asked}"
        );
        assert_eq!(out[100], 7);
    }
}
