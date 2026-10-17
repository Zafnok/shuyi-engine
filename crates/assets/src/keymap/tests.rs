//! The keymap format's tests, over the [`sample`](super::sample) game's
//! actions and keymap file.
use proptest::prelude::*;

use super::sample::{Action, KeyGroup, LEFT_HANDED, RIGHT_HANDED};
use super::*;

/// The sample game's validated keymap.
type KeymapDef = super::KeymapDef<Action>;
/// One of the sample game's chord lookups.
type Bindings = super::Bindings<Action>;

#[test]
fn the_sample_keymap_loads_with_its_two_layouts() {
    let k = sample::keymap();
    assert_eq!(
        k.layout_names,
        [
            (RIGHT_HANDED, "layout_picker.right_handed".to_owned()),
            (LEFT_HANDED, "layout_picker.left_handed".to_owned()),
        ]
    );
    assert_eq!(
        k.bindings(RIGHT_HANDED, KeyGroup::General)
            .and_then(|b| b.get(&Chord::plain(Key::F)).copied()),
        Some(Action::Confirm)
    );
    assert_eq!(<Action as ActionSet>::ALL, Action::ALL);
    assert_eq!(Action::for_ui(UiAction::Cancel), Some(Action::Cancel));
    assert_eq!(
        <Action as ActionSet>::fixed_keys(),
        [(Chord::plain(Key::Escape), Action::Cancel)]
    );
}

#[test]
fn chord_parse_plain_and_shifted() {
    assert_eq!(Chord::parse("h"), Ok(Chord::plain(Key::H)));
    assert_eq!(Chord::parse("Shift+h"), Ok(Chord::shifted(Key::H)));
    assert_eq!(Chord::parse("Shift+Tab"), Ok(Chord::shifted(Key::Tab)));
    assert_eq!(Chord::parse("F12"), Ok(Chord::plain(Key::F12)));
    assert_eq!(Chord::parse("7"), Ok(Chord::plain(Key::Digit7)));
    assert_eq!(Chord::parse(";"), Ok(Chord::plain(Key::Semicolon)));
    assert_eq!(Chord::parse("Shift+;"), Ok(Chord::shifted(Key::Semicolon)));
    assert_eq!("Left".parse::<Chord>(), Ok(Chord::plain(Key::Left)));
}

#[test]
fn chord_parse_rejects_garbage() {
    for bad in [
        "",
        "Shift+",
        "shift+h",
        "Ctrl+h",
        "hh",
        "F13",
        "left",
        "Shift+Shift+h",
    ] {
        assert!(Chord::parse(bad).is_err(), "{bad:?} should not parse");
    }
}

#[test]
fn chord_parse_hints_at_uppercase_letters() {
    let err = Chord::parse("H").err().unwrap_or_default();
    assert!(err.contains("\"Shift+h\""), "{err}");
    for no_hint in ["Hh", "HH", "?"] {
        let err = Chord::parse(no_hint).err().unwrap_or_default();
        assert!(!err.contains("shifted letter"), "{err}");
    }
}

#[test]
fn chord_display() {
    assert_eq!(Chord::plain(Key::Digit0).to_string(), "0");
    assert_eq!(Chord::shifted(Key::L).to_string(), "Shift+l");
    assert_eq!(Chord::plain(Key::Backspace).to_string(), "Backspace");
    assert_eq!(Key::Escape.to_string(), "Escape");
    assert_eq!(Chord::plain(Key::Semicolon).to_string(), ";");
    assert_eq!(Key::from_name(";"), Some(Key::Semicolon));
}

fn arb_chord() -> impl Strategy<Value = Chord> {
    (prop::sample::select(Key::ALL), any::<bool>()).prop_map(|(key, shift)| Chord { key, shift })
}

proptest! {
    #[test]
    fn chord_round_trips(c in arb_chord()) {
        prop_assert_eq!(Chord::parse(&c.to_string()), Ok(c));
    }
}

/// One layout block with every action but `skip` listed as `[]`, then
/// `extra`.
fn block(name: &str, skip: &str, extra: &str) -> String {
    let lines: Vec<String> = Action::ALL
        .iter()
        .filter(|a| a.name() != skip)
        .map(|a| format!("            \"{a}\": [],\n"))
        .collect();
    format!(
        "        (id: \"{name}\", name: \"layout.{name}\", keys: {{\n{}{extra}        }}),\n",
        lines.concat()
    )
}

/// A keymap source holding `blocks` as its layouts and an empty
/// `layout_picker`.
fn source_of(blocks: &[String]) -> String {
    source_with_picker(blocks, "")
}

/// A keymap source holding `blocks` as its layouts, then `repeat`, then
/// `picker` as the `layout_picker` section's entries (on the line after
/// `layout_picker: {`), then a `pad` section with every action `[]` and
/// the default `stick`.
fn source_with_picker(blocks: &[String], picker: &str) -> String {
    source_with_pad(blocks, picker, "", "")
}

/// [`source_with_picker`], except that the `pad` section leaves out
/// `skip` and has `extra` spliced in after the other actions.
fn source_with_pad(blocks: &[String], picker: &str, skip: &str, extra: &str) -> String {
    let pad: Vec<String> = Action::ALL
        .iter()
        .filter(|a| a.name() != skip)
        .map(|a| format!("        \"{a}\": [],\n"))
        .collect();
    format!(
        "(\n    layouts: [\n{}    ],\n    repeat: (delay_ms: 300, interval_ms: 55),\n    \
         layout_picker: {{\n{picker}    }},\n    pad: {{\n{}{extra}    }},\n    stick: \
         (press_percent: 50, release_percent: 35),\n)",
        blocks.concat(),
        pad.concat()
    )
}

/// A keymap source with empty layouts and picker, whose `pad` section
/// leaves out `skip` and has `extra` spliced in.
fn pad_source(skip: &str, extra: &str) -> String {
    source_with_pad(&empty_layouts(), "", skip, extra)
}

/// Line of the `pad` section's entry number `n` (0-based) in
/// [`pad_source`].
fn pad_line(n: usize) -> Option<u32> {
    u32::try_from(3 + 2 * (Action::ALL.len() + 2) + 4 + 1 + n).ok()
}

/// A complete valid keymap source, except that the `LeftHanded` layout
/// (listed first, starting on line 3) leaves out `skip` and has `extra`
/// spliced in. `RightHanded` follows with every action `[]`.
fn source_with(skip: &str, extra: &str) -> String {
    source_of(&[
        block("LeftHanded", skip, extra),
        block("RightHanded", "", ""),
    ])
}

fn errors(src: &str) -> Vec<String> {
    KeymapDef::from_source("k.ron", src)
        .err()
        .unwrap_or_default()
        .iter()
        .map(ToString::to_string)
        .collect()
}

fn left(k: &KeymapDef) -> Bindings {
    k.bindings(LEFT_HANDED, KeyGroup::General)
        .unwrap_or_default()
}

#[test]
fn button_names_round_trip() {
    assert_eq!(Button::ALL.len(), 16 + 4 + 4);
    for (i, &b) in Button::ALL.iter().enumerate() {
        assert_eq!(b as usize, i);
        assert_eq!(Button::parse(&b.to_string()), Ok(b));
        assert_eq!(b.name().parse::<Button>(), Ok(b));
        assert_eq!(Button::from_name(b.name()), Some(b));
    }
    assert_eq!(Button::South.name(), "South");
    assert_eq!(Button::RightStickRight.to_string(), "RightStickRight");
    // The browser Gamepad API's standard mapping order.
    let standard = [
        "South",
        "East",
        "West",
        "North",
        "LeftShoulder",
        "RightShoulder",
        "LeftTrigger",
        "RightTrigger",
        "Select",
        "Start",
        "LeftStickPress",
        "RightStickPress",
        "DpadUp",
        "DpadDown",
        "DpadLeft",
        "DpadRight",
    ];
    let names: Vec<&str> = Button::ALL[..16].iter().map(|b| b.name()).collect();
    assert_eq!(names, standard);
}

#[test]
fn button_parse_rejects_garbage() {
    for bad in ["", "south", "A", "Shift+South", "South ", "f"] {
        assert_eq!(Button::from_name(bad), None, "{bad:?}");
        let err = Button::parse(bad).err().unwrap_or_default();
        assert!(
            err.starts_with(&format!(
                "unknown button \"{bad}\"; known buttons: South, East,"
            )),
            "{err}"
        );
    }
}

#[test]
fn pad_section_loads_in_file_order() {
    let extra = "        \"CursorUp\": [\"LeftStickUp\", \"DpadUp\", \"RightStickUp\"],\n";
    let k = KeymapDef::from_source("k.ron", &pad_source("CursorUp", extra)).unwrap_or_default();
    assert_eq!(
        k.buttons(Action::CursorUp),
        [Button::LeftStickUp, Button::DpadUp, Button::RightStickUp]
    );
    assert_eq!(k.buttons(Action::Confirm), []);
    assert_eq!(k.pad.len(), Action::ALL.len());
    assert_eq!(k.stick, StickDef::default());
    assert_eq!(KeymapDef::default().buttons(Action::CursorUp), []);
}

#[test]
fn pad_button_on_two_actions_is_error_with_line() {
    let src = pad_source("Info", "        \"Info\": [\"South\"],\n");
    // Confirm in the pad section: the last `"Confirm": []` of the file.
    let empty = "\"Confirm\": []";
    let at = src.rfind(empty).unwrap_or(0);
    let rest = &src[at + empty.len()..];
    let src = format!("{}\"Confirm\": [\"South\"]{rest}", &src[..at]);
    let errs = KeymapDef::from_source("k.ron", &src)
        .err()
        .unwrap_or_default();
    assert_eq!(errs.len(), 1, "{errs:?}");
    assert_eq!(
        errs[0].message,
        "pad: button \"South\" is bound to both \"Confirm\" and \"Info\""
    );
    assert_eq!(errs[0].line, pad_line(Action::ALL.len() - 1));
}

#[test]
fn pad_button_listed_twice_is_error() {
    let errs = errors(&pad_source(
        "Info",
        "        \"Info\": [\"West\", \"West\"],\n",
    ));
    assert_eq!(errs.len(), 1, "{errs:?}");
    assert!(
        errs[0].ends_with("pad: button \"West\" is listed twice for \"Info\""),
        "{}",
        errs[0]
    );
}

#[test]
fn a_chord_may_be_on_one_action_of_each_key_group() {
    // controls.md, Dialogue log: the log has its own group.
    assert_eq!(Action::DialogueLog.group(), KeyGroup::DialogueLog);
    let others = Action::ALL
        .into_iter()
        .filter(|&a| a != Action::DialogueLog);
    assert!(others.into_iter().all(|a| a.group() == KeyGroup::General));
    let shared = "            \"Confirm\": [\"f\"],
        \"DialogueLog\": [\"f\"],
";
    let src = source_of(&[
        block("LeftHanded", "Confirm", "").replace(
            "            \"DialogueLog\": [],
",
            shared,
        ),
        block("RightHanded", "", ""),
    ]);
    let k = KeymapDef::from_source("k.ron", &src).unwrap_or_default();
    let f = Chord::plain(Key::F);
    assert_eq!(left(&k).get(&f), Some(&Action::Confirm));
    let log = k.bindings(LEFT_HANDED, KeyGroup::DialogueLog);
    assert_eq!(log.unwrap_or_default().get(&f), Some(&Action::DialogueLog));
    // In one group they still clash.
    let errs = errors(&source_with(
        "Info",
        "            \"Info\": [\"Up\"],
        \"CursorUp\": [\"Up\"],
",
    ));
    assert!(
        errs.iter().any(|e| e.contains("is bound to both")),
        "{errs:?}"
    );
}

#[test]
fn a_button_may_be_on_one_action_of_each_key_group() {
    let mut src = pad_source(
        "Info",
        "        \"Info\": [\"West\"],
",
    );
    // The pad section is last: its DialogueLog entry is the last one.
    let log = "\"DialogueLog\": []";
    if let Some(at) = src.rfind(log) {
        src.replace_range(at..at + log.len(), "\"DialogueLog\": [\"West\"]");
    }
    let k = KeymapDef::from_source("k.ron", &src).unwrap_or_default();
    assert_eq!(k.buttons(Action::Info), [Button::West]);
    assert_eq!(k.buttons(Action::DialogueLog), [Button::West]);
}

#[test]
fn pad_button_on_debug_is_error_with_line() {
    let src = pad_source("Debug", "        \"Debug\": [\"RightTrigger\"],\n");
    let errs = KeymapDef::from_source("k.ron", &src)
        .err()
        .unwrap_or_default();
    assert_eq!(errs.len(), 1, "{errs:?}");
    assert_eq!(
        errs[0].message,
        "pad: action \"Debug\" can't have a button: it can't be rebound, so it is keyboard only"
    );
    assert_eq!(errs[0].line, pad_line(Action::ALL.len() - 1));
    // Listed empty, it is fine.
    assert!(errors(&pad_source("", "")).is_empty());
}

#[test]
fn pad_unknown_button_and_action_are_errors() {
    let errs = errors(&pad_source(
        "Info",
        "        \"Info\": [\"f\"],\n        \"Attack\": [],\n",
    ));
    assert_eq!(errs.len(), 2, "{errs:?}");
    assert!(
        errs[0].contains("pad: unknown action \"Attack\""),
        "{}",
        errs[0]
    );
    assert!(
        errs[1].contains("pad: action \"Info\": unknown button \"f\"; known buttons: South"),
        "{}",
        errs[1]
    );
}

#[test]
fn pad_missing_action_is_error() {
    let errs = errors(&pad_source("Rewind", ""));
    assert_eq!(
        errs,
        vec!["k.ron: pad: missing action \"Rewind\" (list it with [] to leave it unbound)"]
    );
}

#[test]
fn pad_more_than_three_buttons_is_error() {
    let four = "        \"Info\": [\"South\", \"East\", \"West\", \"North\"],\n";
    let errs = KeymapDef::from_source("k.ron", &pad_source("Info", four))
        .err()
        .unwrap_or_default();
    assert_eq!(errs.len(), 1, "{errs:?}");
    assert_eq!(
        errs[0].message,
        "pad: action \"Info\" has 4 buttons; at most 3 are allowed"
    );
    assert_eq!(errs[0].line, pad_line(Action::ALL.len() - 1));
    let three = "        \"Info\": [\"South\", \"East\", \"West\"],\n";
    let k = KeymapDef::from_source("k.ron", &pad_source("Info", three)).unwrap_or_default();
    assert_eq!(k.buttons(Action::Info).len(), 3);
}

#[test]
fn missing_pad_or_stick_section_is_error() {
    let src = pad_source("", "");
    let stick = "    stick: (press_percent: 50, release_percent: 35),\n";
    assert!(src.contains(stick));
    assert_eq!(errors(&src.replace(stick, "")).len(), 1);
    let at = src.find("    pad: {").unwrap_or(0);
    let end = src.find("    stick:").unwrap_or(0);
    assert_eq!(errors(&format!("{}{}", &src[..at], &src[end..])).len(), 1);
}

#[test]
fn stick_thresholds_must_be_in_order_and_in_range() {
    let stick_line = u32::try_from(3 + 2 * (Action::ALL.len() + 2) + 6 + Action::ALL.len()).ok();
    for (press, release, ok) in [
        (50, 35, true),
        (100, 99, true),
        (2, 1, true),
        (101, 35, false),
        (50, 50, false),
        (50, 51, false),
        (50, 0, false),
    ] {
        let src = pad_source("", "").replace(
            "press_percent: 50, release_percent: 35",
            &format!("press_percent: {press}, release_percent: {release}"),
        );
        match KeymapDef::from_source("k.ron", &src) {
            Ok(k) => {
                assert!(ok, "{press}/{release} should be refused");
                assert_eq!(
                    (k.stick.press_percent, k.stick.release_percent),
                    (press, release)
                );
            }
            Err(errs) => {
                assert!(!ok, "{press}/{release}: {errs:?}");
                assert_eq!(errs.len(), 1);
                assert!(errs[0].message.starts_with("stick release_percent"));
                assert_eq!(errs[0].line, stick_line);
            }
        }
    }
}

#[test]
fn valid_keymap_loads() {
    let src = source_with("Confirm", "        \"Confirm\": [\"f\", \"Space\"],\n");
    let k = KeymapDef::from_source("k.ron", &src).unwrap_or_default();
    let b = left(&k);
    assert_eq!(b.get(&Chord::plain(Key::F)), Some(&Action::Confirm));
    assert_eq!(b.get(&Chord::plain(Key::Space)), Some(&Action::Confirm));
    assert_eq!(b.len(), 2);
    let general = k.bindings(RIGHT_HANDED, KeyGroup::General);
    assert_eq!(general.map(|b| b.len()), Some(0));
    assert_eq!(
        k.chords(LEFT_HANDED, Action::Confirm),
        [Chord::plain(Key::F), Chord::plain(Key::Space)]
    );
    assert_eq!(k.chords(RIGHT_HANDED, Action::Confirm), []);
    assert_eq!(k.layouts.len(), 2);
    assert_eq!(k.repeat, RepeatDef::default());
}

#[test]
fn layouts_are_independent() {
    // The same chord may mean different actions in different layouts.
    let src = source_of(&[
        block("LeftHanded", "Confirm", "\"Confirm\": [\"f\"],\n"),
        block("RightHanded", "Cancel", "\"Cancel\": [\"f\"],\n"),
    ]);
    let k = KeymapDef::from_source("k.ron", &src).unwrap_or_default();
    let get = |l| {
        k.bindings(l, KeyGroup::General)
            .and_then(|b| b.get(&Chord::plain(Key::F)).copied())
    };
    assert_eq!(get(LEFT_HANDED), Some(Action::Confirm));
    assert_eq!(get(RIGHT_HANDED), Some(Action::Cancel));
}

#[test]
fn unknown_action_is_error_with_line() {
    let src = source_with("", "        \"Attack\": [\"f\"],\n");
    let errs = KeymapDef::from_source("k.ron", &src)
        .err()
        .unwrap_or_default();
    assert_eq!(errs.len(), 1);
    assert!(
        errs[0]
            .message
            .starts_with("layout \"LeftHanded\": unknown action \"Attack\""),
        "{}",
        errs[0].message
    );
    assert!(errs[0].message.contains("Confirm"));
    let line = u32::try_from(Action::ALL.len() + 4).unwrap_or(0);
    assert_eq!(errs[0].line, Some(line));
}

#[test]
fn bad_chord_is_error() {
    let src = source_with("Info", "        \"Info\": [\"s\", \"Ctrl+i\"],\n");
    let errs = errors(&src);
    assert_eq!(errs.len(), 1);
    assert!(errs[0].contains("layout \"LeftHanded\""), "{}", errs[0]);
    assert!(errs[0].contains("\"Info\""), "{}", errs[0]);
    assert!(errs[0].contains("\"Ctrl+i\""), "{}", errs[0]);
}

#[test]
fn conflicting_chord_is_error() {
    let src = source_with("Info", "        \"Info\": [\"f\"],\n").replacen(
        "\"Confirm\": []",
        "\"Confirm\": [\"f\"]",
        1,
    );
    let errs = errors(&src);
    assert_eq!(errs.len(), 1);
    assert!(
        errs[0].contains(
            "layout \"LeftHanded\": chord \"f\" is bound to both \"Confirm\" and \"Info\""
        ),
        "{}",
        errs[0]
    );
}

#[test]
fn conflict_in_the_second_layout_points_at_its_own_line() {
    let src = source_of(&[
        block("LeftHanded", "", ""),
        block("RightHanded", "Info", "\"Info\": [\"d\", \"d\"],\n"),
    ]);
    let errs = KeymapDef::from_source("k.ron", &src)
        .err()
        .unwrap_or_default();
    assert_eq!(errs.len(), 1);
    assert!(
        errs[0]
            .message
            .starts_with("layout \"RightHanded\": chord \"d\" is listed twice"),
        "{}",
        errs[0].message
    );
    let n = Action::ALL.len();
    // Line 3 opens LeftHanded; its n actions and closing brace follow,
    // then RightHanded's line and its n - 1 other actions.
    let line = u32::try_from(3 + n + 1 + 1 + (n - 1) + 1).unwrap_or(0);
    assert_eq!(errs[0].line, Some(line));
}

#[test]
fn duplicate_chord_in_one_action_is_error() {
    let src = source_with("Info", "        \"Info\": [\"s\", \"s\"],\n");
    let errs = errors(&src);
    assert_eq!(errs.len(), 1);
    assert!(errs[0].contains("listed twice"), "{}", errs[0]);
}

#[test]
fn shift_makes_a_distinct_chord() {
    let src = source_with("", "")
        .replacen("\"CursorLeft\": []", "\"CursorLeft\": [\"h\"]", 1)
        .replacen("\"Info\": []", "\"Info\": [\"Shift+h\"]", 1);
    let k = KeymapDef::from_source("k.ron", &src).unwrap_or_default();
    assert_eq!(left(&k).len(), 2);
}

#[test]
fn missing_action_is_error() {
    let errs = errors(&source_with("Debug", ""));
    assert_eq!(
        errs,
        vec![
            "k.ron: layout \"LeftHanded\": missing action \"Debug\" (list it with [] to \
             leave it unbound)"
        ]
    );
}

#[test]
fn a_keymap_needs_a_layout() {
    let errs = errors(&source_of(&[]));
    assert_eq!(errs, vec!["k.ron:2: layouts: list at least one layout"]);
}

#[test]
fn a_keymap_has_the_layouts_it_lists_in_file_order() {
    let k = KeymapDef::from_source("k.ron", &source_of(&[block("Vim", "", "")]))
        .unwrap_or_else(|e| panic!("{e:?}"));
    let vim = Layout::named("Vim");
    assert_eq!(k.layout_ids().collect::<Vec<_>>(), [vim]);
    assert_eq!(k.layout_text(vim), Some("layout.Vim"));
    assert_eq!(k.layout_text(RIGHT_HANDED), None);
    assert!(k.has_layout(vim) && !k.has_layout(LEFT_HANDED));
    let k =
        KeymapDef::from_source("k.ron", &source_with("", "")).unwrap_or_else(|e| panic!("{e:?}"));
    assert_eq!(k.first_layout(), Some(LEFT_HANDED));
    assert_eq!(
        k.layout_ids().collect::<Vec<_>>(),
        [LEFT_HANDED, RIGHT_HANDED]
    );
}

#[test]
fn a_layout_listed_twice_or_badly_named_is_error_with_line() {
    let src = source_of(&[
        block("LeftHanded", "", ""),
        block("RightHanded", "", ""),
        block("LeftHanded", "Bogus", ""),
        block("Two words", "", ""),
    ]);
    let errs = KeymapDef::from_source("k.ron", &src)
        .err()
        .unwrap_or_default();
    // Only the layouts themselves: their contents aren't checked.
    assert_eq!(errs.len(), 2);
    assert_eq!(errs[0].message, "layout \"LeftHanded\" is listed twice");
    let line = |n: usize| u32::try_from(3 + n * (Action::ALL.len() + 2)).ok();
    assert_eq!(errs[0].line, line(2));
    assert_eq!(
        errs[1].message,
        "layout id \"Two words\" must be 1 to 24 ASCII letters, digits or _"
    );
    assert_eq!(errs[1].line, line(3));
}

#[test]
fn zero_interval_is_error() {
    let src = source_with("", "").replace("interval_ms: 55", "interval_ms: 0");
    let errs = KeymapDef::from_source("k.ron", &src)
        .err()
        .unwrap_or_default();
    assert_eq!(errs.len(), 1);
    assert!(errs[0].message.contains("interval_ms"), "{}", errs[0]);
    let line = u32::try_from(3 + 2 * (Action::ALL.len() + 2) + 1).unwrap_or(0);
    assert_eq!(errs[0].line, Some(line));
}

#[test]
fn all_errors_reported_together() {
    let src = source_of(&[
        block(
            "LeftHanded",
            "Confirm",
            "        \"Confirm\": [\"Nope\", \"f\"],\n        \"Bogus\": [],\n",
        )
        .replacen("\"Cancel\": []", "\"Cancel\": [\"f\"]", 1),
        block("RightHanded", "Debug", ""),
        block("LeftHanded", "", ""),
    ])
    .replace("interval_ms: 55", "interval_ms: 0");
    assert_eq!(errors(&src).len(), 6);
}

#[test]
fn old_single_layout_format_is_rejected() {
    let src = "(\n    bindings: {},\n    repeat: (delay_ms: 1, interval_ms: 1),\n)";
    assert_eq!(errors(src).len(), 1);
}

/// Two complete layouts, every action `[]`.
fn empty_layouts() -> [String; 2] {
    [block("LeftHanded", "", ""), block("RightHanded", "", "")]
}

/// Line of the `layout_picker` section's first entry in
/// [`source_with_picker`] with [`empty_layouts`].
fn first_picker_line() -> Option<u32> {
    u32::try_from(3 + 2 * (Action::ALL.len() + 2) + 3).ok()
}

#[test]
fn layout_picker_section_loads_and_leaves_unlisted_actions_unbound() {
    let picker = "        \"CursorUp\": [\"Up\", \"w\"],\n        \"Confirm\": [\"Enter\"],\n";
    let src = source_with_picker(&empty_layouts(), picker);
    let k = KeymapDef::from_source("k.ron", &src).unwrap_or_default();
    let expected: Bindings = [
        (Chord::plain(Key::Up), Action::CursorUp),
        (Chord::plain(Key::W), Action::CursorUp),
        (Chord::plain(Key::Enter), Action::Confirm),
    ]
    .into();
    assert_eq!(k.layout_picker, expected);
}

#[test]
fn layout_picker_unknown_action_is_error_with_line() {
    let src = source_with_picker(&empty_layouts(), "        \"Attack\": [\"f\"],\n");
    let errs = KeymapDef::from_source("k.ron", &src)
        .err()
        .unwrap_or_default();
    assert_eq!(errs.len(), 1);
    assert!(
        errs[0]
            .message
            .starts_with("layout_picker: unknown action \"Attack\""),
        "{}",
        errs[0].message
    );
    assert_eq!(errs[0].line, first_picker_line());
}

#[test]
fn layout_picker_bad_chord_is_error_with_line() {
    let src = source_with_picker(&empty_layouts(), "        \"Confirm\": [\"Nope\"],\n");
    let errs = KeymapDef::from_source("k.ron", &src)
        .err()
        .unwrap_or_default();
    assert_eq!(errs.len(), 1);
    assert!(
        errs[0]
            .message
            .starts_with("layout_picker: action \"Confirm\": "),
        "{}",
        errs[0].message
    );
    assert_eq!(errs[0].line, first_picker_line());
}

#[test]
fn layout_picker_conflicting_chord_is_error() {
    let picker = "        \"Confirm\": [\"f\"],\n        \"Cancel\": [\"f\"],\n";
    let errs = errors(&source_with_picker(&empty_layouts(), picker));
    assert_eq!(errs.len(), 1, "{errs:?}");
    assert!(
        errs[0].contains("layout_picker: chord \"f\" is bound to both"),
        "{}",
        errs[0]
    );
}

#[test]
fn missing_layout_picker_section_is_error() {
    let src = source_of(&empty_layouts()).replace("    layout_picker: {\n    },\n", "");
    assert!(!src.contains("layout_picker"));
    assert_eq!(errors(&src).len(), 1);
}

#[test]
fn syntax_error_is_positioned() {
    let errs = KeymapDef::from_source("k.ron", "(\n  layouts: [\n  (id: \"a\" name: \"b\") ],\n)")
        .err()
        .unwrap_or_default();
    assert_eq!(errs.len(), 1);
    assert_eq!(errs[0].line, Some(3));
}

#[test]
fn line_of_quoted_finds_keys_and_fields() {
    let src = "(\n \"Info\": [],\n    interval_ms: 3,\n \"Info\": [],\n)";
    assert_eq!(line_of_quoted(src, 0, "Info"), Some(2));
    assert_eq!(line_of_quoted(src, 1, "Info"), Some(2));
    assert_eq!(line_of_quoted(src, 2, "Info"), Some(4));
    assert_eq!(line_of_quoted(src, 0, "interval_ms"), Some(3));
    assert_eq!(line_of_quoted(src, 0, "nope"), None);
    assert_eq!(line_of_quoted(src, 9, "Info"), None);
}

#[test]
fn layout_names_round_trip() {
    for l in [RIGHT_HANDED, LEFT_HANDED, Layout::named("a_1")] {
        assert_eq!(Layout::from_name(l.name()), Some(l));
        assert_eq!(Layout::from_name(&l.to_string()), Some(l));
    }
    assert_eq!(RIGHT_HANDED.name(), "RightHanded");
    assert_eq!(LEFT_HANDED.name(), "LeftHanded");
    assert_eq!(format!("{RIGHT_HANDED:?}"), "Layout(RightHanded)");
    assert_ne!(Layout::from_name("lefthanded"), Some(LEFT_HANDED));
    let longest = "x".repeat(LAYOUT_NAME_MAX);
    assert_eq!(
        Layout::from_name(&longest).map(|l| l.to_string()),
        Some(longest.clone())
    );
    for bad in ["", "Left-handed", "two words", "é", &format!("{longest}x")] {
        assert_eq!(Layout::from_name(bad), None, "{bad:?}");
    }
    // Ordered by name, like the saved file's map.
    assert!(LEFT_HANDED < RIGHT_HANDED && Layout::named("a") < Layout::named("ab"));
}

#[test]
fn more_than_three_chords_on_one_action_is_error() {
    let src = source_with("Info", "        \"Info\": [\"a\", \"b\", \"c\", \"e\"],\n");
    let errs = KeymapDef::from_source("k.ron", &src)
        .err()
        .unwrap_or_default();
    assert_eq!(errs.len(), 1);
    assert_eq!(
        errs[0].message,
        "layout \"LeftHanded\": action \"Info\" has 4 keys; at most 3 are allowed"
    );
    assert_eq!(errs[0].line, u32::try_from(Action::ALL.len() + 3).ok());
    // Three is fine.
    let src = source_with("Info", "        \"Info\": [\"a\", \"b\", \"c\"],\n");
    let k = KeymapDef::from_source("k.ron", &src).unwrap_or_default();
    assert_eq!(k.chords(LEFT_HANDED, Action::Info).len(), 3);
}

#[test]
fn the_layout_picker_may_list_more_than_three_chords() {
    let picker = "        \"Confirm\": [\"f\", \"j\", \"Enter\", \"Space\"],\n";
    let src = source_with_picker(&empty_layouts(), picker);
    let k = KeymapDef::from_source("k.ron", &src).unwrap_or_default();
    assert_eq!(k.layout_picker.len(), 4);
}

#[test]
fn escape_and_delete_are_errors_anywhere() {
    for chord in ["Escape", "Delete"] {
        let extra = format!("        \"Cancel\": [\"d\", \"{chord}\"],\n");
        let errs = errors(&source_with("Cancel", &extra));
        assert_eq!(
            errs,
            vec![format!(
                "k.ron:{}: layout \"LeftHanded\": action \"Cancel\": \"{chord}\" can't be \
                 bound here: Esc and Delete are fixed, see controls.md",
                Action::ALL.len() + 3
            )],
            "{chord}"
        );
        let picker = format!("        \"Cancel\": [\"{chord}\"],\n");
        let errs = errors(&source_with_picker(&empty_layouts(), &picker));
        assert_eq!(errs.len(), 1, "{chord}");
        assert!(
            errs[0].contains("layout_picker: action \"Cancel\""),
            "{}",
            errs[0]
        );
        assert!(errs[0].contains("Esc and Delete are fixed"), "{}", errs[0]);
    }
}

#[test]
fn shifted_escape_and_delete_are_ordinary_chords() {
    let extra = "        \"Info\": [\"Shift+Escape\", \"Shift+Delete\"],
";
    let k = KeymapDef::from_source("k.ron", &source_with("Info", extra)).unwrap_or_default();
    assert_eq!(
        k.chords(LEFT_HANDED, Action::Info),
        [Chord::shifted(Key::Escape), Chord::shifted(Key::Delete)]
    );
}

#[test]
fn chords_keep_the_file_order() {
    let src = source_with(
        "Info",
        "        \"Info\": [\"Space\", \"e\", \"Shift+a\"],\n",
    );
    let k = KeymapDef::from_source("k.ron", &src).unwrap_or_default();
    let chords: Vec<String> = k
        .chords(LEFT_HANDED, Action::Info)
        .iter()
        .map(ToString::to_string)
        .collect();
    assert_eq!(chords, ["Space", "e", "Shift+a"]);
    assert_eq!(k.chords(LEFT_HANDED, Action::Debug), []);
    assert_eq!(KeymapDef::default().chords(LEFT_HANDED, Action::Info), []);
}

#[test]
fn reserved_chords() {
    let reserved: Vec<Chord> = Key::ALL
        .iter()
        .flat_map(|&k| [Chord::plain(k), Chord::shifted(k)])
        .filter(|c| c.is_reserved())
        .collect();
    assert_eq!(
        reserved,
        [Chord::plain(Key::Escape), Chord::plain(Key::Delete)]
    );
}

#[test]
fn new_keys_have_readable_names() {
    for (name, key) in [
        (",", Key::Comma),
        (".", Key::Period),
        ("/", Key::Slash),
        ("'", Key::Apostrophe),
        ("[", Key::LeftBracket),
        ("]", Key::RightBracket),
        ("\\", Key::Backslash),
        ("-", Key::Minus),
        ("=", Key::Equal),
        ("`", Key::Backquote),
        ("Insert", Key::Insert),
        ("Delete", Key::Delete),
        ("Home", Key::Home),
        ("End", Key::End),
        ("PageUp", Key::PageUp),
        ("PageDown", Key::PageDown),
        ("Kp0", Key::Kp0),
        ("Kp9", Key::Kp9),
        ("Kp+", Key::KpAdd),
        ("Kp-", Key::KpSubtract),
        ("Kp*", Key::KpMultiply),
        ("Kp/", Key::KpDivide),
        ("Kp.", Key::KpDecimal),
    ] {
        assert_eq!(Chord::parse(name), Ok(Chord::plain(key)), "{name}");
        let shifted = format!("Shift+{name}");
        assert_eq!(Chord::parse(&shifted), Ok(Chord::shifted(key)), "{shifted}");
    }
}

#[test]
fn every_key_round_trips_through_its_chord_name() {
    for &key in Key::ALL {
        for chord in [Chord::plain(key), Chord::shifted(key)] {
            assert_eq!(Chord::parse(&chord.to_string()), Ok(chord));
        }
    }
}

#[test]
fn punctuation_chords_load_from_a_keymap_file() {
    // RON strings need `\\` for a backslash; everything else is literal.
    let extra = "        \"Info\": [\",\", \"\\\\\", \"Kp+\"],\n";
    let k = KeymapDef::from_source("k.ron", &source_with("Info", extra)).unwrap_or_default();
    assert_eq!(
        k.chords(LEFT_HANDED, Action::Info),
        [
            Chord::plain(Key::Comma),
            Chord::plain(Key::Backslash),
            Chord::plain(Key::KpAdd)
        ]
    );
}
