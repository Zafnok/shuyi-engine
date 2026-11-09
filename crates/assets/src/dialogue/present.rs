//! Who is there when a scene plays. An `@if` block's steps play only for a
//! character who is [`Present`]; [`Scene::resolved`] gives the scene as it
//! plays for those present. Who that is, and which scripts must keep someone
//! inside an `@if` block, is the game's to say. [`Scene::conditions`] lists
//! who a scene's blocks test, and [`ParsedScene::text_boxes`] where each
//! text box it plays is in its file.
use crate::prelude::*;

use alloc::collections::BTreeSet;

use super::{CharacterId, ChoiceOption, Lines, ParsedScene, Scene, Step};

/// Who is there when a scene plays.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Present {
    /// Everyone: every `@if` block plays its first part (a scene viewer; a
    /// scene played outside a story).
    Everyone,
    /// Only these characters.
    Only(BTreeSet<CharacterId>),
}

impl Present {
    /// Only `characters`.
    pub fn only(characters: impl IntoIterator<Item = CharacterId>) -> Self {
        Self::Only(characters.into_iter().collect())
    }

    /// Whether `character` is there.
    pub fn has(&self, character: &CharacterId) -> bool {
        match self {
            Present::Everyone => true,
            Present::Only(characters) => characters.contains(character),
        }
    }
}

impl<D: Clone> Scene<D> {
    /// The scene as it plays for those `present`: every `@if` block
    /// replaced by the steps of the part that holds, in reactions and in
    /// other blocks too. The lines keep their ids.
    #[must_use]
    pub fn resolved(&self, present: &Present) -> Scene<D> {
        Scene {
            id: self.id.clone(),
            steps: resolved(&self.steps, present),
        }
    }
}

impl<D> Scene<D> {
    /// Whether the scene has a speech or narration line to play, whichever
    /// reply is picked. Run on a [`Scene::resolved`] scene: one with
    /// nothing left to say for those present isn't played.
    pub fn has_text(&self) -> bool {
        self.steps.iter().any(Step::has_text)
    }

    /// Every character an `@if` block of the scene tests, in reactions and
    /// in other blocks too, each once, in the order they first appear: who
    /// a preview offers to send away, to see each part played.
    pub fn conditions(&self) -> Vec<CharacterId> {
        let mut found = Vec::new();
        conditions(&self.steps, &mut found);
        found
    }
}

/// Adds the characters the `@if` blocks of `steps` test to `found`, those
/// not in it yet.
fn conditions<D>(steps: &[Step<D>], found: &mut Vec<CharacterId>) {
    for step in steps {
        match step {
            Step::If {
                character,
                then,
                otherwise,
            } => {
                if !found.contains(character) {
                    found.push(character.clone());
                }
                conditions(then, found);
                conditions(otherwise, found);
            }
            Step::Choice { options } => {
                for option in options {
                    conditions(&option.steps, found);
                }
            }
            _ => {}
        }
    }
}

/// One text box of a scene as it plays: where its line is, and the line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextBoxAt<'a, D> {
    /// The 1-based line of the file where its step starts.
    pub line: u32,
    /// The speech or narration step.
    pub step: &'a Step<D>,
}

impl<D> ParsedScene<D> {
    /// The scene's text boxes (its speech and narration lines), in the
    /// order they play for those `present`, with where each is: at the
    /// `n`-th reply choice reached, the reply `replies[n]` is taken, the
    /// first when the list has no `n`-th or names no such reply. A choice
    /// with no replies is passed over, as the player passes it.
    pub fn text_boxes(&self, present: &Present, replies: &[usize]) -> Vec<TextBoxAt<'_, D>> {
        let mut out = Vec::new();
        let mut replies = replies.iter().copied();
        text_boxes(
            &self.scene.steps,
            &self.lines,
            present,
            &mut replies,
            &mut out,
        );
        out
    }

    /// Which of [`text_boxes`](Self::text_boxes) the file's line `line`
    /// falls in: the last one that starts on it or before it (a line that
    /// goes on below its first, a portrait or music line after it), the
    /// first for a line before them all. `None` for a scene that has no
    /// text box for those `present`.
    pub fn text_box_at(&self, line: u32, present: &Present, replies: &[usize]) -> Option<usize> {
        let boxes = self.text_boxes(present, replies);
        if boxes.is_empty() {
            return None;
        }
        Some(boxes.iter().rposition(|b| b.line <= line).unwrap_or(0))
    }
}

/// Where the steps of a block part the parser left no record of are: nowhere.
static NO_LINES: Lines = Lines {
    steps: Vec::new(),
    blocks: Vec::new(),
};

/// Adds the text boxes of `steps`, found at `lines`, to `out`, as they
/// play for those `present`, taking the replies from `replies`.
fn text_boxes<'a, D>(
    steps: &'a [Step<D>],
    lines: &Lines,
    present: &Present,
    replies: &mut impl Iterator<Item = usize>,
    out: &mut Vec<TextBoxAt<'a, D>>,
) {
    let mut blocks = lines.blocks.iter();
    for (i, step) in steps.iter().enumerate() {
        let line = lines.steps.get(i).copied().unwrap_or(0);
        match step {
            Step::Say { .. } | Step::Narrate { .. } => out.push(TextBoxAt { line, step }),
            Step::Choice { options } => {
                let parts = blocks.next().map_or(&[][..], Vec::as_slice);
                if options.is_empty() {
                    continue;
                }
                let pick = replies.next().filter(|&r| r < options.len()).unwrap_or(0);
                let lines = parts.get(pick).map_or(&NO_LINES, |p| &p.lines);
                text_boxes(&options[pick].steps, lines, present, replies, out);
            }
            Step::If {
                character,
                then,
                otherwise,
            } => {
                let parts = blocks.next().map_or(&[][..], Vec::as_slice);
                let (part, steps) = if present.has(character) {
                    (0, then)
                } else {
                    (1, otherwise)
                };
                let lines = parts.get(part).map_or(&NO_LINES, |p| &p.lines);
                text_boxes(steps, lines, present, replies, out);
            }
            _ => {}
        }
    }
}

/// `steps` with every `@if` block replaced by the part that holds for
/// those `present`.
fn resolved<D: Clone>(steps: &[Step<D>], present: &Present) -> Vec<Step<D>> {
    let mut out = Vec::with_capacity(steps.len());
    for step in steps {
        match step {
            Step::If {
                character,
                then,
                otherwise,
            } => {
                let part = if present.has(character) {
                    then
                } else {
                    otherwise
                };
                out.extend(resolved(part, present));
            }
            Step::Choice { options } => {
                let option = |o: &ChoiceOption<D>| ChoiceOption {
                    steps: resolved(&o.steps, present),
                    ..o.clone()
                };
                out.push(Step::Choice {
                    options: options.iter().map(option).collect(),
                });
            }
            other => out.push(other.clone()),
        }
    }
    out
}
