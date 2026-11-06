//! Tests of the voice manifest and cast, with a made-up game whose lines
//! change with the weather: a line's only clip, or one clip per weather.

use serde::Deserialize;

use super::*;
use crate::dialogue::{NoDirective, Present, parse_dlg};

/// The tests' clip variants: the line's only clip, or one per weather.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Deserialize)]
enum Weather {
    #[default]
    Any,
    Sun,
    Rain,
}

impl ClipVariant for Weather {
    fn suffix(self) -> &'static str {
        match self {
            Weather::Any => "",
            Weather::Sun => ".sun",
            Weather::Rain => ".rain",
        }
    }

    fn split() -> &'static [Self] {
        &[Weather::Sun, Weather::Rain]
    }
}

/// A variant type that keeps the engine's messages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Deserialize)]
enum Plain {
    #[default]
    Only,
    One,
    Two,
}

impl ClipVariant for Plain {
    fn suffix(self) -> &'static str {
        ""
    }

    fn split() -> &'static [Self] {
        &[Plain::One, Plain::Two]
    }
}

/// A scene `s`: a plain line, a line that changes with the weather
/// (`{sky}`), a line no clip can say (`{name}`), and narration.
const SCRIPT: &str = "\
@scene s
@left guide neutral
guide: Hello.
guide: The {sky} is bright.
guide: Welcome back, {name}.
> Hello.
@end
";

fn dialogue() -> DialogueTable {
    let (parsed, errors) = parse_dlg::<NoDirective>("t.dlg", SCRIPT);
    assert_eq!(errors, []);
    DialogueTable {
        scenes: parsed
            .into_iter()
            .map(|p| (p.scene.id.clone(), p.scene.resolved(&Present::Everyone)))
            .collect(),
        files: BTreeMap::new(),
    }
}

/// The id of the line of [`SCRIPT`] written `text`, said by `speaker`.
fn id(speaker: &str, text: &str) -> String {
    let dialogue = dialogue();
    let lines = dialogue.scenes["s"].lines();
    let line = lines
        .iter()
        .find(|l| l.speaker == speaker && l.text == text);
    line.unwrap().id.to_string()
}

fn hello() -> String {
    id("guide", "Hello.")
}

fn sky() -> String {
    id("guide", "The {sky} is bright.")
}

fn named() -> String {
    id("guide", "Welcome back, {name}.")
}

fn narration() -> String {
    id(NARRATION_SPEAKER, "Hello.")
}

/// The words a line says as played in `weather` (`{sky}` is the sun's or
/// the rain's; `{name}` can't be said).
fn spoken(text: &str, weather: Weather) -> Option<String> {
    if text.contains("{name}") {
        return None;
    }
    let sky = match weather {
        Weather::Sun => "sun",
        Weather::Rain => "rain",
        Weather::Any if text.contains("{sky}") => return None,
        Weather::Any => "",
    };
    Some(text.replace("{sky}", sky))
}

fn clip(line: &str, variant: &str, spoken: &str) -> String {
    let variant = if variant.is_empty() {
        String::new()
    } else {
        format!("variant: {variant}, ")
    };
    format!(
        "(line: {line:?}, {variant}spoken: {spoken:?}, voice: \"guide_a\", made_by: Generated(tool: \"t\", model: \"m\", date: \"2026-10-16\"))"
    )
}

fn full() -> Vec<String> {
    vec![
        clip(&hello(), "", "Hello."),
        clip(&sky(), "Sun", "The sun is bright."),
        clip(&sky(), "Rain", "The rain is bright."),
        clip(&narration(), "", "Hello."),
    ]
}

#[test]
fn a_changed_direction_means_a_remake_and_leaves_the_clip_playable() {
    let table = |line: &str| {
        let src = format!("@scene s\n@left guide neutral\n{line}\n@end\n");
        let (parsed, errors) = parse_dlg::<NoDirective>("t.dlg", &src);
        assert_eq!(errors, []);
        DialogueTable::<NoDirective>::new(
            parsed
                .into_iter()
                .map(|p| (p.scene.id.clone(), p.scene))
                .collect(),
        )
    };
    let plain = table("guide: Hello.");
    let line = plain.scenes["s"].lines()[0].id.to_string();
    let loud = table("guide (loud): Hello.");
    assert_eq!(loud.scenes["s"].lines()[0].id.as_str(), line);
    let made_by =
        "voice: \"v\", made_by: Generated(tool: \"t\", model: \"m\", date: \"2026-11-02\")";
    let entry = |direction: &str| {
        format!("(clips: [(line: {line:?}, spoken: \"Hello.\", {direction}{made_by})])")
    };
    let spoken = |text: &str, _: Weather| Some(text.to_owned());

    let undirected: VoiceManifest<Weather> = from_source("voice.ron", &entry(""), &plain).unwrap();
    let directed: VoiceManifest<Weather> =
        from_source("voice.ron", &entry("direction: Some(\"loud\"), "), &plain).unwrap();
    let remake = |m: &VoiceManifest<Weather>, d: &DialogueTable<NoDirective>| {
        VoiceManifest::needs_remake(&m.clips[0], d, spoken)
    };
    assert!(!remake(&undirected, &plain));
    assert!(!remake(&directed, &loud));
    assert!(remake(&undirected, &loud));
    assert!(remake(&directed, &plain));
    assert!(remake(&directed, &table("guide (quiet): Hello.")));

    // The words still match, so the old take plays until a new one is made.
    assert!(!VoiceManifest::is_stale(
        &undirected.clips[0],
        &loud,
        spoken
    ));
    assert_eq!(undirected.playable(&loud, spoken).len(), 1);

    // New words, or no such line, need a remake too.
    assert!(remake(&undirected, &table("guide: Goodbye.")));
    let reworded = |text: &str, _: Weather| Some(format!("{text}!"));
    assert!(VoiceManifest::needs_remake(
        &undirected.clips[0],
        &plain,
        reworded
    ));
}

fn source(clips: &[String]) -> String {
    format!("(clips: [\n{}\n])", clips.join(",\n"))
}

fn manifest(clips: &[String]) -> VoiceManifest<Weather> {
    from_source("voice.ron", &source(clips), &dialogue()).unwrap()
}

fn errors<V: ClipVariant>(clips: &[String]) -> Vec<String> {
    from_source::<V, _>("voice.ron", &source(clips), &dialogue())
        .err()
        .unwrap_or_default()
        .iter()
        .map(ToString::to_string)
        .collect()
}

#[test]
fn a_manifest_lists_its_clips_in_order() {
    let m = manifest(&full());
    assert_eq!(m.clips.len(), 4);
    assert_eq!(m.clips[1].variant, Weather::Sun);
    assert_eq!(m.clips[0].variant, Weather::Any);
    assert_eq!(m.clips[0].voice, "guide_a");
    assert_eq!(m.clips[0].line.as_str(), hello());
    assert!(matches!(m.clips[0].made_by, MadeBy::Generated { .. }));
    assert_eq!(manifest(&[]), VoiceManifest::default());
    let recorded =
        "(clips: [(line: \"x\", spoken: \"y\", voice: \"v\", made_by: Recorded(actor: \"A\"))])";
    let table = DialogueTable::<NoDirective>::default();
    assert_eq!(
        from_source::<Weather, _>("v.ron", recorded, &table)
            .err()
            .map(|e| e.len()),
        Some(1)
    );
}

#[test]
fn every_problem_with_a_clip_is_reported() {
    let file = "voice.ron";
    let missing = clip("s_00000000", "", "x");
    assert_eq!(
        errors::<Weather>(&[missing]),
        [format!(
            "{file}:2: no dialogue line has the id \"s_00000000\""
        )]
    );
    let twice = [clip(&hello(), "", "Hello."), clip(&hello(), "", "Hello.")];
    assert_eq!(
        errors::<Weather>(&twice),
        [format!("{file}:2: line \"{}\" has two Any clips", hello())]
    );
    let empty = [clip(&hello(), "", " ").replace("guide_a", " ")];
    assert_eq!(
        errors::<Weather>(&empty),
        [
            format!(
                "{file}:2: the clip of line \"{}\" has no spoken text",
                hello()
            ),
            format!("{file}:2: the clip of line \"{}\" has no voice", hello()),
        ]
    );
    let half = [clip(&sky(), "Sun", "x")];
    assert_eq!(
        errors::<Weather>(&half),
        [format!(
            "{file}:2: line \"{}\" lacks some of its variants' clips",
            sky()
        )]
    );
    let mixed = [
        clip(&sky(), "", "x"),
        clip(&sky(), "Sun", "x"),
        clip(&sky(), "Rain", "x"),
    ];
    assert_eq!(
        errors::<Weather>(&mixed),
        [format!(
            "{file}:2: line \"{}\" has an only clip and variants' clips; keep one kind",
            sky()
        )]
    );
    assert_eq!(errors::<Plain>(&[clip(&sky(), "One", "x")]).len(), 1);
    assert_eq!(errors::<Weather>(&["(".to_owned()]).len(), 1);
}

#[test]
fn a_clip_file_is_its_line_id_and_variant() {
    let line = LineId::new("gate_talk_0123abcd_2");
    assert_eq!(
        clip_file(&line, Weather::Any),
        "gate_talk/gate_talk_0123abcd_2.ogg"
    );
    assert_eq!(
        clip_file(&line, Weather::Rain),
        "gate_talk/gate_talk_0123abcd_2.rain.ogg"
    );
    assert!(Weather::Any.is_only() && !Weather::Sun.is_only());
}

#[test]
fn the_playable_clips_say_what_their_line_says() {
    let d = dialogue();
    let m = manifest(&full());
    let playable = m.playable(&d, spoken);
    assert_eq!(playable.len(), 4);
    assert!(!playable.is_empty(), "{playable:?}");
    let line = |id: String| LineId::new(id);
    assert_eq!(
        playable.variant_for(&line(hello()), Weather::Rain),
        Some(Weather::Any)
    );
    assert_eq!(
        playable.variant_for(&line(sky()), Weather::Rain),
        Some(Weather::Rain)
    );
    assert_eq!(
        playable.variant_for(&line(sky()), Weather::Sun),
        Some(Weather::Sun)
    );
    assert_eq!(playable.variant_for(&line(named()), Weather::Sun), None);
    assert_eq!(
        playable.variant_for(&line("s_00000000".into()), Weather::Sun),
        None
    );
    for c in &m.clips {
        assert!(!VoiceManifest::is_stale(c, &d, spoken), "{c:?}");
    }
    let found = Playable::<Weather>::default();
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn clips_that_cannot_say_their_line_are_stale() {
    let d = dialogue();
    // Reworded: the clip says other words.
    let reworded = manifest(&[clip(&hello(), "", "Hello there.")]);
    assert!(VoiceManifest::is_stale(&reworded.clips[0], &d, spoken));
    let found = reworded.playable(&d, spoken);
    assert!(found.is_empty(), "{found:?}");
    // One clip for a line that changes, or a line no clip can say.
    let both = manifest(&[clip(&sky(), "", "The sky is bright.")]);
    assert!(VoiceManifest::is_stale(&both.clips[0], &d, spoken));
    let name = manifest(&[clip(&named(), "", "Welcome back, Ed.")]);
    let found = name.playable(&d, spoken);
    assert!(found.is_empty(), "{found:?}");
    // A line that is gone.
    let mut gone = manifest(&[clip(&hello(), "", "Hello.")]);
    gone.clips[0].line = LineId::new("s_00000000");
    assert!(VoiceManifest::is_stale(&gone.clips[0], &d, spoken));
    let found = gone.playable(&d, spoken);
    assert!(found.is_empty(), "{found:?}");
    // Only the stale ones are left out.
    let some = manifest(&[clip(&hello(), "", "Hi."), clip(&narration(), "", "Hello.")]);
    assert_eq!(some.playable(&d, spoken).len(), 1);
}

/// The tests' voice tool: which of two programs reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
enum Reader {
    Kokoro,
    Piper,
}

/// The tests' voice tool's settings.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
struct Tool {
    reader: Reader,
}

/// A cast source with one entry per line, `"key": (voice: .., settings:
/// (reader: ..))`.
fn cast_source(entries: &[(&str, &str, &str)]) -> String {
    let lines: Vec<String> = entries
        .iter()
        .map(|(key, voice, reader)| {
            format!("{key:?}: (voice: {voice:?}, settings: (reader: {reader})),")
        })
        .collect();
    format!("(voices: {{\n{}\n}})", lines.join("\n"))
}

fn cast_errors(entries: &[(&str, &str, &str)]) -> Vec<String> {
    cast_from_source::<Tool, Weather>("cast.ron", &cast_source(entries), |s| s == "guide")
        .err()
        .unwrap_or_default()
        .iter()
        .map(ToString::to_string)
        .collect()
}

#[test]
fn a_cast_gives_each_speaker_and_variant_a_voice() {
    let source = cast_source(&[
        ("guide", "guide_a", "Kokoro"),
        ("guide.rain", "guide_wet", "Piper"),
        (">", "narrator", "Kokoro"),
    ]);
    let cast: Cast<Tool> =
        cast_from_source::<_, Weather>("cast.ron", &source, |s| s == "guide").unwrap();
    let rain = cast.voice_for("guide", Weather::Rain).unwrap();
    assert_eq!(
        rain,
        &CastVoice::new(
            "guide_wet",
            Tool {
                reader: Reader::Piper
            }
        )
    );
    let sun = cast.voice_for("guide", Weather::Sun).unwrap();
    assert_eq!(sun.voice, "guide_a");
    assert_eq!(sun.settings.reader, Reader::Kokoro);
    assert_eq!(
        cast.voice_for("guide", Weather::Any).unwrap().voice,
        "guide_a"
    );
    let narrator = cast.voice_for(NARRATION_SPEAKER, Weather::Rain).unwrap();
    assert_eq!(narrator.voice, "narrator");
    assert_eq!(cast.voice_for("nobody", Weather::Rain), None);
    assert_eq!(Cast::<Tool>::default().voices.len(), 0);
    assert_eq!(Cast::new(cast.voices.clone()), cast);
}

#[test]
fn a_tool_without_settings_writes_unit() {
    let source = "(voices: {\"guide\": (voice: \"guide_a\", settings: ())})";
    let cast: Cast<()> = cast_from_source::<_, ()>("cast.ron", source, |_| true).unwrap();
    assert_eq!(cast.voice_for("guide", ()).unwrap().voice, "guide_a");
}

#[test]
fn every_problem_with_a_cast_entry_is_reported() {
    assert_eq!(
        cast_errors(&[("guide", " ", "Kokoro")]),
        ["cast.ron:2: \"guide\" has no voice"]
    );
    assert_eq!(
        cast_errors(&[("nobody", "x", "Kokoro")]),
        ["cast.ron:2: \"nobody\" is not a character that speaks"]
    );
    assert_eq!(
        cast_errors(&[("guide.snow", "x", "Kokoro")]),
        [
            "cast.ron:2: \"guide.snow\": \".snow\" is not a variant's suffix (those are: \".sun\", \".rain\")"
        ]
    );
    assert_eq!(
        cast_errors(&[("nobody.rain", "x", "Kokoro")]),
        ["cast.ron:2: \"nobody.rain\": \"nobody\" is not a character that speaks"]
    );
    // Narration may have variants too.
    assert_eq!(cast_errors(&[(">.sun", "x", "Kokoro")]), [""; 0]);
    // Every entry's problems, in key order.
    assert_eq!(
        cast_errors(&[("nobody", "x", "Kokoro"), ("guide", "", "Piper")]),
        [
            "cast.ron:3: \"guide\" has no voice",
            "cast.ron:2: \"nobody\" is not a character that speaks",
        ]
    );
    // Settings the tool rejects, and a cast that isn't RON.
    let unknown = cast_errors(&[("guide", "x", "Espeak")]);
    assert_eq!(unknown.len(), 1);
    assert!(unknown[0].starts_with("cast.ron:2"), "{unknown:?}");
    assert!(cast_from_source::<Tool, Weather>("cast.ron", "(", |_| true).is_err());
}

#[test]
fn the_variants_are_the_default_and_the_split_ones() {
    assert_eq!(
        Weather::variants(),
        [Weather::Any, Weather::Sun, Weather::Rain]
    );
    assert_eq!(<()>::variants(), [()]);
}

proptest::proptest! {
    /// `voice_for` finds a voice only in the speaker's own entry or the one
    /// for its variant, and prefers the variant's.
    #[test]
    fn voice_for_reads_only_the_speakers_entries(
        keys in proptest::collection::btree_set(
            (0..3_usize, 0..3_usize),
            0..9,
        ),
        speaker in 0..3_usize,
        variant in 0..3_usize,
    ) {
        let speakers = ["guide", "hero", "nobody"];
        let all = Weather::variants();
        let key = |s: usize, v: usize| format!("{}{}", speakers[s], all[v].suffix());
        let voices = keys
            .iter()
            .map(|&(s, v)| (key(s, v), CastVoice::new(key(s, v), ())))
            .collect();
        let cast = Cast::new(voices);
        let found = cast.voice_for(speakers[speaker], all[variant]);
        let own = cast.voices.get(&key(speaker, variant));
        let plain = cast.voices.get(speakers[speaker]);
        proptest::prop_assert_eq!(found, own.or(plain));
    }
}

#[test]
fn line_texts_lists_every_line() {
    let d = dialogue();
    let texts = line_texts(&d);
    assert_eq!(texts.len(), 4);
    assert_eq!(texts[hello().as_str()], "Hello.");
}

#[test]
fn no_voices_is_the_only_clip_and_no_split() {
    assert_eq!(().suffix(), "");
    assert!(().is_only());
    let found = <() as ClipVariant>::split();
    assert!(found.is_empty(), "{found:?}");
}
