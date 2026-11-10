//! [`PlatformFacts`]: where the game runs, as its screens see it
//! ([`Ctx::platform`](crate::Ctx::platform)). A fact is constant for a
//! run: the platform's backend says them before the game loads, read from
//! its target's profile ([`TargetProfile`]) or from the system.
//!
//! A screen asks a fact instead of a `cfg!`, so a test can run it as any
//! target: offer Quit only where [`can_quit`](PlatformFacts::can_quit).
//! The facts are the screens'. A game's rules never read them, so a
//! replay plays the same on every target.

use crate::prelude::*;

pub use shuyi_assets::target::Quit;
use shuyi_assets::target::TargetProfile;

use crate::ctx::Keyboard;
use crate::input::state::Device;

/// Where the game runs: its target's id, and what that target lets a
/// game do and the player press.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct PlatformFacts {
    /// The target's id, as its profile names it (`desktop`, `web`,
    /// `steam-deck`, `playdate`).
    pub target: String,
    /// How the player leaves the game: by its own Quit, or through the
    /// system ([`can_quit`](Self::can_quit)).
    pub quit: Quit,
    /// Whether the game is in a window the player can switch to full
    /// screen and back. Where it isn't, the Fullscreen setting does
    /// nothing, and the engine's options screen leaves its row out.
    pub windowed: bool,
    /// Whether the device has a keyboard: without one the game's
    /// first-key screen never opens, and the key bindings screen shows the
    /// controller's alone.
    pub keyboard: Keyboard,
    /// Whether a mouse or another pointer exists.
    pub pointer: bool,
    /// Whether a touch screen exists.
    pub touch: bool,
    /// The device the player holds, where the target has just the one: help
    /// text names its buttons from the first frame, before any press
    /// ([`Ctx::device`](crate::Ctx::device)). `None` where it isn't known
    /// until something is pressed.
    pub device: Option<Device>,
    /// Whether the player can drop a file on the game's window (the
    /// desktop) or its page (the web) and the game hears it
    /// ([`can_drop_files`](Self::can_drop_files)). The backend says it
    /// ([`with_file_drops`](Self::with_file_drops)): a profile doesn't, and
    /// [`from_profile`](Self::from_profile) and the default give
    /// [`FileDrops::Unheard`], as a session record made before it does.
    #[serde(default)]
    pub file_drops: FileDrops,
}

/// Whether a file dropped on the game's window reaches it
/// ([`PlatformFacts::file_drops`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum FileDrops {
    /// It does (a computer's window, a browser's page): the game's system
    /// hook hears the files.
    Heard,
    /// Nothing can be dropped (a console, a phone, the headless backend).
    #[default]
    Unheard,
}

impl PlatformFacts {
    /// The facts of the target `profile` describes.
    pub fn from_profile(profile: &TargetProfile) -> Self {
        let input = &profile.input;
        Self {
            target: profile.id.clone(),
            quit: profile.quit,
            windowed: profile.display.windowed,
            keyboard: if input.keyboard {
                Keyboard::Present
            } else {
                Keyboard::Absent
            },
            pointer: input.pointer,
            touch: input.touch,
            device: None,
            file_drops: FileDrops::Unheard,
        }
    }

    /// The same facts with files dropped on the window heard, or not
    /// ([`file_drops`](Self::file_drops)).
    #[must_use]
    pub fn with_file_drops(mut self, file_drops: FileDrops) -> Self {
        self.file_drops = file_drops;
        self
    }

    /// The same facts on a target whose player holds `device`.
    #[must_use]
    pub fn with_device(mut self, device: Device) -> Self {
        self.device = Some(device);
        self
    }

    /// Whether the game may offer its own Quit: where the player leaves
    /// through the system (a browser's tab, a console's home button), a
    /// Quit would do nothing, so a game leaves it out.
    pub fn can_quit(&self) -> bool {
        self.quit == Quit::Game
    }

    /// Whether the player can drop a file on the game's window (or its
    /// page) and the game hears it: a screen offers "drop a file here"
    /// only where it does.
    pub fn can_drop_files(&self) -> bool {
        self.file_drops == FileDrops::Heard
    }
}

impl Default for PlatformFacts {
    /// The engine's `desktop` target's facts: a window that can go full
    /// screen, the game's own Quit, a keyboard and a pointer, no touch.
    /// Files dropped are unheard until a backend says.
    fn default() -> Self {
        Self {
            target: "desktop".to_owned(),
            quit: Quit::Game,
            windowed: true,
            keyboard: Keyboard::Present,
            pointer: true,
            touch: false,
            device: None,
            file_drops: FileDrops::Unheard,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_target_says_the_device_its_player_holds() {
        let device = Device::Pad(crate::input::pad::PadKind::Playdate);
        assert_eq!(PlatformFacts::default().device, None);
        assert_eq!(
            PlatformFacts::default().with_device(device).device,
            Some(device)
        );
    }

    /// The engine's profile `id`, from `targets/`.
    fn engine(id: &str, source: &str) -> TargetProfile {
        let file = format!("targets/{id}.ron");
        TargetProfile::from_source(&file, id, source).unwrap()
    }

    fn desktop() -> TargetProfile {
        engine("desktop", include_str!("../../../targets/desktop.ron"))
    }

    #[test]
    fn the_default_facts_are_the_desktops() {
        assert_eq!(
            PlatformFacts::default(),
            PlatformFacts::from_profile(&desktop())
        );
    }

    #[test]
    fn each_engine_profile_gives_its_facts() {
        let facts = |id, source| PlatformFacts::from_profile(&engine(id, source));
        let web = facts("web", include_str!("../../../targets/web.ron"));
        assert_eq!(web.target, "web");
        assert!(!web.can_quit());
        assert!(web.windowed && web.pointer && !web.touch);
        assert_eq!(web.keyboard, Keyboard::Present);
        let deck = facts(
            "steam-deck",
            include_str!("../../../targets/steam-deck.ron"),
        );
        assert!(deck.can_quit());
        assert!(!deck.windowed && !deck.pointer && deck.touch);
        assert_eq!(deck.keyboard, Keyboard::Absent);
        let playdate = facts("playdate", include_str!("../../../targets/playdate.ron"));
        assert!(!playdate.can_quit() && !playdate.windowed);
        assert!(!playdate.pointer && !playdate.touch);
        assert_eq!(playdate.keyboard, Keyboard::Absent);
        let desktop = PlatformFacts::from_profile(&desktop());
        assert!(desktop.can_quit() && !desktop.can_drop_files());
        let heard = desktop.with_file_drops(FileDrops::Heard);
        assert!(heard.can_drop_files());
    }
}
