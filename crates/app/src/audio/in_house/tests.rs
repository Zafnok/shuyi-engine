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
            framing: false,
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

/// The audio thread may run between two plays of a frame: what the frame starts
/// reaches it only when the frame closes, all at once (ticket 3089).
#[test]
fn a_frames_plays_reach_the_mix_together_when_it_closes() {
    let (mut backend, mut side) = backend();
    let clip = effect(&mut backend, &wav(100));
    backend.play(&clip, 1.0, false);
    left(&mut side, 1);
    backend.frame(true);
    let a = backend.play(&clip, 1.0, true);
    assert_eq!(left(&mut side, 1), [2], "the first play goes on alone");
    let b = backend.play(&clip, 1.0, true);
    assert!(a.is_some() && b.is_some());
    assert_eq!(left(&mut side, 1), [3]);
    backend.frame(false);
    assert_eq!(
        left(&mut side, 1),
        [4 + 1 + 1],
        "both copies from their first frame"
    );
    backend.stop_playing(a);
    assert_eq!(left(&mut side, 1), [5 + 2], "out of a frame, sent at once");
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

/// `frames` frames as [`wav`]'s, made as samples rather than read from a file.
fn samples(frames: i16) -> SharedSamples {
    let samples = (1..=frames).flat_map(|frame| [frame, -frame]).collect();
    let samples = Samples::new(RATE, 2, samples);
    SharedSamples::new(samples.unwrap_or_else(|e| panic!("{e}")))
}

/// What `future` gives: every load here is done when it returns.
fn block_on<F: std::future::Future>(future: F) -> F::Output {
    let mut future = std::pin::pin!(future);
    let mut cx = std::task::Context::from_waker(std::task::Waker::noop());
    loop {
        if let std::task::Poll::Ready(value) = future.as_mut().poll(&mut cx) {
            return value;
        }
    }
}

#[test]
fn an_added_sound_mixes_as_the_same_samples_as_a_cue_of_the_manifest() {
    use shuyi_assets::audio::{AudioManifest, CreditRef, Files, SoundCue};
    use shuyi_platform::audio::SoundRequest;
    use shuyi_platform::audio::player::Player;
    let bytes = wav(50);
    let mut manifest = AudioManifest::default();
    let cue = SoundCue::new(vec!["filed.wav".to_owned()], 100, CreditRef::Own);
    manifest.sounds.insert("filed".to_owned(), cue);
    let read = |file: &str| (file == "filed.wav").then(|| std::borrow::Cow::Owned(bytes.clone()));
    let files = Files::new(
        &read,
        &|file: &str| file.to_owned(),
        &|_: &str| false,
        "music",
    );
    let play = |cue: &str| SoundRequest::sound(cue, 0.5);
    let mut heard = Vec::new();
    for requests in [
        vec![play("filed")],
        vec![SoundRequest::add("made", samples(50)), play("made")],
    ] {
        let (mut backend, mut side) = backend();
        let (music, voice) = ("music".to_owned(), "voice".to_owned());
        let load = Player::load(&mut backend, manifest.clone(), &files, music, voice);
        let mut player = block_on(load);
        player.play(&mut backend, &requests, &[], 0.0);
        let found = player.take_warnings();
        assert!(found.is_empty(), "{found:?}");
        let mut out = vec![0; 60 * 2];
        side.fill(&mut out);
        heard.push(out);
        assert!(backend.said.is_empty(), "{:?}", backend.said);
    }
    assert_eq!(heard[0], heard[1]);
    // At half volume, rounded to the nearest, both channels, then silence after
    // its 50 frames.
    assert_eq!(heard[0][..4], [1, 0, 1, -1]);
    assert_eq!(heard[0][98..100], [25, -25]);
    assert_eq!(heard[0][100..], [0; 20]);
}

#[test]
fn a_sound_removed_stops_and_gives_its_place_back() {
    let (mut backend, mut side) = backend();
    // The mixer moves to the callback with the first play.
    let first = effect(&mut backend, &wav(1));
    backend.play(&first, 1.0, false);
    left(&mut side, 2);
    let clip = backend
        .add_sound(&samples(100))
        .unwrap_or_else(|e| panic!("{e}"));
    backend.play(&clip, 1.0, false);
    assert_eq!(left(&mut side, 2), [1, 2]);
    backend.remove_sound(clip);
    assert_eq!(left(&mut side, 2), [0, 0]);
    let engine = backend.engine();
    engine.collect();
    assert_eq!(engine.away, 0, "the bank's sound came back to be freed");
    // Its place in the bank is free for the next.
    let again = backend
        .add_sound(&samples(3))
        .unwrap_or_else(|e| panic!("{e}"));
    backend.play(&again, 1.0, false);
    assert_eq!(left(&mut side, 4), [1, 2, 3, 0]);
    assert!(backend.said.is_empty(), "{:?}", backend.said);
}

#[test]
fn samples_written_as_a_wav_read_back_the_same() {
    // Samples a game made, written as a WAV for a device that reads files
    // (quad-snd's path), read back by the WAV adapter as the same samples.
    let every: Vec<i16> = (i16::MIN..=i16::MAX).step_by(97).collect();
    for (rate, channels) in [(1, 1), (8_000, 2), (11_025, 1), (44_100, 2), (192_000, 1)] {
        for written in [Vec::new(), vec![i16::MIN, i16::MAX], every.clone()] {
            let Ok(made) = Samples::new(rate, channels, written.clone()) else {
                continue;
            };
            let limits = Limits::default();
            let back = WavSource::new(made.to_wav().as_slice(), &limits)
                .and_then(|mut source| read_all(&mut source, &limits))
                .unwrap_or_else(|e| panic!("{e}"));
            assert_eq!(back.samples(), written.as_slice());
            assert_eq!(back.format().rate, rate);
            assert_eq!(u32::from(back.format().channels.count()), channels);
            let pcm = pcm_of(&made).unwrap_or_else(|e| panic!("{e}"));
            assert_eq!(*pcm, back);
        }
    }
}

/// The parity gates' file with a region: a second at 440 Hz, a second at 880 Hz, and
/// `LOOPSTART` and `LOOPLENGTH` naming the second second.
const INTRO_LOOP: &[u8] = include_bytes!("../../../../../tools/sound-parity/corpus/intro_loop.ogg");

/// `bytes` decoded whole, as the mixer would hear them played once.
fn decoded(bytes: &[u8]) -> Vec<i16> {
    let limits = Limits::default();
    let pcm = VorbisSource::new(bytes, &limits).and_then(|mut s| read_all(&mut s, &limits));
    pcm.unwrap_or_else(|e| panic!("{e}")).samples().to_vec()
}

/// The first `samples` samples of `clip`'s looped play, read from its source in
/// chunks of an odd size, as a worker would.
fn looped_samples(clip: &Clip, samples: usize) -> Vec<i16> {
    let Clip::Stream { track, region, .. } = clip else {
        panic!("a track");
    };
    // As a looped play has it: the whole track where there is no region.
    let region = region.get().or(Region::new(0, u64::MAX));
    let mut source = track_source(track, region).unwrap_or_else(|e| panic!("{e}"));
    let mut out = Vec::new();
    let mut chunk = [0; 1234];
    while out.len() < samples {
        let read = source.read(&mut chunk).unwrap_or_else(|e| panic!("{e}"));
        assert!(read > 0, "a looped track doesn't end");
        out.extend_from_slice(&chunk[..read * 2]);
    }
    out.truncate(samples);
    out
}

/// What the intro once, then `region` (in stereo frames) twice and a half, are.
fn intro_then_round(whole: &[i16], start: usize, end: usize) -> Vec<i16> {
    let round = &whole[start * 2..end * 2];
    let mut expected = whole[..end * 2].to_vec();
    expected.extend_from_slice(round);
    expected.extend_from_slice(round);
    expected.extend_from_slice(&round[..round.len() / 2]);
    expected
}

/// Ticket 2397: the file's own region, from its tags, exact at every way round.
#[test]
fn an_ogg_files_own_region_is_gone_round_exactly() {
    let (mut backend, _side) = backend();
    let clip = stream(INTRO_LOOP.to_vec());
    let looping = backend.set_loop_region(&clip, None);
    assert_eq!(
        looping,
        Ok(Looping::Region {
            start: 1.0,
            end: 2.0
        })
    );
    let whole = decoded(INTRO_LOOP);
    let expected = intro_then_round(&whole, 44_100, 88_200);
    assert!(looped_samples(&clip, expected.len()) == expected);
}

/// A cue's region wins, in the file's frames, both near the start (read again from
/// the first frame) and far in (an exact seek).
#[test]
fn a_cues_region_wins_over_the_files_in_its_frames() {
    let (mut backend, _side) = backend();
    let whole = decoded(INTRO_LOOP);
    for (start_ms, end_ms, start, end) in [(500, 1500, 22_050, 66_150), (100, 1999, 4410, 88_155)] {
        let clip = stream(INTRO_LOOP.to_vec());
        let asked = LoopRegion::new(start_ms, end_ms);
        let looping = backend.set_loop_region(&clip, asked);
        let secs = |frame: u32| f64::from(frame) / 44_100.0;
        let said = Looping::Region {
            start: secs(start),
            end: secs(end),
        };
        assert_eq!(looping, Ok(said), "{start_ms}");
        let (start, end) = (start as usize, end as usize);
        let expected = intro_then_round(&whole, start, end);
        assert!(
            looped_samples(&clip, expected.len()) == expected,
            "{start_ms}"
        );
    }
}

#[test]
fn a_track_with_no_region_or_tags_that_do_not_read_loops_whole() {
    let (mut backend, _side) = backend();
    let plain = stream(wav(3));
    assert_eq!(backend.set_loop_region(&plain, None), Ok(Looping::Whole));
    let mut track = Track::new(INTRO_LOOP.to_vec()).unwrap_or_else(|e| panic!("{e}"));
    track.file_region = Err("LOOPLENGTH is 0: a region has a frame at least".to_owned());
    let unread = clip_of(track);
    let looping = backend.set_loop_region(&unread, None);
    let why = "LOOPLENGTH is 0: a region has a frame at least".to_owned();
    assert_eq!(looping, Ok(Looping::Unread(why)));
    let whole = decoded(INTRO_LOOP);
    let twice = [whole.as_slice(), whole.as_slice()].concat();
    assert!(looped_samples(&unread, twice.len()) == twice);
    // A sound of the bank has nothing to go round but itself.
    let effect = effect(&mut backend, &wav(3));
    let asked = LoopRegion::new(0, 1);
    assert_eq!(backend.set_loop_region(&effect, asked), Ok(Looping::Whole));
}

/// Through the worker and the mix: a cue's region of a WAV track, exact at the mixer's
/// own rate, and a play once ignores it.
#[test]
fn a_tracks_region_is_heard_through_the_mix_and_a_play_once_plays_through() {
    let (mut backend, mut side) = backend();
    let clip = stream(wav(200));
    // 1 ms to 3 ms at 48 kHz: frames 48 to 144, which are numbered 49 to 144.
    let looping = backend.set_loop_region(&clip, LoopRegion::new(1, 3));
    assert_eq!(
        looping,
        Ok(Looping::Region {
            start: 0.001,
            end: 0.003
        })
    );
    backend.play(&clip, 1.0, true);
    let mut heard = Vec::new();
    until(&mut side, "the region never came round", |side| {
        heard.extend(left(side, 16).into_iter().filter(|sample| *sample != 0));
        heard.len() >= 500
    });
    let expected = (1..=144).chain((0..).flat_map(|_| 49..=144));
    for (index, (sample, expected)) in heard.iter().zip(expected).enumerate() {
        assert_eq!(*sample, expected, "{index}");
    }
    backend.play(&clip, 1.0, false);
    let once = left(&mut side, 210);
    assert_eq!(once[..200], (1..=200).collect::<Vec<i16>>()[..]);
}

/// Far into a long file, the way round is a seek that lands on the frame: what
/// comes out is still exactly the whole decode's.
#[test]
fn a_region_far_into_a_long_file_is_gone_round_by_a_seek_exactly() {
    const LONG: &[u8] = include_bytes!("../../../../../tools/sound-parity/corpus/long.ogg");
    let (mut backend, _side) = backend();
    let clip = stream(LONG.to_vec());
    let looping = backend.set_loop_region(&clip, LoopRegion::new(5000, 6000));
    assert_eq!(
        looping,
        Ok(Looping::Region {
            start: 5.0,
            end: 6.0
        })
    );
    let whole = decoded(LONG);
    let expected = intro_then_round(&whole, 220_500, 264_600);
    assert!(looped_samples(&clip, expected.len()) == expected);
}
