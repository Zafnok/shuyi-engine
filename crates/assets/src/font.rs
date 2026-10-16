//! A bitmap font atlas: a grid of fixed-size glyph cells in one image, and
//! a glyph map (a RON file) from `char` to cell index. The game names the
//! two files and the glyphs it needs.
use crate::prelude::*;

use alloc::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::error::ContentError;
use crate::image::png_size;
use crate::ron::parse_ron;

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
            None => errors.push(ContentError::new(
                png_file,
                "file not found in asset bundle",
            )),
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
        assert!(def.image_problems(0, 0).is_empty());
        let mut def = required_only();
        def.cell_h = 0;
        assert_eq!(def.problems(REQUIRED), ["cell_h must be greater than 0"]);
        assert!(def.image_problems(0, 0).is_empty());
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
