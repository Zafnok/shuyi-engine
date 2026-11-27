use alloc::vec;

use proptest::prelude::*;
use shuyi_assets::audio::{CreditRef, SoundCue};
use shuyi_assets::dialogue::LineId;
use shuyi_ui::toy::Voice;

use super::*;

type Request = AudioRequest<Voice>;

/// A cue of `n` files at `volume` percent, played by `rules`.
fn cue(n: usize, volume: u8, rules: CueRules) -> SoundCue {
    let files = (0..n).map(|i| format!("sfx/{i}.wav")).collect();
    SoundCue::new(files, volume, CreditRef::Own).with_rules(rules)
}

/// A manifest of `cues`, by id.
fn manifest(cues: &[(&str, SoundCue)]) -> AudioManifest {
    let mut m = AudioManifest::default();
    for (id, cue) in cues {
        m.sounds.insert((*id).to_owned(), cue.clone());
    }
    m
}

fn play(cue: &str) -> Request {
    Request::play_sound(cue, 1.0)
}

/// What `cues` makes of `asked` at `now`: the requests and the drops.
fn apply(
    cues: &mut Cues,
    m: &AudioManifest,
    now: u64,
    asked: &[Request],
) -> (Vec<SoundRequest>, Vec<DroppedSound>) {
    let (mut out, mut dropped) = (Vec::new(), Vec::new());
    cues.apply(m, Millis(now), asked, &mut out, &mut dropped);
    (out, dropped)
}

/// The file and volume of each sound in `out`.
fn picks(out: &[SoundRequest]) -> Vec<(usize, u8)> {
    out.iter()
        .filter_map(|r| match r {
            SoundRequest::Sound {
                variant,
                cue_volume,
                ..
            } => Some((*variant, *cue_volume)),
            _ => None,
        })
        .collect()
}

/// The files `n` plays of `pick` on a cue of `files` play, one a frame.
fn files_played(pick: Pick, files: usize, n: usize, seed: u64) -> Vec<usize> {
    let m = manifest(&[("step", cue(files, 100, CueRules::DEFAULT.with_pick(pick)))]);
    let mut cues = Cues::new(seed);
    let mut played = Vec::new();
    for frame in 0..n {
        let (out, _) = apply(&mut cues, &m, frame as u64 * 16, &[play("step")]);
        played.extend(picks(&out).into_iter().map(|(file, _)| file));
    }
    played
}

#[test]
fn with_the_default_rules_every_request_passes_in_order_at_its_cues_volume() {
    let m = manifest(&[
        ("click", cue(1, 70, CueRules::DEFAULT)),
        ("step", cue(3, 60, CueRules::DEFAULT)),
    ]);
    let mut queue = shuyi_ui::audio::AudioQueue::<Voice>::default();
    queue.play_sound_at("click", 0.5);
    queue.play_voice(&LineId::new("intro.hello"), Voice::Only);
    let hum = queue.start_sound("step");
    queue.play_sound("step");
    queue.play_sound("click");
    queue.stop_sound(hum, 0);
    queue.play_music("theme");
    let asked = queue.take();
    let (out, dropped) = apply(&mut Cues::new(1), &m, 0, &asked);
    assert_eq!(dropped, []);
    // As before, each sound at its cue's volume, of a file the cue has.
    let own = |r: &SoundRequest| match r {
        SoundRequest::Sound { cue, .. } | SoundRequest::Start { cue, .. } => {
            m.sounds.get(cue).map_or(0, |d| d.volume)
        }
        _ => 0,
    };
    let file = |r: &SoundRequest| match r {
        SoundRequest::Sound { variant, .. } | SoundRequest::Start { variant, .. } => *variant,
        _ => 0,
    };
    let copy = |r: &SoundRequest| match r {
        SoundRequest::Sound { copy, .. } => *copy,
        _ => CopyId::NONE,
    };
    let want: Vec<_> = (asked.iter().filter_map(SoundRequest::from_request))
        .zip(&out)
        .map(|(r, got)| r.clone().with_pick(file(got), own(&r)).with_copy(copy(got)))
        .collect();
    assert_eq!(out, want);
    assert_eq!(out.len(), 6, "the music's is a command: {out:?}");
    assert!(out.iter().all(|r| file(r) < 3));
}

#[test]
fn in_order_plays_the_files_in_turn_and_a_cue_of_one_file_plays_it_each_time() {
    assert_eq!(files_played(Pick::InOrder, 3, 7, 5), [0, 1, 2, 0, 1, 2, 0]);
    for pick in [Pick::Random, Pick::NoRepeat, Pick::Shuffle, Pick::InOrder] {
        assert_eq!(files_played(pick, 1, 4, 5), [0, 0, 0, 0], "{pick:?}");
    }
}

/// A cue whose files shrank starts its order again from the first: its
/// last file is past the end.
#[test]
fn in_order_starts_again_when_its_last_file_is_gone() {
    let rules = CueRules::DEFAULT.with_pick(Pick::InOrder);
    let (three, two) = (
        manifest(&[("step", cue(3, 50, rules))]),
        manifest(&[("step", cue(2, 50, rules))]),
    );
    let mut cues = Cues::new(1);
    let steps = [play("step"), play("step"), play("step")];
    let (out, _) = apply(&mut cues, &three, 0, &steps);
    assert_eq!(
        picks(&out).iter().map(|p| p.0).collect::<Vec<_>>(),
        [0, 1, 2]
    );
    let (out, _) = apply(&mut cues, &two, 0, &steps);
    assert_eq!(
        picks(&out).iter().map(|p| p.0).collect::<Vec<_>>(),
        [0, 1, 0]
    );
}

/// A shuffle deals every order of its files, each as likely.
#[test]
fn a_shuffle_deals_every_order() {
    let mut rng = VariantRng::new(4);
    let mut played = Played::default();
    let mut seen = alloc::collections::BTreeSet::new();
    for _ in 0..300 {
        played.deal(3, None, &mut rng);
        seen.insert(played.order.clone());
    }
    assert_eq!(seen.len(), 6, "{seen:?}");
}

/// Each volume from the lowest to the cue's is picked, the ends among them.
#[test]
fn every_volume_in_the_range_is_picked() {
    let m = manifest(&[(
        "hit",
        cue(1, 47, CueRules::DEFAULT.with_volume_min(Some(45))),
    )]);
    let asked: Vec<_> = (0..60).map(|_| play("hit")).collect();
    let (out, _) = apply(&mut Cues::new(2), &m, 0, &asked);
    let mut levels: Vec<u8> = picks(&out).iter().map(|p| p.1).collect();
    levels.sort_unstable();
    levels.dedup();
    assert_eq!(levels, [45, 46, 47]);
}

#[test]
fn random_plays_every_file_and_may_repeat_one() {
    let played = files_played(Pick::Random, 3, 60, 9);
    for file in 0..3 {
        assert!(played.contains(&file), "{file} never played: {played:?}");
    }
    assert!(
        played.windows(2).any(|w| w[0] == w[1]),
        "60 random picks of 3 repeat one: {played:?}"
    );
}

#[test]
fn a_gap_drops_the_plays_inside_it_in_one_frame_and_across_frames() {
    let spaced = CueRules::DEFAULT.with_min_gap_ms(40);
    let m = manifest(&[
        ("tick", cue(2, 80, spaced)),
        ("click", cue(1, 80, CueRules::DEFAULT)),
    ]);
    let mut cues = Cues::new(3);
    let gap = DroppedSound {
        cue: "tick".into(),
        why: DropWhy::Gap,
    };
    let (out, dropped) = apply(
        &mut cues,
        &m,
        100,
        &[play("tick"), play("click"), play("tick")],
    );
    assert_eq!(picks(&out).len(), 2, "one tick and the click: {out:?}");
    assert_eq!(dropped, core::slice::from_ref(&gap));
    let (out, dropped) = apply(&mut cues, &m, 139, &[play("tick")]);
    assert_eq!((out.len(), dropped), (0, vec![gap.clone()]), "39 ms after");
    let (out, dropped) = apply(&mut cues, &m, 140, &[play("tick")]);
    assert_eq!(
        (out.len(), dropped.len()),
        (1, 0),
        "40 ms after the last that sounded"
    );
    let (out, dropped) = apply(&mut cues, &m, 150, &[play("click"), play("click")]);
    assert_eq!((out.len(), dropped.len()), (2, 0), "a cue with no gap");
    assert_eq!(DropWhy::Gap.word(), "gap");
}

#[test]
fn a_cue_the_manifest_lacks_or_with_no_files_goes_through_at_file_0_volume_0() {
    let m = manifest(&[("empty", cue(0, 80, CueRules::DEFAULT.with_min_gap_ms(100)))]);
    let mut cues = Cues::new(3);
    let asked = [play("nope"), play("empty"), play("empty")];
    let (out, dropped) = apply(&mut cues, &m, 0, &asked);
    assert_eq!(dropped, []);
    assert_eq!(
        out,
        [
            SoundRequest::sound("nope", 1.0)
                .with_pick(0, 0)
                .with_copy(CopyId(1)),
            SoundRequest::sound("empty", 1.0)
                .with_pick(0, 0)
                .with_copy(CopyId(2)),
            SoundRequest::sound("empty", 1.0)
                .with_pick(0, 0)
                .with_copy(CopyId(3)),
        ]
    );
}

#[test]
fn a_lowest_volume_over_the_cues_volume_in_code_plays_at_the_cues() {
    let m = manifest(&[(
        "loud",
        cue(1, 50, CueRules::DEFAULT.with_volume_min(Some(90))),
    )]);
    let (out, _) = apply(&mut Cues::new(3), &m, 0, &[play("loud"), play("loud")]);
    assert_eq!(picks(&out), [(0, 50), (0, 50)]);
    // One equal to it draws nothing: the files picked are as with none.
    let at = |min| manifest(&[("step", cue(3, 50, CueRules::DEFAULT.with_volume_min(min)))]);
    let steps = [play("step"), play("step"), play("step"), play("step")];
    let (equal, _) = apply(&mut Cues::new(3), &at(Some(50)), 0, &steps);
    let (none, _) = apply(&mut Cues::new(3), &at(None), 0, &steps);
    assert_eq!(picks(&equal), picks(&none));
}

#[test]
fn a_started_sound_is_picked_and_never_dropped_nor_starts_a_gap() {
    let rules = CueRules::DEFAULT
        .with_min_gap_ms(1000)
        .with_pick(Pick::InOrder)
        .with_volume_min(Some(50));
    let m = manifest(&[("hum", cue(2, 60, rules))]);
    let mut queue = shuyi_ui::audio::AudioQueue::<Voice>::default();
    let [one, two] = [queue.start_sound("hum"), queue.start_sound("hum")];
    queue.play_sound("hum");
    let (out, dropped) = apply(&mut Cues::new(3), &m, 0, &queue.take());
    assert_eq!(dropped, []);
    let starts: Vec<_> = out
        .iter()
        .filter_map(|r| match r {
            SoundRequest::Start {
                id,
                variant,
                cue_volume,
                ..
            } => Some((*id, *variant, (50..=60).contains(cue_volume))),
            _ => None,
        })
        .collect();
    assert_eq!(starts, [(one, 0, true), (two, 1, true)]);
    assert_eq!(picks(&out).iter().map(|p| p.0).collect::<Vec<_>>(), [0]);
}

#[test]
fn the_rng_is_deterministic_per_seed() {
    let run = |seed| {
        let mut rng = VariantRng::new(seed);
        (0..8).map(|_| rng.next_u64()).collect::<Vec<_>>()
    };
    assert_eq!(run(1), run(1));
    assert_ne!(run(1), run(2));
    assert_eq!(VariantRng::new(0).next_u64(), 0xE220_A839_7B1D_CDAF);
    assert_eq!(VariantRng::new(5).below(0), 0);
    assert_eq!(VariantRng::new(5).below(1), 0);
}

fn any_pick() -> impl Strategy<Value = Pick> {
    prop_oneof![
        Just(Pick::Random),
        Just(Pick::NoRepeat),
        Just(Pick::Shuffle),
        Just(Pick::InOrder),
    ]
}

/// Plays of two cues: the milliseconds since the play before, and which.
fn any_plays() -> impl Strategy<Value = Vec<(u64, bool)>> {
    prop::collection::vec((0u64..80, any::<bool>()), 1..60)
}

/// Two cues of `files` files under `rules`, at volumes 90 and 70.
fn two_cues(files: usize, rules: CueRules) -> AudioManifest {
    manifest(&[("a", cue(files, 90, rules)), ("b", cue(files, 70, rules))])
}

/// A play's cue, its time, and its file and volume or `None` when dropped.
type Heard = (&'static str, u64, Option<(usize, u8)>);

/// What `plays` give, one by one: each play's cue, time, and its file and
/// volume or `None` when dropped.
fn run(m: &AudioManifest, seed: u64, plays: &[(u64, bool)]) -> Vec<Heard> {
    let mut cues = Cues::new(seed);
    let mut now = 0;
    plays
        .iter()
        .map(|&(after, which)| {
            now += after;
            let id = if which { "a" } else { "b" };
            let (out, dropped) = apply(&mut cues, m, now, &[play(id)]);
            let pick = picks(&out).first().copied();
            assert_eq!(pick.is_none(), dropped.len() == 1);
            (id, now, pick)
        })
        .collect()
}

proptest! {
    /// Every variant gets its chance, whatever the seed.
    #[test]
    fn every_variant_gets_picked(seed: u64, n in 1usize..=6) {
        let mut rng = VariantRng::new(seed);
        let mut seen = vec![false; n];
        for _ in 0..200 {
            let i = rng.below(n);
            prop_assert!(i < n);
            seen[i] = true;
        }
        prop_assert!(seen.iter().all(|&s| s), "{seen:?}");
    }

    #[test]
    fn no_repeat_never_plays_the_same_file_twice_in_a_row(seed: u64, n in 2usize..=8) {
        let played = files_played(Pick::NoRepeat, n, 40, seed);
        prop_assert!(played.windows(2).all(|w| w[0] != w[1]), "{played:?}");
        prop_assert!(played.iter().all(|&f| f < n));
    }

    #[test]
    fn shuffle_plays_each_file_once_in_every_n_and_never_twice_in_a_row(
        seed: u64,
        n in 2usize..=8,
    ) {
        let played = files_played(Pick::Shuffle, n, n * 5, seed);
        for round in played.chunks(n) {
            let mut sorted = round.to_vec();
            sorted.sort_unstable();
            prop_assert_eq!(sorted, (0..n).collect::<Vec<_>>());
        }
        prop_assert!(played.windows(2).all(|w| w[0] != w[1]), "{played:?}");
    }

    #[test]
    fn the_volume_picked_is_within_its_range(
        seed: u64,
        min in 0u8..=100,
        top in 0u8..=100,
        plays in 1usize..30,
    ) {
        let (low, high) = (min.min(top), min.max(top));
        let rules = CueRules::DEFAULT.with_volume_min(Some(low));
        let m = manifest(&[("hit", cue(2, high, rules))]);
        let mut cues = Cues::new(seed);
        let asked: Vec<_> = (0..plays).map(|_| play("hit")).collect();
        let (out, _) = apply(&mut cues, &m, 0, &asked);
        for (_, level) in picks(&out) {
            prop_assert!((low..=high).contains(&level), "{level} not in {low}..={high}");
        }
    }

    #[test]
    fn two_plays_of_a_cue_that_sound_are_at_least_its_gap_apart(
        seed: u64,
        gap in 0u32..100,
        pick in any_pick(),
        plays in any_plays(),
    ) {
        let m = two_cues(3, CueRules::DEFAULT.with_min_gap_ms(gap).with_pick(pick));
        let got = run(&m, seed, &plays);
        for id in ["a", "b"] {
            let times: Vec<u64> = got
                .iter()
                .filter(|(cue, _, pick)| *cue == id && pick.is_some())
                .map(|(_, at, _)| *at)
                .collect();
            prop_assert!(times.windows(2).all(|w| w[1] - w[0] >= u64::from(gap)), "{times:?}");
        }
        // A play dropped is one inside its gap.
        for (i, (cue, at, pick)) in got.iter().enumerate() {
            if pick.is_none() {
                let last = got[..i].iter().rev().find(|(c, _, p)| c == cue && p.is_some());
                prop_assert!(last.is_some_and(|(_, was, _)| at - was < u64::from(gap)));
            }
        }
    }

    /// The same requests with the dropped ones taken out give the same
    /// files and volumes; the same seed and requests give the same plays.
    #[test]
    fn a_dropped_play_draws_nothing(
        seed: u64,
        gap in 1u32..100,
        pick in any_pick(),
        plays in any_plays(),
    ) {
        let rules = CueRules::DEFAULT
            .with_min_gap_ms(gap)
            .with_pick(pick)
            .with_volume_min(Some(20));
        let m = two_cues(4, rules);
        let got = run(&m, seed, &plays);
        prop_assert_eq!(&got, &run(&m, seed, &plays));
        let mut kept = Vec::new();
        let mut now = 0;
        let mut since = 0;
        for (&(after, which), (_, _, pick)) in plays.iter().zip(&got) {
            now += after;
            if pick.is_some() {
                kept.push((now - since, which));
                since = now;
            }
        }
        let sounded: Vec<_> = got.into_iter().filter(|(_, _, pick)| pick.is_some()).collect();
        prop_assert_eq!(run(&m, seed, &kept), sounded);
    }
}

/// A cue of `n` files each `length_ms` long (`None`: unknown), at volume
/// 100, played by `rules`.
fn timed(n: usize, length_ms: Option<u32>, rules: CueRules) -> SoundCue {
    let mut cue = cue(n, 100, rules);
    cue.lengths_ms = vec![length_ms; n];
    cue
}

/// The rules of a cue of `max_copies` copies at most, full by `when_full`.
fn capped(max_copies: u8, when_full: WhenFull) -> CueRules {
    CueRules::DEFAULT.with_max_copies(Some(max_copies), when_full)
}

/// The copies `out` plays, and the copies it stops, in order.
fn copies(out: &[SoundRequest]) -> (Vec<u64>, Vec<u64>) {
    let (mut played, mut stopped) = (Vec::new(), Vec::new());
    for request in out {
        match request {
            SoundRequest::Sound { copy, .. } => played.push(copy.get()),
            SoundRequest::StopCopy { copy } => stopped.push(copy.get()),
            _ => {}
        }
    }
    (played, stopped)
}

/// Each drop's cue and why.
fn drops(dropped: &[DroppedSound]) -> Vec<(&str, DropWhy)> {
    dropped.iter().map(|d| (d.cue.as_str(), d.why)).collect()
}

#[test]
fn skip_drops_a_play_over_the_cap_as_copies() {
    let m = manifest(&[("hit", timed(2, Some(100), capped(2, WhenFull::Skip)))]);
    let mut cues = Cues::new(1);
    let (out, dropped) = apply(&mut cues, &m, 0, &[play("hit"), play("hit"), play("hit")]);
    assert_eq!(copies(&out), (vec![1, 2], vec![]));
    assert_eq!(drops(&dropped), [("hit", DropWhy::Copies)]);
    // 99 ms later both still sound; at 100 ms they have ended.
    let (out, dropped) = apply(&mut cues, &m, 99, &[play("hit")]);
    assert_eq!(
        (out.len(), drops(&dropped)),
        (0, vec![("hit", DropWhy::Copies)])
    );
    let (out, dropped) = apply(&mut cues, &m, 100, &[play("hit"), play("hit")]);
    assert_eq!((copies(&out), dropped), ((vec![3, 4], vec![]), vec![]));
    assert_eq!(DropWhy::Copies.word(), "copies");
}

#[test]
fn stop_oldest_cuts_the_cues_oldest_copy_and_the_new_one_sounds() {
    let m = manifest(&[
        ("step", timed(1, Some(500), capped(1, WhenFull::StopOldest))),
        ("hum", timed(1, Some(500), CueRules::DEFAULT)),
    ]);
    let mut cues = Cues::new(1);
    let (out, _) = apply(&mut cues, &m, 0, &[play("hum"), play("step")]);
    assert_eq!(copies(&out), (vec![1, 2], vec![]));
    let (out, dropped) = apply(&mut cues, &m, 100, &[play("step")]);
    // The stop comes before the new play, and the drop names the cut copy.
    assert_eq!(
        out,
        [
            SoundRequest::StopCopy { copy: CopyId(2) },
            SoundRequest::sound("step", 1.0).with_copy(CopyId(3)),
        ]
    );
    assert_eq!(drops(&dropped), [("step", DropWhy::Cut)]);
    assert_eq!(DropWhy::Cut.word(), "cut");
    // The oldest of two: a cap of two cuts the first.
    let m = manifest(&[("step", timed(1, Some(500), capped(2, WhenFull::StopOldest)))]);
    let mut cues = Cues::new(1);
    let (out, _) = apply(
        &mut cues,
        &m,
        0,
        &[play("step"), play("step"), play("step")],
    );
    assert_eq!(copies(&out), (vec![1, 2, 3], vec![1]));
}

#[test]
fn a_copy_whose_length_has_passed_holds_nothing() {
    let rules = capped(1, WhenFull::Skip);
    let m = manifest(&[("blip", timed(1, Some(50), rules))]);
    let mut cues = Cues::new(1);
    let (out, _) = apply(&mut cues, &m, 1000, &[play("blip")]);
    assert_eq!(copies(&out).0, [1]);
    let (out, dropped) = apply(&mut cues, &m, 1049, &[play("blip")]);
    assert_eq!(
        (out.len(), drops(&dropped)),
        (0, vec![("blip", DropWhy::Copies)])
    );
    let (out, dropped) = apply(&mut cues, &m, 1050, &[play("blip")]);
    assert_eq!((copies(&out).0, dropped), (vec![2], vec![]));
    // A clock that goes back counts as no time passing.
    let (out, _) = apply(&mut cues, &m, 10, &[play("blip")]);
    assert_eq!(out.len(), 0, "copy 2 sounds until 1100");
}

#[test]
fn a_file_of_unknown_length_never_holds_a_copy_or_a_voice() {
    let rules = capped(1, WhenFull::Skip).with_priority(255);
    let mut m = manifest(&[
        ("odd", timed(1, None, rules)),
        ("silent", timed(1, Some(0), rules)),
        (
            "low",
            timed(1, Some(500), CueRules::DEFAULT.with_priority(0)),
        ),
    ]);
    m.max_voices = Some(1);
    let mut cues = Cues::new(1);
    let asked = [play("odd"), play("odd"), play("silent"), play("silent")];
    let (out, dropped) = apply(&mut cues, &m, 0, &asked);
    assert_eq!((copies(&out).0, dropped), (vec![1, 2, 3, 4], vec![]));
    // The one voice is free for the low one, which an unknown one cuts.
    let (out, dropped) = apply(&mut cues, &m, 0, &[play("low"), play("odd")]);
    assert_eq!(copies(&out), (vec![5, 6], vec![5]));
    assert_eq!(drops(&dropped), [("low", DropWhy::Cut)]);
}

/// Three voices, full: `boss` (priority 200) at 0 ms, then `hit` (100) and
/// `boss` at 10 ms.
fn voices_full() -> (Cues, AudioManifest) {
    let at = |priority| timed(1, Some(1000), CueRules::DEFAULT.with_priority(priority));
    let mut m = manifest(&[
        ("boss", at(200)),
        ("hit", at(100)),
        ("step", at(50)),
        ("big", at(200)),
    ]);
    m.max_voices = Some(3);
    let mut cues = Cues::new(1);
    apply(&mut cues, &m, 0, &[play("boss")]);
    apply(&mut cues, &m, 10, &[play("hit"), play("boss")]);
    (cues, m)
}

#[test]
fn with_the_voices_full_of_higher_priorities_a_lower_play_is_dropped() {
    let (mut cues, m) = voices_full();
    let (out, dropped) = apply(&mut cues, &m, 20, &[play("step")]);
    assert_eq!(
        (out.len(), drops(&dropped)),
        (0, vec![("step", DropWhy::Voices)])
    );
    assert_eq!(DropWhy::Voices.word(), "voices");
    // A dropped play takes no id: the next copy is the 4th.
    let (out, _) = apply(&mut cues, &m, 20, &[play("hit")]);
    assert_eq!(copies(&out), (vec![4], vec![2]));
}

#[test]
fn an_equal_or_higher_play_cuts_the_oldest_of_the_lowest_and_sounds() {
    // Higher: the lowest is `hit`, copy 2.
    let (mut cues, m) = voices_full();
    let (out, dropped) = apply(&mut cues, &m, 20, &[play("big")]);
    assert_eq!(copies(&out), (vec![4], vec![2]));
    assert_eq!(drops(&dropped), [("hit", DropWhy::Cut)]);
    // Equal, among equals: the oldest, copy 1, then copy 3.
    let (out, _) = apply(&mut cues, &m, 30, &[play("big"), play("big")]);
    assert_eq!(copies(&out), (vec![5, 6], vec![1, 3]));
    // Ties in age go to the lower id.
    let mut m = manifest(&[("a", timed(1, Some(1000), CueRules::DEFAULT))]);
    m.max_voices = Some(2);
    let mut cues = Cues::new(1);
    let (out, _) = apply(&mut cues, &m, 0, &[play("a"), play("a"), play("a")]);
    assert_eq!(copies(&out), (vec![1, 2, 3], vec![1]));
}

#[test]
fn stop_oldest_frees_a_voice_so_the_play_sounds() {
    let rules = capped(1, WhenFull::StopOldest).with_priority(0);
    let mut m = manifest(&[
        ("step", timed(1, Some(500), rules)),
        (
            "boss",
            timed(1, Some(500), CueRules::DEFAULT.with_priority(255)),
        ),
    ]);
    m.max_voices = Some(2);
    let mut cues = Cues::new(1);
    apply(&mut cues, &m, 0, &[play("step"), play("boss")]);
    let (out, dropped) = apply(&mut cues, &m, 10, &[play("step")]);
    assert_eq!(copies(&out), (vec![3], vec![1]));
    assert_eq!(drops(&dropped), [("step", DropWhy::Cut)]);
}

/// A cue in the property tests: a cap or none, a rule when full, a
/// priority from few (to tie often), a gap, and its files' lengths.
fn any_counted_cue() -> impl Strategy<Value = SoundCue> {
    let when_full = prop_oneof![Just(WhenFull::Skip), Just(WhenFull::StopOldest)];
    let length = prop_oneof![Just(None), (0u32..200).prop_map(Some)];
    (
        prop::option::of(1u8..4),
        when_full,
        0u8..3,
        prop::collection::vec(length, 1..4),
        0u32..30,
    )
        .prop_map(|(max_copies, when_full, priority, lengths, gap)| {
            let rules = CueRules::DEFAULT
                .with_max_copies(max_copies, when_full)
                .with_priority(priority)
                .with_min_gap_ms(gap)
                .with_volume_min(Some(20));
            let mut cue = cue(lengths.len(), 90, rules);
            cue.lengths_ms = lengths;
            cue
        })
}

/// A manifest of cues `a` and `b`, under a voice limit or none.
fn any_counted() -> impl Strategy<Value = AudioManifest> {
    (
        any_counted_cue(),
        any_counted_cue(),
        prop::option::of(1u16..5),
    )
        .prop_map(|(a, b, voices)| {
            let mut m = manifest(&[("a", a), ("b", b)]);
            m.max_voices = voices;
            m
        })
}

/// One frame: its time, and what the cue rules made of its play.
type Frame = (u64, Vec<SoundRequest>, Vec<DroppedSound>);

/// What `plays` (the milliseconds since the play before, and `a` or `b`)
/// give, a frame each.
fn frames(m: &AudioManifest, seed: u64, plays: &[(u64, bool)]) -> Vec<Frame> {
    let mut cues = Cues::new(seed);
    let mut now = 0;
    plays
        .iter()
        .map(|&(after, which)| {
            now += after;
            let id = if which { "a" } else { "b" };
            let (out, dropped) = apply(&mut cues, m, now, &[play(id)]);
            (now, out, dropped)
        })
        .collect()
}

/// Whether a frame's play was dropped: a drop of its own, not a cut.
fn dropped_play(frame: &Frame) -> bool {
    frame.2.iter().any(|d| d.why != DropWhy::Cut)
}

proptest! {
    /// The model: the copies sounding, rebuilt from the `Sound` and
    /// `StopCopy` requests and the files' lengths. No more than a cue's
    /// `max_copies`, or `max_voices` in all, ever sound.
    #[test]
    fn no_more_copies_or_voices_sound_than_the_caps(
        m in any_counted(),
        seed: u64,
        plays in any_plays(),
    ) {
        let mut sounding: Vec<(u64, String, u64)> = Vec::new();
        for (now, out, _) in frames(&m, seed, &plays) {
            sounding.retain(|(_, _, ends)| *ends > now);
            for request in out {
                match request {
                    SoundRequest::StopCopy { copy } => {
                        let at = sounding.iter().position(|(c, _, _)| *c == copy.get());
                        prop_assert!(at.is_some(), "copy {copy:?} cut while not sounding");
                        sounding.retain(|(c, _, _)| *c != copy.get());
                    }
                    SoundRequest::Sound { cue, variant, copy, .. } => {
                        let length = m.sounds[&cue].length_ms(variant).filter(|&ms| ms > 0);
                        if let Some(length) = length {
                            sounding.push((copy.get(), cue, now + u64::from(length)));
                        }
                    }
                    _ => {}
                }
            }
            for (id, def) in &m.sounds {
                let mine = sounding.iter().filter(|(_, cue, _)| cue == id).count();
                let max = def.rules.max_copies.map_or(usize::MAX, usize::from);
                prop_assert!(mine <= max, "{mine} copies of {id}, cap {max}");
            }
            let max = m.max_voices.map_or(usize::MAX, usize::from);
            prop_assert!(sounding.len() <= max, "{} voices, at most {max}", sounding.len());
        }
    }

    /// The same requests with the dropped ones taken out give the same
    /// plays, files, volumes and cuts; the same seed gives the same.
    #[test]
    fn a_play_dropped_by_the_caps_draws_nothing(
        m in any_counted(),
        seed: u64,
        plays in any_plays(),
    ) {
        let got = frames(&m, seed, &plays);
        prop_assert_eq!(&got, &frames(&m, seed, &plays));
        let mut kept = Vec::new();
        let mut since = 0;
        for (&(_, which), frame) in plays.iter().zip(&got) {
            if !dropped_play(frame) {
                kept.push((frame.0 - since, which));
                since = frame.0;
            }
        }
        let sounded: Vec<_> = got.into_iter().filter(|f| !dropped_play(f)).collect();
        prop_assert_eq!(frames(&m, seed, &kept), sounded);
    }
}
