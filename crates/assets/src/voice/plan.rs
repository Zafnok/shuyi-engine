//! What a voice tool has to do for one language, as a pure function of the
//! dialogue, the scenes to voice, the cast and the manifest's clips: the
//! clips to make or make again (and why), the recorded clips a person must
//! record again, the clips to remove and the lines nobody is cast to say.
//! The tool reads and writes its files; [`plan`] only says what to do.
//!
//! The tool brings three hooks:
//!
//! - which variants a line needs, from its speaker and text (none for a
//!   line nobody says aloud, such as a reply);
//! - the words a line says as a variant (the hook
//!   [`VoiceManifest::is_stale`](super::VoiceManifest::is_stale) takes);
//! - whether the voice of a generated clip changed, in the tool's own
//!   words ("the sample changed"). The plan itself compares the clip's
//!   [`voice`](VoiceClip::voice) with the cast's.
use crate::prelude::*;

use alloc::collections::{BTreeMap, BTreeSet};

use super::{Cast, CastVoice, ClipVariant, MadeBy, VoiceClip, clip_file};
use crate::dialogue::{DialogueTable, LineId};

/// Why a clip is to be made, or recorded, again.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Why {
    /// The line has no clip.
    Missing,
    /// The clip says other words than the line says today.
    Words,
    /// The line's direction changed.
    Direction,
    /// The cast names another voice than the one that made the clip.
    Recast,
    /// The tool says the voice changed, and why, in its own words.
    Voice(String),
}

impl Why {
    /// What a tool's report says of it.
    pub fn says(&self) -> &str {
        match self {
            Self::Missing => "no clip",
            Self::Words => "the words changed",
            Self::Direction => "the direction changed",
            Self::Recast => "the cast names another voice",
            Self::Voice(reason) => reason,
        }
    }
}

/// A clip a voiced line needs, with what the voice needs to say it.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct WantedClip<V> {
    /// The line.
    pub line: LineId,
    /// Which of its clips.
    pub variant: V,
    /// Who says it: a character id, or the narration speaker.
    pub speaker: String,
    /// The cast key whose voice says it: the speaker and the variant's
    /// suffix if the cast has that key, else the speaker.
    pub key: String,
    /// The words it says (what the manifest's `spoken` will be).
    pub spoken: String,
    /// How it is said.
    pub direction: Option<String>,
    /// The line before it in the scene, for context: its speaker and
    /// words.
    pub before: Option<(String, String)>,
}

impl<V: ClipVariant> WantedClip<V> {
    /// Its file, relative to the language's folder ([`clip_file`]).
    pub fn file(&self) -> String {
        clip_file(&self.line, self.variant)
    }
}

/// A clip a voiced line needs whose speaker the cast gives no voice.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Uncast<V> {
    /// The line.
    pub line: LineId,
    /// Which of its clips.
    pub variant: V,
    /// Who says it.
    pub speaker: String,
}

/// Everything there is to do for one language's clips.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct VoicePlan<V> {
    /// Every clip the voiced lines need and the cast can say, by scene id,
    /// then in script order.
    pub wanted: Vec<WantedClip<V>>,
    /// The generated (or missing) clips of [`wanted`](Self::wanted) to
    /// make, by index.
    pub make: Vec<(usize, Why)>,
    /// The recorded clips of [`wanted`](Self::wanted) a person must record
    /// again, by index: judged by their words and direction only.
    pub record: Vec<(usize, Why)>,
    /// Clips to remove: their line is gone, or their line, in a voiced
    /// scene, needs other variants. A clip of a scene that isn't voiced
    /// stays while its line exists.
    pub remove: Vec<VoiceClip<V>>,
    /// The clips voiced lines need whose speaker has no voice in the cast.
    pub uncast: Vec<Uncast<V>>,
}

impl<V> Default for VoicePlan<V> {
    fn default() -> Self {
        Self {
            wanted: Vec::new(),
            make: Vec::new(),
            record: Vec::new(),
            remove: Vec::new(),
            uncast: Vec::new(),
        }
    }
}

/// Plans `dialogue`'s clips: the lines of the scenes in `voiced` need a
/// clip for each variant `variants(speaker, text)` names, saying
/// `spoken(text, variant)` (a variant whose words are `None` needs none).
/// `clips` are the manifest's. A generated clip whose voice is still the
/// cast's is made again if `voice_changed(clip, cast voice)` gives a
/// reason.
pub fn plan<V: ClipVariant, S, D>(
    dialogue: &DialogueTable<D>,
    voiced: &BTreeSet<String>,
    cast: &Cast<S>,
    clips: &[VoiceClip<V>],
    variants: impl Fn(&str, &str) -> &'static [V],
    spoken: impl Fn(&str, V) -> Option<String>,
    voice_changed: impl Fn(&VoiceClip<V>, &CastVoice<S>) -> Option<String>,
) -> VoicePlan<V> {
    let mut plan = VoicePlan::default();
    let mut needed = BTreeSet::new();
    let mut in_voiced = BTreeSet::new();
    let mut all = BTreeSet::new();
    for (id, scene) in &dialogue.scenes {
        let lines = scene.lines();
        all.extend(lines.iter().map(|l| l.id.clone()));
        if !voiced.contains(id) {
            continue;
        }
        let mut before = None;
        for line in lines {
            in_voiced.insert(line.id.clone());
            for &variant in variants(line.speaker, line.text) {
                let Some(words) = spoken(line.text, variant) else {
                    continue;
                };
                needed.insert((line.id.clone(), variant));
                let Some(key) = key_for(cast, line.speaker, variant) else {
                    plan.uncast.push(Uncast {
                        line: line.id.clone(),
                        variant,
                        speaker: line.speaker.to_owned(),
                    });
                    continue;
                };
                plan.wanted.push(WantedClip {
                    line: line.id.clone(),
                    variant,
                    speaker: line.speaker.to_owned(),
                    key,
                    spoken: words,
                    direction: line.direction.map(str::to_owned),
                    before: before.clone(),
                });
            }
            let shown = V::variants()
                .into_iter()
                .find_map(|v| spoken(line.text, v))
                .unwrap_or_else(|| line.text.to_owned());
            before = Some((line.speaker.to_owned(), shown));
        }
    }
    let made: BTreeMap<(&LineId, V), &VoiceClip<V>> = clips
        .iter()
        .map(|clip| ((&clip.line, clip.variant), clip))
        .collect();
    for (i, wanted) in plan.wanted.iter().enumerate() {
        let Some(&clip) = made.get(&(&wanted.line, wanted.variant)) else {
            plan.make.push((i, Why::Missing));
            continue;
        };
        let voice = &cast.voices[&wanted.key];
        if let Some(why) = why_again(clip, wanted, voice, &voice_changed) {
            match clip.made_by {
                MadeBy::Recorded { .. } => plan.record.push((i, why)),
                MadeBy::Generated { .. } => plan.make.push((i, why)),
            }
        }
    }
    plan.remove = clips
        .iter()
        .filter(|clip| {
            let ours = in_voiced.contains(&clip.line) || !all.contains(&clip.line);
            ours && !needed.contains(&(clip.line.clone(), clip.variant))
        })
        .cloned()
        .collect();
    plan
}

/// The cast key whose voice says `speaker`'s clips of `variant`, as
/// [`Cast::voice_for`] picks it.
fn key_for<S, V: ClipVariant>(cast: &Cast<S>, speaker: &str, variant: V) -> Option<String> {
    let own = format!("{speaker}{}", variant.suffix());
    [own.as_str(), speaker]
        .into_iter()
        .find(|key| cast.voices.contains_key(*key))
        .map(str::to_owned)
}

/// Why `clip` must be made, or recorded, again as `wanted` with the cast's
/// `voice`, if it must. A recorded clip is judged by its words and
/// direction only.
fn why_again<V, S>(
    clip: &VoiceClip<V>,
    wanted: &WantedClip<V>,
    voice: &CastVoice<S>,
    voice_changed: &impl Fn(&VoiceClip<V>, &CastVoice<S>) -> Option<String>,
) -> Option<Why> {
    if clip.spoken != wanted.spoken {
        Some(Why::Words)
    } else if clip.direction != wanted.direction {
        Some(Why::Direction)
    } else if matches!(clip.made_by, MadeBy::Recorded { .. }) {
        None
    } else if clip.voice != voice.voice {
        Some(Why::Recast)
    } else {
        voice_changed(clip, voice).map(Why::Voice)
    }
}

/// The clips of `plan` the voice under cast key `key` says, in the plan's
/// order (by scene, then script order): what that voice records in a
/// session, each with the line before it.
pub fn recording_script<'a, V>(plan: &'a VoicePlan<V>, key: &str) -> Vec<&'a WantedClip<V>> {
    plan.wanted.iter().filter(|w| w.key == key).collect()
}

#[cfg(test)]
mod tests;
