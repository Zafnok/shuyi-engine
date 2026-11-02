//! Controllers (ticket 0219, game ADR-0034): from each connected pad's raw state
//! to the [`Button`] presses and releases [`InputState`] is fed. Pure:
//! `app` fills a [`PadState`] per pad every frame (gilrs on native, the
//! browser's Gamepad API on web) and [`Pads::update`] does the rest, so both
//! platforms share these rules (`docs/design/controls.md`, *Controller*):
//!
//! - **Sticks are 4-way buttons.** A stick pushed far enough counts as one
//!   held direction button, the direction it is pushed furthest, so it
//!   repeats like a held key and never moves diagonally.
//! - **Switch-style pads swap Confirm and Cancel.** Bindings name button
//!   *positions*; on a [`PadKind::Nintendo`] pad the bottom and right face
//!   buttons trade places before the keymap sees them.
//! - **Several pads drive the game as one.** A button held on two pads is
//!   one press, released when the last pad lets go; unplugging a pad
//!   releases what it held.
//!
//! [`InputState`]: super::InputState

use crate::prelude::*;
use alloc::collections::BTreeMap;

use super::{Button, ButtonChord, StickDef};

/// USB vendor ids of the pad makers the game tells apart.
const MICROSOFT: u16 = 0x045e;
const SONY: u16 = 0x054c;
const NINTENDO: u16 = 0x057e;

/// USB product ids of Sony's PS4 pad (the `DualShock` 4): its two models
/// and its wireless adaptor. Every other Sony pad counts as a PS5 one
/// (ticket 0229).
const DUALSHOCK_4: [u16; 3] = [0x05c4, 0x09cc, 0x0ba0];

/// Who made a pad, which decides the Confirm / Cancel swap here and the
/// button names shown on screen (ticket 0220).
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Default,
    serde::Serialize,
    serde::Deserialize,
)]
pub enum PadKind {
    /// A Microsoft pad.
    Xbox,
    /// A Sony pad other than a PS4 one (a PS5 pad).
    PlayStation,
    /// A Sony PS4 pad: named as a [`PlayStation`](Self::PlayStation) pad,
    /// except its left centre button is `Share`.
    PlayStation4,
    /// A Nintendo pad (Switch Pro controller, Joy-Cons).
    Nintendo,
    /// The Playdate's own buttons (game ADR-0070): the D-pad, A and B. Its
    /// backend reports A at the binding position `East` and B at `West`
    /// (`targets/playdate.ron`, ADR-0008 §1), with no swap, so a game binds
    /// them in its keymap's `targets."playdate"` table.
    Playdate,
    /// Anything else, or a pad that doesn't say. Treated like an Xbox pad.
    #[default]
    Generic,
}

impl PadKind {
    /// Every kind.
    pub const ALL: [PadKind; 6] = [
        Self::Xbox,
        Self::PlayStation,
        Self::PlayStation4,
        Self::Nintendo,
        Self::Generic,
        Self::Playdate,
    ];

    /// The kind of a pad from its USB vendor and product ids; unknown
    /// vendors (and 0, "not reported") are [`Generic`](Self::Generic). The
    /// product only matters for Sony, to tell a PS4 pad apart.
    pub fn from_ids(vendor: u16, product: u16) -> Self {
        match vendor {
            MICROSOFT => Self::Xbox,
            SONY if DUALSHOCK_4.contains(&product) => Self::PlayStation4,
            SONY => Self::PlayStation,
            NINTENDO => Self::Nintendo,
            _ => Self::Generic,
        }
    }

    /// The position bindings use for the button physically at `button` on
    /// this kind of pad: on a Nintendo pad the bottom and right face buttons
    /// swap (so its right button, labelled `A`, is `South` and confirms by
    /// default); every other button, and every other pad, is unchanged.
    pub fn position(self, button: Button) -> Button {
        match (self, button) {
            (Self::Nintendo, Button::South) => Button::East,
            (Self::Nintendo, Button::East) => Button::South,
            _ => button,
        }
    }

    /// What help text calls the button at binding position `button` on
    /// this kind of pad (`docs/design/controls.md`, *Button names on
    /// screen*): Xbox letters (also on generic pads), Sony's shapes,
    /// or Nintendo's letters (a PS4 pad says `Share` where later Sony pads
    /// say `Create`). The Playdate's `A` is `East` and its `B` is `West`, as
    /// printed on it; every other name is the Xbox one, its D-pad's and its
    /// crank's among them, but `South`'s, which is the `X` that leaves free:
    /// `A` would name a button its A doesn't press. It has none of the other
    /// buttons (their actions move to chords and menus in tickets 1034 and
    /// 1035). The only place buttons get their names.
    // check-text: not player text (a button's name is what is printed on the pad)
    pub fn button_name(self, button: Button) -> &'static str {
        let [xbox, playstation, nintendo] = names(button);
        match self {
            Self::Playdate if button == Button::East => "A",
            Self::Playdate if button == Button::West => "B",
            Self::Playdate if button == Button::South => "X",
            Self::Xbox | Self::Generic | Self::Playdate => xbox,
            Self::PlayStation4 if button == Button::Select => "Share",
            Self::PlayStation | Self::PlayStation4 => playstation,
            Self::Nintendo => nintendo,
        }
    }

    /// What help text calls `chord` on this kind of pad: its
    /// [button's name](Self::button_name), or a button chord's two names
    /// with `join` between them (`A+B`, `B+←` with `+`), in the chord's
    /// order.
    pub fn chord_name(self, chord: ButtonChord, join: &str) -> String {
        match chord.buttons() {
            Some((a, b)) => format!("{}{join}{}", self.button_name(a), self.button_name(b)),
            None => self.button_name(chord.first()).to_owned(),
        }
    }
}

/// A Sony pad's face buttons' shapes: glyphs of our own in the font
/// (`assets-src/fonts/pad-shapes.bdf`, ticket 0220).
const CROSS: &str = "\u{2715}";
const CIRCLE: &str = "\u{25ef}";
const SQUARE: &str = "\u{25a1}";
const TRIANGLE: &str = "\u{25b3}";

/// `button`'s name on an Xbox, a Sony and a Nintendo pad, by
/// binding position: on a Nintendo pad `South` is the right face button
/// (see [`PadKind::position`]), so it is its `A`. Directions and stick
/// presses are named the same on every pad, except Sony's `L3` /
/// `R3`.
// check-text: not player text (a button's name is what is printed on the pad)
fn names(button: Button) -> [&'static str; 3] {
    match button {
        Button::South => ["A", CROSS, "A"],
        Button::East => ["B", CIRCLE, "B"],
        Button::West => ["X", SQUARE, "Y"],
        Button::North => ["Y", TRIANGLE, "X"],
        Button::LeftShoulder => ["LB", "L1", "L"],
        Button::RightShoulder => ["RB", "R1", "R"],
        Button::LeftTrigger => ["LT", "L2", "ZL"],
        Button::RightTrigger => ["RT", "R2", "ZR"],
        Button::Select => ["Back", "Create", "\u{2212}"],
        Button::Start => ["Start", "Options", "+"],
        Button::LeftStickPress => ["LS", "L3", "LS"],
        Button::RightStickPress => ["RS", "R3", "RS"],
        Button::DpadUp => ["↑"; 3],
        Button::DpadDown => ["↓"; 3],
        Button::DpadLeft => ["←"; 3],
        Button::DpadRight => ["→"; 3],
        Button::LeftStickUp => ["L-stick ↑"; 3],
        Button::LeftStickDown => ["L-stick ↓"; 3],
        Button::LeftStickLeft => ["L-stick ←"; 3],
        Button::LeftStickRight => ["L-stick →"; 3],
        Button::RightStickUp => ["R-stick ↑"; 3],
        Button::RightStickDown => ["R-stick ↓"; 3],
        Button::RightStickLeft => ["R-stick ←"; 3],
        Button::RightStickRight => ["R-stick →"; 3],
        Button::CrankBack => ["Crank back"; 3],
        Button::CrankForward => ["Crank forward"; 3],
        Button::SystemMenu0 => ["Menu item 1"; 3],
        Button::SystemMenu1 => ["Menu item 2"; 3],
        Button::SystemMenu2 => ["Menu item 3"; 3],
        Button::SystemMenu3 => ["Menu item 4"; 3],
        Button::SystemMenu4 => ["Menu item 5"; 3],
        Button::SystemMenu5 => ["Menu item 6"; 3],
        Button::SystemMenu6 => ["Menu item 7"; 3],
        Button::SystemMenu7 => ["Menu item 8"; 3],
    }
}

/// A set of [`Button`]s.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct ButtonSet(u64);

impl ButtonSet {
    /// No buttons.
    pub const EMPTY: Self = Self(0);

    /// Adds `button`.
    pub fn insert(&mut self, button: Button) {
        self.0 |= bit(button);
    }

    /// Takes `button` out.
    pub fn remove(&mut self, button: Button) {
        self.0 &= !bit(button);
    }

    /// Whether `button` is in the set.
    pub fn contains(self, button: Button) -> bool {
        self.0 & bit(button) != 0
    }

    /// The buttons in either set.
    #[must_use]
    pub fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    /// The buttons in the set, in [`Button::ALL`] order.
    pub fn iter(self) -> impl Iterator<Item = Button> {
        Button::ALL
            .iter()
            .copied()
            .filter(move |&b| self.contains(b))
    }
}

/// `button`'s bit: its index in [`Button::ALL`].
fn bit(button: Button) -> u64 {
    1 << (button as u64)
}

impl FromIterator<Button> for ButtonSet {
    fn from_iter<I: IntoIterator<Item = Button>>(buttons: I) -> Self {
        let mut set = Self::EMPTY;
        for button in buttons {
            set.insert(button);
        }
        set
    }
}

/// How many buttons the browser Gamepad API's "standard" mapping has before
/// its optional extras: the first 16 of [`Button::ALL`], in the same order.
const STANDARD_BUTTONS: u32 = 16;

/// One pad's raw state in one frame, as the platform reports it.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct PadState {
    /// The real buttons held, by where they physically are on the pad (the
    /// stick directions come from the sticks below).
    pub buttons: ButtonSet,
    /// The left stick as `(x, y)`, each from -1 to 1: `+x` is right, `+y`
    /// is **down**.
    pub left_stick: (f32, f32),
    /// The right stick, like [`left_stick`](Self::left_stick).
    pub right_stick: (f32, f32),
}

impl PadState {
    /// A pad the browser reports in the Gamepad API's "standard" mapping:
    /// bit `i` of `buttons` is set while its `buttons[i]` is pressed (only
    /// the first 16 count), and `axes` are its first four axes (left stick
    /// x, y, right stick x, y; down is positive).
    pub fn from_standard(buttons: u32, axes: [f32; 4]) -> Self {
        let [left_x, left_y, right_x, right_y] = axes;
        Self {
            buttons: ButtonSet(u64::from(buttons) & ((1 << STANDARD_BUTTONS) - 1)),
            left_stick: (left_x, left_y),
            right_stick: (right_x, right_y),
        }
    }
}

/// A stick's direction buttons: up, down, left, right.
pub type StickButtons = [Button; 4];

const DPAD: StickButtons = [
    Button::DpadUp,
    Button::DpadDown,
    Button::DpadLeft,
    Button::DpadRight,
];
const LEFT_STICK: StickButtons = [
    Button::LeftStickUp,
    Button::LeftStickDown,
    Button::LeftStickLeft,
    Button::LeftStickRight,
];
const RIGHT_STICK: StickButtons = [
    Button::RightStickUp,
    Button::RightStickDown,
    Button::RightStickLeft,
    Button::RightStickRight,
];

/// A pad's three sets of direction buttons, each with the name help text
/// gives the whole set.
// check-text: not player text
pub const DIRECTION_SETS: [(&str, StickButtons); 3] = [
    ("D-pad", DPAD),
    ("L-stick", LEFT_STICK),
    ("R-stick", RIGHT_STICK),
];

/// What moves the cursor on a pad of `kind`, for help text. `bound` is the
/// buttons of the cursor's up, down, left and right. Every whole set of
/// directions among them is named, joined with `/` (`D-pad/L-stick` with the
/// defaults); if there is none, each direction's first button, in
/// up-left-down-right order. `None` if a direction has no button.
pub(super) fn cursor_buttons_name(kind: PadKind, bound: &[Vec<Button>; 4]) -> Option<String> {
    let [up, down, left, right] = bound.each_ref().map(|buttons| buttons.first());
    let first = [up?, left?, down?, right?];
    let whole: Vec<&str> = DIRECTION_SETS
        .iter()
        .filter(|(_, set)| set.iter().zip(bound).all(|(b, list)| list.contains(b)))
        .map(|&(name, _)| name)
        .collect();
    let names = if whole.is_empty() {
        first.map(|&b| kind.button_name(b)).to_vec()
    } else {
        whole
    };
    Some(names.join("/"))
}

/// The one direction button a stick at `(x, y)` holds, if any. `held` is
/// the one it held last frame.
///
/// A direction starts when the stick is pushed at least `press_percent` its
/// way, and it is the direction pushed furthest (left or right on a perfect
/// diagonal). It then stays held until the stick falls below
/// `release_percent` that way, or another direction is pushed further by
/// the gap between the two thresholds, so neither the edge of the dead zone
/// nor a diagonal flickers.
fn pushed(
    held: Option<Button>,
    (x, y): (f32, f32),
    [up, down, left, right]: StickButtons,
    def: StickDef,
) -> Option<Button> {
    let press = f32::from(def.press_percent) / 100.0;
    let release = f32::from(def.release_percent) / 100.0;
    // How far the stick is pushed each way. On a tie the last wins.
    let amounts = [(up, -y), (down, y), (left, -x), (right, x)];
    let along = |button| {
        let found = amounts.iter().find(|&&(b, _)| b == button);
        found.map_or(0.0, |&(_, amount)| amount)
    };
    let (furthest, amount) = amounts.iter().copied().max_by(|a, b| a.1.total_cmp(&b.1))?;
    let fresh = (amount >= press).then_some(furthest);
    let Some(held) = held.filter(|&h| along(h) >= release) else {
        return fresh;
    };
    match fresh {
        Some(new) if amount >= along(held) + (press - release) => Some(new),
        _ => Some(held),
    }
}

/// The buttons a pad of `kind` holds, by binding position, stick directions
/// included. `was` is what it held last frame (for the sticks).
fn resolve(was: ButtonSet, kind: PadKind, state: &PadState, stick: StickDef) -> ButtonSet {
    let mut down: ButtonSet = state.buttons.iter().map(|b| kind.position(b)).collect();
    for (buttons, at) in [
        (LEFT_STICK, state.left_stick),
        (RIGHT_STICK, state.right_stick),
    ] {
        let held = buttons.into_iter().find(|&b| was.contains(b));
        if let Some(button) = pushed(held, at, buttons, stick) {
            down.insert(button);
        }
    }
    down
}

/// What tells one connected pad from another: the platform's own number for
/// it, stable while the pad stays plugged in.
pub type PadId = usize;

/// Every connected pad, merged into one stream of button presses and
/// releases (see the [module docs](self)).
#[derive(Debug, Clone)]
pub struct Pads {
    stick: StickDef,
    /// Each connected pad's buttons held, by binding position.
    held: BTreeMap<PadId, ButtonSet>,
}

impl Pads {
    /// No pads yet, with the sticks' thresholds from the keymap.
    pub fn new(stick: StickDef) -> Self {
        Self {
            stick,
            held: BTreeMap::new(),
        }
    }

    /// Takes this frame's pads (`connected`: every pad plugged in, with its
    /// kind and state; a pad left out was removed) and returns what changed
    /// since the last call as `(button, pressed)`, releases first, each in
    /// [`Button::ALL`] order. Buttons are binding positions: feed them to
    /// [`InputState::pad_down`] and [`pad_up`].
    ///
    /// [`InputState::pad_down`]: super::InputState::pad_down
    /// [`pad_up`]: super::InputState::pad_up
    pub fn update(&mut self, connected: &[(PadId, PadKind, PadState)]) -> Vec<(Button, bool)> {
        let before = self.down();
        let mut held = BTreeMap::new();
        for (id, kind, state) in connected {
            let was = self.held.get(id).copied().unwrap_or_default();
            held.insert(*id, resolve(was, *kind, state, self.stick));
        }
        self.held = held;
        let after = self.down();
        let released = before.iter().filter(|&b| !after.contains(b));
        let pressed = after.iter().filter(|&b| !before.contains(b));
        released
            .map(|b| (b, false))
            .chain(pressed.map(|b| (b, true)))
            .collect()
    }

    /// The kind of the pad holding `button` (a binding position), the
    /// first of `connected` if several do: for a button [`update`] has
    /// just reported pressed, the pad that pressed it. `connected` is what
    /// `update` was given. `None` if no pad holds it.
    ///
    /// [`update`]: Self::update
    pub fn kind_holding(
        &self,
        button: Button,
        connected: &[(PadId, PadKind, PadState)],
    ) -> Option<PadKind> {
        let holds = |id| self.held.get(id).is_some_and(|set| set.contains(button));
        let pad = connected.iter().find(|(id, _, _)| holds(id));
        pad.map(|&(_, kind, _)| kind)
    }

    /// The buttons held on any pad, by binding position.
    pub fn down(&self) -> ButtonSet {
        self.held
            .values()
            .fold(ButtonSet::EMPTY, |all, &pad| all.union(pad))
    }
}

#[cfg(test)]
mod tests;
