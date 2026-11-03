//! The null output driving the mixer through the script of its golden table: a
//! second of a real callback's blocks, sized by the output and not the script, comes
//! to the same samples as `shuyi-sound`'s own run of it (`tests/mix_golden.rs` there).
#![cfg(not(target_arch = "wasm32"))]

use std::sync::mpsc;
use std::time::Duration;

use shuyi_sound::mix::{
    Config, Mixer, Params, RATE_ONE, Region, SharedPcm, SoundId, UNITY, VoiceId,
};
use shuyi_sound::{Channels, Limits, Pcm, PcmFormat, SharedSource};
use shuyi_sound_out::{Output, OutputConfig};

const GOLDEN: &str = include_str!("../../sound/tests/golden/mix.ron");

fn hash(samples: &[i16]) -> String {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for byte in samples.iter().flat_map(|sample| sample.to_le_bytes()) {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{hash:016x}")
}

fn noise(seed: u32, count: usize) -> Vec<i16> {
    let mut state = seed;
    (0..count)
        .map(|_| {
            state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            i16::from_le_bytes([
                (state >> 16).to_le_bytes()[0],
                (state >> 24).to_le_bytes()[0],
            ])
        })
        .collect()
}

fn pcm(rate: u32, channels: Channels, samples: Vec<i16>) -> SharedPcm {
    let format = PcmFormat::new(rate, u32::from(channels.count()), &Limits::default())
        .unwrap_or_else(|e| panic!("{e}"));
    SharedPcm::new(Pcm::new(format, samples, &Limits::default()).unwrap_or_else(|e| panic!("{e}")))
}

/// The golden table's script, stage by stage: `advance` does what comes next and says
/// how many frames play before the stage after.
struct Script {
    mixer: Mixer,
    looped: SoundId,
    whole: SoundId,
    at_48: Option<SharedPcm>,
    voices: Vec<VoiceId>,
    stage: usize,
    /// How many frames this stage has left.
    left: usize,
}

impl Script {
    fn new() -> Self {
        let mut mixer = Mixer::new(Config::new(48_000, Channels::Stereo, 8));
        let at_44 = pcm(44_100, Channels::Stereo, noise(1, 40_000));
        let at_22 = pcm(22_050, Channels::Mono, noise(2, 9_000));
        let at_48 = pcm(48_000, Channels::Stereo, noise(3, 20_000));
        let looped = mixer.add_sound(at_22).unwrap_or_else(|e| panic!("{e}"));
        let whole = mixer.add_sound(at_44).unwrap_or_else(|e| panic!("{e}"));
        Self {
            mixer,
            looped,
            whole,
            at_48: Some(at_48),
            voices: Vec::new(),
            stage: 0,
            left: 0,
        }
    }

    fn advance(&mut self) -> Option<usize> {
        let stage = self.stage;
        self.stage += 1;
        let mixer = &mut self.mixer;
        let full = |e| panic!("{e}");
        Some(match stage {
            0 => {
                let a = mixer
                    .play(self.whole, Params::new().with_volume(UNITY / 2))
                    .unwrap_or_else(full);
                let b = mixer
                    .play(
                        self.looped,
                        Params::new()
                            .looped()
                            .with_rate(RATE_ONE * 3 / 2)
                            .with_balance(UNITY, UNITY / 4),
                    )
                    .unwrap_or_else(full);
                self.voices.extend([a, b]);
                1000
            }
            1 => {
                mixer.fade_to(self.voices[0], 0, 100);
                let region = Region::new(5_000, 15_000).unwrap_or_else(|| panic!("frames"));
                let at_48 = self.at_48.take().unwrap_or_else(|| panic!("once"));
                let c = mixer
                    .stream(
                        SharedSource::new(at_48),
                        Params::new().with_region(region).with_volume(UNITY / 3),
                    )
                    .unwrap_or_else(full);
                self.voices.push(c);
                2777
            }
            2 => {
                mixer.pause(self.voices[1]);
                1234
            }
            3 => {
                mixer.resume(self.voices[1]);
                mixer.set_balance(self.voices[2], UNITY / 2, UNITY);
                mixer
                    .play(self.looped, Params::new().with_volume(u16::MAX))
                    .unwrap_or_else(full);
                4801
            }
            4 => {
                mixer.stop(self.voices[0]);
                mixer.fade_to(self.voices[2], UNITY, 37);
                mixer.set_volume(self.voices[1], 1);
                20_001
            }
            5 => {
                mixer.stop(self.voices[1]);
                3
            }
            _ => return None,
        })
    }

    /// Fills `out` with what comes next, wherever its stages fall in it. Past the end
    /// of the script it is silence, and says how many frames it had.
    fn render(&mut self, out: &mut [i16]) -> usize {
        let mut done = 0;
        while done < out.len() {
            if self.left == 0 {
                let Some(frames) = self.advance() else {
                    out[done..].fill(0);
                    return done / 2;
                };
                self.left = frames;
            }
            let frames = self.left.min((out.len() - done) / 2);
            self.mixer.render(&mut out[done..done + frames * 2]);
            self.left -= frames;
            done += frames * 2;
        }
        done / 2
    }
}

#[test]
fn a_second_of_the_null_output_is_the_mixers_golden_render() {
    let (blocks, heard) = mpsc::channel::<Vec<i16>>();
    let mut script = Script::new();
    // A size that lines up with nothing of the script's.
    let config = OutputConfig::new(48_000, Channels::Stereo, 480);
    let output = Output::null(config, move |out: &mut [i16]| {
        let frames = script.render(out);
        let _ = blocks.send(out[..frames * 2].to_vec());
    });
    assert_eq!(output.rate(), 48_000);
    let mut samples = Vec::new();
    let mut seconds = 0;
    while samples.len() < 48_000 * 2 {
        // Past the script's end its blocks are empty: a second has gone by then.
        let block = heard
            .recv_timeout(Duration::from_secs(10))
            .unwrap_or_else(|e| panic!("the output stopped asking: {e}"));
        if block.is_empty() {
            seconds += 1;
            if seconds > 20 {
                break;
            }
            continue;
        }
        samples.extend(block);
    }
    drop(output);
    assert_eq!(samples.len(), 29_816 * 2, "the script's whole length");
    let hash = hash(&samples);
    let row = GOLDEN
        .lines()
        .find(|line| line.contains("script: stereo output"))
        .unwrap_or_else(|| panic!("the table has no such row"));
    assert!(row.contains(&hash), "{hash} isn't {row}");
}
