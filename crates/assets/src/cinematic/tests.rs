//! Tests of the cinematic file's frame, with a made-up game's two shot
//! kinds: a card of one letter, and a count of a shared list's length.

use alloc::collections::BTreeSet;

use serde::Deserialize;

use super::*;
use crate::audio::{CreditRef, MusicCue};

/// A shot as the toy writes it.
#[derive(Debug, Deserialize)]
enum ToyShot {
    /// A card showing one letter.
    Card(char),
    /// How long a shared list is, by its name.
    Count(String),
}

/// A shot as the toy keeps it.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Kept {
    Card(char),
    Count(usize),
}

/// The toy's checks: a card's letter is a capital, and a count names one
/// of the file's lists, each of which some shot must use.
struct Checks<'a> {
    lists: &'a BTreeMap<String, Vec<u8>>,
    used: BTreeSet<&'a str>,
}

impl<'a> Checks<'a> {
    fn of<S>(raw: &'a CinematicFile<S, Vec<u8>>) -> Self {
        Self {
            lists: &raw.actions,
            used: BTreeSet::new(),
        }
    }
}

impl ShotCheck<ToyShot> for Checks<'_> {
    type Shot = Kept;

    fn check(&mut self, n: usize, shot: &ToyShot, problems: &mut Vec<String>) -> Kept {
        match shot {
            ToyShot::Card(c) => {
                if !c.is_ascii_uppercase() {
                    problems.push(format!("shot {n}: '{c}' is not a capital letter"));
                }
                Kept::Card(*c)
            }
            ToyShot::Count(name) => {
                let Some((name, list)) = self.lists.get_key_value(name) else {
                    problems.push(format!("shot {n}: no list \"{name}\""));
                    return Kept::Count(0);
                };
                self.used.insert(name);
                Kept::Count(list.len())
            }
        }
    }

    fn finish(&mut self, problems: &mut Vec<String>) {
        let unused = self
            .lists
            .keys()
            .filter(|k| !self.used.contains(k.as_str()));
        problems.extend(unused.map(|k| format!("actions \"{k}\": no shot uses it")));
    }
}

/// Checks with no lists, for files without `actions`.
struct Cards;

impl ShotCheck<ToyShot> for Cards {
    type Shot = Kept;

    fn check(&mut self, n: usize, shot: &ToyShot, problems: &mut Vec<String>) -> Kept {
        match shot {
            ToyShot::Card(c) => Kept::Card(*c),
            ToyShot::Count(_) => {
                problems.push(format!("shot {n}: a count needs a list"));
                Kept::Count(0)
            }
        }
    }
}

/// An audio manifest with one track, `theme`, 10 s long.
fn audio() -> AudioManifest {
    let mut audio = AudioManifest::default();
    let theme = MusicCue {
        file: "theme.ogg".to_owned(),
        volume: 100,
        looped: true,
        length_ms: 10_000,
        credit: CreditRef::Own,
    };
    audio.music.insert("theme".to_owned(), theme);
    audio
}

/// A cinematic file with `length` and `shots` as written.
fn source(length: &str, shots: &str) -> String {
    format!("(id: \"t\", length: {length}, shots: [{shots}])")
}

const CARD: &str = "(at: 0.0, shot: Card('A'))";

fn load(source: &str) -> Result<CinematicDef<Kept>, Vec<ContentError>> {
    from_source::<ToyShot, _>("c.ron", "t", source, &audio(), Cards)
}

/// The messages of the errors of `source` as file `t`.
fn problems(source: &str) -> Vec<String> {
    match load(source) {
        Ok(def) => panic!("loaded: {def:?}"),
        Err(errors) => {
            assert!(errors.iter().all(|e| e.file == "c.ron"), "{errors:?}");
            errors.into_iter().map(|e| e.message).collect()
        }
    }
}

#[test]
fn a_cinematic_is_its_shots_in_whole_milliseconds() {
    let src = source(
        "Seconds(2.5)",
        "(at: 0, shot: Card('A')), (at: 1.2344, shot: Card('B'))",
    );
    let def = load(&src).unwrap();
    assert_eq!(
        def,
        CinematicDef {
            id: "t".to_owned(),
            music: None,
            length_ms: 2500,
            shots: vec![
                ShotDef {
                    at_ms: 0,
                    shot: Kept::Card('A'),
                },
                ShotDef {
                    at_ms: 1234,
                    shot: Kept::Card('B'),
                },
            ],
        }
    );
}

#[test]
fn a_cinematic_on_a_music_cue_is_as_long_as_the_cue() {
    let on_theme = |last: &str| {
        let shots = format!("{CARD}, (at: {last}, shot: Card('B'))");
        source("Music(\"theme\")", &shots)
    };
    let def = load(&on_theme("9.999")).unwrap();
    assert_eq!(def.music.as_deref(), Some("theme"));
    assert_eq!(def.length_ms, 10_000);
    assert_eq!(def.shots[1].at_ms, 9999);
    // A shot at the cue's very end would never show.
    assert_eq!(
        problems(&on_theme("10")),
        ["shot 2: at 10 is not before the end (10 seconds)"]
    );
    // Without the cue there is no end to check the shots against.
    let src = on_theme("500").replace("theme", "nope");
    assert_eq!(problems(&src), ["length: no music cue \"nope\""]);
}

#[test]
fn the_id_is_the_file_name() {
    let src = source("Seconds(1)", CARD).replace("\"t\"", "\"x\"");
    assert_eq!(problems(&src), ["id \"x\" must match the file name \"t\""]);
}

#[test]
fn a_stated_length_is_above_zero_and_not_too_long() {
    let stated = |length: &str| load(&source(length, CARD));
    assert_eq!(stated("Seconds(0.001)").unwrap().length_ms, 1);
    assert_eq!(stated("Seconds(3600)").unwrap().length_ms, 3_600_000);
    assert_eq!(stated("Seconds(2.5)").unwrap().music, None);
    for length in ["0", "0.0004", "-1", "3600.5", "inf", "NaN"] {
        assert_eq!(
            problems(&source(&format!("Seconds({length})"), CARD)),
            [format!(
                "length: {length} seconds; it must be above 0 and at most 3600"
            )],
        );
    }
}

#[test]
fn shots_are_in_order_from_zero_to_before_the_end() {
    let check = |shots: &str| problems(&source("Seconds(10)", shots));
    let card = |at: &str| format!("(at: {at}, shot: Card('A'))");
    let cards = |ats: &[&str]| ats.iter().map(|at| card(at)).collect::<Vec<_>>().join(", ");
    assert_eq!(check(""), ["shots: a cinematic needs at least one shot"]);
    assert_eq!(
        check(&cards(&["0.5"])),
        ["shot 1: the first shot must be at 0.0, not 0.5"]
    );
    assert_eq!(
        check(&cards(&["0", "4", "4"])),
        ["shot 3: at 4 must be after the shot before it; shots are in order of time"]
    );
    assert_eq!(
        check(&cards(&["0", "4", "3.999"])),
        ["shot 3: at 3.999 must be after the shot before it; shots are in order of time"]
    );
    assert_eq!(
        check(&cards(&["0", "10"])),
        ["shot 2: at 10 is not before the end (10 seconds)"]
    );
    assert_eq!(
        check(&cards(&["0", "9.999", "11"])),
        ["shot 3: at 11 is not before the end (10 seconds)"]
    );
    for at in ["-1", "NaN", "inf", "5000000"] {
        assert_eq!(
            check(&cards(&["0", at])),
            [format!("shot 2: at {at} is not a time")]
        );
    }
    // A first shot that is no time is only that.
    assert_eq!(check(&cards(&["-1"])), ["shot 1: at -1 is not a time"]);
    // In order, to the millisecond.
    let ok = load(&source("Seconds(10)", &cards(&["0", "0.001"]))).unwrap();
    let starts: Vec<u32> = ok.shots.iter().map(|s| s.at_ms).collect();
    assert_eq!(starts, [0, 1]);
}

#[test]
fn every_problem_is_reported_at_once_each_shots_after_its_time() {
    let src = "(id: \"x\", length: Music(\"nope\"), shots: [
        (at: 1, shot: Count(\"list\")),
        (at: 1, shot: Card('a')),
    ])";
    assert_eq!(
        problems(src),
        [
            "id \"x\" must match the file name \"t\"",
            "length: no music cue \"nope\"",
            "shot 1: the first shot must be at 0.0, not 1",
            "shot 1: a count needs a list",
            "shot 2: at 1 must be after the shot before it; shots are in order of time",
        ]
    );
    // A file that isn't RON is one error, with its place.
    let errors = load("(id: 3)").unwrap_err();
    assert_eq!(errors.len(), 1);
    assert_eq!(
        (errors[0].file.as_str(), errors[0].line),
        ("c.ron", Some(1))
    );
    // A field the format doesn't have is refused: there are no fades.
    let src = source("Seconds(1)", CARD).replace("id:", "fade: 1, id:");
    assert_eq!(problems(&src).len(), 1);
    let src = source("Seconds(1)", "(at: 0, shot: Card('A'), fade: 1)");
    assert_eq!(problems(&src).len(), 1);
}

#[test]
fn a_game_checks_its_shots_with_the_lists_they_share() {
    let src = "(id: \"t\", length: Seconds(5), actions: {\"two\": [1, 2], \"none\": []}, shots: [
        (at: 0, shot: Count(\"two\")),
        (at: 1, shot: Card('b')),
        (at: 2, shot: Count(\"three\")),
    ])";
    let raw: CinematicFile<ToyShot, Vec<u8>> = parse("c.ron", src).unwrap();
    let errors = check("c.ron", "t", &raw, &audio(), Checks::of(&raw)).unwrap_err();
    let messages: Vec<String> = errors.into_iter().map(|e| e.message).collect();
    assert_eq!(
        messages,
        [
            "shot 2: 'b' is not a capital letter",
            "shot 3: no list \"three\"",
            "actions \"none\": no shot uses it",
        ]
    );
    let src = "(id: \"t\", length: Seconds(5), actions: {\"two\": [1, 2]}, shots: [
        (at: 0, shot: Count(\"two\")),
        (at: 1, shot: Card('B')),
    ])";
    let raw: CinematicFile<ToyShot, Vec<u8>> = parse("c.ron", src).unwrap();
    let def = check("c.ron", "t", &raw, &audio(), Checks::of(&raw)).unwrap();
    let shots: Vec<Kept> = def.shots.into_iter().map(|s| s.shot).collect();
    assert_eq!(shots, [Kept::Count(2), Kept::Card('B')]);
    // Without lists, `actions` may be left out.
    let raw: CinematicFile<ToyShot, Vec<u8>> = parse("c.ron", &source("Seconds(1)", CARD)).unwrap();
    assert!(raw.actions.is_empty());
}

#[test]
fn seconds_are_whole_milliseconds_or_no_time() {
    assert_eq!(millis(1.2344), Some(1234));
    assert_eq!(millis(1.2346), Some(1235));
    assert_eq!(millis(0.0), Some(0));
    assert_eq!(millis(4_294_967.0), Some(4_294_967_000));
    for bad in [-0.001, -1.0, 4_294_968.0, f32::NAN, f32::INFINITY] {
        assert_eq!(millis(bad), None, "{bad}");
    }
    assert!((seconds(1234) - 1.234).abs() < 1e-12);
}
