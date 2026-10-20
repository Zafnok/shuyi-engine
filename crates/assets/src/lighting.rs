//! Lights: named recipes for shading pictures, read from a RON file of
//! `name: (grey: 0.35, tint: (0.62, 0.56, 0.46))` entries. Look data only:
//! a game's rules never see it. As a format (ADR-0007): [`LightingRon`].
use crate::prelude::*;

use alloc::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::error::ContentError;
use crate::format::{AssetKind, AssetPath, DecodeCx, Format, TEXT_MAX_BYTES, text};
use crate::grid_map::DEFAULT_LIGHT;
use crate::ron::parse_ron;

impl AssetKind for Lighting {
    const NAME: &'static str = "lights";
}

/// A lights' file (`lights`, `.ron`), checked as [`from_source`] says,
/// with the light a map that names none is in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LightingRon {
    default: String,
}

impl LightingRon {
    /// The lights of a game whose maps are in `default` unless they say.
    pub fn new(default: impl Into<String>) -> Self {
        Self {
            default: default.into(),
        }
    }
}

impl Default for LightingRon {
    /// Maps are in [`DEFAULT_LIGHT`] unless they say.
    fn default() -> Self {
        Self::new(DEFAULT_LIGHT)
    }
}

impl Format for LightingRon {
    type Asset = Lighting;

    fn id(&self) -> &'static str {
        "lights"
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
    ) -> Result<Lighting, Vec<ContentError>> {
        from_source(file.shown(), text(file, bytes)?, &self.default)
    }
}

/// How many parts a [`Light`]'s numbers are counted in: the file's `0.35`
/// is kept as 350.
const PARTS: u16 = 1000;

/// One light: how a picture's colours change in it. Each pixel goes
/// `grey` of the way to its own grey, then its red, green and blue are
/// multiplied by `tint`. The numbers are thousandths (0 to 1000), so a
/// light is a plain value a frame item can carry and compare.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Light {
    /// How far a colour goes to its own grey: 0 = not at all, 1000 = grey.
    pub grey: u16,
    /// What red, green and blue are multiplied by: 1000 = left as they
    /// are.
    pub tint: [u16; 3],
}

impl Light {
    /// The light that changes nothing.
    pub const PLAIN: Self = Self {
        grey: 0,
        tint: [PARTS; 3],
    };

    /// The light of `grey` and `tint` as the file writes them (0 to 1),
    /// to the nearest thousandth; what is outside that range is cut to it.
    pub fn new(grey: f32, tint: [f32; 3]) -> Self {
        Self {
            grey: parts(grey),
            tint: tint.map(parts),
        }
    }

    /// Whether it changes no colour.
    pub fn is_plain(self) -> bool {
        self == Self::PLAIN
    }

    /// The colour `rgb` (red, green, blue) in this light. The one place
    /// that knows the sum: every renderer shades with it.
    pub fn apply(self, rgb: [u8; 3]) -> [u8; 3] {
        let [r, g, b] = rgb.map(f32::from);
        let own_grey = 0.299 * r + 0.587 * g + 0.114 * b;
        let grey = fraction(self.grey);
        let shade = |c: f32, tint: u16| to_channel((c + (own_grey - c) * grey) * fraction(tint));
        [
            shade(r, self.tint[0]),
            shade(g, self.tint[1]),
            shade(b, self.tint[2]),
        ]
    }
}

impl Default for Light {
    fn default() -> Self {
        Self::PLAIN
    }
}

/// `v` (0 to 1) in thousandths. NaN counts as 0.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // clamped first
fn parts(v: f32) -> u16 {
    libm::roundf(v * f32::from(PARTS)).clamp(0.0, f32::from(PARTS)) as u16
}

/// Thousandths as a fraction.
fn fraction(parts: u16) -> f32 {
    f32::from(parts) / f32::from(PARTS)
}

/// Rounds and saturates a channel value to `0..=255`.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // clamped first
fn to_channel(v: f32) -> u8 {
    libm::roundf(v).clamp(0.0, 255.0) as u8
}

/// Every light, by name.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Lighting {
    /// The lights by name.
    pub lights: BTreeMap<String, Light>,
}

impl Lighting {
    /// The light called `name`.
    pub fn get(&self, name: &str) -> Option<Light> {
        self.lights.get(name).copied()
    }

    /// Every light's name in double quotes, in order, for an error
    /// message.
    pub fn names(&self) -> String {
        let quoted: Vec<String> = self.lights.keys().map(|l| format!("\"{l}\"")).collect();
        quoted.join(", ")
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawLight {
    grey: f32,
    tint: (f32, f32, f32),
}

/// Parses lights' file `source`, attributing errors to `file`. Every
/// number must be from 0 to 1, and there must be a light called `default`:
/// the light of a map that names none. Reports every problem found.
pub fn from_source(file: &str, source: &str, default: &str) -> Result<Lighting, Vec<ContentError>> {
    let raw: BTreeMap<String, RawLight> = parse_ron(file, source).map_err(|e| vec![e])?;
    let mut errors = Vec::new();
    let mut lights = BTreeMap::new();
    for (name, light) in raw {
        let (r, g, b) = light.tint;
        let numbers = [
            ("grey", light.grey),
            ("tint red", r),
            ("tint green", g),
            ("tint blue", b),
        ];
        for (what, v) in numbers {
            if !(0.0..=1.0).contains(&v) {
                let message = format!("light \"{name}\": {what} is {v}; it must be from 0 to 1");
                errors.push(ContentError::new(file, message));
            }
        }
        lights.insert(name, Light::new(light.grey, [r, g, b]));
    }
    if !lights.contains_key(default) {
        let message = format!("no light \"{default}\": it is the light of a map that names none");
        errors.push(ContentError::new(file, message));
    }
    if errors.is_empty() {
        Ok(Lighting { lights })
    } else {
        Err(errors)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A dusk light: 35% to grey, then × 0.62, 0.56, 0.46.
    const C3: Light = Light {
        grey: 350,
        tint: [620, 560, 460],
    };

    #[test]
    fn c3s_light_on_three_colours() {
        // A grass green: its grey is 135.5; red 96 → 109.8 → 68.1, green
        // 168 → 156.6 → 87.7, blue 72 → 94.2 → 43.3.
        assert_eq!(C3.apply([96, 168, 72]), [68, 88, 43]);
        // A water blue: its grey is 127.4.
        assert_eq!(C3.apply([64, 144, 208]), [53, 77, 83]);
        // A sand: its grey is 195.3.
        assert_eq!(C3.apply([224, 200, 96]), [133, 111, 60]);
        // White keeps the tint; black stays black.
        assert_eq!(C3.apply([255, 255, 255]), [158, 143, 117]);
        assert_eq!(C3.apply([0, 0, 0]), [0, 0, 0]);
    }

    #[test]
    fn grey_and_tint_each_do_their_part() {
        let grey = Light {
            grey: 1000,
            ..Light::PLAIN
        };
        // All the way to its own grey: 0.299 × 255.
        assert_eq!(grey.apply([255, 0, 0]), [76, 76, 76]);
        let half = Light {
            grey: 500,
            ..Light::PLAIN
        };
        assert_eq!(half.apply([255, 0, 0]), [166, 38, 38]);
        let tint = Light {
            grey: 0,
            tint: [1000, 500, 0],
        };
        assert_eq!(tint.apply([200, 200, 200]), [200, 100, 0]);
    }

    #[test]
    fn the_plain_light_changes_nothing() {
        assert!(Light::PLAIN.is_plain());
        assert!(Light::default().is_plain());
        assert!(!C3.is_plain());
        for rgb in [[0, 0, 0], [255, 255, 255], [1, 2, 3], [96, 168, 72]] {
            assert_eq!(Light::PLAIN.apply(rgb), rgb);
        }
    }

    #[test]
    fn a_light_keeps_thousandths() {
        assert_eq!(Light::new(0.35, [0.62, 0.56, 0.46]), C3);
        assert_eq!(Light::new(0.0, [1.0; 3]), Light::PLAIN);
        assert_eq!(Light::new(0.0004, [0.9996, 0.5, 0.5]).grey, 0);
        assert_eq!(Light::new(0.0006, [0.9996, 0.5, 0.5]).tint[0], 1000);
        // Outside 0 to 1 is cut; NaN is 0.
        let cut = Light::new(-1.0, [2.0, f32::NAN, 0.25]);
        assert_eq!((cut.grey, cut.tint), (0, [1000, 0, 250]));
    }

    const VALID: &str = r#"{
        "noon": (grey: 0.0, tint: (1.0, 1.0, 1.0)),
        "dusk": (grey: 0.35, tint: (0.62, 0.56, 0.46)),
    }"#;

    #[test]
    fn a_valid_file_loads() {
        let lighting = from_source("l.ron", VALID, "noon").unwrap();
        assert_eq!(lighting.lights.len(), 2);
        assert_eq!(lighting.get("noon"), Some(Light::PLAIN));
        assert_eq!(lighting.get("dusk"), Some(C3));
        assert_eq!(lighting.get("night"), None);
        assert_eq!(lighting.names(), "\"dusk\", \"noon\"");
    }

    fn messages(source: &str) -> Vec<String> {
        let errors = from_source("l.ron", source, "noon")
            .err()
            .unwrap_or_default();
        errors
            .into_iter()
            .map(|e| {
                assert_eq!(e.file, "l.ron");
                e.message
            })
            .collect()
    }

    #[test]
    fn every_number_must_be_from_0_to_1() {
        let source = r#"{
            "noon": (grey: 0.0, tint: (1.0, 1.0, 1.0)),
            "odd": (grey: 1.5, tint: (-0.1, 1.001, 2.0)),
        }"#;
        assert_eq!(
            messages(source),
            [
                "light \"odd\": grey is 1.5; it must be from 0 to 1",
                "light \"odd\": tint red is -0.1; it must be from 0 to 1",
                "light \"odd\": tint green is 1.001; it must be from 0 to 1",
                "light \"odd\": tint blue is 2; it must be from 0 to 1",
            ]
        );
        // The ends are in.
        let ends = r#"{"noon": (grey: 1.0, tint: (0.0, 0.0, 1.0))}"#;
        assert_eq!(messages(ends), [""; 0]);
    }

    #[test]
    fn the_default_light_must_exist() {
        let source = r#"{"dusk": (grey: 0.35, tint: (0.62, 0.56, 0.46))}"#;
        assert_eq!(
            messages(source),
            ["no light \"noon\": it is the light of a map that names none"]
        );
        assert_eq!(messages("{}").len(), 1);
        // The default is whichever the game names.
        assert!(from_source("l.ron", source, "dusk").is_ok());
    }

    #[test]
    fn a_broken_file_is_reported_where_it_breaks() {
        let errors = from_source("l.ron", "{\n \"noon\": (grey: 0.0),\n}", "noon")
            .err()
            .unwrap_or_default();
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].line, Some(2));
        // A field a light doesn't have.
        let extra = VALID.replace("grey: 0.0,", "grey: 0.0, glow: 1.0,");
        assert!(from_source("l.ron", &extra, "noon").is_err());
    }
}
