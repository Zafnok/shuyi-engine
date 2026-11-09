//! Sound groups (ADR-0091): every sound, track and voice clip plays in a
//! group, and every group under [`MASTER`], so the player sets a volume
//! for each and one over all of them. The five built-in groups are
//! [`MASTER`]; [`MUSIC`], [`SFX`] and [`VOICE`] under it; and [`UI`] under
//! [`SFX`], so the sound volume lowers the menus' sounds too. A game adds
//! its own, and sets the built-ins' defaults, in its audio manifest's
//! `groups` (`shuyi_assets::audio`), and a cue names its group there.
//!
//! [`SoundGroups`] is the tree and which cue plays where; [`GroupGains`]
//! each group's gain from the player's volumes, on the loudness curve
//! ([`loudness`]) and times its parent's; and [`Gains`] the two together,
//! which the platform hands its sound device every frame. A device gets
//! one gain per cue and never the tree.

use crate::prelude::*;
use alloc::collections::BTreeMap;

use shuyi_assets::audio::AudioManifest;
use shuyi_assets::audio::groups::{BUILT_IN, is_built_in};
pub use shuyi_assets::audio::groups::{MASTER, MUSIC, SFX, UI, VOICE};

use super::AudioConfig;
use super::loudness::{Gain, loudness};
use crate::settings::Volumes;

/// The volume `music`, `sfx` and `voice` start at: the position whose gain
/// is nearest the 0.8 they played at before the curve. A game's own group
/// starts there too unless its manifest says otherwise. *Tunable.*
const BUILT_IN_DEFAULT: u8 = 97;

/// The volume a new player hears built-in group `group` at: 100 for
/// [`MASTER`] and [`UI`] (the sound volume already lowers the menus').
fn built_in_default(group: &str) -> u8 {
    if group == MASTER || group == UI {
        100
    } else {
        BUILT_IN_DEFAULT
    }
}

/// Where each built-in group is in `BUILT_IN`, and in every
/// [`SoundGroups`]: they come first.
const MUSIC_AT: usize = 1;
const SFX_AT: usize = 2;
const VOICE_AT: usize = 3;
const UI_AT: usize = 4;

/// One group: its id, its parent's index (`None` for [`MASTER`]) and its
/// default volume.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Group {
    id: String,
    parent: Option<usize>,
    default: u8,
}

/// The sound groups, and the group each cue plays in: the cue's own
/// `group` in the manifest; else a sound cue in [`UI`] if it is one of the
/// menus' ([`AudioConfig`]) and in [`SFX`] if not, and a music cue in
/// [`MUSIC`]. A voice clip plays in [`VOICE`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SoundGroups {
    /// Every group, parents before their children: the built-ins first.
    groups: Vec<Group>,
    /// The sound cues that play in a group other than [`SFX`], by cue.
    sounds: BTreeMap<String, usize>,
    /// The music cues that play in a group other than [`MUSIC`], by cue.
    music: BTreeMap<String, usize>,
}

impl SoundGroups {
    /// The built-in groups, then the groups `manifest` declares (parents
    /// first), each at the default the manifest gives it, else at its own;
    /// the menus' cues of `config` in [`UI`], and each cue `manifest` puts
    /// in a group in that group. A declared group whose parent isn't there
    /// (a manifest made in code, never checked) is left out, and its cues
    /// play where they would without it.
    pub fn new(config: &AudioConfig, manifest: &AudioManifest) -> Self {
        let set = |id: &str| manifest.groups.get(id).and_then(|g| g.volume);
        let at = |id: &str| BUILT_IN.iter().position(|&(g, _)| g == id);
        let mut groups: Vec<Group> = BUILT_IN
            .iter()
            .map(|&(id, parent)| Group {
                id: id.to_owned(),
                parent: parent.and_then(at),
                default: set(id).unwrap_or_else(|| built_in_default(id)),
            })
            .collect();
        let mut waiting: Vec<_> = (manifest.groups.iter())
            .filter(|(id, _)| !is_built_in(id))
            .collect();
        while !waiting.is_empty() {
            let before = waiting.len();
            waiting.retain(|&(id, def)| {
                let parent = def.parent.as_deref().unwrap_or(MASTER);
                let Some(parent) = groups.iter().position(|g| g.id == parent) else {
                    return true;
                };
                groups.push(Group {
                    id: id.clone(),
                    parent: Some(parent),
                    default: def.volume.unwrap_or(BUILT_IN_DEFAULT),
                });
                false
            });
            if waiting.len() == before {
                break;
            }
        }
        let menus = [
            config.move_cue,
            config.select_cue,
            config.cancel_cue,
            config.denied_cue,
            config.cursor_cue,
        ];
        let mut sounds: BTreeMap<_, _> = menus.iter().map(|&cue| (cue.to_owned(), UI_AT)).collect();
        let index = |cue: &String, group: &Option<String>| {
            let group = group.as_deref()?;
            Some((cue.clone(), groups.iter().position(|g| g.id == group)?))
        };
        let own = manifest
            .sounds
            .iter()
            .filter_map(|(c, s)| index(c, &s.group));
        sounds.extend(own);
        let music = (manifest.music.iter())
            .filter_map(|(c, t)| index(c, &t.group))
            .collect();
        Self {
            groups,
            sounds,
            music,
        }
    }

    /// Where `group` is, if there is one.
    fn index(&self, group: &str) -> Option<usize> {
        self.groups.iter().position(|g| g.id == group)
    }

    /// Every group's id, parents before their children.
    pub fn ids(&self) -> impl Iterator<Item = &str> {
        self.groups.iter().map(|g| g.id.as_str())
    }

    /// The volume a new player hears `group` at (0–100), or `None` if
    /// there is no such group.
    pub fn default_volume(&self, group: &str) -> Option<u8> {
        Some(self.groups[self.index(group)?].default)
    }

    /// The group `group` is under: `None` for [`MASTER`] and for a group
    /// there isn't.
    pub fn parent(&self, group: &str) -> Option<&str> {
        let parent = self.groups[self.index(group)?].parent?;
        Some(&self.groups[parent].id)
    }

    /// Where sound cue `cue` plays.
    fn sound_at(&self, cue: &str) -> usize {
        self.sounds.get(cue).copied().unwrap_or(SFX_AT)
    }

    /// The group sound cue `cue` plays in.
    pub fn sound_group(&self, cue: &str) -> &str {
        &self.groups[self.sound_at(cue)].id
    }

    /// Where music cue `cue` plays.
    fn music_at(&self, cue: &str) -> usize {
        self.music.get(cue).copied().unwrap_or(MUSIC_AT)
    }

    /// The group music cue `cue` plays in.
    pub fn music_group(&self, cue: &str) -> &str {
        &self.groups[self.music_at(cue)].id
    }
}

impl Default for SoundGroups {
    /// The built-in groups of a game whose menus play
    /// [`AudioConfig::DEFAULT`]'s cues and whose manifest declares none.
    fn default() -> Self {
        Self::new(&AudioConfig::DEFAULT, &AudioManifest::default())
    }
}

/// Each group's gain, in the order of its [`SoundGroups`]: its volume on
/// the loudness curve, times its parent's gain, down from [`MASTER`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GroupGains(Vec<Gain>);

impl GroupGains {
    /// The gains of `groups` at the player's `volumes` (a group not in
    /// them at its default).
    pub fn of(groups: &SoundGroups, volumes: &Volumes) -> Self {
        let mut gains = Self::default();
        gains.update(groups, volumes);
        gains
    }

    /// Works the gains out again, in place: `Game` does every frame.
    pub fn update(&mut self, groups: &SoundGroups, volumes: &Volumes) {
        self.0.clear();
        for group in &groups.groups {
            let own = loudness(volumes.get(&group.id).unwrap_or(group.default));
            let gain = match group.parent {
                Some(parent) => own.times(self.0[parent]),
                None => own,
            };
            self.0.push(gain);
        }
    }

    /// Every group of `groups` at [`Gain::FULL`]: what a device plays at
    /// before it is told otherwise.
    pub fn full(groups: &SoundGroups) -> Self {
        Self(vec![Gain::FULL; groups.groups.len()])
    }

    /// The same gains with `group`'s at `gain`, as it is: its children's
    /// stay. For a device's tests, which want gains they can name. A group
    /// `groups` lacks changes nothing.
    #[must_use]
    pub fn with(mut self, groups: &SoundGroups, group: &str, gain: Gain) -> Self {
        if let Some(at) = groups.index(group)
            && let Some(slot) = self.0.get_mut(at)
        {
            *slot = gain;
        }
        self
    }
}

/// One frame's gains (ADR-0091): what every sound, track and voice clip
/// plays at, as the platform hands it to the sound device. A device asks
/// it for a cue's gain and multiplies the cue's own volume by it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Gains<'a> {
    groups: &'a SoundGroups,
    table: &'a GroupGains,
    muted: bool,
}

impl<'a> Gains<'a> {
    /// The gains of `groups`, as `gains` says.
    pub const fn new(groups: &'a SoundGroups, gains: &'a GroupGains) -> Self {
        Self {
            groups,
            table: gains,
            muted: false,
        }
    }

    /// The same view with every gain at [`Gain::SILENT`]: a game muted
    /// while its window has no focus.
    #[must_use]
    pub const fn muted(self) -> Self {
        Self {
            muted: true,
            ..self
        }
    }

    /// Whether it is [`muted`](Self::muted).
    pub const fn is_muted(&self) -> bool {
        self.muted
    }

    /// The groups it is over.
    pub const fn sound_groups(&self) -> &'a SoundGroups {
        self.groups
    }

    /// The groups' gains, as worked out: not [`muted`](Self::muted).
    pub const fn group_gains(&self) -> &'a GroupGains {
        self.table
    }

    /// The gain of the group at `at`.
    fn at(&self, at: usize) -> Gain {
        if self.muted {
            Gain::SILENT
        } else {
            self.table.0.get(at).copied().unwrap_or(Gain::SILENT)
        }
    }

    /// The gain of `group`: silence for a group there isn't.
    pub fn group(&self, group: &str) -> Gain {
        self.groups
            .index(group)
            .map_or(Gain::SILENT, |at| self.at(at))
    }

    /// The gain sound cue `cue` plays at: its group's.
    pub fn sound(&self, cue: &str) -> Gain {
        self.at(self.groups.sound_at(cue))
    }

    /// The gain music cue `cue` plays at: its group's.
    pub fn music(&self, cue: &str) -> Gain {
        self.at(self.groups.music_at(cue))
    }

    /// The gain every voice clip plays at: [`VOICE`]'s.
    pub fn voice(&self) -> Gain {
        self.at(VOICE_AT)
    }

    /// Every group's id and gain, parents first: for a device that records
    /// them.
    pub fn each(&self) -> impl Iterator<Item = (&'a str, Gain)> + 'a {
        let view = *self;
        let groups: &'a [Group] = &view.groups.groups;
        groups
            .iter()
            .enumerate()
            .map(move |(at, group)| (group.id.as_str(), view.at(at)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::MAX_VOLUME;

    fn volumes(set: &[(&str, u8)]) -> Volumes {
        let mut volumes = Volumes::default();
        for &(group, volume) in set {
            volumes.set(group, volume);
        }
        volumes
    }

    #[test]
    fn the_built_in_groups_are_a_tree_under_master_at_their_defaults() {
        let groups = SoundGroups::default();
        let ids: Vec<_> = groups.ids().collect();
        assert_eq!(ids, [MASTER, MUSIC, SFX, VOICE, UI]);
        let parents: Vec<_> = ids.iter().map(|g| groups.parent(g)).collect();
        assert_eq!(
            parents,
            [None, Some(MASTER), Some(MASTER), Some(MASTER), Some(SFX)]
        );
        let defaults: Vec<_> = ids.iter().map(|g| groups.default_volume(g)).collect();
        assert_eq!(defaults, [100, 97, 97, 97, 100].map(Some));
        assert_eq!(groups.default_volume("ambience"), None);
        assert_eq!(groups.parent("ambience"), None);
    }

    #[test]
    fn the_menus_sounds_play_in_ui_other_sounds_in_sfx_music_in_music() {
        let config = AudioConfig {
            cursor_cue: "tick",
            ..AudioConfig::DEFAULT
        };
        let groups = SoundGroups::new(&config, &AudioManifest::default());
        for cue in ["ui_move", "ui_select", "ui_cancel", "tick"] {
            assert_eq!(groups.sound_group(cue), UI, "{cue}");
        }
        assert_eq!(groups.sound_group("ui_cursor"), SFX);
        assert_eq!(groups.sound_group("footstep"), SFX);
        assert_eq!(groups.music_group("theme"), MUSIC);
        let volumes = volumes(&[(MUSIC, 50), (SFX, 60), (VOICE, 70), (UI, 80)]);
        let gains = GroupGains::of(&groups, &volumes);
        let view = Gains::new(&groups, &gains);
        assert_eq!(view.sound("tick"), view.group(UI));
        assert_eq!(view.sound("footstep"), view.group(SFX));
        assert_eq!(view.music("theme"), view.group(MUSIC));
        assert_eq!(view.voice(), view.group(VOICE));
        let all: Vec<_> = [MUSIC, SFX, VOICE, UI].map(|g| view.group(g)).into();
        assert!(all.windows(2).all(|w| w[0] != w[1]), "{all:?}");
    }

    /// A manifest declaring `ambience` under `sfx` at 80, `weather` under
    /// `master` and `rain` under `weather` (before it in the map's order), and setting `music`'s
    /// default to 90; `footfall` plays in `ambience`, the menus' `ui_move`
    /// in `sfx`, and the track `storm` in `weather`.
    fn manifest() -> AudioManifest {
        use shuyi_assets::audio::{CreditRef, GroupDef, MusicCue, SoundCue};
        let mut m = AudioManifest::default();
        let group = |parent: Option<&str>, volume| {
            let mut g = GroupDef::default();
            g.parent = parent.map(str::to_owned);
            g.volume = volume;
            g
        };
        m.groups.insert("rain".into(), group(Some("weather"), None));
        m.groups
            .insert("ambience".into(), group(Some(SFX), Some(80)));
        m.groups.insert("weather".into(), group(None, None));
        m.groups.insert(MUSIC.into(), group(None, Some(90)));
        m.groups
            .insert("orphan".into(), group(Some("nowhere"), None));
        let sound = |group: &str| SoundCue::new(Vec::new(), 100, CreditRef::Own).with_group(group);
        m.sounds.insert("footfall".into(), sound("ambience"));
        m.sounds.insert("ui_move".into(), sound(SFX));
        m.sounds.insert("lost".into(), sound("orphan"));
        let track = |group: Option<&str>| MusicCue {
            file: "a.ogg".into(),
            volume: 100,
            looped: true,
            length_ms: 1000,
            credit: CreditRef::Own,
            group: group.map(str::to_owned),
        };
        m.music.insert("storm".into(), track(Some("weather")));
        m.music.insert("theme".into(), track(None));
        m
    }

    #[test]
    fn the_manifests_groups_come_after_the_built_ins_parents_first() {
        let groups = SoundGroups::new(&AudioConfig::DEFAULT, &manifest());
        let ids: Vec<_> = groups.ids().collect();
        assert_eq!(
            ids,
            [MASTER, MUSIC, SFX, VOICE, UI, "ambience", "weather", "rain"]
        );
        assert_eq!(groups.parent("ambience"), Some(SFX));
        assert_eq!(groups.parent("rain"), Some("weather"));
        assert_eq!(groups.parent("weather"), Some(MASTER));
        // A declared default, a game's own group's default, and a
        // built-in's default the manifest sets.
        assert_eq!(groups.default_volume("ambience"), Some(80));
        assert_eq!(groups.default_volume("rain"), Some(97));
        assert_eq!(groups.default_volume(MUSIC), Some(90));
        assert_eq!(groups.default_volume(SFX), Some(97));
        // A group whose parent isn't there is left out.
        assert_eq!(groups.default_volume("orphan"), None);
    }

    #[test]
    fn a_cue_plays_in_its_group_else_where_it_would() {
        let groups = SoundGroups::new(&AudioConfig::DEFAULT, &manifest());
        assert_eq!(groups.sound_group("footfall"), "ambience");
        assert_eq!(groups.sound_group("ui_move"), SFX, "its group over ui");
        assert_eq!(groups.sound_group("ui_select"), UI);
        assert_eq!(groups.sound_group("splash"), SFX);
        assert_eq!(groups.sound_group("lost"), SFX, "its group is left out");
        assert_eq!(groups.music_group("storm"), "weather");
        assert_eq!(groups.music_group("theme"), MUSIC);
        let gains = GroupGains::of(&groups, &volumes(&[("weather", 50)]));
        let view = Gains::new(&groups, &gains);
        assert_eq!(view.music("storm"), view.group("weather"));
        assert_eq!(view.music("storm"), loudness(50));
        assert_eq!(view.music("theme"), view.group(MUSIC));
        assert_eq!(view.sound("footfall"), view.group("ambience"));
        // ambience at 80 under sfx at 97: its own gain times sfx's.
        let ambience = loudness(80).times(loudness(97));
        assert_eq!(view.group("ambience"), ambience);
        assert_eq!(view.group("rain"), loudness(97).times(loudness(50)));
        assert_eq!(view.each().count(), 8);
    }

    #[test]
    fn a_groups_gain_is_its_loudness_times_its_parents_down_from_master() {
        let groups = SoundGroups::default();
        // master 97 → 3329; sfx 85 → 1453 × 3329 / 4096 = 1181; ui 50 →
        // 130 × 1181 / 4096 = 37.
        let volumes = volumes(&[(MASTER, 97), (SFX, 85), (UI, 50)]);
        let gains = GroupGains::of(&groups, &volumes);
        let view = Gains::new(&groups, &gains);
        assert_eq!(view.group(MASTER), Gain::new(3329));
        assert_eq!(view.group(SFX), Gain::new(1181));
        assert_eq!(view.group(UI), Gain::new(37));
        // music at its default, 97: 3329 × 3329 / 4096 = 2706.
        assert_eq!(view.group(MUSIC), Gain::new(2706));
        assert_eq!(view.group("ambience"), Gain::SILENT);
        let defaults = GroupGains::of(&groups, &Volumes::default());
        let view = Gains::new(&groups, &defaults);
        assert_eq!(view.group(MASTER), Gain::FULL);
        assert_eq!(view.group(SFX), Gain::new(3329));
        assert_eq!(view.group(UI), Gain::new(3329));
    }

    #[test]
    fn with_master_at_zero_every_gain_is_zero_and_muted_is_too() {
        let groups = SoundGroups::default();
        let gains = GroupGains::of(&groups, &volumes(&[(MASTER, 0), (UI, MAX_VOLUME)]));
        let view = Gains::new(&groups, &gains);
        assert!(view.each().all(|(_, gain)| gain == Gain::SILENT));
        let loud = GroupGains::full(&groups);
        assert!(!Gains::new(&groups, &loud).is_muted());
        let muted = Gains::new(&groups, &loud).muted();
        assert!(muted.is_muted());
        assert_eq!(muted.each().count(), 5);
        assert!(muted.each().all(|(_, gain)| gain == Gain::SILENT));
        let cues = [muted.sound("ui_move"), muted.sound("x"), muted.music("y")];
        assert_eq!(cues, [Gain::SILENT; 3]);
        assert_eq!(muted.voice(), Gain::SILENT);
        assert_eq!(muted.group_gains(), &loud);
    }

    #[test]
    fn a_test_names_the_gains_it_wants() {
        let groups = SoundGroups::default();
        let half = Gain::new(2048);
        let gains = GroupGains::full(&groups).with(&groups, MUSIC, half).with(
            &groups,
            "ambience",
            Gain::SILENT,
        );
        let view = Gains::new(&groups, &gains);
        let each: Vec<_> = view.each().collect();
        assert_eq!(
            each,
            [
                (MASTER, Gain::FULL),
                (MUSIC, half),
                (SFX, Gain::FULL),
                (VOICE, Gain::FULL),
                (UI, Gain::FULL)
            ]
        );
        assert_eq!(view.sound_groups(), &groups);
    }
}
