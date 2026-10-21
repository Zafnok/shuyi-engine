//! One-bit output by role (ADR-0008 §4): a fixed set of 8 × 8 [`Pattern`]s,
//! and the pattern map (`palette@onebit.ron`, beside the palette) that
//! gives each of a skin's palette roles a foreground and a background
//! pattern, with the groups of roles that tell things apart and must stay
//! distinct. As a format (ADR-0007): [`PatternMapRon`]. [`check`] is what
//! `shuyi check --target` checks of a one-bit target; the renderer that
//! draws with the map is `shuyi_ui::frame::onebit`.
//!
//! ```ron
//! (
//!     roles: {
//!         "text": (fg: Black, bg: White),
//!         "team.player": (fg: White),
//!         "team.enemy": (fg: Dither50),
//!         "team.ally": (fg: Inverted(DiagonalUp)),
//!     },
//!     distinct: [["team.player", "team.enemy", "team.ally"]],
//!     text: ["text"],
//! )
//! ```
use crate::prelude::*;

use alloc::collections::{BTreeMap, BTreeSet};
use core::fmt;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::error::ContentError;
use crate::format::{AssetKind, AssetPath, DecodeCx, Format, TEXT_MAX_BYTES, text};
use crate::palette::{PaletteDef, format_hex};
use crate::ron::parse_ron_implicit_some;

/// The Bayer 8 × 8 order, by row then column: the dithers' thresholds. A
/// dither with `k` black pixels is black where the order is below `k`.
pub const BAYER: [[u8; 8]; 8] = [
    [0, 32, 8, 40, 2, 34, 10, 42],
    [48, 16, 56, 24, 50, 18, 58, 26],
    [12, 44, 4, 36, 14, 46, 6, 38],
    [60, 28, 52, 20, 62, 30, 54, 22],
    [3, 35, 11, 43, 1, 33, 9, 41],
    [51, 19, 59, 27, 49, 17, 57, 25],
    [15, 47, 7, 39, 13, 45, 5, 37],
    [63, 31, 55, 23, 61, 29, 53, 21],
];

/// The eight rows of a pattern: a row's highest bit is its left pixel, a
/// set bit white, as in the Playdate's frame buffer.
pub type Rows = [u8; 8];

/// Bayer ordered dither of a grey `darkness` from 0 (white) to 255
/// (black): black where `4 × order + 2 < darkness`, so 64 is
/// [`Base::Dither25`], 128 [`Base::Dither50`] and 192
/// [`Base::Dither75`] exactly, and the levels between fall evenly.
pub fn dither(darkness: u8) -> Rows {
    // At most 4 × 63 + 2 = 254: no overflow.
    rows_where(|x, y| 4 * BAYER[y][x] + 2 < darkness)
}

/// The rows of a pattern black where `black(x, y)` says (each 0 to 7).
fn rows_where(black: impl Fn(usize, usize) -> bool) -> Rows {
    let mut rows = [0; 8];
    for (y, row) in rows.iter_mut().enumerate() {
        for x in 0..8 {
            if !black(x, y) {
                *row |= 0x80 >> x;
            }
        }
    }
    rows
}

/// One of the fixed patterns, before any inversion: black and white
/// solids, three ordered dithers, and four kinds of line.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Base {
    /// Every pixel black.
    Black,
    /// Every pixel white.
    White,
    /// A quarter of the pixels black, Bayer ordered.
    Dither25,
    /// Half the pixels black, Bayer ordered: a one-pixel checker.
    Dither50,
    /// Three quarters of the pixels black, Bayer ordered.
    Dither75,
    /// Black lines rising to the right, one pixel in four.
    DiagonalUp,
    /// Black lines falling to the right, one pixel in four.
    DiagonalDown,
    /// Black rows, one in four.
    HLines,
    /// Black columns, one in four.
    VLines,
    /// A checker of 2 × 2 squares.
    Checker2,
}

impl Base {
    /// Every base pattern.
    pub const ALL: [Self; 10] = [
        Self::Black,
        Self::White,
        Self::Dither25,
        Self::Dither50,
        Self::Dither75,
        Self::DiagonalUp,
        Self::DiagonalDown,
        Self::HLines,
        Self::VLines,
        Self::Checker2,
    ];

    /// Its name in a pattern map.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Black => "Black",
            Self::White => "White",
            Self::Dither25 => "Dither25",
            Self::Dither50 => "Dither50",
            Self::Dither75 => "Dither75",
            Self::DiagonalUp => "DiagonalUp",
            Self::DiagonalDown => "DiagonalDown",
            Self::HLines => "HLines",
            Self::VLines => "VLines",
            Self::Checker2 => "Checker2",
        }
    }

    /// How many of its 64 pixels are black.
    pub const fn black_pixels(self) -> u32 {
        match self {
            Self::Black => 64,
            Self::White => 0,
            Self::Dither25
            | Self::DiagonalUp
            | Self::DiagonalDown
            | Self::HLines
            | Self::VLines => 16,
            Self::Dither50 | Self::Checker2 => 32,
            Self::Dither75 => 48,
        }
    }

    /// Whether its pixel `(x, y)` (each 0 to 7) is black.
    const fn is_black(self, x: usize, y: usize) -> bool {
        match self {
            Self::Black => true,
            Self::White => false,
            Self::Dither25 => BAYER[y][x] < 16,
            Self::Dither50 => BAYER[y][x] < 32,
            Self::Dither75 => BAYER[y][x] < 48,
            Self::DiagonalUp => (x + y).is_multiple_of(4),
            Self::DiagonalDown => (x + 8 - y).is_multiple_of(4),
            Self::HLines => y.is_multiple_of(4),
            Self::VLines => x.is_multiple_of(4),
            Self::Checker2 => (x / 2 + y / 2).is_multiple_of(2),
        }
    }

    /// Its rows.
    pub fn rows(self) -> Rows {
        rows_where(|x, y| self.is_black(x, y))
    }
}

/// A pattern of 8 × 8 pixels: one of the [`Base`] patterns, or one of
/// them inverted. A pattern map spells it `Dither50` or
/// `Inverted(Dither50)`.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Pattern {
    base: Base,
    inverted: bool,
}

impl Pattern {
    /// Every pixel black.
    pub const BLACK: Self = Self::new(Base::Black);
    /// Every pixel white.
    pub const WHITE: Self = Self::new(Base::White);

    /// `base`, as it is.
    pub const fn new(base: Base) -> Self {
        Self {
            base,
            inverted: false,
        }
    }

    /// The same pattern with black and white swapped.
    #[must_use]
    pub const fn inverted(self) -> Self {
        Self {
            base: self.base,
            inverted: !self.inverted,
        }
    }

    /// The base pattern.
    pub const fn base(self) -> Base {
        self.base
    }

    /// Whether it is its base inverted.
    pub const fn is_inverted(self) -> bool {
        self.inverted
    }

    /// Its rows, fixed to the screen: screen pixel `(x, y)` takes bit
    /// `x % 8` of row `y % 8`.
    pub fn rows(self) -> Rows {
        let rows = self.base.rows();
        if self.inverted {
            rows.map(|row| !row)
        } else {
            rows
        }
    }

    /// How many of its 64 pixels are black.
    pub const fn black_pixels(self) -> u32 {
        if self.inverted {
            64 - self.base.black_pixels()
        } else {
            self.base.black_pixels()
        }
    }

    /// Whether it is solid black or solid white.
    pub const fn is_solid(self) -> bool {
        matches!(self.base, Base::Black | Base::White)
    }

    /// Whether the two look the same on a screen: one's pixels are the
    /// other's moved (`Dither50` and `Inverted(Dither50)` are both a
    /// one-pixel checker, one pixel apart).
    pub fn looks_like(self, other: Self) -> bool {
        let (a, b) = (self.rows(), other.rows());
        (0..8).any(|dy| (0..8u32).any(|dx| (0..8).all(|y| a[y] == b[(y + dy) % 8].rotate_left(dx))))
    }
}

impl From<Base> for Pattern {
    fn from(base: Base) -> Self {
        Self::new(base)
    }
}

impl fmt::Display for Pattern {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.inverted {
            write!(f, "Inverted({})", self.base.name())
        } else {
            f.write_str(self.base.name())
        }
    }
}

/// How a pattern map spells a pattern: a base, or `Inverted(…)` of one.
#[derive(Serialize, Deserialize)]
#[serde(rename = "Pattern")]
enum Spelled {
    Black,
    White,
    Dither25,
    Dither50,
    Dither75,
    DiagonalUp,
    DiagonalDown,
    HLines,
    VLines,
    Checker2,
    Inverted(Box<Spelled>),
}

impl Spelled {
    fn of(pattern: Pattern) -> Self {
        let plain = match pattern.base {
            Base::Black => Self::Black,
            Base::White => Self::White,
            Base::Dither25 => Self::Dither25,
            Base::Dither50 => Self::Dither50,
            Base::Dither75 => Self::Dither75,
            Base::DiagonalUp => Self::DiagonalUp,
            Base::DiagonalDown => Self::DiagonalDown,
            Base::HLines => Self::HLines,
            Base::VLines => Self::VLines,
            Base::Checker2 => Self::Checker2,
        };
        if pattern.inverted {
            Self::Inverted(Box::new(plain))
        } else {
            plain
        }
    }

    /// The pattern; inverting twice gives the base back.
    fn pattern(self) -> Pattern {
        let base = match self {
            Self::Inverted(inner) => return inner.pattern().inverted(),
            Self::Black => Base::Black,
            Self::White => Base::White,
            Self::Dither25 => Base::Dither25,
            Self::Dither50 => Base::Dither50,
            Self::Dither75 => Base::Dither75,
            Self::DiagonalUp => Base::DiagonalUp,
            Self::DiagonalDown => Base::DiagonalDown,
            Self::HLines => Base::HLines,
            Self::VLines => Base::VLines,
            Self::Checker2 => Base::Checker2,
        };
        Pattern::new(base)
    }
}

impl Serialize for Pattern {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        Spelled::of(*self).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Pattern {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Spelled::deserialize(deserializer).map(Spelled::pattern)
    }
}

/// A role's two patterns: `fg` for what is drawn in its colour (a glyph, a
/// rectangle, a sprite painted in it), `bg` for a cell's background in it
/// (`fg` again, if the map doesn't say).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RolePatterns {
    /// What is drawn in the role's colour.
    pub fg: Pattern,
    /// A cell's background in the role's colour.
    #[serde(default)]
    pub bg: Option<Pattern>,
}

impl RolePatterns {
    /// `fg`, with `bg` behind.
    pub const fn new(fg: Pattern, bg: Pattern) -> Self {
        Self { fg, bg: Some(bg) }
    }

    /// The background pattern: `bg`, or else `fg`.
    pub fn background(&self) -> Pattern {
        self.bg.unwrap_or(self.fg)
    }
}

impl AssetKind for PatternMapDef {
    const NAME: &'static str = "pattern map";
}

/// A pattern map: each palette role's patterns on a one-bit target, the
/// groups of roles that must stay distinct, and the roles text is drawn
/// in.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PatternMapDef {
    /// Each role's patterns, by the palette's name for it.
    pub roles: BTreeMap<String, RolePatterns>,
    /// Groups of roles that tell things apart (`["team.player",
    /// "team.enemy"]`): no two of a group may have foregrounds that look
    /// alike.
    #[serde(default)]
    pub distinct: Vec<Vec<String>>,
    /// The roles text is drawn in, which should be black on white or white
    /// on black: text is never dithered.
    #[serde(default)]
    pub text: Vec<String>,
}

impl PatternMapDef {
    /// The patterns of `role`, if the map has them.
    pub fn get(&self, role: &str) -> Option<&RolePatterns> {
        self.roles.get(role)
    }

    /// Parses pattern map `source`, attributing errors to `file`. Reports
    /// every distinct group of fewer than two roles or with a role twice
    /// in it. A map is checked against its palette by [`check`].
    pub fn from_source(file: &str, source: &str) -> Result<Self, Vec<ContentError>> {
        let map: Self = parse_ron_implicit_some(file, source).map_err(|e| vec![e])?;
        let mut errors = Vec::new();
        for (i, group) in map.distinct.iter().enumerate() {
            let shown = group.join(", ");
            if group.len() < 2 {
                errors.push(ContentError::new(
                    file,
                    format!("distinct group {} [{shown}] needs two roles or more", i + 1),
                ));
            }
            let mut seen = BTreeSet::new();
            for role in group.iter().filter(|role| !seen.insert(role.as_str())) {
                errors.push(ContentError::new(
                    file,
                    format!("distinct group {} names \"{role}\" twice", i + 1),
                ));
            }
        }
        if errors.is_empty() {
            Ok(map)
        } else {
            Err(errors)
        }
    }
}

/// A pattern map file (`pattern-map`, `.ron`), read as
/// [`PatternMapDef::from_source`] says.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PatternMapRon;

impl Format for PatternMapRon {
    type Asset = PatternMapDef;

    fn id(&self) -> &'static str {
        "pattern-map"
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
    ) -> Result<PatternMapDef, Vec<ContentError>> {
        PatternMapDef::from_source(file.shown(), text(file, bytes)?)
    }
}

/// Where a palette's pattern map is: beside it, its name with `@onebit`
/// (`skin/palette.ron` → `skin/palette@onebit.ron`).
pub fn map_path(palette: &str) -> String {
    let path = AssetPath::new(palette);
    path.sibling(&format!("{}@onebit.ron", path.stem()))
}

/// How bad a [`Problem`] is.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Severity {
    /// The skin loses meaning on a one-bit target: the check fails.
    Error,
    /// It reads, short of what a one-bit target wants.
    Warning,
}

/// Which rule a [`Problem`] breaks.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Rule {
    /// A role of the palette has no patterns.
    Missing,
    /// The map names a role the palette doesn't have.
    Unknown,
    /// Two roles of one colour have different patterns: the frame holds
    /// colours, so it can't tell them apart.
    SharedColour,
    /// Two roles of a distinct group have foregrounds that look alike.
    Distinct,
    /// A text role isn't black on white or white on black.
    Text,
}

/// What [`check`] found wrong with a pattern map.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Problem {
    /// An error or a warning.
    pub severity: Severity,
    /// The rule it breaks.
    pub rule: Rule,
    /// The roles it is about.
    pub roles: Vec<String>,
    /// What is wrong.
    pub message: String,
}

impl Problem {
    fn error(rule: Rule, roles: Vec<String>, message: String) -> Self {
        Self {
            severity: Severity::Error,
            rule,
            roles,
            message,
        }
    }
}

/// What is wrong with `map` as the one-bit look of `palette` (ADR-0008
/// §4), in this order:
///
/// - errors: a role of the palette with no patterns ([`Rule::Missing`]),
///   a role of the map, a group or the text roles that the palette lacks
///   ([`Rule::Unknown`]), two roles of one colour with different patterns
///   ([`Rule::SharedColour`]), and two roles of a distinct group whose
///   foregrounds look alike ([`Rule::Distinct`], [`Pattern::looks_like`]);
/// - warnings: a text role that isn't `Black` on `White` or `White` on
///   `Black` ([`Rule::Text`]).
pub fn check(palette: &PaletteDef, map: &PatternMapDef) -> Vec<Problem> {
    let mut out = Vec::new();
    for role in palette
        .colors
        .keys()
        .filter(|r| !map.roles.contains_key(*r))
    {
        out.push(Problem::error(
            Rule::Missing,
            vec![role.clone()],
            format!("role \"{role}\" has no patterns"),
        ));
    }
    let named = map.roles.keys().map(|role| (role, "the map"));
    let grouped = map
        .distinct
        .iter()
        .flatten()
        .map(|role| (role, "a distinct group"));
    let text = map.text.iter().map(|role| (role, "the text roles"));
    let mut unknown = BTreeSet::new();
    for (role, place) in named.chain(grouped).chain(text) {
        if !palette.colors.contains_key(role) && unknown.insert((role, place)) {
            out.push(Problem::error(
                Rule::Unknown,
                vec![role.clone()],
                format!("{place} names role \"{role}\", which the palette doesn't have"),
            ));
        }
    }
    let mut by_colour: BTreeMap<[u8; 3], Vec<(&String, &RolePatterns)>> = BTreeMap::new();
    for (role, rgb) in &palette.colors {
        if let Some(patterns) = map.roles.get(role) {
            by_colour.entry(*rgb).or_default().push((role, patterns));
        }
    }
    for (rgb, roles) in &by_colour {
        let Some(&(first, patterns)) = roles.first() else {
            continue;
        };
        let drawn = |p: &RolePatterns| (p.fg, p.background());
        for &(other, theirs) in &roles[1..] {
            if drawn(theirs) != drawn(patterns) {
                out.push(Problem::error(
                    Rule::SharedColour,
                    vec![first.clone(), other.clone()],
                    format!(
                        "roles \"{first}\" and \"{other}\" are both {} in the palette, so the \
                         frame can't tell them apart, but their patterns differ: give them the \
                         same patterns or different colours",
                        format_hex(*rgb)
                    ),
                ));
            }
        }
    }
    for group in &map.distinct {
        let known: Vec<(&String, Pattern)> = group
            .iter()
            .filter_map(|role| map.roles.get(role).map(|p| (role, p.fg)))
            .collect();
        for (i, &(a, pa)) in known.iter().enumerate() {
            for &(b, pb) in &known[i + 1..] {
                if pa.looks_like(pb) {
                    let how = if pa == pb {
                        format!("both {pa}")
                    } else {
                        format!("{pa} and {pb} are the same pattern moved")
                    };
                    out.push(Problem::error(
                        Rule::Distinct,
                        vec![a.clone(), b.clone()],
                        format!(
                            "roles \"{a}\" and \"{b}\" must stay distinct, but their foregrounds \
                             look the same in one bit ({how})"
                        ),
                    ));
                }
            }
        }
    }
    let solid_pair = |p: &RolePatterns| {
        let pair = (p.fg, p.background());
        pair == (Pattern::BLACK, Pattern::WHITE) || pair == (Pattern::WHITE, Pattern::BLACK)
    };
    for role in &map.text {
        if let Some(patterns) = map.roles.get(role).filter(|p| !solid_pair(p)) {
            out.push(Problem {
                severity: Severity::Warning,
                rule: Rule::Text,
                roles: vec![role.clone()],
                message: format!(
                    "text role \"{role}\" is {} on {}: text reads best Black on White or White \
                     on Black",
                    patterns.fg,
                    patterns.background()
                ),
            });
        }
    }
    out
}

#[cfg(test)]
mod tests;
