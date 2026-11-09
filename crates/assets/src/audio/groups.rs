//! The ids of the built-in sound groups (ADR-0091): every sound, track and
//! voice clip plays in a group, and every group under [`MASTER`]. [`MUSIC`],
//! [`SFX`] and [`VOICE`] are under [`MASTER`], and [`UI`] under [`SFX`].
//! The tree and the player's volumes on it are `shuyi-ui`'s
//! (`shuyi_ui::audio::groups`); the manifest's rules name groups by these
//! ids.
//!
//! A game declares its own groups in the manifest's `groups` list
//! ([`GroupDef`]), and sets a built-in group's default there too, and
//! lowers a group while a voice plays in its `ducks` list ([`DuckRule`],
//! ADR-0099); this module checks them.

use crate::prelude::*;

use serde::{Deserialize, Serialize};

use super::{Check, MAX_VOLUME};
use crate::error::{ContentError, line_of};

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

/// A ducking rule (ADR-0099): while a sound of group `when` plays, group
/// `lower`'s gain goes down to `loudness(to)` of itself over `attack_ms`,
/// and comes back over `release_ms` once it stops. Written in the
/// manifest's `ducks` list as `(when: "voice", lower: "music", to: 80,
/// attack_ms: 100, release_ms: 500)`. Build it with `default()` and assign
/// its fields (ADR-0039).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[non_exhaustive]
pub struct DuckRule {
    /// The group whose playing lowers another: only [`VOICE`] for now, the
    /// one a device says is playing.
    pub when: String,
    /// The group lowered: any group but `when` and its parents.
    pub lower: String,
    /// How far down it goes, as a position on the loudness curve (0–100):
    /// its gain is multiplied by that position's. 80 lowers it by 12 dB,
    /// and 100 changes nothing.
    pub to: u8,
    /// How long it takes to go down, in milliseconds (0: at once).
    pub attack_ms: u32,
    /// How long it takes to come back, in milliseconds (0: at once).
    pub release_ms: u32,
}

/// One entry of the manifest's `ducks` list.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RawDuck {
    when: String,
    lower: String,
    to: u8,
    attack_ms: u32,
    release_ms: u32,
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
    /// ancestor among the declared groups. A loop above it that it isn't
    /// in is its parents' to report.
    fn loop_from(&self, id: &str) -> Option<String> {
        let mut chain = vec![id];
        let mut at = id;
        while let Some(parent) = self.manifest.groups.get(at)?.parent.as_deref() {
            let seen = chain.contains(&parent);
            chain.push(parent);
            if parent == id {
                return Some(chain.join(", "));
            }
            if seen {
                return None;
            }
            at = parent;
        }
        None
    }

    /// Checks and keeps the manifest's `ducks`, after its `groups`: each
    /// rule's `when` is [`VOICE`], its `lower` a group there is that isn't
    /// `when` or one of its parents, its `to` at most 100, and no two
    /// rules lower the same group.
    pub(super) fn ducks(&mut self, ducks: Vec<RawDuck>) {
        for (at, raw) in ducks.into_iter().enumerate() {
            let RawDuck {
                when,
                lower,
                to,
                attack_ms,
                release_ms,
            } = raw;
            let rule = format!("ducks[{at}] (lower: \"{lower}\")");
            if when != VOICE {
                let why = format!(
                    "lowers \"{lower}\" when \"{when}\" plays: only \"{VOICE}\" can, the one \
                     group a device says is playing"
                );
                self.duck_err(&lower, &rule, &why);
            }
            if !self.is_group(&lower) {
                self.duck_err(&lower, &rule, &format!("names unknown group \"{lower}\""));
            } else if when == VOICE && Self::is_under(&when, &lower) {
                let why = format!(
                    "lowers \"{lower}\", which \"{when}\" plays under, so would lower it too"
                );
                self.duck_err(&lower, &rule, &why);
            }
            if to > MAX_VOLUME {
                let why = format!("lowers to {to}, over {MAX_VOLUME} on the loudness curve");
                self.duck_err(&lower, &rule, &why);
            }
            if self.manifest.ducks.iter().any(|d| d.lower == lower) {
                self.duck_err(&lower, &rule, "lowers a group another rule lowers already");
            }
            self.manifest.ducks.push(DuckRule {
                when,
                lower,
                to,
                attack_ms,
                release_ms,
            });
        }
    }

    /// Records a problem with ducking rule `rule`, at the first line with
    /// its `lower`: a rule has no id to find.
    fn duck_err(&mut self, lower: &str, rule: &str, why: &str) {
        let e = ContentError::new(self.file, format!("{rule} {why}"));
        let line = line_of(self.source, &format!("lower: \"{lower}\""));
        self.errors.push(match line {
            Some(l) => e.at(l, None),
            None => e,
        });
    }

    /// Whether built-in group `group` is `ancestor` or under it. A
    /// rule's `when` is always built in ([`VOICE`]), and a built-in group's
    /// parents are built in too.
    fn is_under(group: &str, ancestor: &str) -> bool {
        let mut at = Some(group);
        while let Some(group) = at {
            if group == ancestor {
                return true;
            }
            at = BUILT_IN
                .iter()
                .find(|&&(id, _)| id == group)
                .and_then(|&(_, parent)| parent);
        }
        false
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
