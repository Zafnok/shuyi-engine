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
    let want: Vec<_> = (asked.iter().filter_map(SoundRequest::from_request))
        .zip(&out)
        .map(|(r, got)| r.clone().with_pick(file(got), own(&r)))
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
            SoundRequest::sound("nope", 1.0).with_pick(0, 0),
            SoundRequest::sound("empty", 1.0).with_pick(0, 0),
            SoundRequest::sound("empty", 1.0).with_pick(0, 0),
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
