//! The mixer's rings between two real threads: nothing lost, repeated or out of
//! order, under as much contention as a plain test can make.

// WASI preview 1 has no threads, so these are native only.
#![cfg(not(target_os = "wasi"))]

use std::thread;

use shuyi_sound::mix::stream::feed;
use shuyi_sound::mix::{Config, Mixer, Params, UNITY};
use shuyi_sound::{Channels, Limits, Pcm, PcmFormat, SharedSource, Source};

/// How many commands and frames go across.
const COUNT: i32 = 200_000;

fn format(channels: Channels) -> PcmFormat {
    PcmFormat::new(48_000, u32::from(channels.count()), &Limits::default())
        .unwrap_or_else(|e| panic!("{e}"))
}

#[test]
fn every_frame_a_worker_writes_arrives_in_order() {
    let (mut feeder, mut feed) = feed(format(Channels::Stereo), 64);
    let worker = thread::spawn(move || {
        let mut next = 0;
        while next < COUNT {
            let mut samples = Vec::new();
            for n in next..(next + 37).min(COUNT) {
                samples.extend([low(n), high(n)]);
            }
            let wrote = feeder.write(&samples);
            next += i32::try_from(wrote).unwrap_or(0);
            if wrote == 0 {
                thread::yield_now();
            }
        }
        feeder.close();
    });
    let mut out = [0; 58];
    let mut expected = 0;
    loop {
        match feed.read(&mut out) {
            Ok(0) => break,
            Ok(frames) => {
                for frame in out[..frames * 2].chunks(2) {
                    assert_eq!(frame, [low(expected), high(expected)]);
                    expected += 1;
                }
            }
            Err(shuyi_sound::Error::Underrun) => thread::yield_now(),
            Err(e) => panic!("{e}"),
        }
    }
    assert_eq!(expected, COUNT);
    assert!(worker.join().is_ok());
}

fn low(n: i32) -> i16 {
    i16::try_from(n % 30_000).unwrap_or(0)
}

fn high(n: i32) -> i16 {
    i16::try_from(n / 30_000).unwrap_or(0)
}

#[test]
fn every_command_arrives_in_order_and_every_ending_comes_back() {
    let mut mixer = Mixer::new(Config::new(48_000, Channels::Stereo, 8));
    let one = Pcm::new(format(Channels::Mono), vec![1], &Limits::default());
    let one = one.unwrap_or_else(|e| panic!("{e}"));
    let sound = mixer
        .add_sound(one.into())
        .unwrap_or_else(|e| panic!("{e}"));
    let (mut remote, mut inbox) = mixer.remote(8);

    let game = thread::spawn(move || {
        let mut played = 0u32;
        while played < 50_000 {
            match remote.play(sound, Params::new().with_volume(UNITY)) {
                Ok(voice) => {
                    played += 1;
                    // What the audio thread hears of it may come at any moment.
                    let _ = remote.is_playing(voice);
                }
                Err(_) => thread::yield_now(),
            }
        }
        // Every voice comes free in the end, as the mixer reports each.
        while remote.play(sound, Params::new()).is_err() {
            thread::yield_now();
        }
        remote
    });

    let mut heard = 0u64;
    let mut out = [0; 2];
    while !game.is_finished() {
        inbox.apply(&mut mixer);
        mixer.render(&mut out);
        heard += u64::from(out[0].unsigned_abs());
    }
    let remote = game.join();
    assert!(remote.is_ok());
    // Each play is one frame of 1 at full volume, however many came at once.
    for _ in 0..4 {
        inbox.apply(&mut mixer);
        mixer.render(&mut out);
        heard += u64::from(out[0].unsigned_abs());
    }
    assert_eq!(heard, 50_001);
    assert_eq!(mixer.playing(), 0);
}

#[test]
fn a_mixer_moves_to_another_thread_with_its_streams() {
    let pcm = Pcm::new(format(Channels::Mono), vec![5; 300], &Limits::default());
    let pcm = pcm.unwrap_or_else(|e| panic!("{e}"));
    let mut mixer = Mixer::new(Config::new(48_000, Channels::Mono, 2));
    let stream = mixer
        .add_stream(Box::new(SharedSource::new(pcm.into())))
        .unwrap_or_else(|e| panic!("{e}"));
    let (mut remote, mut inbox) = mixer.remote(4);
    let voice = remote
        .play_stream(stream, Params::new())
        .unwrap_or_else(|e| panic!("{e}"));
    let audio = thread::spawn(move || {
        inbox.apply(&mut mixer);
        let mut out = [0; 300];
        mixer.render(&mut out);
        assert_eq!(out, [5; 300]);
        mixer.render(&mut out);
        inbox.apply(&mut mixer);
        assert!(
            mixer
                .take_finished_stream()
                .is_some_and(|source| { source.total_frames() == Some(300) })
        );
    });
    assert!(audio.join().is_ok());
    assert!(!remote.is_playing(voice));
}
