//! The mixer's golden table: fixed scripts of plays, fades and stops, whose
//! rendered samples must hash the same on every target (`shuyi_assets::golden`).

use std::fmt::Write as _;

use shuyi_sound::mix::{Config, Mixer, Params, RATE_ONE, Region, SharedPcm, UNITY};
use shuyi_sound::{Channels, Limits, Pcm, PcmFormat, SharedSource};

/// `samples` as a hash, FNV-1a over their bytes: the table's value.
fn hash(samples: &[i16]) -> String {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for byte in samples.iter().flat_map(|sample| sample.to_le_bytes()) {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    let mut text = String::new();
    let _ = write!(text, "{hash:016x}");
    text
}

/// A little generator of its own: the same numbers everywhere.
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

/// The script: every part of the mixer, in blocks of a size that lines up with
/// nothing.
fn script(channels: Channels) -> Vec<i16> {
    let per_frame = usize::from(channels.count());
    let mut mixer = Mixer::new(Config::new(48_000, channels, 8));
    let at_44 = pcm(44_100, Channels::Stereo, noise(1, 40_000));
    let at_22 = pcm(22_050, Channels::Mono, noise(2, 9_000));
    let at_48 = pcm(48_000, Channels::Stereo, noise(3, 20_000));
    let looped = mixer
        .add_sound(SharedPcm::clone(&at_22))
        .unwrap_or_else(|e| panic!("{e}"));
    let whole = mixer.add_sound(at_44).unwrap_or_else(|e| panic!("{e}"));
    let mut out = Vec::new();
    let mut render = |mixer: &mut Mixer, frames: usize| {
        let mut block = vec![0; frames * per_frame];
        mixer.render(&mut block);
        out.extend(block);
    };

    let a = mixer
        .play(whole, Params::new().with_volume(UNITY / 2))
        .unwrap_or_else(|e| panic!("{e}"));
    let b = mixer
        .play(
            looped,
            Params::new()
                .looped()
                .with_rate(RATE_ONE * 3 / 2)
                .with_balance(UNITY, UNITY / 4),
        )
        .unwrap_or_else(|e| panic!("{e}"));
    render(&mut mixer, 1000);
    mixer.fade_to(a, 0, 100);
    let region = Region::new(5_000, 15_000).unwrap_or_else(|| panic!("frames"));
    let c = mixer
        .stream(
            SharedSource::new(at_48),
            Params::new().with_region(region).with_volume(UNITY / 3),
        )
        .unwrap_or_else(|e| panic!("{e}"));
    render(&mut mixer, 2777);
    mixer.pause(b);
    render(&mut mixer, 1234);
    mixer.resume(b);
    mixer.set_balance(c, UNITY / 2, UNITY);
    mixer
        .play(looped, Params::new().with_volume(u16::MAX))
        .unwrap_or_else(|e| panic!("{e}"));
    render(&mut mixer, 4801);
    mixer.stop(a);
    mixer.fade_to(c, UNITY, 37);
    mixer.set_volume(b, 1);
    render(&mut mixer, 20_001);
    mixer.stop(b);
    render(&mut mixer, 3);
    out
}

#[test]
fn golden_render() {
    let mut golden = shuyi_assets::golden!("tests/golden/mix.ron");
    golden.check("script: mono output", hash(&script(Channels::Mono)));
    golden.check("script: stereo output", hash(&script(Channels::Stereo)));
    golden.assert();
}
