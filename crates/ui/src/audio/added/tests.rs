use super::*;
use crate::toy::Voice;
use shuyi_assets::audio::{CreditRef, SoundCue};

type AudioQueue = super::AudioQueue<Voice>;
type AudioRequest = super::AudioRequest<Voice>;

/// `frames` frames of silence at `rate`, mono or stereo.
fn silence(rate: u32, channels: u32, frames: usize) -> Samples {
    let samples = vec![0; frames * usize::try_from(channels).unwrap_or(1)];
    Samples::new(rate, channels, samples).unwrap_or_else(|e| panic!("{e}"))
}

/// What the requests are, in short: each kind and its cue.
fn kinds(requests: &[AudioRequest]) -> Vec<String> {
    requests
        .iter()
        .map(|r| match r {
            AudioRequest::AddSound { cue, .. } => format!("add {cue}"),
            AudioRequest::RemoveSound { cue } => format!("remove {cue}"),
            AudioRequest::PlaySound { cue, .. } => format!("play {cue}"),
            AudioRequest::StartSound { cue, .. } => format!("start {cue}"),
            AudioRequest::StopSound { .. } => "stop".to_owned(),
            other => format!("{other:?}"),
        })
        .collect()
}

#[test]
fn an_added_sound_is_a_request_and_its_plays_follow_it_in_order() {
    let mut q = AudioQueue::default();
    let tone = silence(11_025, 1, 1102);
    assert_eq!(q.add_sound("tone", tone.clone()), Ok(()));
    assert!(q.is_added("tone"));
    assert_eq!(q.added_bytes(), 2204);
    q.play_sound("tone");
    let id = q.start_sound("tone");
    q.stop_sound(id, 0);
    q.remove_sound("tone");
    let requests = q.take();
    assert_eq!(
        kinds(&requests),
        ["add tone", "play tone", "start tone", "stop", "remove tone"]
    );
    let AudioRequest::AddSound { samples, .. } = &requests[0] else {
        panic!("{requests:?}");
    };
    assert_eq!(**samples, tone);
    // Adding and removing play nothing.
    assert_eq!(requests[0].cue(), None);
    assert_eq!(requests[4].cue(), None);
    assert!(!q.is_added("tone"));
    assert_eq!(q.added_bytes(), 0);
    assert_eq!(q.take_problems(), Vec::<String>::new());
}

#[test]
fn adding_a_cue_again_replaces_it_and_its_bytes() {
    let mut q = AudioQueue::default();
    assert_eq!(q.add_sound("a", silence(8_000, 1, 100)), Ok(()));
    assert_eq!(q.add_sound("b", silence(8_000, 2, 10)), Ok(()));
    assert_eq!(q.added_bytes(), 200 + 40);
    assert_eq!(q.add_sound("a", silence(8_000, 1, 50)), Ok(()));
    assert_eq!(q.added_bytes(), 100 + 40);
    assert_eq!(kinds(&q.take()), ["add a", "add b", "add a"]);
}

#[test]
fn a_cue_of_the_manifest_is_refused_and_logged() {
    let mut manifest = AudioManifest::default();
    let click = SoundCue::new(vec!["click.wav".into()], 100, CreditRef::Own);
    manifest.sounds.insert("click".into(), click);
    let mut q = AudioQueue::default();
    q.set_manifest(&manifest);
    let refused = q.add_sound("click", silence(8_000, 1, 10));
    assert_eq!(refused, Err(AddSoundError::InManifest("click".into())));
    assert!(!q.is_added("click"));
    assert_eq!(q.added_bytes(), 0);
    let found = q.take();
    assert!(found.is_empty(), "{found:?}");
    let problems = q.take_problems();
    assert_eq!(problems.len(), 1, "{problems:?}");
    assert!(problems[0].contains("\"click\""), "{problems:?}");
    assert!(problems[0].contains("manifest"), "{problems:?}");
}

#[test]
fn the_limits_refuse_a_sound_too_long_and_samples_too_many() {
    let mut q = AudioQueue::default();
    // Sixty seconds is the longest: one frame more is refused.
    assert_eq!(q.add_sound("minute", silence(1_000, 1, 60_000)), Ok(()));
    let long = q.add_sound("long", silence(1_000, 1, 60_001));
    assert_eq!(
        long,
        Err(AddSoundError::TooLong {
            cue: "long".into(),
            millis: 60_001
        })
    );
    q.remove_sound("minute");
    // 32 MiB in all, two bytes a sample: 48 kHz stereo is 11.5 MiB a minute.
    let minute = || silence(48_000, 2, 48_000 * 60);
    assert_eq!(q.add_sound("one", minute()), Ok(()));
    assert_eq!(q.add_sound("two", minute()), Ok(()));
    let three = q.add_sound("three", minute());
    assert_eq!(
        three,
        Err(AddSoundError::TooMuch {
            cue: "three".into(),
            bytes: 3 * 11_520_000
        })
    );
    const { assert!(3 * 11_520_000 > ADDED_BYTES_MAX && 2 * 11_520_000 <= ADDED_BYTES_MAX) };
    // Replacing one counts it once, and a removal makes room.
    assert_eq!(q.add_sound("two", minute()), Ok(()));
    q.remove_sound("one");
    assert_eq!(q.add_sound("three", minute()), Ok(()));
    assert_eq!(q.added_bytes(), 2 * 11_520_000);
    let problems = q.take_problems();
    assert_eq!(problems.len(), 2, "{problems:?}");
    assert!(problems[0].contains("60001 milliseconds"), "{problems:?}");
    assert!(problems[1].contains("34560000 bytes"), "{problems:?}");
    assert_eq!(
        kinds(&q.take()),
        [
            "add minute",
            "remove minute",
            "add one",
            "add two",
            "add two",
            "remove one",
            "add three"
        ]
    );
}

#[test]
fn samples_of_exactly_the_limit_are_allowed() {
    let mut q = AudioQueue::default();
    // 8,388,608 frames at 192 kHz, mono: 16 MiB in 43.7 seconds. Two are the limit.
    let half = || silence(192_000, 1, 8_388_608);
    assert_eq!(q.add_sound("a", half()), Ok(()));
    assert_eq!(q.add_sound("b", half()), Ok(()));
    assert_eq!(q.added_bytes(), ADDED_BYTES_MAX);
    let more = q.add_sound("c", silence(192_000, 1, 1));
    assert_eq!(
        more,
        Err(AddSoundError::TooMuch {
            cue: "c".into(),
            bytes: ADDED_BYTES_MAX + 2
        })
    );
}

#[test]
fn a_removed_cue_plays_nothing_and_is_logged_once() {
    let mut q = AudioQueue::default();
    assert_eq!(q.add_sound("tone", silence(8_000, 1, 8)), Ok(()));
    q.remove_sound("tone");
    q.take();
    q.play_sound("tone");
    q.play_sound_at("tone", 0.5);
    let id = q.start_sound("tone");
    assert!(!q.sound_is_live(id));
    let found = q.take();
    assert!(found.is_empty(), "{found:?}");
    let problems = q.take_problems();
    assert_eq!(problems.len(), 1, "{problems:?}");
    assert!(problems[0].contains("\"tone\" was removed"), "{problems:?}");
    // Still removed, still logged once.
    q.play_sound("tone");
    let found = q.take();
    assert!(found.is_empty(), "{found:?}");
    let found = q.take_problems();
    assert!(found.is_empty(), "{found:?}");
    // Added again, it plays; removed again, its first play is logged again.
    assert_eq!(q.add_sound("tone", silence(8_000, 1, 8)), Ok(()));
    q.play_sound("tone");
    q.remove_sound("tone");
    q.play_sound("tone");
    assert_eq!(kinds(&q.take()), ["add tone", "play tone", "remove tone"]);
    assert_eq!(q.take_problems().len(), 1);
}

#[test]
fn removing_a_cue_never_added_is_logged() {
    let mut q = AudioQueue::default();
    q.remove_sound("nothing");
    let found = q.take();
    assert!(found.is_empty(), "{found:?}");
    let problems = q.take_problems();
    assert_eq!(problems.len(), 1, "{problems:?}");
    assert!(problems[0].contains("wasn't added"), "{problems:?}");
    // A cue never added plays as any cue does: the platform says if it's unknown.
    q.play_sound("nothing");
    assert_eq!(kinds(&q.take()), ["play nothing"]);
}

#[test]
fn the_cues_the_game_made_are_known_after_their_removal() {
    let mut q = AudioQueue::default();
    assert!(!q.knows_added("tone"));
    assert_eq!(q.add_sound("tone", silence(8_000, 1, 8)), Ok(()));
    assert!(q.knows_added("tone"));
    q.remove_sound("tone");
    assert!(q.knows_added("tone"));
}
