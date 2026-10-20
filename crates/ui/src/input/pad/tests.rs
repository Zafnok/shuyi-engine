//! Tests of controller state handling: pad kinds and the Nintendo swap,
//! sticks as 4-way buttons with hysteresis, and merging several pads.

use proptest::prelude::*;

use super::*;
use crate::input::sample::{Action, InputState, Keymap, RIGHT_HANDED, load_keymap};
use Button::{
    CrankBack, CrankForward, DpadUp, East, LeftStickDown, LeftStickLeft, LeftStickRight,
    LeftStickUp, North, RightStickDown, RightStickLeft, RightStickRight, RightStickUp, South,
    Start, West,
};

fn stick() -> StickDef {
    StickDef {
        press_percent: 50,
        release_percent: 35,
    }
}

/// A pad holding `buttons`, sticks centred.
fn holding(buttons: &[Button]) -> PadState {
    PadState {
        buttons: buttons.iter().copied().collect(),
        ..PadState::default()
    }
}

/// A pad with its left stick at `(x, y)` and nothing else.
fn left_at(x: f32, y: f32) -> PadState {
    PadState {
        left_stick: (x, y),
        ..PadState::default()
    }
}

fn xbox(state: PadState) -> (PadId, PadKind, PadState) {
    (0, PadKind::Xbox, state)
}

fn pads() -> Pads {
    Pads::new(stick())
}

const KINDS: [PadKind; 6] = PadKind::ALL;

/// `docs/design/controls.md`, *Controller → Default buttons* and *Button
/// names on screen*: each position's name on each kind of pad.
#[test]
fn buttons_are_named_as_the_pad_in_use_labels_them() {
    use Button::{LeftShoulder, LeftTrigger, RightShoulder, RightTrigger, Select};
    let table = [
        (South, "A", "✕", "A"),
        (East, "B", "◯", "B"),
        (West, "X", "□", "Y"),
        (North, "Y", "△", "X"),
        (LeftShoulder, "LB", "L1", "L"),
        (RightShoulder, "RB", "R1", "R"),
        (LeftTrigger, "LT", "L2", "ZL"),
        (RightTrigger, "RT", "R2", "ZR"),
        (Select, "Back", "Create", "−"),
        (Start, "Start", "Options", "+"),
        (Button::LeftStickPress, "LS", "L3", "LS"),
        (Button::RightStickPress, "RS", "R3", "RS"),
        (DpadUp, "↑", "↑", "↑"),
        (Button::DpadDown, "↓", "↓", "↓"),
        (Button::DpadLeft, "←", "←", "←"),
        (Button::DpadRight, "→", "→", "→"),
        (LeftStickUp, "L-stick ↑", "L-stick ↑", "L-stick ↑"),
        (LeftStickDown, "L-stick ↓", "L-stick ↓", "L-stick ↓"),
        (LeftStickLeft, "L-stick ←", "L-stick ←", "L-stick ←"),
        (LeftStickRight, "L-stick →", "L-stick →", "L-stick →"),
        (RightStickUp, "R-stick ↑", "R-stick ↑", "R-stick ↑"),
        (RightStickDown, "R-stick ↓", "R-stick ↓", "R-stick ↓"),
        (RightStickLeft, "R-stick ←", "R-stick ←", "R-stick ←"),
        (RightStickRight, "R-stick →", "R-stick →", "R-stick →"),
        (CrankBack, "Crank back", "Crank back", "Crank back"),
        (
            CrankForward,
            "Crank forward",
            "Crank forward",
            "Crank forward",
        ),
        (
            Button::SystemMenu0,
            "Menu item 1",
            "Menu item 1",
            "Menu item 1",
        ),
        (
            Button::SystemMenu1,
            "Menu item 2",
            "Menu item 2",
            "Menu item 2",
        ),
        (
            Button::SystemMenu2,
            "Menu item 3",
            "Menu item 3",
            "Menu item 3",
        ),
        (
            Button::SystemMenu3,
            "Menu item 4",
            "Menu item 4",
            "Menu item 4",
        ),
        (
            Button::SystemMenu4,
            "Menu item 5",
            "Menu item 5",
            "Menu item 5",
        ),
        (
            Button::SystemMenu5,
            "Menu item 6",
            "Menu item 6",
            "Menu item 6",
        ),
        (
            Button::SystemMenu6,
            "Menu item 7",
            "Menu item 7",
            "Menu item 7",
        ),
        (
            Button::SystemMenu7,
            "Menu item 8",
            "Menu item 8",
            "Menu item 8",
        ),
    ];
    assert_eq!(table.map(|row| row.0), Button::ALL);
    for (button, xbox, playstation, nintendo) in table {
        assert_eq!(PadKind::Xbox.button_name(button), xbox, "{button}");
        // Steam Deck and unknown pads carry the Xbox letters.
        assert_eq!(PadKind::Generic.button_name(button), xbox, "{button}");
        assert_eq!(
            PadKind::PlayStation.button_name(button),
            playstation,
            "{button}"
        );
        assert_eq!(PadKind::Nintendo.button_name(button), nintendo, "{button}");
    }
}

/// `controls.md`, *Button names on screen*: a PS4 pad's left centre
/// button is `Share`; every other name is a PS5 pad's.
#[test]
fn a_dualshock_4_says_share_and_otherwise_names_as_a_playstation_pad() {
    for &button in Button::ALL {
        let expected = match button {
            Button::Select => "Share",
            other => PadKind::PlayStation.button_name(other),
        };
        assert_eq!(PadKind::PlayStation4.button_name(button), expected);
    }
    assert_eq!(PadKind::PlayStation.button_name(Button::Select), "Create");
}

/// The Playdate's buttons are named as printed on it (its A, B and
/// D-pad), and its front end reports A and B at the positions the default
/// bindings give Confirm and Cancel, so no swap: A confirms, B cancels
/// (`docs/design/playdate.md`, *Buttons and the crank*).
#[test]
fn a_playdates_a_confirms_and_its_b_cancels_named_as_printed_on_it() {
    let playdate = PadKind::Playdate;
    for button in Button::ALL {
        assert_eq!(playdate.position(*button), *button, "{button}");
    }
    assert_eq!(playdate.button_name(South), "A");
    assert_eq!(playdate.button_name(East), "B");
    let dpad = [
        DpadUp,
        Button::DpadDown,
        Button::DpadLeft,
        Button::DpadRight,
    ];
    assert_eq!(dpad.map(|b| playdate.button_name(b)), ["↑", "↓", "←", "→"]);
    let buttons = [South, East, DpadUp, Button::DpadDown, Button::DpadLeft];
    assert_eq!(
        actions_of(playdate, &buttons),
        [
            vec![Action::Confirm],
            vec![Action::Cancel],
            vec![Action::CursorUp],
            vec![Action::CursorDown],
            vec![Action::CursorLeft],
        ]
    );
    let dpad = dpad.map(|b| vec![b]);
    assert_eq!(
        cursor_buttons_name(playdate, &dpad).as_deref(),
        Some("D-pad")
    );
}

/// On a Nintendo pad the physical right button is the binding position
/// `South`, so Confirm's default button is the one labelled `A` there, as
/// in Fire Emblem on Switch.
#[test]
fn a_nintendo_pads_right_button_is_its_a() {
    let nintendo = PadKind::Nintendo;
    assert_eq!(nintendo.button_name(nintendo.position(East)), "A");
    assert_eq!(nintendo.button_name(nintendo.position(South)), "B");
    assert_eq!(nintendo.button_name(nintendo.position(North)), "X");
    assert_eq!(nintendo.button_name(nintendo.position(West)), "Y");
}

/// Within one kind of pad, no two buttons share a name.
#[test]
fn button_names_are_distinct_on_each_pad() {
    for kind in KINDS {
        let mut names: Vec<&str> = Button::ALL.iter().map(|&b| kind.button_name(b)).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), Button::ALL.len(), "{kind:?}");
    }
}

#[test]
fn the_sets_of_directions_are_each_up_down_left_right() {
    let sets = DIRECTION_SETS.map(|(name, set)| (name, set.map(Button::name)));
    assert_eq!(
        sets,
        [
            ("D-pad", ["DpadUp", "DpadDown", "DpadLeft", "DpadRight"]),
            (
                "L-stick",
                [
                    "LeftStickUp",
                    "LeftStickDown",
                    "LeftStickLeft",
                    "LeftStickRight"
                ]
            ),
            (
                "R-stick",
                [
                    "RightStickUp",
                    "RightStickDown",
                    "RightStickLeft",
                    "RightStickRight"
                ]
            ),
        ]
    );
}

#[test]
fn the_pad_holding_a_button_is_the_first_connected_that_does() {
    let mut p = pads();
    let sony = PadKind::PlayStation;
    let connected = [
        (4, PadKind::Nintendo, holding(&[West])),
        (2, sony, holding(&[South, West])),
        (9, PadKind::Xbox, left_at(-1.0, 0.0)),
    ];
    p.update(&connected);
    // A Nintendo pad's left button is still `West`; its `South` would be
    // its right button.
    assert_eq!(p.kind_holding(West, &connected), Some(PadKind::Nintendo));
    assert_eq!(p.kind_holding(South, &connected), Some(sony));
    assert_eq!(
        p.kind_holding(LeftStickLeft, &connected),
        Some(PadKind::Xbox)
    );
    assert_eq!(p.kind_holding(North, &connected), None);
    // A pad `update` hasn't seen holds nothing yet.
    let late = [(5, PadKind::Xbox, holding(&[North]))];
    assert_eq!(p.kind_holding(North, &late), None);
    assert_eq!(pads().kind_holding(South, &connected), None);
}

#[test]
fn usb_ids_give_the_pad_kind() {
    // Other makers: the product doesn't matter (DualShock 4 ids included).
    for product in [0, 0x05c4, 0x09cc, 0x0ba0, 0x0ce6, 0x02e0, 0x2009] {
        assert_eq!(PadKind::from_ids(0x045e, product), PadKind::Xbox);
        assert_eq!(PadKind::from_ids(0x057e, product), PadKind::Nintendo);
        // Unknown makers (8BitDo, Valve) and "not reported".
        for vendor in [0x2dc8, 0x28de, 0, 0xffff, 0x045f, 0x057d] {
            assert_eq!(
                PadKind::from_ids(vendor, product),
                PadKind::Generic,
                "{vendor:#06x} {product:#06x}"
            );
        }
    }
    // Sony: the DualShock 4's two models and its wireless adaptor...
    for product in [0x05c4, 0x09cc, 0x0ba0] {
        assert_eq!(
            PadKind::from_ids(0x054c, product),
            PadKind::PlayStation4,
            "{product:#06x}"
        );
    }
    // ...and everything else (a DualSense, "not reported", near misses).
    for product in [0x0ce6, 0, 0x05c5, 0x09cb, 0x0ba1, 0xffff] {
        assert_eq!(
            PadKind::from_ids(0x054c, product),
            PadKind::PlayStation,
            "{product:#06x}"
        );
    }
    assert_eq!(PadKind::default(), PadKind::Generic);
}

#[test]
fn only_nintendo_pads_swap_and_only_the_bottom_and_right_buttons() {
    for &button in Button::ALL {
        for kind in [
            PadKind::Xbox,
            PadKind::PlayStation,
            PadKind::PlayStation4,
            PadKind::Generic,
        ] {
            assert_eq!(kind.position(button), button, "{kind:?} {button}");
        }
        let expected = match button {
            South => East,
            East => South,
            other => other,
        };
        assert_eq!(PadKind::Nintendo.position(button), expected, "{button}");
    }
}

#[test]
fn button_sets_hold_buttons() {
    let mut set = ButtonSet::EMPTY;
    assert_eq!(set, ButtonSet::default());
    assert_eq!(set.iter().count(), 0);
    set.insert(Start);
    set.insert(South);
    set.insert(Start);
    assert!(set.contains(South) && set.contains(Start));
    assert!(!set.contains(East));
    // In `Button::ALL` order, whatever the insertion order.
    assert_eq!(set.iter().collect::<Vec<_>>(), [South, Start]);
    let other: ButtonSet = [East, South].into_iter().collect();
    assert_eq!(
        set.union(other).iter().collect::<Vec<_>>(),
        [South, East, Start]
    );
    // Every button has its own bit.
    let all: ButtonSet = Button::ALL.iter().copied().collect();
    assert_eq!(all.iter().collect::<Vec<_>>(), Button::ALL);
    for &button in Button::ALL {
        let one: ButtonSet = [button].into_iter().collect();
        assert_eq!(one.iter().collect::<Vec<_>>(), [button]);
    }
}

#[test]
fn the_standard_mapping_fills_a_pad_state() {
    // Bit i is `Button::ALL[i]` for the 16 standard buttons.
    for (i, &button) in Button::ALL[..16].iter().enumerate() {
        let state = PadState::from_standard(1 << i, [0.0; 4]);
        assert_eq!(state.buttons.iter().collect::<Vec<_>>(), [button], "{i}");
    }
    // Bit 16 (the Home button) and above aren't buttons of ours: they
    // must not turn into stick directions.
    let state = PadState::from_standard(u32::MAX << 16, [0.0; 4]);
    assert_eq!(state.buttons, ButtonSet::EMPTY);
    let state = PadState::from_standard(0b1001, [0.1, 0.2, 0.3, 0.4]);
    assert_eq!(state.buttons.iter().collect::<Vec<_>>(), [South, North]);
    assert_eq!(state.left_stick, (0.1, 0.2));
    assert_eq!(state.right_stick, (0.3, 0.4));
}

#[test]
fn a_press_and_a_release_are_one_event_each() {
    let mut p = pads();
    assert_eq!(p.update(&[xbox(holding(&[]))]), []);
    assert_eq!(p.update(&[xbox(holding(&[South]))]), [(South, true)]);
    assert_eq!(p.down().iter().collect::<Vec<_>>(), [South]);
    // Still held: nothing new.
    assert_eq!(p.update(&[xbox(holding(&[South]))]), []);
    assert_eq!(p.update(&[xbox(holding(&[]))]), [(South, false)]);
    assert_eq!(p.down(), ButtonSet::EMPTY);
    assert_eq!(p.update(&[xbox(holding(&[]))]), []);
}

#[test]
fn releases_come_before_presses_each_in_button_order() {
    let mut p = pads();
    assert_eq!(
        p.update(&[xbox(holding(&[Start, West, South]))]),
        [(South, true), (West, true), (Start, true)]
    );
    assert_eq!(
        p.update(&[xbox(holding(&[DpadUp, East, West]))]),
        [(South, false), (Start, false), (East, true), (DpadUp, true)]
    );
}

#[test]
fn the_stick_presses_at_the_press_threshold() {
    for (x, y, button) in [
        (0.5, 0.0, LeftStickRight),
        (-0.5, 0.0, LeftStickLeft),
        (0.0, 0.5, LeftStickDown),
        (0.0, -0.5, LeftStickUp),
        (1.0, 0.0, LeftStickRight),
    ] {
        let mut p = pads();
        assert_eq!(
            p.update(&[xbox(left_at(x, y))]),
            [(button, true)],
            "{x} {y}"
        );
    }
    // The dead zone: just short of the threshold in every direction.
    for (x, y) in [
        (0.49, 0.0),
        (-0.49, 0.0),
        (0.0, 0.49),
        (0.0, -0.49),
        (0.0, 0.0),
    ] {
        let mut p = pads();
        assert_eq!(p.update(&[xbox(left_at(x, y))]), [], "{x} {y}");
    }
}

#[test]
fn the_right_stick_has_its_own_direction_buttons() {
    for (x, y, button) in [
        (0.9, 0.0, RightStickRight),
        (-0.9, 0.0, RightStickLeft),
        (0.0, 0.9, RightStickDown),
        (0.0, -0.9, RightStickUp),
    ] {
        let mut p = pads();
        let state = PadState {
            right_stick: (x, y),
            ..PadState::default()
        };
        assert_eq!(p.update(&[xbox(state)]), [(button, true)], "{x} {y}");
    }
    // Both sticks at once: one direction each.
    let mut p = pads();
    let state = PadState {
        left_stick: (0.0, -1.0),
        right_stick: (1.0, 0.0),
        ..PadState::default()
    };
    assert_eq!(
        p.update(&[xbox(state)]),
        [(LeftStickUp, true), (RightStickRight, true)]
    );
}

#[test]
fn a_held_stick_direction_lasts_until_below_the_release_threshold() {
    let mut p = pads();
    assert_eq!(
        p.update(&[xbox(left_at(0.6, 0.0))]),
        [(LeftStickRight, true)]
    );
    // Between release and press: no flicker, whichever way it wobbles.
    for x in [0.49, 0.36, 0.45, 0.35, 0.5, 0.4] {
        assert_eq!(p.update(&[xbox(left_at(x, 0.0))]), [], "{x}");
    }
    assert_eq!(
        p.update(&[xbox(left_at(0.34, 0.0))]),
        [(LeftStickRight, false)]
    );
    // Released, the same wobble doesn't press it again...
    for x in [0.36, 0.45, 0.49, 0.35] {
        assert_eq!(p.update(&[xbox(left_at(x, 0.0))]), [], "{x}");
    }
    // ...until it reaches the press threshold.
    assert_eq!(
        p.update(&[xbox(left_at(0.5, 0.0))]),
        [(LeftStickRight, true)]
    );
}

#[test]
fn the_thresholds_come_from_the_keymap() {
    let mut p = Pads::new(StickDef {
        press_percent: 80,
        release_percent: 10,
    });
    assert_eq!(p.update(&[xbox(left_at(0.79, 0.0))]), []);
    assert_eq!(
        p.update(&[xbox(left_at(0.8, 0.0))]),
        [(LeftStickRight, true)]
    );
    assert_eq!(p.update(&[xbox(left_at(0.1, 0.0))]), []);
    assert_eq!(
        p.update(&[xbox(left_at(0.09, 0.0))]),
        [(LeftStickRight, false)]
    );
}

#[test]
fn a_diagonal_push_is_one_direction_the_one_pushed_furthest() {
    for (x, y, button) in [
        (0.9, -0.6, LeftStickRight),
        (0.6, -0.9, LeftStickUp),
        (-0.9, 0.6, LeftStickLeft),
        (-0.6, 0.9, LeftStickDown),
        // A perfect diagonal counts as left or right.
        (0.8, 0.8, LeftStickRight),
        (0.8, -0.8, LeftStickRight),
        (-0.8, 0.8, LeftStickLeft),
        (-0.8, -0.8, LeftStickLeft),
    ] {
        let mut p = pads();
        assert_eq!(
            p.update(&[xbox(left_at(x, y))]),
            [(button, true)],
            "{x} {y}"
        );
    }
}

#[test]
fn a_held_direction_gives_way_only_to_one_pushed_clearly_further() {
    let mut p = pads();
    assert_eq!(
        p.update(&[xbox(left_at(0.6, 0.0))]),
        [(LeftStickRight, true)]
    );
    // Up is now pushed further, but not by the gap between the thresholds
    // (0.15): right stays, so a wobbling diagonal doesn't flicker.
    assert_eq!(p.update(&[xbox(left_at(0.6, -0.74))]), []);
    assert_eq!(p.update(&[xbox(left_at(0.6, -0.6))]), []);
    // Clearly further: up takes over in one frame.
    assert_eq!(
        p.update(&[xbox(left_at(0.6, -0.76))]),
        [(LeftStickRight, false), (LeftStickUp, true)]
    );
    // And back the same way.
    assert_eq!(p.update(&[xbox(left_at(0.9, -0.76))]), []);
    assert_eq!(
        p.update(&[xbox(left_at(0.92, -0.76))]),
        [(LeftStickUp, false), (LeftStickRight, true)]
    );
}

#[test]
fn rolling_the_stick_round_changes_direction_without_centring() {
    let mut p = pads();
    assert_eq!(
        p.update(&[xbox(left_at(1.0, 0.0))]),
        [(LeftStickRight, true)]
    );
    // Rolled from right to up: right has fallen below release.
    assert_eq!(
        p.update(&[xbox(left_at(0.2, -0.98))]),
        [(LeftStickRight, false), (LeftStickUp, true)]
    );
    // A held direction that falls below release while nothing else is
    // pushed far enough just ends.
    assert_eq!(
        p.update(&[xbox(left_at(0.3, -0.3))]),
        [(LeftStickUp, false)]
    );
}

#[test]
fn a_broken_stick_reading_presses_nothing() {
    for (x, y) in [(f32::NAN, 0.0), (0.0, f32::NAN), (f32::NAN, f32::NAN)] {
        let mut p = pads();
        assert_eq!(p.update(&[xbox(left_at(x, y))]), [], "{x} {y}");
        // And it ends a held direction.
        assert_eq!(
            p.update(&[xbox(left_at(1.0, 0.0))]),
            [(LeftStickRight, true)]
        );
        assert_eq!(
            p.update(&[xbox(left_at(x, y))]),
            [(LeftStickRight, false)],
            "{x} {y}"
        );
    }
}

#[test]
fn a_removed_pad_releases_what_it_held() {
    let mut p = pads();
    let held = PadState {
        buttons: [South, Start].into_iter().collect(),
        left_stick: (0.0, 1.0),
        ..PadState::default()
    };
    assert_eq!(
        p.update(&[xbox(held)]),
        [(South, true), (Start, true), (LeftStickDown, true)]
    );
    // Unplugged mid-press.
    assert_eq!(
        p.update(&[]),
        [(South, false), (Start, false), (LeftStickDown, false)]
    );
    assert_eq!(p.down(), ButtonSet::EMPTY);
    assert_eq!(p.update(&[]), []);
    // Plugged back in still holding: pressed again, and the stick starts
    // from scratch (it needs the press threshold, not the release one).
    assert_eq!(p.update(&[xbox(left_at(0.0, 0.4))]), []);
    assert_eq!(
        p.update(&[xbox(held)]),
        [(South, true), (Start, true), (LeftStickDown, true)]
    );
}

#[test]
fn two_pads_holding_one_button_are_one_press() {
    let mut p = pads();
    let a = |state| (3, PadKind::Xbox, state);
    let b = |state| (7, PadKind::PlayStation, state);
    assert_eq!(
        p.update(&[a(holding(&[South])), b(holding(&[]))]),
        [(South, true)]
    );
    // The second pad joins in: no second press.
    assert_eq!(p.update(&[a(holding(&[South])), b(holding(&[South]))]), []);
    // The first lets go: still held by the second.
    assert_eq!(p.update(&[a(holding(&[])), b(holding(&[South]))]), []);
    assert_eq!(
        p.update(&[a(holding(&[])), b(holding(&[]))]),
        [(South, false)]
    );
    // Different buttons on different pads are separate presses, and
    // unplugging one pad releases only what the other doesn't hold.
    assert_eq!(
        p.update(&[a(holding(&[South, West])), b(holding(&[South, North]))]),
        [(South, true), (West, true), (North, true)]
    );
    assert_eq!(p.update(&[b(holding(&[South, North]))]), [(West, false)]);
}

#[test]
fn each_pad_keeps_its_own_stick_state() {
    let mut p = pads();
    let a = |x| (0, PadKind::Xbox, left_at(x, 0.0));
    let b = |x| (1, PadKind::Xbox, left_at(x, 0.0));
    assert_eq!(p.update(&[a(0.6), b(0.0)]), [(LeftStickRight, true)]);
    // Pad 1 at 0.4 never pressed, so it isn't held; pad 0 at 0.4 still is.
    assert_eq!(p.update(&[a(0.4), b(0.4)]), []);
    assert_eq!(p.update(&[a(0.0), b(0.4)]), [(LeftStickRight, false)]);
}

#[test]
fn a_nintendo_pad_reports_its_right_button_as_south() {
    let mut p = pads();
    let switch = |state| (0, PadKind::Nintendo, state);
    assert_eq!(p.update(&[switch(holding(&[East]))]), [(South, true)]);
    assert_eq!(
        p.update(&[switch(holding(&[South]))]),
        [(South, false), (East, true)]
    );
    assert_eq!(
        p.update(&[switch(holding(&[South, West, North, Start]))]),
        [(West, true), (North, true), (Start, true)]
    );
    // Its right button and an Xbox pad's bottom button are the same press.
    let mut p = pads();
    assert_eq!(
        p.update(&[
            switch(holding(&[East])),
            (1, PadKind::Xbox, holding(&[South]))
        ]),
        [(South, true)]
    );
}

/// The embedded keymap's right-handed layout with its default buttons.
fn default_input() -> InputState {
    let def = load_keymap().unwrap_or_else(|e| panic!("{e:?}"));
    InputState::new(Keymap::for_layout(&def, RIGHT_HANDED))
}

/// The actions a pad of `kind` gives for pressing each of `buttons` alone.
fn actions_of(kind: PadKind, buttons: &[Button]) -> Vec<Vec<Action>> {
    let mut input = default_input();
    let mut p = pads();
    let mut frame = |state: PadState| {
        for (button, pressed) in p.update(&[(0, kind, state)]) {
            if pressed {
                input.pad_down(button, kind);
            } else {
                input.pad_up(button);
            }
        }
        input.update(0.0)
    };
    let mut seen = Vec::new();
    for &button in buttons {
        seen.push(frame(holding(&[button])));
        assert_eq!(frame(holding(&[])), [], "{button}");
    }
    seen
}

/// `controls.md`: on Switch-style pads the right button confirms and the
/// bottom one cancels; every other button is as on any pad.
#[test]
fn on_a_nintendo_pad_east_confirms_and_south_cancels() {
    let buttons = [South, East, North, West, Start, DpadUp];
    let others = [
        vec![Action::Info],
        // Another key group's action shares the button.
        vec![Action::DangerZone, Action::DialogueLog],
        vec![Action::EndTurn],
        vec![Action::CursorUp],
    ];
    for kind in [
        PadKind::Xbox,
        PadKind::PlayStation,
        PadKind::PlayStation4,
        PadKind::Generic,
    ] {
        let seen = actions_of(kind, &buttons);
        assert_eq!(
            seen[..2],
            [vec![Action::Confirm], vec![Action::Cancel]],
            "{kind:?}"
        );
        assert_eq!(seen[2..], others, "{kind:?}");
    }
    let seen = actions_of(PadKind::Nintendo, &buttons);
    assert_eq!(seen[..2], [vec![Action::Cancel], vec![Action::Confirm]]);
    assert_eq!(seen[2..], others);
}

/// The left stick moves the cursor one step per push and repeats like a
/// held key; the right stick does nothing by default.
#[test]
fn the_left_stick_moves_the_cursor_and_the_right_stick_does_nothing() {
    let mut input = default_input();
    let mut p = pads();
    let mut frame = |state: PadState, dt: f32| {
        for (button, pressed) in p.update(&[xbox(state)]) {
            if pressed {
                input.pad_down(button, PadKind::Xbox);
            } else {
                input.pad_up(button);
            }
        }
        input.update(dt)
    };
    assert_eq!(frame(left_at(0.0, -1.0), 0.0), [Action::CursorUp]);
    // Held for the repeat delay (300 ms): one repeat.
    assert_eq!(frame(left_at(0.0, -0.9), 0.3), [Action::CursorUp]);
    assert_eq!(frame(left_at(0.0, 0.0), 1.0), []);
    let right = PadState {
        right_stick: (1.0, 0.0),
        ..PadState::default()
    };
    assert_eq!(frame(right, 1.0), []);
}

fn arb_state() -> impl Strategy<Value = PadState> {
    let axis = || prop_oneof![Just(0.0f32), -1.0f32..=1.0];
    (0u32..(1 << 16), axis(), axis(), axis(), axis())
        .prop_map(|(buttons, lx, ly, rx, ry)| PadState::from_standard(buttons, [lx, ly, rx, ry]))
}

fn arb_kind() -> impl Strategy<Value = PadKind> {
    prop::sample::select(vec![
        PadKind::Xbox,
        PadKind::PlayStation,
        PadKind::PlayStation4,
        PadKind::Nintendo,
        PadKind::Generic,
    ])
}

proptest! {
    /// Whatever the pads do, the events are exactly the changes of
    /// [`Pads::down`]: replaying them tracks it, no button is pressed while
    /// down or released while up, and each stick holds at most one
    /// direction per pad.
    #[test]
    fn events_track_what_is_down(
        frames in prop::collection::vec(
            prop::collection::vec((0usize..3, arb_kind(), arb_state()), 0..3),
            0..40,
        ),
    ) {
        let mut p = pads();
        let mut down = std::collections::BTreeSet::new();
        for connected in &frames {
            for (button, pressed) in p.update(connected) {
                if pressed {
                    prop_assert!(down.insert(button), "{button} pressed twice");
                } else {
                    prop_assert!(down.remove(&button), "{button} released while up");
                }
            }
            prop_assert_eq!(p.down().iter().collect::<Vec<_>>(), down.iter().copied().collect::<Vec<_>>());
            if connected.is_empty() {
                prop_assert!(down.is_empty());
            }
            if let [(_, _, _)] = connected.as_slice() {
                for buttons in [LEFT_STICK, RIGHT_STICK] {
                    let held = buttons.iter().filter(|b| down.contains(b)).count();
                    prop_assert!(held <= 1, "{held} directions on one stick");
                }
            }
        }
    }
}
