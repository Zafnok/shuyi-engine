//! The audio the Harness records and its stand-in for the music player
//! (ticket 1111): the sounds, voices and music commands each frame asked
//! for, the gains it told the platform, and a music clock that counts
//! from when a track starts sounding.

use shuyi_assets::audio::DuckRule;
use shuyi_platform::harness::{FRAME_DT, Harness};
use shuyi_ui::audio::groups::{MASTER, MUSIC, SFX, UI, VOICE};
use shuyi_ui::audio::loudness::loudness;
use shuyi_ui::audio::{AudioRequest, Gain, MusicCommand, MusicSwitch};
use shuyi_ui::frame::GlyphBuffer;
use shuyi_ui::settings::MAX_VOLUME;
use shuyi_ui::toy::{self, Action, Toy, ToyCtx, Voice};
use shuyi_ui::{FrameInput, Game, GameHooks, Screen, Transition};

/// Clicks on Confirm, plays the track `theme` on Cancel, the fights' pool
/// on Info, and crossfades to `drums` over 1 s on Menu; stops the music
/// over 3 s on Select, and with the default fade on down.
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
                Action::Select => ctx.audio.stop_music_with(3000),
                Action::CursorDown => ctx.audio.stop_music(),
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

/// Acceptance (2255): after `play_voice`, `voice_playing` names the line
/// from the next frame until the clip's length has passed, then is `None`;
/// a voice stopped is `None` from the next frame.
#[test]
fn a_voice_sounds_from_the_next_frame_until_its_length_has_passed() {
    let mut h = beeper();
    h.voice_length(1.0);
    let lines = h.ctx().game.lines();
    let manifest = h.ctx().game.manifest(&[(0, "Only"), (1, "Only")]);
    h.ctx_mut().set_voice_manifest(&manifest);
    h.ctx_mut().play_voice(&lines[0]);
    h.frame(&[], FRAME_DT);
    assert_eq!(h.ctx().voice_playing(), None, "the platform hasn't read it");
    h.frame(&[], FRAME_DT);
    assert_eq!(h.ctx().voice_playing(), Some(&lines[0]));
    h.wait(0.9);
    assert_eq!(h.ctx().voice_playing(), Some(&lines[0]), "still sounding");
    h.wait(0.2);
    assert_eq!(h.ctx().voice_playing(), None, "ended");
    // Stopped: `None` from the next frame.
    h.ctx_mut().play_voice(&lines[1]);
    h.wait(0.1);
    assert_eq!(h.ctx().voice_playing(), Some(&lines[1]));
    h.ctx_mut().stop_voice();
    h.frame(&[], FRAME_DT);
    assert_eq!(h.ctx().voice_playing(), Some(&lines[1]), "a frame behind");
    h.frame(&[], FRAME_DT);
    assert_eq!(h.ctx().voice_playing(), None, "stopped");
}

#[test]
fn the_gains_and_fullscreen_are_what_the_last_frame_said() {
    let mut h = beeper();
    // Before any frame: no group heard of yet.
    assert_eq!(h.gain(MUSIC), Gain::SILENT);
    assert!(!h.fullscreen());
    h.ctx_mut().preview_volume(MUSIC, MAX_VOLUME / 2);
    h.ctx_mut().preview_volume(SFX, 0);
    h.ctx_mut()
        .change_settings(|s| s.fullscreen = true)
        .unwrap_or_else(|e| panic!("{e}"));
    // Not told until a frame runs.
    assert!(!h.fullscreen());
    h.wait(0.0);
    assert_eq!(h.gain(MUSIC), loudness(MAX_VOLUME / 2));
    // The menus' sounds are under the sounds'.
    assert_eq!((h.gain(SFX), h.gain(UI)), (Gain::SILENT, Gain::SILENT));
    assert_eq!(h.gain(VOICE), loudness(97));
    assert!(h.fullscreen());
}

/// Acceptance: the master volume at 0 silences every group from the next
/// frame, and Restore defaults brings back the defaults' gains.
#[test]
fn master_at_zero_silences_every_group_and_the_defaults_come_back() {
    let mut h = beeper();
    h.wait(0.0);
    let groups = [MASTER, MUSIC, SFX, VOICE, UI];
    let defaults = groups.map(|g| h.gain(g));
    assert_eq!(defaults, [4096, 3329, 3329, 3329, 3329].map(Gain::new));
    h.ctx_mut().preview_volume(MASTER, 0);
    assert_eq!(
        h.gain(MUSIC),
        Gain::new(3329),
        "not told until a frame runs"
    );
    h.wait(0.0);
    assert_eq!(groups.map(|g| h.gain(g)), [Gain::SILENT; 5]);
    h.ctx_mut().preview_settings(|s| *s = s.restored());
    h.wait(0.0);
    assert_eq!(groups.map(|g| h.gain(g)), defaults);
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

/// The frames that stopped `cue`.
fn stops(h: &Harness<Toy>, cue: &str) -> Vec<u32> {
    let log = h.music_log().into_iter();
    log.filter_map(|(frame, command)| match command {
        MusicCommand::Stop { cue: c } => (c == cue).then_some(frame),
        _ => None,
    })
    .collect()
}

/// The gain `gains` gave in `frame`.
fn gain_at(gains: &[(u32, f32)], frame: u32) -> f32 {
    let found = gains.iter().find(|(f, _)| *f == frame);
    found.map_or(f32::NAN, |(_, gain)| *gain)
}

/// A screen's stop of 3 s, with the toy's default fade of 0.5 s: two
/// thirds of the gain after one second, one third after two, and one stop
/// at three. The default fade is the next stop's again.
#[test]
fn a_stop_with_its_own_fade_takes_that_long() {
    let mut h = beeper();
    h.act(Action::Cancel).wait(1.0).clear_audio();
    h.act(Action::Select);
    let asked = h.frame_count();
    assert_eq!(
        h.audio_log(),
        [(asked, &AudioRequest::stop_music_with(3000))]
    );
    h.wait(3.1);
    // The frame of the request already moves the fade one frame's time.
    // `FRAME_DT` is a sixtieth of a second.
    let per_second = 60;
    assert!((FRAME_DT * 60.0 - 1.0).abs() < 1e-6);
    let theme = gains(&h, "theme");
    let at = |seconds: u32| gain_at(&theme, asked + seconds * per_second - 1);
    assert!((at(1) - 2.0 / 3.0).abs() < 1e-3, "{theme:?}");
    assert!((at(2) - 1.0 / 3.0).abs() < 1e-3, "{theme:?}");
    let stopped = stops(&h, "theme");
    assert_eq!(stopped.len(), 1, "{stopped:?}");
    assert!(
        stopped[0].abs_diff(asked + 3 * per_second - 1) <= 1,
        "{stopped:?}"
    );
    let found: Vec<_> = h
        .music_log()
        .into_iter()
        .filter(|(_, c)| matches!(c, MusicCommand::Load { .. } | MusicCommand::Start { .. }))
        .collect();
    assert!(found.is_empty(), "{found:?}");
    // The next track's stop fades over the default 0.5 s.
    h.act(Action::Cancel).wait(1.0).clear_audio();
    h.act(Action::CursorDown);
    let asked = h.frame_count();
    h.wait(1.0);
    let stopped = stops(&h, "theme");
    assert_eq!(stopped.len(), 1, "{stopped:?}");
    assert!(
        stopped[0].abs_diff(asked + per_second / 2 - 1) <= 1,
        "{stopped:?}"
    );
}

/// The beeper, with the toy game's audio lowering `music` to 80 on the
/// curve while a voice plays, over 100 ms down and 500 ms back, and every
/// voice clip a second long.
fn ducker() -> Harness<Toy> {
    let mut ctx = toy::ctx();
    let mut rule = DuckRule::default();
    rule.when = VOICE.into();
    rule.lower = MUSIC.into();
    rule.to = 80;
    rule.attack_ms = 100;
    rule.release_ms = 500;
    ctx.game.audio.ducks.push(rule);
    let mut h = Harness::new(Game::new(ctx, Box::new(Beeper), GameHooks::default()));
    h.voice_length(1.0);
    let manifest = h.ctx().game.manifest(&[(0, "Only")]);
    h.ctx_mut().set_voice_manifest(&manifest);
    h
}

/// Acceptance (2343): a voice lowers the music by the rule once it has
/// played for the attack, and the music is back once the voice has ended
/// and the release has passed; the sounds never move.
#[test]
fn a_voice_lowers_the_music_as_the_manifests_rule_says() {
    let mut h = ducker();
    h.wait(0.0);
    let (music, sfx) = (h.gain(MUSIC), h.gain(SFX));
    assert_eq!(music, loudness(97));
    let line = h.ctx().game.lines()[0].clone();
    h.ctx_mut().play_voice(&line);
    h.frame(&[], FRAME_DT);
    assert_eq!(h.gain(MUSIC), music, "the platform hasn't read it");
    h.wait(0.1);
    assert_eq!(h.ctx().voice_playing(), Some(&line));
    assert_eq!(h.gain(MUSIC), music.times(loudness(80)));
    assert_eq!(h.gain(SFX), sfx);
    // The voice ends a second after it started, and the music comes back
    // over half a second.
    h.wait(0.85);
    assert_eq!(h.gain(MUSIC), music.times(loudness(80)), "still lowered");
    h.wait(0.1);
    assert_eq!(h.ctx().voice_playing(), None, "ended");
    assert!(h.gain(MUSIC) < music, "coming back");
    h.wait(0.5);
    assert_eq!(h.gain(MUSIC), music);
    assert_eq!(h.gain(SFX), sfx);
}

/// Acceptance (2343): a voice the player turned down to 0 lowers nothing.
#[test]
fn a_voice_turned_down_to_0_lowers_nothing() {
    let mut h = ducker();
    h.ctx_mut().preview_volume(VOICE, 0);
    h.wait(0.0);
    let music = h.gain(MUSIC);
    let line = h.ctx().game.lines()[0].clone();
    h.ctx_mut().play_voice(&line);
    for _ in 0..30 {
        h.frame(&[], FRAME_DT);
        assert_eq!(h.gain(MUSIC), music);
    }
    assert_eq!(h.ctx().voice_playing(), Some(&line), "it was playing");
}
