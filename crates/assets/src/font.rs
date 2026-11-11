//! A bitmap font atlas: a grid of fixed-size glyph cells in one image, and
//! a glyph map (a RON file) from `char` to cell index. The game names the
//! two files and the glyphs it needs. As a format (ADR-0007), the glyph
//! map is a sidecar read with the PNG beside it ([`FontAtlasRon`]). Once
//! the atlas and the game's text both load, [`undrawn`] finds every
//! character of the text the atlas can't draw.
//!
//! An atlas may hold a second, wide font beside the narrow one: a wide
//! glyph (a Japanese or Chinese character) takes two cells of the atlas,
//! side by side, and two cells of the screen. [`CellWidths`] says which
//! characters are wide, for everything that measures text.
//!
//! **Wide sets.** Japanese and Chinese share code points but not shapes
//! (直, 骨, 角), so an atlas may hold several wide fonts, each a **set**:
//! the unnamed one ([`FontAtlasDef::wide`]) and named ones
//! ([`FontAtlasDef::wide_sets`]). A language pack names the set it draws
//! from ([`LangInfo::wide_font`](crate::lang::LangInfo::wide_font)), and a
//! wide glyph is looked up in that set first, then in the others
//! ([`FontAtlasDef::wide_cell`]), so a pack's name for itself shows in its
//! own shapes whatever language is on. The punctuation of
//! [`WIDE_PUNCTUATION`] is narrow, but wide in a language whose pack says
//! so ([`CellWidths::in_lang`]).
use crate::prelude::*;

use alloc::collections::{BTreeMap, BTreeSet};
use alloc::rc::Rc;

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

/// The punctuation narrow in English and wide in Chinese, with the same
/// code points: `“ ” ‘ ’ … —`. An atlas may have each both narrow and in a
/// wide set; a language's pack says which it is
/// ([`LangInfo::wide_punctuation`](crate::lang::LangInfo::wide_punctuation)).
pub const WIDE_PUNCTUATION: [char; 6] = [
    '\u{201C}', '\u{201D}', '\u{2018}', '\u{2019}', '\u{2026}', '\u{2014}',
];

/// Whether `c` is one of [`WIDE_PUNCTUATION`].
pub fn is_wide_punctuation(c: char) -> bool {
    WIDE_PUNCTUATION.contains(&c)
}

/// A set of wide glyphs: each character → the cell index of its left half.
pub type WideSet = BTreeMap<char, u32>;

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
    /// The unnamed wide set: every wide glyph, two cells wide, → the cell
    /// index of its left half; its right half is the next cell, in the
    /// same row. A character is narrow or wide, never both, but for
    /// [`WIDE_PUNCTUATION`]. Empty in an atlas without wide glyphs, and
    /// left out of its file.
    #[serde(default)]
    pub wide: WideSet,
    /// The named wide sets, by name (the atlas's own word, which a pack's
    /// `wide_font` names), each laid out as [`wide`](Self::wide). A
    /// character may be in several sets, each with its own shape. Empty in
    /// an atlas of one wide set or none, and left out of its file.
    #[serde(default)]
    pub wide_sets: BTreeMap<String, WideSet>,
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

    /// The pixel rectangle of narrow glyph `c`, or `None` if the atlas
    /// lacks it or has it wide ([`wide_rect`](Self::wide_rect)).
    pub fn glyph_rect(&self, c: char) -> Option<AtlasRect> {
        self.glyphs.get(&c).map(|&i| self.cell_rect(i))
    }

    /// The pixel rectangle of wide glyph `c`, both its cells, or `None` if
    /// no wide set has `c`: [`wide_rect_in`](Self::wide_rect_in) with no
    /// set named.
    pub fn wide_rect(&self, c: char) -> Option<AtlasRect> {
        self.wide_rect_in(None, c)
    }

    /// The pixel rectangle of wide glyph `c`, both its cells, looked up as
    /// [`wide_cell`](Self::wide_cell) says, or `None` if no wide set has
    /// `c`.
    pub fn wide_rect_in(&self, set: Option<&str>, c: char) -> Option<AtlasRect> {
        self.wide_cell(set, c).map(|i| {
            let rect = self.cell_rect(i);
            AtlasRect {
                w: rect.w * 2,
                ..rect
            }
        })
    }

    /// The cell index of wide glyph `c`'s left half: from the named set
    /// `set` (the language's, [`CellWidths::wide_font`]) first, then the
    /// unnamed set, then the other named sets in name order. `None` if no
    /// set has `c`. Every renderer looks wide glyphs up with it.
    pub fn wide_cell(&self, set: Option<&str>, c: char) -> Option<u32> {
        let named = set.and_then(|name| self.wide_sets.get(name));
        named
            .and_then(|glyphs| glyphs.get(&c))
            .or_else(|| self.wide.get(&c))
            .or_else(|| self.wide_sets.values().find_map(|glyphs| glyphs.get(&c)))
            .copied()
    }

    /// The wide set `name` names, the unnamed one for `None`, if the atlas
    /// has it (the unnamed one always).
    pub fn wide_set(&self, name: Option<&str>) -> Option<&WideSet> {
        match name {
            None => Some(&self.wide),
            Some(name) => self.wide_sets.get(name),
        }
    }

    /// Every wide set: the unnamed one (`None`), then the named ones in
    /// name order.
    pub fn all_wide_sets(&self) -> impl Iterator<Item = (Option<&str>, &WideSet)> + '_ {
        let named = self
            .wide_sets
            .iter()
            .map(|(name, glyphs)| (Some(name.as_str()), glyphs));
        core::iter::once((None, &self.wide)).chain(named)
    }

    /// The characters two cells wide where no language is named
    /// ([`CellWidths::of`]): every set's, but punctuation of
    /// [`WIDE_PUNCTUATION`] the atlas also has narrow.
    pub fn wide_chars(&self) -> BTreeSet<char> {
        self.all_wide_sets()
            .flat_map(|(_, glyphs)| glyphs.keys().copied())
            .filter(|&c| !(is_wide_punctuation(c) && self.glyphs.contains_key(&c)))
            .collect()
    }

    /// Whether any wide set has `c`.
    pub fn has_wide(&self, c: char) -> bool {
        self.all_wide_sets()
            .any(|(_, glyphs)| glyphs.contains_key(&c))
    }

    /// Whether the atlas draws `c`, narrow or wide in any set: the one
    /// answer to "can this character be drawn?" where no language is named
    /// (a name box's letters). [`undrawn`] checks a language's text against
    /// its own set.
    pub fn draws(&self, c: char) -> bool {
        self.glyphs.contains_key(&c) || self.has_wide(c)
    }

    /// Every character [`draws`](Self::draws) accepts, in code-point order,
    /// each once.
    pub fn drawn(&self) -> impl Iterator<Item = char> + '_ {
        let wide = self.all_wide_sets().flat_map(|(_, glyphs)| glyphs.keys());
        let all: BTreeSet<char> = self.glyphs.keys().chain(wide).copied().collect();
        all.into_iter()
    }

    /// One past the last cell index any glyph takes: how many cells of the
    /// atlas its glyphs reach.
    pub fn cell_end(&self) -> u32 {
        self.cells()
            .map(|(i, _)| i.saturating_add(1))
            .max()
            .unwrap_or(0)
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

    /// Every cell a glyph takes: `(cell index, glyph)`, a narrow glyph's
    /// one and both of a wide glyph's, in every set.
    fn cells(&self) -> impl Iterator<Item = (u32, char)> + '_ {
        let narrow = self.glyphs.iter().map(|(&c, &i)| (i, c));
        let wide = self
            .all_wide_sets()
            .flat_map(|(_, glyphs)| glyphs)
            .flat_map(|(&c, &i)| [(i, c), (i.saturating_add(1), c)]);
        narrow.chain(wide)
    }

    /// Problems with the glyph map on its own, in plain words: a size of 0,
    /// a glyph of `required` it lacks, two glyphs in one cell, a character
    /// both narrow and wide (but for [`WIDE_PUNCTUATION`]), a wide glyph
    /// split across a row's end, or a named set with an empty name.
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
            .filter(|&c| !self.draws(c))
            .map(describe)
            .collect();
        if !missing.is_empty() {
            out.push(format!("missing required glyphs: {}", missing.join(", ")));
        }
        if self.wide_sets.keys().any(|name| name.trim().is_empty()) {
            out.push("a wide set's name must not be empty".to_owned());
        }
        for (name, glyphs) in self.all_wide_sets() {
            let set = in_set(name);
            let both = glyphs
                .keys()
                .filter(|&&c| self.glyphs.contains_key(&c) && !is_wide_punctuation(c));
            for &c in both {
                out.push(format!("{} is both narrow and wide{set}", describe(c)));
            }
            if self.columns == 0 {
                continue;
            }
            for (&c, &i) in glyphs {
                if i % self.columns == self.columns - 1 {
                    out.push(format!(
                        "wide {}{set} has cell index {i}, the last of its row: its right half \
                         would start the next row",
                        describe(c)
                    ));
                }
            }
        }
        let mut owner: BTreeMap<u32, char> = BTreeMap::new();
        for (i, c) in self.cells() {
            if let Some(&first) = owner.get(&i) {
                if first != c {
                    out.push(format!(
                        "{} and {} share cell index {i}",
                        describe(first),
                        describe(c)
                    ));
                }
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
        if let Some((i, c)) = self.cells().find(|&(i, _)| u64::from(i) >= cells) {
            out.push(format!(
                "{} has cell index {i}, but the image only holds {cells} cells",
                describe(c)
            ));
        }
        out
    }
}

/// How many cells of the screen each character takes, in one language:
/// two for a character with a wide glyph in any set of the atlas
/// ([`FontAtlasDef::wide`], [`FontAtlasDef::wide_sets`]), and for
/// [`WIDE_PUNCTUATION`] in a language whose pack has it wide; one for any
/// other. [`text_width`](crate::lang::text_width) measures text with it,
/// and a glyph buffer prints with it. It also names the language's wide
/// set, which a renderer looks wide glyphs up in first
/// ([`wide_font`](Self::wide_font)). [`CellWidths::NARROW`] has nothing
/// wide. Cheap to clone: the set is shared.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CellWidths {
    /// The wide characters; `None` when there are none.
    wide: Option<Rc<BTreeSet<char>>>,
    /// Whether [`WIDE_PUNCTUATION`] is wide.
    punctuation: bool,
    /// The language's wide set, if it names one.
    set: Option<Rc<str>>,
}

impl CellWidths {
    /// Nothing is wide: one cell for every character.
    pub const NARROW: Self = Self {
        wide: None,
        punctuation: false,
        set: None,
    };

    /// The widths of `atlas`'s glyphs where no language is named: a
    /// character with a wide glyph in any set takes two cells, but one of
    /// [`WIDE_PUNCTUATION`] the atlas also has narrow takes one
    /// ([`in_lang`](Self::in_lang) makes it wide).
    pub fn of(atlas: &FontAtlasDef) -> Self {
        Self::wide(atlas.wide_chars())
    }

    /// Widths where the characters of `wide` take two cells.
    pub fn wide(wide: impl IntoIterator<Item = char>) -> Self {
        let wide: BTreeSet<char> = wide.into_iter().collect();
        Self {
            wide: (!wide.is_empty()).then(|| Rc::new(wide)),
            ..Self::NARROW
        }
    }

    /// These widths in a language that draws from the wide set `wide_font`
    /// (`None`: no set of its own) and has [`WIDE_PUNCTUATION`] wide or
    /// not: what its pack's `lang.ron` says
    /// ([`LangInfo::widths`](crate::lang::LangInfo::widths)).
    #[must_use]
    pub fn in_lang(self, wide_font: Option<&str>, wide_punctuation: bool) -> Self {
        Self {
            punctuation: wide_punctuation,
            set: wide_font.map(Rc::from),
            ..self
        }
    }

    /// The wide set the language draws from first, if it names one
    /// ([`FontAtlasDef::wide_cell`]).
    pub fn wide_font(&self) -> Option<&str> {
        self.set.as_deref()
    }

    /// Whether [`WIDE_PUNCTUATION`] takes two cells in this language.
    pub fn wide_punctuation(&self) -> bool {
        self.punctuation
    }

    /// Whether `c` takes two cells.
    pub fn is_wide(&self, c: char) -> bool {
        self.wide.as_ref().is_some_and(|wide| wide.contains(&c))
            || (self.punctuation && is_wide_punctuation(c))
    }

    /// How many cells `c` takes: 1 or 2.
    pub fn of_char(&self, c: char) -> usize {
        if self.is_wide(c) { 2 } else { 1 }
    }
}

/// ` in wide set "ja"` for a named set, nothing for the unnamed one: what a
/// message about a wide glyph adds to say which set.
fn in_set(name: Option<&str>) -> String {
    name.map(|name| format!(" in wide set {name:?}"))
        .unwrap_or_default()
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
    /// The wide set the language draws `c` from, when it is wide there and
    /// the language names one.
    set: Option<&'a str>,
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
        } else if let Some(set) = self.set {
            format!(
                "{c} isn't in the font atlas's wide set {set:?}, which the language draws from \
                 ({keys}); remake the atlas with font-atlas --wide {set}=<font.bdf> from a font \
                 that has it"
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

/// The wide set a language draws from, as [`undrawn`] checks it.
#[derive(Clone, Copy)]
enum SetOf<'a> {
    /// It names none: any set.
    Any,
    /// It names this one, which the atlas has.
    Named(&'a str, &'a WideSet),
    /// It names one the atlas lacks (said once): its wide characters
    /// aren't checked.
    Missing,
}

/// How one language's text is checked against the atlas: its width table,
/// and the wide set it draws from.
struct LangFont<'a> {
    widths: CellWidths,
    set: SetOf<'a>,
}

impl<'a> LangFont<'a> {
    /// Whether the language draws `c`: from its own wide set if it names
    /// one and `c` is wide there, any set if wide and it names none, else
    /// narrow.
    fn draws(&self, atlas: &FontAtlasDef, c: char) -> bool {
        if !self.widths.is_wide(c) {
            return atlas.glyphs.contains_key(&c);
        }
        match self.set {
            SetOf::Any => atlas.has_wide(c),
            SetOf::Named(_, glyphs) => glyphs.contains_key(&c),
            SetOf::Missing => true,
        }
    }

    /// The set named in the error for `c`: the language's, when `c` is wide.
    fn set_for(&self, c: char) -> Option<&'a str> {
        match self.set {
            SetOf::Named(name, _) if self.widths.is_wide(c) => Some(name),
            _ => None,
        }
    }
}

/// Every character of `lang`'s text ([`Lang::texts`]) that `atlas` can't
/// draw in its language: a check across assets, after both load. A
/// language's wide characters (as its widths have them, [`Lang::widths`]:
/// a pack's or the source's) must be in the wide set it names, or in any
/// set if it names none; the rest narrow. A pack that names a set the atlas
/// lacks is one error, on its `lang.ron` (the source, on its folder), and
/// its wide characters aren't checked. One error per language and
/// character, in the order of their first use, on the file of that use
/// ([`TextAt::path`]), naming its key and how many more keys of that
/// language use it. Control characters (a `\n`) are never errors; a
/// combining mark ([`is_combining_mark`]) always is, glyph or not. The
/// errors have no line: a [`Lang`] keeps keys, which find the line.
pub fn undrawn(atlas: &FontAtlasDef, lang: &Lang) -> Vec<ContentError> {
    let base = CellWidths::of(atlas);
    let mut errors = Vec::new();
    let mut fonts: BTreeMap<Option<&LangCode>, LangFont<'_>> = BTreeMap::new();
    let mut found: Vec<Undrawn<'_>> = Vec::new();
    let mut index: BTreeMap<(Option<&LangCode>, char), usize> = BTreeMap::new();
    for (at, text) in lang.texts() {
        let font = fonts.entry(at.lang).or_insert_with(|| {
            let source = &lang.source_lang().code;
            let widths = lang.widths(at.lang.unwrap_or(source), &base);
            let set = match widths.wide_font() {
                None => SetOf::Any,
                Some(name) => {
                    if let Some((name, glyphs)) = atlas.wide_sets.get_key_value(name) {
                        SetOf::Named(name, glyphs)
                    } else {
                        // A pack names it in its lang.ron; the source, in
                        // the game's code (`SourceLang::with_wide_font`).
                        let (path, whose) = match at.lang {
                            Some(code) => (format!("{code}/{}", crate::lang::INFO_FILE), ""),
                            None => (source.to_string(), "the source language's "),
                        };
                        errors.push(ContentError::new(
                            path,
                            format!(
                                "{whose}wide_font names {name:?}, a wide set the font atlas \
                                 doesn't have; remake the atlas with font-atlas --wide \
                                 {name}=<font.bdf>"
                            ),
                        ));
                        SetOf::Missing
                    }
                }
            };
            LangFont { widths, set }
        });
        for c in text.chars() {
            if c.is_control() || (font.draws(atlas, c) && !is_combining_mark(c)) {
                continue;
            }
            let set = font.set_for(c);
            let i = *index.entry((at.lang, c)).or_insert_with(|| {
                found.push(Undrawn {
                    c,
                    first: at,
                    keys: BTreeSet::new(),
                    set,
                });
                found.len() - 1
            });
            found[i].keys.insert((at.part, at.key));
        }
    }
    errors.extend(found.iter().map(Undrawn::error));
    errors
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
            ..FontAtlasDef::default()
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
            wide: [('あ', 33)].into(),
            wide_sets: BTreeMap::new(),
        };
        let r = |x, y| Some(AtlasRect { x, y, w: 8, h: 16 });
        assert_eq!(def.glyph_rect('a'), r(0, 0));
        assert_eq!(def.glyph_rect('b'), r(248, 0));
        assert_eq!(def.glyph_rect('c'), r(0, 16));
        assert_eq!(def.glyph_rect('d'), r(48, 32));
        assert_eq!(def.glyph_rect('z'), None);
        // A wide glyph is both its cells, and no narrow glyph.
        let wide = Some(AtlasRect {
            x: 8,
            y: 16,
            w: 16,
            h: 16,
        });
        assert_eq!(def.wide_rect('あ'), wide);
        assert_eq!(def.glyph_rect('あ'), None);
        assert_eq!(def.wide_rect('a'), None);
    }

    /// [`required_only`] with wide glyphs `あ` and `い` after it, in cells
    /// 16 and 17, and 18 and 19.
    fn with_wide() -> FontAtlasDef {
        FontAtlasDef {
            wide: [('あ', 16), ('い', 18)].into(),
            ..required_only()
        }
    }

    #[test]
    fn a_wide_glyph_is_drawn_and_listed_among_the_narrow_ones() {
        let def = with_wide();
        assert_eq!(def.problems(REQUIRED), Vec::<String>::new());
        assert_eq!(def.image_problems(128, 32), Vec::<String>::new());
        assert!(def.draws('あ') && def.draws('A') && !def.draws('う'));
        let drawn: Vec<char> = def.drawn().collect();
        let mut sorted = drawn.clone();
        sorted.sort_unstable();
        assert_eq!(drawn, sorted, "in code-point order");
        assert_eq!(drawn.len(), REQUIRED.chars().count() + 2);
        // A required glyph may be wide.
        assert_eq!(def.problems("あ"), Vec::<String>::new());
        // Narrow and wide in turn are listed in turn, either first.
        let mixed = FontAtlasDef {
            glyphs: [('b', 0), ('d', 1)].into(),
            wide: [('a', 2), ('c', 4), ('e', 6)].into(),
            ..FontAtlasDef::default()
        };
        assert_eq!(mixed.drawn().collect::<String>(), "abcde");
    }

    #[test]
    fn wide_glyph_problems() {
        // Both narrow and wide.
        let mut def = with_wide();
        def.glyphs.insert('あ', 30);
        assert_eq!(
            def.problems(REQUIRED),
            ["'あ' (U+3042) is both narrow and wide"]
        );
        // Its right half in a narrow glyph's cell.
        let mut def = with_wide();
        def.wide.insert('う', 12);
        assert_eq!(
            def.problems(REQUIRED),
            ["'─' (U+2500) and 'う' (U+3046) share cell index 12"]
        );
        // Split across a row's end: cell 31 is the last of the second row,
        // and the last the image holds, so its right half is past it too.
        let mut def = with_wide();
        def.wide.insert('う', 31);
        assert_eq!(
            def.problems(REQUIRED),
            [
                "wide 'う' (U+3046) has cell index 31, the last of its row: its right half \
                 would start the next row"
            ]
        );
        assert_eq!(
            def.image_problems(128, 32),
            ["'う' (U+3046) has cell index 32, but the image only holds 32 cells"]
        );
    }

    /// [`required_only`] with wide sets `ja` (直 and あ, in cells 16 and
    /// 18) and `zh-Hans` (直 and 简, in cells 20 and 22), and `“` and `…`
    /// in `zh-Hans` too (24 and 26) beside their narrow glyphs (28, 29).
    fn with_sets() -> FontAtlasDef {
        let mut def = required_only();
        def.glyphs.extend([('“', 28), ('…', 29)]);
        def.wide_sets = [
            ("ja".to_owned(), [('直', 16), ('あ', 18)].into()),
            (
                "zh-Hans".to_owned(),
                [('直', 20), ('简', 22), ('“', 24), ('…', 26)].into(),
            ),
        ]
        .into();
        def
    }

    #[test]
    fn a_wide_glyph_is_looked_up_in_the_languages_set_first_then_any_other() {
        let def = with_sets();
        assert_eq!(def.problems(REQUIRED), Vec::<String>::new());
        assert_eq!(def.image_problems(128, 32), Vec::<String>::new());
        // 直 is in both sets, each with its own shape.
        assert_eq!(def.wide_cell(Some("ja"), '直'), Some(16));
        assert_eq!(def.wide_cell(Some("zh-Hans"), '直'), Some(20));
        // A character the language's set lacks comes from another set.
        assert_eq!(def.wide_cell(Some("ja"), '简'), Some(22));
        assert_eq!(def.wide_cell(Some("zh-Hans"), 'あ'), Some(18));
        // No set named, or one the atlas lacks: the unnamed set, then the
        // named ones in name order.
        assert_eq!(def.wide_cell(None, '直'), Some(16));
        assert_eq!(def.wide_cell(Some("ko"), '直'), Some(16));
        let mut unnamed = with_sets();
        unnamed.wide.insert('直', 30);
        assert_eq!(unnamed.wide_cell(None, '直'), Some(30));
        assert_eq!(unnamed.wide_cell(Some("zh-Hans"), '直'), Some(20));
        assert_eq!(def.wide_cell(Some("ja"), 'A'), None);
        let rect = def.wide_rect_in(Some("zh-Hans"), '直');
        assert_eq!(rect.map(|r| (r.x, r.y, r.w)), Some((32, 16, 16)));
        assert_eq!(def.wide_rect('直'), def.wide_rect_in(Some("ja"), '直'));
        // Every set counts as drawn, and is listed once.
        assert!(def.draws('简') && def.draws('あ') && !def.draws('骨'));
        assert_eq!(def.drawn().filter(|&c| c == '直').count(), 1);
        assert_eq!(def.wide_set(Some("ja")).map(BTreeMap::len), Some(2));
        assert_eq!(def.wide_set(None), Some(&BTreeMap::new()));
        assert_eq!(def.wide_set(Some("ko")), None);
        assert_eq!(def.cell_end(), 30);
    }

    #[test]
    fn wide_set_problems() {
        // Six punctuation marks may be narrow and wide; no other character.
        let mut def = with_sets();
        def.glyphs.insert('简', 30);
        assert_eq!(
            def.problems(REQUIRED),
            ["'简' (U+7B80) is both narrow and wide in wide set \"zh-Hans\""]
        );
        // A set's glyph past the image, or across a row's end.
        let mut def = with_sets();
        def.wide_sets.get_mut("ja").unwrap().insert('骨', 31);
        assert_eq!(
            def.problems(REQUIRED),
            [
                "wide '骨' (U+9AA8) in wide set \"ja\" has cell index 31, the last of its row: \
                 its right half would start the next row"
            ]
        );
        assert_eq!(
            def.image_problems(128, 32),
            ["'骨' (U+9AA8) has cell index 32, but the image only holds 32 cells"]
        );
        // Two sets' glyphs in one cell, and an empty name.
        let mut def = with_sets();
        def.wide_sets.get_mut("ja").unwrap().insert('骨', 20);
        def.wide_sets.insert(" ".to_owned(), BTreeMap::new());
        assert_eq!(
            def.problems(REQUIRED),
            [
                "a wide set's name must not be empty",
                "'骨' (U+9AA8) and '直' (U+76F4) share cell index 20",
                "'骨' (U+9AA8) and '直' (U+76F4) share cell index 21",
            ]
        );
        // An atlas file without sets reads as none.
        let src = "(cell_w: 8, cell_h: 16, columns: 16, glyphs: {}, wide: {'あ': 0})";
        let def: FontAtlasDef = ron::from_str(src).unwrap();
        assert!(def.wide_sets.is_empty());
        let src =
            "(cell_w: 8, cell_h: 16, columns: 16, glyphs: {}, wide_sets: {\"ja\": {'あ': 0}})";
        let def: FontAtlasDef = ron::from_str(src).unwrap();
        assert_eq!(def.wide_cell(Some("ja"), 'あ'), Some(0));
    }

    #[test]
    fn widths_by_language_make_six_punctuation_marks_wide() {
        let base = CellWidths::of(&with_sets());
        // Every set's characters are wide; punctuation the atlas also has
        // narrow is narrow where no language says otherwise.
        assert!(base.is_wide('直') && base.is_wide('简') && base.is_wide('あ'));
        assert!(!base.is_wide('“') && !base.is_wide('…'));
        assert_eq!(base.wide_font(), None);
        let chinese = base.clone().in_lang(Some("zh-Hans"), true);
        assert_eq!(chinese.wide_font(), Some("zh-Hans"));
        assert!(chinese.wide_punctuation());
        for c in WIDE_PUNCTUATION {
            assert!(chinese.is_wide(c), "{c}");
            assert!(!base.is_wide(c), "{c}");
        }
        let width = |text: &str, widths: &CellWidths| crate::lang::text_width(text, widths);
        assert_eq!(width("“直”", &chinese), 6);
        assert_eq!(width("……", &chinese), 4);
        assert_eq!(width("“ok”", &base), 4);
        assert_eq!(width("“ok”", &base.clone().in_lang(Some("ja"), false)), 4);
        // Punctuation only wide in the atlas (2301's) stays wide.
        let mut only_wide = with_sets();
        only_wide.glyphs.remove(&'…');
        assert!(CellWidths::of(&only_wide).is_wide('…'));
        assert!(!CellWidths::NARROW.wide_punctuation());
    }

    #[test]
    fn widths_count_a_wide_glyph_twice() {
        let widths = CellWidths::of(&with_wide());
        assert!(widths.is_wide('あ') && !widths.is_wide('A'));
        assert_eq!((widths.of_char('い'), widths.of_char('A')), (2, 1));
        assert_eq!(CellWidths::of(&required_only()), CellWidths::NARROW);
        assert_eq!(CellWidths::wide([]), CellWidths::default());
        assert!(!CellWidths::NARROW.is_wide('あ'));
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
        // No columns, and a wide glyph: said, not divided by.
        let mut def = with_wide();
        def.columns = 0;
        assert_eq!(def.problems(REQUIRED), ["columns must be greater than 0"]);
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
            ..FontAtlasDef::default()
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
            widths: &crate::font::CellWidths::NARROW,
            variants: &[],
        };

        /// An atlas of ASCII's printable characters, and `extra`.
        fn ascii_and(extra: &str) -> FontAtlasDef {
            FontAtlasDef {
                cell_w: 8,
                cell_h: 16,
                columns: 16,
                glyphs: (' '..='~').chain(extra.chars()).zip(0..).collect(),
                ..FontAtlasDef::default()
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
        fn a_wide_glyph_counts_as_drawn() {
            let lang = lang(
                &[("menu.title", "Start")],
                &[("ja", &[("menu.title", "あA")])],
            );
            let found = errors(&ascii_and(""), &lang);
            assert_eq!(found.len(), 1, "{found:?}");
            assert!(found[0].starts_with("ja/ui.ron: 'あ' (U+3042) isn't in the font atlas"));
            let mut atlas = ascii_and("");
            atlas.wide.insert('あ', 96);
            assert_eq!(errors(&atlas, &lang), Vec::<String>::new());
        }

        /// English's `menu.title`, and a pack per `(code, lang.ron's extra
        /// fields, its menu.title)`.
        fn lang_with_info(packs: &[(&str, &str, &str)]) -> Lang {
            let english = owned(&[("menu.title", "Start")]);
            let packs = packs
                .iter()
                .map(|&(code, extra, text)| {
                    let ui = format!(r#"[(key: "menu.title", source: "Start", text: {text:?})]"#);
                    let info = format!(r#"(name: "Test", made_by: Machine{extra})"#);
                    let pack =
                        pack_from_sources(("lang.ron", &info), ("ui.ron", &ui), &english, &RULES);
                    (LangCode::new(code).unwrap(), pack.unwrap())
                })
                .collect();
            Lang::new(english, packs)
        }

        /// An ASCII atlas with wide sets `ja` (直, あ) and `zh-Hans` (直, 简,
        /// and `“ ”` beside their narrow glyphs).
        fn two_sets() -> FontAtlasDef {
            let mut atlas = ascii_and("“”");
            atlas.wide_sets = [
                ("ja".to_owned(), [('直', 100), ('あ', 102)].into()),
                (
                    "zh-Hans".to_owned(),
                    [('直', 104), ('简', 106), ('“', 108), ('”', 110)].into(),
                ),
            ]
            .into();
            atlas
        }

        #[test]
        fn a_packs_wide_characters_must_be_in_the_set_it_names() {
            let ja = r#", wide_font: Some("ja")"#;
            let zh = r#", wide_font: Some("zh-Hans"), wide_punctuation: true"#;
            // Each pack's characters are in its own set: nothing to say.
            let fine = lang_with_info(&[("ja", ja, "直あ"), ("zh-Hans", zh, "“直简”")]);
            assert_eq!(errors(&two_sets(), &fine), Vec::<String>::new());
            // 简 is in the atlas, but not in the set Japanese draws from.
            let lacks = lang_with_info(&[("ja", ja, "直简")]);
            assert_eq!(
                errors(&two_sets(), &lacks),
                [
                    "ja/ui.ron: '简' (U+7B80) isn't in the font atlas's wide set \"ja\", which \
                     the language draws from (menu.title); remake the atlas with font-atlas \
                     --wide ja=<font.bdf> from a font that has it"
                ]
            );
            // A narrow character it lacks is the atlas's, not its set's.
            let narrow_missing = lang_with_info(&[("ja", ja, "直Ж")]);
            assert_eq!(
                errors(&two_sets(), &narrow_missing),
                [
                    "ja/ui.ron: 'Ж' (U+0416) isn't in the font atlas (menu.title); remake the \
                     atlas with font-atlas from a font that has it"
                ]
            );
            // Its wide punctuation must be in its set too; narrow, `“` is.
            let mut no_quotes = two_sets();
            no_quotes.wide_sets.get_mut("zh-Hans").unwrap().remove(&'“');
            let found = errors(&no_quotes, &fine);
            assert_eq!(found.len(), 1, "{found:?}");
            assert!(
                found[0]
                    .starts_with("zh-Hans/ui.ron: '“' (U+201C) isn't in the font atlas's wide set")
            );
            let narrow = lang_with_info(&[("ja", ja, "“直”")]);
            assert_eq!(errors(&no_quotes, &narrow), Vec::<String>::new());
            // A pack naming no set may draw from any.
            let any = lang_with_info(&[("ko", "", "直简あ")]);
            assert_eq!(errors(&two_sets(), &any), Vec::<String>::new());
            // A set the atlas lacks is said once, on the pack's lang.ron.
            let ko = r#", wide_font: Some("ko")"#;
            let missing = lang_with_info(&[("ko", ko, "直简Ж")]);
            let found = errors(&two_sets(), &missing);
            assert_eq!(
                found[0],
                "ko/lang.ron: wide_font names \"ko\", a wide set the font atlas doesn't have; \
                 remake the atlas with font-atlas --wide ko=<font.bdf>"
            );
            assert_eq!(found.len(), 2, "{found:?}");
            assert!(found[1].starts_with("ko/ui.ron: 'Ж' (U+0416) isn't in the font atlas ("));
        }

        #[test]
        fn the_sources_wide_characters_must_be_in_the_set_it_names() {
            use crate::lang::SourceLang;
            let source = |set: &str| {
                let code = LangCode::new("zh-Hans").unwrap();
                SourceLang::new(code, "中文")
                    .with_wide_font(set)
                    .with_wide_punctuation(true)
            };
            let written = |text: &str, set: &str| {
                lang(&[("menu.title", text)], &[]).with_source_lang(source(set))
            };
            assert_eq!(
                errors(&two_sets(), &written("“直简”", "zh-Hans")),
                Vec::<String>::new()
            );
            // 简 is wide in the atlas, but not in the set it names.
            assert_eq!(
                errors(&two_sets(), &written("直简", "ja")),
                [
                    "zh-Hans: '简' (U+7B80) isn't in the font atlas's wide set \"ja\", which the \
                     language draws from (menu.title); remake the atlas with font-atlas --wide \
                     ja=<font.bdf> from a font that has it"
                ]
            );
            // Its punctuation is wide, so it must be in its set too.
            let mut no_quotes = two_sets();
            no_quotes.wide_sets.get_mut("zh-Hans").unwrap().remove(&'“');
            let found = errors(&no_quotes, &written("“直”", "zh-Hans"));
            assert_eq!(found.len(), 1, "{found:?}");
            assert!(
                found[0].starts_with("zh-Hans: '“' (U+201C) isn't in the font atlas's wide set")
            );
            // A set the atlas lacks is said once, on the source's folder.
            assert_eq!(
                errors(&two_sets(), &written("直", "ko")),
                [
                    "zh-Hans: the source language's wide_font names \"ko\", a wide set the font \
                     atlas doesn't have; remake the atlas with font-atlas --wide ko=<font.bdf>"
                ]
            );
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
