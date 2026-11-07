//! Tests of the plan, on the voice tests' weather game: scene `a` is
//! voiced, scene `b` isn't. The guide has a voice of their own in the
//! rain.

use proptest::prelude::*;

use super::super::tests::Weather;
use super::*;
use crate::dialogue::{NARRATION_SPEAKER, NoDirective, Present, REPLY_SPEAKER, Scene, parse_dlg};

const SCRIPT: &str = "\
@scene a
@left guide neutral
> The gate opens.
guide (warm): Hello.
guide: The {sky} is bright.
> Look up.
@choice
* earnest: Hi.
  guide: Good.
* wry: Bye.
@endchoice
stranger: Who goes there?
@end
@scene b
@left guide neutral
guide: Not voiced.
@end
";

fn table(script: &str) -> DialogueTable {
    let (parsed, errors) = parse_dlg::<NoDirective>("t.dlg", script);
    assert_eq!(errors, []);
    DialogueTable::new(
        parsed
            .into_iter()
            .map(|p| (p.scene.id.clone(), p.scene.resolved(&Present::Everyone)))
            .collect(),
    )
}

fn dialogue() -> DialogueTable {
    table(SCRIPT)
}

/// The id of the line written `text`, said by `speaker`, in `dialogue`.
fn id_in(dialogue: &DialogueTable, speaker: &str, text: &str) -> LineId {
    dialogue
        .scenes
        .values()
        .flat_map(Scene::lines)
        .find(|l| l.speaker == speaker && l.text == text)
        .unwrap()
        .id
        .clone()
}

fn id(speaker: &str, text: &str) -> LineId {
    id_in(&dialogue(), speaker, text)
}

fn voiced() -> BTreeSet<String> {
    BTreeSet::from(["a".to_owned()])
}

fn cast() -> Cast<()> {
    Cast::new(
        [
            (NARRATION_SPEAKER, "narrator"),
            ("guide", "guide_a"),
            ("guide.rain", "guide_rain"),
        ]
        .into_iter()
        .map(|(key, voice)| (key.to_owned(), CastVoice::new(voice, ())))
        .collect(),
    )
}

/// No clip for a reply, one per weather for a line with `{sky}`, else the
/// only one.
fn variants(speaker: &str, text: &str) -> &'static [Weather] {
    if speaker == REPLY_SPEAKER {
        &[]
    } else if text.contains("{sky}") {
        Weather::split()
    } else {
        &[Weather::Any]
    }
}

fn spoken(text: &str, weather: Weather) -> Option<String> {
    let sky = match weather {
        Weather::Sun => "sun",
        Weather::Rain => "rain",
        Weather::Any if text.contains("{sky}") => return None,
        Weather::Any => "",
    };
    Some(text.replace("{sky}", sky))
}

fn same_voice(_: &VoiceClip<Weather>, _: &CastVoice<()>) -> Option<String> {
    None
}

fn generated() -> MadeBy {
    MadeBy::Generated {
        tool: "t".to_owned(),
        model: "m".to_owned(),
        date: "2026-11-06".to_owned(),
    }
}

/// A generated clip that says `wanted` as planned.
fn made(wanted: &WantedClip<Weather>) -> VoiceClip<Weather> {
    VoiceClip {
        line: wanted.line.clone(),
        variant: wanted.variant,
        spoken: wanted.spoken.clone(),
        direction: wanted.direction.clone(),
        voice: cast()
            .voice_for(&wanted.speaker, wanted.variant)
            .unwrap()
            .voice
            .clone(),
        made_by: generated(),
    }
}

fn plan_of(clips: &[VoiceClip<Weather>]) -> VoicePlan<Weather> {
    plan(
        &dialogue(),
        &voiced(),
        &cast(),
        clips,
        variants,
        spoken,
        same_voice,
    )
}

/// Every clip the plan wants, made as it wants it.
fn all_made() -> Vec<VoiceClip<Weather>> {
    plan_of(&[]).wanted.iter().map(made).collect()
}

/// The `(line, variant)` of the wanted clip at each index of `of`, with why.
fn whys(plan: &VoicePlan<Weather>, of: &[(usize, Why)]) -> Vec<(LineId, Weather, Why)> {
    of.iter()
        .map(|(i, why)| {
            (
                plan.wanted[*i].line.clone(),
                plan.wanted[*i].variant,
                why.clone(),
            )
        })
        .collect()
}

#[test]
fn the_voiced_scenes_lines_are_wanted_in_script_order() {
    let plan = plan_of(&[]);
    let wanted: Vec<_> = plan
        .wanted
        .iter()
        .map(|w| {
            (
                w.speaker.as_str(),
                w.key.as_str(),
                w.spoken.as_str(),
                w.variant,
            )
        })
        .collect();
    assert_eq!(
        wanted,
        [
            (">", ">", "The gate opens.", Weather::Any),
            ("guide", "guide", "Hello.", Weather::Any),
            ("guide", "guide", "The sun is bright.", Weather::Sun),
            ("guide", "guide.rain", "The rain is bright.", Weather::Rain),
            (">", ">", "Look up.", Weather::Any),
            ("guide", "guide", "Good.", Weather::Any),
        ]
    );
    let hello = &plan.wanted[1];
    assert_eq!(hello.direction.as_deref(), Some("warm"));
    assert_eq!(
        hello.before,
        Some((">".to_owned(), "The gate opens.".to_owned()))
    );
    assert_eq!(plan.wanted[0].before, None);
    assert_eq!(hello.file(), format!("a/{}.ogg", id("guide", "Hello.")));
}

#[test]
fn a_line_without_a_clip_is_missing() {
    let plan = plan_of(&[]);
    assert_eq!(plan.make.len(), plan.wanted.len());
    assert!(plan.make.iter().all(|(_, why)| *why == Why::Missing));
    assert_eq!(Why::Missing.says(), "no clip");
}

#[test]
fn a_reply_needs_no_clip() {
    let plan = plan_of(&[]);
    assert!(plan.wanted.iter().all(|w| w.speaker != REPLY_SPEAKER));
    assert!(plan.uncast.iter().all(|u| u.speaker != REPLY_SPEAKER));
    // The reply is still the line before the reaction, for context.
    assert_eq!(
        plan.wanted[5].before,
        Some((REPLY_SPEAKER.to_owned(), "Hi.".to_owned()))
    );
}

#[test]
fn a_line_nobody_is_cast_to_say_is_uncast() {
    let plan = plan_of(&[]);
    assert_eq!(
        plan.uncast,
        [Uncast {
            line: id("stranger", "Who goes there?"),
            variant: Weather::Any,
            speaker: "stranger".to_owned(),
        }]
    );
}

#[test]
fn a_clip_with_old_words_is_made_again() {
    let mut clips = all_made();
    clips[1].spoken = "Hi.".to_owned();
    let plan = plan_of(&clips);
    assert_eq!(
        whys(&plan, &plan.make),
        [(id("guide", "Hello."), Weather::Any, Why::Words)]
    );
}

#[test]
fn a_clip_with_an_old_direction_is_made_again() {
    let mut clips = all_made();
    clips[1].direction = None;
    let plan = plan_of(&clips);
    assert_eq!(
        whys(&plan, &plan.make),
        [(id("guide", "Hello."), Weather::Any, Why::Direction)]
    );
}

#[test]
fn a_clip_of_another_voice_is_made_again() {
    let mut clips = all_made();
    clips[3].voice = "guide_a".to_owned();
    let plan = plan_of(&clips);
    assert_eq!(
        whys(&plan, &plan.make),
        [(
            id("guide", "The {sky} is bright."),
            Weather::Rain,
            Why::Recast
        )]
    );
}

#[test]
fn a_clip_whose_voice_the_tool_says_changed_is_made_again() {
    let clips = all_made();
    let changed = |clip: &VoiceClip<Weather>, voice: &CastVoice<()>| {
        (voice.voice == "narrator" && clip.spoken == "Look up.")
            .then(|| "the sample changed".to_owned())
    };
    let plan = plan(
        &dialogue(),
        &voiced(),
        &cast(),
        &clips,
        variants,
        spoken,
        changed,
    );
    let why = Why::Voice("the sample changed".to_owned());
    assert_eq!(
        whys(&plan, &plan.make),
        [(id(NARRATION_SPEAKER, "Look up."), Weather::Any, why.clone())]
    );
    assert_eq!(why.says(), "the sample changed");
}

#[test]
fn a_recorded_clip_is_recorded_again_for_its_words_and_direction_only() {
    let mut clips = all_made();
    for clip in &mut clips {
        clip.made_by = MadeBy::Recorded {
            actor: "Ann".to_owned(),
        };
        clip.voice = "someone else".to_owned();
    }
    clips[0].spoken = "The door opens.".to_owned();
    clips[1].direction = Some("cold".to_owned());
    let always = |_: &VoiceClip<Weather>, _: &CastVoice<()>| Some("changed".to_owned());
    let plan = plan(
        &dialogue(),
        &voiced(),
        &cast(),
        &clips,
        variants,
        spoken,
        always,
    );
    assert_eq!(plan.make, []);
    assert_eq!(
        whys(&plan, &plan.record),
        [
            (
                id(NARRATION_SPEAKER, "The gate opens."),
                Weather::Any,
                Why::Words
            ),
            (id("guide", "Hello."), Weather::Any, Why::Direction),
        ]
    );
}

#[test]
fn a_clip_whose_line_is_gone_is_removed() {
    let clips = all_made();
    let edited = table(&SCRIPT.replace("Hello.", "Hello there."));
    let plan = plan(
        &edited,
        &voiced(),
        &cast(),
        &clips,
        variants,
        spoken,
        same_voice,
    );
    assert_eq!(plan.remove, [clips[1].clone()]);
    let new = id_in(&edited, "guide", "Hello there.");
    assert_eq!(whys(&plan, &plan.make), [(new, Weather::Any, Why::Missing)]);
}

#[test]
fn a_clip_of_variants_the_line_no_longer_needs_is_removed() {
    let line = id("guide", "The {sky} is bright.");
    let mut clips = all_made();
    let only = VoiceClip {
        variant: Weather::Any,
        ..clips[2].clone()
    };
    clips.push(only.clone());
    let plan = plan_of(&clips);
    assert_eq!(plan.remove, [only]);
    assert_eq!(plan.remove[0].line, line);
}

#[test]
fn a_clip_of_a_scene_not_voiced_stays_while_its_line_exists() {
    let line = id("guide", "Not voiced.");
    let kept = VoiceClip {
        line: line.clone(),
        variant: Weather::Sun,
        spoken: "Not voiced.".to_owned(),
        direction: None,
        voice: "guide_a".to_owned(),
        made_by: generated(),
    };
    let mut clips = all_made();
    clips.push(kept.clone());
    let kept_plan = plan_of(&clips);
    assert_eq!(kept_plan.remove, []);
    assert!(kept_plan.wanted.iter().all(|w| w.line != line));

    let gone = table(&SCRIPT.replace("Not voiced.", "Still not voiced."));
    let gone_plan = plan(
        &gone,
        &voiced(),
        &cast(),
        &clips,
        variants,
        spoken,
        same_voice,
    );
    assert_eq!(gone_plan.remove, [kept]);
}

#[test]
fn an_uncast_lines_clip_is_kept() {
    let stranger = VoiceClip {
        line: id("stranger", "Who goes there?"),
        variant: Weather::Any,
        spoken: "Who goes there?".to_owned(),
        direction: None,
        voice: "stranger".to_owned(),
        made_by: generated(),
    };
    let plan = plan_of(&[stranger]);
    assert_eq!(plan.remove, []);
}

#[test]
fn a_recording_script_is_one_voices_clips_in_script_order() {
    let plan = plan_of(&[]);
    let script = |key: &str| -> Vec<(String, Option<(String, String)>)> {
        recording_script(&plan, key)
            .into_iter()
            .map(|w| (w.spoken.clone(), w.before.clone()))
            .collect()
    };
    let before = |speaker: &str, words: &str| Some((speaker.to_owned(), words.to_owned()));
    assert_eq!(
        script("guide"),
        [
            ("Hello.".to_owned(), before(">", "The gate opens.")),
            ("The sun is bright.".to_owned(), before("guide", "Hello.")),
            ("Good.".to_owned(), before(REPLY_SPEAKER, "Hi.")),
        ]
    );
    assert_eq!(
        script("guide.rain"),
        [("The rain is bright.".to_owned(), before("guide", "Hello."))]
    );
    // A line with variants is heard, before the next, in its first variant
    // that has words.
    assert_eq!(
        script(NARRATION_SPEAKER),
        [
            ("The gate opens.".to_owned(), None),
            ("Look up.".to_owned(), before("guide", "The sun is bright.")),
        ]
    );
    assert_eq!(script("nobody"), []);
}

#[test]
fn each_why_says_what_it_is() {
    let says: Vec<String> = [
        Why::Missing,
        Why::Words,
        Why::Direction,
        Why::Recast,
        Why::Voice("spelled differently".to_owned()),
    ]
    .iter()
    .map(|why| why.says().to_owned())
    .collect();
    assert_eq!(
        says,
        [
            "no clip",
            "the words changed",
            "the direction changed",
            "the cast names another voice",
            "spelled differently",
        ]
    );
}

proptest! {
    /// Clips made exactly as the plan wants them, in any order and from any
    /// subset of the voiced scenes' wants, leave nothing to make again,
    /// record or remove; only the clips left out are missing.
    #[test]
    fn a_manifest_made_as_planned_leaves_nothing_to_do(
        keep in proptest::collection::vec(any::<bool>(), 6),
        recorded in proptest::collection::vec(any::<bool>(), 6),
        reverse in any::<bool>(),
    ) {
        let wanted = plan_of(&[]).wanted;
        let mut clips: Vec<_> = wanted
            .iter()
            .zip(&recorded)
            .map(|(w, &r)| {
                let mut clip = made(w);
                if r {
                    clip.made_by = MadeBy::Recorded { actor: "Ann".to_owned() };
                }
                clip
            })
            .zip(&keep)
            .filter(|(_, k)| **k)
            .map(|(c, _)| c)
            .collect();
        if reverse {
            clips.reverse();
        }
        let plan = plan_of(&clips);
        prop_assert_eq!(&plan.wanted, &wanted);
        prop_assert!(plan.record.is_empty());
        prop_assert!(plan.remove.is_empty());
        prop_assert!(plan.make.iter().all(|(i, why)| *why == Why::Missing && !keep[*i]));
        prop_assert_eq!(plan.make.len(), keep.iter().filter(|k| !**k).count());
    }
}
