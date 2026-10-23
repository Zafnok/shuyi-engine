//! Named colours: a palette file (`{"name": "#RRGGBB", ...}`) that all
//! drawing code refers to by name. The game says which names it needs; a
//! theme is another palette file with exactly the names of the default
//! one ([`theme_problems`]). As a format (ADR-0007): [`PaletteRon`].
use crate::prelude::*;
use serde::{Deserialize, Serialize};

use alloc::collections::BTreeMap;

use crate::error::ContentError;
use crate::format::{AssetKind, AssetPath, DecodeCx, Format, TEXT_MAX_BYTES, text};
use crate::ron::parse_ron;

impl AssetKind for PaletteDef {
    const NAME: &'static str = "palette";
}

/// A palette file (`palette`, `.ron`), checked as
/// [`PaletteDef::from_source`] says against the colours the game
/// requires.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PaletteRon {
    required: Vec<String>,
}

impl PaletteRon {
    /// A palette that must hold every colour of `required`.
    pub fn requiring<S: Into<String>>(required: impl IntoIterator<Item = S>) -> Self {
        Self {
            required: required.into_iter().map(Into::into).collect(),
        }
    }
}

impl Format for PaletteRon {
    type Asset = PaletteDef;

    fn id(&self) -> &'static str {
        "palette"
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
        _cx: &DecodeCx<'_>,
    ) -> Result<PaletteDef, Vec<ContentError>> {
        let source = text(file, bytes)?;
        let required: Vec<&str> = self.required.iter().map(String::as_str).collect();
        PaletteDef::from_source(file.shown(), source, &required)
    }
}

/// The validated palette: colour name → RGB.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct PaletteDef {
    /// Every colour in the file, by name.
    pub colors: BTreeMap<String, [u8; 3]>,
}

impl PaletteDef {
    /// Looks up a colour by name.
    pub fn get(&self, name: &str) -> Option<[u8; 3]> {
        self.colors.get(name).copied()
    }

    /// Parses and validates palette `source`, attributing errors to `file`.
    /// Reports every bad hex value and every colour of `required` it lacks.
    pub fn from_source(
        file: &str,
        source: &str,
        required: &[&str],
    ) -> Result<Self, Vec<ContentError>> {
        let raw: BTreeMap<String, String> = parse_ron(file, source).map_err(|e| vec![e])?;
        let mut errors = Vec::new();
        let mut colors = BTreeMap::new();
        for (name, value) in &raw {
            match parse_hex(value) {
                Ok(rgb) => {
                    colors.insert(name.clone(), rgb);
                }
                Err(why) => {
                    let mut e = ContentError::new(file, format!("colour \"{name}\": {why}"));
                    if let Some(line) = line_of_key(source, name) {
                        e = e.at(line, None);
                    }
                    errors.push(e);
                }
            }
        }
        for &name in required {
            if !raw.contains_key(name) {
                errors.push(ContentError::new(
                    file,
                    format!("missing required colour \"{name}\""),
                ));
            }
        }
        if errors.is_empty() {
            Ok(Self { colors })
        } else {
            Err(errors)
        }
    }
}

/// What is wrong with theme `palette` (its file shown as `file`): it must
/// define exactly the colour names of the default theme's `base` (a name
/// missing would leave something unpainted; an extra one is a typo).
pub fn theme_problems(file: &str, base: &PaletteDef, palette: &PaletteDef) -> Vec<ContentError> {
    let missing = base
        .colors
        .keys()
        .filter(|n| !palette.colors.contains_key(*n))
        .map(|name| format!("missing colour \"{name}\""));
    let extra = palette
        .colors
        .keys()
        .filter(|n| !base.colors.contains_key(*n))
        .map(|name| format!("colour \"{name}\" is not in the default theme"));
    missing
        .chain(extra)
        .map(|message| ContentError::new(file, message))
        .collect()
}

/// Parses `#RRGGBB` (hex digits in either case) into RGB.
pub fn parse_hex(s: &str) -> Result<[u8; 3], String> {
    let Some(digits) = s.strip_prefix('#') else {
        return Err(format!("\"{s}\" must start with '#' (format \"#RRGGBB\")"));
    };
    if digits.len() != 6 || !digits.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(format!(
            "\"{s}\" must be '#' followed by exactly 6 hex digits (format \"#RRGGBB\")"
        ));
    }
    let channel = |i: usize| u8::from_str_radix(&digits[i..i + 2], 16).map_err(|e| e.to_string());
    Ok([channel(0)?, channel(2)?, channel(4)?])
}

/// Formats RGB as lowercase `#rrggbb`.
pub fn format_hex([r, g, b]: [u8; 3]) -> String {
    format!("#{r:02x}{g:02x}{b:02x}")
}

/// 1-based line of the first line containing `"key"`, for error positions.
pub fn line_of_key(source: &str, key: &str) -> Option<u32> {
    let needle = format!("\"{key}\"");
    let index = source.lines().position(|l| l.contains(&needle))?;
    u32::try_from(index + 1).ok()
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    /// The colours these tests require.
    const REQUIRED: &[&str] = &["black", "white", "text", "player", "enemy", "cursor"];

    fn full_palette_except(skip: &str, extra: &str) -> String {
        let lines: Vec<String> = REQUIRED
            .iter()
            .filter(|&&name| name != skip)
            .map(|name| format!("    \"{name}\": \"#010203\",\n"))
            .collect();
        format!("{{\n{}{extra}}}", lines.concat())
    }

    fn from_source(file: &str, source: &str) -> Result<PaletteDef, Vec<ContentError>> {
        PaletteDef::from_source(file, source, REQUIRED)
    }

    #[test]
    fn hex_valid_upper_and_lower() {
        assert_eq!(parse_hex("#1a2B3c"), Ok([0x1a, 0x2b, 0x3c]));
        assert_eq!(parse_hex("#FFFFFF"), Ok([255, 255, 255]));
        assert_eq!(parse_hex("#000000"), Ok([0, 0, 0]));
    }

    #[test]
    fn hex_missing_hash() {
        assert!(parse_hex("1a2b3c").is_err_and(|e| e.contains("'#'")));
    }

    #[test]
    fn hex_wrong_length() {
        assert!(parse_hex("#1a2b3").is_err());
        assert!(parse_hex("#1a2b3c4").is_err());
        assert!(parse_hex("#").is_err());
    }

    #[test]
    fn hex_non_hex_digits() {
        assert!(parse_hex("#1a2g3c").is_err());
        assert!(parse_hex("#+1+2+3").is_err());
        // Multi-byte chars must not panic on slicing.
        assert!(parse_hex("#ééé").is_err());
    }

    #[test]
    fn format_hex_is_lowercase_padded() {
        assert_eq!(format_hex([0x0a, 0xbc, 0x01]), "#0abc01");
    }

    proptest! {
        #[test]
        fn hex_round_trips(c in any::<[u8; 3]>()) {
            prop_assert_eq!(parse_hex(&format_hex(c)), Ok(c));
            prop_assert_eq!(parse_hex(&format_hex(c).to_uppercase()), Ok(c));
        }
    }

    #[test]
    fn valid_palette_loads_all_colours() {
        let src = full_palette_except("", "    \"grass\": \"#00FF00\",\n");
        let p = from_source("p.ron", &src).unwrap_or_default();
        assert_eq!(p.get("grass"), Some([0, 255, 0]));
        assert_eq!(p.get("player"), Some([1, 2, 3]));
        assert_eq!(p.colors.len(), REQUIRED.len() + 1);
        // Nothing required: any names will do.
        let any = PaletteDef::from_source("p.ron", "{\"x\": \"#000000\"}", &[]);
        assert_eq!(any.map(|p| p.colors.len()), Ok(1));
    }

    #[test]
    fn missing_required_colour_is_error() {
        let src = full_palette_except("player", "");
        let errs = from_source("p.ron", &src).err().unwrap_or_default();
        assert_eq!(errs.len(), 1);
        assert_eq!(
            errs[0].to_string(),
            "p.ron: missing required colour \"player\""
        );
    }

    #[test]
    fn bad_hex_is_error_with_line() {
        let src = full_palette_except("", "    \"grass\": \"00ff00\",\n");
        let errs = from_source("p.ron", &src).err().unwrap_or_default();
        assert_eq!(errs.len(), 1);
        let line = u32::try_from(REQUIRED.len() + 2).unwrap_or(0);
        assert_eq!(errs[0].line, Some(line));
        assert!(errs[0].message.contains("\"grass\""));
        assert!(errs[0].message.contains("must start with '#'"));
    }

    #[test]
    fn multiple_errors_reported_together() {
        let src = full_palette_except(
            "enemy",
            "    \"grass\": \"#12\",\n    \"road\": \"#zzzzzz\",\n",
        );
        let errs = from_source("p.ron", &src).err().unwrap_or_default();
        assert_eq!(errs.len(), 3);
        let text: Vec<String> = errs.iter().map(ToString::to_string).collect();
        assert!(text.iter().any(|t| t.contains("\"enemy\"")));
        assert!(text.iter().any(|t| t.contains("\"grass\"")));
        assert!(text.iter().any(|t| t.contains("\"road\"")));
    }

    #[test]
    fn syntax_error_is_single_positioned_error() {
        let errs = from_source("p.ron", "{\n  \"a\" \"#000000\"\n}")
            .err()
            .unwrap_or_default();
        assert_eq!(errs.len(), 1);
        assert_eq!(errs[0].line, Some(2));
    }

    #[test]
    fn line_of_key_finds_quoted_key() {
        let src = "{\n \"text_dim\": \"#000000\",\n \"text\": \"#000000\",\n}";
        assert_eq!(line_of_key(src, "text"), Some(3));
        assert_eq!(line_of_key(src, "text_dim"), Some(2));
        assert_eq!(line_of_key(src, "nope"), None);
    }

    fn palette(names: &[&str]) -> PaletteDef {
        PaletteDef {
            colors: names.iter().map(|&n| (n.to_owned(), [1, 2, 3])).collect(),
        }
    }

    #[test]
    fn a_theme_must_have_exactly_the_default_themes_names() {
        let base = palette(&["a", "b", "c"]);
        // Other colours, same names: nothing wrong.
        let mut same = palette(&["a", "b", "c"]);
        same.colors.insert("a".into(), [9, 9, 9]);
        let found = theme_problems("t.ron", &base, &same);
        assert!(found.is_empty(), "{found:?}");
        let other = palette(&["a", "c", "d", "e"]);
        let text: Vec<String> = theme_problems("t.ron", &base, &other)
            .iter()
            .map(ToString::to_string)
            .collect();
        assert_eq!(
            text,
            [
                "t.ron: missing colour \"b\"",
                "t.ron: colour \"d\" is not in the default theme",
                "t.ron: colour \"e\" is not in the default theme",
            ]
        );
    }
}
