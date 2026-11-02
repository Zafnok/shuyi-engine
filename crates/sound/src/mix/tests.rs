use alloc::vec::Vec;

use proptest::prelude::*;

use super::stream::{Feeder, LoopedRegion, feed};
use super::*;
use crate::error::Error;
use crate::pcm::{Limits, PcmFormat};
use crate::source::SharedSource;

const RATE: u32 = 48_000;

fn format(rate: u32, channels: Channels) -> PcmFormat {
    PcmFormat::new(rate, u32::from(channels.count()), &Limits::default())
        .unwrap_or_else(|e| panic!("{e}"))
}

fn pcm(rate: u32, channels: Channels, samples: Vec<i16>) -> SharedPcm {
    let pcm = Pcm::new(format(rate, channels), samples, &Limits::default());
    SharedPcm::new(pcm.unwrap_or_else(|e| panic!("{e}")))
}

/// Mono samples from 1 up, at the mixer's rate.
fn ramp(frames: i16) -> SharedPcm {
    pcm(RATE, Channels::Mono, (1..=frames).collect())
}

fn stereo_mixer(voices: usize) -> Mixer {
    Mixer::new(Config::new(RATE, Channels::Stereo, voices))
}

fn mono_mixer(voices: usize) -> Mixer {
    Mixer::new(Config::new(RATE, Channels::Mono, voices))
}

fn play(mixer: &mut Mixer, sound: SharedPcm, params: Params) -> VoiceId {
    let id = mixer.add_sound(sound).unwrap_or_else(|e| panic!("{e}"));
    mixer.play(id, params).unwrap_or_else(|e| panic!("{e}"))
}

fn render(mixer: &mut Mixer, frames: usize) -> Vec<i16> {
    let mut out = vec![1; frames * usize::from(mixer.config().channels.count())];
    mixer.render(&mut out);
    out
}

/// The left samples of stereo `out`.
fn left(out: &[i16]) -> Vec<i16> {
    out.chunks(2).map(|frame| frame[0]).collect()
}

fn right(out: &[i16]) -> Vec<i16> {
    out.chunks(2).map(|frame| frame[1]).collect()
}

#[test]
fn nothing_playing_is_silence() {
    let mut mixer = stereo_mixer(2);
    assert_eq!(render(&mut mixer, 3), [0; 6]);
    assert_eq!(mixer.playing(), 0);
}

#[test]
fn a_sound_plays_exactly_and_ends() {
    let mut mixer = stereo_mixer(1);
    let voice = play(&mut mixer, ramp(3), Params::new());
    assert!(mixer.is_playing(voice));
    assert_eq!(render(&mut mixer, 2), [1, 1, 2, 2]);
    assert!(mixer.is_playing(voice), "it has a frame left");
    assert_eq!(render(&mut mixer, 3), [3, 3, 0, 0, 0, 0]);
    assert!(!mixer.is_playing(voice));
    assert_eq!(mixer.volume(voice), None);
}

#[test]
fn a_part_of_a_frame_at_the_end_is_silence() {
    let mut mixer = stereo_mixer(1);
    play(&mut mixer, ramp(9), Params::new());
    let mut out = [7; 3];
    mixer.render(&mut out);
    assert_eq!(out, [1, 1, 0]);
}

#[test]
fn a_render_longer_than_a_block_is_the_same_as_many_short_ones() {
    let frames = i16::try_from(BLOCK_FRAMES * 3).unwrap_or(0);
    let mut whole = stereo_mixer(1);
    play(&mut whole, ramp(frames), Params::new());
    let mut parts = stereo_mixer(1);
    play(&mut parts, ramp(frames), Params::new());
    let long = render(&mut whole, BLOCK_FRAMES * 3);
    let mut short = Vec::new();
    for _ in 0..BLOCK_FRAMES * 3 {
        short.extend(render(&mut parts, 1));
    }
    assert_eq!(long, short);
    assert_eq!(left(&long)[BLOCK_FRAMES * 3 - 1], frames);
}

#[test]
fn two_sounds_that_overflow_clamp_and_never_wrap() {
    let mut mixer = stereo_mixer(2);
    let high = pcm(RATE, Channels::Mono, vec![30_000, -30_000]);
    play(&mut mixer, SharedPcm::clone(&high), Params::new());
    play(&mut mixer, high, Params::new());
    assert_eq!(render(&mut mixer, 2), [32_767, 32_767, -32_768, -32_768]);
}

#[test]
fn many_voices_at_a_high_volume_saturate_and_never_wrap() {
    let mut mixer = stereo_mixer(1000);
    let high = pcm(RATE, Channels::Mono, vec![32_767]);
    let id = mixer.add_sound(high).unwrap_or_else(|e| panic!("{e}"));
    let loud = Params::new()
        .with_volume(u16::MAX)
        .with_balance(u16::MAX, u16::MAX);
    for _ in 0..1000 {
        mixer.play(id, loud).unwrap_or_else(|e| panic!("{e}"));
    }
    assert_eq!(render(&mut mixer, 1), [32_767, 32_767]);
}

#[test]
fn a_volume_scales_and_rounds_to_the_nearest() {
    let mut mixer = stereo_mixer(1);
    let sound = pcm(RATE, Channels::Mono, vec![1000, -1000, 3, -3, 1, -1]);
    let voice = play(&mut mixer, sound, Params::new().with_volume(UNITY / 2));
    assert_eq!(left(&render(&mut mixer, 6)), [500, -500, 2, -1, 1, 0]);
    assert!(!mixer.is_playing(voice) || mixer.volume(voice) == Some(UNITY / 2));
}

#[test]
fn a_volume_over_unity_amplifies() {
    let mut mixer = stereo_mixer(1);
    let sound = pcm(RATE, Channels::Mono, vec![100, -100, 20_000]);
    play(&mut mixer, sound, Params::new().with_volume(UNITY * 2));
    assert_eq!(left(&render(&mut mixer, 3)), [200, -200, 32_767]);
}

#[test]
fn set_volume_takes_effect_at_the_next_frame_and_ends_a_fade() {
    let mut mixer = stereo_mixer(1);
    let voice = play(
        &mut mixer,
        pcm(RATE, Channels::Mono, vec![1000; 10]),
        Params::new(),
    );
    assert_eq!(left(&render(&mut mixer, 1)), [1000]);
    mixer.set_volume(voice, UNITY / 4);
    assert_eq!(mixer.volume(voice), Some(UNITY / 4));
    assert_eq!(left(&render(&mut mixer, 1)), [250]);
    mixer.fade_to(voice, 0, 1000);
    mixer.set_volume(voice, UNITY);
    assert_eq!(left(&render(&mut mixer, 2)), [1000, 1000]);
}

#[test]
fn a_fade_over_100_ms_ends_at_exactly_nothing_after_4800_frames() {
    let mut mixer = stereo_mixer(1);
    let sound = pcm(RATE, Channels::Mono, vec![32_767; 6000]);
    let voice = play(&mut mixer, sound, Params::new());
    mixer.fade_to(voice, 0, 100);
    let out = left(&render(&mut mixer, 4799));
    assert_eq!(out[0], 32_767, "it starts where it was");
    assert!(
        out.windows(2).all(|pair| pair[1] <= pair[0]),
        "it only falls"
    );
    assert!(out[4798] > 0);
    let last = left(&render(&mut mixer, 1));
    assert!(last[0] > 0, "frame 4799 is the fade's last with sound");
    assert_eq!(mixer.volume(voice), Some(0), "exactly, after 4800 frames");
    assert_eq!(left(&render(&mut mixer, 2)), [0, 0]);
    assert!(mixer.is_playing(voice), "the caller stops it");
}

#[test]
fn a_fade_up_lands_exactly_and_is_a_straight_line() {
    let mut mixer = stereo_mixer(1);
    let sound = pcm(RATE, Channels::Mono, vec![10_000; 100]);
    let voice = play(&mut mixer, sound, Params::new().with_volume(0));
    // 1 ms at 48 kHz.
    mixer.fade_to(voice, UNITY, 1);
    let out = left(&render(&mut mixer, 49));
    assert_eq!(out[0], 0);
    assert_eq!(out[24], 5000);
    assert_eq!(out[48], 10_000);
    assert_eq!(mixer.volume(voice), Some(UNITY));
}

#[test]
fn a_fade_of_no_time_is_a_jump() {
    let mut mixer = stereo_mixer(1);
    let voice = play(&mut mixer, ramp(9), Params::new());
    mixer.fade_to(voice, 7, 0);
    assert_eq!(mixer.volume(voice), Some(7));
}

#[test]
fn looping_a_sound_goes_on_with_no_gap() {
    let mut mixer = stereo_mixer(1);
    let voice = play(&mut mixer, ramp(3), Params::new().looped());
    assert_eq!(left(&render(&mut mixer, 7)), [1, 2, 3, 1, 2, 3, 1]);
    assert!(mixer.is_playing(voice));
}

#[test]
fn a_region_of_a_sound_plays_its_intro_once() {
    let mut mixer = stereo_mixer(1);
    let region = Region::new(2, 4).unwrap_or_else(|| panic!("frames"));
    play(&mut mixer, ramp(5), Params::new().with_region(region));
    assert_eq!(left(&render(&mut mixer, 8)), [1, 2, 3, 4, 3, 4, 3, 4]);
}

#[test]
fn a_region_past_a_sounds_end_stops_at_its_end() {
    let mut mixer = stereo_mixer(1);
    let region = Region::new(1, 99).unwrap_or_else(|| panic!("frames"));
    play(&mut mixer, ramp(3), Params::new().with_region(region));
    assert_eq!(left(&render(&mut mixer, 5)), [1, 2, 3, 2, 3]);
}

#[test]
fn a_region_after_everything_ends_rather_than_spins() {
    let mut mixer = stereo_mixer(1);
    let region = Region::new(5, 9).unwrap_or_else(|| panic!("frames"));
    let voice = play(&mut mixer, ramp(3), Params::new().with_region(region));
    assert_eq!(left(&render(&mut mixer, 4)), [1, 2, 3, 0]);
    assert!(!mixer.is_playing(voice));
}

#[test]
fn a_region_is_some_frames() {
    assert_eq!(Region::new(3, 3), None);
    assert_eq!(Region::new(4, 3), None);
    let region = Region::new(3, 4).unwrap_or_else(|| panic!("frames"));
    assert_eq!((region.start(), region.end()), (3, 4));
}

#[test]
fn a_looped_sound_with_no_frames_ends() {
    let mut mixer = stereo_mixer(1);
    let voice = play(
        &mut mixer,
        pcm(RATE, Channels::Mono, Vec::new()),
        Params::new().looped(),
    );
    assert_eq!(render(&mut mixer, 1), [0, 0]);
    assert!(!mixer.is_playing(voice));
}

#[test]
fn a_voice_paused_for_4800_frames_resumes_at_the_frame_where_it_stopped() {
    let mut mixer = stereo_mixer(1);
    let voice = play(&mut mixer, ramp(10_000), Params::new());
    assert_eq!(left(&render(&mut mixer, 3)), [1, 2, 3]);
    mixer.pause(voice);
    assert!(mixer.is_paused(voice));
    assert!(render(&mut mixer, 4800).iter().all(|s| *s == 0));
    assert!(mixer.is_playing(voice), "held, not over");
    mixer.resume(voice);
    assert!(!mixer.is_paused(voice));
    assert_eq!(left(&render(&mut mixer, 2)), [4, 5]);
}

#[test]
fn a_pause_holds_a_fade_too() {
    let mut mixer = stereo_mixer(1);
    let sound = pcm(RATE, Channels::Mono, vec![1000; 100]);
    let voice = play(&mut mixer, sound, Params::new());
    mixer.fade_to(voice, 0, 1);
    render(&mut mixer, 24);
    let held = mixer.volume(voice);
    mixer.pause(voice);
    render(&mut mixer, 480);
    assert_eq!(mixer.volume(voice), held);
    mixer.resume(voice);
    render(&mut mixer, 24);
    assert_eq!(mixer.volume(voice), Some(0));
}

#[test]
fn a_sound_at_rate_two_ends_in_half_its_frames() {
    let mut mixer = stereo_mixer(1);
    let voice = play(&mut mixer, ramp(100), Params::new().with_rate(RATE_ONE * 2));
    let out = left(&render(&mut mixer, 50));
    assert_eq!(out[..3], [1, 3, 5]);
    assert_eq!(out[49], 99);
    render(&mut mixer, 1);
    assert!(!mixer.is_playing(voice));
}

#[test]
fn a_sound_at_half_rate_lasts_twice_and_interpolates() {
    let mut mixer = stereo_mixer(1);
    let sound = pcm(RATE, Channels::Mono, vec![0, 100, 200]);
    let voice = play(&mut mixer, sound, Params::new().with_rate(RATE_ONE / 2));
    assert_eq!(left(&render(&mut mixer, 5)), [0, 50, 100, 150, 200]);
    assert!(mixer.is_playing(voice), "its last frame's second half");
    assert_eq!(left(&render(&mut mixer, 1)), [200], "the last frame holds");
    render(&mut mixer, 1);
    assert!(!mixer.is_playing(voice));
}

#[test]
fn a_rate_is_held_within_its_limits() {
    let mut mixer = stereo_mixer(2);
    let sound = ramp(10);
    let fast = play(
        &mut mixer,
        SharedPcm::clone(&sound),
        Params::new().with_rate(u32::MAX),
    );
    let slow = play(&mut mixer, sound, Params::new().with_rate(0));
    render(&mut mixer, 1);
    render(&mut mixer, 1);
    assert!(!mixer.is_playing(fast), "at most 256 frames a frame");
    assert!(mixer.is_playing(slow), "at least 1/256 of a frame a frame");
    render(&mut mixer, 2570);
    assert!(!mixer.is_playing(slow));
}

#[test]
fn a_looped_join_interpolates_into_the_loops_start() {
    let mut mixer = stereo_mixer(1);
    let sound = pcm(RATE, Channels::Mono, vec![0, 100]);
    play(
        &mut mixer,
        sound,
        Params::new().looped().with_rate(RATE_ONE / 2),
    );
    assert_eq!(left(&render(&mut mixer, 5)), [0, 50, 100, 50, 0]);
}

#[test]
fn a_sound_at_another_rate_takes_the_time_it_should() {
    for (source_rate, frames, expected) in [
        (44_100, 44_100, 48_000),
        (22_050, 22_050, 48_000),
        (96_000, 9_600, 4_800),
        (48_000, 777, 777),
    ] {
        let mut mixer = stereo_mixer(1);
        let sound = pcm(source_rate, Channels::Mono, vec![1000; frames]);
        let voice = play(&mut mixer, sound, Params::new());
        render(&mut mixer, expected - 1);
        assert!(mixer.is_playing(voice), "{source_rate}");
        let last = render(&mut mixer, 2);
        assert_eq!(left(&last), [1000, 0], "{source_rate}");
        assert!(!mixer.is_playing(voice), "{source_rate}");
    }
}

#[test]
fn resampling_interpolates_in_fixed_point() {
    let mut mixer = stereo_mixer(1);
    // Four output frames for each source frame.
    let sound = pcm(12_000, Channels::Mono, vec![0, 400, -400]);
    play(&mut mixer, sound, Params::new());
    assert_eq!(
        left(&render(&mut mixer, 9)),
        [0, 100, 200, 300, 400, 200, 0, -200, -400]
    );
}

#[test]
fn mono_comes_out_of_both_sides() {
    let mut mixer = stereo_mixer(1);
    play(&mut mixer, ramp(2), Params::new());
    assert_eq!(render(&mut mixer, 2), [1, 1, 2, 2]);
}

#[test]
fn stereo_stays_on_its_sides() {
    let mut mixer = stereo_mixer(1);
    let sound = pcm(RATE, Channels::Stereo, vec![1, -1, 2, -2]);
    play(&mut mixer, sound, Params::new());
    assert_eq!(render(&mut mixer, 2), [1, -1, 2, -2]);
}

#[test]
fn stereo_on_a_mono_output_is_the_mean() {
    let mut mixer = mono_mixer(1);
    let sound = pcm(RATE, Channels::Stereo, vec![100, 200, -100, -300, 1, 2]);
    play(&mut mixer, sound, Params::new());
    assert_eq!(render(&mut mixer, 3), [150, -200, 2]);
}

#[test]
fn mono_on_a_mono_output_is_exact() {
    let mut mixer = mono_mixer(1);
    let sound = pcm(RATE, Channels::Mono, vec![i16::MIN, i16::MAX, -1, 1]);
    play(&mut mixer, sound, Params::new());
    assert_eq!(render(&mut mixer, 4), [i16::MIN, i16::MAX, -1, 1]);
}

#[test]
fn a_voice_at_full_left_sounds_only_on_the_left() {
    let mut mixer = stereo_mixer(1);
    let sound = pcm(RATE, Channels::Mono, vec![1000; 4]);
    let voice = play(&mut mixer, sound, Params::new().with_balance(UNITY, 0));
    let out = render(&mut mixer, 2);
    assert_eq!(left(&out), [1000, 1000]);
    assert_eq!(right(&out), [0, 0]);
    mixer.set_balance(voice, 0, UNITY / 2);
    let out = render(&mut mixer, 2);
    assert_eq!(left(&out), [0, 0]);
    assert_eq!(right(&out), [500, 500]);
}

#[test]
fn a_mono_output_plays_the_mean_of_the_balance() {
    let mut mixer = mono_mixer(1);
    let sound = pcm(RATE, Channels::Mono, vec![1000; 2]);
    let voice = play(&mut mixer, sound, Params::new().with_balance(UNITY, 0));
    assert_eq!(render(&mut mixer, 1), [500]);
    mixer.set_balance(voice, UNITY, UNITY);
    assert_eq!(render(&mut mixer, 1), [1000]);
}

#[test]
fn a_full_mixer_refuses_and_stops_nothing() {
    let mut mixer = stereo_mixer(2);
    let sound = mixer.add_sound(ramp(9)).unwrap_or_else(|e| panic!("{e}"));
    let first = mixer
        .play(sound, Params::new())
        .unwrap_or_else(|e| panic!("{e}"));
    let second = mixer
        .play(sound, Params::new())
        .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(mixer.play(sound, Params::new()), Err(Full));
    let stream = SharedSource::new(ramp(9));
    assert_eq!(mixer.stream(stream, Params::new()).err(), Some(Full));
    assert!(mixer.is_playing(first) && mixer.is_playing(second));
    assert_eq!(mixer.playing(), 2);
    assert_eq!(left(&render(&mut mixer, 1)), [2]);
    mixer.stop(first);
    assert!(mixer.play(sound, Params::new()).is_ok());
}

#[test]
fn an_id_of_an_ended_voice_means_nothing_though_its_voice_is_busy_again() {
    let mut mixer = stereo_mixer(1);
    let sound = mixer.add_sound(ramp(9)).unwrap_or_else(|e| panic!("{e}"));
    let old = mixer
        .play(sound, Params::new())
        .unwrap_or_else(|e| panic!("{e}"));
    mixer.stop(old);
    assert!(!mixer.is_playing(old));
    let new = mixer
        .play(sound, Params::new())
        .unwrap_or_else(|e| panic!("{e}"));
    assert_ne!(old, new);
    mixer.stop(old);
    mixer.pause(old);
    mixer.resume(old);
    mixer.set_volume(old, 0);
    mixer.fade_to(old, 0, 10);
    mixer.set_balance(old, 0, 0);
    assert!(!mixer.is_paused(new));
    assert_eq!(mixer.volume(new), Some(UNITY));
    assert_eq!(left(&render(&mut mixer, 1)), [1]);
}

#[test]
fn the_bank_is_fixed_and_a_removed_sound_is_gone() {
    let mut mixer = Mixer::new(Config::new(RATE, Channels::Stereo, 2).with_sounds(1));
    let sound = ramp(9);
    let id = mixer
        .add_sound(SharedPcm::clone(&sound))
        .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(mixer.add_sound(SharedPcm::clone(&sound)), Err(Full));
    let voice = mixer
        .play(id, Params::new())
        .unwrap_or_else(|e| panic!("{e}"));
    let back = mixer.remove_sound(id);
    assert!(back.is_some_and(|back| SharedPcm::ptr_eq(&back, &sound)));
    assert!(!mixer.is_playing(voice), "what played it stopped");
    assert_eq!(mixer.remove_sound(id), None);

    let other = mixer.add_sound(ramp(2)).unwrap_or_else(|e| panic!("{e}"));
    assert_ne!(other, id, "the slot's new tenant isn't the old id");
    let late = mixer
        .play(id, Params::new())
        .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(render(&mut mixer, 1), [0, 0], "no frames");
    assert!(!mixer.is_playing(late));
}

#[test]
fn removing_a_sound_leaves_what_plays_another() {
    let mut mixer = stereo_mixer(2);
    let gone = mixer.add_sound(ramp(9)).unwrap_or_else(|e| panic!("{e}"));
    let kept = mixer.add_sound(ramp(9)).unwrap_or_else(|e| panic!("{e}"));
    let voice = mixer
        .play(kept, Params::new())
        .unwrap_or_else(|e| panic!("{e}"));
    assert!(mixer.remove_sound(gone).is_some());
    assert!(mixer.is_playing(voice));
}

// -- streams --

#[test]
fn a_stream_plays_as_a_sound_does() {
    let mut mixer = stereo_mixer(1);
    let voice = mixer
        .stream(SharedSource::new(ramp(3)), Params::new())
        .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(left(&render(&mut mixer, 2)), [1, 2]);
    assert!(mixer.is_playing(voice));
    assert_eq!(left(&render(&mut mixer, 2)), [3, 0]);
    assert!(!mixer.is_playing(voice));
}

#[test]
fn a_looped_stream_comes_round_with_no_gap() {
    let mut mixer = stereo_mixer(1);
    mixer
        .stream(SharedSource::new(ramp(3)), Params::new().looped())
        .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(left(&render(&mut mixer, 7)), [1, 2, 3, 1, 2, 3, 1]);
}

#[test]
fn a_looped_stream_with_no_frames_ends() {
    let mut mixer = stereo_mixer(1);
    let empty = pcm(RATE, Channels::Mono, Vec::new());
    let voice = mixer
        .stream(SharedSource::new(empty), Params::new().looped())
        .unwrap_or_else(|e| panic!("{e}"));
    render(&mut mixer, 1);
    assert!(!mixer.is_playing(voice));
}

/// Frames of `frames` frames, `0..`, in a short pcm.
fn numbered(frames: usize) -> SharedPcm {
    pcm(
        RATE,
        Channels::Stereo,
        (0..frames)
            .flat_map(|frame| {
                let n = i16::try_from(frame).unwrap_or(0);
                [n, -n]
            })
            .collect(),
    )
}

fn stereo_numbers(frames: impl Iterator<Item = usize>) -> Vec<i16> {
    frames
        .flat_map(|frame| {
            let n = i16::try_from(frame).unwrap_or(0);
            [n, -n]
        })
        .collect()
}

#[test]
fn a_streams_region_plays_its_intro_once_and_the_region_twice_exactly() {
    // A pcm longer than the buffers, so that the joins fall mid-read.
    let (intro, end) = (1000, 1600);
    let mut mixer = stereo_mixer(1);
    let region = Region::new(intro, end).unwrap_or_else(|| panic!("frames"));
    mixer
        .stream(
            SharedSource::new(numbered(2000)),
            Params::new().with_region(region),
        )
        .unwrap_or_else(|e| panic!("{e}"));
    let out = render(&mut mixer, 1000 + 600 + 600);
    let expected = stereo_numbers((0..1600).chain(1000..1600));
    assert_eq!(out, expected);
}

#[test]
fn a_source_that_cant_go_back_ends_at_its_regions_end() {
    let (mut feeder, feed) = feed(format(RATE, Channels::Mono), 4);
    feeder.write(&[1, 2, 3, 4]);
    let mut mixer = stereo_mixer(1);
    let voice = mixer
        .stream(
            NoGoingBack(feed),
            Params::new().with_region(Region::new(1, 3).unwrap_or_else(|| panic!("frames"))),
        )
        .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(left(&render(&mut mixer, 4)), [1, 2, 3, 0]);
    assert!(!mixer.is_playing(voice));
}

/// A feed without its way back.
struct NoGoingBack(super::stream::Feed);

impl Source for NoGoingBack {
    fn format(&self) -> PcmFormat {
        self.0.format()
    }
    fn total_frames(&self) -> Option<u64> {
        None
    }
    fn read(&mut self, out: &mut [i16]) -> Result<usize, Error> {
        self.0.read(out)
    }
}

#[test]
fn an_empty_feed_is_an_underrun_that_delays_and_loses_nothing() {
    let (mut feeder, feed) = feed(format(RATE, Channels::Mono), 16);
    feeder.write(&[1, 2]);
    let mut mixer = stereo_mixer(1);
    let voice = mixer
        .stream(feed, Params::new())
        .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(left(&render(&mut mixer, 4)), [1, 2, 0, 0]);
    assert_eq!(mixer.underrun_frames(), 2);
    assert!(mixer.is_playing(voice), "a late worker doesn't end it");
    assert_eq!(left(&render(&mut mixer, 2)), [0, 0]);
    assert_eq!(mixer.underrun_frames(), 4);
    feeder.write(&[3, 4]);
    assert_eq!(left(&render(&mut mixer, 2)), [3, 4]);
    assert_eq!(mixer.underrun_frames(), 4);
    feeder.close();
    assert_eq!(left(&render(&mut mixer, 1)), [0]);
    assert!(!mixer.is_playing(voice), "closed and read, it ended");
    assert_eq!(mixer.underrun_frames(), 4);
}

#[test]
fn an_underrun_in_the_middle_of_a_step_loses_nothing_either() {
    let (mut feeder, feed) = feed(format(RATE, Channels::Mono), 16);
    feeder.write(&[0]);
    let mut mixer = stereo_mixer(1);
    mixer
        .stream(feed, Params::new().with_rate(RATE_ONE / 2))
        .unwrap_or_else(|e| panic!("{e}"));
    // Half way to a frame it hasn't got.
    assert_eq!(left(&render(&mut mixer, 3)), [0, 0, 0]);
    assert_eq!(mixer.underrun_frames(), 2);
    feeder.write(&[100]);
    feeder.close();
    assert_eq!(left(&render(&mut mixer, 3)), [50, 100, 100]);
}

#[test]
fn a_feeder_writes_what_there_is_room_for() {
    let (mut feeder, mut feed) = feed(format(RATE, Channels::Stereo), 4);
    assert_eq!(feeder.room(), 4);
    assert_eq!(feeder.write(&[1, 2, 3, 4, 5]), 2, "two whole frames");
    assert_eq!(feeder.room(), 2);
    assert_eq!(feeder.write(&[5, 6, 7, 8, 9, 10]), 2);
    assert_eq!(feeder.write(&[0, 0]), 0);
    let mut out = [0; 6];
    assert_eq!(feed.read(&mut out), Ok(3));
    assert_eq!(out, [1, 2, 3, 4, 5, 6]);
    assert_eq!(feeder.room(), 3);
    assert_eq!(feed.read(&mut out), Ok(1));
    assert_eq!(out[..2], [7, 8]);
    assert_eq!(feed.read(&mut out), Err(Error::Underrun));
    assert_eq!(feed.total_frames(), None);
    assert_eq!(feed.rewind_to(0), Ok(()));
    let mut no_room = [0; 1];
    assert!(matches!(feed.read(&mut no_room), Err(Error::NoRoom { .. })));
    feeder.close();
    assert_eq!(feed.read(&mut out), Ok(0));
}

#[test]
fn what_is_written_before_the_close_is_read_before_the_end() {
    let (mut feeder, mut feed) = feed(format(RATE, Channels::Mono), 4);
    feeder.write(&[1, 2]);
    feeder.close();
    let mut out = [0; 8];
    assert_eq!(feed.read(&mut out), Ok(2));
    assert_eq!(feed.read(&mut out), Ok(0));
}

fn fill_until_closed(feeder: &mut Feeder, source: &mut impl Source, mixer: &mut Mixer) -> Vec<i16> {
    let mut out = Vec::new();
    while mixer.playing() > 0 {
        feeder.fill_from(source).unwrap_or_else(|e| panic!("{e}"));
        out.extend(render(mixer, 7));
    }
    out
}

#[test]
fn fill_from_moves_a_whole_source_through_a_small_ring_and_closes_it() {
    let sound = numbered(5000);
    let (mut feeder, feed) = feed(sound.format(), 64);
    let mut mixer = stereo_mixer(1);
    mixer
        .stream(feed, Params::new())
        .unwrap_or_else(|e| panic!("{e}"));
    let mut source = SharedSource::new(sound);
    let mut out = Vec::new();
    // First what the mix can't have yet.
    feeder
        .fill_from(&mut source)
        .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(feeder.room(), 0, "a ring is filled and no more");
    out.extend(fill_until_closed(&mut feeder, &mut source, &mut mixer));
    let played = out.len() / 2;
    assert!(played >= 5000, "{played}");
    assert_eq!(out[..10_000], stereo_numbers(0..5000));
    assert!(out[10_000..].iter().all(|s| *s == 0));
    assert_eq!(mixer.underrun_frames(), 0);
}

#[test]
fn fill_from_says_how_many_it_moved_and_never_more_than_there_is_room_for() {
    let sound = pcm(RATE, Channels::Mono, (1..=9).collect());
    let mut source = SharedSource::new(sound);
    let (mut feeder, mut feed) = feed(format(RATE, Channels::Mono), 1);
    let mut out = [0; 1];
    for expected in 1..=9 {
        assert_eq!(feeder.fill_from(&mut source), Ok(1), "{expected}");
        assert_eq!(feed.read(&mut out), Ok(1));
        assert_eq!(out, [expected], "none lost for want of room");
    }
    assert_eq!(feeder.fill_from(&mut source), Ok(0));
    assert_eq!(feed.read(&mut out), Ok(0), "it closed at the end");

    let mut source = SharedSource::new(numbered(40));
    let (mut feeder, _feed) = super::stream::feed(format(RATE, Channels::Stereo), 32);
    assert_eq!(
        feeder.fill_from(&mut source),
        Ok(32),
        "more than one read's"
    );
}

#[test]
fn a_worker_with_a_source_of_other_channels_is_told() {
    let (mut feeder, _feed) = feed(format(RATE, Channels::Stereo), 4);
    let mut source = SharedSource::new(ramp(3));
    assert_eq!(feeder.fill_from(&mut source), Err(Error::Channels(1)));
}

/// `numbered(frames)` restarted by a worker through [`LoopedRegion`], counting
/// each restart.
fn looped_region(frames: usize, region: Region) -> impl Source + Send + 'static {
    let sound = numbered(frames);
    let first = SharedSource::new(SharedPcm::clone(&sound));
    LoopedRegion::new(first, region, move || {
        Ok(SharedSource::new(SharedPcm::clone(&sound)))
    })
}

#[test]
fn a_worker_supplies_a_loops_frames_and_the_mix_hears_them_exactly() {
    let region = Region::new(1000, 1600).unwrap_or_else(|| panic!("frames"));
    let mut source = looped_region(2000, region);
    let (mut feeder, feed) = feed(source.format(), 256);
    let mut mixer = stereo_mixer(1);
    mixer
        .stream(feed, Params::new())
        .unwrap_or_else(|e| panic!("{e}"));
    let mut out = Vec::new();
    for _ in 0..300 {
        feeder
            .fill_from(&mut source)
            .unwrap_or_else(|e| panic!("{e}"));
        out.extend(render(&mut mixer, 11));
    }
    let expected = stereo_numbers((0..1600).chain(1000..1600).chain(1000..1600));
    assert_eq!(out[..expected.len()], expected);
    assert_eq!(mixer.underrun_frames(), 0);
    assert!(mixer.playing() == 1, "a loop never ends");
}

#[test]
fn a_looped_regions_file_may_end_before_the_region_does() {
    let region = Region::new(1, 99).unwrap_or_else(|| panic!("frames"));
    let mut source = looped_region(3, region);
    let mut out = [0; 16];
    let mut seen = Vec::new();
    while seen.len() < 16 {
        let frames = source.read(&mut out).unwrap_or_else(|e| panic!("{e}"));
        assert!(frames > 0, "it comes round, it doesn't end");
        seen.extend_from_slice(&out[..frames * 2]);
    }
    assert_eq!(
        seen[..16],
        stereo_numbers([0, 1, 2, 1, 2, 1, 2, 1].into_iter())
    );
}

#[test]
fn a_mono_regions_pre_roll_stops_exactly_at_the_regions_start() {
    let region = Region::new(3, 5).unwrap_or_else(|| panic!("frames"));
    let sound = ramp(9);
    let first = SharedSource::new(SharedPcm::clone(&sound));
    let mut source = LoopedRegion::new(first, region, move || {
        Ok(SharedSource::new(SharedPcm::clone(&sound)))
    });
    let mut out = [0; 16];
    let mut seen = Vec::new();
    while seen.len() < 9 {
        let frames = source.read(&mut out).unwrap_or_else(|e| panic!("{e}"));
        assert!(frames > 0, "it comes round, it doesn't end");
        seen.extend_from_slice(&out[..frames]);
    }
    assert_eq!(seen[..9], [1, 2, 3, 4, 5, 4, 5, 4, 5]);
}

#[test]
fn a_looped_region_after_the_files_end_ends_rather_than_spins() {
    let region = Region::new(5, 9).unwrap_or_else(|| panic!("frames"));
    let mut source = looped_region(3, region);
    let mut out = [0; 16];
    assert_eq!(source.read(&mut out), Ok(3));
    assert_eq!(source.read(&mut out), Ok(0));
    assert_eq!(source.total_frames(), None);
    assert_eq!(source.rewind_to(0), Err(Error::NoRewind));
}

#[test]
fn a_restart_that_fails_is_the_workers_to_hear() {
    let region = Region::new(0, 1).unwrap_or_else(|| panic!("frames"));
    let mut source = LoopedRegion::new(SharedSource::new(ramp(1)), region, || {
        Err::<SharedSource, _>(Error::NoRewind)
    });
    let mut out = [0; 1];
    assert_eq!(source.read(&mut out), Ok(1));
    assert_eq!(source.read(&mut out), Err(Error::NoRewind));
}

#[test]
fn two_streams_sound_at_once_for_a_crossfade() {
    let mut mixer = Mixer::new(Config::new(RATE, Channels::Stereo, 2));
    let a = pcm(RATE, Channels::Mono, vec![1000; 100]);
    let b = pcm(RATE, Channels::Mono, vec![2000; 100]);
    let outgoing = mixer
        .stream(SharedSource::new(a), Params::new())
        .unwrap_or_else(|e| panic!("{e}"));
    let incoming = mixer
        .stream(SharedSource::new(b), Params::new().with_volume(0))
        .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(left(&render(&mut mixer, 1)), [1000]);
    mixer.fade_to(outgoing, 0, 1);
    mixer.fade_to(incoming, UNITY, 1);
    let out = left(&render(&mut mixer, 49));
    assert_eq!(out[0], 1000);
    assert_eq!(out[24], 1500, "half of each");
    assert_eq!(out[48], 2000);
}

#[test]
fn stream_slots_are_fixed_and_a_finished_one_makes_room() {
    let mut mixer = Mixer::new(Config::new(RATE, Channels::Stereo, 3).with_streams(1));
    let source = || SharedSource::new(ramp(1));
    let voice = mixer
        .stream(source(), Params::new())
        .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(mixer.stream(source(), Params::new()).err(), Some(Full));
    render(&mut mixer, 2);
    assert!(!mixer.is_playing(voice));
    assert!(mixer.take_finished_stream().is_some(), "one to free");
    assert!(mixer.take_finished_stream().is_none());

    // A finished stream left in its slot gives way too.
    let again = mixer
        .stream(source(), Params::new())
        .unwrap_or_else(|e| panic!("{e}"));
    mixer.stop(again);
    assert!(mixer.stream(source(), Params::new()).is_ok());
}

#[test]
fn a_stream_held_plays_once_and_only_by_its_own_id() {
    let mut mixer = Mixer::new(Config::new(RATE, Channels::Stereo, 2).with_streams(1));
    let stream = mixer
        .add_stream(Box::new(SharedSource::new(ramp(2))))
        .unwrap_or_else(|e| panic!("{e}"));
    let voice = mixer
        .play_stream(stream, Params::new())
        .unwrap_or_else(|e| panic!("{e}"));
    let second = mixer
        .play_stream(stream, Params::new())
        .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(left(&render(&mut mixer, 2)), [1, 2]);
    assert!(!mixer.is_playing(second), "a stream plays once");
    render(&mut mixer, 1);
    assert!(!mixer.is_playing(voice));
    let taken = mixer.take_finished_stream();
    assert!(taken.is_some());
    let stale = mixer
        .play_stream(stream, Params::new())
        .unwrap_or_else(|e| panic!("{e}"));
    render(&mut mixer, 1);
    assert!(!mixer.is_playing(stale), "it is gone");
}

#[test]
fn an_old_streams_id_doesnt_play_the_stream_that_took_its_slot() {
    let mut mixer = Mixer::new(Config::new(RATE, Channels::Stereo, 2).with_streams(1));
    let old = mixer
        .add_stream(Box::new(SharedSource::new(ramp(1))))
        .unwrap_or_else(|e| panic!("{e}"));
    let voice = mixer
        .play_stream(old, Params::new())
        .unwrap_or_else(|e| panic!("{e}"));
    render(&mut mixer, 2);
    assert!(!mixer.is_playing(voice));
    assert!(mixer.take_finished_stream().is_some());
    let new = mixer
        .add_stream(Box::new(SharedSource::new(ramp(2))))
        .unwrap_or_else(|e| panic!("{e}"));
    assert_ne!(old, new);
    mixer
        .play_stream(old, Params::new())
        .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(
        left(&render(&mut mixer, 1)),
        [0],
        "the old id plays nothing"
    );
    mixer
        .play_stream(new, Params::new())
        .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(
        left(&render(&mut mixer, 2)),
        [1, 2],
        "nothing of it was used"
    );
}

#[test]
fn a_mixer_shows_what_it_is_made_with_and_how_late_it_has_been() {
    let mixer = stereo_mixer(3);
    let shown = format!("{mixer:?}");
    assert!(shown.starts_with("Mixer"), "{shown}");
    assert!(shown.contains("voices: 3"), "{shown}");
    assert!(shown.contains("underrun_frames: 0"), "{shown}");
}

// -- the queue, on one thread: `tests/mix_queue.rs` has two --

#[test]
fn a_remote_commands_the_mixer_through_its_inbox() {
    let mut mixer = stereo_mixer(2);
    let sound = mixer.add_sound(ramp(100)).unwrap_or_else(|e| panic!("{e}"));
    let (mut remote, mut inbox) = mixer.remote(16);
    let voice = remote
        .play(sound, Params::new().with_volume(UNITY / 2))
        .unwrap_or_else(|e| panic!("{e}"));
    assert!(remote.is_playing(voice), "as far as it knows");
    assert_eq!(render(&mut mixer, 1), [0, 0], "nothing yet");
    inbox.apply(&mut mixer);
    assert!(mixer.is_playing(voice));
    assert_eq!(left(&render(&mut mixer, 1)), [1], "half of 1 and a half");

    let check = |remote_result: Result<(), Full>| assert_eq!(remote_result, Ok(()));
    check(remote.set_volume(voice, UNITY));
    check(remote.set_balance(voice, UNITY, 0));
    inbox.apply(&mut mixer);
    assert_eq!(render(&mut mixer, 1), [2, 0]);

    check(remote.pause(voice));
    inbox.apply(&mut mixer);
    assert!(mixer.is_paused(voice));
    check(remote.resume(voice));
    check(remote.fade_to(voice, 0, 1));
    inbox.apply(&mut mixer);
    assert!(!mixer.is_paused(voice));
    render(&mut mixer, 48);
    assert_eq!(mixer.volume(voice), Some(0));

    check(remote.stop(voice));
    assert!(remote.is_playing(voice), "until the mixer says");
    inbox.apply(&mut mixer);
    assert!(!mixer.is_playing(voice));
    inbox.apply(&mut mixer);
    assert!(!remote.is_playing(voice));
}

#[test]
fn a_remote_says_full_when_its_voices_are_taken_until_it_hears_one_ended() {
    let mut mixer = stereo_mixer(1);
    let sound = mixer.add_sound(ramp(1)).unwrap_or_else(|e| panic!("{e}"));
    let (mut remote, mut inbox) = mixer.remote(4);
    let voice = remote
        .play(sound, Params::new())
        .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(remote.play(sound, Params::new()), Err(Full));
    inbox.apply(&mut mixer);
    render(&mut mixer, 2);
    assert!(remote.is_playing(voice), "the mixer hasn't said");
    assert_eq!(remote.play(sound, Params::new()), Err(Full));
    inbox.apply(&mut mixer);
    assert!(!remote.is_playing(voice));
    let next = remote
        .play(sound, Params::new())
        .unwrap_or_else(|e| panic!("{e}"));
    assert_ne!(voice, next, "the same voice, a new id");
    assert!(remote.is_playing(next));
    assert!(!remote.is_playing(voice));
}

#[test]
fn a_remote_says_full_when_its_queue_is_and_takes_no_voice_for_it() {
    let mut mixer = stereo_mixer(4);
    let sound = mixer.add_sound(ramp(9)).unwrap_or_else(|e| panic!("{e}"));
    let (mut remote, mut inbox) = mixer.remote(1);
    let voice = remote
        .play(sound, Params::new())
        .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(remote.play(sound, Params::new()), Err(Full));
    assert_eq!(remote.stop(voice), Err(Full));
    inbox.apply(&mut mixer);
    assert_eq!(mixer.playing(), 1);
    let second = remote
        .play(sound, Params::new())
        .unwrap_or_else(|e| panic!("{e}"));
    assert_ne!(voice, second);
}

#[test]
fn a_remote_plays_a_stream_the_mixer_was_given_ahead_of_time() {
    let mut mixer = stereo_mixer(1);
    let stream = mixer
        .add_stream(Box::new(SharedSource::new(ramp(2))))
        .unwrap_or_else(|e| panic!("{e}"));
    let (mut remote, mut inbox) = mixer.remote(4);
    let voice = remote
        .play_stream(stream, Params::new())
        .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(remote.play_stream(stream, Params::new()), Err(Full));
    inbox.apply(&mut mixer);
    assert_eq!(left(&render(&mut mixer, 3)), [1, 2, 0]);
    inbox.apply(&mut mixer);
    assert!(!remote.is_playing(voice));
    assert!(mixer.take_finished_stream().is_some());
}

// -- what holds of any script --

/// One thing a script does.
#[derive(Debug, Clone)]
enum Step {
    Play(u8, Params),
    Stream(u8, Params),
    Volume(u8, u16),
    Fade(u8, u16, u16),
    Balance(u8, u16, u16),
    Stop(u8),
    Pause(u8),
    Resume(u8),
    Render(u16),
}

prop_compose! {
    fn any_params()(
        volume: u16,
        looped in proptest::bool::weighted(0.2),
        rate in prop_oneof![Just(RATE_ONE), any::<u32>()],
        left: u16,
        right: u16,
        region in proptest::option::weighted(0.2, (0u64..200, 0u64..200)),
    ) -> Params {
        let mut params = Params::new()
            .with_volume(volume)
            .with_rate(rate)
            .with_balance(left, right);
        params.looped = looped;
        params.region = region.and_then(|(a, b)| Region::new(a.min(b), a.max(b)));
        params
    }
}

fn step() -> impl Strategy<Value = Step> {
    prop_oneof![
        3 => (any::<u8>(), any_params()).prop_map(|(n, p)| Step::Play(n, p)),
        1 => (any::<u8>(), any_params()).prop_map(|(n, p)| Step::Stream(n, p)),
        1 => (any::<u8>(), any::<u16>()).prop_map(|(n, v)| Step::Volume(n, v)),
        1 => (any::<u8>(), any::<u16>(), any::<u16>()).prop_map(|(n, v, m)| Step::Fade(n, v, m)),
        1 => (any::<u8>(), any::<u16>(), any::<u16>()).prop_map(|(n, l, r)| Step::Balance(n, l, r)),
        1 => any::<u8>().prop_map(Step::Stop),
        1 => any::<u8>().prop_map(Step::Pause),
        1 => any::<u8>().prop_map(Step::Resume),
        3 => (0u16..700).prop_map(Step::Render),
    ]
}

/// Sounds of every kind a script's plays pick from.
fn bank() -> [SharedPcm; 4] {
    [
        ramp(50),
        pcm(44_100, Channels::Stereo, vec![30_000; 120]),
        pcm(11_025, Channels::Mono, vec![-30_000; 7]),
        pcm(RATE, Channels::Stereo, Vec::new()),
    ]
}

/// Runs `steps`, and says whether every voice that isn't looped ended when it
/// should have: each, at any rate, within the frames its length and slowest rate
/// allow, once nothing pauses it.
fn run(steps: &[Step], channels: Channels) {
    let mut mixer = Mixer::new(Config::new(RATE, channels, 6));
    let sounds: Vec<SoundId> = bank()
        .into_iter()
        .map(|sound| mixer.add_sound(sound).unwrap_or_else(|e| panic!("{e}")))
        .collect();
    let mut voices: Vec<(VoiceId, bool)> = Vec::new();
    let pick = |voices: &[(VoiceId, bool)], n: u8| {
        (!voices.is_empty()).then(|| voices[usize::from(n) % voices.len()].0)
    };
    for step in steps {
        match step {
            Step::Play(n, params) => {
                let sound = sounds[usize::from(*n) % sounds.len()];
                if let Ok(voice) = mixer.play(sound, *params) {
                    voices.push((voice, params.looped || params.region.is_some()));
                }
            }
            Step::Stream(n, params) => {
                let sound = bank()[usize::from(*n) % 4].clone();
                if let Ok(voice) = mixer.stream(SharedSource::new(sound), *params) {
                    voices.push((voice, params.looped || params.region.is_some()));
                }
            }
            Step::Volume(n, volume) => {
                if let Some(voice) = pick(&voices, *n) {
                    mixer.set_volume(voice, *volume);
                }
            }
            Step::Fade(n, volume, millis) => {
                if let Some(voice) = pick(&voices, *n) {
                    mixer.fade_to(voice, *volume, u64::from(*millis));
                }
            }
            Step::Balance(n, left, right) => {
                if let Some(voice) = pick(&voices, *n) {
                    mixer.set_balance(voice, *left, *right);
                }
            }
            Step::Stop(n) => {
                if let Some(voice) = pick(&voices, *n) {
                    mixer.stop(voice);
                }
            }
            Step::Pause(n) => {
                if let Some(voice) = pick(&voices, *n) {
                    mixer.pause(voice);
                }
            }
            Step::Resume(n) => {
                if let Some(voice) = pick(&voices, *n) {
                    mixer.resume(voice);
                }
            }
            Step::Render(frames) => {
                let mut out = vec![0; usize::from(*frames) * usize::from(channels.count())];
                mixer.render(&mut out);
            }
        }
    }
    // The longest: 120 frames at 44.1 kHz, or 50 at 48 kHz, or 7 at 11 kHz, at
    // 1/256 of a frame an output frame, whichever its rate.
    for (voice, _) in &voices {
        mixer.resume(*voice);
    }
    let mut out = vec![0; 256 * 130 * 2 * usize::from(channels.count())];
    mixer.render(&mut out);
    for (voice, loops) in voices {
        assert!(
            loops || !mixer.is_playing(voice),
            "{voice:?} outlived its sound"
        );
    }
}

proptest! {
    // A script's end is a long render: a few are plenty, on wasm32 too.
    #![proptest_config(ProptestConfig::with_cases(32))]

    /// For any script, `render` never panics, and no voice outlives its sound
    /// unless it loops.
    #[test]
    fn scripts_never_panic_and_voices_end(
        steps in proptest::collection::vec(step(), 0..40),
        stereo: bool,
    ) {
        run(&steps, if stereo { Channels::Stereo } else { Channels::Mono });
    }
}
