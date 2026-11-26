//! The language in use ([`Ctx::lang`]): the one the player picked, else
//! the game's pick at start from the system's languages
//! ([`FirstLanguage`]), never the debug language; and the languages a row
//! offers ([`Ctx::languages_offered`]); and where a language's copy of a
//! file is ([`Ctx::localized`]).

use super::*;

use crate::settings::TextSpeed;
use crate::storage::MemoryStorage;
use crate::toy::{DEBUG_LANG, Toy, ToyCtx};

fn code(text: &str) -> LangCode {
    LangCode::new(text).unwrap_or_else(|| panic!("{text}"))
}

/// A pack named `name` whose `home.greeting` is `greeting`.
fn pack(name: &str, greeting: &str) -> shuyi_assets::lang::LangPack {
    let text = format!(
        r#"(
            info: (name: "{name}", made_by: Machine),
            ui: {{"home.greeting": (key: "home.greeting", source: "Hello, {{name}}", text: "{greeting}")}},
            data: {{}},
            lines: {{}},
        )"#
    );
    ron::from_str(&text).unwrap_or_else(|e| panic!("{e}"))
}

/// The toy with a `pt` pack beside its debug one, starting by `first`.
fn toy(first: FirstLanguage) -> Toy {
    let mut toy = Toy::default().with_lang(|english| {
        let packs = [
            (code("pt"), pack("Português", "Olá, {name}")),
            (code(DEBUG_LANG), pack("Debug", "HELLO, {name}")),
        ];
        Lang::new(english, packs.into())
    });
    toy.first_language = first;
    toy
}

/// A first start of `toy` on a system whose languages are `system`.
fn start(toy: Toy, system: &[&str]) -> ToyCtx {
    let mut c = Ctx::new(toy).unwrap_or_else(|e| panic!("{e:?}"));
    c.platform = c.platform.clone().with_languages(system);
    c.with_storage(Box::new(MemoryStorage::new()))
}

#[test]
fn with_no_pick_the_system_language_the_game_has_is_shown_else_the_source() {
    let c = start(toy(FirstLanguage::System), &["pt-BR", "en-US"]);
    assert_eq!(c.lang, code("pt"));
    assert_eq!(c.text("home.greeting"), "Olá, {name}");
    assert_eq!(c.settings().language, None, "the pick isn't saved");
    let c = start(toy(FirstLanguage::System), &["de"]);
    assert_eq!(c.lang, LangCode::default());
    let c = start(toy(FirstLanguage::System), &[]);
    assert_eq!(c.lang, LangCode::default());
}

#[test]
fn the_source_and_a_fixed_language_ignore_the_system() {
    let c = start(toy(FirstLanguage::Source), &["pt-BR"]);
    assert_eq!(c.lang, LangCode::default());
    let c = start(toy(FirstLanguage::Fixed(code("pt"))), &["de"]);
    assert_eq!(c.lang, code("pt"));
}

#[test]
fn the_debug_language_is_never_picked_from_the_system() {
    let c = start(toy(FirstLanguage::System), &[DEBUG_LANG, "pt"]);
    assert_eq!(c.lang, code("pt"));
    let c = start(toy(FirstLanguage::System), &[DEBUG_LANG]);
    assert_eq!(c.lang, LangCode::default());
}

#[test]
fn the_players_pick_wins_and_is_kept_per_profile() {
    let mut c = start(toy(FirstLanguage::System), &["pt"]);
    c.change_settings(|s| s.language = Some(code("en")))
        .unwrap_or_else(|e| panic!("{e:?}"));
    assert_eq!(c.lang, code("en"));
    let mei = c.add_profile("Mei").unwrap_or_else(|e| panic!("{e:?}"));
    c.switch_profile(mei).unwrap_or_else(|e| panic!("{e:?}"));
    // A new profile has picked none: the system's again.
    assert_eq!(c.lang, code("pt"));
    c.change_settings(|s| s.language = Some(code(DEBUG_LANG)))
        .unwrap_or_else(|e| panic!("{e:?}"));
    assert_eq!(c.lang, code(DEBUG_LANG));
    c.switch_profile(profiles::ProfileId::FIRST)
        .unwrap_or_else(|e| panic!("{e:?}"));
    assert_eq!(c.lang, code("en"));
    c.switch_profile(mei).unwrap_or_else(|e| panic!("{e:?}"));
    assert_eq!(c.lang, code(DEBUG_LANG));
}

#[test]
fn a_saved_language_with_no_pack_shows_through_its_chain() {
    let mut c = start(toy(FirstLanguage::Source), &[]);
    c.change_settings(|s| s.language = Some(code("pt-BR")))
        .unwrap_or_else(|e| panic!("{e:?}"));
    assert_eq!(c.lang, code("pt-BR"));
    assert_eq!(c.text("home.greeting"), "Olá, {name}");
    assert_eq!(c.widths(), &CellWidths::NARROW);
}

#[test]
fn a_change_to_another_setting_leaves_the_language_a_tool_set() {
    let mut c = start(toy(FirstLanguage::System), &[]);
    c.lang = code("pt");
    c.change_settings(|s| s.text_speed = TextSpeed::Fast)
        .unwrap_or_else(|e| panic!("{e:?}"));
    assert_eq!(c.lang, code("pt"));
    assert!(c.preview_settings(|s| s.fullscreen = true));
    assert_eq!(c.lang, code("pt"));
    // A change of the language sets it again: back to the system's pick
    // once the player's is taken away.
    c.change_settings(|s| s.language = Some(code("pt")))
        .unwrap_or_else(|e| panic!("{e:?}"));
    c.change_settings(|s| s.language = None)
        .unwrap_or_else(|e| panic!("{e:?}"));
    assert_eq!(c.lang, LangCode::default());
}

#[test]
fn the_systems_languages_arriving_set_the_pick_unless_the_player_made_one() {
    let mut c = start(toy(FirstLanguage::System), &[]);
    assert_eq!(c.lang, LangCode::default());
    c.set_system_languages(vec![code("pt-PT")]);
    assert_eq!(c.lang, code("pt"));
    assert_eq!(c.platform.languages, [code("pt-PT")]);
    c.change_settings(|s| s.language = Some(code("en")))
        .unwrap_or_else(|e| panic!("{e:?}"));
    c.set_system_languages(vec![code("pt")]);
    assert_eq!(c.lang, code("en"));
}

#[test]
fn the_languages_offered_are_the_source_then_the_packs_by_their_own_names() {
    let mut c = start(toy(FirstLanguage::System), &[]);
    let offered = |c: &ToyCtx| -> Vec<(String, String)> {
        let all = c.languages_offered().into_iter();
        all.map(|(code, name)| (code.as_str().to_owned(), name))
            .collect()
    };
    c.debug_tools = true;
    let all = [("en", "English"), ("dbg", "Debug"), ("pt", "Português")];
    let all = all.map(|(c, n)| (c.to_owned(), n.to_owned()));
    assert_eq!(offered(&c), all);
    c.debug_tools = false;
    assert_eq!(offered(&c), [all[0].clone(), all[2].clone()]);
}

#[test]
fn a_languages_copy_of_a_file_follows_the_language_in_use() {
    let mut c = start(toy(FirstLanguage::System), &[]);
    let copies = [
        "lang/pt/files/title/end.png",
        "lang/dbg/files/title/end.png",
    ];
    let end = |c: &ToyCtx| c.localized("lang", "title/end.png", |p| copies.contains(&p));
    assert_eq!(end(&c), "title/end.png");
    // A change of language gives another path at once.
    c.change_settings(|s| s.language = Some(code("pt")))
        .unwrap_or_else(|e| panic!("{e:?}"));
    assert_eq!(end(&c), "lang/pt/files/title/end.png");
    // A copy `pt-BR` lacks comes from `pt`, along its chain.
    c.change_settings(|s| s.language = Some(code("pt-BR")))
        .unwrap_or_else(|e| panic!("{e:?}"));
    assert_eq!(end(&c), "lang/pt/files/title/end.png");
    // The debug language's copy only with the debug tools.
    c.change_settings(|s| s.language = Some(code(DEBUG_LANG)))
        .unwrap_or_else(|e| panic!("{e:?}"));
    c.debug_tools = true;
    assert_eq!(end(&c), "lang/dbg/files/title/end.png");
    c.debug_tools = false;
    assert_eq!(end(&c), "title/end.png");
}
