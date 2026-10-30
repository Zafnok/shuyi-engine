//! The import settings tests: the names and the chain, the splitter and
//! where its errors point, which file wins, layers, and never a panic.
use proptest::prelude::*;
use serde::Deserialize;

use super::*;
use crate::Formats;
use crate::bundle::Layer;
use crate::format::builtin_tests::{bundle, edited};
use crate::format::{AssetPath, DecodeCx, TEXT_MAX_BYTES};

/// How a picture is dithered.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
enum Dither {
    #[default]
    Atkinson,
    Bayer,
    None,
}

/// The tests' `onebit` section.
#[derive(Debug, Default, Clone, PartialEq, Eq, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct OneBit {
    dither: Dither,
    cut: u8,
}

impl ImportSettings for OneBit {
    const SECTION: &'static str = "onebit";
}

/// The tests' `indexed` section.
#[derive(Debug, Default, Clone, PartialEq, Eq, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct Indexed {
    group: String,
}

impl ImportSettings for Indexed {
    const SECTION: &'static str = "indexed";
}

#[test]
fn a_files_settings_keep_its_full_name() {
    assert_eq!(settings_of("images/hero.png"), "images/hero.png.meta");
    assert_eq!(settings_of("hero.ogg"), "hero.ogg.meta");
}

#[test]
fn the_chain_goes_from_the_files_own_up_to_the_root() {
    assert_eq!(
        settings_chain("hero.png"),
        ["hero.png.meta", "_folder.meta"]
    );
    assert_eq!(
        settings_chain("images/ui/hero.png"),
        [
            "images/ui/hero.png.meta",
            "images/ui/_folder.meta",
            "images/_folder.meta",
            "_folder.meta"
        ]
    );
}

#[test]
fn a_settings_file_is_named_meta_in_any_case() {
    for path in ["images/hero.png.meta", "_folder.meta", "a/HERO.PNG.META"] {
        assert!(is_settings_file(path), "{path}");
    }
    for path in [
        "images/hero.png",
        "meta",
        ".meta",
        "images/.meta",
        "metadata.ron",
    ] {
        assert!(!is_settings_file(path), "{path}");
    }
}

/// The sections of `source`, as a file `s.meta`.
fn split(source: &str) -> Result<Vec<Section<'_>>, Vec<ContentError>> {
    sections("s.meta", source.as_bytes(), TEXT_MAX_BYTES)
}

/// The names of `source`'s sections, or its errors' messages.
fn names(source: &str) -> Result<Vec<String>, Vec<String>> {
    split(source)
        .map(|sections| sections.into_iter().map(|s| s.name).collect())
        .map_err(|errors| errors.into_iter().map(|e| e.message).collect())
}

/// The one error of reading `source`'s section `at` as `S`, as `(line,
/// column)`.
fn error_at<S: ImportSettings + fmt::Debug>(source: &str, at: usize) -> Option<(u32, u32)> {
    let sections = split(source).ok()?;
    let error = read_section::<S>("s.meta", sections.get(at)?).err()?;
    assert_eq!(error.file, "s.meta");
    assert_eq!(error.code, Some(codes::PARSE));
    Some((error.line?, error.column?))
}

#[test]
fn unit_has_no_section() {
    assert_eq!(names("()"), Ok(Vec::new()));
    assert_eq!(names(" () "), Ok(Vec::new()));
}

#[test]
fn each_section_is_split_with_its_text() {
    let source = "(onebit: (dither: bayer), indexed: (group: \"ui\"))";
    assert_eq!(names(source), Ok(vec!["onebit".into(), "indexed".into()]));
    let sections = split(source).unwrap_or_default();
    let onebit = sections
        .first()
        .map(|s| read_section::<OneBit>("s.meta", s));
    let indexed = sections
        .get(1)
        .map(|s| read_section::<Indexed>("s.meta", s));
    assert_eq!(
        onebit,
        Some(Ok(OneBit {
            dither: Dither::Bayer,
            cut: 0
        }))
    );
    assert_eq!(indexed, Some(Ok(Indexed { group: "ui".into() })));
}

#[test]
fn a_comment_before_a_section_is_skipped() {
    let source = "// the hero's settings\n(\n    // made one-bit\n    onebit: (cut: 9),\n)";
    let sections = split(source).unwrap_or_default();
    assert_eq!(sections.len(), 1);
    assert_eq!(
        sections.first().map(|s| (s.name.as_str(), s.at)),
        Some(("onebit", Place { line: 4, column: 5 }))
    );
}

#[test]
fn a_string_may_hold_a_closing_bracket_and_a_comma() {
    let source = r#"(indexed: (group: "a), b"), onebit: ())"#;
    assert_eq!(names(source), Ok(vec!["indexed".into(), "onebit".into()]));
    let sections = split(source).unwrap_or_default();
    let indexed = sections
        .first()
        .map(|s| read_section::<Indexed>("s.meta", s));
    assert_eq!(
        indexed,
        Some(Ok(Indexed {
            group: "a), b".into()
        }))
    );
}

#[test]
fn an_error_in_a_section_points_at_its_line_in_the_file() {
    // An unknown field in the second section, on the file's line 3.
    let source = "(\n    onebit: (),\n    indexed: (team: \"ui\"),\n)";
    assert_eq!(error_at::<Indexed>(source, 1), Some((3, 19)));
    // A bad value on the section's own first line: the file's column.
    let source = "(indexed: (), onebit: (cut: 300))";
    assert_eq!(error_at::<OneBit>(source, 1), Some((1, 32)));
    // Columns count characters, not bytes.
    let source = "(indexed: (group: \"éé\"), onebit: (cut: 300))";
    assert_eq!(error_at::<OneBit>(source, 1), Some((1, 43)));
    // A bad value on a later line of the section.
    let source = "(onebit: (\n    dither: bayer,\n    cut: 300,\n))";
    assert_eq!(error_at::<OneBit>(source, 0), Some((3, 13)));
    // A value of the wrong kind.
    let source = "(onebit: (dither: blue))";
    assert_eq!(error_at::<OneBit>(source, 0), Some((1, 23)));
}

#[test]
fn a_file_that_doesnt_read_is_an_error_at_its_place() {
    let errors = split("(\n    onebit: (dither: bayer),\n    indexed: (\n").err();
    let error = errors.as_deref().and_then(<[_]>::first);
    assert_eq!(
        error.map(|e| (e.line, e.code)),
        Some((Some(4), Some(codes::PARSE)))
    );
    for source in ["[1, 2]", "7", "\"onebit\"", ""] {
        assert!(names(source).is_err(), "{source}");
    }
    // Not a struct: the error says what a settings file holds.
    let list = names("[1, 2]").err().unwrap_or_default();
    assert!(
        list.iter().any(|m| m.contains("a struct of sections")),
        "{list:?}"
    );
}

#[test]
fn a_section_whose_name_isnt_right_before_its_colon_is_placed_at_its_value() {
    // A comment between the name and the colon: the value's place.
    let source = "(onebit /* x */ :  (cut: 1))";
    let sections = split(source).unwrap_or_default();
    assert_eq!(
        sections.first().map(|s| (s.name.as_str(), s.at)),
        Some((
            "onebit",
            Place {
                line: 1,
                column: 20
            }
        ))
    );
}

#[test]
fn a_section_given_twice_is_an_error() {
    let errors = split("(\n    onebit: (),\n    onebit: (cut: 1),\n)").err();
    let error = errors.as_deref().and_then(<[_]>::first);
    assert_eq!(
        error.map(|e| (e.line, e.column, e.code)),
        Some((Some(3), Some(5), Some(codes::DUPLICATE)))
    );
    assert!(error.is_some_and(|e| e.message.contains("\"onebit\" is given twice")));
}

#[test]
fn a_file_too_big_or_not_text_is_refused() {
    // Up to the limit reads.
    assert_eq!(sections("s.meta", b"()", 2), Ok(Vec::new()));
    let big = sections("s.meta", b"(onebit: ())", 4).err();
    assert_eq!(
        big.as_deref().and_then(<[_]>::first).and_then(|e| e.code),
        Some(codes::TOO_BIG)
    );
    let binary = sections("s.meta", b"(\xff)", TEXT_MAX_BYTES).err();
    assert_eq!(
        binary
            .as_deref()
            .and_then(<[_]>::first)
            .and_then(|e| e.code),
        Some(codes::NOT_TEXT)
    );
}

/// A base layer of settings files: the root's, `images/`'s, a broken one
/// in `images/bad/`, and two files' own.
static BASE: &[(&str, &[u8])] = &[
    ("_folder.meta", b"(onebit: (dither: none))"),
    ("images/_folder.meta", b"(onebit: (dither: bayer, cut: 9))"),
    ("images/bad/_folder.meta", b"(onebit: ("),
    ("images/hero.png.meta", b"// its own\n(onebit: (cut: 7))"),
    ("images/ui/a.png.meta", b"(onebit: (cut: 5))"),
];

/// A target's layer over [`BASE`]: `images/`'s defaults, and the hero's
/// own.
static TOP: &[(&str, &[u8])] = &[
    ("images/_folder.meta", b"(onebit: (dither: none, cut: 1))"),
    ("images/hero.png.meta", b"(onebit: (cut: 2))"),
];

/// The `S` of the file at `file` of the bundle of `layers`.
fn settings<S: ImportSettings>(layers: Vec<Layer>, file: &str) -> Result<S, Vec<ContentError>> {
    let bundle = Bundle::new(layers);
    DecodeCx::in_bundle(&bundle).import_settings(&AssetPath::new(file))
}

/// The `onebit` section of `dither` and `cut`.
fn one_bit(dither: Dither, cut: u8) -> OneBit {
    OneBit { dither, cut }
}

#[test]
fn the_nearest_file_with_the_section_wins() {
    let base = || vec![Layer::table("base", BASE)];
    // Its own.
    assert_eq!(
        settings(base(), "images/ui/a.png"),
        Ok(one_bit(Dither::Atkinson, 5))
    );
    // Its folder's.
    assert_eq!(
        settings(base(), "images/b.png"),
        Ok(one_bit(Dither::Bayer, 9))
    );
    // The folder above's, when its own folder has none.
    assert_eq!(
        settings(base(), "images/ui/c.png"),
        Ok(one_bit(Dither::Bayer, 9))
    );
    // The root's.
    assert_eq!(
        settings(base(), "sounds/a.ogg"),
        Ok(one_bit(Dither::None, 0))
    );
    // None has the section: its default.
    assert_eq!(
        settings::<Indexed>(base(), "images/b.png"),
        Ok(Indexed::default())
    );
    // No bundle: its default.
    let alone = DecodeCx::new().import_settings::<OneBit>(&AssetPath::new("images/b.png"));
    assert_eq!(alone, Ok(OneBit::default()));
}

#[test]
fn a_section_is_taken_whole() {
    // The hero's own section leaves `dither` out: its default, not the
    // folder's `bayer`.
    let base = vec![Layer::table("base", BASE)];
    assert_eq!(
        settings(base, "images/hero.png"),
        Ok(one_bit(Dither::Atkinson, 7))
    );
}

#[test]
fn a_broken_file_on_the_way_is_an_error() {
    let errors = settings::<OneBit>(vec![Layer::table("base", BASE)], "images/bad/x.png").err();
    let error = errors.as_deref().and_then(<[_]>::first);
    assert_eq!(
        error.map(|e| (e.file.as_str(), e.code)),
        Some(("base/images/bad/_folder.meta", Some(codes::PARSE)))
    );
}

#[test]
fn a_higher_layers_file_replaces_the_lower_ones_but_nearness_wins() {
    let layers = || vec![Layer::table("top", TOP), Layer::table("base", BASE)];
    // The top's own file replaces the base's.
    assert_eq!(
        settings(layers(), "images/hero.png"),
        Ok(one_bit(Dither::Atkinson, 2))
    );
    // The top's folder defaults replace the base's.
    assert_eq!(
        settings(layers(), "images/b.png"),
        Ok(one_bit(Dither::None, 1))
    );
    // The base's own file is nearer than the top's folder defaults.
    assert_eq!(
        settings(layers(), "images/ui/a.png"),
        Ok(one_bit(Dither::Atkinson, 5))
    );
}

/// A settings file that reads, to edit.
const GOOD: &str =
    "// a hero\n(\n    onebit: (dither: bayer, cut: 9),\n    indexed: (group: \"ui\"),\n)";

/// Splits `bytes`, reads them as a file's own settings and a folder's,
/// and checks them: none may panic.
fn never_panics(bytes: &[u8]) {
    let _ = sections("s.meta", bytes, TEXT_MAX_BYTES);
    let bundle = bundle(vec![
        ("images/_folder.meta", bytes.to_vec()),
        ("images/a.png", Vec::new()),
        ("images/a.png.meta", bytes.to_vec()),
    ]);
    let cx = DecodeCx::in_bundle(bundle);
    let _ = cx.import_settings::<OneBit>(&AssetPath::new("images/a.png"));
    let _ = cx.import_settings::<Indexed>(&AssetPath::new("images/b.png"));
    let mut formats = Formats::new();
    let _ = formats.register_import_settings::<OneBit>();
    let _ = formats.check_import_settings(bundle);
}

proptest! {
    #[test]
    fn random_bytes_never_panic(bytes in proptest::collection::vec(any::<u8>(), 0..300)) {
        never_panics(&bytes);
    }

    #[test]
    fn an_edited_settings_file_never_panics(text in edited(GOOD)) {
        never_panics(text.as_bytes());
    }
}
