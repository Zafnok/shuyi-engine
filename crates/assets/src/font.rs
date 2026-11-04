//! A bitmap font atlas: a grid of fixed-size glyph cells in one image, and
//! a glyph map (a RON file) from `char` to cell index. The game names the
//! two files and the glyphs it needs. As a format (ADR-0007), the glyph
//! map is a sidecar read with the PNG beside it ([`FontAtlasRon`]). Once
//! the atlas and the game's text both load, [`undrawn`] finds every
//! character of the text the atlas can't draw.
use crate::prelude::*;

use alloc::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::error::ContentError;
use crate::format::{AssetKind, AssetPath, DecodeCx, Format, TEXT_MAX_BYTES, text};
use crate::image::png_size;
use crate::lang::{Lang, LangCode, TextAt, TextPart};
use crate::ron::parse_ron;

impl AssetKind for FontAtlasDef {
    const NAME: &'static str = "font atlas";
}

/// A font atlas's glyph map (`font-atlas`, `.ron`), checked as
/// [`FontAtlasDef::from_source`] says against the PNG beside it with its
/// name (`fonts/atlas.ron` and `fonts/atlas.png`) and the glyphs the game
/// requires.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FontAtlasRon {
    required: String,
}

impl FontAtlasRon {
    /// The glyph map of an atlas that must hold every glyph of `required`.
    pub fn requiring(required: impl Into<String>) -> Self {
        Self {
            required: required.into(),
        }
    }
}

impl Format for FontAtlasRon {
    type Asset = FontAtlasDef;

    fn id(&self) -> &'static str {
        "font-atlas"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["ron"]
    }

    fn max_bytes(&self) -> usize {
        TEXT_MAX_BYTES
    }

    fn decode(
        &self,
        file: &AssetPath,
        bytes: &[u8],
        cx: &DecodeCx<'_>,
    ) -> Result<FontAtlasDef, Vec<ContentError>> {
        let source = text(file, bytes)?;
        let png = file.with_extension("png");
        FontAtlasDef::from_source(
            file.shown(),
            source,
            &cx.shown(&png),
            cx.bytes(&png),
            &self.required,
        )
    }
}

/// The glyph used when a cell's glyph is not in the atlas.
pub const FALLBACK_GLYPH: char = '?';

/// The validated font atlas layout.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct FontAtlasDef {
    /// Cell width in pixels.
    pub cell_w: u32,
    /// Cell height in pixels.
    pub cell_h: u32,
    /// Cells per atlas row.
    pub columns: u32,
    /// Every glyph in the atlas → its cell index (row-major).
    pub glyphs: BTreeMap<char, u32>,
}

/// A pixel rectangle inside the atlas image.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AtlasRect {
    /// Left edge in pixels.
    pub x: u32,
    /// Top edge in pixels.
    pub y: u32,
    /// Width in pixels.
    pub w: u32,
    /// Height in pixels.
    pub h: u32,
}

impl FontAtlasDef {
    /// The pixel rectangle of cell `index`. `columns` must be non-zero (a
    /// validated atlas guarantees it).
    pub fn cell_rect(&self, index: u32) -> AtlasRect {
        AtlasRect {
            x: index % self.columns * self.cell_w,
            y: index / self.columns * self.cell_h,
            w: self.cell_w,
            h: self.cell_h,
        }
    }

    /// The pixel rectangle of glyph `c`, or `None` if the atlas lacks it.
    pub fn glyph_rect(&self, c: char) -> Option<AtlasRect> {
        self.glyphs.get(&c).map(|&i| self.cell_rect(i))
    }

    /// Whether the atlas draws `c`: the one answer to "can this character
    /// be drawn?" that the tools and checks ask ([`undrawn`]).
    pub fn draws(&self, c: char) -> bool {
        self.glyphs.contains_key(&c)
    }

    /// Every character [`draws`](Self::draws) accepts, in code-point order.
    pub fn drawn(&self) -> impl Iterator<Item = char> + '_ {
        self.glyphs.keys().copied()
    }

    /// Parses glyph map `source` (errors attributed to `file`) and validates
    /// it: it must hold every glyph of `required`, and fit the atlas image
    /// `png` (`None` if missing; errors attributed to `png_file`).
    pub fn from_source(
        file: &str,
        source: &str,
        png_file: &str,
        png: Option<&[u8]>,
        required: &str,
    ) -> Result<Self, Vec<ContentError>> {
        let def: Self = parse_ron(file, source).map_err(|e| vec![e])?;
        let mut errors: Vec<ContentError> = def
            .problems(required)
            .into_iter()
            .map(|m| ContentError::new(file, m))
            .collect();
        match png.map(png_size) {
            None => errors.push(ContentError::not_found(png_file)),
            Some(None) => errors.push(ContentError::new(png_file, "not a valid PNG file")),
            Some(Some((w, h))) => errors.extend(
                def.image_problems(w, h)
                    .into_iter()
                    .map(|m| ContentError::new(png_file, m)),
            ),
        }
        if errors.is_empty() {
            Ok(def)
        } else {
            Err(errors)
        }
    }

    /// Problems with the glyph map on its own, in plain words: a size of 0,
    /// a glyph of `required` it lacks, or two glyphs in one cell.
    pub fn problems(&self, required: &str) -> Vec<String> {
        let mut out = Vec::new();
        for (name, v) in [
            ("cell_w", self.cell_w),
            ("cell_h", self.cell_h),
            ("columns", self.columns),
        ] {
            if v == 0 {
                out.push(format!("{name} must be greater than 0"));
            }
        }
        let missing: Vec<String> = required
            .chars()
            .filter(|c| !self.glyphs.contains_key(c))
            .map(describe)
            .collect();
        if !missing.is_empty() {
            out.push(format!("missing required glyphs: {}", missing.join(", ")));
        }
        let mut owner: BTreeMap<u32, char> = BTreeMap::new();
        for (&c, &i) in &self.glyphs {
            if let Some(&first) = owner.get(&i) {
                out.push(format!(
                    "{} and {} share cell index {i}",
                    describe(first),
                    describe(c)
                ));
            } else {
                owner.insert(i, c);
            }
        }
        out
    }

    /// Problems fitting the glyph map onto a `width × height` pixel image.
    pub fn image_problems(&self, width: u32, height: u32) -> Vec<String> {
        if self.cell_w == 0 || self.cell_h == 0 || self.columns == 0 {
            return Vec::new(); // Already reported by `problems`.
        }
        let mut out = Vec::new();
        let expected_w = u64::from(self.columns) * u64::from(self.cell_w);
        if u64::from(width) != expected_w {
            out.push(format!(
                "image is {width} px wide, expected {expected_w} (columns × cell_w)"
            ));
        }
        if !height.is_multiple_of(self.cell_h) {
            out.push(format!(
                "image height {height} px is not a multiple of cell_h {}",
                self.cell_h
            ));
        }
        let cells = u64::from(self.columns) * u64::from(height / self.cell_h);
        if let Some((&c, &i)) = self.glyphs.iter().find(|&(_, &i)| u64::from(i) >= cells) {
            out.push(format!(
                "{} has cell index {i}, but the image only holds {cells} cells",
                describe(c)
            ));
        }
        out
    }
}

/// `'x' (U+0078)`, readable even for invisible or confusable glyphs.
pub fn describe(c: char) -> String {
    format!("'{}' (U+{:04X})", c.escape_debug(), u32::from(c))
}

/// Whether `c` is a combining mark (U+0300 to U+036F). A cell draws one
/// beside its letter, not over it, so text needs the accented letter as one
/// character instead (`é`, U+00E9), and an atlas never keeps one.
pub fn is_combining_mark(c: char) -> bool {
    ('\u{300}'..='\u{36f}').contains(&c)
}

/// The ASCII spelling of typographic punctuation (`’` is `'`, `…` is
/// `...`), if `c` is some.
pub fn ascii_spelling(c: char) -> Option<&'static str> {
    match c {
        '\u{2018}' | '\u{2019}' => Some("'"),
        '\u{201C}' | '\u{201D}' => Some("\""),
        '\u{2026}' => Some("..."),
        '\u{2014}' => Some("--"),
        '\u{2013}' => Some("-"),
        _ => None,
    }
}

/// A character the atlas can't draw, in one language: where it is first
/// used, and every key that uses it.
struct Undrawn<'a> {
    c: char,
    first: TextAt<'a>,
    keys: BTreeSet<(TextPart, &'a str)>,
}

impl Undrawn<'_> {
    fn error(&self) -> ContentError {
        let more = self.keys.len() - 1;
        let keys = match more {
            0 => self.first.key.to_owned(),
            1 => format!("{} and 1 more key", self.first.key),
            _ => format!("{} and {more} more keys", self.first.key),
        };
        let c = describe(self.c);
        let message = if is_combining_mark(self.c) {
            format!(
                "{c} is a combining mark, which a cell draws beside its letter, not over it \
                 ({keys}); write the accented letter as one character, as 'é' (U+00E9) is"
            )
        } else {
            let instead = ascii_spelling(self.c)
                .map(|ascii| format!(", or write {ascii} instead"))
                .unwrap_or_default();
            format!(
                "{c} isn't in the font atlas ({keys}); remake the atlas with font-atlas from a \
                 font that has it{instead}"
            )
        };
        ContentError::new(self.first.path(), message)
    }
}

/// Every character of `lang`'s text ([`Lang::texts`]) that `atlas` can't
/// draw ([`FontAtlasDef::draws`]): a check across assets, after both load.
/// One error per language and character, in the order of their first use,
/// on the file of that use ([`TextAt::path`]), naming its key and how many
/// more keys of that language use it. Control characters (a `\n`) are never
/// errors; a combining mark ([`is_combining_mark`]) always is, glyph or not.
/// The errors have no line: a [`Lang`] keeps keys, which find the line.
pub fn undrawn(atlas: &FontAtlasDef, lang: &Lang) -> Vec<ContentError> {
    let mut found: Vec<Undrawn<'_>> = Vec::new();
    let mut index: BTreeMap<(Option<&LangCode>, char), usize> = BTreeMap::new();
    for (at, text) in lang.texts() {
        for c in text.chars() {
            if c.is_control() || (atlas.draws(c) && !is_combining_mark(c)) {
                continue;
            }
            let i = *index.entry((at.lang, c)).or_insert_with(|| {
                found.push(Undrawn {
                    c,
                    first: at,
                    keys: BTreeSet::new(),
                });
                found.len() - 1
            });
            found[i].keys.insert((at.part, at.key));
        }
    }
    found.iter().map(Undrawn::error).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::image::png_header;

    /// The glyphs these tests require: a few, an invisible one among them.
    const REQUIRED: &str = " !'?ABCabc\u{ad}♣─";

    /// A valid atlas holding exactly the required glyphs, 16 per row.
    fn required_only() -> FontAtlasDef {
        FontAtlasDef {
            cell_w: 8,
            cell_h: 16,
            columns: 16,
            glyphs: REQUIRED.chars().zip(0..).collect(),
        }
    }

    fn rows_for(def: &FontAtlasDef) -> u32 {
        let n = u32::try_from(def.glyphs.len()).unwrap_or(u32::MAX);
        n.div_ceil(def.columns)
    }

    fn from_source(source: &str, png: Option<&[u8]>) -> Result<FontAtlasDef, Vec<ContentError>> {
        FontAtlasDef::from_source("a.ron", source, "base/fonts/atlas.png", png, REQUIRED)
    }

    #[test]
    fn describe_names_the_glyph_and_its_code_point() {
        assert_eq!(describe('a'), "'a' (U+0061)");
        assert_eq!(describe('\''), "'\\'' (U+0027)");
        assert_eq!(describe('\u{ad}'), "'\\u{ad}' (U+00AD)");
        assert_eq!(describe('♣'), "'♣' (U+2663)");
        assert!(REQUIRED.contains(FALLBACK_GLYPH));
    }

    #[test]
    fn cell_and_glyph_rects() {
        let def = FontAtlasDef {
            cell_w: 8,
            cell_h: 16,
            columns: 32,
            glyphs: [('a', 0), ('b', 31), ('c', 32), ('d', 70)].into(),
        };
        let r = |x, y| Some(AtlasRect { x, y, w: 8, h: 16 });
        assert_eq!(def.glyph_rect('a'), r(0, 0));
        assert_eq!(def.glyph_rect('b'), r(248, 0));
        assert_eq!(def.glyph_rect('c'), r(0, 16));
        assert_eq!(def.glyph_rect('d'), r(48, 32));
        assert_eq!(def.glyph_rect('z'), None);
    }

    #[test]
    fn valid_atlas_has_no_problems() {
        let def = required_only();
        assert_eq!(def.problems(REQUIRED), Vec::<String>::new());
        let rows = rows_for(&def);
        assert_eq!(def.image_problems(128, rows * 16), Vec::<String>::new());
    }

    #[test]
    fn zero_sizes_are_problems() {
        let mut def = required_only();
        def.cell_w = 0;
        def.columns = 0;
        assert_eq!(
            def.problems(REQUIRED),
            [
                "cell_w must be greater than 0",
                "columns must be greater than 0"
            ]
        );
        let got = def.image_problems(0, 0);
        assert!(got.is_empty(), "{got:?}");
        let mut def = required_only();
        def.cell_h = 0;
        assert_eq!(def.problems(REQUIRED), ["cell_h must be greater than 0"]);
        let got = def.image_problems(0, 0);
        assert!(got.is_empty(), "{got:?}");
    }

    #[test]
    fn missing_required_glyphs_listed_together() {
        let mut def = required_only();
        def.glyphs.remove(&'♣');
        def.glyphs.remove(&'\'');
        assert_eq!(
            def.problems(REQUIRED),
            ["missing required glyphs: '\\'' (U+0027), '♣' (U+2663)"]
        );
        // Nothing required, nothing missing.
        assert_eq!(def.problems(""), Vec::<String>::new());
    }

    #[test]
    fn shared_index_is_problem() {
        let mut def = required_only();
        def.glyphs.insert('\u{2603}', 0);
        assert_eq!(
            def.problems(REQUIRED),
            ["' ' (U+0020) and '☃' (U+2603) share cell index 0"]
        );
    }

    #[test]
    fn image_size_problems() {
        let def = required_only();
        let rows = rows_for(&def);
        assert_eq!(
            def.image_problems(120, rows * 16),
            ["image is 120 px wide, expected 128 (columns × cell_w)"]
        );
        assert_eq!(
            def.image_problems(128, rows * 16 + 3),
            [format!(
                "image height {} px is not a multiple of cell_h 16",
                rows * 16 + 3
            )]
        );
        // Two columns of three rows hold six cells: the last is index 5.
        let narrow = FontAtlasDef {
            cell_w: 8,
            cell_h: 16,
            columns: 2,
            glyphs: [('a', 5)].into(),
        };
        assert_eq!(narrow.image_problems(16, 48), Vec::<String>::new());
        assert_eq!(narrow.image_problems(16, 32).len(), 1);
        // A glyph in the cell just past what the image holds.
        let mut def = required_only();
        def.glyphs.insert('z', rows * 16);
        let problems = def.image_problems(128, rows * 16);
        assert_eq!(
            problems,
            [format!(
                "'z' (U+007A) has cell index {}, but the image only holds {} cells",
                rows * 16,
                rows * 16
            )]
        );
    }

    #[test]
    fn from_source_reports_file_and_image_errors() {
        let src = "(cell_w: 8, cell_h: 16, columns: 0, glyphs: {'a': 0})";
        let errs = from_source(src, None).err().unwrap_or_default();
        let text: Vec<String> = errs.iter().map(ToString::to_string).collect();
        assert_eq!(text.len(), 3, "{text:?}");
        assert_eq!(text[0], "a.ron: columns must be greater than 0");
        assert!(text[1].starts_with("a.ron: missing required glyphs: ' ' (U+0020)"));
        assert_eq!(
            text[2],
            "base/fonts/atlas.png: file not found in asset bundle"
        );
        let errs = from_source(src, Some(b"nope")).err().unwrap_or_default();
        assert_eq!(
            errs.last().map(ToString::to_string).as_deref(),
            Some("base/fonts/atlas.png: not a valid PNG file")
        );
    }

    #[test]
    fn draws_exactly_the_characters_drawn_lists() {
        let def = required_only();
        let drawn: Vec<char> = def.drawn().collect();
        let mut sorted = drawn.clone();
        sorted.sort_unstable();
        assert_eq!(drawn, sorted, "in code-point order");
        assert_eq!(drawn.len(), REQUIRED.chars().count());
        assert!(drawn.iter().all(|&c| def.draws(c)));
        assert!(!def.draws('Ж') && !def.draws('z'));
        assert_eq!(FontAtlasDef::default().drawn().count(), 0);
    }

    mod undrawn {
        use alloc::collections::BTreeMap;

        use super::super::*;
        use crate::lang::{PackRules, pack_from_sources};

        const RULES: PackRules<'static> = PackRules {
            data_problem: |_, _| None,
            tokens: &crate::dialogue::NoTokens,
            limits: crate::dialogue::tests::LIMITS,
            variants: &[],
        };

        /// An atlas of ASCII's printable characters, and `extra`.
        fn ascii_and(extra: &str) -> FontAtlasDef {
            FontAtlasDef {
                cell_w: 8,
                cell_h: 16,
                columns: 16,
                glyphs: (' '..='~').chain(extra.chars()).zip(0..).collect(),
            }
        }

        fn owned(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
            pairs
                .iter()
                .map(|&(k, v)| (k.to_owned(), v.to_owned()))
                .collect()
        }

        /// English screen text `english`, and a pack per `(code, entries)`
        /// translating those keys to those texts.
        fn lang(english: &[(&str, &str)], packs: &[(&str, &[(&str, &str)])]) -> Lang {
            let english = owned(english);
            let packs = packs
                .iter()
                .map(|&(code, entries)| {
                    let ui: Vec<String> = entries
                        .iter()
                        .map(|(key, text)| {
                            let source = &english[*key];
                            format!("(key: {key:?}, source: {source:?}, text: {text:?})")
                        })
                        .collect();
                    let ui = format!("[{}]", ui.join(", "));
                    let info = r#"(name: "Test", made_by: Machine)"#;
                    let pack =
                        pack_from_sources(("lang.ron", info), ("ui.ron", &ui), &english, &RULES);
                    (LangCode::new(code).unwrap(), pack.unwrap())
                })
                .collect();
            Lang::new(english, packs)
        }

        fn errors(atlas: &FontAtlasDef, lang: &Lang) -> Vec<String> {
            undrawn(atlas, lang)
                .iter()
                .map(ToString::to_string)
                .collect()
        }

        #[test]
        fn a_character_the_atlas_lacks_names_its_file_its_key_and_what_to_do() {
            let lang = lang(
                &[("menu.title", "Start")],
                &[("ru", &[("menu.title", "Жa")])],
            );
            let found = undrawn(&ascii_and(""), &lang);
            assert_eq!(found.len(), 1, "{found:?}");
            assert_eq!(found[0].file, "ru/ui.ron");
            assert_eq!(found[0].line, None);
            assert_eq!(
                found[0].message,
                "'Ж' (U+0416) isn't in the font atlas (menu.title); remake the atlas with \
                 font-atlas from a font that has it"
            );
            assert_eq!(errors(&ascii_and("Ж"), &lang), Vec::<String>::new());
        }

        #[test]
        fn one_error_a_language_and_character_counting_its_other_keys() {
            let english = [("a.one", "One"), ("a.three", "Three"), ("a.two", "Two")];
            let ru: &[(&str, &str)] = &[("a.one", "Ж"), ("a.two", "ЖЖ"), ("a.three", "Жи")];
            let uk: &[(&str, &str)] = &[("a.two", "Ж")];
            let both = lang(&english, &[("ru", ru), ("uk", uk)]);
            let lacks = "isn't in the font atlas";
            let fix = "remake the atlas with font-atlas from a font that has it";
            assert_eq!(
                errors(&ascii_and(""), &both),
                [
                    format!("ru/ui.ron: 'Ж' (U+0416) {lacks} (a.one and 2 more keys); {fix}"),
                    format!("ru/ui.ron: 'и' (U+0438) {lacks} (a.three); {fix}"),
                    format!("uk/ui.ron: 'Ж' (U+0416) {lacks} (a.two); {fix}"),
                ]
            );
            let two = lang(&english, &[("ru", &[("a.one", "Ж"), ("a.two", "Ж")])]);
            assert!(errors(&ascii_and(""), &two)[0].contains("(a.one and 1 more key)"));
        }

        #[test]
        fn the_english_text_is_checked_too() {
            let lang = lang(&[("menu.title", "Café")], &[]);
            let found = undrawn(&ascii_and(""), &lang);
            assert_eq!(found.len(), 1);
            assert_eq!(found[0].file, "en");
            assert!(
                found[0]
                    .message
                    .starts_with("'é' (U+00E9) isn't in the font atlas")
            );
        }

        #[test]
        fn punctuation_names_its_ascii_spelling_and_a_newline_is_never_an_error() {
            let lang = lang(&[("a.one", "It’s\nhere…")], &[]);
            let found = errors(&ascii_and(""), &lang);
            assert_eq!(found.len(), 2, "{found:?}");
            assert!(found[0].starts_with("en: '’' (U+2019)"), "{found:?}");
            assert!(found[0].ends_with(", or write ' instead"), "{found:?}");
            assert!(found[1].ends_with(", or write ... instead"), "{found:?}");
            for c in ['\u{2018}', '\u{201C}', '\u{201D}', '\u{2014}', '\u{2013}'] {
                assert!(ascii_spelling(c).is_some(), "{c:?}");
            }
            assert_eq!(ascii_spelling('a'), None);
        }

        #[test]
        fn a_combining_mark_is_an_error_even_with_its_glyph() {
            let lang = lang(&[("a.one", "Cafe\u{301}")], &[]);
            let found = errors(&ascii_and("\u{301}"), &lang);
            assert_eq!(
                found,
                [
                    "en: '\\u{301}' (U+0301) is a combining mark, which a cell draws beside its \
                  letter, not over it (a.one); write the accented letter as one character, as \
                  'é' (U+00E9) is"
                ]
            );
            assert!(is_combining_mark('\u{300}') && is_combining_mark('\u{36f}'));
            assert!(!is_combining_mark('\u{2ff}') && !is_combining_mark('\u{370}'));
        }

        proptest::proptest! {
            /// `undrawn` is empty exactly when every character of the text
            /// but a control character is drawn and none is a combining
            /// mark.
            #[test]
            fn undrawn_is_empty_exactly_when_the_atlas_draws_the_text(
                texts in proptest::collection::vec("\\PC{0,6}|[a\\n\u{301}é]{0,4}", 1..4),
                extra in "[\u{301}éЖ]{0,3}",
            ) {
                let english: Vec<(String, String)> = texts
                    .iter()
                    .enumerate()
                    .map(|(i, text)| (format!("k.{i}"), text.clone()))
                    .collect();
                let lang = Lang::new(english.into_iter().collect(), BTreeMap::new());
                let atlas = ascii_and(&extra);
                let clean = texts.iter().flat_map(|t| t.chars()).all(|c| {
                    c.is_control() || (atlas.draws(c) && !is_combining_mark(c))
                });
                proptest::prop_assert_eq!(undrawn(&atlas, &lang).is_empty(), clean);
            }
        }
    }

    #[test]
    fn from_source_ok_and_syntax_error() {
        let def = required_only();
        let src = ron::to_string(&def).unwrap_or_default();
        let png = png_header(128, rows_for(&def) * 16);
        assert_eq!(from_source(&src, Some(&png)), Ok(def));
        let png = png_header(64, rows_for(&required_only()) * 16);
        let errs = from_source(&src, Some(&png)).err().unwrap_or_default();
        assert_eq!(errs.len(), 1);
        assert_eq!(errs[0].file, "base/fonts/atlas.png");
        let errs = from_source("(cell_w: ", None).err().unwrap_or_default();
        assert_eq!(errs.len(), 1);
        assert!(errs[0].line.is_some());
    }
}
