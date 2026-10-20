//! Export targets as data (ADR-0008 §1): a [`TargetProfile`] says what a
//! target's display shows, how fast it runs, what the player can press,
//! what a game may spend there, and which platform backend builds it. The
//! engine's profiles are the files of `targets/` at the repository's root
//! (`playdate.ron`, `steam-deck.ron`, …); a game adds one, or overrides
//! one by its id, with a file of its own. As a format (ADR-0007):
//! [`TargetRon`].
//!
//! Every unit is an integer: pixels are `u16`, milliseconds `u32`, bytes
//! and instructions `u64`.
use crate::prelude::*;

use serde::Deserialize;

use crate::error::ContentError;
use crate::format::{AssetKind, AssetPath, DecodeCx, Format, TEXT_MAX_BYTES, text};
use crate::keymap::Button;
use crate::ron::parse_ron_implicit_some;

impl AssetKind for TargetProfile {
    const NAME: &'static str = "target profile";
}

/// A target profile file (`target-ron`, `.ron`), read and checked as
/// [`TargetProfile::from_source`] says.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TargetRon;

impl Format for TargetRon {
    type Asset = TargetProfile;

    fn id(&self) -> &'static str {
        "target-ron"
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
    ) -> Result<TargetProfile, Vec<ContentError>> {
        TargetProfile::from_source(file.shown(), file.stem(), text(file, bytes)?)
    }
}

/// One export target.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TargetProfile {
    /// Its id, which `shuyi check --target` and a game's `shuyi.ron` name
    /// it by: the file's name without `.ron`.
    pub id: String,
    /// Whether the engine runs on it yet.
    pub status: Status,
    /// Its screen.
    pub display: Display,
    /// The frames a second a game holds there.
    pub frame_rate: FrameRate,
    /// What the player can press.
    pub input: Input,
    /// What a game may spend there.
    #[serde(default)]
    pub budgets: Budgets,
    /// The platform backend that builds it: a crate (`shuyi-app`,
    /// `shuyi-playdate`), or `private` for a console's.
    pub backend: String,
}

/// Whether the engine runs on a target yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum Status {
    /// The target's example runs on the device.
    Supported,
    /// Not yet.
    Planned,
}

/// A target's screen.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Display {
    /// The sizes it shows, in device pixels: one, or one per mode (the
    /// Switch handheld and docked). Empty for a window or a browser
    /// canvas, which takes any size.
    #[serde(default)]
    pub sizes: Vec<Size>,
    /// How many colours it shows. ADR-0012 moves this into the target's
    /// capabilities (ticket 3017).
    pub color: Color,
    /// Text shorter than this, in device pixels, is an error.
    pub min_text_px: u16,
    /// Text shorter than this, in device pixels, is a warning.
    pub good_text_px: u16,
    /// The edges a game may not draw what matters under.
    #[serde(default)]
    pub safe_area: Insets,
}

/// A screen's size, in device pixels.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Size {
    /// The mode it is the size in (`handheld`, `docked`), if the display
    /// has more than one.
    #[serde(default)]
    pub mode: Option<String>,
    /// Its width.
    pub width: u16,
    /// Its height.
    pub height: u16,
}

/// How many colours a display shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum Color {
    /// Full colour, 8 bits a channel and alpha.
    Rgba8,
    /// Black and white (ADR-0008 §4).
    OneBit,
}

/// Insets from each edge of a display, in device pixels.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct Insets {
    /// From the top.
    pub top: u16,
    /// From the right.
    pub right: u16,
    /// From the bottom.
    pub bottom: u16,
    /// From the left.
    pub left: u16,
}

/// The frames a second a game holds on a target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FrameRate {
    /// What it aims at.
    pub target: u16,
    /// What it never drops below.
    pub floor: u16,
}

/// What the player can press on a target (ADR-0008 §3).
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct Input {
    /// The pad's buttons, by position, as [`Button`] names them. The
    /// system's own (the Playdate's Menu, the Deck's Steam button) are not
    /// listed: they are never the game's.
    pub buttons: Vec<Button>,
    /// Pairs of the buttons pressed together that count as one more input.
    pub chords: Vec<[Button; 2]>,
    /// Analog inputs: `LeftStick`, `RightStick`, `Crank`.
    pub axes: Vec<String>,
    /// The items of the game's own the system menu holds.
    pub system_menu_slots: u8,
    /// Whether a mouse or another pointer exists.
    pub pointer: bool,
    /// Whether a touch screen exists.
    pub touch: bool,
    /// Whether a keyboard exists.
    pub keyboard: bool,
}

/// What a game may spend on a target (ADR-0008 §6). A budget left out is
/// not measured there yet.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct Budgets {
    /// The most instructions one frame of the engine's work may take in
    /// the heaviest benchmark scenario.
    pub frame_instructions: Option<u64>,
    /// The most heap in use at once, in bytes.
    pub peak_heap_bytes: Option<u64>,
    /// The biggest package, in bytes.
    pub package_bytes: Option<u64>,
    /// The longest start-up, in milliseconds.
    pub startup_ms: Option<u32>,
}

impl TargetProfile {
    /// Parses and checks profile `source`, the file `stem.ron` shown as
    /// `file`. Reports every problem: an id that isn't the file's name,
    /// text sizes or frame rates out of order, an empty size, a safe area
    /// that leaves nothing, a chord of buttons the target doesn't have.
    pub fn from_source(file: &str, stem: &str, source: &str) -> Result<Self, Vec<ContentError>> {
        let profile: Self = parse_ron_implicit_some(file, source).map_err(|e| vec![e])?;
        let problems = profile.problems(stem);
        if problems.is_empty() {
            Ok(profile)
        } else {
            Err(problems
                .into_iter()
                .map(|message| ContentError::new(file, message))
                .collect())
        }
    }

    /// What is wrong with this profile, read from the file `stem.ron`.
    fn problems(&self, stem: &str) -> Vec<String> {
        let mut out = Vec::new();
        if self.id != stem {
            out.push(format!(
                "id \"{}\" is not the file's name \"{stem}\"",
                self.id
            ));
        }
        let display = &self.display;
        if display.min_text_px > display.good_text_px {
            out.push(format!(
                "display: min_text_px {} is more than good_text_px {}",
                display.min_text_px, display.good_text_px
            ));
        }
        let insets = display.safe_area;
        for size in &display.sizes {
            let name = size.name();
            if size.width == 0 || size.height == 0 {
                out.push(format!("display: size {name} is empty"));
            } else if u32::from(insets.left) + u32::from(insets.right) >= u32::from(size.width)
                || u32::from(insets.top) + u32::from(insets.bottom) >= u32::from(size.height)
            {
                out.push(format!("display: the safe area leaves nothing of {name}"));
            }
        }
        let rate = self.frame_rate;
        if rate.floor == 0 || rate.floor > rate.target {
            out.push(format!(
                "frame_rate: floor {} must be from 1 to target {}",
                rate.floor, rate.target
            ));
        }
        for chord in &self.input.chords {
            for button in chord {
                if !self.input.buttons.contains(button) {
                    out.push(format!(
                        "input: chord {} + {} uses {button}, which is not in buttons",
                        chord[0], chord[1]
                    ));
                }
            }
        }
        out
    }
}

impl Size {
    /// A size of `width` × `height`, in no mode.
    pub fn new(width: u16, height: u16) -> Self {
        Self {
            mode: None,
            width,
            height,
        }
    }

    /// The size as messages show it: `400 × 240`, or `1920 × 1080
    /// docked`.
    pub fn name(&self) -> String {
        match &self.mode {
            Some(mode) => format!("{} × {} {mode}", self.width, self.height),
            None => format!("{} × {}", self.width, self.height),
        }
    }

    /// What `insets` leave of it, saturating at nothing.
    #[must_use]
    pub fn inside(&self, insets: Insets) -> Self {
        Self {
            mode: self.mode.clone(),
            width: self
                .width
                .saturating_sub(insets.left)
                .saturating_sub(insets.right),
            height: self
                .height
                .saturating_sub(insets.top)
                .saturating_sub(insets.bottom),
        }
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;
    use crate::format::Formats;

    /// A whole profile, with `extra` added to its display.
    fn profile(id: &str, display_extra: &str) -> String {
        format!(
            "(
    id: \"{id}\",
    status: Supported,
    display: (sizes: [(width: 400, height: 240)], color: OneBit, min_text_px: 8, good_text_px: 10, {display_extra}),
    frame_rate: (target: 30, floor: 30),
    input: (buttons: [\"East\", \"West\"], chords: [(\"East\", \"West\")], axes: [\"Crank\"], system_menu_slots: 3),
    budgets: (startup_ms: 5000),
    backend: \"shuyi-playdate\",
)"
        )
    }

    fn read(stem: &str, source: &str) -> Result<TargetProfile, Vec<String>> {
        TargetProfile::from_source("t.ron", stem, source)
            .map_err(|e| e.iter().map(ToString::to_string).collect())
    }

    #[test]
    fn a_profile_reads_with_optional_fields_as_plain_values() {
        let p = read("small", &profile("small", "")).unwrap();
        assert_eq!(p.display.sizes, [Size::new(400, 240)]);
        assert_eq!(p.display.safe_area, Insets::default());
        assert_eq!(p.input.chords, [[Button::East, Button::West]]);
        assert_eq!(p.budgets.startup_ms, Some(5000));
        assert_eq!(p.budgets.frame_instructions, None);
        assert!(!p.input.touch);
    }

    #[test]
    fn unknown_fields_and_buttons_are_refused_with_where() {
        let errors = read("small", &profile("small", "colour: Rgba8")).unwrap_err();
        assert_eq!(errors.len(), 1);
        assert!(errors[0].starts_with("t.ron:4:"), "{errors:?}");
        let source = profile("small", "").replace("\"West\"],", "\"Wst\"],");
        let errors = read("small", &source).unwrap_err();
        assert!(errors[0].contains("unknown button \"Wst\""), "{errors:?}");
    }

    #[test]
    fn every_problem_of_a_profile_is_reported() {
        let source = profile("other", "safe_area: (left: 200, right: 200)")
            .replace("min_text_px: 8", "min_text_px: 12")
            .replace("floor: 30", "floor: 40")
            .replace("[\"East\", \"West\"]", "[\"East\"]")
            .replace(
                "[(width: 400, height: 240)]",
                "[(width: 400, height: 240), (mode: \"tiny\", width: 0, height: 9)]",
            );
        assert_eq!(
            read("small", &source).unwrap_err(),
            [
                "t.ron: id \"other\" is not the file's name \"small\"",
                "t.ron: display: min_text_px 12 is more than good_text_px 10",
                "t.ron: display: the safe area leaves nothing of 400 × 240",
                "t.ron: display: size 0 × 9 tiny is empty",
                "t.ron: frame_rate: floor 40 must be from 1 to target 30",
                "t.ron: input: chord East + West uses West, which is not in buttons",
            ]
        );
    }

    #[test]
    fn a_size_shrinks_by_its_insets_and_shows_its_mode() {
        let insets = Insets {
            top: 10,
            right: 20,
            bottom: 30,
            left: 40,
        };
        assert_eq!(Size::new(400, 240).inside(insets), Size::new(340, 200));
        assert_eq!(Size::new(10, 10).inside(insets), Size::new(0, 0));
        let docked = Size {
            mode: Some("docked".to_owned()),
            ..Size::new(1920, 1080)
        };
        assert_eq!(docked.name(), "1920 × 1080 docked");
    }

    #[test]
    fn the_standard_registry_reads_a_profile_by_its_stem() {
        let formats = Formats::standard();
        assert_eq!(formats.ids::<TargetProfile>(), ["target-ron"]);
        let file = AssetPath::new("targets/small.ron");
        let p = formats.decode::<TargetProfile>(
            &file,
            profile("small", "").as_bytes(),
            &DecodeCx::new(),
        );
        assert_eq!(p.map(|p| p.id), Ok("small".to_owned()));
        assert_eq!(TargetRon.max_bytes(), TEXT_MAX_BYTES);
    }

    proptest! {
        #[test]
        fn no_bytes_panic_the_target_format(
            random in proptest::collection::vec(any::<u8>(), 0..512),
            cut in 0..600_usize,
            (at, with) in (0..600_usize, any::<char>()),
        ) {
            let whole = profile("small", "");
            let truncated = &whole.as_bytes()[..cut.min(whole.len())];
            let mut changed: Vec<char> = whole.chars().collect();
            let i = at.min(changed.len() - 1);
            changed[i] = with;
            let changed: String = changed.into_iter().collect();
            for bytes in [&random[..], truncated, changed.as_bytes()] {
                let _ = TargetRon.decode(&AssetPath::new("small.ron"), bytes, &DecodeCx::new());
            }
        }
    }
}
