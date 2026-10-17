//! Tests of the player's key bindings: slots, moves, reserved keys, and the
//! saved config's repair on load.

use proptest::prelude::*;

use super::*;
use crate::input::sample::{
    Action, KeyGroup, KeymapDef, LEFT_HANDED, LayoutBindings, PadBindings, PlayerKeys,
    RIGHT_HANDED, load_keymap,
};
use crate::input::{Button, Key};
use Action::{Cancel, Confirm, CursorDown, CursorLeft, CursorRight, CursorUp, Debug, Info};

fn chord(s: &str) -> Chord {
    Chord::parse(s).unwrap_or_else(|e| panic!("{e}"))
}

fn def() -> KeymapDef {
    load_keymap().unwrap_or_else(|e| panic!("{e:?}"))
}

fn right() -> LayoutBindings {
    LayoutBindings::defaults(&def(), RIGHT_HANDED)
}

/// Slots as names, `-` for empty.
fn names(slots: Slots) -> [String; SLOTS] {
    slots.map(|c| c.map_or_else(|| "-".to_owned(), |c| c.to_string()))
}

#[test]
fn defaults_fill_the_first_slots_in_file_order() {
    let b = right();
    assert_eq!(names(b.slots(Confirm)), ["f", "-", "-"]);
    assert_eq!(names(b.slots(Cancel)), ["d", "-", "-"]);
    assert_eq!(names(b.slots(Action::Menu)), ["-", "-", "-"]);
    // Debug has no slots, but keeps its key.
    assert_eq!(names(b.slots(Debug)), ["-", "-", "-"]);
    assert_eq!(
        b.keymap(RepeatDef::default()).action(chord("F2")),
        Some(Debug)
    );
    let left = LayoutBindings::defaults(&def(), LEFT_HANDED);
    assert_eq!(names(left.slots(Confirm)), ["j", "-", "-"]);
    assert!(b.unmapped_required().is_empty());
    assert!(left.unmapped_required().is_empty());
}

#[test]
fn default_bindings_give_the_default_keymap() {
    let def = def();
    for layout in [RIGHT_HANDED, LEFT_HANDED] {
        // The layout's keys, plus the controller buttons every keymap has.
        let keys = LayoutBindings::defaults(&def, layout).keymap(def.repeat);
        assert_eq!(
            keys.with_default_pad(&def),
            Keymap::for_layout(&def, layout),
            "{layout}"
        );
    }
}

#[test]
fn a_hand_built_definition_is_repaired_into_the_invariant() {
    let mut def = KeymapDef::default();
    let keys = [
        (Confirm, vec![chord("f"), chord("Escape"), chord("g")]),
        (Info, vec![chord("f"), chord("a"), chord("b"), chord("c")]),
    ];
    def.layouts.insert(LEFT_HANDED, keys.into());
    let b = LayoutBindings::defaults(&def, LEFT_HANDED);
    // `f` moved to Info (listed later); Esc skipped; a 4th chord dropped.
    assert_eq!(names(b.slots(Confirm)), ["-", "-", "g"]);
    assert_eq!(names(b.slots(Info)), ["f", "a", "b"]);
}

#[test]
fn binding_a_key_another_action_has_moves_it() {
    let mut b = right();
    // Info's only key `e` goes to Confirm's second slot.
    assert_eq!(b.bind(Confirm, 1, chord("e")), Ok(Some((Info, 0))));
    assert_eq!(names(b.slots(Confirm)), ["f", "e", "-"]);
    assert!(b.is_unmapped(Info));
    assert!(!b.is_unmapped(Confirm));
    let km = b.keymap(RepeatDef::default());
    assert_eq!(km.action(chord("e")), Some(Confirm));
    assert_eq!(km.chords_for(Info), vec![]);
    assert_eq!(km.chords_for(Confirm), vec![chord("f"), chord("e")]);
}

#[test]
fn binding_within_an_action_moves_between_slots() {
    let mut b = right();
    assert_eq!(b.bind(Confirm, 2, chord("f")), Ok(Some((Confirm, 0))));
    assert_eq!(names(b.slots(Confirm)), ["-", "-", "f"]);
    // Binding it where it already is changes nothing.
    assert_eq!(b.bind(Confirm, 2, chord("f")), Ok(None));
    assert_eq!(names(b.slots(Confirm)), ["-", "-", "f"]);
}

#[test]
fn binding_over_a_key_replaces_it() {
    let mut b = right();
    assert_eq!(b.bind(Confirm, 0, chord("Shift+g")), Ok(None));
    assert_eq!(names(b.slots(Confirm)), ["Shift+g", "-", "-"]);
    let km = b.keymap(RepeatDef::default());
    assert_eq!(km.action(chord("f")), None);
    assert_eq!(km.action(chord("g")), None);
    assert_eq!(km.action(chord("Shift+g")), Some(Confirm));
}

#[test]
fn esc_delete_and_the_debug_key_are_reserved() {
    let mut b = right();
    for s in ["Escape", "Delete", "F2"] {
        let c = chord(s);
        assert!(b.is_reserved(c), "{s}");
        assert_eq!(b.bind(Info, 0, c), Err(BindError::Reserved(c)), "{s}");
    }
    assert!(!b.is_reserved(chord("Shift+F2")));
    assert_eq!(b, right(), "a refused bind changes nothing");
    // Shifted Esc and Delete are ordinary keys (Nick, ticket 0217).
    assert_eq!(b.bind(Info, 1, chord("Shift+Escape")), Ok(None));
    assert_eq!(b.bind(Info, 2, chord("Shift+Delete")), Ok(None));
    let km = b.keymap(RepeatDef::default());
    assert_eq!(km.action(chord("Shift+Escape")), Some(Info));
    assert_eq!(km.action(chord("Shift+Delete")), Some(Info));
    assert_eq!(km.action(chord("Escape")), Some(Cancel));
    assert_eq!(
        BindError::<Action>::Reserved(chord("Escape")).to_string(),
        "Escape is a fixed key and can't be bound"
    );
}

#[test]
fn without_debug_tools_the_debug_key_can_be_bound_and_wins() {
    let mut b = LayoutBindings::from_def(&def(), RIGHT_HANDED, false);
    assert!(!b.is_reserved(chord("F2")));
    assert!(b.is_reserved(chord("Escape")));
    assert_eq!(b.bind(Info, 1, chord("F2")), Ok(None));
    let km = b.keymap(RepeatDef::default());
    assert_eq!(km.action(chord("F2")), Some(Info));
    assert_eq!(km.chords_for(Debug), vec![]);
}

#[test]
fn debug_and_bad_slots_are_refused() {
    let mut b = right();
    assert_eq!(
        b.bind(Debug, 0, chord("g")),
        Err(BindError::NotRebindable(Debug))
    );
    assert_eq!(
        b.bind(Info, SLOTS, chord("g")),
        Err(BindError::NoSuchSlot(SLOTS))
    );
    assert_eq!(b, right());
    assert_eq!(
        BindError::NotRebindable(Debug).to_string(),
        "Debug can't be rebound"
    );
    assert_eq!(
        BindError::<Action>::NoSuchSlot(3).to_string(),
        "there is no key slot 3"
    );
}

#[test]
fn clear_empties_a_slot() {
    let mut b = right();
    b.bind(Info, 2, chord("g")).ok();
    b.clear(Info, 0);
    assert_eq!(names(b.slots(Info)), ["-", "-", "g"]);
    b.clear(Info, 2);
    assert!(b.is_unmapped(Info));
    // Slots and actions that don't exist: nothing happens.
    b.clear(Info, SLOTS);
    b.clear(Debug, 0);
    assert_eq!(
        b.keymap(RepeatDef::default()).action(chord("F2")),
        Some(Debug)
    );
}

#[test]
fn unmapped_required_lists_exactly_the_required_actions_with_no_key() {
    let mut b = right();
    for action in Action::ALL {
        for i in 0..SLOTS {
            b.clear(action, i);
        }
    }
    assert_eq!(
        b.unmapped_required(),
        [
            CursorLeft,
            CursorDown,
            CursorUp,
            CursorRight,
            Confirm,
            Cancel,
            Action::EndTurn
        ]
    );
    b.bind(Cancel, 2, chord("x")).ok();
    b.bind(CursorUp, 0, chord("Up")).ok();
    assert_eq!(
        b.unmapped_required(),
        [
            CursorLeft,
            CursorDown,
            CursorRight,
            Confirm,
            Action::EndTurn
        ]
    );
    // Losing an optional action's key doesn't count.
    let mut b = right();
    b.clear(Info, 0);
    assert!(b.unmapped_required().is_empty());
    // Moving Confirm's only key away does.
    b.bind(Info, 0, chord("f")).ok();
    assert_eq!(b.unmapped_required(), [Confirm]);
}

#[test]
fn the_keymap_has_the_fixed_keys() {
    let mut b = right();
    for i in 0..SLOTS {
        b.clear(Cancel, i);
    }
    let km = b.keymap(RepeatDef::default());
    assert_eq!(km.action(chord("Escape")), Some(Cancel));
    assert_eq!(km.action(chord("Delete")), None);
    assert_eq!(km.chords_for(Cancel), vec![]);
    assert_eq!(Keymap::fixed_chords_for(Cancel), vec![chord("Escape")]);
    assert_eq!(Keymap::fixed_chords_for(Confirm), vec![]);
}

/// One edit of a property test.
#[derive(Debug, Clone)]
enum Op {
    Bind(Action, usize, Chord),
    Clear(Action, usize),
}

fn arb_op() -> impl Strategy<Value = Op> {
    let action = prop::sample::select(Action::ALL.to_vec());
    let slot = 0..=SLOTS;
    let key = prop::sample::select(Key::ALL);
    prop_oneof![
        (action.clone(), slot.clone(), key, any::<bool>()).prop_map(|(a, i, key, shift)| Op::Bind(
            a,
            i,
            Chord { key, shift }
        )),
        (action, slot).prop_map(|(a, i)| Op::Clear(a, i)),
    ]
}

proptest! {
    #[test]
    fn no_chord_is_in_two_slots_and_none_is_reserved(
        ops in prop::collection::vec(arb_op(), 0..80),
        layout in prop::sample::select(vec![RIGHT_HANDED, LEFT_HANDED]),
    ) {
        let mut b = LayoutBindings::defaults(&def(), layout);
        for op in ops {
            match op {
                Op::Bind(a, i, c) => {
                    let before = b.clone();
                    if b.bind(a, i, c).is_ok() {
                        prop_assert_eq!(b.slots(a).get(i).copied().flatten(), Some(c));
                    } else {
                        prop_assert_eq!(&b, &before);
                    }
                }
                Op::Clear(a, i) => b.clear(a, i),
            }
            // At most once in each key group.
            for group in KeyGroup::ALL {
                let all: Vec<Chord> = Action::ALL
                    .into_iter()
                    .filter(|a| a.group() == group)
                    .flat_map(|a| b.slots(a).into_iter().flatten())
                    .collect();
                let mut unique = all.clone();
                unique.sort_unstable();
                unique.dedup();
                prop_assert_eq!(unique.len(), all.len(), "a chord in two slots: {:?}", all);
                prop_assert!(all.iter().all(|&c| !b.is_reserved(c)));
            }
        }
    }
}

#[test]
fn a_key_moves_only_within_its_group() {
    // controls.md, Dialogue log: the log's key may be another group's too.
    let mut b = LayoutBindings::defaults(&def(), RIGHT_HANDED);
    assert_eq!(b.bind(Action::DialogueLog, 0, chord("f")), Ok(None));
    assert_eq!(b.slots(Confirm)[0], Some(chord("f")), "Confirm keeps f");
    assert_eq!(b.find(KeyGroup::General, chord("f")), Some((Confirm, 0)));
    assert_eq!(
        b.find(KeyGroup::DialogueLog, chord("f")),
        Some((Action::DialogueLog, 0))
    );
    let km = b.keymap(RepeatDef::default());
    assert_eq!(km.actions(chord("f")), [Confirm, Action::DialogueLog]);
    // In one group it still moves.
    assert_eq!(b.bind(Info, 1, chord("f")), Ok(Some((Confirm, 0))));
    assert!(b.is_unmapped(Confirm));
    assert_eq!(b.slots(Action::DialogueLog)[0], Some(chord("f")));
}

#[test]
fn saved_keys_of_another_group_keep_the_logs_default() {
    // A config from before the log: `b` on Unit info; the log keeps `b`.
    let saved =
        "PlayerKeys(version: 2, layouts: {\"RightHanded\": {\"Info\": [Some(\"b\"), None, None]}})";
    let (keys, warnings) = PlayerKeys::from_ron(saved, &def());
    assert_eq!(warnings, Vec::<String>::new());
    let b = keys.bindings(&def(), RIGHT_HANDED);
    assert_eq!(b.slots(Info)[0], Some(chord("b")));
    assert_eq!(b.slots(Action::DialogueLog)[0], Some(chord("b")));
}

// --- Saved config ----------------------------------------------------------

#[test]
fn player_keys_start_as_the_defaults() {
    let keys = PlayerKeys::default();
    for layout in [RIGHT_HANDED, LEFT_HANDED] {
        assert!(!keys.is_custom(layout));
        assert_eq!(
            keys.bindings(&def(), layout),
            LayoutBindings::defaults(&def(), layout)
        );
    }
}

#[test]
fn each_layout_keeps_its_own_keys() {
    let def = def();
    let mut keys = PlayerKeys::default();
    let mut b = keys.bindings(&def, RIGHT_HANDED);
    b.bind(Info, 1, chord("g")).ok();
    keys.set(&def, RIGHT_HANDED, b.clone());
    assert!(keys.is_custom(RIGHT_HANDED));
    assert!(!keys.is_custom(LEFT_HANDED));
    assert_eq!(keys.bindings(&def, RIGHT_HANDED), b);
    assert_eq!(
        keys.keymap(&def, LEFT_HANDED),
        Keymap::for_layout(&def, LEFT_HANDED)
    );
    // Restoring the defaults leaves no entry.
    keys.set(
        &def,
        RIGHT_HANDED,
        LayoutBindings::defaults(&def, RIGHT_HANDED),
    );
    assert_eq!(keys, PlayerKeys::default());
}

#[test]
fn saved_keys_round_trip() {
    let def = def();
    let mut keys = PlayerKeys::default();
    let mut b = keys.bindings(&def, LEFT_HANDED);
    b.bind(Confirm, 1, chord("Shift+,")).ok();
    b.bind(Info, 2, chord("\\")).ok();
    b.bind(Info, 0, chord("Kp+")).ok();
    b.clear(Action::Rewind, 0);
    keys.set(&def, LEFT_HANDED, b);
    let text = keys.to_ron();
    assert!(text.starts_with("PlayerKeys("), "{text}");
    assert!(text.contains("version: 2"), "{text}");
    assert!(text.contains("pad: None"), "{text}");
    assert!(
        text.contains("\"Confirm\": [Some(\"j\"), Some(\"Shift+,\"), None]"),
        "{text}"
    );
    assert!(!text.contains("RightHanded"), "{text}");
    assert!(!text.contains("Debug"), "{text}");
    assert_eq!(PlayerKeys::from_ron(&text, &def), (keys, vec![]));
}

#[test]
fn empty_saved_keys_round_trip() {
    let text = PlayerKeys::default().to_ron();
    assert_eq!(
        PlayerKeys::from_ron(&text, &def()),
        (PlayerKeys::default(), vec![])
    );
}

/// A saved config with one right-handed layout whose entries are `actions`
/// (RON map entries).
fn saved(actions: &str) -> String {
    format!("PlayerKeys(version: 1, layouts: {{\"RightHanded\": {{{actions}}}}})")
}

/// Loads `text`: the right-handed bindings and the warnings.
fn load(text: &str) -> (LayoutBindings, Vec<String>) {
    let (keys, warnings) = PlayerKeys::from_ron(text, &def());
    (keys.bindings(&def(), RIGHT_HANDED), warnings)
}

#[test]
fn unreadable_saved_keys_give_the_defaults_and_a_warning() {
    for text in [
        "",
        "garbage",
        "PlayerKeys(version: 1, layouts: 5)",
        "(version: 1)",
    ] {
        let (keys, warnings) = PlayerKeys::from_ron(text, &def());
        assert_eq!(keys, PlayerKeys::default(), "{text}");
        assert_eq!(warnings.len(), 1, "{text}: {warnings:?}");
        assert!(
            warnings[0].starts_with("unreadable, using the default keys: "),
            "{warnings:?}"
        );
    }
}

#[test]
fn another_version_gives_the_defaults_and_a_warning() {
    let text = saved("\"Info\": [Some(\"g\")]").replace("version: 1", "version: 3");
    let (keys, warnings) = PlayerKeys::from_ron(&text, &def());
    assert_eq!(keys, PlayerKeys::default());
    assert_eq!(warnings, ["version 3 isn't 2, using the default keys"]);
    let (keys, warnings) = PlayerKeys::from_ron(&text.replace("version: 3", "version: 0"), &def());
    assert_eq!(keys, PlayerKeys::default());
    assert_eq!(warnings, ["version 0 isn't 2, using the default keys"]);
}

#[test]
fn unknown_layouts_and_actions_are_dropped() {
    let text = "PlayerKeys(version: 1, layouts: {\"Vim\": {}, \"RightHanded\": \
                {\"Attack\": [Some(\"g\")], \"Debug\": [Some(\"h\")], \"Info\": [Some(\"g\")]}})";
    let (b, warnings) = load(text);
    assert_eq!(names(b.slots(Info)), ["g", "-", "-"]);
    assert_eq!(b.keymap(RepeatDef::default()).action(chord("h")), None);
    assert_eq!(
        warnings,
        [
            "RightHanded: unknown action \"Attack\" dropped",
            "RightHanded: unknown action \"Debug\" dropped",
            "unknown layout \"Vim\" dropped",
        ]
    );
}

#[test]
fn a_missing_action_keeps_its_default_slots() {
    let (b, warnings) = load(&saved("\"Info\": [None, Some(\"g\"), None]"));
    assert!(warnings.is_empty(), "{warnings:?}");
    assert_eq!(names(b.slots(Info)), ["-", "g", "-"]);
    assert_eq!(names(b.slots(Confirm)), ["f", "-", "-"]);
}

#[test]
fn bad_and_reserved_chords_and_extra_slots_are_dropped() {
    let (b, warnings) = load(&saved(
        "\"Info\": [Some(\"Nope\"), Some(\"Escape\"), Some(\"g\"), Some(\"h\")]",
    ));
    assert_eq!(names(b.slots(Info)), ["-", "-", "g"]);
    assert_eq!(warnings.len(), 3, "{warnings:?}");
    assert!(warnings[0].starts_with("RightHanded: Info: 4 slots, only the first 3 kept"));
    assert!(warnings[1].starts_with("RightHanded: Info: unknown key chord \"Nope\""));
    assert_eq!(
        warnings[2],
        "RightHanded: Info: Escape is a fixed key and can't be bound"
    );
    // Short slot lists are padded with empty slots.
    let (b, warnings) = load(&saved("\"Info\": [Some(\"g\")]"));
    assert!(warnings.is_empty());
    assert_eq!(names(b.slots(Info)), ["g", "-", "-"]);
}

#[test]
fn a_chord_in_two_stored_slots_stays_in_the_first() {
    let (b, warnings) = load(&saved(
        "\"Rewind\": [Some(\"g\")], \"Info\": [Some(\"e\"), Some(\"g\")]",
    ));
    // Action order: Info comes before Rewind, so Info keeps `g`.
    assert_eq!(names(b.slots(Info)), ["e", "g", "-"]);
    assert!(b.is_unmapped(Action::Rewind));
    assert_eq!(
        warnings,
        ["RightHanded: Rewind: g is already on another action"]
    );
}

#[test]
fn a_stored_chord_moves_off_a_default_slot() {
    // Info takes `r`, Rewind's default key; Rewind isn't stored.
    let (b, warnings) = load(&saved("\"Info\": [Some(\"r\")]"));
    assert!(warnings.is_empty(), "{warnings:?}");
    assert_eq!(names(b.slots(Info)), ["r", "-", "-"]);
    assert!(b.is_unmapped(Action::Rewind));
}

#[test]
fn a_required_action_left_unmapped_resets_the_layout() {
    // Info takes `f`, Confirm's only default key: Confirm has none left.
    let (b, warnings) = load(&saved("\"Info\": [Some(\"f\")]"));
    assert_eq!(b, right());
    assert_eq!(
        warnings,
        ["RightHanded: Confirm would have no key, using the default keys"]
    );
    // Stored with every slot empty: the same.
    let (b, warnings) = load(&saved("\"Cancel\": [None, None, None], \"EndTurn\": []"));
    assert_eq!(b, right());
    assert_eq!(
        warnings,
        ["RightHanded: Cancel, EndTurn would have no key, using the default keys"]
    );
}

#[test]
fn a_repaired_layout_equal_to_the_defaults_has_no_entry() {
    let (keys, warnings) = PlayerKeys::from_ron(&saved("\"Confirm\": [Some(\"f\")]"), &def());
    assert!(warnings.is_empty());
    assert!(!keys.is_custom(RIGHT_HANDED));
}

// --- Controller buttons (ticket 0816) ---------------------------------------

#[test]
fn find_names_the_slot_a_chord_is_in() {
    let mut b = right();
    assert_eq!(b.find(KeyGroup::General, chord("f")), Some((Confirm, 0)));
    assert_eq!(b.find(KeyGroup::General, chord("g")), None);
    assert_eq!(b.bind(Info, 2, chord("g")), Ok(None));
    assert_eq!(b.find(KeyGroup::General, chord("g")), Some((Info, 2)));
    b.clear(Info, 2);
    assert_eq!(b.find(KeyGroup::General, chord("g")), None);
}

fn pad() -> PadBindings {
    PadBindings::defaults(&def())
}

/// Button slots as names, `-` for empty.
fn buttons(slots: ButtonSlots) -> [String; SLOTS] {
    slots.map(|b| b.map_or_else(|| "-".to_owned(), |b| b.to_string()))
}

#[test]
fn default_buttons_fill_the_first_slots_in_file_order() {
    let b = pad();
    assert_eq!(buttons(b.slots(CursorUp)), ["DpadUp", "LeftStickUp", "-"]);
    assert_eq!(buttons(b.slots(Confirm)), ["South", "-", "-"]);
    assert_eq!(buttons(b.slots(Cancel)), ["East", "-", "-"]);
    assert_eq!(buttons(b.slots(Action::Select)), ["-", "-", "-"]);
    assert_eq!(buttons(b.slots(Debug)), ["-", "-", "-"]);
    assert!(b.unmapped_required().is_empty());
    assert!(b.is_unmapped(Action::Menu) && !b.is_unmapped(Info));
    assert_eq!(b.find(KeyGroup::General, Button::South), Some((Confirm, 0)));
    assert_eq!(
        b.find(KeyGroup::General, Button::LeftStickUp),
        Some((CursorUp, 1))
    );
    assert_eq!(b.find(KeyGroup::General, Button::RightStickUp), None);
    // Every default button, with its action.
    let def = def();
    let pairs: Vec<(ButtonChord, Action)> = b.pairs().collect();
    for action in Action::ALL {
        let bound: Vec<ButtonChord> = pairs
            .iter()
            .filter(|&&(_, a)| a == action)
            .map(|&(b, _)| b)
            .collect();
        assert_eq!(bound, def.buttons(action), "{action}");
    }
}

#[test]
fn a_bound_button_moves_and_any_button_can_be_bound() {
    let mut b = pad();
    // A free button: nothing moves. The right stick's directions count.
    assert_eq!(b.bind(Info, 1, Button::RightStickUp), Ok(None));
    assert_eq!(buttons(b.slots(Info)), ["North", "RightStickUp", "-"]);
    // Confirm's only button goes to Info: Confirm has none left.
    assert_eq!(b.bind(Info, 2, Button::South), Ok(Some((Confirm, 0))));
    assert_eq!(buttons(b.slots(Info)), ["North", "RightStickUp", "South"]);
    assert!(b.is_unmapped(Confirm));
    assert_eq!(b.unmapped_required(), [Confirm]);
    // Onto its own slot: nothing happens. Within one action: it moves.
    assert_eq!(b.bind(Info, 2, Button::South), Ok(None));
    assert_eq!(b.bind(Info, 0, Button::South), Ok(Some((Info, 2))));
    assert_eq!(buttons(b.slots(Info)), ["South", "RightStickUp", "-"]);
    // No button is reserved: every one can go in a slot.
    for &button in Button::ALL {
        assert!(b.bind(Action::Menu, 0, button).is_ok(), "{button}");
        assert_eq!(b.find(KeyGroup::General, button), Some((Action::Menu, 0)));
    }
    b.clear(Action::Menu, 0);
    assert!(b.is_unmapped(Action::Menu));
    let before = b.clone();
    b.clear(Action::Menu, SLOTS);
    b.clear(Debug, 0);
    assert_eq!(b, before);
}

#[test]
fn binding_a_button_to_debug_or_a_missing_slot_is_refused() {
    let mut b = pad();
    assert_eq!(
        b.bind(Debug, 0, Button::West),
        Err(BindError::NotRebindable(Debug))
    );
    assert_eq!(
        b.bind(Info, SLOTS, Button::West),
        Err(BindError::NoSuchSlot(SLOTS))
    );
    assert_eq!(b, pad());
}

#[test]
fn required_actions_without_a_button_are_listed_in_action_order() {
    let mut b = pad();
    b.clear(Action::EndTurn, 0);
    b.clear(Cancel, 0);
    b.clear(CursorUp, 0);
    // Cursor up still has the stick.
    assert_eq!(b.unmapped_required().len(), 2);
    b.clear(CursorUp, 1);
    let unmapped = b.unmapped_required();
    assert_eq!(unmapped.len(), 3);
    let order: Vec<Action> = Action::ALL
        .into_iter()
        .filter(|a| unmapped.contains(a))
        .collect();
    assert_eq!(unmapped, order);
    // Optional actions never count.
    b.clear(Info, 0);
    assert_eq!(b.unmapped_required().len(), 3);
}

/// One edit of the buttons' property test.
#[derive(Debug, Clone)]
enum PadOp {
    Bind(Action, usize, ButtonChord),
    Clear(Action, usize),
}

fn arb_pad_op() -> impl Strategy<Value = PadOp> {
    let action = prop::sample::select(Action::ALL.to_vec());
    let slot = 0..=SLOTS;
    let one = || prop::sample::select(Button::ALL);
    // A button, or two together.
    let button = (one(), prop::option::of(one())).prop_map(|(a, b)| {
        b.and_then(|b| ButtonChord::pair(a, b))
            .unwrap_or(ButtonChord::single(a))
    });
    prop_oneof![
        (action.clone(), slot.clone(), button).prop_map(|(a, i, b)| PadOp::Bind(a, i, b)),
        (action, slot).prop_map(|(a, i)| PadOp::Clear(a, i)),
    ]
}

proptest! {
    #[test]
    fn no_button_is_in_two_slots(ops in prop::collection::vec(arb_pad_op(), 0..80)) {
        let mut b = pad();
        for op in ops {
            match op {
                PadOp::Bind(a, i, button) => {
                    let before = b.clone();
                    if b.bind(a, i, button).is_ok() {
                        prop_assert_eq!(b.slots(a).get(i).copied().flatten(), Some(button));
                        prop_assert_eq!(b.find(a.group(), button), Some((a, i)));
                    } else {
                        prop_assert_eq!(&b, &before);
                    }
                }
                PadOp::Clear(a, i) => b.clear(a, i),
            }
            for group in KeyGroup::ALL {
                let all: Vec<ButtonChord> = Action::ALL
                    .into_iter()
                    .filter(|a| a.group() == group)
                    .flat_map(|a| b.slots(a).into_iter().flatten())
                    .collect();
                let mut unique = all.clone();
                unique.sort_unstable();
                unique.dedup();
                prop_assert_eq!(unique.len(), all.len(), "a button in two slots: {:?}", all);
            }
            // The keymap these give has exactly these buttons.
            let km = Keymap::new([], RepeatDef::default()).with_pad(b.pairs());
            for action in Action::ALL {
                let slots: Vec<ButtonChord> = b.slots(action).into_iter().flatten().collect();
                prop_assert_eq!(km.buttons_for(action), slots);
            }
        }
    }
}

/// The player's buttons with Unit info also on the right stick pressed in.
fn info_on_right_trigger() -> PadBindings {
    let mut b = pad();
    assert_eq!(b.bind(Info, 1, Button::RightStickPress), Ok(None));
    b
}

#[test]
fn the_buttons_are_shared_by_both_layouts_and_the_layout_picker() {
    let def = def();
    let mut keys = PlayerKeys::default();
    assert!(!keys.is_custom_pad());
    assert_eq!(keys.pad_bindings(&def), pad());
    let custom = info_on_right_trigger();
    keys.set_pad(&def, custom.clone());
    assert!(keys.is_custom_pad());
    assert_eq!(keys.pad_bindings(&def), custom);
    let keymaps = [
        keys.keymap(&def, RIGHT_HANDED),
        keys.keymap(&def, LEFT_HANDED),
        keys.layout_picker_keymap(&def),
    ];
    for km in &keymaps {
        assert_eq!(km.pad_action(Button::RightStickPress), Some(Info));
        assert_eq!(
            km.buttons_for(Info),
            [Button::North, Button::RightStickPress]
        );
        assert_eq!(km.pad_action(Button::South), Some(Confirm));
    }
    // The keys are each layout's own, untouched by the buttons.
    for layout in [RIGHT_HANDED, LEFT_HANDED] {
        assert!(!keys.is_custom(layout));
        let default_keys = LayoutBindings::defaults(&def, layout).keymap(def.repeat);
        assert_eq!(
            keys.keymap(&def, layout),
            default_keys.with_pad(custom.pairs())
        );
    }
    assert_eq!(keymaps[2].primary(Cancel), None);
    assert_eq!(
        keymaps[2].chords_for(Confirm),
        Keymap::layout_picker(&def).chords_for(Confirm)
    );
    // Changing a layout's keys leaves the buttons alone.
    let mut b = keys.bindings(&def, LEFT_HANDED);
    b.bind(Info, 0, chord("g")).ok();
    keys.set(&def, LEFT_HANDED, b);
    assert_eq!(keys.pad_bindings(&def), custom);
    // Buttons equal to the defaults are "no entry".
    keys.set_pad(&def, pad());
    assert!(!keys.is_custom_pad());
    assert_eq!(
        keys.keymap(&def, RIGHT_HANDED),
        Keymap::for_layout(&def, RIGHT_HANDED)
    );
    assert_eq!(keys.layout_picker_keymap(&def), Keymap::layout_picker(&def));
}

#[test]
fn saved_buttons_round_trip() {
    let def = def();
    let mut keys = PlayerKeys::default();
    let mut custom = info_on_right_trigger();
    custom.clear(Action::Rewind, 0);
    keys.set_pad(&def, custom.clone());
    let text = keys.to_ron();
    assert!(text.contains("version: 2"), "{text}");
    assert!(text.contains("layouts: {}"), "{text}");
    assert!(text.contains("pad: Some({"), "{text}");
    assert!(
        text.contains("\"Info\": [Some(\"North\"), Some(\"RightStickPress\"), None]"),
        "{text}"
    );
    assert!(text.contains("\"Rewind\": [None, None, None]"), "{text}");
    assert!(!text.contains("Debug"), "{text}");
    let (loaded, warnings) = PlayerKeys::from_ron(&text, &def);
    assert_eq!(warnings, Vec::<String>::new());
    assert_eq!(loaded, keys);
    assert_eq!(loaded.pad_bindings(&def), custom);
    // Keys and buttons together.
    let mut b = keys.bindings(&def, LEFT_HANDED);
    b.bind(Info, 1, chord("g")).ok();
    keys.set(&def, LEFT_HANDED, b);
    assert_eq!(PlayerKeys::from_ron(&keys.to_ron(), &def), (keys, vec![]));
}

/// A saved version 2 config with no layouts and the buttons `actions`.
fn saved_pad(actions: &str) -> String {
    format!("PlayerKeys(version: 2, layouts: {{}}, pad: Some({{{actions}}}))")
}

/// Loads `text`: the buttons and the warnings.
fn load_pad(text: &str) -> (PadBindings, Vec<String>) {
    let (keys, warnings) = PlayerKeys::from_ron(text, &def());
    (keys.pad_bindings(&def()), warnings)
}

#[test]
fn a_version_1_config_loads_its_keys_with_the_default_buttons() {
    let (keys, warnings) = PlayerKeys::from_ron(&saved("\"Info\": [Some(\"g\")]"), &def());
    assert!(warnings.is_empty(), "{warnings:?}");
    let b = keys.bindings(&def(), RIGHT_HANDED);
    assert_eq!(names(b.slots(Info)), ["g", "-", "-"]);
    assert!(!keys.is_custom_pad());
    assert_eq!(keys.pad_bindings(&def()), pad());
    // Saved again, it is version 2.
    assert!(keys.to_ron().contains("version: 2"));
    // Version 2 without buttons, or with `pad: None`: the defaults too.
    for text in [
        "PlayerKeys(version: 2, layouts: {})",
        "PlayerKeys(version: 2, layouts: {}, pad: None)",
    ] {
        let (keys, warnings) = PlayerKeys::from_ron(text, &def());
        assert!(warnings.is_empty(), "{text}: {warnings:?}");
        assert_eq!(keys, PlayerKeys::default(), "{text}");
    }
}

#[test]
fn unknown_actions_and_buttons_and_extra_slots_are_dropped() {
    let (b, warnings) = load_pad(&saved_pad(
        "\"Dance\": [Some(\"West\")], \"Debug\": [Some(\"West\")], \
         \"Info\": [Some(\"Turbo\"), Some(\"RightStickPress\"), None, Some(\"RightStickUp\")]",
    ));
    assert_eq!(buttons(b.slots(Info)), ["-", "RightStickPress", "-"]);
    assert_eq!(buttons(b.slots(Action::DangerZone)), ["West", "-", "-"]);
    assert_eq!(warnings.len(), 4, "{warnings:?}");
    assert_eq!(warnings[0], "controller: unknown action \"Dance\" dropped");
    assert_eq!(warnings[1], "controller: unknown action \"Debug\" dropped");
    assert_eq!(
        warnings[2],
        "controller: Info: 4 slots, only the first 3 kept"
    );
    assert!(
        warnings[3].starts_with("controller: Info: unknown button \"Turbo\""),
        "{warnings:?}"
    );
    // A short list is padded, and an action that isn't stored keeps its
    // default buttons.
    let (b, warnings) = load_pad(&saved_pad("\"Info\": [Some(\"RightStickPress\")]"));
    assert!(warnings.is_empty());
    assert_eq!(buttons(b.slots(Info)), ["RightStickPress", "-", "-"]);
    assert_eq!(buttons(b.slots(Confirm)), ["South", "-", "-"]);
}

#[test]
fn a_button_in_two_stored_slots_stays_in_the_first_and_moves_off_a_default() {
    let (b, warnings) = load_pad(&saved_pad(
        "\"Rewind\": [Some(\"RightStickPress\")], \
         \"Info\": [Some(\"North\"), Some(\"RightStickPress\")]",
    ));
    assert_eq!(buttons(b.slots(Info)), ["North", "RightStickPress", "-"]);
    assert!(b.is_unmapped(Action::Rewind));
    assert_eq!(
        warnings,
        ["controller: Rewind: RightStickPress is already on another action"]
    );
    // Info takes Rewind's default button; Rewind isn't stored.
    let (b, warnings) = load_pad(&saved_pad("\"Info\": [Some(\"LeftTrigger\")]"));
    assert!(warnings.is_empty(), "{warnings:?}");
    assert_eq!(buttons(b.slots(Info)), ["LeftTrigger", "-", "-"]);
    assert!(b.is_unmapped(Action::Rewind));
}

#[test]
fn a_required_action_left_without_a_button_resets_the_buttons_only() {
    // Info takes Confirm's only default button; the keys are kept.
    let text = "PlayerKeys(version: 2, layouts: {\"RightHanded\": {\"Info\": [Some(\"g\")]}}, \
                pad: Some({\"Info\": [Some(\"South\")]}))";
    let (keys, warnings) = PlayerKeys::from_ron(text, &def());
    assert_eq!(keys.pad_bindings(&def()), pad());
    assert!(!keys.is_custom_pad());
    assert!(keys.is_custom(RIGHT_HANDED));
    assert_eq!(
        warnings,
        ["controller: Confirm would have no button, using the default buttons"]
    );
    // Buttons repaired to the defaults have no entry.
    let same = saved_pad("\"Confirm\": [Some(\"South\")]");
    let (keys, warnings) = PlayerKeys::from_ron(&same, &def());
    assert!(warnings.is_empty());
    assert!(!keys.is_custom_pad());
}

#[test]
fn unreadable_buttons_give_the_defaults_for_everything() {
    let text = "PlayerKeys(version: 2, layouts: {}, pad: 5)";
    let (keys, warnings) = PlayerKeys::from_ron(text, &def());
    assert_eq!(keys, PlayerKeys::default());
    assert_eq!(warnings.len(), 1);
    assert!(warnings[0].starts_with("unreadable, using the default keys"));
}

/// Ticket 1106: a file saved by the build before input became generic
/// (custom keys in both layouts, custom buttons; written by `main`'s
/// `PlayerKeys::to_ron` at 2f10b42d) loads without a fix and saves back
/// byte for byte. `ron` ends lines as the platform does (CRLF on
/// Windows), and the repository keeps the file with LF, so the file is
/// compared in this platform's line endings.
#[test]
fn a_file_saved_before_input_was_generic_round_trips_byte_for_byte() {
    let lf = include_str!("saved_before_1106.ron").replace("\r\n", "\n");
    let saved = if cfg!(windows) {
        lf.replace('\n', "\r\n")
    } else {
        lf
    };
    let saved = saved.as_str();
    let def = def();
    let (keys, warnings) = PlayerKeys::from_ron(saved, &def);
    assert_eq!(warnings, Vec::<String>::new());
    assert!(keys.is_custom(RIGHT_HANDED) && keys.is_custom(LEFT_HANDED));
    assert!(keys.is_custom_pad());
    let right = keys.bindings(&def, RIGHT_HANDED);
    assert_eq!(
        right.slots(Info),
        [Some(chord("e")), Some(chord("g")), None]
    );
    assert_eq!(
        right.slots(Action::Select),
        [Some(chord("Shift+f")), None, None]
    );
    assert!(right.is_unmapped(Action::DialogueLog));
    let left = keys.bindings(&def, LEFT_HANDED);
    assert_eq!(
        left.slots(Action::ZoomMap),
        [Some(chord("Kp+")), None, None]
    );
    let pad = keys.pad_bindings(&def);
    assert_eq!(pad.slots(Info)[1], Some(Button::RightStickPress.into()));
    assert_eq!(keys.to_ron(), saved);
}
