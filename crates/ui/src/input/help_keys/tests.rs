//! Tests of help text's key names, over the sample game's actions.
use super::*;
use crate::input::sample::{Action, Keymap, RIGHT_HANDED, load_keymap};
use crate::input::{Button, Chord, Key, PadKind, RepeatDef};

/// What help text names the sample game's keys from.
type HelpKeys<'a> = super::HelpKeys<'a, Action>;

const XBOX: Device = Device::Pad(PadKind::Xbox);
const PLAYSTATION: Device = Device::Pad(PadKind::PlayStation);
const NINTENDO: Device = Device::Pad(PadKind::Nintendo);

/// The right-handed defaults: keys and buttons.
fn defaults() -> Keymap {
    let def = load_keymap().unwrap_or_else(|e| panic!("{e:?}"));
    Keymap::for_layout(&def, RIGHT_HANDED)
}

/// Ticket 0243: the words inside a key's name are the language's.
#[test]
fn a_keys_name_is_worded_in_the_players_language() {
    let km = defaults();
    let english = HelpKeys::keyboard(&km);
    assert_eq!(english.not_mapped(), NOT_MAPPED);
    assert_eq!(cursor_keys_name(english), ARROWS);
    let keys = english.worded("kein", "Pfeile");
    assert_eq!(keys.not_mapped(), "kein");
    assert_eq!(cursor_keys_name(keys), "Pfeile");
    // Another device, or other bindings, keep the words.
    let pad_only = keymap(&[]);
    let unbound = keys.of(&pad_only);
    assert_eq!(key_name(unbound, Action::Confirm), "kein");
    assert_eq!(all_key_names(unbound, Action::Confirm), "kein");
    assert_eq!(cursor_keys_name(unbound), "kein");
    assert_eq!(key_name(keys, Action::Confirm), "f");
    let on_pad = HelpKeys::new(&km, XBOX).worded("kein", "Pfeile");
    assert_eq!(on_pad.device(), XBOX);
    assert_eq!(on_pad.on_keyboard(), keys);
    assert_eq!(on_pad.of(&pad_only).device(), XBOX);
    assert_ne!(keys, english);
}

#[test]
fn help_keys_are_the_keymap_and_a_device() {
    let km = defaults();
    let keys = HelpKeys::new(&km, XBOX);
    assert_eq!(keys.device(), XBOX);
    assert_eq!(HelpKeys::keyboard(&km).device(), Device::Keyboard);
    assert_eq!(
        HelpKeys::keyboard(&km),
        HelpKeys::new(&km, Device::Keyboard)
    );
    assert_ne!(keys, HelpKeys::keyboard(&km));
    // It stands in for the keymap.
    assert_eq!(keys.chords_for(Action::Confirm), [Chord::plain(Key::F)]);
    assert_eq!(keys.primary_button(Action::Confirm), Some(Button::South));
}

/// `docs/design/controls.md`, *Button names on screen*: the example
/// battle help bar on each kind of pad, and the keys again.
#[test]
fn key_names_follow_the_device() {
    let km = defaults();
    let line = |device| {
        let keys = HelpKeys::new(&km, device);
        help_line(&[
            (Some(cursor_keys_name(keys)), "move"),
            (Some(key_name(keys, Action::Confirm)), "select"),
            (Some(key_name(keys, Action::Info)), "info"),
            (Some(key_name(keys, Action::NextUnit)), "next unit"),
            (Some(key_name(keys, Action::Rewind)), "rewind"),
            (Some(key_name(keys, Action::Cancel)), "back"),
            (Some(key_name(keys, Action::EndTurn)), "end turn"),
        ])
    };
    assert_eq!(
        line(Device::Keyboard),
        "arrows move · f select · e info · s next unit · r rewind · d back · Space end turn"
    );
    let xbox = "D-pad/L-stick move · A select · Y info · RB next unit · LT rewind · B back \
                · Start end turn";
    assert_eq!(line(XBOX), xbox);
    assert_eq!(line(Device::Pad(PadKind::Generic)), xbox);
    assert_eq!(
        line(PLAYSTATION),
        "D-pad/L-stick move · ✕ select · △ info · R1 next unit · L2 rewind · ◯ back \
         · Options end turn"
    );
    assert_eq!(
        line(NINTENDO),
        "D-pad/L-stick move · A select · X info · R next unit · ZL rewind · B back \
         · + end turn"
    );
}

#[test]
fn the_other_default_buttons_are_named_too() {
    let km = defaults();
    let name = |device, action| key_name(HelpKeys::new(&km, device), action);
    let names = |action| [XBOX, PLAYSTATION, NINTENDO].map(|d| name(d, action));
    assert_eq!(names(Action::PrevUnit), ["LB", "L1", "L"]);
    assert_eq!(names(Action::DangerZone), ["X", "□", "Y"]);
    assert_eq!(names(Action::ToggleAutoEnd), ["Back", "Create", "−"]);
    assert_eq!(names(Action::CursorLeft), ["←", "←", "←"]);
}

/// An action with no button shows `! not mapped` on a pad, whatever
/// keys it has; and the other way round.
#[test]
fn an_action_with_no_button_is_not_mapped_on_a_pad() {
    let km = defaults();
    for device in [XBOX, PLAYSTATION, NINTENDO] {
        let keys = HelpKeys::new(&km, device);
        // Keyboard-only by default: the debug key; empty everywhere:
        // Select.
        assert_eq!(key_name(keys, Action::Debug), NOT_MAPPED);
        assert_eq!(key_name(keys, Action::Select), "! not mapped");
        assert_eq!(all_key_names(keys, Action::Debug), NOT_MAPPED);
    }
    assert_ne!(key_name(HelpKeys::keyboard(&km), Action::Debug), NOT_MAPPED);
    // Buttons but no keys.
    let pad_only = keymap(&[]).with_pad([(Button::South, Action::Confirm)]);
    assert_eq!(
        key_name(HelpKeys::new(&pad_only, XBOX), Action::Confirm),
        "A"
    );
    let keys = HelpKeys::keyboard(&pad_only);
    assert_eq!(key_name(keys, Action::Confirm), NOT_MAPPED);
    assert_eq!(all_key_names(keys, Action::Confirm), NOT_MAPPED);
    // The fixed Cancel key is a key, not a button.
    assert_eq!(
        key_name(HelpKeys::new(&pad_only, XBOX), Action::Cancel),
        NOT_MAPPED
    );
    assert_eq!(
        all_key_names(HelpKeys::new(&pad_only, XBOX), Action::Cancel),
        NOT_MAPPED
    );
}

#[test]
fn all_key_names_lists_every_button_in_order_on_a_pad() {
    let km = defaults();
    let names = |device, action| all_key_names(HelpKeys::new(&km, device), action);
    assert_eq!(names(XBOX, Action::CursorUp), "↑/L-stick ↑");
    assert_eq!(names(PLAYSTATION, Action::Confirm), "✕");
    // No fixed extra on a pad: Cancel is its button alone.
    assert_eq!(names(NINTENDO, Action::Cancel), "B");
    assert_eq!(names(Device::Keyboard, Action::Cancel), "d/Escape");
    let two = keymap(&[]).with_pad([
        (Button::RightStickPress, Action::Info),
        (Button::North, Action::Info),
    ]);
    assert_eq!(
        all_key_names(HelpKeys::new(&two, PLAYSTATION), Action::Info),
        "R3/△"
    );
    assert_eq!(
        key_name(HelpKeys::new(&two, PLAYSTATION), Action::Info),
        "R3"
    );
}

#[test]
fn the_cursor_on_a_pad_is_its_buttons_or_not_mapped() {
    let km = defaults();
    assert_eq!(
        cursor_keys_name(HelpKeys::new(&km, PLAYSTATION)),
        "D-pad/L-stick"
    );
    assert_eq!(cursor_keys_name(HelpKeys::keyboard(&km)), "arrows");
    // Keys but no buttons, and the other way round.
    let arrows = keymap(&[
        ("Up", Action::CursorUp),
        ("Left", Action::CursorLeft),
        ("Down", Action::CursorDown),
        ("Right", Action::CursorRight),
    ]);
    assert_eq!(cursor_keys_name(HelpKeys::new(&arrows, XBOX)), NOT_MAPPED);
    let dpad = keymap(&[]).with_pad([
        (Button::DpadUp, Action::CursorUp),
        (Button::DpadLeft, Action::CursorLeft),
        (Button::DpadDown, Action::CursorDown),
        (Button::DpadRight, Action::CursorRight),
    ]);
    assert_eq!(cursor_keys_name(HelpKeys::new(&dpad, XBOX)), "D-pad");
    assert_eq!(cursor_keys_name(HelpKeys::keyboard(&dpad)), NOT_MAPPED);
}

fn keymap(pairs: &[(&str, Action)]) -> Keymap {
    Keymap::new(
        pairs.iter().map(|&(c, a)| (Chord::parse(c).unwrap(), a)),
        RepeatDef::default(),
    )
}

#[test]
fn all_key_names_lists_every_chord_in_slot_order_then_the_fixed_ones() {
    let km = keymap(&[
        ("Space", Action::Confirm),
        ("f", Action::Confirm),
        ("Enter", Action::Confirm),
        ("d", Action::Cancel),
    ]);
    assert_eq!(
        all_key_names(HelpKeys::keyboard(&km), Action::Confirm),
        "Space/f/Enter"
    );
    assert_eq!(
        all_key_names(HelpKeys::keyboard(&km), Action::Cancel),
        "d/Escape"
    );
    assert_eq!(
        all_key_names(HelpKeys::keyboard(&km), Action::Info),
        NOT_MAPPED
    );
    // Esc alone doesn't count as a key of Cancel's own.
    assert_eq!(
        all_key_names(HelpKeys::keyboard(&keymap(&[])), Action::Cancel),
        NOT_MAPPED
    );
}

#[test]
fn key_names() {
    let km = keymap(&[
        ("f", Action::Confirm),
        ("Shift+Space", Action::ToggleAutoEnd),
    ]);
    assert_eq!(key_name(HelpKeys::keyboard(&km), Action::Confirm), "f");
    assert_eq!(
        key_name(HelpKeys::keyboard(&km), Action::ToggleAutoEnd),
        "Shift+Space"
    );
    // Cancel shows its own keys, never the fixed Esc.
    assert_eq!(
        key_name(HelpKeys::keyboard(&km), Action::Cancel),
        "! not mapped"
    );
    assert_eq!(key_name(HelpKeys::keyboard(&km), Action::Info), NOT_MAPPED);
}

#[test]
fn arrow_keys_are_called_arrows() {
    let km = keymap(&[
        ("Up", Action::CursorUp),
        ("Left", Action::CursorLeft),
        ("Down", Action::CursorDown),
        ("Right", Action::CursorRight),
    ]);
    assert_eq!(cursor_keys_name(HelpKeys::keyboard(&km)), "arrows");
}

#[test]
fn other_cursor_keys_are_listed() {
    let wasd = keymap(&[
        ("w", Action::CursorUp),
        ("a", Action::CursorLeft),
        ("s", Action::CursorDown),
        ("d", Action::CursorRight),
    ]);
    assert_eq!(cursor_keys_name(HelpKeys::keyboard(&wasd)), "wasd");
    // One arrow out of place, or shifted arrows, is not "arrows".
    let mixed = keymap(&[
        ("Up", Action::CursorUp),
        ("Left", Action::CursorLeft),
        ("Down", Action::CursorDown),
        ("l", Action::CursorRight),
    ]);
    assert_eq!(cursor_keys_name(HelpKeys::keyboard(&mixed)), "UpLeftDownl");
    let shifted = keymap(&[
        ("Shift+Up", Action::CursorUp),
        ("Shift+Left", Action::CursorLeft),
        ("Shift+Down", Action::CursorDown),
        ("Shift+Right", Action::CursorRight),
    ]);
    assert_eq!(
        cursor_keys_name(HelpKeys::keyboard(&shifted)),
        "Shift+UpShift+LeftShift+DownShift+Right"
    );
}

#[test]
fn unbound_cursor_key_is_not_mapped() {
    let km = keymap(&[
        ("Up", Action::CursorUp),
        ("Left", Action::CursorLeft),
        ("Down", Action::CursorDown),
    ]);
    assert_eq!(cursor_keys_name(HelpKeys::keyboard(&km)), NOT_MAPPED);
}

#[test]
fn help_line_skips_unbound() {
    let line = help_line(&[
        (Some("arrows".into()), "move"),
        (None, "info"),
        (Some("f".into()), "select"),
        (Some("d".into()), "back"),
    ]);
    assert_eq!(line, "arrows move · f select · d back");
    assert_eq!(help_line(&[]), "");
    assert_eq!(help_line(&[(None, "x")]), "");
}
