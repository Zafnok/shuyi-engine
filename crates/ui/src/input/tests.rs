//! The input mechanism's tests, over the sample game's actions
//! ([`sample`](super::sample)).
use core::time::Duration;

use proptest::prelude::*;

use super::sample::{
    Action, InputState, Keymap, KeymapDef, LEFT_HANDED, LayoutBindings, PlayerKeys, RIGHT_HANDED,
    load_keymap,
};
use super::state::seconds_to_micros;
use super::*;
use Action::{Confirm, CursorDown, CursorLeft, CursorRight, CursorUp, Info};

fn chord(s: &str) -> Chord {
    Chord::parse(s).unwrap_or(Chord::plain(Key::F1))
}

/// A fixed test keymap, independent of the shipped defaults.
fn test_keymap(delay_ms: u32, interval_ms: u32) -> Keymap {
    let bindings = [
        ("h", CursorLeft),
        ("Left", CursorLeft),
        ("j", CursorDown),
        ("k", CursorUp),
        ("l", CursorRight),
        ("f", Confirm),
        ("Shift+h", Info),
    ]
    .into_iter()
    .map(|(c, a)| (chord(c), a));
    Keymap::new(
        bindings,
        RepeatDef {
            delay_ms,
            interval_ms,
        },
    )
}

fn default_state() -> InputState {
    state_with(170, 55)
}

fn state_with(delay_ms: u32, interval_ms: u32) -> InputState {
    InputState::new(test_keymap(delay_ms, interval_ms))
}

/// The kind of pad the tests press buttons on.
const PAD: PadKind = PadKind::Xbox;

/// The buttons [`pad_state`] adds to the test keymap.
const TEST_PAD: [(Button, Action); 5] = [
    (Button::DpadLeft, CursorLeft),
    (Button::DpadDown, CursorDown),
    (Button::LeftStickDown, CursorDown),
    (Button::DpadRight, CursorRight),
    (Button::South, Confirm),
];

/// [`default_state`] with a few controller buttons bound too.
fn pad_state() -> InputState {
    InputState::new(test_keymap(170, 55).with_pad(TEST_PAD))
}

/// Seconds for `ms` milliseconds.
fn ms(ms: u32) -> f32 {
    Duration::from_millis(u64::from(ms)).as_secs_f32()
}

#[test]
fn only_the_plain_fixed_chords_abort_capture_and_clear_a_slot() {
    assert!(is_capture_abort(chord("Escape")));
    assert!(!is_capture_abort(chord("Shift+Escape")));
    assert!(!is_capture_abort(chord("Delete")));
    assert!(is_clear_slot(chord("Delete")));
    assert!(!is_clear_slot(chord("Shift+Delete")));
    assert!(!is_clear_slot(chord("Escape")));
    assert_eq!(capture_abort_key_name(), "Escape");
    assert_eq!(clear_slot_key_name(), "Delete");
    // Both are reserved, so neither can end up in a slot.
    assert!(chord("Escape").is_reserved() && chord("Delete").is_reserved());
}

#[test]
fn keymap_lookup_with_and_without_shift() {
    let km = default_state().keymap().clone();
    assert_eq!(km.action(chord("h")), Some(CursorLeft));
    assert_eq!(km.action(chord("Shift+h")), Some(Info));
    assert_eq!(km.action(chord("Left")), Some(CursorLeft));
    assert_eq!(km.action(chord("Shift+Left")), None);
    assert_eq!(km.action(chord("Shift+f")), None);
    assert_eq!(km.action(chord("z")), None);
    assert_eq!(
        km.repeat(),
        RepeatDef {
            delay_ms: 170,
            interval_ms: 55
        }
    );
    let custom = state_with(400, 40);
    assert_eq!(
        custom.keymap().repeat(),
        RepeatDef {
            delay_ms: 400,
            interval_ms: 40
        }
    );
}

#[test]
fn chords_for_lists_every_binding_in_order() {
    let km = default_state().keymap().clone();
    assert_eq!(km.chords_for(CursorLeft), vec![chord("h"), chord("Left")]);
    assert_eq!(km.chords_for(Info), vec![chord("Shift+h")]);
    assert_eq!(km.chords_for(Action::Cancel), vec![]);
    assert_eq!(km.primary(CursorLeft), Some(chord("h")));
    assert_eq!(km.primary(Confirm), Some(chord("f")));
    assert_eq!(km.primary(Action::Cancel), None);
}

#[test]
fn with_pad_looks_buttons_up_and_lists_them_in_order() {
    let km = pad_state().keymap().clone();
    assert_eq!(km.pad_action(Button::South), Some(Confirm));
    assert_eq!(km.pad_action(Button::LeftStickDown), Some(CursorDown));
    assert_eq!(km.pad_action(Button::East), None);
    assert_eq!(
        km.buttons_for(CursorDown),
        vec![Button::DpadDown, Button::LeftStickDown]
    );
    assert!(km.buttons_for(Info).is_empty());
    // The keys are untouched, and a keymap has no buttons by itself.
    assert_eq!(km.action(chord("f")), Some(Confirm));
    assert_eq!(km.chords_for(CursorLeft), vec![chord("h"), chord("Left")]);
    let keys_only = test_keymap(170, 55);
    assert_eq!(keys_only.pad_action(Button::South), None);
    assert!(keys_only.buttons_for(Confirm).is_empty());
    assert_ne!(km, keys_only);
}

#[test]
fn a_repeated_button_keeps_its_last_action_and_with_pad_replaces_the_table() {
    let km = test_keymap(170, 55).with_pad([
        (Button::South, Confirm),
        (Button::East, Confirm),
        (Button::North, Info),
        (Button::South, Info),
    ]);
    assert_eq!(km.buttons_for(Confirm), vec![Button::East]);
    assert_eq!(km.buttons_for(Info), vec![Button::North, Button::South]);
    assert_eq!(km.pad_action(Button::South), Some(Info));
    let km = km.with_pad([(Button::West, Confirm)]);
    assert_eq!(km.buttons_for(Confirm), vec![Button::West]);
    assert!(km.buttons_for(Info).is_empty());
    assert_eq!(km.pad_action(Button::South), None);
    assert_eq!(km.pad_action(Button::East), None);
}

#[test]
fn every_keymap_from_the_definition_has_the_default_buttons() {
    let def = load_keymap().unwrap_or_default();
    let keymaps = [
        Keymap::for_layout(&def, RIGHT_HANDED),
        Keymap::for_layout(&def, LEFT_HANDED),
        Keymap::layout_picker(&def),
        PlayerKeys::default().keymap(&def, LEFT_HANDED),
        Keymap::new([], def.repeat).with_default_pad(&def),
    ];
    for km in keymaps {
        assert_eq!(km.pad_action(Button::South), Some(Confirm));
        assert_eq!(km.pad_action(Button::East), Some(Action::Cancel));
        assert_eq!(km.pad_action(Button::Start), Some(Action::EndTurn));
        assert_eq!(
            km.buttons_for(CursorUp),
            vec![Button::DpadUp, Button::LeftStickUp]
        );
        // Unused by default.
        assert_eq!(km.pad_action(Button::RightStickPress), None);
        assert_eq!(km.pad_action(Button::RightStickUp), None);
        assert!(km.buttons_for(Action::Debug).is_empty());
        for action in Action::ALL {
            assert_eq!(km.buttons_for(action), def.buttons(action), "{action}");
        }
    }
    // A layout's keys alone carry no buttons.
    let keys = LayoutBindings::defaults(&def, RIGHT_HANDED).keymap(def.repeat);
    assert_eq!(keys.pad_action(Button::South), None);
    assert_eq!(keys.clone().with_default_pad(&def), {
        PlayerKeys::default().keymap(&def, RIGHT_HANDED)
    });
    // A definition built by hand without buttons gives none.
    let empty = Keymap::for_layout(&KeymapDef::default(), LEFT_HANDED);
    assert_eq!(empty.pad_action(Button::South), None);
}

#[test]
fn for_layout_picks_that_layouts_bindings() {
    let def = load_keymap().unwrap_or_default();
    let right = Keymap::for_layout(&def, RIGHT_HANDED);
    let left = Keymap::for_layout(&def, LEFT_HANDED);
    assert_eq!(right.action(chord("f")), Some(Confirm));
    assert_eq!(right.action(chord("j")), None);
    assert_eq!(left.action(chord("j")), Some(Confirm));
    assert_eq!(left.action(chord("f")), None);
    assert_eq!(left.action(chord(";")), Some(Action::PrevUnit));
    assert_eq!(left.repeat(), def.repeat);
    assert_eq!(right.chords_for(Confirm).len(), 1);
    let empty = Keymap::for_layout(&KeymapDef::default(), LEFT_HANDED);
    assert_eq!(empty.primary(Confirm), None);
}

#[test]
fn escape_always_cancels_and_delete_does_nothing() {
    let def = load_keymap().unwrap_or_default();
    let keymaps = [
        Keymap::for_layout(&def, RIGHT_HANDED),
        Keymap::for_layout(&def, LEFT_HANDED),
        Keymap::layout_picker(&def),
        Keymap::new([], RepeatDef::default()),
        // Even a keymap that tries to bind them elsewhere.
        Keymap::new(
            [(chord("Escape"), Confirm), (chord("Delete"), Confirm)],
            RepeatDef::default(),
        ),
    ];
    for km in keymaps {
        assert_eq!(km.action(chord("Escape")), Some(Action::Cancel));
        assert_eq!(km.action(chord("Shift+Escape")), None);
        assert_eq!(km.action(chord("Delete")), None);
        assert_eq!(km.action(chord("Shift+Delete")), None);
        assert!(!km.chords_for(Action::Cancel).contains(&chord("Escape")));
        assert!(!km.chords_for(Confirm).contains(&chord("Escape")));
    }
    // Shifted, they are ordinary chords.
    let km = Keymap::new(
        [
            (chord("Shift+Escape"), Info),
            (chord("Shift+Delete"), Confirm),
        ],
        RepeatDef::default(),
    );
    assert_eq!(km.action(chord("Shift+Escape")), Some(Info));
    assert_eq!(km.action(chord("Shift+Delete")), Some(Confirm));
    assert_eq!(km.action(chord("Escape")), Some(Action::Cancel));
    assert_eq!(km.chords_for(Info), vec![chord("Shift+Escape")]);
    assert_eq!(
        Keymap::fixed_chords_for(Action::Cancel),
        vec![chord("Escape")]
    );
    assert!(
        Action::ALL
            .into_iter()
            .filter(|&a| a != Action::Cancel)
            .all(|a| Keymap::fixed_chords_for(a).is_empty())
    );
}

#[test]
fn chords_keep_the_order_given_and_a_repeated_chord_its_last_action() {
    let km = Keymap::new(
        [
            (chord("Space"), Confirm),
            (chord("f"), Confirm),
            (chord("e"), Info),
            (chord("Space"), Info),
        ],
        RepeatDef::default(),
    );
    assert_eq!(km.chords_for(Confirm), vec![chord("f")]);
    assert_eq!(km.chords_for(Info), vec![chord("e"), chord("Space")]);
    assert_eq!(km.action(chord("Space")), Some(Info));
    assert_eq!(km.primary(Info), Some(chord("e")));
}

#[test]
fn layout_picker_keys_work_for_either_hand() {
    let def = load_keymap().unwrap_or_default();
    let km = Keymap::layout_picker(&def);
    for (c, a) in [
        ("Up", CursorUp),
        ("w", CursorUp),
        ("Down", CursorDown),
        ("s", CursorDown),
        ("f", Confirm),
        ("j", Confirm),
        ("Enter", Confirm),
        ("Space", Confirm),
    ] {
        assert_eq!(km.action(chord(c)), Some(a), "{c}");
    }
    assert_eq!(km.chords_for(Confirm).len(), 4);
    assert_eq!(km.primary(Action::Cancel), None);
    assert_eq!(km.action(chord("d")), None);
    assert_eq!(km.repeat(), def.repeat);
}

#[test]
fn set_keymap_switches_bindings_and_forgets_held_keys() {
    let mut s = default_state();
    s.key_down(chord("l"));
    s.key_down(chord("f"));
    s.set_keymap(Keymap::layout_picker(&load_keymap().unwrap_or_default()));
    assert!(!s.is_held(Confirm));
    assert_eq!(s.update(ms(1000)), vec![]);
    // `f` is still physically down: ignored until pressed again.
    s.key_up(Key::F);
    s.key_down(chord("j"));
    assert_eq!(s.update(0.0), vec![Confirm]);
    assert_eq!(s.keymap().action(chord("l")), None);
}

#[test]
fn set_keymap_forgets_held_buttons_too() {
    let mut s = pad_state();
    s.pad_down(Button::DpadRight, PAD);
    s.pad_down(Button::South, PAD);
    s.set_keymap(test_keymap(170, 55).with_pad([(Button::East, Confirm)]));
    assert!(!s.is_held(Confirm) && !s.is_held(CursorRight));
    assert_eq!(s.update(ms(1000)), vec![]);
    // South is unbound now; East confirms.
    s.pad_up(Button::South);
    s.pad_down(Button::South, PAD);
    s.pad_down(Button::East, PAD);
    assert_eq!(s.update(0.0), vec![Confirm]);
}

#[test]
fn the_device_is_whatever_was_pressed_last() {
    let mut s = pad_state();
    assert_eq!(s.device(), Device::Keyboard);
    s.pad_down(Button::South, PadKind::PlayStation);
    assert_eq!(s.device(), Device::Pad(PadKind::PlayStation));
    // Releases and passing time don't count.
    s.pad_up(Button::South);
    s.key_up(Key::F);
    s.update(ms(500));
    assert_eq!(s.device(), Device::Pad(PadKind::PlayStation));
    s.key_down(chord("f"));
    assert_eq!(s.device(), Device::Keyboard);
    // Another pad, of another kind.
    s.pad_down(Button::DpadLeft, PadKind::Nintendo);
    assert_eq!(s.device(), Device::Pad(PadKind::Nintendo));
    s.pad_down(Button::DpadDown, PadKind::Generic);
    assert_eq!(s.device(), Device::Pad(PadKind::Generic));
    // The fixed Cancel key is a press like any other.
    s.key_down(chord("Escape"));
    assert_eq!(s.device(), Device::Keyboard);
    assert_eq!(Device::default(), Device::Keyboard);
}

#[test]
fn presses_that_do_nothing_leave_the_device_alone() {
    let mut s = pad_state();
    // An unbound button, and a button on a keymap without buttons.
    s.pad_down(Button::RightStickPress, PAD);
    assert_eq!(s.device(), Device::Keyboard);
    let mut keys_only = default_state();
    keys_only.pad_down(Button::South, PAD);
    assert_eq!(keys_only.device(), Device::Keyboard);
    // An unbound key, and a key reported down again while held.
    s.key_down(chord("f"));
    s.pad_down(Button::South, PAD);
    assert_eq!(s.device(), Device::Pad(PAD));
    s.key_down(chord("z"));
    s.key_down(chord("Shift+f"));
    s.key_down(chord("f"));
    assert_eq!(s.device(), Device::Pad(PAD));
    // Likewise a button held on one pad and pressed on another.
    s.key_up(Key::F);
    s.key_down(chord("f"));
    assert_eq!(s.device(), Device::Keyboard);
    s.pad_down(Button::South, PadKind::Nintendo);
    assert_eq!(s.device(), Device::Keyboard);
}

#[test]
fn switching_bindings_keeps_the_device() {
    let mut s = pad_state();
    s.pad_down(Button::South, PadKind::Nintendo);
    s.set_keymap(test_keymap(170, 55));
    assert_eq!(s.device(), Device::Pad(PadKind::Nintendo));
    assert!(!s.is_held(Confirm));
}

/// A keymap with only these buttons.
fn buttons(pad: &[(Button, Action)]) -> Keymap {
    Keymap::new([], RepeatDef::default()).with_pad(pad.iter().copied())
}

#[test]
fn the_first_button_is_the_primary_one() {
    let km = pad_state().keymap().clone();
    assert_eq!(km.primary_button(CursorDown), Some(Button::DpadDown.into()));
    assert_eq!(km.primary_button(Confirm), Some(Button::South.into()));
    assert_eq!(km.primary_button(Info), None);
    assert_eq!(test_keymap(170, 55).primary_button(Confirm), None);
}

#[test]
fn the_default_cursor_buttons_are_the_dpad_and_the_stick() {
    let def = load_keymap().unwrap_or_default();
    let km = Keymap::for_layout(&def, LEFT_HANDED);
    for kind in [
        PadKind::Xbox,
        PadKind::PlayStation,
        PadKind::Nintendo,
        PadKind::Generic,
    ] {
        assert_eq!(
            km.cursor_buttons_name(kind).as_deref(),
            Some("D-pad/L-stick")
        );
    }
}

#[test]
fn rebound_cursor_buttons_are_named_by_whole_sets_or_one_by_one() {
    use Button::{
        DpadDown, DpadLeft, DpadRight, DpadUp, East, LeftStickDown, LeftStickLeft, LeftStickRight,
        LeftStickUp, North, RightStickDown, RightStickLeft, RightStickRight, RightStickUp, South,
        West,
    };
    let name = |pad: &[(Button, Action)], kind| buttons(pad).cursor_buttons_name(kind);
    let on = |[up, down, left, right]: [Button; 4]| {
        [
            (up, CursorUp),
            (down, CursorDown),
            (left, CursorLeft),
            (right, CursorRight),
        ]
    };
    let dpad = on([DpadUp, DpadDown, DpadLeft, DpadRight]);
    let stick = on([LeftStickUp, LeftStickDown, LeftStickLeft, LeftStickRight]);
    let right = on([
        RightStickUp,
        RightStickDown,
        RightStickLeft,
        RightStickRight,
    ]);
    let xbox = PadKind::Xbox;
    assert_eq!(name(&dpad, xbox).as_deref(), Some("D-pad"));
    assert_eq!(name(&stick, xbox).as_deref(), Some("L-stick"));
    assert_eq!(name(&right, xbox).as_deref(), Some("R-stick"));
    // Whole sets are named D-pad first, whatever the slot order.
    let all = [right, stick, dpad].concat();
    assert_eq!(name(&all, xbox).as_deref(), Some("D-pad/L-stick/R-stick"));
    // A set with a direction elsewhere isn't whole; the whole one is
    // still named, and the odd button isn't.
    let mut mixed = [dpad, stick].concat();
    mixed.retain(|&(b, _)| b != LeftStickLeft);
    mixed.push((West, CursorLeft));
    assert_eq!(name(&mixed, xbox).as_deref(), Some("D-pad"));
    // No whole set: each direction's first button, up-left-down-right,
    // as the pad names them.
    let face = on([North, South, West, East]);
    assert_eq!(name(&face, xbox).as_deref(), Some("Y/X/A/B"));
    assert_eq!(name(&face, PadKind::Nintendo).as_deref(), Some("X/Y/A/B"));
    let broken = on([DpadUp, DpadDown, DpadLeft, East]);
    assert_eq!(name(&broken, xbox).as_deref(), Some("↑/←/↓/B"));
    // A direction with no button at all.
    for missing in 0..4 {
        let mut some = dpad.to_vec();
        some.remove(missing);
        assert_eq!(name(&some, xbox), None, "{missing}");
    }
    assert_eq!(name(&[], xbox), None);
    assert_eq!(test_keymap(170, 55).cursor_buttons_name(xbox), None);
}

#[test]
fn a_button_press_emits_once_and_is_held_until_released() {
    let mut s = pad_state();
    s.pad_down(Button::South, PAD);
    assert_eq!(s.update(0.0), vec![Confirm]);
    assert_eq!(s.update(ms(1000)), vec![]);
    assert!(s.is_held(Confirm));
    // Reported down again while held (a second pad): ignored.
    s.pad_down(Button::South, PAD);
    assert_eq!(s.update(0.0), vec![]);
    s.pad_up(Button::South);
    assert!(!s.is_held(Confirm));
    assert_eq!(s.update(ms(1000)), vec![]);
}

#[test]
fn unbound_buttons_are_ignored() {
    let mut s = pad_state();
    s.pad_down(Button::RightStickPress, PAD);
    assert!(s.held_pairs().is_empty());
    s.pad_up(Button::RightStickPress);
    assert_eq!(s.update(ms(500)), vec![]);
    // Without buttons in the keymap, every button is unbound.
    let mut s = default_state();
    s.pad_down(Button::South, PAD);
    assert_eq!(s.update(ms(500)), vec![]);
}

#[test]
fn a_held_button_repeats_with_the_key_timings() {
    let mut s = pad_state();
    s.pad_down(Button::DpadRight, PAD);
    assert_eq!(s.update(ms(169)), vec![CursorRight]);
    assert_eq!(s.update(ms(1)), vec![CursorRight]); // 170: first repeat
    assert_eq!(s.update(ms(54)), vec![]); // 224
    assert_eq!(s.update(ms(1)), vec![CursorRight]); // 225
    assert_eq!(s.update(ms(110)), vec![CursorRight, CursorRight]); // 335
    s.pad_up(Button::DpadRight);
    assert_eq!(s.update(ms(1000)), vec![]);
    assert!(!s.is_held(CursorRight));
}

#[test]
fn a_key_and_a_button_on_one_action_are_separate_inputs() {
    let mut s = pad_state();
    s.key_down(chord("f"));
    s.pad_down(Button::South, PAD);
    assert_eq!(s.update(0.0), vec![Confirm, Confirm]);
    // Letting go of one leaves the action held by the other.
    s.key_up(Key::F);
    assert!(s.is_held(Confirm));
    assert_eq!(s.held_pairs(), vec![(Input::Pad(Button::South), Confirm)]);
    s.pad_up(Button::South);
    assert!(!s.is_held(Confirm));
    // Likewise for two directions' repeat: the stick takes over from
    // the D-pad, and the D-pad resumes after a full delay.
    s.pad_down(Button::DpadDown, PAD);
    s.pad_down(Button::LeftStickDown, PAD);
    assert_eq!(s.update(ms(170)), vec![CursorDown, CursorDown, CursorDown]);
    s.pad_up(Button::LeftStickDown);
    assert_eq!(s.update(ms(169)), vec![]);
    assert_eq!(s.update(ms(1)), vec![CursorDown]);
}

#[test]
fn the_newest_of_keys_and_buttons_repeats() {
    let mut s = pad_state();
    s.key_down(chord("l"));
    assert_eq!(s.update(ms(100)), vec![CursorRight]);
    // A button pressed later takes over the repeat from the key...
    s.pad_down(Button::DpadDown, PAD);
    assert_eq!(s.update(ms(169)), vec![CursorDown]);
    assert_eq!(s.update(ms(1)), vec![CursorDown]);
    assert!(s.is_held(CursorRight) && s.is_held(CursorDown));
    // ...and hands it back when released, after a full delay.
    s.pad_up(Button::DpadDown);
    assert_eq!(s.update(ms(169)), vec![]);
    assert_eq!(s.update(ms(1)), vec![CursorRight]);
    // And the other way round: a key takes over from a button.
    let mut s = pad_state();
    s.pad_down(Button::DpadLeft, PAD);
    s.key_down(chord("j"));
    assert_eq!(s.update(ms(170)), vec![CursorLeft, CursorDown, CursorDown]);
    // Releasing the older, non-repeating one changes nothing.
    s.pad_up(Button::DpadLeft);
    assert_eq!(s.update(ms(55)), vec![CursorDown]);
    // A non-repeatable button doesn't stop the key's repeat.
    s.pad_down(Button::South, PAD);
    s.pad_up(Button::South);
    assert_eq!(s.update(ms(55)), vec![Confirm, CursorDown]);
}

#[test]
fn one_key_gives_an_action_of_each_group() {
    // controls.md, Dialogue log: another group's action may share it.
    let log = Action::DialogueLog;
    let km = Keymap::new(
        [
            (chord("Up"), CursorUp),
            (chord("Up"), log),
            (chord("b"), log),
            (chord("b"), Info),
            // The same group again: the last one keeps it.
            (chord("b"), Confirm),
        ],
        RepeatDef::default(),
    )
    .with_pad([(Button::West, log), (Button::West, Action::DangerZone)]);
    assert_eq!(km.actions(chord("Up")), [CursorUp, log]);
    assert_eq!(km.action(chord("Up")), Some(CursorUp));
    assert_eq!(km.actions(chord("b")), [Confirm, log]);
    assert_eq!(km.chords_for(Info), []);
    assert_eq!(km.chords_for(log), [chord("Up"), chord("b")]);
    assert_eq!(km.pad_actions(Button::West), [Action::DangerZone, log]);
    assert_eq!(km.pad_action(Button::West), Some(Action::DangerZone));
    // A press gives both, held while it is, and only the cursor repeats.
    let mut s = InputState::new(km);
    s.key_down(chord("Up"));
    assert_eq!(s.update(0.0), [CursorUp, log]);
    assert!(s.is_held(CursorUp) && s.is_held(log));
    assert_eq!(s.update(0.3), [CursorUp]);
    s.key_up(Key::Up);
    assert!(!s.is_held(log));
    s.pad_down(Button::West, PAD);
    assert_eq!(s.update(0.0), [Action::DangerZone, log]);
}

#[test]
fn press_emits_once_immediately() {
    let mut s = default_state();
    s.key_down(chord("f"));
    assert_eq!(s.update(0.0), vec![Confirm]);
    assert_eq!(s.update(ms(1000)), vec![]);
    assert!(s.is_held(Confirm));
    s.key_up(Key::F);
    assert!(!s.is_held(Confirm));
    assert_eq!(s.update(ms(1000)), vec![]);
}

#[test]
fn unbound_keys_are_ignored() {
    let mut s = default_state();
    s.key_down(chord("z"));
    s.key_up(Key::Z);
    assert_eq!(s.update(ms(500)), vec![]);
}

#[test]
fn repeat_exact_timing() {
    let mut s = default_state();
    s.key_down(chord("l"));
    assert_eq!(s.update(ms(169)), vec![CursorRight]);
    assert_eq!(s.update(ms(1)), vec![CursorRight]); // 170: first repeat
    assert_eq!(s.update(ms(54)), vec![]); // 224
    assert_eq!(s.update(ms(1)), vec![CursorRight]); // 225
    assert_eq!(s.update(ms(55)), vec![CursorRight]); // 280
    assert_eq!(s.update(ms(110)), vec![CursorRight, CursorRight]); // 390
}

#[test]
fn release_stops_repeat() {
    let mut s = default_state();
    s.key_down(chord("j"));
    assert_eq!(s.update(ms(200)), vec![CursorDown, CursorDown]);
    s.key_up(Key::J);
    assert_eq!(s.update(ms(1000)), vec![]);
    assert!(!s.is_held(CursorDown));
}

#[test]
fn repeat_of_one_frame_press_and_release() {
    // Press and release within one frame: the press still counts.
    let mut s = default_state();
    s.key_down(chord("k"));
    s.key_up(Key::K);
    assert_eq!(s.update(ms(500)), vec![CursorUp]);
}

#[test]
fn new_direction_takes_over_repeat() {
    let mut s = default_state();
    s.key_down(chord("l"));
    assert_eq!(s.update(ms(100)), vec![CursorRight]);
    s.key_down(chord("j"));
    // The new key starts its own delay; the old one no longer repeats.
    assert_eq!(s.update(ms(169)), vec![CursorDown]);
    assert_eq!(s.update(ms(1)), vec![CursorDown]);
    assert!(s.is_held(CursorRight));
    assert!(s.is_held(CursorDown));
}

#[test]
fn releasing_the_old_direction_keeps_the_new_one_repeating() {
    let mut s = default_state();
    s.key_down(chord("l"));
    s.key_down(chord("j"));
    assert_eq!(s.update(ms(100)), vec![CursorRight, CursorDown]);
    s.key_up(Key::L);
    assert_eq!(s.update(ms(70)), vec![CursorDown]);
}

#[test]
fn releasing_the_new_direction_resumes_the_old_after_a_delay() {
    let mut s = default_state();
    s.key_down(chord("l"));
    s.key_down(chord("j"));
    assert_eq!(
        s.update(ms(300)),
        vec![CursorRight, CursorDown, CursorDown, CursorDown, CursorDown]
    );
    s.key_up(Key::J);
    assert_eq!(s.update(ms(169)), vec![]);
    assert_eq!(s.update(ms(1)), vec![CursorRight]);
}

#[test]
fn non_repeatable_press_does_not_stop_cursor_repeat() {
    let mut s = default_state();
    s.key_down(chord("l"));
    assert_eq!(s.update(ms(100)), vec![CursorRight]);
    s.key_down(chord("f"));
    s.key_up(Key::F);
    assert_eq!(s.update(ms(70)), vec![Confirm, CursorRight]);
}

#[test]
fn repeated_key_down_while_held_is_ignored() {
    let mut s = default_state();
    s.key_down(chord("l"));
    assert_eq!(s.update(ms(100)), vec![CursorRight]);
    s.key_down(chord("l"));
    s.key_down(chord("Shift+l"));
    assert_eq!(s.update(ms(70)), vec![CursorRight]);
}

#[test]
fn huge_dt_is_capped() {
    let mut s = default_state();
    s.key_down(chord("h"));
    let out = s.update(ms(10_000));
    assert_eq!(out.len(), 1 + 5);
    assert!(out.iter().all(|&a| a == CursorLeft));
    // Dropped repeats are not carried over into the next frame.
    assert_eq!(s.update(ms(55)), vec![CursorLeft]);
}

#[test]
fn cap_counts_only_repeats() {
    // Exactly 5 repeats due: all emitted.
    let mut s = default_state();
    s.key_down(chord("h"));
    assert_eq!(s.update(ms(170 + 4 * 55)).len(), 1 + 5);
    // 6 due in one frame after the press frame: capped to 5.
    let mut s = default_state();
    s.key_down(chord("h"));
    assert_eq!(s.update(0.0).len(), 1);
    assert_eq!(s.update(ms(170 + 5 * 55)).len(), 5);
}

#[test]
fn bad_dt_counts_as_zero() {
    let mut s = default_state();
    s.key_down(chord("h"));
    assert_eq!(s.update(-1.0), vec![CursorLeft]);
    assert_eq!(s.update(f32::NAN), vec![]);
    assert_eq!(s.update(f32::INFINITY), vec![]);
    assert_eq!(s.update(ms(170)), vec![CursorLeft]);
}

#[test]
fn seconds_to_micros_rounds_to_nearest() {
    assert_eq!(seconds_to_micros(0.017), 17_000);
    assert_eq!(seconds_to_micros(0.000_000_4), 0);
    assert_eq!(seconds_to_micros(0.000_000_6), 1);
    assert_eq!(seconds_to_micros(1.0), 1_000_000);
    assert_eq!(seconds_to_micros(-0.5), 0);
}

#[test]
fn zero_interval_does_not_hang() {
    let mut s = state_with(0, 0);
    s.key_down(chord("h"));
    assert_eq!(s.update(ms(10)).len(), 1 + 5);
}

/// One step of a random input script.
#[derive(Debug, Clone, Copy)]
enum Step {
    KeyDown(Chord),
    KeyUp(Key),
    PadDown(Button),
    PadUp(Button),
    Wait(u32),
}

fn arb_step() -> impl Strategy<Value = Step> {
    // Bound and unbound keys and buttons, repeatable and not.
    let key = || prop::sample::select(vec![Key::H, Key::J, Key::L, Key::F, Key::Z]);
    let button = || {
        prop::sample::select(vec![
            Button::DpadLeft,
            Button::DpadDown,
            Button::LeftStickDown,
            Button::South,
            Button::North,
        ])
    };
    prop_oneof![
        (key(), any::<bool>()).prop_map(|(key, shift)| Step::KeyDown(Chord { key, shift })),
        key().prop_map(Step::KeyUp),
        button().prop_map(Step::PadDown),
        button().prop_map(Step::PadUp),
        (0u32..400).prop_map(Step::Wait),
    ]
}

proptest! {
    /// Any mix of key and button presses and releases: nothing stays
    /// held after its release, only bound inputs are held (each once,
    /// oldest first), and the one repeating is always the newest held
    /// cursor input.
    #[test]
    fn keys_and_buttons_never_stick_and_only_the_newest_cursor_input_repeats(
        steps in prop::collection::vec(arb_step(), 0..60),
    ) {
        let mut s = pad_state();
        let km = s.keymap().clone();
        // What should be held, oldest first, and the presses not yet
        // handed out.
        let mut held: Vec<(Input, Action)> = Vec::new();
        let mut pending = Vec::new();
        for &step in &steps {
            let press = match step {
                Step::KeyDown(chord) => {
                    s.key_down(chord);
                    Some((Input::Key(chord.key), km.action(chord)))
                }
                Step::PadDown(button) => {
                    s.pad_down(button, PAD);
                    Some((Input::Pad(button), km.pad_action(button)))
                }
                _ => None,
            };
            if let Some((input, Some(action))) = press
                && !held.iter().any(|&(i, _)| i == input)
            {
                held.push((input, action));
                pending.push(action);
            }
            let release = match step {
                Step::KeyUp(key) => {
                    s.key_up(key);
                    Some(Input::Key(key))
                }
                Step::PadUp(button) => {
                    s.pad_up(button);
                    Some(Input::Pad(button))
                }
                _ => None,
            };
            if let Some(input) = release {
                held.retain(|&(i, _)| i != input);
                prop_assert!(!s.held_pairs().iter().any(|&(i, _)| i == input));
            }
            prop_assert_eq!(&s.held_pairs(), &held);
            let newest = held.iter().rev().find(|(_, a)| a.is_repeatable());
            prop_assert_eq!(s.repeat.map(|r| (r.input, r.action)), newest.copied());
            for action in Action::ALL {
                prop_assert_eq!(s.is_held(action), held.iter().any(|&(_, a)| a == action));
            }
            if let Step::Wait(millis) = step {
                let out = s.update(ms(millis));
                // The presses, in order, then only the newest cursor
                // input's repeats.
                prop_assert!(out.len() >= pending.len());
                prop_assert_eq!(&out[..pending.len()], &pending[..]);
                let repeats = &out[pending.len()..];
                match newest {
                    Some(&(_, action)) => prop_assert!(repeats.iter().all(|&a| a == action)),
                    None => prop_assert!(repeats.is_empty()),
                }
                pending.clear();
            }
        }
    }

    #[test]
    fn held_key_emits_expected_count(
        slices in prop::collection::vec(0u32..=100, 0..60),
        delay in 1u32..400,
        interval in 20u32..200,
    ) {
        let mut s = state_with(delay, interval);
        s.key_down(chord("l"));
        let mut total = 0;
        for &slice in &slices {
            let out = s.update(ms(slice));
            prop_assert!(out.iter().all(|&a| a == CursorRight));
            total += out.len();
        }
        if slices.is_empty() {
            total += s.update(0.0).len();
        }
        let t: u32 = slices.iter().sum();
        let expected = if t < delay { 1 } else { 2 + (t - delay) / interval };
        prop_assert_eq!(Ok(total), usize::try_from(expected));
    }
}

#[test]
fn a_frame_holds_what_the_player_did_this_frame() {
    let held = FrameInput::new(vec![Confirm], 0.016, vec![Confirm, CursorUp]);
    assert_eq!(held.actions, [Confirm]);
    assert!(held.is_held(Confirm) && held.is_held(CursorUp));
    assert!(!held.is_held(Info));
    assert!(!held.pad_pressed());
    assert_eq!(held.pressed_chords(), []);
    assert!(held.text().is_empty());
    assert_eq!(held.pressed_buttons(), []);
    assert_eq!(held.released_buttons(), []);

    let typed = held.with_typing(vec![chord("h"), chord("Shift+h")], vec!['h', 'H']);
    assert_eq!(typed.pressed_chords(), [chord("h"), chord("Shift+h")]);
    assert_eq!(typed.text(), ['h', 'H']);

    let pad = typed.with_buttons(vec![Button::South, Button::DpadUp], vec![Button::East]);
    assert!(pad.pad_pressed());
    assert_eq!(pad.pressed_buttons(), [Button::South, Button::DpadUp]);
    assert_eq!(pad.released_buttons(), [Button::East]);
    // Only buttons going down count as being on a controller.
    let released =
        FrameInput::<Action>::new(vec![], 0.0, vec![]).with_buttons(vec![], vec![Button::East]);
    assert!(!released.pad_pressed());
    assert!(!pad.clone().with_pad(false).pad_pressed());
    assert!(
        FrameInput::<Action>::new(vec![], 0.0, vec![])
            .with_pad(true)
            .pad_pressed()
    );
}

#[test]
fn the_game_can_start_on_a_pad_and_mark_the_keyboard_used() {
    let pad = Device::Pad(PadKind::Playdate);
    let mut s = default_state().starting_on(pad);
    assert_eq!(s.device(), pad);
    s.keyboard_used();
    assert_eq!(s.device(), Device::Keyboard);
    // Nothing else happens: no action, nothing held.
    assert!(s.update(0.0).is_empty());
    assert_eq!(default_state().device(), Device::Keyboard);
}

#[test]
fn a_text_box_has_its_own_fixed_keys_shifted_or_not() {
    for shift in [false, true] {
        let key = |k| Chord { key: k, shift };
        assert_eq!(text_key(key(Key::Enter)), Some(TextKey::Done));
        assert_eq!(text_key(key(Key::Backspace)), Some(TextKey::Delete));
        assert_eq!(text_key(key(Key::Escape)), Some(TextKey::Cancel));
        assert_eq!(text_key(key(Key::Space)), None);
        assert_eq!(text_key(key(Key::A)), None);
    }
    assert_eq!(text_key_names(), ["Enter", "Backspace", "Escape"]);
}

#[test]
fn has_input_asks_the_device_in_use() {
    let def = load_keymap().unwrap_or_default();
    let km = Keymap::for_layout(&def, RIGHT_HANDED);
    let pad = Device::Pad(PadKind::Xbox);
    // Confirm has a key and a button; Debug only a key; the optional
    // split key Select neither.
    assert!(km.has_input(Confirm, Device::Keyboard));
    assert!(km.has_input(Confirm, pad));
    assert!(km.has_input(Action::Debug, Device::Keyboard));
    assert!(!km.has_input(Action::Debug, pad));
    assert!(!km.has_input(Action::Select, Device::Keyboard));
    assert!(!km.has_input(Action::Select, pad));
}
