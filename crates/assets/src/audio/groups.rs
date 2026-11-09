//! The ids of the built-in sound groups (ADR-0091): every sound, track and
//! voice clip plays in a group, and every group under [`MASTER`]. [`MUSIC`],
//! [`SFX`] and [`VOICE`] are under [`MASTER`], and [`UI`] under [`SFX`].
//! The tree and the player's volumes on it are `shuyi-ui`'s
//! (`shuyi_ui::audio::groups`); the manifest's rules name groups by these
//! ids.
//!
//! A game declares its own groups in the manifest's `groups` list
//! ([`GroupDef`]), and sets a built-in group's default there too; this
//! module checks them.

use crate::prelude::*;

use serde::{Deserialize, Serialize};

use super::Check;

/// The group over every other: its volume lowers everything.
pub const MASTER: &str = "master";
/// The music's group, under [`MASTER`].
pub const MUSIC: &str = "music";
/// The sounds' group, under [`MASTER`]: every sound cue plays in it unless
/// it is a menu's.
pub const SFX: &str = "sfx";
/// The voice clips' group, under [`MASTER`].
pub const VOICE: &str = "voice";
/// The menus' sounds' group, under [`SFX`]: the cues the engine's widgets
/// play.
pub const UI: &str = "ui";

/// The built-in groups, parents before their children, each with its
/// parent (`None` for [`MASTER`]).
pub const BUILT_IN: [(&str, Option<&str>); 5] = [
    (MASTER, None),
    (MUSIC, Some(MASTER)),
    (SFX, Some(MASTER)),
    (VOICE, Some(MASTER)),
    (UI, Some(SFX)),
];

/// Whether `group` is one of the [`BUILT_IN`] groups.
#[must_use]
pub fn is_built_in(group: &str) -> bool {
    BUILT_IN.iter().any(|&(id, _)| id == group)
}

/// A sound group as the manifest declares it: a game's own, or a built-in
/// one whose default the game sets (`volume` alone). Build it with
/// `default()` and assign its fields (ADR-0039).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[non_exhaustive]
pub struct GroupDef {
    /// The group it is under: `None` is [`MASTER`]. Always `None` on a
    /// built-in group, which keeps its own parent.
    pub parent: Option<String>,
    /// The volume a new player starts at (0–100): `None` is the group's
    /// default (97 for a game's own, as for the built-ins under
    /// [`MASTER`]).
    pub volume: Option<u8>,
}

/// One entry of the manifest's `groups` list.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RawGroup {
    id: String,
    #[serde(default)]
    parent: Option<String>,
    #[serde(default)]
    volume: Option<u8>,
}

impl Check<'_> {
    /// Checks and keeps the manifest's `groups`: their ids, parents and
    /// volumes, and that no group is its own ancestor.
    pub(super) fn groups(&mut self, groups: Vec<RawGroup>) {
        for RawGroup { id, parent, volume } in groups {
            if id.is_empty() {
                self.err(&id, "a group id is empty".into());
            }
            if self.manifest.groups.contains_key(&id) {
                self.err(&id, format!("duplicate group id \"{id}\""));
            }
            if is_built_in(&id) && parent.is_some() {
                let why = "is built in: it may set only its volume, not its parent";
                self.err(&id, format!("group \"{id}\" {why}"));
            }
            if let Some(volume) = volume {
                self.volume(&id, volume);
            }
            self.manifest.groups.insert(id, GroupDef { parent, volume });
        }
        let mut problems = Vec::new();
        for (id, group) in &self.manifest.groups {
            match &group.parent {
                Some(parent) if !self.is_group(parent) => problems.push((
                    id.clone(),
                    format!("group \"{id}\" names unknown parent \"{parent}\""),
                )),
                Some(_) => {
                    if let Some(chain) = self.loop_from(id) {
                        let why = format!("group \"{id}\" is in a loop of parents: {chain}");
                        problems.push((id.clone(), why));
                    }
                }
                None => {}
            }
        }
        for (id, why) in problems {
            self.err(&id, why);
        }
    }

    /// Whether `group` is built in or declared.
    pub(super) fn is_group(&self, group: &str) -> bool {
        is_built_in(group) || self.manifest.groups.contains_key(group)
    }

    /// The parents from `id` back to itself (`a, b, a`), if it is its own
    /// ancestor among the declared groups.
    fn loop_from(&self, id: &str) -> Option<String> {
        let mut chain = vec![id];
        let mut at = id;
        while let Some(parent) = self.manifest.groups.get(at)?.parent.as_deref() {
            chain.push(parent);
            if parent == id {
                return Some(chain.join(", "));
            }
            if chain.len() > self.manifest.groups.len() + 1 {
                return None;
            }
            at = parent;
        }
        None
    }

    /// Reports cue `id` if it names a group there isn't.
    pub(super) fn cue_group(&mut self, kind: &str, id: &str, group: Option<&str>) {
        if let Some(group) = group.filter(|g| !self.is_group(g)) {
            self.err(
                id,
                format!("{kind} \"{id}\" names unknown group \"{group}\""),
            );
        }
    }
}
