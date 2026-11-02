//! The backend with no device: the test thread calls the callback's side itself, so
//! that what comes out is exact, and it runs on any machine.

use std::time::{Duration, Instant};

use shuyi_platform::audio::player::RATE_ONE;

use super::*;

const RATE: u32 = 48_000;

/// A stereo 16-bit WAV at `RATE` of `frames` frames, left counting up from 1, right
/// counting down from -1.
fn wav(frames: i16) -> Vec<u8> {
    let bytes = u32::try_from(frames).unwrap_or(0) * 4;
    let mut w = Vec::new();
    w.extend_from_slice(b"RIFF");
    w.extend_from_slice(&(36 + bytes).to_le_bytes());
    w.extend_from_slice(b"WAVEfmt ");
    w.extend_from_slice(&16u32.to_le_bytes());
    w.extend_from_slice(&1u16.to_le_bytes());
    w.extend_from_slice(&2u16.to_le_bytes());
    w.extend_from_slice(&RATE.to_le_bytes());
    w.extend_from_slice(&(RATE * 4).to_le_bytes());
    w.extend_from_slice(&4u16.to_le_bytes());
    w.extend_from_slice(&16u16.to_le_bytes());
    w.extend_from_slice(b"data");
    w.extend_from_slice(&bytes.to_le_bytes());
    for frame in 1..=frames {
        w.extend_from_slice(&frame.to_le_bytes());
        w.extend_from_slice(&(-frame).to_le_bytes());
    }
    w
}

/// The backend, its engine made here with no output, and the callback's side.
fn backend() -> (InHouse, AudioSide) {
    let (engine, side) = sides(RATE);
    (
        InHouse {
            engine: Some(engine),
            said: Vec::new(),
        },
        side,
    )
}

/// `frames` frames from the callback's side, left channel.
fn left(side: &mut AudioSide, frames: usize) -> Vec<i16> {
    let mut out = vec![0; frames * 2];
    side.fill(&mut out);
    out.iter().step_by(2).copied().collect()
}

fn effect(backend: &mut InHouse, bytes: &[u8]) -> Clip {
    backend.load_effect(bytes).unwrap_or_else(|e| panic!("{e}"))
}

/// A track of `bytes`, as `poll_music` makes one.
fn stream(bytes: Vec<u8>) -> Clip {
    clip_of(Track::new(bytes).unwrap_or_else(|e| panic!("{e}")))
}

/// Gives the worker time to do what `done` looks for, while the mix runs.
fn until(side: &mut AudioSide, what: &str, mut done: impl FnMut(&mut AudioSide) -> bool) {
    let start = Instant::now();
    while !done(side) {
        assert!(start.elapsed() < Duration::from_secs(20), "{what}");
        std::thread::sleep(Duration::from_millis(1));
    }
}

#[test]
fn a_sound_loaded_before_the_first_play_comes_out_exact() {
    let (mut backend, mut side) = backend();
    let clip = effect(&mut backend, &wav(3));
    assert_eq!(left(&mut side, 2), [0, 0], "nothing asked yet");
    let playing = backend.play(&clip, 1.0, false);
    assert!(playing.is_some());
    assert_eq!(left(&mut side, 5), [1, 2, 3, 0, 0]);
}

#[test]
fn a_sound_loaded_after_the_mixer_moved_plays_at_once() {
    let (mut backend, mut side) = backend();
    let first = effect(&mut backend, &wav(1));
    backend.play(&first, 1.0, false);
    left(&mut side, 4);
    // The hand-over and the play go together: the play finds its sound.
    let second = effect(&mut backend, &wav(3));
    backend.play(&second, 1.0, false);
    assert_eq!(left(&mut side, 4), [1, 2, 3, 0]);
}

#[test]
fn copies_stop_alone_and_volumes_reach_one() {
    let (mut backend, mut side) = backend();
    let clip = effect(&mut backend, &wav(100));
    let a = backend.play(&clip, 1.0, true);
    let b = backend.play(&clip, 1.0, true);
    assert_eq!(left(&mut side, 2), [2, 4]);
    backend.stop_playing(a);
    assert_eq!(left(&mut side, 1), [3]);
    backend.set_playing_volume(&b, 0.0);
    // After its short ramp it is gone, still playing.
    left(&mut side, 48_000 / 100);
    assert_eq!(left(&mut side, 1), [0]);
    assert!(
        backend
            .engine()
            .remote
            .is_playing(b.unwrap_or_else(|| panic!("played")))
    );
}

#[test]
fn a_pause_holds_where_it_is_and_a_resume_goes_on_from_there() {
    let (mut backend, mut side) = backend();
    let clip = effect(&mut backend, &wav(9));
    let playing = backend.play(&clip, 1.0, false);
    assert_eq!(left(&mut side, 2), [1, 2]);
    assert_eq!(backend.pause(&playing), Ok(()));
    assert_eq!(left(&mut side, 3), [0, 0, 0]);
    assert_eq!(backend.resume(&playing), Ok(()));
    assert_eq!(left(&mut side, 2), [3, 4]);
}

#[test]
fn a_rate_goes_with_the_sounds_next_play() {
    let (mut backend, mut side) = backend();
    let clip = effect(&mut backend, &wav(9));
    assert_eq!(backend.set_rate(&clip, RATE_ONE * 2), Ok(()));
    backend.play(&clip, 1.0, false);
    assert_eq!(left(&mut side, 3), [1, 3, 5]);
}

#[test]
fn with_every_voice_busy_a_sound_is_skipped_and_said_and_what_plays_goes_on() {
    let (mut backend, mut side) = backend();
    let clip = effect(&mut backend, &wav(100));
    let voices: Vec<_> = (0..VOICES)
        .map(|_| backend.play(&clip, 1.0, true))
        .collect();
    assert!(voices.iter().all(Option::is_some));
    assert_eq!(backend.play(&clip, 1.0, true), None);
    assert_eq!(backend.said, ["no voice is free: a sound was skipped"]);
    let all = i16::try_from(VOICES).unwrap_or(0);
    assert_eq!(left(&mut side, 2), [all, all * 2], "no voice was stolen");
}

#[test]
fn a_file_that_is_neither_format_is_an_error_not_a_crash() {
    let (mut backend, _side) = backend();
    let refused = backend.load_effect(b"not a sound");
    assert_eq!(
        refused.err().as_deref(),
        Some("not a readable OGG or WAV file")
    );
    let refused = backend.load_effect(b"RIFF too short");
    assert!(refused.is_err());
    assert!(Track::new(b"OggS is all it has".to_vec()).is_err());
}

#[test]
fn a_track_streams_from_the_ring_with_its_first_frames_ready() {
    let (mut backend, mut side) = backend();
    let clip = stream(wav(4));
    backend.play(&clip, 1.0, false);
    // Decoded before the play was sent: nothing is late.
    assert_eq!(left(&mut side, 6), [1, 2, 3, 4, 0, 0]);
    assert_eq!(backend.length(&clip), None, "only an OGG file says");
}

#[test]
fn a_looped_track_comes_round_on_the_exact_frame() {
    let (mut backend, mut side) = backend();
    let clip = stream(wav(3));
    backend.play(&clip, 1.0, true);
    let mut heard = Vec::new();
    until(&mut side, "the loop never came round", |side| {
        heard.extend(left(side, 16).into_iter().filter(|sample| *sample != 0));
        heard.len() >= 300
    });
    for (index, sample) in heard.iter().enumerate() {
        assert_eq!(
            *sample,
            i16::try_from(index % 3 + 1).unwrap_or(0),
            "{index}"
        );
    }
}

#[test]
fn a_stopped_track_gives_its_stream_back_and_a_new_one_can_start() {
    let (mut backend, mut side) = backend();
    // More starts than there are slots: each comes back for the next.
    for _ in 0..STREAMS * 3 {
        let clip = stream(wav(50));
        assert!(
            backend.play(&clip, 1.0, true).is_some(),
            "{:?}",
            backend.said
        );
        left(&mut side, 1);
        backend.stop(&clip);
        until(&mut side, "the stream never came back", |side| {
            left(side, 1);
            let engine = backend.engine.as_mut().unwrap_or_else(|| panic!("made"));
            engine.collect();
            engine.sweep();
            engine.collect();
            engine.live.is_empty() && engine.away == 0
        });
    }
    assert!(backend.said.is_empty(), "{:?}", backend.said);
}

#[test]
fn playing_a_track_again_starts_it_over_alone() {
    let (mut backend, mut side) = backend();
    let clip = stream(wav(100));
    backend.play(&clip, 1.0, false);
    assert_eq!(left(&mut side, 2), [1, 2]);
    backend.play(&clip, 1.0, false);
    assert_eq!(left(&mut side, 2), [1, 2], "the old one stopped first");
}

#[test]
fn volumes_become_the_mixers_units_held_to_what_a_voice_can_take() {
    assert_eq!(level(0.0), 0);
    assert_eq!(level(1.0), UNITY);
    assert_eq!(level(0.5), UNITY / 2);
    assert_eq!(level(-1.0), 0);
    assert_eq!(level(4.0), UNITY);
    assert_eq!(level(f32::NAN), 0);
}
