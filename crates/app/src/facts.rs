//! Which target this build is, and its facts (ADR-0060): the id is
//! `SHUYI_TARGET` when the build sets it, else `web` on the web and
//! `desktop` elsewhere. One binary serves Linux desktops and the Steam
//! Deck, so only the build can tell them apart: the Deck's exporter sets
//! `SHUYI_TARGET=steam-deck`. The facts are read from the engine's profile
//! of that id, embedded from `targets/`, and never typed twice.
//!
//! One fact is the backend's, not the profile's: files dropped on the
//! window reach the game on `desktop` and `web`, where miniquad reads a
//! drop ([`drops`](crate::drops)), and not on the Steam Deck, whose game
//! mode has no desktop to drag a file from.
//!
//! A debug build shows another graphics tier than its target's when
//! `SHUYI_GRAPHICS` names one ([`preview_graphics`]), so a lesser
//! target's stills can be seen on the desktop.

use shuyi_assets::target::TargetProfile;
use shuyi_platform::{Loader, Start};
use shuyi_ui::{FileDrops, GameData, GraphicsTier, PlatformFacts};

/// The engine's profiles of the targets this backend builds, by id.
const PROFILES: [(&str, &str); 3] = [
    ("desktop", include_str!("../../../targets/desktop.ron")),
    ("web", include_str!("../../../targets/web.ron")),
    (
        "steam-deck",
        include_str!("../../../targets/steam-deck.ron"),
    ),
];

/// The target's id: `set` when the build set one, else `web` on the web
/// (`wasm`) and `desktop` elsewhere.
pub(crate) fn target_id(wasm: bool, set: Option<&'static str>) -> &'static str {
    match set {
        Some(id) => id,
        None if wasm => "web",
        None => "desktop",
    }
}

/// This build's target id ([`target_id`]).
pub(crate) fn this_target() -> &'static str {
    target_id(cfg!(target_arch = "wasm32"), option_env!("SHUYI_TARGET"))
}

/// The facts of the target `id`, read from its embedded profile; an error
/// naming `id` and the ids known when this backend has no profile of it.
pub(crate) fn facts_for(id: &str) -> Result<PlatformFacts, String> {
    let Some((_, source)) = PROFILES.iter().find(|(known, _)| *known == id) else {
        let known: Vec<&str> = PROFILES.iter().map(|(known, _)| *known).collect();
        return Err(format!(
            "this build is for the target \"{id}\" (SHUYI_TARGET), which shuyi-app has no \
             profile of: it knows {}",
            known.join(", ")
        ));
    };
    let file = format!("targets/{id}.ron");
    TargetProfile::from_source(&file, id, source)
        .map(|profile| PlatformFacts::from_profile(&profile).with_file_drops(file_drops(id)))
        .map_err(|errors| {
            let lines: Vec<String> = errors.iter().map(ToString::to_string).collect();
            lines.join("\n")
        })
}

/// Whether this backend hears a file dropped on the window on the target
/// `id`.
fn file_drops(id: &str) -> FileDrops {
    if matches!(id, "desktop" | "web") {
        FileDrops::Heard
    } else {
        FileDrops::Unheard
    }
}

/// The facts of the target `id` for `start`'s game. On an error the
/// game's load gives it instead, so the window shows it as it shows a
/// broken asset, and the facts are the `desktop` target's, which no game
/// sees.
pub(crate) fn start_as<G: GameData>(id: &str, mut start: Start<G>) -> (Start<G>, PlatformFacts) {
    match facts_for(id) {
        Ok(facts) => (start, facts),
        Err(why) => {
            start.load = Loader::Once(Box::new(move |_| Err(why)));
            (start, PlatformFacts::default())
        }
    }
}

/// `facts` at the graphics tier `asked` names (the value of
/// [`GRAPHICS_VAR`](shuyi_platform::GRAPHICS_VAR), in a debug build), and
/// a warning when it names none: then the facts stay their target's.
pub(crate) fn preview_graphics(
    facts: PlatformFacts,
    asked: Option<&str>,
) -> (PlatformFacts, Option<String>) {
    let Some(asked) = asked else {
        return (facts, None);
    };
    match GraphicsTier::from_name(asked) {
        Some(tier) => (facts.with_graphics(tier), None),
        None => {
            let known: Vec<&str> = GraphicsTier::ALL.iter().map(|t| t.name()).collect();
            let warning = format!(
                "{}: no graphics tier \"{asked}\"; the tiers are {}, and the target's, {}, stays",
                shuyi_platform::GRAPHICS_VAR,
                known.join(", "),
                facts.graphics
            );
            (facts, Some(warning))
        }
    }
}

/// What [`preview_graphics`] reads: the variable, in a build with the
/// debug tools alone.
pub(crate) fn asked_graphics() -> Option<String> {
    if !shuyi_ui::DEBUG_TOOLS {
        return None;
    }
    std::env::var(shuyi_platform::GRAPHICS_VAR).ok()
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use shuyi_assets::font::FontAtlasDef;
    use shuyi_assets::image::ImageTable;
    use shuyi_platform::headless::HeadlessPlatform;
    use shuyi_platform::{App, Card, Look, Millis};
    use shuyi_ui::Keyboard;
    use shuyi_ui::toy::{self, Toy};

    use super::*;

    #[test]
    fn the_target_is_the_builds_else_the_web_or_the_desktop() {
        assert_eq!(target_id(false, None), "desktop");
        assert_eq!(target_id(true, None), "web");
        for wasm in [false, true] {
            assert_eq!(target_id(wasm, Some("steam-deck")), "steam-deck");
        }
    }

    #[test]
    fn each_embedded_profile_gives_its_facts() {
        for (id, _) in PROFILES {
            let facts = facts_for(id).unwrap_or_else(|e| panic!("{e}"));
            assert_eq!(facts.target, id);
        }
        let heard = PlatformFacts::default().with_file_drops(FileDrops::Heard);
        assert_eq!(facts_for("desktop"), Ok(heard));
        let web = facts_for("web").unwrap();
        assert!(!web.can_quit() && web.windowed && web.can_drop_files());
        let deck = facts_for("steam-deck").unwrap();
        assert!(deck.can_quit() && !deck.windowed && !deck.can_drop_files());
        assert_eq!(deck.keyboard, Keyboard::Absent);
    }

    #[test]
    fn an_unknown_target_is_an_error_naming_it_and_the_known_ones() {
        let error = facts_for("nowhere").unwrap_err();
        assert!(error.contains("\"nowhere\""), "{error}");
        assert!(error.contains("desktop, web, steam-deck"), "{error}");
    }

    #[test]
    fn a_debug_build_previews_the_graphics_tier_it_is_asked_for() {
        let desktop = facts_for("desktop").unwrap();
        assert_eq!(
            preview_graphics(desktop.clone(), None),
            (desktop.clone(), None)
        );
        let (reduced, warning) = preview_graphics(desktop.clone(), Some("reduced"));
        assert_eq!(
            reduced,
            desktop.clone().with_graphics(GraphicsTier::Reduced)
        );
        assert_eq!(warning, None);
        let (minimal, _) = preview_graphics(desktop.clone(), Some("Minimal"));
        assert_eq!(minimal.graphics, GraphicsTier::Minimal);
        let (same, warning) = preview_graphics(desktop.clone(), Some("low"));
        assert_eq!(same, desktop);
        assert_eq!(
            warning.as_deref(),
            Some(
                "SHUYI_GRAPHICS: no graphics tier \"low\"; the tiers are Minimal, Reduced, Full, and the target's, Full, stays"
            )
        );
    }

    static FONT: FontAtlasDef = FontAtlasDef::new(0, 0, BTreeMap::new(), 0);
    static IMAGES: ImageTable = ImageTable::new(BTreeMap::new());

    /// The toy's look, for a start: it draws with neither.
    fn look(_: &Toy) -> Look<'_> {
        Look::new(&FONT, &IMAGES)
    }

    fn toy_start() -> Start<Toy> {
        Start::new(Card::text("A toy"), look, |setup| {
            Ok(toy::game_on(setup.ctx(toy::ctx())))
        })
    }

    /// The game's load shows why its target is unknown, as a broken
    /// asset's error is shown.
    #[test]
    fn a_game_built_for_an_unknown_target_shows_why_it_cant_start() {
        let (start, facts) = start_as("nowhere", toy_start());
        assert_eq!(facts, PlatformFacts::default());
        let mut app = App::new(start);
        let mut p = HeadlessPlatform::new();
        let _ = app.frame(&mut p, Millis(0));
        let _ = app.frame(&mut p, Millis(16));
        assert!(app.game().is_none());
        let errors = app.errors().unwrap_or_default();
        assert!(errors.contains("\"nowhere\""), "{errors}");
        let (start, facts) = start_as("web", toy_start());
        assert_eq!(facts.target, "web");
        let mut app = App::new(start);
        let mut p = HeadlessPlatform::new().with_facts(facts);
        let _ = app.frame(&mut p, Millis(0));
        let _ = app.frame(&mut p, Millis(16));
        assert!(app.game().is_some_and(|g| !g.ctx().platform.can_quit()));
    }
}
