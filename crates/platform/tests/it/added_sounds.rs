//! Sounds a game adds at run time, in the Harness: samples handed to the
//! audio queue as a cue, played as a cue of the manifest is, and heard by the
//! headless backend's `Speaker` as the same samples.

use proptest::prelude::*;
use shuyi_platform::harness::Harness;
use shuyi_ui::audio::{AudioRequest, Samples};
use shuyi_ui::frame::glyph_buffer::GlyphBuffer;
use shuyi_ui::log::{LogCategory, LogLevel};
use shuyi_ui::screen::{FrameInput, Screen, ScreenStack, Transition};
use shuyi_ui::toy::{self, Action, Home, Toy, ToyCtx};
use shuyi_ui::{Game, GameHooks};

/// A screen that makes a sound: Confirm adds its samples as the cue `cue`,
/// Select plays it, Menu starts it, and Cancel removes it.
struct Synth {
    cue: &'static str,
    samples: Samples,
}

impl Screen<Toy> for Synth {
    fn name(&self) -> &'static str {
        "synth"
    }

    fn actions(&self) -> Option<Vec<Action>> {
        Some(vec![
            Action::Confirm,
            Action::Select,
            Action::Menu,
            Action::Cancel,
        ])
    }

    fn update(&mut self, ctx: &mut ToyCtx, input: &FrameInput<Action>) -> Transition<Toy> {
        for &action in &input.actions {
            match action {
                Action::Confirm => {
                    let _ = ctx.audio.add_sound(self.cue, self.samples.clone());
                }
                Action::Select => ctx.audio.play_sound(self.cue),
                Action::Menu => {
                    ctx.audio.start_sound_at(self.cue, 0.5);
                }
                Action::Cancel => ctx.audio.remove_sound(self.cue),
                _ => {}
            }
        }
        Transition::None
    }

    fn draw(&self, _: &ToyCtx, _: &mut GlyphBuffer) {}
}

/// The toy's home with a [`Synth`] of `samples` as `cue` over it.
fn synth(cue: &'static str, samples: Samples) -> Harness<Toy> {
    let mut stack = ScreenStack::new(Box::new(Home::default()));
    stack.push(Box::new(Synth { cue, samples }));
    Harness::new(Game::with_stack(toy::ctx(), stack, GameHooks::default()))
}

/// A tenth of a second of a square wave at 400 a second, 8-bit, mono, at
/// 8,000 samples a second: 800 frames.
fn tone() -> Samples {
    let bytes: Vec<u8> = (0..800)
        .map(|i| if i % 20 < 10 { 64 } else { 192 })
        .collect();
    Samples::from_u8(8_000, 1, &bytes).unwrap_or_else(|e| panic!("{e}"))
}

/// The audio warnings in the game's log, oldest first.
fn audio_warnings(h: &Harness<Toy>) -> Vec<String> {
    h.log()
        .lines()
        .filter(|l| l.level == LogLevel::Warn && l.category == LogCategory::Audio)
        .map(|l| l.text.clone())
        .collect()
}

#[test]
fn a_tone_added_and_played_twice_is_heard_twice_as_its_samples() {
    let mut h = synth("tone", tone());
    h.act(Action::Confirm);
    assert_eq!(h.sounds_added(), ["tone"]);
    h.act(Action::Select);
    h.act(Action::Select);
    let plays = h.added_plays();
    assert_eq!(plays.len(), 2, "{plays:?}");
    for play in plays {
        assert_eq!(play.cue, "tone");
        assert_eq!(*play.samples, tone());
        assert_eq!(play.samples.millis(), 100);
        assert!((play.volume - 1.0).abs() < f32::EPSILON);
        assert_eq!(play.id, None);
    }
    // The plays are sounds as any cue's, and nothing was logged.
    assert_eq!(h.sounds(), ["tone", "tone"]);
    assert!(audio_warnings(&h).is_empty(), "{:?}", audio_warnings(&h));
}

#[test]
fn a_sound_added_starts_with_a_handle_and_its_removal_stops_it() {
    let mut h = synth("tone", tone());
    h.act(Action::Confirm);
    h.act(Action::Menu);
    let playing: Vec<&str> = h.sounds_playing().iter().map(|p| p.cue.as_str()).collect();
    assert_eq!(playing, ["tone"]);
    let plays = h.added_plays();
    assert_eq!(plays.len(), 1);
    assert!((plays[0].volume - 0.5).abs() < f32::EPSILON);
    assert_eq!(plays[0].id.map(shuyi_ui::audio::SoundId::get), Some(1));
    h.act(Action::Cancel);
    let found = h.sounds_added();
    assert!(found.is_empty(), "{found:?}");
    let found = h.sounds_playing();
    assert!(found.is_empty(), "{found:?}");
    let removed = h
        .last_frame_audio()
        .iter()
        .any(|r| matches!(r, AudioRequest::RemoveSound { cue } if cue == "tone"));
    assert!(removed);
}

#[test]
fn a_removed_cue_plays_nothing_and_is_logged_once() {
    let mut h = synth("tone", tone());
    h.act(Action::Confirm);
    h.act(Action::Cancel);
    h.act(Action::Select);
    h.act(Action::Select);
    h.act(Action::Menu);
    let found = h.added_plays();
    assert!(found.is_empty(), "{found:?}");
    let found = h.sounds();
    assert!(found.is_empty(), "{found:?}");
    let warnings = audio_warnings(&h);
    assert_eq!(warnings.len(), 1, "{warnings:?}");
    assert!(warnings[0].contains("\"tone\" was removed"), "{warnings:?}");
}

#[test]
fn a_cue_of_the_manifest_is_refused_and_logged() {
    let mut h = synth("hum", tone());
    h.act(Action::Confirm);
    let found = h.sounds_added();
    assert!(found.is_empty(), "{found:?}");
    let warnings = audio_warnings(&h);
    assert_eq!(warnings.len(), 1, "{warnings:?}");
    assert!(
        warnings[0].contains("cue of the audio manifest"),
        "{warnings:?}"
    );
    // It plays the manifest's own sound, not the one refused.
    h.act(Action::Menu);
    assert_eq!(h.sounds(), ["hum"]);
    let found = h.added_plays();
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn a_sound_too_long_is_refused_and_logged() {
    let long = Samples::new(100, 1, vec![0; 6_001]);
    let mut h = synth("long", long.unwrap_or_else(|e| panic!("{e}")));
    h.act(Action::Confirm);
    let found = h.sounds_added();
    assert!(found.is_empty(), "{found:?}");
    let warnings = audio_warnings(&h);
    assert_eq!(warnings.len(), 1, "{warnings:?}");
    assert!(warnings[0].contains("60010 milliseconds"), "{warnings:?}");
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(32))]

    /// Any samples within the limits, added then played, are heard by the
    /// speaker as the same samples.
    #[test]
    fn any_samples_added_are_heard_as_the_same_samples(
        rate in 1u32..=48_000,
        stereo: bool,
        samples in proptest::collection::vec(any::<i16>(), 0..512),
    ) {
        let channels = if stereo { 2 } else { 1 };
        let whole = samples.len() - samples.len() % channels as usize;
        let samples = Samples::new(rate, channels, samples[..whole].to_vec());
        let samples = samples.unwrap_or_else(|e| panic!("{e}"));
        // A sound longer than the limit is refused: the property holds of those within.
        prop_assume!(samples.frames() <= u64::from(rate) * 60);
        let mut h = synth("made", samples.clone());
        h.act(Action::Confirm);
        h.act(Action::Select);
        let plays = h.added_plays();
        prop_assert_eq!(plays.len(), 1);
        prop_assert_eq!(&*plays[0].samples, &samples);
    }
}
