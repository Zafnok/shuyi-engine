//! Who is there when a scene plays. An `@if` block's steps play only for a
//! character who is [`Present`]; [`Scene::resolved`] gives the scene as it
//! plays for those present. Who that is, and which scripts must keep someone
//! inside an `@if` block, is the game's to say.
use crate::prelude::*;

use alloc::collections::BTreeSet;

use super::{CharacterId, ChoiceOption, Scene, Step};

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
