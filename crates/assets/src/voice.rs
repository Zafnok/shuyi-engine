//! Voice clips: a manifest that lists every clip by dialogue line id, and
//! a cast that says which voice says each speaker's lines. A game ships its
//! clips beside it, so nothing here reads a file: the game reads the text
//! and these functions parse and validate it.
//!
//! A clip records the words it says (`spoken`). When the line says
//! something else today (a name in the names table changed), the clip is
//! **stale** and is left out of the [`Playable`] set. A line whose words
//! change with something the player picks (a hero's pronouns, say) has one
//! clip per [`ClipVariant`] the game names.
use crate::prelude::*;

use alloc::collections::{BTreeMap, BTreeSet};
use core::fmt::Debug;

use serde::Deserialize;
use serde::de::DeserializeOwned;

use crate::dialogue::{DialogueTable, LineId, NARRATION_SPEAKER, Scene};
use crate::error::{ContentError, line_of};
use crate::ron::parse_ron;

/// Which recording of a line a clip is: the game's type, with its default
/// for a line's only clip.
pub trait ClipVariant: Copy + Ord + Default + Debug + DeserializeOwned + 'static {
    /// What goes between the line id and `.ogg` in the file name: nothing
    /// for the default, `.m` for a variant `M`, say.
    fn suffix(self) -> &'static str;

    /// Whether this is the default: the line's only clip.
    fn is_only(self) -> bool {
        self == Self::default()
    }

    /// The variants that go together, every one of them or none: the clips
    /// of a line whose words change.
    fn split() -> &'static [Self];

    /// What a line with some of the [`split`](Self::split) variants but not
    /// all is told.
    fn incomplete(line: &str) -> String {
        format!("line \"{line}\" lacks some of its variants' clips")
    }

    /// What a line with its only clip and split ones too is told.
    fn mixed(line: &str) -> String {
        format!("line \"{line}\" has an only clip and variants' clips; keep one kind")
    }
}

/// No voices: a line has only its only clip (`()`, as the engine says
/// "none" elsewhere).
impl ClipVariant for () {
    fn suffix(self) -> &'static str {
        ""
    }

    fn split() -> &'static [Self] {
        &[]
    }
}

/// Who or what made a clip (the credits say which voices are generated).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum MadeBy {
    /// Made by a text-to-speech tool.
    Generated {
        /// The tool.
        tool: String,
        /// The model or voice engine version.
        model: String,
        /// The day it was made, `YYYY-MM-DD`.
        date: String,
    },
    /// Recorded by a person.
    Recorded {
        /// The actor, as they want to be credited.
        actor: String,
    },
}

/// One clip of the manifest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VoiceClip<V> {
    /// The dialogue line it says.
    pub line: LineId,
    /// Which recording of the line it is.
    pub variant: V,
    /// The words it says: the line's text with every token filled in, as
    /// it was when the clip was made.
    pub spoken: String,
    /// The cast voice that says it (a value of the [`Cast`]).
    pub voice: String,
    /// Who or what made it.
    pub made_by: MadeBy,
}

/// The validated voice manifest of one language.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VoiceManifest<V> {
    /// Every clip, in file order.
    pub clips: Vec<VoiceClip<V>>,
}

impl<V> Default for VoiceManifest<V> {
    fn default() -> Self {
        Self { clips: Vec::new() }
    }
}

/// Which voice says each speaker's lines.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Cast {
    /// Voice ids by speaker: a character id, or
    /// [`NARRATION_SPEAKER`] for narration.
    pub voices: BTreeMap<String, String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(bound = "V: ClipVariant")]
struct RawManifest<V> {
    clips: Vec<RawClip<V>>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(bound = "V: ClipVariant")]
struct RawClip<V> {
    line: String,
    #[serde(default)]
    variant: V,
    spoken: String,
    voice: String,
    made_by: MadeBy,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawCast {
    voices: BTreeMap<String, String>,
}

/// A clip's file, relative to its language's folder:
/// `<scene id>/<line id>.ogg`, with the variant's
/// [`suffix`](ClipVariant::suffix) before `.ogg`.
pub fn clip_file<V: ClipVariant>(line: &LineId, variant: V) -> String {
    format!("{}/{line}{}.ogg", line.scene_id(), variant.suffix())
}

/// The text of every dialogue line, by line id.
pub fn line_texts<D>(dialogue: &DialogueTable<D>) -> BTreeMap<&str, &str> {
    dialogue
        .scenes
        .values()
        .flat_map(Scene::lines)
        .map(|line| (line.id.as_str(), line.text))
        .collect()
}

/// Parses and validates manifest `source`, attributing errors to `file`:
/// every clip's line exists in `dialogue`, no line has the same variant
/// twice, a line has either its only clip or every
/// [`split`](ClipVariant::split) one, and `spoken` and `voice` aren't
/// empty. Reports every problem found.
///
/// A stale clip is not an error (see [`VoiceManifest::playable`]).
pub fn from_source<V: ClipVariant, D>(
    file: &str,
    source: &str,
    dialogue: &DialogueTable<D>,
) -> Result<VoiceManifest<V>, Vec<ContentError>> {
    let raw: RawManifest<V> = parse_ron(file, source).map_err(|e| vec![e])?;
    let texts = line_texts(dialogue);
    let mut errors = Vec::new();
    let mut err = |line: &str, message: String| {
        let e = ContentError::new(file, message);
        errors.push(match line_of(source, &format!("\"{line}\"")) {
            Some(l) => e.at(l, None),
            None => e,
        });
    };
    let mut variants: BTreeMap<&str, Vec<V>> = BTreeMap::new();
    for clip in &raw.clips {
        let id = clip.line.as_str();
        if !texts.contains_key(id) {
            err(id, format!("no dialogue line has the id \"{id}\""));
        }
        let seen = variants.entry(id).or_default();
        if seen.contains(&clip.variant) {
            err(
                id,
                format!("line \"{id}\" has two {:?} clips", clip.variant),
            );
        }
        seen.push(clip.variant);
        if clip.spoken.trim().is_empty() {
            err(id, format!("the clip of line \"{id}\" has no spoken text"));
        }
        if clip.voice.trim().is_empty() {
            err(id, format!("the clip of line \"{id}\" has no voice"));
        }
    }
    for (&id, seen) in &variants {
        let split = V::split();
        let some = split.iter().any(|v| seen.contains(v));
        if some && !split.iter().all(|v| seen.contains(v)) {
            err(id, V::incomplete(id));
        }
        if some && seen.iter().any(|v| v.is_only()) {
            err(id, V::mixed(id));
        }
    }
    if !errors.is_empty() {
        return Err(errors);
    }
    let clips = raw
        .clips
        .into_iter()
        .map(|c| VoiceClip {
            line: LineId::new(c.line),
            variant: c.variant,
            spoken: c.spoken,
            voice: c.voice,
            made_by: c.made_by,
        })
        .collect();
    Ok(VoiceManifest { clips })
}

/// Parses and validates cast `source`, attributing errors to `file`: no
/// voice id is empty, and every speaker is [`NARRATION_SPEAKER`] or one
/// that `can_speak`.
pub fn cast_from_source(
    file: &str,
    source: &str,
    can_speak: impl Fn(&str) -> bool,
) -> Result<Cast, Vec<ContentError>> {
    let raw: RawCast = parse_ron(file, source).map_err(|e| vec![e])?;
    let mut errors = Vec::new();
    for (speaker, voice) in &raw.voices {
        let mut err = |message: String| {
            let e = ContentError::new(file, message);
            errors.push(match line_of(source, &format!("\"{speaker}\"")) {
                Some(l) => e.at(l, None),
                None => e,
            });
        };
        if voice.trim().is_empty() {
            err(format!("\"{speaker}\" has no voice"));
        }
        if speaker != NARRATION_SPEAKER && !can_speak(speaker) {
            err(format!("\"{speaker}\" is not a character that speaks"));
        }
    }
    if errors.is_empty() {
        Ok(Cast { voices: raw.voices })
    } else {
        Err(errors)
    }
}

/// The clips that may be played: those that say what their line says
/// today.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Playable<V> {
    clips: BTreeSet<(LineId, V)>,
}

impl<V> Default for Playable<V> {
    fn default() -> Self {
        Self {
            clips: BTreeSet::new(),
        }
    }
}

impl<V: ClipVariant> Playable<V> {
    /// The clip to play for `line` as `variant` plays it: the line's only
    /// clip, else `variant`'s. `None` if the line has no playable clip.
    pub fn variant_for(&self, line: &LineId, variant: V) -> Option<V> {
        [V::default(), variant]
            .into_iter()
            .find(|&v| self.clips.contains(&(line.clone(), v)))
    }

    /// How many clips may be played.
    pub fn len(&self) -> usize {
        self.clips.len()
    }

    /// Whether no clip may be played.
    pub fn is_empty(&self) -> bool {
        self.clips.is_empty()
    }
}

impl<V: ClipVariant> VoiceManifest<V> {
    /// Whether `clip` is stale: `spoken` (the words a line written as the
    /// text it is given says, as played for a variant; `None` when no clip
    /// can say them) is not what the clip says. A clip of a line that is
    /// gone is stale too.
    pub fn is_stale<D>(
        clip: &VoiceClip<V>,
        dialogue: &DialogueTable<D>,
        spoken: impl Fn(&str, V) -> Option<String>,
    ) -> bool {
        let Some(&text) = line_texts(dialogue).get(clip.line.as_str()) else {
            return true;
        };
        stale(clip, text, &spoken)
    }

    /// The clips that aren't stale ([`is_stale`](Self::is_stale)).
    pub fn playable<D>(
        &self,
        dialogue: &DialogueTable<D>,
        spoken: impl Fn(&str, V) -> Option<String>,
    ) -> Playable<V> {
        let texts = line_texts(dialogue);
        let clips = self
            .clips
            .iter()
            .filter(|clip| {
                texts
                    .get(clip.line.as_str())
                    .is_some_and(|text| !stale(clip, text, &spoken))
            })
            .map(|clip| (clip.line.clone(), clip.variant))
            .collect();
        Playable { clips }
    }
}

/// Whether `clip`, of the line written `text`, is stale.
fn stale<V: ClipVariant>(
    clip: &VoiceClip<V>,
    text: &str,
    spoken: &impl Fn(&str, V) -> Option<String>,
) -> bool {
    spoken(text, clip.variant).is_none_or(|words| words != clip.spoken)
}

#[cfg(test)]
mod tests;
