//! The headless backend: its stand-in for the music player, the frames it
//! keeps, its scripted input, storage, keyboard and clock. Games run on it
//! are tested in `tests/it/`.

use shuyi_ui::frame::glyph_buffer::Cell;
use shuyi_ui::input::{Chord, Key};

use super::*;
use crate::platform::Look;

fn command(cue: &str, command: fn(String) -> MusicCommand) -> MusicCommand {
    command(cue.into())
}

fn load(cue: String) -> MusicCommand {
    MusicCommand::Load { cue }
}

fn start(cue: String) -> MusicCommand {
    MusicCommand::start(cue, 1.0)
}

fn stop(cue: String) -> MusicCommand {
    MusicCommand::Stop { cue }
}

/// `speaker` given one frame of `music` at `now`.
fn play(speaker: &mut Speaker, music: &[MusicCommand], now: u64) {
    let frame = AudioFrame {
        requests: &[],
        music,
        music_volume: 1.0,
        sound_volume: 1.0,
        voice_volume: 1.0,
    };
    speaker.play(&frame, Millis(now));
}

#[test]
fn the_speaker_counts_from_a_start_to_its_stop() {
    let mut s = Speaker::default();
    assert_eq!(s.music_playing(Millis(0)), None);
    play(&mut s, &[command("a", load)], 1000);
    assert_eq!(s.music_playing(Millis(1000)), None, "loaded, not started");
    play(&mut s, &[command("a", start)], 1000);
    assert_eq!(s.music_playing(Millis(1000)), Some(("a", 0.0)));
    assert_eq!(s.music_playing(Millis(1750)), Some(("a", 0.75)));
    // A clock that went backwards counts no time.
    assert_eq!(s.music_playing(Millis(500)), Some(("a", 0.0)));
    // A fade and other tracks' commands leave it playing.
    let fade = MusicCommand::Gain {
        cue: "a".into(),
        gain: 0.5,
    };
    play(
        &mut s,
        &[fade, command("b", load), command("b", stop)],
        1750,
    );
    assert_eq!(s.music_playing(Millis(1750)), Some(("a", 0.75)));
    play(&mut s, &[command("a", stop)], 1750);
    assert_eq!(s.music_playing(Millis(2750)), None);
    // The next track counts from its own start; loading a track again
    // replaces it, as the platform does.
    play(&mut s, &[command("b", start)], 3000);
    assert_eq!(s.music_playing(Millis(3500)), Some(("b", 0.5)));
    play(&mut s, &[command("b", load)], 3500);
    assert_eq!(s.music_playing(Millis(3500)), None);
}

#[test]
fn the_speaker_sounds_after_its_load_delay_or_never() {
    let mut s = Speaker::default();
    s.set_load_delay(0.5);
    play(&mut s, &[command("a", start)], 0);
    assert_eq!(s.music_playing(Millis(0)), None);
    assert_eq!(s.music_playing(Millis(250)), None);
    assert_eq!(s.music_playing(Millis(500)), Some(("a", 0.0)));
    assert_eq!(s.music_playing(Millis(750)), Some(("a", 0.25)));
    for nonsense in [-1.0, f64::NAN] {
        s.set_load_delay(nonsense);
        assert_eq!(s.music_playing(Millis(250)), Some(("a", 0.25)));
    }
    s.set_silent();
    assert_eq!(s.music_playing(Millis(750)), None);
}

#[test]
fn the_platforms_speaker_is_the_one_it_plays_on() {
    let mut p = HeadlessPlatform::new();
    p.speaker_mut().set_load_delay(0.5);
    play(p.speaker_mut(), &[command("a", start)], 0);
    let audio = p.audio();
    assert_eq!(audio.music_playing(Millis(250)), None);
    assert_eq!(audio.music_playing(Millis(750)), Some(("a", 0.25)));
    p.speaker_mut().set_silent();
    assert_eq!(p.speaker().music_playing(Millis(750)), None);
}

#[test]
fn the_speaker_holds_the_sounds_started_until_they_stop() {
    let mut queue = shuyi_ui::audio::AudioQueue::<shuyi_ui::toy::Voice>::default();
    let [hum, buzz] = [queue.start_sound("hum"), queue.start_sound("buzz")];
    queue.stop_sound(hum, 0);
    let mut s = Speaker::default();
    let mut hear = |requests: &[SoundRequest]| {
        let frame = AudioFrame {
            requests,
            music: &[],
            music_volume: 1.0,
            sound_volume: 1.0,
            voice_volume: 1.0,
        };
        s.play(&frame, Millis(0));
        s.sounds_playing()
            .iter()
            .map(|p| (p.id, p.cue.clone()))
            .collect::<Vec<_>>()
    };
    let requests: Vec<SoundRequest> = queue
        .take()
        .iter()
        .filter_map(SoundRequest::from_request)
        .collect();
    // Heard out of order, they are kept in id order.
    let [start_hum, start_buzz] = [&requests[0], &requests[1]];
    assert_eq!(
        hear(&[start_buzz.clone(), start_hum.clone()]),
        [(hum, "hum".into()), (buzz, "buzz".into())]
    );
    // A start of an id already playing replaces it.
    assert_eq!(
        hear(&[SoundRequest::start(buzz, "rain", 1.0)]),
        [(hum, "hum".into()), (buzz, "rain".into())]
    );
    assert_eq!(hear(&requests[2..]), [(buzz, "rain".into())]);
    // A frame without them changes nothing, and quitting stops them.
    assert_eq!(hear(&[]).len(), 1);
    s.stop_all();
    let got = s.sounds_playing();
    assert!(got.is_empty(), "{got:?}");
}

#[test]
fn the_speaker_pauses_what_plays_and_mutes_on_suspend_with_its_clock_running() {
    let mut queue = shuyi_ui::audio::AudioQueue::<shuyi_ui::toy::Voice>::default();
    let [first, second] = [queue.start_sound("hum"), queue.start_sound("hum")];
    let mut s = Speaker::default();
    let mut hear = |requests: &[SoundRequest], music: &[MusicCommand]| {
        let frame = AudioFrame {
            requests,
            music,
            music_volume: 1.0,
            sound_volume: 1.0,
            voice_volume: 1.0,
        };
        s.play(&frame, Millis(0));
        s.sounds_playing()
            .iter()
            .map(|p| (p.id.get(), p.paused))
            .collect::<Vec<_>>()
    };
    let started = hear(
        &[
            SoundRequest::start(first, "hum", 1.0),
            SoundRequest::PauseSounds,
            SoundRequest::start(second, "hum", 1.0),
        ],
        &[command("a", start)],
    );
    assert_eq!(started, [(1, true), (2, false)]);
    assert_eq!(
        hear(&[SoundRequest::ResumeSounds], &[]),
        [(1, false), (2, false)]
    );
    assert!(!s.suspended());
    s.suspend();
    s.suspend();
    assert!(s.suspended());
    // It mutes: the track's clock runs on.
    assert_eq!(s.music_playing(Millis(500)), Some(("a", 0.5)));
    s.resume();
    s.resume();
    assert!(!s.suspended());
}

#[test]
fn the_speaker_keeps_the_last_frame_and_stops_on_quit() {
    let mut s = Speaker::default();
    assert_eq!(s.volumes(), (1.0, 1.0, 1.0));
    let click = SoundRequest::Sound {
        cue: "click".into(),
        volume: 1.0,
    };
    let requests = [click];
    let music = [command("a", start)];
    let frame = AudioFrame {
        requests: &requests,
        music: &music,
        music_volume: 0.5,
        sound_volume: 0.25,
        voice_volume: 0.75,
    };
    s.play(&frame, Millis(0));
    assert_eq!(s.requests(), requests);
    assert_eq!(s.music(), music);
    assert_eq!(s.volumes(), (0.5, 0.25, 0.75));
    play(&mut s, &[], 10);
    let found = s.requests();
    assert!(found.is_empty(), "{found:?}");
    let found = s.music();
    assert!(found.is_empty(), "{found:?}");
    assert!(!s.stopped());
    s.stop_all();
    assert!(s.stopped());
    assert_eq!(s.music_playing(Millis(20)), None);
}

fn buffer(glyph: char) -> GlyphBuffer {
    let cell = Cell::new(glyph, Rgb::new(255, 255, 255), Rgb::new(0, 0, 0));
    GlyphBuffer::new(4, 2, cell)
}

#[test]
fn the_platform_keeps_the_last_frame_it_was_shown() {
    let mut p = HeadlessPlatform::new();
    assert_eq!((p.presented(), p.last_frame()), (0, None));
    let card = Card::text("A game");
    p.present(&Frame::new(Show::Card(&card)));
    assert_eq!(p.last_frame(), Some(&Presented::Card(card)));
    assert_eq!(p.last_frame().and_then(Presented::buffer), None);
    let (a, b) = (buffer('a'), buffer('b'));
    let black = Rgb::new(0, 0, 0);
    let mut frame = Frame::new(Show::Game {
        buffer: &a,
        background: black,
        look: None,
    });
    p.present(&frame);
    assert_eq!(p.last_frame().and_then(Presented::buffer), Some(&a));
    frame.show = Show::Game {
        buffer: &b,
        background: Rgb::new(1, 2, 3),
        look: None,
    };
    frame.fullscreen = true;
    p.present(&frame);
    let kept = Presented::Game {
        buffer: b,
        background: Rgb::new(1, 2, 3),
        fullscreen: true,
    };
    assert_eq!(p.last_frame(), Some(&kept));
    p.present(&Frame::new(Show::Errors("no font")));
    assert_eq!(p.last_frame(), Some(&Presented::Errors("no font".into())));
    assert_eq!(p.presented(), 4);
}

#[test]
fn a_frame_with_its_look_is_kept_as_without() {
    let mut p = HeadlessPlatform::new();
    let font = shuyi_assets::font::FontAtlasDef::default();
    let images = shuyi_assets::image::ImageTable::default();
    let a = buffer('a');
    p.present(&Frame::new(Show::Game {
        buffer: &a,
        background: Rgb::new(0, 0, 0),
        look: Some(Look {
            font: &font,
            images: &images,
        }),
    }));
    assert_eq!(p.last_frame().and_then(Presented::buffer), Some(&a));
}

#[test]
fn input_comes_one_queued_poll_at_a_time_and_events_all_at_once() {
    let mut p = HeadlessPlatform::new();
    let enter = Chord::plain(Key::Enter);
    p.push_input(&[RawInputEvent::Down(enter)]);
    p.push_input(&[RawInputEvent::Up(Key::Enter)]);
    p.push_system(SystemEvent::Suspend);
    p.push_system(SystemEvent::Resume);
    let mut input = Vec::new();
    p.poll_input(&mut input);
    assert_eq!(input, [RawInputEvent::Down(enter)]);
    p.poll_input(&mut input);
    assert_eq!(
        input,
        [RawInputEvent::Down(enter), RawInputEvent::Up(Key::Enter)]
    );
    let mut events = Vec::new();
    p.system_events(&mut events);
    p.system_events(&mut events);
    assert_eq!(events, [SystemEvent::Suspend, SystemEvent::Resume]);
    input.clear();
    p.poll_input(&mut input);
    assert!(input.is_empty(), "{input:?}");
}

#[test]
fn the_storage_is_handed_over_once_then_a_fresh_one() {
    let mut saved = MemoryStorage::new();
    saved.write("slot", "one").unwrap_or_else(|e| panic!("{e}"));
    let mut p = HeadlessPlatform::new().with_storage(Box::new(saved));
    let first = p.storage();
    assert_eq!(first.read("slot"), Ok(Some("one".into())));
    assert_eq!(p.storage().read("slot"), Ok(None));
    assert_eq!(p.display(), HeadlessPlatform::DISPLAY);
    let small = DisplayInfo::new(400, 240).one_bit();
    assert_eq!(HeadlessPlatform::new().with_display(small).display(), small);
}

#[test]
fn the_scripted_keyboard_answers_each_opening_in_turn() {
    let ask = |text: &str| TextAsk {
        prompt: "Name".into(),
        text: text.into(),
        max_chars: 8,
    };
    let keyboard = ScriptedKeyboard::new([TextAnswer::Typed("Ada".into())]);
    let mut p = HeadlessPlatform::new().with_keyboard(keyboard);
    let input = p.text_input().unwrap_or_else(|| panic!("no keyboard"));
    assert_eq!(input.poll(), None);
    assert!(input.open(&ask("")));
    // One at a time.
    assert!(!input.open(&ask("x")));
    assert_eq!(input.poll(), Some(TextAnswer::Typed("Ada".into())));
    assert_eq!(input.poll(), None);
    // Out of answers: it cancels.
    assert!(input.open(&ask("Ada")));
    assert_eq!(input.poll(), Some(TextAnswer::Cancelled));
    let asks = p.keyboard().map(ScriptedKeyboard::asks);
    assert_eq!(asks, Some(&[ask(""), ask("Ada")][..]));
    assert!(HeadlessPlatform::new().text_input().is_none());
}

#[test]
fn it_gives_the_clock_a_test_sets_and_none_by_default() {
    let mut p = HeadlessPlatform::new();
    assert!(p.clock().is_none());
    p.set_clock(Some(PerfClock::Real(|| 7)));
    let read = p.clock().map(|mut clock| clock.read());
    assert_eq!(read, Some(7));
    p.set_clock(None);
    assert!(p.clock().is_none());
    let mut fake = HeadlessPlatform::new().with_clock(PerfClock::fake());
    assert_eq!(fake.clock().map(|mut clock| clock.read()), Some(100));
    fake.set_clock(None);
    assert!(fake.clock().is_none());
}

#[test]
fn the_fake_memory_moves_by_fixed_amounts_with_the_frames_presented() {
    let at = |n: u32| {
        let heap = fake_heap(n);
        (heap.live_bytes, heap.peak_bytes, heap.allocs)
    };
    assert_eq!(at(0), (1_048_576, 1_048_576, 0));
    assert_eq!(at(1), (1_064_960, 1_064_960, 1));
    assert_eq!(at(29), (1_523_712, 1_523_712, 1));
    assert_eq!(at(30), (1_048_576, 1_523_712, 2));
}

#[test]
fn the_platform_reads_its_memory_and_counts_the_calls() {
    let mut platform = HeadlessPlatform::new();
    assert!(!platform.has_memory());
    assert_eq!(platform.memory(), None);
    let mut platform = platform.with_fake_memory();
    assert!(platform.has_memory());
    let card = Card::text("Loading");
    platform.present(&Frame::new(Show::Card(&card)));
    assert_eq!(platform.memory(), Some(MemoryReading::new(fake_heap(1))));
    platform.set_memory(Some(|| HeapCounts::new(7, 8, 9)));
    assert_eq!(
        platform.memory().map(|m| m.heap),
        Some(HeapCounts::new(7, 8, 9))
    );
    platform.set_memory(None);
    assert!(!platform.has_memory());
    assert_eq!(platform.memory(), None);
    assert_eq!(platform.memory_calls(), 4);
}

#[test]
fn the_memory_tags_are_a_tests_until_it_takes_them_away() {
    let scopes = MemoryScopes::new(|_, _| 1, |_| 0, |_| {});
    let mut platform = HeadlessPlatform::new();
    assert!(platform.memory_scopes().is_none(), "none of its own");
    platform.set_scopes(Some(scopes));
    assert!(platform.memory_scopes().is_some());
    platform.set_scopes(None);
    assert!(platform.memory_scopes().is_none());
    let given = HeadlessPlatform::new().with_scopes(scopes);
    assert!(given.memory_scopes().is_some());
}

/// The fake snapshot's rows after `n` frames: each name with its live
/// bytes, blocks and peak, in the snapshot's order.
fn fake_rows(n: u32) -> Vec<(&'static str, u64, u32, u64)> {
    let snapshot = fake_snapshot(n);
    let names = ["image", "load", "update", "other"];
    snapshot
        .tags
        .iter()
        .map(|r| {
            assert_eq!(r.allocated_bytes, r.peak_bytes, "{r:?}");
            let name = names.into_iter().find(|&name| name == r.name).unwrap();
            (name, r.live_bytes, r.live_blocks, r.peak_bytes)
        })
        .collect()
}

#[test]
fn the_fake_snapshots_rows_add_up_to_the_live_bytes_of_the_fake_memory() {
    assert_eq!(
        fake_rows(0),
        [
            ("load", 524_288, 40, 524_288),
            ("other", 475_136, 12, 475_136),
            ("image", 49_152, 3, 49_152),
        ]
    );
    assert_eq!(fake_rows(1)[3], ("update", 16_384, 1, 16_384));
    assert_eq!(fake_rows(29)[2], ("update", 475_136, 29, 475_136));
    assert_eq!(fake_rows(30)[3], ("update", 0, 0, 475_136));
    for n in [0, 1, 29, 30, 31, 600] {
        let (snapshot, heap) = (fake_snapshot(n), fake_heap(n));
        let sum: u64 = snapshot.tags.iter().map(|r| r.live_bytes).sum();
        assert_eq!(sum, heap.live_bytes, "frame {n}");
        assert_eq!(snapshot.live_bytes, heap.live_bytes);
        assert_eq!(snapshot.peak_bytes, heap.peak_bytes);
        assert_eq!((snapshot.frame, snapshot.overflowed), (0, 0));
    }
    let classes: Vec<TagClass> = fake_snapshot(1).tags.iter().map(|r| r.class).collect();
    assert_eq!(
        classes,
        [
            TagClass::Phase,
            TagClass::Other,
            TagClass::Asset,
            TagClass::Phase
        ]
    );
}

#[test]
fn the_platform_takes_a_snapshot_from_its_fake_or_a_counter_set() {
    let mut platform = HeadlessPlatform::new();
    assert_eq!(platform.heap_snapshot(), None);
    platform.set_heap_snapshots(Some(|| HeapSnapshot::new(5, 6, Vec::new())));
    assert_eq!(platform.heap_snapshot().map(|s| s.live_bytes), Some(5));
    let mut fake = platform.with_fake_memory();
    let card = Card::text("Loading");
    fake.present(&Frame::new(Show::Card(&card)));
    assert_eq!(fake.heap_snapshot(), Some(fake_snapshot(1)));
    fake.set_heap_snapshots(None);
    fake.set_memory(None);
    assert_eq!(fake.heap_snapshot(), None);
}
