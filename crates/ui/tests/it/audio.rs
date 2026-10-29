//! The audio the Harness records and its stand-in for the music player
//! (ticket 1111): the sounds, voices and music commands each frame asked
//! for, the volumes it told the platform, and a music clock that counts
//! from when a track starts sounding.

use shuyi_platform::harness::{FRAME_DT, Harness};
use shuyi_ui::audio::{AudioRequest, MusicCommand, MusicSwitch};
use shuyi_ui::frame::GlyphBuffer;
use shuyi_ui::settings::MAX_VOLUME;
use shuyi_ui::toy::{self, Action, Toy, ToyCtx, Voice};
use shuyi_ui::{FrameInput, Game, GameHooks, Screen, Transition};

/// Clicks on Confirm, plays the track `theme` on Cancel, the fights' pool
/// on Info, and crossfades to `drums` over 1 s on Menu.
struct Beeper;

impl Screen<Toy> for Beeper {
    fn name(&self) -> &'static str {
        "beeper"
    }

    fn update(&mut self, ctx: &mut ToyCtx, input: &FrameInput<Action>) -> Transition<Toy> {
        for action in &input.actions {
            match action {
                Action::Confirm => ctx.audio.play_sound("click"),
                Action::Cancel => ctx.audio.play_music("theme"),
                Action::Info => {
                    if let Some(cue) = ctx.pick_music("fights") {
                        ctx.audio.play_music(&cue);
                    }
                }
                Action::Menu => ctx
                    .audio
                    .play_music_with("drums", MusicSwitch::crossfade(1000)),
                _ => {}
            }
        }
        Transition::None
    }

    fn draw(&self, _: &ToyCtx, _: &mut GlyphBuffer) {}
}

fn beeper() -> Harness<Toy> {
    Harness::new(Game::new(
        toy::ctx(),
        Box::new(Beeper),
        GameHooks::default(),
    ))
}

fn click() -> AudioRequest<Voice> {
    AudioRequest::PlaySound {
        cue: "click".into(),
        volume: 1.0,
    }
}

fn load(cue: &str) -> MusicCommand {
    MusicCommand::Load { cue: cue.into() }
}

fn start(cue: &str) -> MusicCommand {
    MusicCommand::start(cue, 1.0)
}

#[test]
fn records_the_audio_screens_ask_for() {
    let theme = AudioRequest::play_music("theme", None);
    let mut h = beeper();
    let found = h.audio_requests();
    assert!(found.is_empty(), "{found:?}");
    let found = h.last_frame_audio();
    assert!(found.is_empty(), "{found:?}");
    h.keys("f");
    assert_eq!(h.audio_requests(), [click()]);
    assert_eq!(h.sounds(), ["click"]);
    h.keys("d f");
    assert_eq!(h.audio_requests(), [click(), theme, click()]);
    assert_eq!(h.sounds(), ["click", "click"]);
    assert_eq!(h.music_commands(), [load("theme"), start("theme")]);
    h.clear_audio();
    let found = h.audio_requests();
    assert!(found.is_empty(), "{found:?}");
    let found = h.music_commands();
    assert!(found.is_empty(), "{found:?}");
    let found = h.sounds();
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn last_frame_audio_is_the_last_frame_only() {
    let mut h = beeper();
    // `keys` presses in one frame and releases in the next.
    h.keys("f");
    let found = h.last_frame_audio();
    assert!(found.is_empty(), "{found:?}");
    h.hold("f", 0.0);
    // The hold's press, then its release: the click is a frame back.
    let found = h.last_frame_audio();
    assert!(found.is_empty(), "{found:?}");
    let f = shuyi_ui::input::Chord::parse("f").unwrap_or_else(|e| panic!("{e}"));
    h.frame(&[shuyi_ui::RawInputEvent::Down(f)], FRAME_DT);
    assert_eq!(h.last_frame_audio(), [click()]);
    h.wait(0.1);
    let found = h.last_frame_audio();
    assert!(found.is_empty(), "{found:?}");
    assert_eq!(h.audio_requests(), [click(), click(), click()]);
}

#[test]
fn records_the_voices_asked_for_by_line() {
    let mut h = beeper();
    let lines = h.ctx().game.lines();
    let manifest = h.ctx().game.manifest(&[(0, "Only"), (1, "Only")]);
    h.ctx_mut().set_voice_manifest(&manifest);
    h.ctx_mut().play_voice(&lines[1]);
    h.ctx_mut().play_voice(&lines[0]);
    h.wait(0.0);
    let ids = [lines[1].to_string(), lines[0].to_string()];
    assert_eq!(h.voices(), ids);
}

#[test]
fn the_volumes_and_fullscreen_are_what_the_last_frame_said() {
    let mut h = beeper();
    // Before any frame: as the platform starts.
    assert_eq!(h.volumes(), (1.0, 1.0));
    assert!(!h.fullscreen());
    h.ctx_mut()
        .change_settings(|s| {
            s.music_volume = MAX_VOLUME / 2;
            s.sound_volume = 0;
            s.fullscreen = true;
        })
        .unwrap_or_else(|e| panic!("{e}"));
    // Not told until a frame runs.
    assert!(!h.fullscreen());
    h.wait(0.0);
    let music = h.ctx().settings().music_factor();
    assert!(music > 0.0 && music < 1.0, "{music}");
    assert_eq!(h.volumes(), (music, 0.0));
    assert!(h.fullscreen());
}

/// Whether `cue` sounds, `expected` seconds in (to 10 ms: frame times are
/// `f32`s).
fn sounds_at(h: &Harness<Toy>, cue: &str, expected: f32) -> bool {
    h.music_clock()
        .is_some_and(|c| c.cue == cue && (c.position - expected).abs() < 0.01)
}

#[test]
fn the_music_clock_counts_from_the_start_and_wraps_at_the_track_length() {
    let mut h = beeper();
    assert_eq!(h.music_clock(), None);
    // The track sounds from the frame that asked for it (the press), so
    // the next frame (the release) is the first to see a clock.
    h.keys("d");
    assert!(sounds_at(&h, "theme", FRAME_DT), "{:?}", h.music_clock());
    h.wait(1.0 - FRAME_DT);
    assert!(sounds_at(&h, "theme", 1.0), "{:?}", h.music_clock());
    // The toy's tracks are 10 s long.
    let length = h.music_clock().map(|c| c.length);
    assert!(
        length.is_some_and(|l| (l - 10.0).abs() < 1e-4),
        "{length:?}"
    );
    // Key presses are frames too.
    h.keys("Up");
    assert!(sounds_at(&h, "theme", 1.0 + 2.0 * FRAME_DT));
    h.wait(8.75 - 2.0 * FRAME_DT);
    assert!(sounds_at(&h, "theme", 9.75), "{:?}", h.music_clock());
    // Round to its start again.
    h.wait(1.0);
    assert!(sounds_at(&h, "theme", 0.75), "{:?}", h.music_clock());
}

#[test]
fn a_music_load_delay_starts_the_clock_late() {
    let mut h = beeper();
    h.music_load_delay(2.0).keys("d");
    h.wait(1.5);
    assert_eq!(h.music_clock(), None);
    h.wait(1.0);
    assert!(
        sounds_at(&h, "theme", 0.5 + FRAME_DT),
        "{:?}",
        h.music_clock()
    );
    // A negative or NaN delay is none.
    for delay in [-1.0, f32::NAN] {
        let mut h = beeper();
        h.music_load_delay(2.0).music_load_delay(delay).keys("d");
        h.wait(0.5);
        assert!(
            sounds_at(&h, "theme", 0.5 + FRAME_DT),
            "{delay}: {:?}",
            h.music_clock()
        );
    }
}

#[test]
fn without_music_the_clock_never_starts() {
    let mut h = beeper();
    h.without_music().keys("d");
    h.wait(3.0);
    assert_eq!(h.music_clock(), None);
    // The game asked for its music all the same.
    assert!(h.music_commands().contains(&start("theme")));
}

/// During a fade the clock is still the old track's; the new track's
/// starts when the fade ends (0.5 s, the toy's manifest).
#[test]
fn the_clock_follows_a_switch_of_track_once_the_fade_ends() {
    let mut h = beeper();
    h.keys("d").wait(1.0);
    h.keys("e");
    let so_far = 1.0 + 3.0 * FRAME_DT;
    assert!(sounds_at(&h, "theme", so_far), "{:?}", h.music_clock());
    h.wait(1.0);
    let clock = h.music_clock().unwrap_or_else(|| panic!("no music"));
    let pool = &h.ctx().game.audio.pools["fights"];
    assert!(pool.contains(&clock.cue), "{}", clock.cue);
    // The fade began with the `e` press, two frames before the wait.
    let expected = 1.0 + 2.0 * FRAME_DT - 0.5;
    assert!(
        (clock.position - expected).abs() < 0.05,
        "{}",
        clock.position
    );
}

#[test]
fn the_audio_log_says_which_frame_asked_for_what() {
    let theme = AudioRequest::play_music("theme", None);
    let mut h = beeper();
    // Frames 1 and 2: the click and its release; then 3 and 4.
    h.keys("f d");
    assert_eq!(h.frame_count(), 4);
    assert_eq!(h.audio_log(), [(1, &click()), (3, &theme)]);
    let music: Vec<(u32, MusicCommand)> = h
        .music_log()
        .into_iter()
        .map(|(f, c)| (f, c.clone()))
        .collect();
    assert_eq!(music, [(3, load("theme")), (3, start("theme"))]);
    // Cleared, the log starts again from the next frame.
    h.clear_audio();
    let found = h.audio_log();
    assert!(found.is_empty(), "{found:?}");
    let found = h.music_log();
    assert!(found.is_empty(), "{found:?}");
    h.wait(FRAME_DT).keys("f");
    assert_eq!(h.audio_log(), [(6, &click())]);
}

/// The gain each music command of the run gave `cue`, in order, with its
/// frame: its `Start`'s, then its `Gain`s.
fn gains(h: &Harness<Toy>, cue: &str) -> Vec<(u32, f32)> {
    let log = h.music_log().into_iter();
    log.filter_map(|(frame, command)| match command {
        MusicCommand::Start { cue: c, gain, .. } | MusicCommand::Gain { cue: c, gain } => {
            (c == cue).then_some((frame, *gain))
        }
        _ => None,
    })
    .collect()
}

/// A crossfade of 1 s: the new track starts silent beside the old one, and
/// once it sounds their gains move opposite ways until the old one stops.
#[test]
fn a_crossfade_plays_both_tracks_until_the_old_one_has_faded() {
    let mut h = beeper();
    h.keys("d").wait(1.0).act(Action::Menu);
    assert_eq!(
        h.music_commands()[2..],
        [load("drums"), MusicCommand::start("drums", 0.0)]
    );
    let switched = h.frame_count();
    h.wait(0.5);
    // From the frame after the switch, the first to see the new track
    // sound, both gains move every frame, and they add up to 1.
    let (theme, drums) = (gains(&h, "theme"), gains(&h, "drums"));
    assert_eq!(theme.len(), drums.len(), "{theme:?} {drums:?}");
    assert_eq!(drums[0], (switched, 0.0));
    for ((f, old), (g, new)) in theme[1..].iter().zip(&drums[1..]) {
        assert!(f == g && *f > switched, "{f} {g}");
        assert!((old + new - 1.0).abs() < 1e-4, "{old} {new}");
    }
    let last = theme.last().map_or(0.0, |(_, gain)| *gain);
    assert!((last - 0.5).abs() < 0.05, "{theme:?}");
    assert!(sounds_at(&h, "drums", 0.5), "{:?}", h.music_clock());
    h.wait(0.55);
    let stop = MusicCommand::Stop {
        cue: "theme".into(),
    };
    assert!(h.music_commands().contains(&stop));
    assert_eq!(gains(&h, "drums").last().map(|(_, gain)| *gain), Some(1.0));
}
