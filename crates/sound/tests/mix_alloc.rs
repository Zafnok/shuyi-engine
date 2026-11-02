//! `render` allocates nothing and frees nothing, whatever it plays, and a stream
//! costs the same memory however long it is. One test: the counter is the whole
//! process's, so nothing runs beside it.

use shuyi_alloc_count::{global, tags};
use shuyi_sound::mix::stream::{LoopedRegion, feed};
use shuyi_sound::mix::{Config, Mixer, Params, RATE_ONE, Region, SharedPcm, UNITY};
use shuyi_sound::{Channels, Limits, Pcm, PcmFormat, SharedSource};

/// How many frames of a stream the ring holds.
const RING: usize = 4096;
/// A worker's visit, and a render, in frames.
const BLOCK: usize = 512;

fn format(channels: Channels) -> PcmFormat {
    PcmFormat::new(44_100, u32::from(channels.count()), &Limits::default())
        .unwrap_or_else(|e| panic!("{e}"))
}

/// `frames` frames of stereo that aren't silence.
fn sound(frames: usize) -> SharedPcm {
    let samples = (0..frames * 2)
        .map(|n| i16::try_from(n % 20_000).unwrap_or(0))
        .collect();
    let pcm = Pcm::new(format(Channels::Stereo), samples, &Limits::default());
    SharedPcm::new(pcm.unwrap_or_else(|e| panic!("{e}")))
}

/// The most live bytes while a stream of `frames` frames plays through a worker's
/// ring to its end, above what was live when it started. What it plays is made
/// before the count starts, as a decoder holds its file and not its samples.
fn peak_of_a_stream(frames: usize, name: &'static str) -> u64 {
    let whole = sound(frames);
    let mut source = SharedSource::new(whole);
    let tag = tags::tag(2, name);
    let outside = tags::enter(tag);
    let mut mixer = Mixer::new(Config::new(48_000, Channels::Stereo, 2));
    let (mut feeder, ring) = feed(format(Channels::Stereo), RING);
    let voice = mixer
        .stream(ring, Params::new())
        .unwrap_or_else(|e| panic!("{e}"));
    let mut out = vec![0; BLOCK * 2];
    while mixer.is_playing(voice) {
        feeder
            .fill_from(&mut source)
            .unwrap_or_else(|e| panic!("{e}"));
        mixer.render(&mut out);
    }
    assert_eq!(mixer.underrun_frames(), 0);
    drop((mixer, feeder, out));
    tags::leave(outside);
    global::rows()
        .and_then(|rows| rows.into_iter().find(|row| row.tag == tag))
        .map_or(0, |row| row.peak_bytes)
}

/// The heap's counts: only a build that counts runs this test.
fn counts() -> shuyi_alloc_count::Counts {
    global::reading().unwrap_or_else(|| panic!("this build doesn't count"))
}

#[cfg_attr(not(debug_assertions), ignore = "only a debug build counts the heap")]
#[test]
fn render_allocates_nothing_and_a_stream_costs_the_same_however_long() {
    // Everything playing that can be: sounds at other rates, looped, faded and
    // paused; a stream from a worker with a region; one from a shared sound.
    let mut mixer = Mixer::new(Config::new(48_000, Channels::Stereo, 8));
    let long = sound(44_100);
    let bank = mixer
        .add_sound(SharedPcm::clone(&long))
        .unwrap_or_else(|e| panic!("{e}"));
    let short = mixer.add_sound(sound(10)).unwrap_or_else(|e| panic!("{e}"));
    let looped = mixer
        .play(bank, Params::new().looped().with_rate(RATE_ONE * 3 / 2))
        .unwrap_or_else(|e| panic!("{e}"));
    mixer.fade_to(looped, 0, 500);
    let paused = mixer
        .play(bank, Params::new().looped())
        .unwrap_or_else(|e| panic!("{e}"));
    mixer.pause(paused);
    let region = Region::new(1000, 2000).unwrap_or_else(|| panic!("frames"));
    let restart_from = SharedPcm::clone(&long);
    let mut source = LoopedRegion::new(
        SharedSource::new(SharedPcm::clone(&long)),
        region,
        move || Ok(SharedSource::new(SharedPcm::clone(&restart_from))),
    );
    let (mut feeder, ring) = feed(format(Channels::Stereo), RING);
    mixer
        .stream(ring, Params::new())
        .unwrap_or_else(|e| panic!("{e}"));
    let ended = mixer
        .stream(SharedSource::new(sound(100)), Params::new())
        .unwrap_or_else(|e| panic!("{e}"));
    for _ in 0..3 {
        mixer
            .play(short, Params::new().with_balance(UNITY, 0))
            .unwrap_or_else(|e| panic!("{e}"));
    }
    let (remote, mut inbox) = mixer.remote(16);
    let mut out = vec![0; BLOCK * 2];

    // A worker's `fill_from` can restart a decoder, which may allocate: it isn't
    // part of what is counted, as it isn't on the audio thread.
    for _ in 0..200 {
        feeder
            .fill_from(&mut source)
            .unwrap_or_else(|e| panic!("{e}"));
        assert!(remote.set_volume(paused, UNITY).is_ok());
        let before = counts();
        inbox.apply(&mut mixer);
        mixer.render(&mut out);
        let after = counts();
        assert_eq!(after.allocs, 0, "a render allocated");
        assert_eq!(
            after.live_bytes, before.live_bytes,
            "a render allocated or freed"
        );
    }
    assert!(
        !mixer.is_playing(ended),
        "the stream that ended is held, not freed"
    );
    assert_eq!(mixer.underrun_frames(), 0);
    assert!(out.iter().any(|sample| *sample != 0));
    drop((mixer, feeder, source, remote, inbox));

    // Ten seconds and a minute hold the same.
    let (shorter, longer) = (
        peak_of_a_stream(441_000, "ten seconds"),
        peak_of_a_stream(2_646_000, "a minute"),
    );
    assert_eq!(shorter, longer);
    assert!(shorter > 0 && shorter < 1 << 18, "{shorter} bytes");
}
