//! `font-atlas [--no-text] <font.bdf>... <out-dir>`: turns a BDF bitmap font into a
//! font atlas (`atlas.png` + `atlas.ron`, read as
//! [`shuyi_assets::font::FontAtlasDef`]) and its one-bit copy for a one-bit
//! screen (`atlas_1bit.bin`). It keeps the glyphs the font has of
//! [`ATLAS_RANGES`], of the characters the game's text uses and of the
//! glyphs the project requires ([`AtlasChars`]): when `shuyi.ron` names a
//! game tool program, `font-atlas` runs in it and reads the game's text
//! (`--no-text`: the ranges and the required glyphs only). Further BDF
//! files add glyphs the first lacks (a game's own, e.g. controller button
//! shapes). The atlas must have the glyphs the project requires
//! ([`crate::project::FontAtlas::required_glyphs`]); a character the text
//! uses that the fonts lack is named, and `check` fails on it.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use shuyi_assets::font::{FontAtlasDef, describe, is_combining_mark};
use shuyi_assets::lang::Lang;

use crate::codec::encode_png;
use crate::project::FontAtlas;

/// Unicode ranges copied into the atlas (whatever the font has of them).
pub const ATLAS_RANGES: &[(char, char)] = &[
    (' ', '~'),               // ASCII
    ('\u{a0}', '\u{ff}'),     // Latin-1 supplement
    ('\u{2c7}', '\u{2c7}'),   // ˇ caron
    ('\u{391}', '\u{3c9}'),   // Greek (CP437 has several)
    ('\u{2010}', '\u{206f}'), // General punctuation (• † …)
    ('\u{2190}', '\u{21ff}'), // Arrows
    ('\u{2200}', '\u{22ff}'), // Mathematical operators (≈ ≤ ≥ ∞)
    ('\u{2300}', '\u{23ff}'), // Miscellaneous technical (⌂ ⌐)
    ('\u{2500}', '\u{25ff}'), // Box drawing, blocks, geometric shapes
    ('\u{2600}', '\u{26ff}'), // Miscellaneous symbols (☺ ♣ ♪)
    ('\u{2715}', '\u{2715}'), // ✕ (a controller's cross button)
];

/// Atlas cells per row.
pub const COLUMNS: u32 = 32;

/// The characters an atlas keeps, of those its font has: every one of
/// [`ATLAS_RANGES`], every character the game's text uses, and the glyphs
/// the project requires; never a combining mark or a control character.
/// [`AtlasChars::default`] is the ranges alone.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AtlasChars {
    /// The characters the text uses.
    used: BTreeSet<char>,
    /// The glyphs the project requires.
    required: BTreeSet<char>,
}

/// Whether an atlas may keep `c` at all: it isn't a control character or a
/// combining mark.
fn keepable(c: char) -> bool {
    !c.is_control() && !is_combining_mark(c)
}

impl AtlasChars {
    /// The ranges, every character of `lang`'s text ([`Lang::texts`]) and
    /// the glyphs of `required`.
    pub fn of_lang(lang: &Lang, required: &str) -> Self {
        let used = lang
            .texts()
            .iter()
            .flat_map(|(_, text)| text.chars())
            .filter(|&c| keepable(c))
            .collect();
        Self {
            used,
            required: required.chars().filter(|&c| keepable(c)).collect(),
        }
    }

    /// Whether the atlas keeps `c`, if its font has it.
    pub fn wants(&self, c: char) -> bool {
        keepable(c)
            && (ATLAS_RANGES.iter().any(|(lo, hi)| (lo..=hi).contains(&&c))
                || self.used.contains(&c)
                || self.required.contains(&c))
    }

    /// The characters the text uses, without control characters and
    /// combining marks.
    pub fn used(&self) -> &BTreeSet<char> {
        &self.used
    }
}

/// How many of the characters the text uses that the fonts lack
/// [`run`]'s summary lists by name.
const LACKED_SHOWN: usize = 20;

/// The one-bit atlas's file name, next to `atlas.png`.
pub const ONE_BIT_FILE: &str = "atlas_1bit.bin";

/// A bitmap font: fixed cell size, each glyph an on/off pixel grid placed
/// inside the cell (row-major, `cell_w × cell_h`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BitmapFont {
    /// Cell width in pixels.
    pub cell_w: u32,
    /// Cell height in pixels.
    pub cell_h: u32,
    /// Glyph pixels by character.
    pub glyphs: BTreeMap<char, Vec<bool>>,
}

impl BitmapFont {
    /// Adds `extra`'s glyphs. Fails if its cell size differs or it has a
    /// glyph this font already has: extra files add glyphs, they never
    /// change the font's own.
    pub fn add(&mut self, extra: Self) -> Result<(), String> {
        if (extra.cell_w, extra.cell_h) != (self.cell_w, self.cell_h) {
            return Err(format!(
                "cell size {}×{}, expected {}×{}",
                extra.cell_w, extra.cell_h, self.cell_w, self.cell_h
            ));
        }
        for (c, pixels) in extra.glyphs {
            if self.glyphs.insert(c, pixels).is_some() {
                return Err(format!("U+{:04X} is already in the font", u32::from(c)));
            }
        }
        Ok(())
    }
}

/// Numbered, trimmed lines of a BDF file.
type Lines<'a> = dyn Iterator<Item = (usize, &'a str)> + 'a;

/// The font bounding box: width, height, x offset, y offset.
type BoundingBox = [i64; 4];

/// Parses a BDF font. Glyph bitmaps are positioned in the cell using the
/// font and glyph bounding boxes; pixels outside the cell are dropped.
pub fn parse_bdf(text: &str) -> Result<BitmapFont, String> {
    let mut lines = text.lines().enumerate().map(|(i, l)| (i + 1, l.trim()));
    let mut font_box: Option<BoundingBox> = None;
    let mut glyphs = BTreeMap::new();
    while let Some((n, line)) = lines.next() {
        let mut words = line.split_whitespace();
        match words.next() {
            Some("FONTBOUNDINGBOX") => font_box = Some(numbers(words, n)?),
            Some("STARTCHAR") => {
                let font_box =
                    font_box.ok_or(format!("line {n}: STARTCHAR before FONTBOUNDINGBOX"))?;
                if let (Some(c), pixels) = parse_char(&mut lines, font_box)? {
                    glyphs.insert(c, pixels);
                }
            }
            _ => {}
        }
    }
    let [w, h, _, _] = font_box.ok_or("no FONTBOUNDINGBOX")?;
    let size = |v: i64| u32::try_from(v).ok().filter(|&v| v > 0);
    let (Some(cell_w), Some(cell_h)) = (size(w), size(h)) else {
        return Err(format!("bad FONTBOUNDINGBOX size {w}×{h}"));
    };
    Ok(BitmapFont {
        cell_w,
        cell_h,
        glyphs,
    })
}

/// Parses one glyph, from after `STARTCHAR` through `ENDCHAR`. The char is
/// `None` for glyphs without a Unicode encoding.
fn parse_char(
    lines: &mut Lines<'_>,
    font_box: BoundingBox,
) -> Result<(Option<char>, Vec<bool>), String> {
    let [cell_w, cell_h, _, _] = font_box;
    let mut encoding = None;
    let mut glyph_box = None;
    let mut pixels = vec![false; to_usize(cell_w * cell_h)];
    while let Some((n, line)) = lines.next() {
        let mut words = line.split_whitespace();
        match words.next() {
            Some("ENCODING") => {
                let [e] = numbers(words, n)?;
                encoding = u32::try_from(e).ok().and_then(char::from_u32);
            }
            Some("BBX") => glyph_box = Some(numbers(words, n)?),
            Some("BITMAP") => {
                let glyph_box = glyph_box.ok_or(format!("line {n}: BITMAP before BBX"))?;
                parse_bitmap(lines, n, font_box, glyph_box, &mut pixels)?;
            }
            Some("ENDCHAR") => break,
            _ => {}
        }
    }
    Ok((encoding, pixels))
}

/// Reads a glyph's hex bitmap rows (after `BITMAP` on line `start`) and sets
/// the matching cell pixels.
fn parse_bitmap(
    lines: &mut Lines<'_>,
    start: usize,
    [cell_w, cell_h, font_x, font_y]: BoundingBox,
    [width, height, glyph_x, glyph_y]: BoundingBox,
    pixels: &mut [bool],
) -> Result<(), String> {
    // Rows from the cell top to the glyph top: cell ascent minus glyph top.
    let top = (cell_h + font_y) - (glyph_y + height);
    for row in 0..height {
        let (n, hex) = lines
            .next()
            .ok_or(format!("line {start}: bitmap ends early"))?;
        let bits =
            u128::from_str_radix(hex, 16).map_err(|e| format!("line {n}: bad bitmap row: {e}"))?;
        let row_bits = i64::try_from(hex.len() * 4).unwrap_or(0);
        for col in 0..width.min(row_bits) {
            let (x, y) = (glyph_x - font_x + col, top + row);
            let on = (bits >> (row_bits - 1 - col)) & 1 == 1;
            if on && (0..cell_w).contains(&x) && (0..cell_h).contains(&y) {
                pixels[to_usize(y * cell_w + x)] = true;
            }
        }
    }
    Ok(())
}

/// Parses exactly `N` integers from `words`.
fn numbers<'a, const N: usize>(
    words: impl Iterator<Item = &'a str>,
    line: usize,
) -> Result<[i64; N], String> {
    let v: Vec<i64> = words
        .map(str::parse)
        .collect::<Result<_, _>>()
        .map_err(|e| format!("line {line}: {e}"))?;
    v.try_into()
        .map_err(|v: Vec<i64>| format!("line {line}: expected {N} numbers, got {}", v.len()))
}

/// Non-negative `i64` → `usize` (callers only pass in-range values).
fn to_usize(v: i64) -> usize {
    usize::try_from(v).unwrap_or(0)
}

/// The atlas: the glyph map, plus RGBA pixels (white where the glyph is
/// set, transparent elsewhere) of size `width × height`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Atlas {
    /// Layout and glyph map.
    pub def: FontAtlasDef,
    /// Image width in pixels.
    pub width: u32,
    /// Image height in pixels.
    pub height: u32,
    /// RGBA8 pixels, row-major.
    pub rgba: Vec<u8>,
}

/// Lays out every glyph of `font` that `chars` wants, in codepoint order,
/// [`COLUMNS`] per row.
pub fn build_atlas(font: &BitmapFont, chars: &AtlasChars) -> Atlas {
    let chosen: Vec<(&char, &Vec<bool>)> = font
        .glyphs
        .iter()
        .filter(|&(&c, _)| chars.wants(c))
        .collect();
    let count = u32::try_from(chosen.len()).unwrap_or(u32::MAX);
    let rows = count.div_ceil(COLUMNS).max(1);
    let def = FontAtlasDef {
        cell_w: font.cell_w,
        cell_h: font.cell_h,
        columns: COLUMNS,
        glyphs: chosen.iter().map(|&(&c, _)| c).zip(0..).collect(),
    };
    let (width, height) = (COLUMNS * font.cell_w, rows * font.cell_h);
    let mut rgba = vec![0u8; to_usize(i64::from(width) * i64::from(height) * 4)];
    for (index, (_, pixels)) in (0..).zip(&chosen) {
        let cell = def.cell_rect(index);
        for (i, _) in pixels.iter().enumerate().filter(|&(_, &on)| on) {
            let i = u32::try_from(i).unwrap_or(0);
            let (x, y) = (cell.x + i % font.cell_w, cell.y + i / font.cell_w);
            let at = to_usize((i64::from(y) * i64::from(width) + i64::from(x)) * 4);
            rgba[at..at + 4].copy_from_slice(&[255; 4]);
        }
    }
    Atlas {
        def,
        width,
        height,
        rgba,
    }
}

/// The one-bit atlas ([`ONE_BIT_FILE`]) of the atlas
/// picture `rgba` (`width` pixels wide) laid out as `def`: each glyph in
/// cell index order, one byte per pixel row, the highest bit the left
/// pixel, set where the picture isn't see-through. Fails for cells wider
/// than 8 pixels.
pub fn one_bit(def: &FontAtlasDef, width: u32, rgba: &[u8]) -> Result<Vec<u8>, String> {
    if def.cell_w > 8 {
        return Err(format!(
            "the one-bit atlas holds cells up to 8 pixels wide, not {}",
            def.cell_w
        ));
    }
    let count = def.glyphs.values().max().map_or(0, |&i| i + 1);
    let mut out = Vec::with_capacity(to_usize(i64::from(count) * i64::from(def.cell_h)));
    for index in 0..count {
        let cell = def.cell_rect(index);
        for y in cell.y..cell.y + cell.h {
            let mut row = 0u8;
            for x in cell.x..cell.x + cell.w {
                let alpha = to_usize((i64::from(y) * i64::from(width) + i64::from(x)) * 4 + 3);
                let lit = rgba.get(alpha).is_some_and(|&a| a > 0);
                row |= u8::from(lit) << (7 - (x - cell.x));
            }
            out.push(row);
        }
    }
    Ok(out)
}

/// The `atlas.ron` text for `def`, with a generated-file header naming
/// the tool as `command` and pointing at `readme`, if there is one.
pub fn atlas_ron(
    def: &FontAtlasDef,
    source_name: &str,
    command: &str,
    readme: Option<&str>,
) -> Result<String, String> {
    // LF on every OS, so regenerating never churns line endings.
    let config = ron::ser::PrettyConfig::default().new_line("\n");
    let body = ron::ser::to_string_pretty(def, config).map_err(|e| e.to_string())?;
    let first = format!("// Generated by `{command} font-atlas` from {source_name}. Do not edit");
    let maps = "Maps each glyph to its cell in atlas.png.";
    Ok(match readme {
        Some(readme) => format!("{first};\n// see {readme}. {maps}\n{body}\n"),
        None => format!("{first}.\n// {maps}\n{body}\n"),
    })
}

/// Reads and parses the BDF font at `path`.
pub fn read_font(path: &Path) -> Result<BitmapFont, String> {
    let text = fs::read_to_string(path).map_err(|e| format!("reading {}: {e}", path.display()))?;
    parse_bdf(&text).map_err(|e| format!("{}: {e}", path.display()))
}

/// The line of [`run`]'s summary naming the characters the text uses that
/// `font` lacks, the first [`LACKED_SHOWN`] by name; `None` if it has them
/// all.
fn lacked(font: &BitmapFont, chars: &AtlasChars) -> Option<String> {
    let lacked: Vec<char> = chars
        .used()
        .iter()
        .copied()
        .filter(|c| !font.glyphs.contains_key(c))
        .collect();
    if lacked.is_empty() {
        return None;
    }
    let mut named: Vec<String> = lacked
        .iter()
        .take(LACKED_SHOWN)
        .map(|&c| describe(c))
        .collect();
    if lacked.len() > LACKED_SHOWN {
        named.push(format!("and {} more", lacked.len() - LACKED_SHOWN));
    }
    Some(format!(
        "font-atlas: the fonts lack {} character(s) the text uses: {}",
        lacked.len(),
        named.join(", ")
    ))
}

/// Runs the command: reads the fonts at `font_paths` (the first is the
/// font; the rest add glyphs to it), writes the glyphs `chars` wants to
/// `atlas.png`, `atlas.ron` and `atlas_1bit.bin` in `out_dir`. Fails if
/// the result lacks a glyph `rules` require; a character the text uses
/// that the fonts lack is named in the summary, not a failure. `command` is
/// what `atlas.ron`'s header calls the tool.
pub fn run(
    rules: &FontAtlas,
    command: &str,
    font_paths: &[PathBuf],
    out_dir: &Path,
    chars: &AtlasChars,
) -> Result<String, String> {
    let (font_path, extras) = font_paths.split_first().ok_or("no font given")?;
    let mut font = read_font(font_path)?;
    for path in extras {
        let extra = read_font(path)?;
        font.add(extra)
            .map_err(|e| format!("{}: {e}", path.display()))?;
    }
    let atlas = build_atlas(&font, chars);
    let problems = atlas.def.problems(&rules.required_glyphs);
    if !problems.is_empty() {
        return Err(format!(
            "{}: unusable atlas:\n  {}",
            font_path.display(),
            problems.join("\n  ")
        ));
    }
    let names: Vec<String> = font_paths
        .iter()
        .filter_map(|p| p.file_name())
        .map(|n| n.to_string_lossy().into_owned())
        .collect();
    let ron = atlas_ron(
        &atlas.def,
        &names.join(" + "),
        command,
        rules.readme.as_deref(),
    )?;
    let png = encode_png(atlas.width, atlas.height, &atlas.rgba)?;
    let bits = one_bit(&atlas.def, atlas.width, &atlas.rgba)?;
    fs::create_dir_all(out_dir).map_err(|e| format!("creating {}: {e}", out_dir.display()))?;
    for (file, bytes) in [
        ("atlas.png", png.as_slice()),
        ("atlas.ron", ron.as_bytes()),
        (ONE_BIT_FILE, bits.as_slice()),
    ] {
        let path = out_dir.join(file);
        fs::write(&path, bytes).map_err(|e| format!("writing {}: {e}", path.display()))?;
    }
    let mut summary = format!(
        "font-atlas: {} glyphs, {}×{} px → {}",
        atlas.def.glyphs.len(),
        atlas.width,
        atlas.height,
        out_dir.display()
    );
    if let Some(lacked) = lacked(&font, chars) {
        summary.push('\n');
        summary.push_str(&lacked);
    }
    Ok(summary)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec::decode_png;

    /// What a project requires: the fallback glyph and two letters.
    fn rules() -> FontAtlas {
        FontAtlas {
            required_glyphs: "?Ab".to_owned(),
            readme: None,
        }
    }

    /// A 4×4 font (descent 1): 'A' is a 2×2 box at the baseline's left,
    /// 'b' a single pixel with an x offset, one unencoded glyph.
    const TINY: &str = "STARTFONT 2.1\n\
        FONTBOUNDINGBOX 4 4 0 -1\n\
        CHARS 3\n\
        STARTCHAR A\nENCODING 65\nBBX 2 2 0 0\nBITMAP\nC0\n80\nENDCHAR\n\
        STARTCHAR b\nENCODING 98\nBBX 1 1 2 -1\nBITMAP\n80\nENDCHAR\n\
        STARTCHAR none\nENCODING -1\nBBX 1 1 0 0\nBITMAP\n80\nENDCHAR\n\
        ENDFONT\n";

    fn grid(font: &BitmapFont, c: char) -> Vec<String> {
        let px = &font.glyphs[&c];
        px.chunks(to_usize(i64::from(font.cell_w)))
            .map(|row| row.iter().map(|&on| if on { '#' } else { '.' }).collect())
            .collect()
    }

    #[test]
    fn parses_and_positions_glyphs() {
        let font = parse_bdf(TINY).unwrap();
        assert_eq!((font.cell_w, font.cell_h), (4, 4));
        assert_eq!(font.glyphs.keys().collect::<String>(), "Ab");
        assert_eq!(grid(&font, 'A'), ["....", "##..", "#...", "...."]);
        assert_eq!(grid(&font, 'b'), ["....", "....", "....", "..#."]);
    }

    #[test]
    fn font_x_offset_shifts_glyphs() {
        // The font box starts 1 px left of the origin, so a glyph at x = 0
        // lands in cell column 1.
        let bdf = "FONTBOUNDINGBOX 4 2 -1 0\n\
            STARTCHAR a\nENCODING 97\nBBX 1 1 0 0\nBITMAP\n80\nENDCHAR\n";
        let font = parse_bdf(bdf).unwrap();
        assert_eq!(grid(&font, 'a'), ["....", ".#.."]);
    }

    #[test]
    fn parse_errors() {
        assert!(
            parse_bdf("STARTCHAR A\n")
                .unwrap_err()
                .contains("before FONTBOUNDINGBOX")
        );
        assert_eq!(parse_bdf("").unwrap_err(), "no FONTBOUNDINGBOX");
        assert!(
            parse_bdf("FONTBOUNDINGBOX 0 4 0 0\n")
                .unwrap_err()
                .contains("bad FONTBOUNDINGBOX")
        );
        assert!(
            parse_bdf("FONTBOUNDINGBOX 4 4 0\n")
                .unwrap_err()
                .contains("expected 4 numbers, got 3")
        );
        assert!(
            parse_bdf("FONTBOUNDINGBOX 4 x 0 0\n")
                .unwrap_err()
                .starts_with("line 1: ")
        );
        let no_bbx = "FONTBOUNDINGBOX 4 4 0 0\nSTARTCHAR A\nBITMAP\n";
        assert!(
            parse_bdf(no_bbx)
                .unwrap_err()
                .contains("line 3: BITMAP before BBX")
        );
        let short = "FONTBOUNDINGBOX 4 4 0 0\nSTARTCHAR A\nBBX 1 2 0 0\nBITMAP\n80\n";
        assert!(parse_bdf(short).unwrap_err().contains("bitmap ends early"));
        let bad = "FONTBOUNDINGBOX 4 4 0 0\nSTARTCHAR A\nBBX 1 1 0 0\nBITMAP\nzz\n";
        assert!(parse_bdf(bad).unwrap_err().contains("bad bitmap row"));
    }

    #[test]
    fn atlas_layout_and_pixels() {
        let atlas = build_atlas(&parse_bdf(TINY).unwrap(), &AtlasChars::default());
        assert_eq!(atlas.def.columns, COLUMNS);
        assert_eq!(atlas.def.glyphs, [('A', 0), ('b', 1)].into());
        assert_eq!((atlas.width, atlas.height), (COLUMNS * 4, 4));
        let lit =
            |x: u32, y: u32| atlas.rgba[to_usize(i64::from((y * atlas.width + x) * 4 + 3))] == 255;
        assert!(lit(0, 1) && lit(1, 1) && lit(0, 2));
        assert!(!lit(1, 2) && !lit(0, 0));
        assert!(lit(4 + 2, 3));
        let lit_count = atlas.rgba.chunks(4).filter(|p| p == &[255; 4]).count();
        assert_eq!(lit_count, 4);
        assert!(atlas.rgba.chunks(4).all(|p| p == [255; 4] || p == [0; 4]));
    }

    #[test]
    fn ron_round_trips_with_header() {
        let def = build_atlas(&parse_bdf(TINY).unwrap(), &AtlasChars::default()).def;
        let text = atlas_ron(&def, "tiny.bdf", "shuyi", None).unwrap();
        assert!(text.starts_with(
            "// Generated by `shuyi font-atlas` from tiny.bdf. Do not edit.\n\
             // Maps each glyph to its cell in atlas.png.\n("
        ));
        assert!(!text.contains("\r\n"), "LF only");
        assert_eq!(ron::from_str::<FontAtlasDef>(&text).unwrap(), def);
        let text = atlas_ron(&def, "a + b", "tools", Some("fonts/README.md")).unwrap();
        assert!(text.starts_with(
            "// Generated by `tools font-atlas` from a + b. Do not edit;\n\
             // see fonts/README.md. Maps each glyph to its cell in atlas.png.\n("
        ));
        assert_eq!(ron::from_str::<FontAtlasDef>(&text).unwrap(), def);
    }

    #[test]
    fn run_rejects_fonts_missing_required_glyphs() {
        let dir = std::env::temp_dir().join(format!("shuyi-font-atlas-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let bdf = dir.join("tiny.bdf");
        fs::write(&bdf, TINY).unwrap();
        let run = |fonts: &[PathBuf], out: &Path| {
            run(&rules(), "shuyi", fonts, out, &AtlasChars::default())
        };
        let err = run(std::slice::from_ref(&bdf), &dir.join("out")).unwrap_err();
        assert!(err.contains("missing required glyphs"), "{err}");
        assert!(!dir.join("out").exists());
        assert!(
            run(&[dir.join("missing.bdf")], &dir)
                .unwrap_err()
                .starts_with("reading ")
        );
        // An extra font that can't be read or doesn't fit is named.
        let err = run(&[bdf.clone(), dir.join("missing.bdf")], &dir).unwrap_err();
        assert!(err.starts_with("reading ") && err.contains("missing.bdf"));
        let err = run(&[bdf.clone(), bdf], &dir).unwrap_err();
        assert!(err.contains("tiny.bdf: U+0041 is already in the font"));
        assert_eq!(run(&[], &dir).unwrap_err(), "no font given");
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn run_writes_the_atlas_its_one_bit_copy_and_the_ron() {
        let dir = std::env::temp_dir().join(format!("shuyi-font-run-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let bdf = dir.join("tiny.bdf");
        let question = "STARTCHAR q\nENCODING 63\nBBX 1 1 0 0\nBITMAP\n80\nENDCHAR\n";
        fs::write(&bdf, TINY.replace("ENDFONT", question)).unwrap();
        let out = dir.join("out");
        let said = run(
            &rules(),
            "shuyi",
            std::slice::from_ref(&bdf),
            &out,
            &AtlasChars::default(),
        )
        .unwrap();
        assert_eq!(
            said,
            format!("font-atlas: 3 glyphs, 128×4 px → {}", out.display())
        );
        let (w, h, _) = decode_png(&fs::read(out.join("atlas.png")).unwrap()).unwrap();
        assert_eq!((w, h), (128, 4));
        let ron = fs::read_to_string(out.join("atlas.ron")).unwrap();
        assert!(ron.starts_with("// Generated by `shuyi font-atlas` from tiny.bdf."));
        assert_eq!(fs::read(out.join(ONE_BIT_FILE)).unwrap().len(), 3 * 4);
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn extra_fonts_add_glyphs_and_never_replace_one() {
        let extra = |body: &str| parse_bdf(&format!("FONTBOUNDINGBOX 4 4 0 -1\n{body}")).unwrap();
        let mut font = parse_bdf(TINY).unwrap();
        let c = "STARTCHAR c\nENCODING 99\nBBX 1 1 0 0\nBITMAP\n80\nENDCHAR\n";
        font.add(extra(c)).unwrap();
        assert_eq!(font.glyphs.keys().collect::<String>(), "Abc");
        assert_eq!(grid(&font, 'c'), ["....", "....", "#...", "...."]);
        assert_eq!(grid(&font, 'A'), ["....", "##..", "#...", "...."]);
        // The same glyph again, or another cell size: refused.
        assert_eq!(
            font.add(extra(c)).unwrap_err(),
            "U+0063 is already in the font"
        );
        let wide = parse_bdf("FONTBOUNDINGBOX 5 4 0 -1\n").unwrap();
        assert_eq!(font.add(wide).unwrap_err(), "cell size 5×4, expected 4×4");
        let tall = parse_bdf("FONTBOUNDINGBOX 4 5 0 -1\n").unwrap();
        assert_eq!(font.add(tall).unwrap_err(), "cell size 4×5, expected 4×4");
    }

    /// `TINY` with `?`, and `Ж`, `ł` and a combining acute (U+0301), none
    /// of them in the ranges.
    fn wider() -> String {
        let glyph = |name: &str, code: u32| {
            format!("STARTCHAR {name}\nENCODING {code}\nBBX 1 1 0 0\nBITMAP\n80\nENDCHAR\n")
        };
        let extra = [
            ("q", 63),
            ("zhe", 0x416),
            ("lstroke", 0x142),
            ("acute", 0x301),
        ]
        .map(|(name, code)| glyph(name, code))
        .concat();
        TINY.replace("ENDFONT", &extra)
    }

    /// English screen text `menu.title` written `text`.
    fn text(text: &str) -> Lang {
        let english = [("menu.title".to_owned(), text.to_owned())];
        Lang::new(english.into(), BTreeMap::new())
    }

    #[test]
    fn the_atlas_keeps_the_ranges_the_text_and_the_required_glyphs() {
        let chars = AtlasChars::of_lang(&text("Жar\u{301}\n"), "Ł");
        let used: String = chars.used().iter().collect();
        assert_eq!(used, "arЖ", "no control character or combining mark");
        for c in ['A', 'é', '♣', 'Ж', 'Ł'] {
            assert!(chars.wants(c), "{c:?}");
        }
        for c in ['ł', '\u{301}', '\n', '\u{7}'] {
            assert!(!chars.wants(c), "{c:?}");
        }
        // The ranges alone, as before.
        let ranges = AtlasChars::default();
        assert!(ranges.used().is_empty());
        assert!(ranges.wants('A') && ranges.wants('é') && !ranges.wants('Ж'));
        assert!(!AtlasChars::of_lang(&Lang::default(), "\u{301}").wants('\u{301}'));
    }

    #[test]
    fn the_atlas_has_the_texts_glyphs_in_code_point_order() {
        let font = parse_bdf(&wider()).unwrap();
        let plain = build_atlas(&font, &AtlasChars::default());
        assert_eq!(plain.def.drawn().collect::<String>(), "?Ab");
        let chars = AtlasChars::of_lang(&text("Ж ł e\u{301}"), "");
        let atlas = build_atlas(&font, &chars);
        assert_eq!(atlas.def.drawn().collect::<String>(), "?AbłЖ");
        // The ranges' glyphs keep their cells; the text's come after.
        assert_eq!(atlas.def.glyphs[&'b'], plain.def.glyphs[&'b']);
        assert_eq!(atlas.def.glyphs[&'Ж'], 4);
    }

    #[test]
    fn run_keeps_the_texts_characters_and_names_those_the_fonts_lack() {
        let dir = std::env::temp_dir().join(format!("shuyi-font-text-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let bdf = dir.join("wider.bdf");
        fs::write(&bdf, wider()).unwrap();
        let out = dir.join("out");
        let chars = AtlasChars::of_lang(&text("ЖłЖЯ∑b\u{301}"), "");
        let said = run(&rules(), "shuyi", std::slice::from_ref(&bdf), &out, &chars).unwrap();
        assert_eq!(
            said,
            format!(
                "font-atlas: 5 glyphs, 128×4 px → {}\nfont-atlas: the fonts lack 2 character(s) \
                 the text uses: 'Я' (U+042F), '∑' (U+2211)",
                out.display()
            )
        );
        let ron = fs::read_to_string(out.join("atlas.ron")).unwrap();
        let def: FontAtlasDef = ron::from_str(&ron).unwrap();
        assert_eq!(def.drawn().collect::<String>(), "?AbłЖ");
        // A text of every character but the few the summary lists.
        let many: String = ('\u{4e00}'..'\u{4e19}').collect();
        let chars = AtlasChars::of_lang(&text(&many), "");
        let said = run(&rules(), "shuyi", std::slice::from_ref(&bdf), &out, &chars).unwrap();
        assert!(said.contains("lack 25 character(s)"), "{said}");
        let last = format!("{}, and 5 more", describe('\u{4e13}'));
        assert!(said.ends_with(&last), "{said}");
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn the_templates_atlas_is_what_font_atlas_makes_of_its_bdf() {
        let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../templates/shuyi-default/base");
        let shuyi_ron = fs::read_to_string(base.join("shuyi.ron")).unwrap();
        let config = crate::project::Config::parse(&shuyi_ron).unwrap();
        let rules = config.font_atlas.unwrap();
        assert_eq!(rules.required_glyphs, shuyi_ui::basic::BASIC_GLYPHS);
        let font = read_font(&base.join("files-src/ter-u16n.bdf")).unwrap();
        // A new game's text uses nothing outside the ranges: the atlas is
        // the ranges' alone.
        let atlas = build_atlas(&font, &AtlasChars::default());
        let committed = base.join("crates/game/files/font");
        let png = fs::read(committed.join("atlas.png")).unwrap();
        let (width, height, rgba) = decode_png(&png).unwrap();
        assert_eq!((width, height), (atlas.width, atlas.height));
        assert!(rgba == atlas.rgba, "the template's atlas.png is stale");
        let command = config.command.as_deref().unwrap_or("shuyi");
        let ron = atlas_ron(&atlas.def, "ter-u16n.bdf", command, rules.readme.as_deref()).unwrap();
        let written = fs::read_to_string(committed.join("atlas.ron")).unwrap();
        assert_eq!(
            written.replace("\r\n", "\n"),
            ron,
            "the template's atlas.ron is stale"
        );
        let found = atlas.def.problems(&rules.required_glyphs);
        assert!(found.is_empty(), "{found:?}");
    }

    proptest::proptest! {
        /// Every character of the text is used, unless it is a control
        /// character or a combining mark.
        #[test]
        fn every_character_of_the_text_is_used_but_controls_and_marks(
            texts in proptest::collection::vec("\\PC{0,8}|[a\\n\u{301}é\u{7}]{0,4}", 1..4),
        ) {
            let english: BTreeMap<String, String> = texts
                .iter()
                .enumerate()
                .map(|(i, text)| (format!("k.{i}"), text.clone()))
                .collect();
            let chars = AtlasChars::of_lang(&Lang::new(english, BTreeMap::new()), "");
            for c in texts.iter().flat_map(|t| t.chars()) {
                let kept = !c.is_control() && !is_combining_mark(c);
                proptest::prop_assert_eq!(chars.used().contains(&c), kept, "{:?}", c);
            }
        }
    }

    #[test]
    fn the_one_bit_atlas_packs_rows_left_pixel_highest() {
        let atlas = build_atlas(&parse_bdf(TINY).unwrap(), &AtlasChars::default());
        let bits = one_bit(&atlas.def, atlas.width, &atlas.rgba).unwrap();
        // 'A' (index 0) then 'b' (index 1), four rows each.
        assert_eq!(bits, [0, 0b1100_0000, 0b1000_0000, 0, 0, 0, 0, 0b0010_0000]);
        let wide = FontAtlasDef {
            cell_w: 9,
            ..atlas.def
        };
        assert_eq!(
            one_bit(&wide, atlas.width, &atlas.rgba).unwrap_err(),
            "the one-bit atlas holds cells up to 8 pixels wide, not 9"
        );
        let empty = FontAtlasDef::default();
        let found = one_bit(&empty, 0, &[]).unwrap();
        assert!(found.is_empty(), "{found:?}");
    }
}
