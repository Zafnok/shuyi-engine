//! The input mechanism with a game of its own (ticket 1106): a three-action
//! [`ActionSet`] that knows nothing of the sample game's actions or layouts,
//! through the keymap file, [`Keymap`], [`InputState`], [`PlayerKeys`]
//! and [`HelpKeys`].

use core::time::Duration;

use shuyi_assets::keymap::KeymapDef;
use shuyi_assets::keymap::{ActionSet, Button, Chord, Key, Layout, UiAction};

use super::bindings::{LayoutBindings, PlayerKeys};
use super::help_keys::{HelpKeys, NOT_MAPPED, all_key_names, cursor_keys_name, key_name};
use super::keymap::Keymap;
use super::pad::PadKind;
use super::state::{Device, InputState};

/// A toy game's actions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum Toy {
    /// Required; the menus take it for Confirm.
    Jump,
    /// Repeats while held; the menus take it for Down.
    Duck,
    /// Optional; plain `Escape` also pauses, and the menus take it for
    /// Cancel.
    Pause,
}

impl ActionSet for Toy {
    type Group = ();

    const ALL: &'static [Self] = &[Self::Jump, Self::Duck, Self::Pause];

    fn name(self) -> &'static str {
        match self {
            Self::Jump => "Jump",
            Self::Duck => "Duck",
            Self::Pause => "Pause",
        }
    }

    fn is_repeatable(self) -> bool {
        self == Self::Duck
    }

    fn is_required(self) -> bool {
        self == Self::Jump
    }

    fn is_rebindable(self) -> bool {
        true
    }

    fn group(self) {}

    fn ui(self) -> Option<UiAction> {
        Some(match self {
            Self::Jump => UiAction::Confirm,
            Self::Duck => UiAction::Down,
            Self::Pause => UiAction::Cancel,
        })
    }

    fn fixed_keys() -> &'static [(Chord, Self)] {
        const FIXED: [(Chord, Toy); 1] = [(Chord::plain(Key::Escape), Toy::Pause)];
        &FIXED
    }
}

const MAIN: Layout = Layout::named("Main");
const ALT: Layout = Layout::named("Alt");

/// The toy game's keymap file.
const TOY_KEYMAP: &str = r#"(
    layouts: [
        (id: "Main", name: "toy.main", keys: {
            "Jump": ["Space"],
            "Duck": ["s", "Down"],
            "Pause": ["p"],
        }),
        (id: "Alt", name: "toy.alt", keys: {
            "Jump": ["k"],
            "Duck": ["j"],
            "Pause": [],
        }),
    ],
    layout_picker: {"Jump": ["Enter"]},
    pad: {"Jump": ["South"], "Duck": ["DpadDown"], "Pause": ["Start"]},
    repeat: (delay_ms: 100, interval_ms: 50),
    stick: (press_percent: 50, release_percent: 35),
)"#;

fn def() -> KeymapDef<Toy> {
    KeymapDef::from_source("toy.ron", TOY_KEYMAP).unwrap_or_else(|e| panic!("{e:?}"))
}

fn chord(s: &str) -> Chord {
    Chord::parse(s).unwrap_or_else(|e| panic!("{e}"))
}

fn ms(ms: u64) -> f32 {
    Duration::from_millis(ms).as_secs_f32()
}

#[test]
fn a_keymap_file_names_the_games_own_actions_and_layouts() {
    let def = def();
    assert_eq!(def.layout_ids().collect::<Vec<_>>(), [MAIN, ALT]);
    assert_eq!(def.layout_text(ALT), Some("toy.alt"));
    assert_eq!(def.chords(MAIN, Toy::Duck), [chord("s"), chord("Down")]);
    assert_eq!(def.buttons(Toy::Pause), [Button::Start]);
    // This game's actions are unknown to it.
    let errors = KeymapDef::<Toy>::from_source("toy.ron", &TOY_KEYMAP.replace("Pause", "Menu"))
        .err()
        .unwrap_or_default();
    assert!(
        errors[0]
            .message
            .contains("unknown action \"Menu\"; known actions: Jump, Duck, Pause"),
        "{errors:?}"
    );
}

#[test]
fn the_keymap_looks_up_the_toy_actions_and_its_fixed_key() {
    let km = Keymap::for_layout(&def(), MAIN);
    assert_eq!(km.action(chord("Space")), Some(Toy::Jump));
    assert_eq!(km.action(chord("Down")), Some(Toy::Duck));
    assert_eq!(km.action(chord("Escape")), Some(Toy::Pause));
    assert_eq!(km.chords_for(Toy::Pause), [chord("p")]);
    assert_eq!(Keymap::fixed_chords_for(Toy::Pause), [chord("Escape")]);
    assert_eq!(Keymap::fixed_chords_for(Toy::Jump), []);
    assert_eq!(km.pad_action(Button::South), Some(Toy::Jump));
    let picker = Keymap::layout_picker(&def());
    assert_eq!(picker.action(chord("Enter")), Some(Toy::Jump));
    assert_eq!(picker.action(chord("Space")), None);
}

#[test]
fn a_held_repeatable_action_repeats_and_the_others_do_not() {
    let mut input = InputState::new(Keymap::for_layout(&def(), MAIN));
    input.key_down(chord("Space"));
    input.key_down(chord("s"));
    assert_eq!(input.update(0.0), [Toy::Jump, Toy::Duck]);
    assert!(input.is_held(Toy::Jump) && input.is_held(Toy::Duck));
    assert_eq!(input.update(ms(99)), []);
    // The delay, then every interval.
    assert_eq!(input.update(ms(1)), [Toy::Duck]);
    assert_eq!(input.update(ms(100)), [Toy::Duck, Toy::Duck]);
    input.key_up(Key::S);
    assert_eq!(input.update(ms(200)), []);
    input.pad_down(Button::DpadDown, PadKind::Xbox);
    assert_eq!(input.update(0.0), [Toy::Duck]);
    assert_eq!(input.device(), Device::Pad(PadKind::Xbox));
}

#[test]
fn the_player_rebinds_the_toy_actions_and_the_file_round_trips() {
    let def = def();
    let mut main = LayoutBindings::defaults(&def, MAIN);
    // `s` moves from Duck to Jump: Duck keeps Down.
    assert_eq!(
        main.bind(Toy::Jump, 1, chord("s")),
        Ok(Some((Toy::Duck, 0)))
    );
    assert_eq!(main.slots(Toy::Duck), [None, Some(chord("Down")), None]);
    main.clear(Toy::Jump, 0);
    main.clear(Toy::Jump, 1);
    assert_eq!(main.unmapped_required(), [Toy::Jump]);
    assert!(main.bind(Toy::Pause, 0, chord("Escape")).is_err());
    main.bind(Toy::Jump, 0, chord("x")).ok();
    let mut keys = PlayerKeys::default();
    keys.set(&def, MAIN, main.clone());
    assert!(keys.is_custom(MAIN) && !keys.is_custom(ALT));
    let km = keys.keymap(&def, MAIN);
    assert_eq!(km.action(chord("x")), Some(Toy::Jump));
    assert_eq!(km.action(chord("Escape")), Some(Toy::Pause));

    let saved = keys.to_ron();
    assert!(
        saved.contains("\"Main\"") && saved.contains("\"Jump\""),
        "{saved}"
    );
    let (loaded, warnings) = PlayerKeys::from_ron(&saved, &def);
    assert_eq!(warnings, Vec::<String>::new());
    assert_eq!(loaded, keys);
    assert_eq!(loaded.to_ron(), saved);
    // A layout this keymap doesn't have is dropped.
    let other = saved.replace("\"Main\"", "\"RightHanded\"");
    let (loaded, warnings) = PlayerKeys::from_ron(&other, &def);
    assert_eq!(warnings, ["unknown layout \"RightHanded\" dropped"]);
    assert_eq!(loaded, PlayerKeys::default());
}

#[test]
fn help_text_names_the_toy_keys_and_buttons() {
    let km = Keymap::for_layout(&def(), MAIN);
    let keys = HelpKeys::keyboard(&km);
    assert_eq!(key_name(keys, Toy::Jump), "Space");
    assert_eq!(all_key_names(keys, Toy::Duck), "s/Down");
    assert_eq!(all_key_names(keys, Toy::Pause), "p/Escape");
    // No action moves up, left or right: there is no cursor to name.
    assert_eq!(cursor_keys_name(keys), NOT_MAPPED);
    let alt = Keymap::for_layout(&def(), ALT);
    assert_eq!(key_name(keys.of(&alt), Toy::Pause), NOT_MAPPED);
    // The fixed key alone isn't a key of its own.
    assert_eq!(all_key_names(keys.of(&alt), Toy::Pause), NOT_MAPPED);
    let pad = HelpKeys::new(&km, Device::Pad(PadKind::Xbox));
    assert_eq!(key_name(pad, Toy::Jump), "A");
    assert_eq!(key_name(pad, Toy::Duck), "↓");
}
