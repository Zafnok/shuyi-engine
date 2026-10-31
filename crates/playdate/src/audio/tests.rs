//! The Playdate's sound on the fake SDK: what the sample and file players are
//! told, and what they then sound like.

use alloc::borrow::ToOwned;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use shuyi_assets::audio::{AudioManifest, CreditRef, MusicCue, SoundCue};
use shuyi_platform::audio::player::{Backend, Player};
use shuyi_platform::{Audio, AudioFrame, Millis, SoundRequest};
use shuyi_ui::audio::{MusicCommand, SoundId};

use super::{SAMPLE_PLAYERS, Speaker, player};
use crate::fake::FakeSdk;

/// A sound `click` (volume 50), a looped `hum` and a `step` that shares
/// `hum`'s file at volume 100; the tracks `theme` (looped, volume 80), `sting`
/// (once) and `gone`, which isn't in the `.pdx`.
fn manifest() -> AudioManifest {
    let mut m = AudioManifest::default();
    let sound = |file: &str, volume| SoundCue::new(vec![file.to_owned()], volume, CreditRef::Own);
    m.sounds.insert("click".into(), sound("sfx/click.wav", 50));
    m.sounds
        .insert("hum".into(), sound("sfx/hum.ogg", 100).with_looped(true));
    m.sounds.insert("step".into(), sound("sfx/hum.ogg", 100));
    let track = |file: &str, volume, looped| MusicCue {
        file: file.into(),
        volume,
        looped,
        length_ms: 60_000,
        credit: CreditRef::Own,
    };
    m.music.insert("theme".into(), track("theme.ogg", 80, true));
    m.music
        .insert("sting".into(), track("sting.ogg", 100, false));
    m.music.insert("gone".into(), track("gone.ogg", 100, true));
    m
}

/// The fake SDK with the manifest's files, but for `gone`.
fn sdk() -> FakeSdk {
    let sdk = FakeSdk::default();
    sdk.put_sample("audio/sfx/click");
    sdk.put_sample("audio/sfx/hum");
    sdk.put_track("music/theme", 100.0);
    sdk.put_track("music/sting", 2.0);
    sdk
}

struct Rig {
    sdk: FakeSdk,
    speaker: Speaker<FakeSdk>,
    player: Player<Speaker<FakeSdk>>,
}

impl Rig {
    fn new() -> Self {
        let sdk = sdk();
        let mut speaker = Speaker::new(sdk.clone());
        let mut player = player(&mut speaker, manifest(), 7);
        assert_eq!(player.take_warnings(), [] as [String; 0]);
        Self {
            sdk,
            speaker,
            player,
        }
    }

    fn frame(&mut self, requests: &[SoundRequest], music: &[MusicCommand], now: f64) {
        self.player.play(&mut self.speaker, requests, music, now);
    }
}

fn sound(cue: &str, volume: f32) -> SoundRequest {
    SoundRequest::Sound {
        cue: cue.into(),
        volume,
    }
}

fn load(cue: &str) -> MusicCommand {
    MusicCommand::Load { cue: cue.into() }
}

fn start(cue: &str, gain: f32) -> MusicCommand {
    MusicCommand::start(cue, gain)
}

fn gain(cue: &str, gain: f32) -> MusicCommand {
    MusicCommand::Gain {
        cue: cue.into(),
        gain,
    }
}

fn stop(cue: &str) -> MusicCommand {
    MusicCommand::Stop { cue: cue.into() }
}

fn id(n: u32) -> SoundId {
    let mut queue = shuyi_ui::audio::AudioQueue::<shuyi_ui::toy::Voice>::default();
    let mut last = queue.start_sound("hum");
    for _ in 0..n {
        last = queue.start_sound("hum");
    }
    last
}

#[test]
fn a_cue_plays_at_its_volume_times_the_request_and_the_players() {
    let mut rig = Rig::new();
    rig.frame(&[sound("click", 0.5)], &[], 0.0);
    assert_eq!(
        rig.sdk.sounding_samples(),
        [("audio/sfx/click".to_owned(), 0.25)]
    );
    rig.player.set_volumes(&mut rig.speaker, 1.0, 0.5);
    rig.frame(&[sound("click", 1.0)], &[], 0.0);
    assert_eq!(
        rig.sdk.sounding_samples(),
        [
            ("audio/sfx/click".to_owned(), 0.25),
            ("audio/sfx/click".to_owned(), 0.25)
        ]
    );
}

#[test]
fn cues_that_share_a_file_load_it_once() {
    let rig = Rig::new();
    assert_eq!(
        rig.sdk.samples_loaded(),
        ["audio/sfx/click", "audio/sfx/hum"]
    );
}

#[test]
fn a_sound_the_pdx_lacks_is_a_warning_naming_its_path() {
    let sdk = FakeSdk::default();
    let mut speaker = Speaker::new(sdk);
    let mut player = player(&mut speaker, manifest(), 1);
    let warnings = player.take_warnings();
    assert!(
        warnings.contains(&"sound \"click\": audio/sfx/click: not in the .pdx".to_owned()),
        "{warnings:?}"
    );
}

#[test]
fn a_sound_takes_a_free_player_else_the_next_in_turn() {
    let mut rig = Rig::new();
    let many: Vec<SoundRequest> = (0..SAMPLE_PLAYERS).map(|_| sound("click", 1.0)).collect();
    rig.frame(&many, &[], 0.0);
    assert_eq!(rig.sdk.sounding_samples().len(), SAMPLE_PLAYERS);
    rig.frame(&[sound("step", 1.0)], &[], 0.0);
    let sounding = rig.sdk.sounding_samples();
    assert_eq!(sounding.len(), SAMPLE_PLAYERS, "one was taken");
    assert_eq!(sounding[0].0, "audio/sfx/hum");
    rig.sdk.finish_samples();
    rig.frame(&[sound("step", 1.0)], &[], 0.0);
    assert_eq!(rig.sdk.sounding_samples().len(), 1);
}

#[test]
fn a_handles_sample_player_stops_alone_and_an_old_handle_does_nothing() {
    let mut rig = Rig::new();
    let (first, second) = (id(0), id(1));
    rig.frame(
        &[
            SoundRequest::start(first, "hum", 1.0),
            SoundRequest::start(second, "hum", 0.5),
        ],
        &[],
        0.0,
    );
    assert_eq!(rig.sdk.sounding_samples().len(), 2);
    rig.frame(&[SoundRequest::Stop { id: first }], &[], 0.0);
    assert_eq!(
        rig.sdk.sounding_samples(),
        [("audio/sfx/hum".to_owned(), 0.5)]
    );
    // A volume reaches that one alone.
    rig.frame(
        &[SoundRequest::Volume {
            id: second,
            volume: 0.25,
        }],
        &[],
        0.0,
    );
    assert_eq!(
        rig.sdk.sounding_samples(),
        [("audio/sfx/hum".to_owned(), 0.25)]
    );
    // An old playback's handle: its player went to another sound.
    let old = rig.speaker.play(
        &super::Clip::Sample {
            sample: rig.sdk.sample_id("audio/sfx/click"),
            rate: core::cell::Cell::new(super::RATE_ONE),
        },
        1.0,
        false,
    );
    rig.sdk.finish_samples();
    rig.frame(&[sound("click", 1.0)], &[], 0.0);
    rig.speaker.stop_playing(old);
    assert_eq!(
        rig.sdk.sounding_samples().len(),
        1,
        "the new sound plays on"
    );
}

#[test]
fn a_pause_under_a_menu_pauses_the_sample_player_and_mutes_nothing() {
    let mut rig = Rig::new();
    let hum = id(0);
    rig.frame(&[SoundRequest::start(hum, "hum", 1.0)], &[], 0.0);
    rig.frame(&[SoundRequest::PauseSounds], &[], 1.0);
    assert_eq!(rig.sdk.sounding_samples(), [] as [(String, f32); 0]);
    assert_eq!(rig.sdk.paused_samples(), 1);
    // A paused player isn't taken for another sound.
    let many: Vec<SoundRequest> = (0..SAMPLE_PLAYERS * 2)
        .map(|_| sound("click", 1.0))
        .collect();
    rig.frame(&many, &[], 1.0);
    assert_eq!(rig.sdk.paused_samples(), 1);
    rig.frame(&[SoundRequest::ResumeSounds], &[], 2.0);
    assert_eq!(rig.sdk.paused_samples(), 0);
    assert_eq!(rig.player.take_warnings(), [] as [String; 0], "no fallback");
    let sounding = rig.sdk.sounding_samples();
    assert!(
        sounding.contains(&("audio/sfx/hum".to_owned(), 1.0)),
        "{sounding:?}"
    );
}

#[test]
fn a_switch_opens_the_next_track_before_closing_the_first() {
    let mut rig = Rig::new();
    rig.frame(&[], &[load("theme"), start("theme", 1.0)], 0.0);
    assert_eq!(rig.sdk.sounding_files(), [("music/theme".to_owned(), 0.8)]);
    rig.sdk.take_file_player_calls();
    rig.frame(&[], &[load("sting"), gain("theme", 0.5)], 1.0);
    rig.frame(&[], &[stop("theme"), start("sting", 1.0)], 2.0);
    assert_eq!(
        rig.sdk.take_file_player_calls(),
        ["open music/sting", "close music/theme", "play music/sting"]
    );
    assert_eq!(rig.sdk.sounding_files(), [("music/sting".to_owned(), 1.0)]);
}

/// A crossfade, as the music state sends it: a second file player, both
/// sounding, their volumes moving opposite ways.
#[test]
fn a_crossfade_has_both_file_players_sounding_their_volumes_moving_opposite_ways() {
    let mut rig = Rig::new();
    rig.frame(&[], &[load("theme"), start("theme", 1.0)], 0.0);
    rig.frame(&[], &[load("sting"), start("sting", 0.0)], 1.0);
    let at = |rig: &Rig| rig.sdk.sounding_files();
    assert_eq!(
        at(&rig),
        [
            ("music/theme".to_owned(), 0.8),
            ("music/sting".to_owned(), 0.0)
        ]
    );
    rig.frame(&[], &[gain("theme", 0.5), gain("sting", 0.5)], 1.5);
    assert_eq!(
        at(&rig),
        [
            ("music/theme".to_owned(), 0.4),
            ("music/sting".to_owned(), 0.5)
        ]
    );
    rig.frame(&[], &[gain("theme", 0.25), gain("sting", 0.75)], 1.75);
    assert_eq!(
        at(&rig),
        [
            ("music/theme".to_owned(), 0.2),
            ("music/sting".to_owned(), 0.75)
        ]
    );
    rig.frame(&[], &[stop("theme"), gain("sting", 1.0)], 2.0);
    assert_eq!(at(&rig), [("music/sting".to_owned(), 1.0)]);
    assert_eq!(
        rig.player.music_playing(2.0).map(|(cue, _)| cue),
        Some("sting")
    );
}

#[test]
fn a_track_the_game_does_not_ship_is_silence_with_a_warning() {
    let mut rig = Rig::new();
    rig.frame(&[], &[load("gone"), start("gone", 1.0)], 0.0);
    rig.frame(&[], &[gain("gone", 0.5)], 1.0);
    assert_eq!(rig.sdk.sounding_files(), [] as [(String, f32); 0]);
    assert_eq!(rig.player.music_playing(1.0), None);
    assert_eq!(
        rig.player.take_warnings(),
        ["music \"gone\": music/gone.ogg: not in the .pdx"]
    );
}

/// The player's clock against what the file player has really played: the
/// fake moves a file on only while it plays.
#[test]
fn the_music_clock_follows_the_file_player_through_a_suspend() {
    let mut rig = Rig::new();
    rig.frame(&[], &[load("theme"), start("theme", 1.0)], 10.0);
    let same = |rig: &Rig, now: f64| {
        let (cue, clock) = rig.player.music_playing(now).unwrap_or(("", -1.0));
        assert_eq!(cue, "theme");
        let file = f64::from(rig.sdk.file_offset("music/theme"));
        assert!((clock - file).abs() < 1e-3, "{clock} against {file}");
    };
    same(&rig, 10.0);
    rig.sdk.advance_files(2.5);
    same(&rig, 12.5);
    rig.frame(&[], &[], 12.5);
    rig.player.suspend(&mut rig.speaker);
    // The system holds the game for a minute: the file stands still.
    rig.sdk.advance_files(60.0);
    same(&rig, 72.5);
    rig.player.resume(&mut rig.speaker);
    same(&rig, 72.5);
    rig.frame(&[], &[], 72.5);
    same(&rig, 72.5);
    rig.sdk.advance_files(1.0);
    same(&rig, 73.5);
}

#[test]
fn a_suspend_pauses_every_player_and_the_resume_goes_on_where_they_were() {
    let mut rig = Rig::new();
    let hum = id(0);
    rig.frame(
        &[SoundRequest::start(hum, "hum", 1.0)],
        &[load("theme"), start("theme", 1.0)],
        0.0,
    );
    rig.sdk.advance_files(3.0);
    rig.frame(&[], &[], 3.0);
    rig.player.suspend(&mut rig.speaker);
    assert_eq!(rig.sdk.sounding_samples(), [] as [(String, f32); 0]);
    assert_eq!(rig.sdk.sounding_files(), [] as [(String, f32); 0]);
    assert_eq!(rig.sdk.paused_samples(), 1);
    rig.sdk.advance_files(5.0);
    assert!((rig.sdk.file_offset("music/theme") - 3.0).abs() < 1e-3);
    rig.player.resume(&mut rig.speaker);
    assert_eq!(
        rig.sdk.sounding_samples(),
        [("audio/sfx/hum".to_owned(), 1.0)]
    );
    assert_eq!(rig.sdk.sounding_files(), [("music/theme".to_owned(), 0.8)]);
    assert!(
        (rig.sdk.file_offset("music/theme") - 3.0).abs() < 1e-3,
        "it goes on from where it stopped"
    );
    rig.sdk.advance_files(1.0);
    assert!((rig.sdk.file_offset("music/theme") - 4.0).abs() < 1e-3);
    assert_eq!(rig.player.take_warnings(), [] as [String; 0]);
}

/// A sound a menu paused stays so through a suspend and its resume.
#[test]
fn a_resume_leaves_a_menus_pause_in_place() {
    let mut rig = Rig::new();
    rig.frame(
        &[
            SoundRequest::start(id(0), "hum", 1.0),
            SoundRequest::PauseSounds,
        ],
        &[],
        0.0,
    );
    rig.player.suspend(&mut rig.speaker);
    rig.player.resume(&mut rig.speaker);
    assert_eq!(rig.sdk.paused_samples(), 1);
    rig.frame(&[SoundRequest::ResumeSounds], &[], 1.0);
    assert_eq!(rig.sdk.paused_samples(), 0);
}

/// A track played once that has ended has nothing to resume: it doesn't start
/// over.
#[test]
fn a_resume_does_not_restart_a_track_that_ended() {
    let mut rig = Rig::new();
    rig.frame(&[], &[load("sting"), start("sting", 1.0)], 0.0);
    rig.sdk.advance_files(3.0);
    assert_eq!(rig.sdk.sounding_files(), [] as [(String, f32); 0]);
    rig.player.suspend(&mut rig.speaker);
    rig.player.resume(&mut rig.speaker);
    assert_eq!(rig.sdk.sounding_files(), [] as [(String, f32); 0]);
}

#[test]
fn starting_a_track_again_plays_it_from_the_top() {
    let mut rig = Rig::new();
    rig.frame(&[], &[load("theme"), start("theme", 1.0)], 0.0);
    rig.sdk.advance_files(3.0);
    rig.frame(&[], &[start("theme", 1.0)], 3.0);
    assert!(rig.sdk.file_offset("music/theme").abs() < 1e-3);
    assert_eq!(rig.player.music_playing(3.5).map(|(_, at)| at), Some(0.5));
}

#[test]
fn a_rate_holds_from_the_next_play() {
    let mut rig = Rig::new();
    rig.frame(&[sound("click", 1.0)], &[], 0.0);
    assert_eq!(rig.sdk.last_sample_rate(), Some(1.0));
    rig.player.set_rate(&mut rig.speaker, "click", 3 << 15);
    rig.frame(&[sound("click", 1.0)], &[], 0.0);
    assert_eq!(rig.sdk.last_sample_rate(), Some(1.5));
    assert_eq!(rig.player.take_warnings(), [] as [String; 0]);
}

#[test]
fn the_sdk_knows_how_long_a_clip_plays() {
    let mut rig = Rig::new();
    rig.sdk.put_track("voice/line", 1.5);
    let mut loading = rig.speaker.load_music("voice/line.ogg");
    let Some(Ok(clip)) = rig.speaker.poll_music(&mut loading) else {
        panic!("the file is there");
    };
    assert_eq!(rig.speaker.length(&clip), Some(1.5));
    assert!(rig.speaker.poll_music(&mut loading).is_none(), "given once");
    drop(clip);
    assert_eq!(
        rig.sdk.take_file_player_calls().last().map(String::as_str),
        Some("close voice/line")
    );
    rig.sdk.finish_samples();
}

/// The bytes aren't how the Playdate loads a sound.
#[test]
fn a_sound_is_never_read_as_bytes() {
    let sdk = sdk();
    let mut speaker = Speaker::new(sdk);
    let loaded = super::ready_now(speaker.load_sound(b"RIFF"));
    assert!(loaded.is_err());
}

/// What the backend's [`Audio`] does for the frames `App` hands it, warnings
/// in the SDK's console.
#[test]
fn the_device_is_the_players_warnings_in_the_console() {
    let sdk = sdk();
    let mut device = shuyi_platform::audio::player::PlayerDevice::new(Speaker::new(sdk.clone()));
    let requests = [sound("nope", 1.0), sound("click", 1.0)];
    let frame = AudioFrame {
        requests: &requests,
        music: &[],
        music_volume: 1.0,
        sound_volume: 1.0,
        voice_volume: 1.0,
    };
    device.play(&frame, Millis(0));
    assert_eq!(
        sdk.sounding_samples(),
        [] as [(String, f32); 0],
        "not loaded"
    );
    let loaded = player(device.backend_mut(), manifest(), 1);
    device.set_player(loaded);
    device.play(&frame, Millis(0));
    assert_eq!(sdk.sounding_samples().len(), 1);
    assert_eq!(sdk.logs(), ["audio: unknown sound cue \"nope\""]);
}
