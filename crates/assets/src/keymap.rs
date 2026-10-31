//! The keymap file format, generic over a game's [`ActionSet`] (game ADR-0015,
//! game ADR-0031, game ADR-0034, tickets 1106 and 1107): [`Key`]s and [`Chord`]s,
//! controller [`Button`]s and [`ButtonChord`]s (two buttons together), the
//! named [`Layout`]s a file lists, and
//! [`KeymapDef`], a validated keymap file: each layout's default keys, the
//! keys that work before a layout is chosen, the default buttons, key-repeat
//! timings and the sticks' thresholds.
//!
//! Nothing here names one game's actions or layouts: a game gives its
//! actions as an [`ActionSet`] ([`action_set`]) and its file's text. As a
//! format (ADR-0007): [`KeymapRon`].
use crate::prelude::*;

use alloc::collections::{BTreeMap, BTreeSet};
use core::fmt;
use core::marker::PhantomData;
use core::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::error::{ContentError, codes};
use crate::format::{AssetKind, AssetPath, DecodeCx, Format, TEXT_MAX_BYTES, text};
use crate::ron::parse_ron;

pub mod action_set;
#[cfg(any(test, feature = "sample-actions"))]
pub mod sample;
#[cfg(test)]
mod tests;

pub use action_set::{ActionSet, UiAction};

/// Declares [`Key`] with its keymap-file names, keeping the variant list,
/// [`Key::ALL`] and [`Key::name`] in one table.
macro_rules! keys {
    ($($variant:ident => $name:literal,)+) => {
        /// A hardware-agnostic keyboard key. `app` translates platform key
        /// codes to these; anything without a variant is ignored.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        pub enum Key {
            $(
                #[doc = concat!("The `` ", $name, " `` key.")]
                $variant,
            )+
        }

        impl Key {
            /// Every key, in declaration order.
            pub const ALL: &'static [Key] = &[$(Key::$variant),+];

            /// The name used in keymap files: lowercase letters (`h`), digits
            /// (`0`), punctuation as itself (`;`, `,`, `\`), numpad keys
            /// prefixed `Kp` (`Kp0`, `Kp+`), and capitalised names for the
            /// rest (`Left`, `F12`).
            pub fn name(self) -> &'static str {
                match self {
                    $(Key::$variant => $name,)+
                }
            }
        }
    };
}

keys! {
    A => "a",
    B => "b",
    C => "c",
    D => "d",
    E => "e",
    F => "f",
    G => "g",
    H => "h",
    I => "i",
    J => "j",
    K => "k",
    L => "l",
    M => "m",
    N => "n",
    O => "o",
    P => "p",
    Q => "q",
    R => "r",
    S => "s",
    T => "t",
    U => "u",
    V => "v",
    W => "w",
    X => "x",
    Y => "y",
    Z => "z",
    Digit0 => "0",
    Digit1 => "1",
    Digit2 => "2",
    Digit3 => "3",
    Digit4 => "4",
    Digit5 => "5",
    Digit6 => "6",
    Digit7 => "7",
    Digit8 => "8",
    Digit9 => "9",
    Semicolon => ";",
    Up => "Up",
    Down => "Down",
    Left => "Left",
    Right => "Right",
    Enter => "Enter",
    Escape => "Escape",
    Space => "Space",
    Tab => "Tab",
    Backspace => "Backspace",
    F1 => "F1",
    F2 => "F2",
    F3 => "F3",
    F4 => "F4",
    F5 => "F5",
    F6 => "F6",
    F7 => "F7",
    F8 => "F8",
    F9 => "F9",
    F10 => "F10",
    F11 => "F11",
    F12 => "F12",
    Comma => ",",
    Period => ".",
    Slash => "/",
    Apostrophe => "'",
    LeftBracket => "[",
    RightBracket => "]",
    Backslash => "\\",
    Minus => "-",
    Equal => "=",
    Backquote => "`",
    Insert => "Insert",
    Delete => "Delete",
    Home => "Home",
    End => "End",
    PageUp => "PageUp",
    PageDown => "PageDown",
    Kp0 => "Kp0",
    Kp1 => "Kp1",
    Kp2 => "Kp2",
    Kp3 => "Kp3",
    Kp4 => "Kp4",
    Kp5 => "Kp5",
    Kp6 => "Kp6",
    Kp7 => "Kp7",
    Kp8 => "Kp8",
    Kp9 => "Kp9",
    KpAdd => "Kp+",
    KpSubtract => "Kp-",
    KpMultiply => "Kp*",
    KpDivide => "Kp/",
    KpDecimal => "Kp.",
}

impl Key {
    /// Looks a key up by its [`name`](Self::name) (exact match).
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.iter().copied().find(|k| k.name() == name)
    }
}

/// Key slots per action: a layout lists at most this many chords for an
/// action, and the player can give each action up to this many keys
/// (`docs/design/controls.md`, *Rebinding keys*).
pub const SLOTS: usize = 3;

impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// A key plus modifier state, e.g. `h` or `Shift+h`. `Shift+h` and `h` are
/// different chords.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Chord {
    /// The key pressed.
    pub key: Key,
    /// Whether Shift was held.
    pub shift: bool,
}

/// Prefix for shifted chords in keymap files.
const SHIFT_PREFIX: &str = "Shift+";

impl Chord {
    /// A chord without modifiers.
    pub const fn plain(key: Key) -> Self {
        Self { key, shift: false }
    }

    /// A chord with Shift held.
    pub const fn shifted(key: Key) -> Self {
        Self { key, shift: true }
    }

    /// Whether this chord is fixed by the game and can never be bound
    /// (`docs/design/controls.md`, *Rebinding keys*): plain `Escape` always
    /// cancels and plain `Delete` empties a key slot. `Shift+Escape` and
    /// `Shift+Delete` are ordinary chords.
    pub fn is_reserved(self) -> bool {
        !self.shift && matches!(self.key, Key::Escape | Key::Delete)
    }

    /// Parses `"h"`, `"Shift+h"`, `"Left"`, `"Shift+Tab"`, `"F12"`, …
    pub fn parse(s: &str) -> Result<Self, String> {
        let (shift, name) = match s.strip_prefix(SHIFT_PREFIX) {
            Some(rest) => (true, rest),
            None => (false, s),
        };
        match Key::from_name(name) {
            Some(key) => Ok(Self { key, shift }),
            None => Err(unknown_key_message(s, name)),
        }
    }
}

fn unknown_key_message(chord: &str, name: &str) -> String {
    let hint = if name.len() == 1 && name.bytes().all(|b| b.is_ascii_uppercase()) {
        format!(
            " (letters are lowercase; write \"{SHIFT_PREFIX}{}\" for a shifted letter)",
            name.to_ascii_lowercase()
        )
    } else {
        String::new()
    };
    format!(
        "unknown key chord \"{chord}\"{hint}; expected a key such as \"h\", \"0\", \";\", \"Left\", \
         \"Enter\", \"Escape\", \"Space\", \"Tab\", \"Backspace\" or \"F1\", optionally \
         prefixed by \"{SHIFT_PREFIX}\""
    )
}

impl FromStr for Chord {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

impl fmt::Display for Chord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.shift {
            f.write_str(SHIFT_PREFIX)?;
        }
        f.write_str(self.key.name())
    }
}

/// Declares [`Button`] with its docs, keeping the variant list,
/// [`Button::ALL`] and [`Button::name`] in one table.
macro_rules! buttons {
    ($($variant:ident => $doc:literal,)+) => {
        /// A controller button, named by its **position** on an Xbox-shaped
        /// pad, whatever is printed on it (game ADR-0034). Each stick's four
        /// directions count as buttons too, and so does a crank wound one
        /// notch either way (the Playdate's, ticket 1035: pressed and let go
        /// once per notch), and so does an item of the game's own in the
        /// system's menu (pressed and let go when the player picks it,
        /// ADR-0008 §3). `app` translates what the platform reports to
        /// these; anything else on a pad is ignored, and so are the
        /// system's own buttons (the Playdate's Menu and Lock, the Deck's
        /// Steam and Quick Access), which no keymap can name.
        ///
        /// The first 16 are in the order of the browser Gamepad API's
        /// "standard" mapping.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub enum Button {
            $(
                #[doc = $doc]
                $variant,
            )+
        }

        impl Button {
            /// Every button, in declaration order.
            pub const ALL: &'static [Button] = &[$(Button::$variant),+];

            /// The name used in keymap files (the variant name).
            pub fn name(self) -> &'static str {
                match self {
                    $(Button::$variant => stringify!($variant),)+
                }
            }
        }
    };
}

buttons! {
    South => "The bottom face button (Xbox `A`).",
    East => "The right face button (Xbox `B`).",
    West => "The left face button (Xbox `X`).",
    North => "The top face button (Xbox `Y`).",
    LeftShoulder => "The left shoulder button (Xbox `LB`).",
    RightShoulder => "The right shoulder button (Xbox `RB`).",
    LeftTrigger => "The left trigger (Xbox `LT`), pulled far enough to count as pressed.",
    RightTrigger => "The right trigger (Xbox `RT`), pulled far enough to count as pressed.",
    Select => "The left centre button (Xbox `Back`).",
    Start => "The right centre button (Xbox `Start`).",
    LeftStickPress => "Pressing the left stick in.",
    RightStickPress => "Pressing the right stick in.",
    DpadUp => "D-pad up.",
    DpadDown => "D-pad down.",
    DpadLeft => "D-pad left.",
    DpadRight => "D-pad right.",
    LeftStickUp => "The left stick pushed up.",
    LeftStickDown => "The left stick pushed down.",
    LeftStickLeft => "The left stick pushed left.",
    LeftStickRight => "The left stick pushed right.",
    RightStickUp => "The right stick pushed up.",
    RightStickDown => "The right stick pushed down.",
    RightStickLeft => "The right stick pushed left.",
    RightStickRight => "The right stick pushed right.",
    CrankBack => "The crank wound one notch backwards (counterclockwise).",
    CrankForward => "The crank wound one notch forwards (clockwise).",
    SystemMenu0 => "The first item of the game's own in the system's menu.",
    SystemMenu1 => "The second item of the game's own in the system's menu.",
    SystemMenu2 => "The third item of the game's own in the system's menu.",
    SystemMenu3 => "The fourth item of the game's own in the system's menu.",
    SystemMenu4 => "The fifth item of the game's own in the system's menu.",
    SystemMenu5 => "The sixth item of the game's own in the system's menu.",
    SystemMenu6 => "The seventh item of the game's own in the system's menu.",
    SystemMenu7 => "The eighth item of the game's own in the system's menu.",
}

/// The system menu items a keymap can name: [`Button::SystemMenu0`] to
/// [`Button::SystemMenu7`]. A target holds fewer
/// (`shuyi_assets::target::Input::system_menu_slots`).
pub const SYSTEM_MENU_ITEMS: u8 = 8;

impl Button {
    /// The system menu's item `slot` (from 0), if a keymap can name it
    /// (below [`SYSTEM_MENU_ITEMS`]).
    pub fn system_menu(slot: u8) -> Option<Self> {
        let first = Self::ALL.iter().position(|&b| b == Self::SystemMenu0)?;
        Self::ALL.get(first + usize::from(slot)).copied()
    }

    /// Which item of the system's menu this is (from 0), if it is one.
    pub fn system_menu_slot(self) -> Option<u8> {
        (0..SYSTEM_MENU_ITEMS).find(|&slot| Self::system_menu(slot) == Some(self))
    }

    /// Looks a button up by its [`name`](Self::name) (exact match).
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.iter().copied().find(|b| b.name() == name)
    }

    /// Parses a keymap-file button name: `"South"`, `"DpadUp"`,
    /// `"LeftStickLeft"`, …
    pub fn parse(s: &str) -> Result<Self, String> {
        Self::from_name(s).ok_or_else(|| {
            let known: Vec<&str> = Self::ALL.iter().map(|b| b.name()).collect();
            format!(
                "unknown button \"{s}\"; known buttons: {}",
                known.join(", ")
            )
        })
    }
}

impl FromStr for Button {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

/// A button in a data file (a target's inputs, `shuyi_assets::target`) and
/// in a baked keymap (`pack`) is its [`name`](Button::name).
impl Serialize for Button {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.name())
    }
}

impl<'de> Deserialize<'de> for Button {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let name = String::deserialize(deserializer)?;
        Self::parse(&name).map_err(serde::de::Error::custom)
    }
}

impl fmt::Display for Button {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// What a controller binding is: one [`Button`], or two pressed together
/// (a *button chord*, ticket 1034), written `"South+East"` in keymap files
/// and saved bindings. The two are kept in [`Button::ALL`] order, so
/// `"East+South"` is the same chord as `"South+East"` (and is shown as the
/// latter). How a chord fires is the input's business
/// (`shuyi_ui::input::state`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ButtonChord {
    first: Button,
    second: Option<Button>,
}

/// What joins a chord's two buttons in keymap files and saved bindings.
const CHORD_JOIN: char = '+';

impl ButtonChord {
    /// One button on its own.
    pub const fn single(button: Button) -> Self {
        Self {
            first: button,
            second: None,
        }
    }

    /// `a` and `b` pressed together, in either order; `None` if they are
    /// the same button.
    pub fn pair(a: Button, b: Button) -> Option<Self> {
        let (first, second) = match a.cmp(&b) {
            core::cmp::Ordering::Less => (a, b),
            core::cmp::Ordering::Greater => (b, a),
            core::cmp::Ordering::Equal => return None,
        };
        Some(Self {
            first,
            second: Some(second),
        })
    }

    /// The button, or a chord's first button in [`Button::ALL`] order.
    pub fn first(self) -> Button {
        self.first
    }

    /// The button, if this is one button on its own.
    pub fn single_button(self) -> Option<Button> {
        match self.second {
            None => Some(self.first),
            Some(_) => None,
        }
    }

    /// The two buttons, in [`Button::ALL`] order, if this is a chord.
    pub fn buttons(self) -> Option<(Button, Button)> {
        self.second.map(|second| (self.first, second))
    }

    /// The other button of a chord that has `button`, if this is one.
    pub fn partner(self, button: Button) -> Option<Button> {
        let (a, b) = self.buttons()?;
        if a == button {
            Some(b)
        } else if b == button {
            Some(a)
        } else {
            None
        }
    }

    /// Parses `"South"` or `"South+East"` (either order).
    pub fn parse(s: &str) -> Result<Self, String> {
        let Some((a, b)) = s.split_once(CHORD_JOIN) else {
            return Button::parse(s).map(Self::single);
        };
        let (a, b) = (Button::parse(a.trim())?, Button::parse(b.trim())?);
        Self::pair(a, b).ok_or_else(|| format!("\"{s}\" names the same button twice"))
    }
}

impl From<Button> for ButtonChord {
    fn from(button: Button) -> Self {
        Self::single(button)
    }
}

/// A binding is a button when it is that button on its own.
impl PartialEq<Button> for ButtonChord {
    fn eq(&self, button: &Button) -> bool {
        self.single_button() == Some(*button)
    }
}

impl FromStr for ButtonChord {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

impl fmt::Display for ButtonChord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.first.name())?;
        if let Some(second) = self.second {
            write!(f, "{CHORD_JOIN}{}", second.name())?;
        }
        Ok(())
    }
}

/// Key-repeat timings for held repeatable actions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RepeatDef {
    /// Time from the press to the first repeat, in milliseconds.
    pub delay_ms: u32,
    /// Time between later repeats, in milliseconds (never 0).
    pub interval_ms: u32,
}

impl Default for RepeatDef {
    fn default() -> Self {
        Self {
            delay_ms: 300,
            interval_ms: 55,
        }
    }
}

/// How far a stick must be pushed to count as a held direction
/// ([`Button::LeftStickUp`] and the rest), in percent of its full travel.
/// `release_percent` is below `press_percent`, so a stick resting near the
/// edge of the dead zone doesn't flicker.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct StickDef {
    /// A direction starts once the stick is pushed at least this far.
    pub press_percent: u8,
    /// A held direction ends once the stick falls below this.
    pub release_percent: u8,
}

impl Default for StickDef {
    fn default() -> Self {
        Self {
            press_percent: 50,
            release_percent: 35,
        }
    }
}

/// How [`ButtonChord`]s fire (`shuyi_ui::input::state`): a chord button
/// whose own action waits for its partner waits at most `window_ms`
/// (unless it waits for its release).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ChordDef {
    /// How long, in milliseconds, two buttons pressed one after the other
    /// still count as pressed together.
    pub window_ms: u32,
}

impl Default for ChordDef {
    fn default() -> Self {
        Self { window_ms: 80 }
    }
}

/// How a crank turns into [`Button::CrankBack`] and
/// [`Button::CrankForward`] presses (`shuyi_ui::input::crank`): one per
/// `notch_degrees` wound.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CrankDef {
    /// How far, in degrees, the crank is wound for one notch (1 to 360).
    pub notch_degrees: u16,
}

impl Default for CrankDef {
    fn default() -> Self {
        Self { notch_degrees: 45 }
    }
}

/// Most bytes in a [`Layout`]'s name.
pub const LAYOUT_NAME_MAX: usize = 24;

/// A complete set of key bindings the player can pick: one entry of
/// `layouts` in the keymap file, named by its `id`. The file decides which
/// layouts there are; this is only the name, kept inline so a layout is
/// `Copy`. Names are ASCII letters, digits and `_`, at most
/// [`LAYOUT_NAME_MAX`] bytes (`RightHanded`).
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Layout {
    len: usize,
    bytes: [u8; LAYOUT_NAME_MAX],
}

impl Layout {
    /// The layout named `name`, for a game's constants. Fails to compile in
    /// a `const` (panics elsewhere) if `name` isn't a valid name.
    pub const fn named(name: &'static str) -> Self {
        match Self::try_named(name) {
            Some(layout) => layout,
            None => panic!("a layout name is 1 to 24 ASCII letters, digits or _"),
        }
    }

    /// The layout named `name`, if it is a valid name (saved settings, the
    /// keymap file). Whether a keymap has it is
    /// [`KeymapDef::has_layout`]'s question.
    pub fn from_name(name: &str) -> Option<Self> {
        Self::try_named(name)
    }

    const fn try_named(name: &str) -> Option<Self> {
        let given = name.as_bytes();
        if given.is_empty() || given.len() > LAYOUT_NAME_MAX {
            return None;
        }
        let mut bytes = [0; LAYOUT_NAME_MAX];
        let mut i = 0;
        while i < given.len() {
            let b = given[i];
            if !(b.is_ascii_alphanumeric() || b == b'_') {
                return None;
            }
            bytes[i] = b;
            i += 1;
        }
        Some(Self {
            len: given.len(),
            bytes,
        })
    }

    /// The name used in keymap files and saved settings.
    pub fn name(&self) -> &str {
        let bytes = self.bytes.get(..self.len).unwrap_or_default();
        // Only ASCII is ever stored.
        core::str::from_utf8(bytes).unwrap_or_default()
    }
}

impl Ord for Layout {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.name().cmp(other.name())
    }
}

impl PartialOrd for Layout {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl fmt::Debug for Layout {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Layout({})", self.name())
    }
}

impl fmt::Display for Layout {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// A layout's binary form (a baked keymap, `pack`) is its name.
impl Serialize for Layout {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.name())
    }
}

impl<'de> Deserialize<'de> for Layout {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let name = String::deserialize(deserializer)?;
        Self::from_name(&name)
            .ok_or_else(|| serde::de::Error::custom(format!("{name:?} is not a layout name")))
    }
}

/// Validated bindings as a lookup: each chord maps to exactly one action
/// (of one [group](ActionSet::Group)). Unbound chords are absent.
pub type Bindings<A> = BTreeMap<Chord, A>;

/// One layout's default keys: every action with its chords in the order
/// the keymap file lists them (at most [`SLOTS`], none [reserved]), no chord
/// on two actions of one [group](ActionSet::Group). The order matters: the
/// first chord is the one help text names, and the chords fill the player's
/// key slots in this order.
///
/// [reserved]: Chord::is_reserved
pub type LayoutKeys<A> = BTreeMap<A, Vec<Chord>>;

/// The default controller buttons: every action with its buttons and
/// [button chords](ButtonChord) in the order the keymap file lists them (at
/// most [`SLOTS`]), none on two actions of one [group](ActionSet::Group).
/// One table for every layout.
pub type PadKeys<A> = BTreeMap<A, Vec<ButtonChord>>;

/// The validated keymap: every [`Layout`]'s default keys, the layout
/// picker's bindings, the default controller buttons, plus the key-repeat
/// timings and stick thresholds they share.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeymapDef<A: ActionSet> {
    /// Layout → its default keys. After
    /// [`from_source`](Self::from_source) every layout the file lists is
    /// present, with every action.
    pub layouts: BTreeMap<Layout, LayoutKeys<A>>,
    /// The layouts in the order the file lists them (the order the layout
    /// picker shows them), each with the text key of its name.
    pub layout_names: LayoutNames,
    /// The keys that work before any layout is chosen (the layout picker).
    /// Actions not listed are unbound there.
    pub layout_picker: Bindings<A>,
    /// The default controller buttons, the same in every layout and the
    /// layout picker. After [`from_source`](Self::from_source) every action
    /// is present.
    pub pad: PadKeys<A>,
    /// The default buttons of a Playdate (game ADR-0070, ticket 1034), whose
    /// only buttons are the D-pad, A and B: the file's `playdate` table, or
    /// a copy of [`pad`](Self::pad) for a file without one. A game running
    /// on a Playdate uses it in place of `pad`
    /// ([`for_playdate`](Self::for_playdate)). The same as
    /// `targets["playdate"]` when the file has one.
    pub playdate: PadKeys<A>,
    /// The default buttons on a target, by its profile's id
    /// (`shuyi_assets::target`, ADR-0008 §3): the file's `targets` table,
    /// with its `playdate` table as `"playdate"`. A target in it plays with
    /// its own table in place of [`pad`](Self::pad)
    /// ([`for_target`](Self::for_target)), and `shuyi check --target`
    /// checks the target's controls on it.
    pub targets: BTreeMap<String, PadKeys<A>>,
    /// Key-repeat timings, for held keys and held buttons alike.
    pub repeat: RepeatDef,
    /// How button chords fire (optional in the file).
    pub chord: ChordDef,
    /// How far a stick is pushed to count as a direction.
    pub stick: StickDef,
    /// How far a crank is wound for one notch (optional in the file).
    pub crank: CrankDef,
}

impl<A: ActionSet> Default for KeymapDef<A> {
    fn default() -> Self {
        Self {
            layouts: BTreeMap::new(),
            layout_names: Vec::new(),
            layout_picker: BTreeMap::new(),
            pad: BTreeMap::new(),
            playdate: BTreeMap::new(),
            targets: BTreeMap::new(),
            repeat: RepeatDef::default(),
            chord: ChordDef::default(),
            stick: StickDef::default(),
            crank: CrankDef::default(),
        }
    }
}

impl<A: ActionSet> AssetKind for KeymapDef<A> {
    const NAME: &'static str = "keymap";
}

/// The keymap file (`keymap`, `.ron`) of a game whose actions are `A`,
/// checked as [`KeymapDef::from_source`] says.
pub struct KeymapRon<A>(PhantomData<fn() -> A>);

impl<A> KeymapRon<A> {
    /// The format, for a game's actions `A`.
    pub const fn new() -> Self {
        Self(PhantomData)
    }
}

impl<A> Default for KeymapRon<A> {
    fn default() -> Self {
        Self::new()
    }
}

impl<A> fmt::Debug for KeymapRon<A> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("KeymapRon")
    }
}

impl<A: ActionSet> Format for KeymapRon<A> {
    type Asset = KeymapDef<A>;

    fn id(&self) -> &'static str {
        "keymap"
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
    ) -> Result<KeymapDef<A>, Vec<ContentError>> {
        KeymapDef::from_source(file.shown(), text(file, bytes)?)
    }
}

/// The layouts in file order, each with the text key of its name.
pub type LayoutNames = Vec<(Layout, String)>;

/// The file as written, before validation.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawKeymap {
    layouts: Vec<RawLayout>,
    layout_picker: BTreeMap<String, Vec<String>>,
    pad: BTreeMap<String, Vec<String>>,
    /// Absent (empty) in a file for no Playdate.
    #[serde(default)]
    playdate: BTreeMap<String, Vec<String>>,
    /// Target id → its default buttons; absent (empty) in a file with
    /// none.
    #[serde(default)]
    targets: BTreeMap<String, BTreeMap<String, Vec<String>>>,
    repeat: RepeatDef,
    #[serde(default)]
    chord: ChordDef,
    stick: StickDef,
    #[serde(default)]
    crank: CrankDef,
}

/// One entry of `layouts`, before validation.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawLayout {
    /// The name saved settings know it by.
    id: String,
    /// The text key of the name the player sees.
    name: String,
    /// Action name → chord names.
    keys: BTreeMap<String, Vec<String>>,
}

impl<A: ActionSet> KeymapDef<A> {
    /// `layout`'s default bindings of the actions in `group`, as a chord
    /// lookup (`None` if the keymap has no such layout).
    pub fn bindings(&self, layout: Layout, group: A::Group) -> Option<Bindings<A>> {
        let keys = self.layouts.get(&layout)?;
        Some(
            keys.iter()
                .filter(|(a, _)| a.group() == group)
                .flat_map(|(&a, chords)| chords.iter().map(move |&c| (c, a)))
                .collect(),
        )
    }

    /// `action`'s default chords in `layout`, in file order; empty if the
    /// layout or action is missing.
    pub fn chords(&self, layout: Layout, action: A) -> &[Chord] {
        self.layouts
            .get(&layout)
            .and_then(|keys| keys.get(&action))
            .map_or(&[], Vec::as_slice)
    }

    /// `action`'s default controller buttons and button chords, in file
    /// order; empty if the action is missing (only in a definition built by
    /// hand).
    pub fn buttons(&self, action: A) -> &[ButtonChord] {
        self.pad.get(&action).map_or(&[], Vec::as_slice)
    }

    /// This keymap as a Playdate uses it: its [`playdate`](Self::playdate)
    /// table as the default buttons.
    #[must_use]
    pub fn for_playdate(mut self) -> Self {
        self.pad.clone_from(&self.playdate);
        self
    }

    /// The default buttons of target `id` (its profile's id), if the file
    /// gives that target a table of its own ([`targets`](Self::targets)).
    pub fn target_buttons(&self, id: &str) -> Option<&PadKeys<A>> {
        self.targets.get(id)
    }

    /// This keymap as target `id` uses it: the target's own table as the
    /// default buttons, if it has one, else [`pad`](Self::pad).
    #[must_use]
    pub fn for_target(mut self, id: &str) -> Self {
        if let Some(buttons) = self.targets.get(id) {
            self.pad = buttons.clone();
        }
        self
    }

    /// Whether the keymap has `layout` (a name from saved settings may
    /// name one it no longer has).
    pub fn has_layout(&self, layout: Layout) -> bool {
        self.layouts.contains_key(&layout)
    }

    /// The layouts in file order: the order the layout picker lists them.
    pub fn layout_ids(&self) -> impl Iterator<Item = Layout> + '_ {
        self.layout_names.iter().map(|&(layout, _)| layout)
    }

    /// The first layout the file lists: the one a screen falls back on
    /// while none is chosen.
    pub fn first_layout(&self) -> Option<Layout> {
        self.layout_ids().next()
    }

    /// The text key of `layout`'s name, if the keymap has it.
    pub fn layout_text(&self, layout: Layout) -> Option<&str> {
        let (_, text) = self.layout_names.iter().find(|&&(l, _)| l == layout)?;
        Some(text)
    }

    /// Parses and validates keymap `source`, attributing errors to `file`.
    /// Reports a file with no layouts, a bad or repeated layout `id`, and
    /// within each layout every unknown action, unparsable chord, chord
    /// bound more than once, [reserved](Chord::is_reserved) chord, action
    /// with more than [`SLOTS`] chords and missing action (prefixed with the
    /// layout's id), the same for the `layout_picker` section (except that
    /// unlisted actions are unbound there, and an action may have more than
    /// [`SLOTS`] chords: the picker isn't rebindable), and for the `pad`
    /// and `playdate` sections and each table of `targets` (buttons and
    /// [button chords](ButtonChord) instead of key chords, every action
    /// listed, none on an action that [can't be
    /// rebound](ActionSet::is_rebindable), no system menu item in a chord),
    /// plus a bad target id, a Playdate table given both ways, bad repeat
    /// timing, stick thresholds and crank notch size.
    pub fn from_source(file: &str, source: &str) -> Result<Self, Vec<ContentError>> {
        let raw: RawKeymap = parse_ron(file, source).map_err(|e| vec![e])?;
        let mut errors = Vec::new();
        let (layouts, layout_names) =
            validate_layouts::<A>(file, source, &raw.layouts, &mut errors);
        let picker_line = line_of_quoted(source, 0, "layout_picker");
        let from = picker_line.map_or(0, |line| usize::try_from(line).unwrap_or(0));
        let layout_picker = match validate_bindings::<A, Chord>(
            file,
            source,
            from,
            "layout_picker",
            false,
            &raw.layout_picker,
        ) {
            Ok(keys) => keys
                .iter()
                .flat_map(|(&a, chords)| chords.iter().map(move |&c| (c, a)))
                .collect(),
            Err(e) => {
                errors.extend(e);
                Bindings::new()
            }
        };
        let pad = validate_pad::<A>(file, source, "pad", "pad", &raw.pad, &mut errors);
        let mut targets = validate_targets::<A>(file, source, &raw.targets, &mut errors);
        let playdate = if raw.playdate.is_empty() {
            targets
                .get(PLAYDATE)
                .cloned()
                .unwrap_or_else(|| pad.clone())
        } else {
            if raw.targets.contains_key(PLAYDATE) {
                errors.push(positioned(
                    ContentError::new(
                        file,
                        "the Playdate's buttons are given twice: as `playdate` and as \
                         `targets.\"playdate\"` (keep one)",
                    ),
                    line_of_quoted(source, 0, PLAYDATE),
                ));
            }
            let table =
                validate_pad::<A>(file, source, PLAYDATE, PLAYDATE, &raw.playdate, &mut errors);
            targets.insert(PLAYDATE.to_owned(), table.clone());
            table
        };
        if raw.repeat.interval_ms == 0 {
            errors.push(positioned(
                ContentError::new(file, "repeat interval_ms must be at least 1")
                    .with_code(codes::OUT_OF_RANGE),
                line_of_quoted(source, 0, "repeat"),
            ));
        }
        let StickDef {
            press_percent,
            release_percent,
        } = raw.stick;
        if release_percent == 0 || release_percent >= press_percent || press_percent > 100 {
            errors.push(positioned(
                ContentError::new(
                    file,
                    "stick release_percent must be at least 1 and below press_percent, which \
                     must be at most 100",
                ),
                line_of_quoted(source, 0, "stick"),
            ));
        }
        if !(1..=360).contains(&raw.crank.notch_degrees) {
            errors.push(positioned(
                ContentError::new(file, "crank notch_degrees must be 1 to 360")
                    .with_code(codes::OUT_OF_RANGE),
                line_of_quoted(source, 0, "crank"),
            ));
        }
        if errors.is_empty() {
            Ok(Self {
                layouts,
                layout_names,
                layout_picker,
                pad,
                playdate,
                targets,
                repeat: raw.repeat,
                chord: raw.chord,
                stick: raw.stick,
                crank: raw.crank,
            })
        } else {
            Err(errors)
        }
    }
}

/// The layouts the file lists (`layouts`): each one's validated keys, and
/// the layouts in file order with the text keys of their names. Errors go
/// to `errors`: no layout at all, a bad or repeated id, and every error of
/// a layout's keys.
fn validate_layouts<A: ActionSet>(
    file: &str,
    source: &str,
    raw: &[RawLayout],
    errors: &mut Vec<ContentError>,
) -> (BTreeMap<Layout, LayoutKeys<A>>, LayoutNames) {
    let mut layouts = BTreeMap::new();
    let mut layout_names = Vec::new();
    if raw.is_empty() {
        errors.push(positioned(
            ContentError::new(file, "layouts: list at least one layout"),
            line_of_quoted(source, 0, "layouts"),
        ));
    }
    let mut listed = BTreeSet::new();
    let mut search_from = 0;
    for RawLayout { id, name, keys } in raw {
        let start = line_of_quoted(source, search_from, id);
        // Search the layout's own lines first, so errors in the second
        // layout don't point into the first.
        let from = start.map_or(search_from, |line| {
            usize::try_from(line).unwrap_or(search_from)
        });
        search_from = from;
        let Some(layout) = Layout::from_name(id) else {
            let message = format!(
                "layout id \"{id}\" must be 1 to {LAYOUT_NAME_MAX} ASCII letters, digits or _"
            );
            errors.push(positioned(ContentError::new(file, message), start));
            continue;
        };
        if !listed.insert(layout) {
            let message = format!("layout \"{layout}\" is listed twice");
            errors.push(positioned(
                ContentError::new(file, message).with_code(codes::DUPLICATE),
                start,
            ));
            continue;
        }
        let label = format!("layout \"{layout}\"");
        match validate_bindings::<A, Chord>(file, source, from, &label, true, keys) {
            Ok(keys) => {
                layouts.insert(layout, keys);
                layout_names.push((layout, name.clone()));
            }
            Err(e) => errors.extend(e),
        }
    }
    (layouts, layout_names)
}

/// The Playdate's target id, and the name of its table of the file.
const PLAYDATE: &str = "playdate";

/// Most bytes in a target id of the `targets` table.
const TARGET_ID_MAX: usize = 32;

/// Whether `id` can be a target's id: 1 to [`TARGET_ID_MAX`] lowercase
/// ASCII letters, digits and `-`, as the profiles' files are named
/// (`steam-deck`).
fn is_target_id(id: &str) -> bool {
    (1..=TARGET_ID_MAX).contains(&id.len())
        && id
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

/// The file's `targets`: each target's table of default buttons,
/// validated as `pad` is, by target id. Errors go to `errors`: a bad id,
/// and every error of a table.
fn validate_targets<A: ActionSet>(
    file: &str,
    source: &str,
    raw: &BTreeMap<String, BTreeMap<String, Vec<String>>>,
    errors: &mut Vec<ContentError>,
) -> BTreeMap<String, PadKeys<A>> {
    let mut targets = BTreeMap::new();
    for (id, table) in raw {
        if !is_target_id(id) {
            let message = format!(
                "targets: target id \"{id}\" must be 1 to {TARGET_ID_MAX} lowercase ASCII \
                 letters, digits or -"
            );
            errors.push(positioned(
                ContentError::new(file, message),
                line_of_quoted(source, 0, id),
            ));
            continue;
        }
        let label = format!("targets.\"{id}\"");
        let before = errors.len();
        let buttons = validate_pad::<A>(file, source, &label, id, table, errors);
        if errors.len() == before {
            targets.insert(id.clone(), buttons);
        }
    }
    targets
}

/// A table of default controller buttons (`pad`, `playdate`, a target's)
/// named `label` in messages, validated; its errors go to `errors`. Its
/// lines are searched for from the first that has `anchor` quoted or as a
/// field.
fn validate_pad<A: ActionSet>(
    file: &str,
    source: &str,
    label: &str,
    anchor: &str,
    raw: &BTreeMap<String, Vec<String>>,
    errors: &mut Vec<ContentError>,
) -> PadKeys<A> {
    let line = line_of_quoted(source, 0, anchor);
    let from = line.map_or(0, |line| usize::try_from(line).unwrap_or(0));
    let pad = match validate_bindings::<A, ButtonChord>(file, source, from, label, true, raw) {
        Ok(buttons) => buttons,
        Err(e) => {
            errors.extend(e);
            return PadKeys::new();
        }
    };
    for (&action, buttons) in &pad {
        if !action.is_rebindable() && !buttons.is_empty() {
            let message = format!(
                "{label}: action \"{}\" can't have a button: it can't be rebound, so it is \
                 keyboard only",
                action.name()
            );
            errors.push(positioned(
                ContentError::new(file, message),
                line_of_quoted(source, from, action.name()),
            ));
        }
    }
    pad
}

/// Something `keymap.ron` binds to actions: a key [`Chord`] or a controller
/// [`ButtonChord`].
trait Bound: Copy + Ord + fmt::Display {
    /// What one is called in error messages.
    const NOUN: &'static str;
    /// What several are called in error messages.
    const PLURAL: &'static str;

    /// Parses its keymap-file name.
    fn parse_name(text: &str) -> Result<Self, String>;

    /// Why it may never appear in the file, if so.
    fn refused(self) -> Option<&'static str>;
}

impl Bound for Chord {
    const NOUN: &'static str = "chord";
    const PLURAL: &'static str = "keys";

    fn parse_name(text: &str) -> Result<Self, String> {
        Self::parse(text)
    }

    fn refused(self) -> Option<&'static str> {
        self.is_reserved()
            .then_some("Esc and Delete are fixed, see controls.md")
    }
}

impl Bound for ButtonChord {
    const NOUN: &'static str = "button";
    const PLURAL: &'static str = "buttons";

    fn parse_name(text: &str) -> Result<Self, String> {
        Self::parse(text)
    }

    fn refused(self) -> Option<&'static str> {
        self.buttons()
            .is_some_and(|(a, b)| a.system_menu_slot().or(b.system_menu_slot()).is_some())
            .then_some("a system menu item is picked on its own, never with another button")
    }
}

/// Validates one section's `actions` (action name → chord or button
/// names): a layout, the layout picker or the `pad` table. Messages start
/// with `label`; lines are searched from line index `from` (just after the
/// section's own line) on. For a `complete` section (a layout, `pad`), every
/// action must be listed, with at most [`SLOTS`] entries. A chord or
/// button may be on one action of each [group](ActionSet::Group).
fn validate_bindings<A: ActionSet, T: Bound>(
    file: &str,
    source: &str,
    from: usize,
    label: &str,
    complete: bool,
    actions: &BTreeMap<String, Vec<String>>,
) -> Result<BTreeMap<A, Vec<T>>, Vec<ContentError>> {
    let noun = T::NOUN;
    let mut errors = Vec::new();
    let mut bindings: BTreeMap<(A::Group, T), A> = BTreeMap::new();
    let mut keys: BTreeMap<A, Vec<T>> = BTreeMap::new();
    let err_at = |key: &str, message: String| {
        positioned(
            ContentError::new(file, format!("{label}: {message}")),
            line_of_quoted(source, from, key),
        )
    };
    let coded_at = |key: &str, message: String, code| err_at(key, message).with_code(code);
    for (action_name, names) in actions {
        let Some(action) = A::from_name(action_name) else {
            let known: Vec<&str> = A::ALL.iter().map(|a| a.name()).collect();
            errors.push(coded_at(
                action_name,
                format!(
                    "unknown action \"{action_name}\"; known actions: {}",
                    known.join(", ")
                ),
                codes::UNKNOWN_NAME,
            ));
            continue;
        };
        if complete && names.len() > SLOTS {
            errors.push(err_at(
                action_name,
                format!(
                    "action \"{}\" has {} {}; at most {SLOTS} are allowed",
                    action.name(),
                    names.len(),
                    T::PLURAL
                ),
            ));
        }
        let name = action.name();
        let listed = keys.entry(action).or_default();
        for text in names {
            let bound = match T::parse_name(text) {
                Ok(bound) => bound,
                Err(why) => {
                    errors.push(coded_at(
                        action_name,
                        format!("action \"{action_name}\": {why}"),
                        codes::UNKNOWN_NAME,
                    ));
                    continue;
                }
            };
            if let Some(why) = bound.refused() {
                errors.push(err_at(
                    action_name,
                    format!("action \"{name}\": \"{bound}\" can't be bound here: {why}"),
                ));
            } else if let Some(&other) = bindings.get(&(action.group(), bound)) {
                let message = if other == action {
                    format!("{noun} \"{bound}\" is listed twice for \"{name}\"")
                } else {
                    format!(
                        "{noun} \"{bound}\" is bound to both \"{}\" and \"{name}\"",
                        other.name()
                    )
                };
                errors.push(coded_at(action_name, message, codes::DUPLICATE));
            } else {
                bindings.insert((action.group(), bound), action);
                listed.push(bound);
            }
        }
    }
    for action in A::ALL {
        if complete && !actions.contains_key(action.name()) {
            errors.push(ContentError::new(
                file,
                format!(
                    "{label}: missing action \"{}\" (list it with [] to leave it \
                     unbound)",
                    action.name()
                ),
            ));
        }
    }
    if errors.is_empty() {
        Ok(keys)
    } else {
        Err(errors)
    }
}

/// `error` at `line`, if known.
fn positioned(error: ContentError, line: Option<u32>) -> ContentError {
    match line {
        Some(line) => error.at(line, None),
        None => error,
    }
}

/// 1-based number of the first line, at or after line index `from`
/// (0-based), that contains `key` quoted or starts with the field `key:`,
/// for error positions.
fn line_of_quoted(source: &str, from: usize, key: &str) -> Option<u32> {
    let quoted = format!("\"{key}\"");
    let field = format!("{key}:");
    let (index, _) = source
        .lines()
        .enumerate()
        .skip(from)
        .find(|(_, l)| l.contains(&quoted) || l.trim_start().starts_with(&field))?;
    u32::try_from(index + 1).ok()
}
