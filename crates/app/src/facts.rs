//! Which target this build is, and its facts (ADR-0060): the id is
//! `SHUYI_TARGET` when the build sets it, else `web` on the web and
//! `desktop` elsewhere. One binary serves Linux desktops and the Steam
//! Deck, so only the build can tell them apart: the Deck's exporter sets
//! `SHUYI_TARGET=steam-deck`. The facts are read from the engine's profile
//! of that id, embedded from `targets/`, and never typed twice.

use shuyi_assets::target::TargetProfile;
use shuyi_platform::Start;
use shuyi_ui::{GameData, PlatformFacts};

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
        .map(|profile| PlatformFacts::from_profile(&profile))
        .map_err(|errors| {
            let lines: Vec<String> = errors.iter().map(ToString::to_string).collect();
            lines.join("\n")
        })
}

/// The facts of the target `id` for `start`'s game. On an error the
/// game's load gives it instead, so the window shows it as it shows a
/// broken asset, and the facts are the `desktop` target's, which no game
/// sees.
pub(crate) fn start_as<G: GameData>(id: &str, mut start: Start<G>) -> (Start<G>, PlatformFacts) {
    match facts_for(id) {
        Ok(facts) => (start, facts),
        Err(why) => {
            start.load = Box::new(move |_| Err(why));
            (start, PlatformFacts::default())
        }
    }
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
        assert_eq!(facts_for("desktop"), Ok(PlatformFacts::default()));
        let web = facts_for("web").unwrap();
        assert!(!web.can_quit() && web.windowed);
        let deck = facts_for("steam-deck").unwrap();
        assert!(deck.can_quit() && !deck.windowed);
        assert_eq!(deck.keyboard, Keyboard::Absent);
    }

    #[test]
    fn an_unknown_target_is_an_error_naming_it_and_the_known_ones() {
        let error = facts_for("nowhere").unwrap_err();
        assert!(error.contains("\"nowhere\""), "{error}");
        assert!(error.contains("desktop, web, steam-deck"), "{error}");
    }

    static FONT: FontAtlasDef = FontAtlasDef {
        cell_w: 0,
        cell_h: 0,
        columns: 0,
        glyphs: BTreeMap::new(),
    };
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
