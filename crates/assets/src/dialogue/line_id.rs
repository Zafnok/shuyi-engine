//! Line ids: every speech line, narration line and reply has
//! an id `<scene id>_<8 hex>`, a hash of who says it and its English text
//! as written. Translations and voice clips attach to it, so rewording a
//! line gives it a new id and moving it does not.
use crate::prelude::*;
use serde::{Deserialize, Serialize};

use alloc::collections::BTreeMap;
use core::fmt;

use super::{Scene, Step};

/// What stands for the speaker in a narration line's hash and listing.
pub const NARRATION_SPEAKER: &str = ">";
/// What stands for the speaker in a reply's hash and listing.
pub const REPLY_SPEAKER: &str = "*";

/// The id of one dialogue line: `<scene id>_<8 hex>`, with `_2`, `_3`… on
/// repeats of the same speaker and text in one scene. Unique across every
/// dialogue file. Empty on a step built by hand until
/// [`Scene::assign_line_ids`] runs (the parser runs it).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize)]
pub struct LineId(String);

impl LineId {
    /// The id written `id`, as a file that refers to lines names one (a
    /// voice manifest). Nothing checks that a line has it.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// The id as text.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The id of the scene the line is in: the id without its hash and
    /// repeat number (`ch1_gate_1a2b3c4d_2` → `ch1_gate`). An id that
    /// doesn't end like a line id is returned whole.
    pub fn scene_id(&self) -> &str {
        scene_of(&self.0).unwrap_or(&self.0)
    }
}

/// The id of the scene the line with id `id` is in, or `None` if `id`
/// doesn't end like a line id (`_<8 hex>`, then `_<n>` on a repeat).
pub fn scene_of(id: &str) -> Option<&str> {
    /// `id` without a `_<8 hex>` ending, if it has one.
    fn strip_hash(id: &str) -> Option<&str> {
        id.rsplit_once('_')
            .filter(|(_, tail)| tail.len() == 8 && tail.bytes().all(|b| b.is_ascii_hexdigit()))
            .map(|(scene, _)| scene)
    }
    // A repeat: `<scene>_<hash>_<n>`.
    let repeat = || {
        id.rsplit_once('_')
            .filter(|(_, n)| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
            .and_then(|(rest, _)| strip_hash(rest))
    };
    strip_hash(id).or_else(repeat)
}

impl fmt::Display for LineId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// One line that shows text, as [`Scene::lines`] lists it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Line<'a> {
    /// Its id.
    pub id: &'a LineId,
    /// Who says it: a character id, [`NARRATION_SPEAKER`] or
    /// [`REPLY_SPEAKER`].
    pub speaker: &'a str,
    /// Its text as written (tokens unexpanded).
    pub text: &'a str,
}

/// FNV-1a, 64 bits. Written here because line ids are stored in translation
/// and voice files: the hash must never change (`std`'s hasher may).
pub(super) fn fnv1a64(bytes: &[u8]) -> u64 {
    const OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;
    bytes.iter().fold(OFFSET_BASIS, |hash, &b| {
        (hash ^ u64::from(b)).wrapping_mul(PRIME)
    })
}

/// The hash part of a line id: the low 32 bits of FNV-1a 64 over
/// `speaker`, a newline and `text`, as 8 lowercase hex digits.
pub fn line_hash(speaker: &str, text: &str) -> String {
    let hash = fnv1a64(format!("{speaker}\n{text}").as_bytes());
    format!("{:08x}", hash & 0xffff_ffff)
}

/// What stands for the speaker in a caption's hash.
const CAPTION_SPEAKER: &str = "@caption";

/// The key a language pack translates the caption `text` of scene `scene`
/// by: `caption.<scene id>_<8 hex>`, hashed like a line id.
/// The same caption twice in a scene has one key.
pub fn caption_key(scene: &str, text: &str) -> String {
    format!("caption.{scene}_{}", line_hash(CAPTION_SPEAKER, text))
}

/// Calls `f(speaker, text, id)` for every line of `steps` that shows text,
/// in script order: a reply, then its reaction, then the next reply; an
/// `@if` block's lines, then its `@else`'s.
fn visit_mut<D>(steps: &mut [Step<D>], f: &mut impl FnMut(&str, &str, &mut LineId)) {
    for step in steps {
        match step {
            Step::Say {
                speaker,
                text,
                line,
                ..
            } => f(&speaker.0, text, line),
            Step::Narrate { text, line } => f(NARRATION_SPEAKER, text, line),
            Step::Choice { options } => {
                for o in options {
                    f(REPLY_SPEAKER, &o.text, &mut o.line);
                    visit_mut(&mut o.steps, f);
                }
            }
            Step::If {
                then, otherwise, ..
            } => {
                visit_mut(then, f);
                visit_mut(otherwise, f);
            }
            Step::Caption { .. }
            | Step::Place { .. }
            | Step::Clear { .. }
            | Step::Music(_)
            | Step::Directive(_) => {}
        }
    }
}

/// Adds the lines of `steps` to `out`, in the order of [`visit_mut`].
fn collect<'a, D>(steps: &'a [Step<D>], out: &mut Vec<Line<'a>>) {
    for step in steps {
        match step {
            Step::Say {
                speaker,
                text,
                line,
                ..
            } => out.push(Line {
                id: line,
                speaker: &speaker.0,
                text,
            }),
            Step::Narrate { text, line } => out.push(Line {
                id: line,
                speaker: NARRATION_SPEAKER,
                text,
            }),
            Step::Choice { options } => {
                for o in options {
                    out.push(Line {
                        id: &o.line,
                        speaker: REPLY_SPEAKER,
                        text: &o.text,
                    });
                    collect(&o.steps, out);
                }
            }
            Step::If {
                then, otherwise, ..
            } => {
                collect(then, out);
                collect(otherwise, out);
            }
            Step::Caption { .. }
            | Step::Place { .. }
            | Step::Clear { .. }
            | Step::Music(_)
            | Step::Directive(_) => {}
        }
    }
}

impl<D> Scene<D> {
    /// Every line that shows text (speech, narration, replies and the lines
    /// of their reactions), in script order, whoever is there: both parts
    /// of every `@if` block.
    pub fn lines(&self) -> Vec<Line<'_>> {
        let mut out = Vec::new();
        collect(&self.steps, &mut out);
        out
    }

    /// Gives every line of the scene its id, from the scene id, the
    /// speakers and the texts as they are now. The parser calls this on
    /// each scene it reads; call it after building or editing a scene by
    /// hand.
    pub fn assign_line_ids(&mut self) {
        let id = &self.id;
        let mut seen: BTreeMap<String, u32> = BTreeMap::new();
        visit_mut(&mut self.steps, &mut |speaker, text, line| {
            let hash = line_hash(speaker, text);
            let n = seen.entry(hash.clone()).or_insert(0);
            *n += 1;
            *line = LineId(if *n == 1 {
                format!("{id}_{hash}")
            } else {
                format!("{id}_{hash}_{n}")
            });
        });
    }
}
