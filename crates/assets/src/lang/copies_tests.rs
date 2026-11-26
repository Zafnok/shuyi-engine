//! Tests of a language's own copies of the game's files (`files/<path>`
//! in a pack's folder): where [`localized_path`] finds one along a chain,
//! through a bundle's layers, and from a baked pack.

use super::*;
use crate::bundle::{Bundle, Layer};
use crate::format::Formats;
use crate::format::builtin_tests::{ENGLISH_UI, PACK_INFO, PACK_UI, bundle, lang_pack, png_file};
use crate::pack::{Bake, Pack};

fn code(code: &str) -> LangCode {
    LangCode::new(code).unwrap()
}

fn chain(tags: &[&str]) -> Vec<LangCode> {
    tags.iter().map(|tag| code(tag)).collect()
}

/// Where `x.png` is read along `tags` in `bundle`.
fn localized(bundle: &Bundle, tags: &[&str]) -> String {
    localized_path("lang", &chain(tags), "x.png", |p| bundle.has(p))
}

#[test]
fn a_copy_is_found_along_the_chain_else_the_plain_path() {
    let pt = ("lang/pt/files/x.png", b"pt".to_vec());
    let pt_br = ("lang/pt-BR/files/x.png", b"pt-BR".to_vec());
    let plain = ("x.png", b"plain".to_vec());
    let pt_br_chain = ["pt-BR", "pt", "en"];
    // `pt-BR` has no copy of its own: `pt`'s.
    let only_pt = bundle(vec![plain.clone(), pt.clone()]);
    assert_eq!(localized(only_pt, &pt_br_chain), "lang/pt/files/x.png");
    // Its own copy comes first.
    let both = bundle(vec![plain.clone(), pt, pt_br.clone()]);
    assert_eq!(localized(both, &pt_br_chain), "lang/pt-BR/files/x.png");
    assert_eq!(localized(both, &["pt", "en"]), "lang/pt/files/x.png");
    // No copy along it: the plain path, the source's too.
    assert_eq!(localized(both, &["ja", "en"]), "x.png");
    assert_eq!(localized(both, &["en"]), "x.png");
    // With `lang/pt/` left out of the bundle, `pt` has no copy.
    let no_pt = bundle(vec![plain.clone(), pt_br]);
    assert_eq!(localized(no_pt, &["pt", "en"]), "x.png");
    // Languages at the bundle's root.
    let root = bundle(vec![plain, ("ja/files/x.png", b"ja".to_vec())]);
    let found = localized_path("", &chain(&["ja", "en"]), "x.png", |p| root.has(p));
    assert_eq!(found, "ja/files/x.png");
}

#[test]
fn a_targets_layer_holds_its_own_copy_of_a_languages_file() {
    static BASE: &[(&str, &[u8])] = &[
        ("lang/ja/files/title/end.png", b"ja colour"),
        ("lang/pt/files/title/end.png", b"pt colour"),
        ("title/end.png", b"colour"),
    ];
    static PLAYDATE: &[(&str, &[u8])] = &[
        ("lang/ja/files/title/end.png", b"ja one-bit"),
        ("title/end.png", b"one-bit"),
    ];
    let game = || vec![Layer::table("assets", BASE)];
    let targets = [Layer::table("assets@playdate", PLAYDATE)];
    let bundle = Bundle::for_target(game(), &targets, Some("playdate"));
    let read = |tags: &[&str]| {
        let path = localized_path("lang", &chain(tags), "title/end.png", |p| bundle.has(p));
        bundle.file(&path).map(std::borrow::Cow::into_owned)
    };
    // The target's copy of the language's file.
    assert_eq!(read(&["ja", "en"]).as_deref(), Some("ja one-bit"));
    // A language the target has no copy of keeps its words, in colour.
    assert_eq!(read(&["pt", "en"]).as_deref(), Some("pt colour"));
    assert_eq!(read(&["en"]).as_deref(), Some("one-bit"));
    // On another target, the game's own.
    let desk = Bundle::for_target(game(), &targets, None);
    let path = localized_path("lang", &chain(&["ja", "en"]), "title/end.png", |p| {
        desk.has(p)
    });
    assert_eq!(desk.file(&path).as_deref(), Some("ja colour"));
}

#[test]
fn a_copy_read_back_from_a_baked_pack_equals_the_loose_one() {
    let picture = png_file([1, 2, 3, 255, 4, 5, 6, 255]);
    let loose = bundle(vec![
        ("lang/en/ui.ron", ENGLISH_UI.into()),
        ("lang/xx/lang.ron", PACK_INFO.into()),
        ("lang/xx/ui.ron", PACK_UI.into()),
        ("lang/xx/files/title/end.png", picture.clone()),
        ("lang/xx/files/title/end.png.meta", b"()".to_vec()),
        ("title/end.png", png_file([0; 8])),
    ]);
    let mut formats = Formats::standard();
    formats.register(lang_pack()).unwrap();
    let bake = Bake::new("test 1").asset::<LangPack>("lang/xx/lang");
    let baked = bake
        .bake(&formats, loose, None)
        .unwrap_or_else(|e| panic!("{e:?}"));
    let pack = Pack::open(&baked.bytes, "test 1").unwrap_or_else(|e| panic!("{e}"));
    let packed = Bundle::new(vec![pack.into_owned().into_layer("pack")]);
    let tags = chain(&["xx", "en"]);
    let path = |bundle: &Bundle| localized_path("lang", &tags, "title/end.png", |p| bundle.has(p));
    assert_eq!(path(&packed), "lang/xx/files/title/end.png");
    assert_eq!(path(&packed), path(loose));
    assert_eq!(packed.bytes(&path(&packed)).as_deref(), Some(&*picture));
    assert_eq!(packed.bytes(&path(&packed)), loose.bytes(&path(loose)));
}
