//! The ids of the built-in sound groups (ADR-0091): every sound, track and
//! voice clip plays in a group, and every group under [`MASTER`]. [`MUSIC`],
//! [`SFX`] and [`VOICE`] are under [`MASTER`], and [`UI`] under [`SFX`].
//! The tree and the player's volumes on it are `shuyi-ui`'s
//! (`shuyi_ui::audio::groups`); the manifest's rules name groups by these
//! ids.

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
