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

/// Where the game runs: its target's id, and what that target lets a
/// game do and the player press.
#[derive(Debug, Clone, PartialEq, Eq)]
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
        }
    }

    /// Whether the game may offer its own Quit: where the player leaves
    /// through the system (a browser's tab, a console's home button), a
    /// Quit would do nothing, so a game leaves it out.
    pub fn can_quit(&self) -> bool {
        self.quit == Quit::Game
    }
}

impl Default for PlatformFacts {
    /// The engine's `desktop` target's facts: a window that can go full
    /// screen, the game's own Quit, a keyboard and a pointer, no touch.
    fn default() -> Self {
        Self {
            target: "desktop".to_owned(),
            quit: Quit::Game,
            windowed: true,
            keyboard: Keyboard::Present,
            pointer: true,
            touch: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert!(desktop.can_quit());
    }
}
