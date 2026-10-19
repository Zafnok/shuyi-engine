//! Colors: [`Rgb`] values and the [`Palette`] of named colors (game ADR-0012).
//!
//! Code refers to colors by name, never raw RGB. The colors a game's code
//! needs are its [`ColorSet`], an enum whose every name the palette must
//! have, so a missing name is impossible at render time; the engine's own
//! widgets ask for the few [`EngineColor`]s, which the game's set names.
//! Data-driven names (terrain) go through [`Palette::lookup`] and are
//! validated when their content is loaded.

use crate::prelude::*;
use alloc::collections::BTreeMap;
use core::any::TypeId;

use shuyi_assets::palette::PaletteDef;

/// A 24-bit RGB colour.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
pub struct Rgb {
    /// Red channel.
    pub r: u8,
    /// Green channel.
    pub g: u8,
    /// Blue channel.
    pub b: u8,
}

impl Rgb {
    /// Builds a colour from its channels.
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    /// Linear interpolation from `self` (`t = 0`) to `other` (`t = 1`).
    /// `t` is clamped to `0..=1`; NaN counts as 0.
    #[must_use]
    pub fn lerp(self, other: Rgb, t: f32) -> Rgb {
        let t = if t.is_nan() { 0.0 } else { t.clamp(0.0, 1.0) };
        let mix = |a: u8, b: u8| to_channel(f32::from(a) + (f32::from(b) - f32::from(a)) * t);
        Rgb::new(
            mix(self.r, other.r),
            mix(self.g, other.g),
            mix(self.b, other.b),
        )
    }

    /// Multiplies every channel by `factor` (e.g. `0.5` dims by half),
    /// saturating at 0 and 255. NaN counts as 0.
    #[must_use]
    pub fn scale(self, factor: f32) -> Rgb {
        let f = if factor.is_nan() { 0.0 } else { factor };
        let s = |c: u8| to_channel(f32::from(c) * f);
        Rgb::new(s(self.r), s(self.g), s(self.b))
    }

    /// This color grey and darker (a game's unit that has acted, painted
    /// [`Paint::Dimmed`](super::Paint::Dimmed)): [`DIM_GREY`] of the way to
    /// its own grey, then [`DIM_DARK`] as bright. A change of brightness,
    /// not of hue.
    #[must_use]
    pub fn dimmed(self) -> Rgb {
        let [r, g, b] = [self.r, self.g, self.b].map(f32::from);
        let grey = 0.299 * r + 0.587 * g + 0.114 * b;
        let dim = |c: f32| to_channel((c + (grey - c) * DIM_GREY) * DIM_DARK);
        Rgb::new(dim(r), dim(g), dim(b))
    }

    /// Lowercase `#rrggbb`.
    pub fn to_hex(self) -> String {
        shuyi_assets::palette::format_hex([self.r, self.g, self.b])
    }
}

impl From<[u8; 3]> for Rgb {
    fn from([r, g, b]: [u8; 3]) -> Self {
        Self::new(r, g, b)
    }
}

/// How far a [dimmed](Rgb::dimmed) colour goes to its own grey (`0` = not
/// at all, `1` = grey). *Tunable.*
pub const DIM_GREY: f32 = 0.75;

/// How bright a [dimmed](Rgb::dimmed) colour is left (`1` = as bright as it
/// was). *Tunable*: the first renders used 0.6; Nick, seeing it in the game
/// on 2026-10-03, chose to lighten it a bit (ticket 0436).
pub const DIM_DARK: f32 = 0.75;

/// Rounds and saturates a channel value to `0..=255`.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // clamped first
pub fn to_channel(v: f32) -> u8 {
    v.round().clamp(0.0, 255.0) as u8
}

/// The colors the engine's own widgets ask for. A game's [`ColorSet`] says
/// which of its palette's names each one is
/// ([`engine_name`](ColorSet::engine_name)).
#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
pub enum EngineColor {
    /// Text.
    Text,
    /// Text that matters less: hints, what can't be picked.
    TextDim,
    /// A panel's background.
    Panel,
    /// A panel's border.
    Border,
    /// What the player has picked or is on.
    Highlight,
    /// Something going wrong.
    Warning,
    /// The border of what has the focus, and the bar of a focused item.
    Focus,
    /// Behind everything: the screen around the panels.
    Background,
    /// What stands apart from the rest of a highlight: the keys that move
    /// the cursor, say.
    Accent,
    /// Brighter than text: what just changed.
    Bright,
}

impl EngineColor {
    /// Every engine color.
    pub const ALL: &[EngineColor] = &[
        EngineColor::Text,
        EngineColor::TextDim,
        EngineColor::Panel,
        EngineColor::Border,
        EngineColor::Highlight,
        EngineColor::Warning,
        EngineColor::Focus,
        EngineColor::Background,
        EngineColor::Accent,
        EngineColor::Bright,
    ];

    /// Its name, which is the palette's name for it unless the game's
    /// [`ColorSet::engine_name`] says otherwise.
    pub const fn name(self) -> &'static str {
        match self {
            EngineColor::Text => "text",
            EngineColor::TextDim => "text_dim",
            EngineColor::Panel => "panel",
            EngineColor::Border => "border",
            EngineColor::Highlight => "highlight",
            EngineColor::Warning => "warning",
            EngineColor::Focus => "focus",
            EngineColor::Background => "background",
            EngineColor::Accent => "accent",
            EngineColor::Bright => "bright",
        }
    }
}

/// The colors a game's code draws with, by name: usually an enum, whose
/// every name its palette must have.
pub trait ColorSet: Copy + 'static {
    /// Every color of the set, each at its [`index`](Self::index).
    const ALL: &'static [Self];

    /// The color's name in the palette.
    fn name(self) -> &'static str;

    /// Where the color is in [`ALL`](Self::ALL).
    fn index(self) -> usize;

    /// The palette's name for an engine color. By default its own
    /// ([`EngineColor::name`]), so the palette must have `text`, `panel`
    /// and the rest; a game whose palette calls them otherwise says so here.
    fn engine_name(color: EngineColor) -> &'static str {
        color.name()
    }
}

/// The engine's own colors as a set: for a palette with nothing more than
/// the engine's names.
impl ColorSet for EngineColor {
    const ALL: &'static [Self] = EngineColor::ALL;

    fn name(self) -> &'static str {
        EngineColor::name(self)
    }

    fn index(self) -> usize {
        self as usize
    }
}

/// Every name a palette for the set `C` must have: the set's, in its
/// order, then the engine colors' that it doesn't already name.
pub fn required_names<C: ColorSet>() -> Vec<&'static str> {
    let mut names: Vec<&'static str> = C::ALL.iter().map(|c| c.name()).collect();
    for &e in EngineColor::ALL {
        let name = C::engine_name(e);
        if !names.contains(&name) {
            names.push(name);
        }
    }
    names
}

/// Named colors, built from the validated [`PaletteDef`] for a game's
/// [`ColorSet`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Palette {
    /// The set it was built for.
    set: TypeId,
    /// One entry per color of the set, by its [`ColorSet::index`].
    colors: Vec<Rgb>,
    /// One entry per [`EngineColor`], in [`EngineColor::ALL`] order.
    engine: Vec<Rgb>,
    /// [`required_names`] of the set: the names [`name_of`](Self::name_of)
    /// prefers, in order.
    required: Vec<&'static str>,
    /// Every color in the file, by name.
    named: BTreeMap<String, Rgb>,
}

impl Palette {
    /// Builds the palette for the colors of `C`. Fails with every one of
    /// [`required_names`] the definition lacks (a palette checked against
    /// them when it loaded never does).
    pub fn new<C: ColorSet>(def: &PaletteDef) -> Result<Self, Vec<&'static str>> {
        let named: BTreeMap<String, Rgb> = def
            .colors
            .iter()
            .map(|(name, &rgb)| (name.clone(), Rgb::from(rgb)))
            .collect();
        let required = required_names::<C>();
        let missing: Vec<&'static str> = required
            .iter()
            .copied()
            .filter(|n| !named.contains_key(*n))
            .collect();
        if !missing.is_empty() {
            return Err(missing);
        }
        let colors = C::ALL.iter().map(|c| named[c.name()]).collect();
        let engine = EngineColor::ALL
            .iter()
            .map(|&e| named[C::engine_name(e)])
            .collect();
        Ok(Self {
            set: TypeId::of::<C>(),
            colors,
            engine,
            required,
            named,
        })
    }

    /// The RGB value of a color of the set the palette was built for. A
    /// color of another set is looked up by its name (black if there is
    /// no such name).
    pub fn get<C: ColorSet>(&self, color: C) -> Rgb {
        if TypeId::of::<C>() == self.set {
            self.colors[color.index()]
        } else {
            self.lookup(color.name()).unwrap_or(Rgb::new(0, 0, 0))
        }
    }

    /// The RGB value of an engine color.
    pub fn engine(&self, color: EngineColor) -> Rgb {
        self.engine[color as usize]
    }

    /// Looks up a data-driven color name (e.g. terrain).
    pub fn lookup(&self, name: &str) -> Option<Rgb> {
        self.named.get(name).copied()
    }

    /// Every named color, alphabetically.
    pub fn iter(&self) -> impl Iterator<Item = (&str, Rgb)> {
        self.named.iter().map(|(n, &c)| (n.as_str(), c))
    }

    /// The name of a color with exactly this RGB value, if any. When several
    /// names share a value, the set's names win (in [`required_names`]
    /// order), then the alphabetically first other name.
    pub fn name_of(&self, rgb: Rgb) -> Option<&str> {
        self.required
            .iter()
            .copied()
            .find(|&n| self.lookup(n) == Some(rgb))
            .or_else(|| {
                self.named
                    .iter()
                    .find(|&(_, &v)| v == rgb)
                    .map(|(n, _)| n.as_str())
            })
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// A palette file of `pairs`.
    pub(crate) fn def(pairs: &[(&str, [u8; 3])]) -> PaletteDef {
        PaletteDef {
            colors: pairs.iter().map(|&(n, c)| (n.to_owned(), c)).collect(),
        }
    }

    /// A palette with the engine's names and a little more: what the
    /// engine's own tests draw with.
    pub(crate) fn engine_palette() -> Palette {
        let def = def(&[
            ("text", [230, 230, 230]),
            ("text_dim", [128, 128, 128]),
            ("panel", [20, 24, 40]),
            ("border", [90, 110, 160]),
            ("highlight", [250, 210, 90]),
            ("warning", [220, 60, 50]),
            ("focus", [120, 150, 220]),
            ("background", [0, 0, 0]),
            ("accent", [80, 200, 240]),
            ("bright", [255, 255, 255]),
            ("grass", [40, 120, 50]),
        ]);
        Palette::new::<EngineColor>(&def).unwrap()
    }

    /// A game's colors, whose palette calls two engine colors otherwise.
    #[derive(Copy, Clone, PartialEq, Eq, Debug)]
    enum Toy {
        Ink,
        Hero,
        Paper,
    }

    impl ColorSet for Toy {
        const ALL: &'static [Self] = &[Toy::Ink, Toy::Hero, Toy::Paper];

        fn name(self) -> &'static str {
            match self {
                Toy::Ink => "ink",
                Toy::Hero => "hero",
                Toy::Paper => "paper",
            }
        }

        fn index(self) -> usize {
            self as usize
        }

        fn engine_name(color: EngineColor) -> &'static str {
            match color {
                EngineColor::Text => "ink",
                EngineColor::Panel => "paper",
                other => other.name(),
            }
        }
    }

    fn toy_def() -> PaletteDef {
        def(&[
            ("ink", [1, 1, 1]),
            ("hero", [2, 2, 2]),
            ("paper", [3, 3, 3]),
            ("text_dim", [4, 4, 4]),
            ("border", [5, 5, 5]),
            ("highlight", [6, 6, 6]),
            ("warning", [7, 7, 7]),
            ("text", [8, 8, 8]),
            ("focus", [10, 10, 10]),
            ("background", [11, 11, 11]),
            ("accent", [12, 12, 12]),
            ("bright", [13, 13, 13]),
        ])
    }

    #[test]
    fn engine_color_names_and_indices() {
        let names: Vec<&str> = EngineColor::ALL.iter().map(|c| c.name()).collect();
        assert_eq!(
            names,
            [
                "text",
                "text_dim",
                "panel",
                "border",
                "highlight",
                "warning",
                "focus",
                "background",
                "accent",
                "bright"
            ]
        );
        for (i, &c) in EngineColor::ALL.iter().enumerate() {
            assert_eq!(ColorSet::index(c), i);
            assert_eq!(ColorSet::name(c), c.name());
            assert_eq!(<EngineColor as ColorSet>::engine_name(c), c.name());
        }
        assert_eq!(<EngineColor as ColorSet>::ALL, EngineColor::ALL);
    }

    #[test]
    fn required_names_are_the_set_then_the_engine_names_it_lacks() {
        assert_eq!(
            required_names::<EngineColor>(),
            [
                "text",
                "text_dim",
                "panel",
                "border",
                "highlight",
                "warning",
                "focus",
                "background",
                "accent",
                "bright"
            ]
        );
        assert_eq!(
            required_names::<Toy>(),
            [
                "ink",
                "hero",
                "paper",
                "text_dim",
                "border",
                "highlight",
                "warning",
                "focus",
                "background",
                "accent",
                "bright"
            ]
        );
    }

    #[test]
    fn a_palette_gives_the_sets_colors_and_the_engines() {
        let p = Palette::new::<Toy>(&toy_def()).unwrap();
        assert_eq!(p.get(Toy::Ink), Rgb::new(1, 1, 1));
        assert_eq!(p.get(Toy::Hero), Rgb::new(2, 2, 2));
        assert_eq!(p.get(Toy::Paper), Rgb::new(3, 3, 3));
        // The game named these two otherwise.
        assert_eq!(p.engine(EngineColor::Text), Rgb::new(1, 1, 1));
        assert_eq!(p.engine(EngineColor::Panel), Rgb::new(3, 3, 3));
        assert_eq!(p.engine(EngineColor::TextDim), Rgb::new(4, 4, 4));
        assert_eq!(p.engine(EngineColor::Warning), Rgb::new(7, 7, 7));
        // A color of another set goes by its name.
        assert_eq!(p.get(EngineColor::Text), Rgb::new(8, 8, 8));
        assert_eq!(p.get(EngineColor::Panel), Rgb::new(0, 0, 0));
        let e = engine_palette();
        assert_eq!(e.get(EngineColor::Highlight), Rgb::new(250, 210, 90));
        assert_eq!(e.engine(EngineColor::Highlight), Rgb::new(250, 210, 90));
        assert_eq!(e.get(Toy::Hero), Rgb::new(0, 0, 0));
    }

    #[test]
    fn a_palette_lacking_names_says_which() {
        assert_eq!(
            Palette::new::<Toy>(&PaletteDef::default()),
            Err(required_names::<Toy>())
        );
        let mut d = toy_def();
        d.colors.remove("hero");
        d.colors.remove("warning");
        // `text` isn't needed: the game paints engine text in `ink`.
        d.colors.remove("text");
        assert_eq!(Palette::new::<Toy>(&d), Err(vec!["hero", "warning"]));
    }

    #[test]
    fn lookup_and_iter() {
        let p = engine_palette();
        assert_eq!(p.lookup("grass"), Some(Rgb::new(40, 120, 50)));
        assert_eq!(p.lookup("no_such_colour"), None);
        let all: Vec<(&str, Rgb)> = p.iter().collect();
        assert_eq!(all.len(), 11);
        assert!(all.windows(2).all(|w| w[0].0 < w[1].0));
        assert!(all.iter().all(|&(n, c)| p.lookup(n) == Some(c)));
    }

    #[test]
    fn name_of_prefers_the_sets_names_then_alphabetical() {
        let mut d = toy_def();
        for (name, rgb) in [
            ("zeta", [1, 2, 3]),
            ("alpha", [1, 2, 3]),
            ("aaa", [2, 2, 2]),
            ("text", [9, 9, 9]),
            ("border", [1, 1, 1]),
        ] {
            d.colors.insert(name.to_owned(), rgb);
        }
        let p = Palette::new::<Toy>(&d).unwrap();
        assert_eq!(p.name_of(Rgb::new(2, 2, 2)), Some("hero"));
        // The set's own name before an engine name it doesn't use.
        assert_eq!(p.name_of(Rgb::new(1, 1, 1)), Some("ink"));
        assert_eq!(p.name_of(Rgb::new(1, 2, 3)), Some("alpha"));
        assert_eq!(p.name_of(Rgb::new(9, 9, 9)), Some("text"));
        assert_eq!(p.name_of(Rgb::new(7, 7, 7)), Some("warning"));
        assert_eq!(p.name_of(Rgb::new(70, 7, 7)), None);
    }

    #[test]
    fn a_dimmed_colour_is_greyer_and_darker() {
        // A grey stays grey, three quarters as bright.
        assert_eq!(Rgb::new(200, 200, 200).dimmed(), Rgb::new(150, 150, 150));
        assert_eq!(Rgb::new(0, 0, 0).dimmed(), Rgb::new(0, 0, 0));
        assert_eq!(Rgb::new(255, 255, 255).dimmed(), Rgb::new(191, 191, 191));
        // Red: its grey is 76.245; 255 goes three quarters of the way
        // there (120.93), the others up to 57.18; then 0.75 of each.
        assert_eq!(Rgb::new(255, 0, 0).dimmed(), Rgb::new(91, 43, 43));
        // Green is the brightest to the eye, blue the darkest.
        assert_eq!(Rgb::new(0, 255, 0).dimmed(), Rgb::new(84, 132, 84));
        assert_eq!(Rgb::new(0, 0, 255).dimmed(), Rgb::new(16, 16, 64));
    }

    #[test]
    fn lerp_endpoints_and_midpoint() {
        let a = Rgb::new(0, 100, 255);
        let b = Rgb::new(200, 0, 55);
        assert_eq!(a.lerp(b, 0.0), a);
        assert_eq!(a.lerp(b, 1.0), b);
        assert_eq!(a.lerp(b, 0.5), Rgb::new(100, 50, 155));
        assert_eq!(a.lerp(b, 0.25), Rgb::new(50, 75, 205));
    }

    #[test]
    fn lerp_clamps_t() {
        let a = Rgb::new(10, 20, 30);
        let b = Rgb::new(40, 50, 60);
        assert_eq!(a.lerp(b, -3.0), a);
        assert_eq!(a.lerp(b, 7.0), b);
        assert_eq!(a.lerp(b, f32::NAN), a);
    }

    #[test]
    fn scale_dims_and_saturates() {
        let c = Rgb::new(100, 200, 51);
        assert_eq!(c.scale(0.5), Rgb::new(50, 100, 26));
        assert_eq!(c.scale(1.0), c);
        assert_eq!(c.scale(2.0), Rgb::new(200, 255, 102));
        assert_eq!(c.scale(-1.0), Rgb::new(0, 0, 0));
        assert_eq!(c.scale(f32::NAN), Rgb::new(0, 0, 0));
    }

    #[test]
    fn to_hex_and_from_array() {
        assert_eq!(Rgb::from([1, 0xab, 255]).to_hex(), "#01abff");
    }
}
