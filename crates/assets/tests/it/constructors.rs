//! The data types games build in code, built from outside the crate as a
//! game builds them: through their constructors, since they are
//! `#[non_exhaustive]` (ADR-0039, tickets 2085 and 2095).

use std::collections::BTreeMap;

use shuyi_assets::audio::{SAMPLE_RATE, Sound};
use shuyi_assets::credits::{CreditGroup, CreditsFile, FileCredit};
use shuyi_assets::dialogue::{DialogueLimits, NoTokens};
use shuyi_assets::font::{CellWidths, FontAtlasDef, FontInk, InkBox};
use shuyi_assets::lang::{LangInfo, LineKind, MadeBy, PackRules, SourceLine};
use shuyi_assets::pack::PackInfo;
use shuyi_assets::target::{
    Budgets, Color, Display, FrameRate, Input, Quit, Status, TargetProfile,
};
use shuyi_core::ContentHash;

const LIMITS: DialogueLimits = DialogueLimits::new(40, 3).with_max_text(200);

/// A game's pack rules, in a `const` item as a game writes them.
const RULES: PackRules<'static> =
    PackRules::new(|_, _| None, &NoTokens, LIMITS, &CellWidths::NARROW).with_variants(&["m", "f"]);

/// A font atlas in a `const` item, as a game's tests or tools may hold one.
const ATLAS: FontAtlasDef = FontAtlasDef::new(8, 16, BTreeMap::new(), 16);

#[test]
fn a_font_atlas_is_built_in_a_const_and_its_optional_fields_assigned() {
    assert_eq!((ATLAS.cell_w, ATLAS.cell_h, ATLAS.columns), (8, 16, 16));
    assert!(ATLAS.glyphs.is_empty() && ATLAS.wide.is_empty() && ATLAS.bold.is_empty());
    assert_eq!((ATLAS.ink.as_ref(), ATLAS.bold_ink.as_ref()), (None, None));

    let mut atlas = FontAtlasDef::new(8, 16, BTreeMap::from([('A', 0), ('?', 1)]), 16);
    atlas.wide.insert('田', 2);
    let mut ink = FontInk::new(InkBox::new(1, 2, 6, 11));
    ink.columns.insert('W', [0, 6]);
    atlas.ink = Some(ink);
    assert_eq!(atlas.glyph_rect('?').map(|r| (r.x, r.w)), Some((8, 8)));
    assert_eq!(atlas.ink.map(|ink| ink.columns_of('W')), Some((0, 6)));
}

#[test]
fn a_sound_is_built_mono_or_stereo() {
    let mono = Sound::mono(SAMPLE_RATE);
    assert_eq!((mono.channels, mono.sample_rate), (1, SAMPLE_RATE));
    let stereo = Sound::stereo(SAMPLE_RATE);
    assert_eq!((stereo.channels, stereo.sample_rate), (2, SAMPLE_RATE));
}

#[test]
fn a_target_profile_is_built_and_its_optional_fields_assigned() {
    let mut display = Display::new(Color::Rgba8)
        .with_min_text_px(8)
        .with_good_text_px(12);
    display.windowed = true;
    let mut input = Input::default();
    input.keyboard = true;
    let mut budgets = Budgets::default();
    budgets.startup_ms = Some(2_000);
    let frame_rate = FrameRate {
        target: 60,
        floor: 30,
    };
    let mut profile = TargetProfile::new(
        "desk",
        Status::Planned,
        display,
        frame_rate,
        input,
        "shuyi-app",
    );
    profile.budgets = budgets;
    assert_eq!(profile.id, "desk");
    assert_eq!(profile.quit, Quit::Game);
    assert_eq!(profile.display.sizes, []);
    assert!(profile.display.windowed);
    assert!(profile.input.keyboard);
    assert_eq!(profile.budgets.startup_ms, Some(2_000));
    assert_eq!(profile.backend, "shuyi-app");
}

#[test]
fn a_target_profile_built_in_code_is_the_one_its_file_reads() {
    let source = r#"(
    id: "desk",
    status: Planned,
    display: (color: Rgba8, min_text_px: 8, good_text_px: 12),
    frame_rate: (target: 60, floor: 30),
    input: (),
    backend: "shuyi-app",
)"#;
    let read = TargetProfile::from_source("desk.ron", "desk", source).unwrap();
    let frame_rate = FrameRate {
        target: 60,
        floor: 30,
    };
    let built = TargetProfile::new(
        "desk",
        Status::Planned,
        Display::new(Color::Rgba8)
            .with_min_text_px(8)
            .with_good_text_px(12),
        frame_rate,
        Input::default(),
        "shuyi-app",
    );
    assert_eq!(built, read);
}

#[test]
fn the_pack_and_language_types_are_built_by_their_constructors() {
    let info = PackInfo::new("game 1", None, ContentHash(3));
    assert_eq!(
        (info.stamp.as_str(), info.target, info.content),
        ("game 1", None, ContentHash(3))
    );

    let mut lang = LangInfo::new("Deutsch", MadeBy::Human("Ann".to_owned()));
    lang.wide_punctuation = true;
    assert_eq!(lang.name, "Deutsch");
    assert_eq!(lang.wide_font, None);
    assert!(lang.wide_punctuation);

    let line = SourceLine::new("gate", "Hello.", LineKind::Speech, "ch01");
    assert_eq!(
        (line.scene.as_str(), line.text.as_str()),
        ("gate", "Hello.")
    );
    assert_eq!((line.kind, line.file.as_str()), (LineKind::Speech, "ch01"));

    assert_eq!(RULES.variants, ["m", "f"]);
    assert_eq!(RULES.limits, LIMITS);
    assert_eq!((RULES.data_problem)("k", "t"), None);
}

#[test]
fn a_credits_file_is_built_by_its_constructors() {
    let mut credit = FileCredit::new(
        "font",
        CreditGroup::Fonts,
        "A Font",
        "Ann",
        "https://example.org/font",
        "OFL-1.1",
    );
    credit.hidden = true;
    let file = CreditsFile::new(vec![credit]);
    let only = &file.credits[0];
    assert_eq!(
        (only.id.as_str(), only.license.as_str()),
        ("font", "OFL-1.1")
    );
    assert!(only.hidden);
    assert_eq!(only.private, [] as [String; 0]);
}
