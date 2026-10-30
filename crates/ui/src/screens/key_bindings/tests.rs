//! The Key bindings screen on the toy game, whose keymap file is this
//! repository's game's less the Playdate's actions.

use super::*;
use crate::frame::console::ConsoleSpec;
use crate::storage::Storage;
use crate::toy::{Action, LEFT_HANDED, RIGHT_HANDED, Toy, ToyCtx};
use crate::toy::{KEY_ROWS, ctx};

type Ctx = ToyCtx;
type KeyBindingsScreen = super::KeyBindingsScreen<Toy>;
type KeyBindingsView = super::KeyBindingsView<Action>;
type RowView = super::RowView<Action>;
type LayoutBindings = crate::input::bindings::LayoutBindings<Action>;
type PadBindings = crate::input::bindings::PadBindings<Action>;
type FrameInput = crate::screen::FrameInput<Action>;
type AudioRequest = crate::audio::AudioRequest<crate::toy::Voice>;

/// The console the skin is laid out for.
const PC: ConsoleSpec = ConsoleSpec::text(100, 32);

use Action::{Cancel, Confirm, CursorDown, CursorLeft, CursorRight, CursorUp, Info};

fn chord(s: &str) -> Chord {
    Chord::parse(s).unwrap()
}

fn act(actions: &[Action]) -> FrameInput {
    FrameInput::new(actions.to_vec(), 0.0, vec![])
}

/// A frame where `c` was pressed (and did nothing as an action).
fn press(c: &str) -> FrameInput {
    act(&[]).with_typing(vec![chord(c)], vec![])
}

/// The screen with the Debug key reserved whatever the build profile.
fn screen(c: &Ctx) -> KeyBindingsScreen {
    let reserved = LayoutBindings::from_def(c.game.keymap_def(), RIGHT_HANDED, true);
    KeyBindingsScreen::new(c, &KEY_ROWS).with_bindings(reserved)
}

/// Moves the focus to `action`'s slot `slot`.
fn focus_on(s: &mut KeyBindingsScreen, c: &mut Ctx, action: Action, slot: usize) {
    for _ in 0..=KEY_ROWS.len() {
        if s.focus().map(|(a, _)| a) == Some(action) {
            break;
        }
        s.update(c, &act(&[CursorDown]));
    }
    s.update(c, &act(&[CursorLeft, CursorLeft]));
    for _ in 0..slot {
        s.update(c, &act(&[CursorRight]));
    }
    assert_eq!(s.focus(), Some((action, slot)));
}

/// Moves the focus down to Restore defaults. Bounded, so a screen that
/// never gets there fails the test instead of looping for ever.
fn focus_on_restore(s: &mut KeyBindingsScreen, c: &mut Ctx) {
    for _ in 0..=KEY_ROWS.len() {
        if s.focus().is_none() {
            return;
        }
        s.update(c, &act(&[CursorDown]));
    }
    panic!("Restore defaults was never reached");
}

/// Focuses `action`'s slot `slot`, confirms, and presses `key`.
fn bind(s: &mut KeyBindingsScreen, c: &mut Ctx, action: Action, slot: usize, key: &str) {
    focus_on(s, c, action, slot);
    s.update(c, &act(&[Confirm]));
    assert!(s.is_capturing());
    s.update(c, &press(key));
}

fn sounds(c: &mut Ctx) -> Vec<String> {
    c.audio
        .take()
        .into_iter()
        .filter_map(|r| match r {
            AudioRequest::PlaySound { cue, .. } => Some(cue),
            _ => None,
        })
        .collect()
}

fn cue(sound: MenuSound) -> String {
    let mut c = ctx();
    c.audio.menu(sound);
    sounds(&mut c).remove(0)
}

/// `action`'s row of `view`.
fn row_of(view: &KeyBindingsView, action: Action) -> &RowView {
    view.row(action)
        .unwrap_or_else(|| panic!("no row for {action:?}"))
}

/// What `action`'s slots show, as text: the key or button, `-` for an empty
/// slot, `…` for the one waiting.
fn shown(view: &KeyBindingsView, action: Action) -> Vec<String> {
    let slot = |s: &SlotView| match s {
        SlotView::Empty => "-".to_owned(),
        SlotView::Bound(name) => name.clone(),
        SlotView::Capturing(prompt) => format!("…{prompt}"),
    };
    row_of(view, action).slots.iter().map(slot).collect()
}

#[test]
fn rows_list_every_rebindable_action_once_required_first() {
    let mut listed: Vec<Action> = KEY_ROWS.iter().map(|r| r.0).collect();
    let required = listed.iter().take_while(|a| a.is_required()).count();
    assert_eq!(required, 7);
    assert!(listed[required..].iter().all(|a| !a.is_required()));
    listed.sort();
    let mut rebindable: Vec<Action> = Action::ALL
        .into_iter()
        .filter(|a| a.is_rebindable())
        .collect();
    rebindable.sort();
    assert_eq!(listed, rebindable);
    assert_eq!(label(&ctx(), &KEY_ROWS, Action::Info), "Unit info");
    assert_eq!(label(&ctx(), &KEY_ROWS, Action::Debug), "Debug");
    assert_eq!(
        blocked_message(&ctx(), &KEY_ROWS, Side::Keyboard, Cancel),
        "Give Cancel a key first"
    );
    assert_eq!(
        blocked_message(&ctx(), &KEY_ROWS, Side::Controller, Cancel),
        "Give Cancel a button first"
    );
}

#[test]
fn starts_on_the_first_slot_of_the_layout_in_use() {
    let c = ctx().with_layout(LEFT_HANDED);
    let s = KeyBindingsScreen::new(&c, &KEY_ROWS);
    assert_eq!(s.name(), "key_bindings");
    assert_eq!(s.focus(), Some((CursorUp, 0)));
    assert_eq!(s.bindings(), &c.layout_bindings(LEFT_HANDED));
    assert!(!s.is_capturing());
    assert_eq!(s.message(), None);
    assert_eq!(s.side(), Side::Keyboard);
    let view = s.view(&c);
    assert_eq!(view.title, "Key bindings");
    assert_eq!(
        view.switch.as_ref().unwrap().keyboard,
        "Keyboard · Left-handed"
    );
    assert_eq!(view.switch.as_ref().unwrap().controller, "Controller");
    assert_eq!(view.switch.as_ref().unwrap().shown, Side::Keyboard);
    assert!(!view.switch.as_ref().unwrap().focused);
    assert_eq!(view.columns, ["Key 1", "Key 2", "Key 3"]);
    let headings: Vec<_> = view.groups.iter().map(|g| g.heading.as_str()).collect();
    assert_eq!(
        headings,
        ["Must have a key", "Optional", "In the dialogue log"]
    );
    assert_eq!(
        view.groups.iter().map(|g| g.rows.len()).collect::<Vec<_>>(),
        [7, 10, 1]
    );
    assert_eq!(view.groups[2].rows[0].label, "Open the log");
    assert_eq!(view.restore.label, "Restore defaults");
    assert!(!view.restore.focused);
    assert_eq!(
        (&view.message, &view.question, &view.choices),
        (&None, &None, &None)
    );
    // The cursor is on the first slot of the first action, and only there.
    let focused: Vec<_> = view.rows().map(|r| (r.action, r.focus)).collect();
    assert_eq!(focused[0], (CursorUp, Some(0)));
    assert!(focused[1..].iter().all(|&(_, f)| f.is_none()));
    assert_eq!(view.focused().map(|r| r.action), Some(CursorUp));
    // Before a layout is chosen (debug menu over the picker): right-handed.
    let none = crate::toy::first_launch();
    let s = KeyBindingsScreen::new(&none, &KEY_ROWS);
    assert_eq!(s.bindings(), &none.layout_bindings(RIGHT_HANDED));
}

#[test]
fn up_and_down_wrap_through_the_rows_and_restore_defaults() {
    let mut c = ctx();
    let mut s = screen(&c);
    s.update(&mut c, &act(&[CursorDown]));
    assert_eq!(s.focus(), Some((CursorDown, 0)));
    assert_eq!(sounds(&mut c), [cue(MenuSound::Move)]);
    s.update(&mut c, &act(&[CursorUp, CursorUp]));
    assert_eq!(s.focus(), None);
    assert!(s.is_on_switch(), "up from the first row is the switch");
    s.update(&mut c, &act(&[CursorUp]));
    assert_eq!(s.focus(), None, "and up again wraps to Restore");
    assert!(!s.is_on_switch());
    s.update(&mut c, &act(&[CursorUp]));
    assert_eq!(s.focus(), Some((Action::DialogueLog, 0)));
    s.update(&mut c, &act(&[CursorUp]));
    assert_eq!(s.focus(), Some((Action::Menu, 0)));
    s.update(&mut c, &act(&[CursorDown]));
    s.update(&mut c, &act(&[CursorDown, CursorDown]));
    assert!(s.is_on_switch());
    s.update(&mut c, &act(&[CursorDown]));
    assert_eq!(s.focus(), Some((CursorUp, 0)));
}

#[test]
fn left_and_right_move_between_the_slots_and_stop_at_the_ends() {
    let mut c = ctx();
    let mut s = screen(&c);
    s.update(&mut c, &act(&[CursorLeft]));
    assert_eq!(s.focus(), Some((CursorUp, 0)));
    assert!(sounds(&mut c).is_empty(), "nowhere to go: no sound");
    s.update(&mut c, &act(&[CursorRight]));
    assert_eq!(s.focus(), Some((CursorUp, 1)));
    assert_eq!(sounds(&mut c), [cue(MenuSound::Move)]);
    s.update(&mut c, &act(&[CursorRight, CursorRight, CursorRight]));
    assert_eq!(s.focus(), Some((CursorUp, SLOTS - 1)));
    assert_eq!(sounds(&mut c), [cue(MenuSound::Move)]);
    s.update(&mut c, &act(&[CursorLeft]));
    assert_eq!(s.focus(), Some((CursorUp, 1)));
    // Restore defaults has no slots: Left/Right do nothing there, and the
    // slot is kept for the way back.
    s.update(&mut c, &act(&[CursorUp, CursorUp]));
    sounds(&mut c);
    s.update(&mut c, &act(&[CursorRight, CursorLeft]));
    let found = sounds(&mut c);
    assert!(found.is_empty(), "{found:?}");
    s.update(&mut c, &act(&[CursorDown, CursorDown]));
    assert_eq!(s.focus(), Some((CursorUp, 1)));
}

#[test]
fn confirm_captures_the_next_key_into_the_slot() {
    let mut c = ctx();
    let mut s = screen(&c);
    focus_on(&mut s, &mut c, Confirm, 1);
    sounds(&mut c);
    // The key that confirms is not the key captured, even in one frame.
    let confirm = act(&[Confirm, CursorDown]).with_typing(vec![chord("f")], vec![]);
    s.update(&mut c, &confirm);
    assert!(s.is_capturing());
    assert_eq!(s.focus(), Some((Confirm, 1)), "later actions are dropped");
    assert_eq!(sounds(&mut c), [cue(MenuSound::Select)]);
    // A frame with no press keeps waiting.
    s.update(&mut c, &act(&[]));
    assert!(s.is_capturing());
    s.update(&mut c, &press("g"));
    assert!(!s.is_capturing());
    assert_eq!(
        s.bindings().slots(Confirm),
        [Some(chord("f")), Some(chord("g")), None]
    );
    assert_eq!(s.moved, None);
    assert_eq!(sounds(&mut c), [cue(MenuSound::Select)]);
    // Shifted chords are their own keys.
    bind(&mut s, &mut c, Confirm, 2, "Shift+g");
    assert_eq!(s.bindings().slots(Confirm)[2], Some(chord("Shift+g")));
}

#[test]
fn actions_and_repeats_are_ignored_while_capturing() {
    let mut c = ctx();
    let mut s = screen(&c);
    s.update(&mut c, &act(&[Confirm]));
    let t = s.update(&mut c, &act(&[CursorDown, Cancel, Confirm]));
    assert_eq!(format!("{t:?}"), "None");
    assert!(s.is_capturing());
    assert_eq!(s.focus(), Some((CursorUp, 0)));
    // The pressed key's own action that frame is ignored too: `d` is bound,
    // not treated as Cancel.
    let d = act(&[Cancel]).with_typing(vec![chord("d")], vec![]);
    let t = s.update(&mut c, &d);
    assert_eq!(format!("{t:?}"), "None");
    assert_eq!(s.bindings().slots(CursorUp)[0], Some(chord("d")));
}

#[test]
fn a_cursor_key_captured_and_still_held_doesnt_move_the_focus() {
    let mut c = ctx();
    let mut s = screen(&c);
    focus_on(&mut s, &mut c, Info, 1);
    s.update(&mut c, &act(&[Confirm]));
    // `Down` is pressed for the slot and kept down: it repeats as Cursor
    // down under the keys the screen was opened with.
    let held = |actions: &[Action]| FrameInput::new(actions.to_vec(), 0.0, vec![CursorDown]);
    s.update(
        &mut c,
        &held(&[CursorDown]).with_typing(vec![chord("Down")], vec![]),
    );
    assert_eq!(s.bindings().slots(Info)[1], Some(chord("Down")));
    sounds(&mut c);
    s.update(&mut c, &held(&[CursorDown, CursorDown, CursorRight]));
    assert_eq!(s.focus(), Some((Info, 1)));
    let found = sounds(&mut c);
    assert!(found.is_empty(), "{found:?}");
    // Other actions still work meanwhile.
    s.update(&mut c, &held(&[Confirm]));
    assert!(s.is_capturing());
    s.update(
        &mut c,
        &held(&[]).with_typing(vec![chord("Escape")], vec![]),
    );
    s.update(&mut c, &held(&[CursorDown]));
    assert_eq!(s.focus(), Some((Info, 1)), "backing out waits for it too");
    // Once every cursor key is up, the next press moves again.
    s.update(&mut c, &act(&[]));
    s.update(&mut c, &held(&[CursorDown]));
    assert_eq!(s.focus(), Some((Action::DangerZone, 1)));
    // A key that isn't held never blocks: released in the capture frame.
    bind(&mut s, &mut c, Info, 2, "g");
    s.update(&mut c, &act(&[CursorUp]));
    assert_eq!(s.focus(), Some((Action::NextUnit, 2)));
    // The ignored clear-slot key doesn't start the wait.
    s.update(&mut c, &act(&[Confirm]));
    s.update(&mut c, &press("Delete"));
    assert!(!s.await_release);
}

#[test]
fn the_abort_key_leaves_capture_with_nothing_changed() {
    let mut c = ctx();
    let mut s = screen(&c);
    let before = s.bindings().clone();
    s.update(&mut c, &act(&[Confirm]));
    sounds(&mut c);
    // Escape is also Cancel in every keymap: it must not close the screen.
    let escape = act(&[Cancel]).with_typing(vec![chord("Escape")], vec![]);
    let t = s.update(&mut c, &escape);
    assert_eq!(format!("{t:?}"), "None");
    assert!(!s.is_capturing());
    assert_eq!(s.bindings(), &before);
    assert_eq!(sounds(&mut c), [cue(MenuSound::Cancel)]);
    // Keys pressed after it in the same frame aren't captured.
    s.update(&mut c, &act(&[Confirm]));
    let both = act(&[]).with_typing(vec![chord("Escape"), chord("g")], vec![]);
    s.update(&mut c, &both);
    assert_eq!(s.bindings(), &before);
    // Shift+Escape is an ordinary key.
    s.update(&mut c, &act(&[Confirm]));
    s.update(&mut c, &press("Shift+Escape"));
    assert_eq!(s.bindings().slots(CursorUp)[0], Some(chord("Shift+Escape")));
}

#[test]
fn the_clear_key_is_ignored_while_capturing() {
    let mut c = ctx();
    let mut s = screen(&c);
    let before = s.bindings().clone();
    s.update(&mut c, &act(&[Confirm]));
    sounds(&mut c);
    s.update(&mut c, &press("Delete"));
    assert!(s.is_capturing());
    assert_eq!(s.bindings(), &before);
    assert_eq!(s.message(), None);
    let found = sounds(&mut c);
    assert!(found.is_empty(), "{found:?}");
    // A key pressed after it in the same frame is still captured.
    let both = act(&[]).with_typing(vec![chord("Delete"), chord("g"), chord("h")], vec![]);
    s.update(&mut c, &both);
    assert!(!s.is_capturing());
    assert_eq!(s.bindings().slots(CursorUp)[0], Some(chord("g")));
    assert_eq!(
        s.bindings()
            .find(shuyi_assets::keymap::sample::KeyGroup::General, chord("h")),
        None
    );
}

#[test]
fn a_reserved_key_is_refused_and_capture_goes_on() {
    let mut c = ctx();
    let mut s = screen(&c);
    let before = s.bindings().clone();
    s.update(&mut c, &act(&[Confirm]));
    sounds(&mut c);
    // The Debug key, in a build with debug tools.
    s.update(&mut c, &press("F2"));
    assert!(s.is_capturing());
    assert_eq!(s.message(), Some("That key can't be used"));
    assert_eq!(s.bindings(), &before);
    assert_eq!(sounds(&mut c), [cue(MenuSound::Denied)]);
    assert_eq!(
        s.view(&c).message.as_deref(),
        Some("That key can't be used")
    );
    // The next key binds and clears the message.
    s.update(&mut c, &press("g"));
    assert!(!s.is_capturing());
    assert_eq!(s.message(), None);
    assert_eq!(s.bindings().slots(CursorUp)[0], Some(chord("g")));
    // Backing out clears it too.
    s.update(&mut c, &act(&[Confirm]));
    s.update(&mut c, &press("F2"));
    s.update(&mut c, &press("Escape"));
    assert_eq!(s.message(), None);
}

#[test]
fn a_key_used_elsewhere_moves_and_its_old_row_is_highlighted_briefly() {
    let mut c = ctx();
    let mut s = screen(&c);
    bind(&mut s, &mut c, Confirm, 1, "e");
    assert_eq!(
        s.bindings().slots(Confirm),
        [Some(chord("f")), Some(chord("e")), None]
    );
    assert!(s.bindings().is_unmapped(Info));
    assert_eq!(s.moved, Some((Info, MOVED_FLASH_SECS)));
    let lost = |s: &KeyBindingsScreen, c: &Ctx, action| row_of(&s.view(c), action).lost_key;
    assert!(lost(&s, &c, Info));
    assert!(!lost(&s, &c, Confirm));
    let view = s.view(&c);
    assert!(row_of(&view, Info).unmapped);
    assert!(!row_of(&view, Info).blocks_leaving, "an optional action");
    assert!(!row_of(&view, Confirm).unmapped);
    // The highlight lasts MOVED_FLASH_SECS.
    s.update(
        &mut c,
        &FrameInput::new(vec![], MOVED_FLASH_SECS - 0.5, vec![]),
    );
    assert!(lost(&s, &c, Info));
    s.update(&mut c, &FrameInput::new(vec![], 0.5, vec![]));
    assert_eq!(s.moved, None);
    assert!(!lost(&s, &c, Info));
    // A NaN frame time passes no time (FrameInput makes it 0).
    bind(&mut s, &mut c, Confirm, 2, "w");
    let moved = s.moved;
    assert!(moved.is_some());
    s.update(&mut c, &FrameInput::new(vec![], f32::NAN, vec![]));
    assert_eq!(s.moved, moved);
    // Its own row, when focused, is the focused one (a skin shows the
    // cursor over the highlight).
    bind(&mut s, &mut c, Info, 0, "s");
    focus_on(&mut s, &mut c, Action::NextUnit, 0);
    let view = s.view(&c);
    assert!(row_of(&view, Action::NextUnit).focused());
    assert_eq!(view.focused().map(|r| r.action), Some(Action::NextUnit));
}

#[test]
fn rebinding_a_slot_to_its_own_key_changes_nothing() {
    let mut c = ctx();
    let mut s = screen(&c);
    let before = s.bindings().clone();
    bind(&mut s, &mut c, Confirm, 0, "f");
    assert!(!s.is_capturing());
    assert_eq!(s.bindings(), &before);
    assert_eq!(s.moved, None);
}

#[test]
fn the_clear_key_empties_the_focused_slot() {
    let mut c = ctx();
    let mut s = screen(&c);
    focus_on(&mut s, &mut c, Info, 0);
    sounds(&mut c);
    s.update(&mut c, &press("Shift+Delete"));
    assert!(!s.bindings().is_unmapped(Info));
    s.update(&mut c, &press("Delete"));
    assert!(s.bindings().is_unmapped(Info));
    assert_eq!(sounds(&mut c), [cue(MenuSound::Select)]);
    // An empty slot: nothing to do, no sound.
    s.update(&mut c, &press("Delete"));
    let found = sounds(&mut c);
    assert!(found.is_empty(), "{found:?}");
    // A required action's last key may be emptied too.
    focus_on(&mut s, &mut c, Confirm, 0);
    s.update(&mut c, &press("Delete"));
    assert!(s.bindings().is_unmapped(Confirm));
    // On Restore defaults there is no slot.
    let before = s.bindings().clone();
    s.update(
        &mut c,
        &act(&[CursorUp, CursorUp, CursorUp, CursorUp, CursorUp]),
    );
    assert_eq!(s.focus(), None);
    s.update(&mut c, &press("Delete"));
    assert_eq!(s.bindings(), &before);
}

#[test]
fn clearing_a_slot_clears_the_message() {
    let mut c = ctx();
    let mut s = screen(&c);
    focus_on(&mut s, &mut c, Confirm, 0);
    s.update(&mut c, &press("Delete"));
    s.update(&mut c, &act(&[Cancel]));
    assert!(s.message().is_some());
    focus_on(&mut s, &mut c, Info, 0);
    s.update(&mut c, &act(&[Cancel]));
    assert!(s.message().is_some());
    s.update(&mut c, &press("Delete"));
    assert_eq!(s.message(), None);
}

#[test]
fn leaving_saves_the_edit_and_pops() {
    let mut c = ctx();
    let mut s = KeyBindingsScreen::new(&c, &KEY_ROWS);
    bind(&mut s, &mut c, Confirm, 1, "g");
    sounds(&mut c);
    let before = c.keymap.clone();
    assert_eq!(c.keymap, before, "nothing applies until the screen closes");
    let t = s.update(&mut c, &act(&[Cancel, CursorDown]));
    assert_eq!(format!("{t:?}"), "Pop");
    assert_eq!(sounds(&mut c), [cue(MenuSound::Cancel)]);
    assert_eq!(c.keymap.action(chord("g")), Some(Confirm));
    assert_eq!(&c.layout_bindings(RIGHT_HANDED), s.bindings());
    assert!(c.player_keys().is_custom(RIGHT_HANDED));
    assert!(!c.player_keys().is_custom(LEFT_HANDED));
    assert_eq!(
        c.storage.read(crate::ctx::KEYBINDINGS_KEY).unwrap(),
        Some(c.player_keys().to_ron())
    );
}

#[test]
fn leaving_without_a_change_writes_nothing() {
    let mut c = ctx();
    let mut s = KeyBindingsScreen::new(&c, &KEY_ROWS);
    let t = s.update(&mut c, &act(&[Cancel]));
    assert_eq!(format!("{t:?}"), "Pop");
    assert_eq!(c.storage.read(crate::ctx::KEYBINDINGS_KEY), Ok(None));
}

#[test]
fn leaving_is_blocked_while_a_required_action_has_no_key() {
    let mut c = ctx();
    let mut s = KeyBindingsScreen::new(&c, &KEY_ROWS);
    // Cancel and Cursor down lose their keys; the message names the one
    // listed first on the screen.
    bind(&mut s, &mut c, Info, 1, "d");
    bind(&mut s, &mut c, Info, 2, "Down");
    sounds(&mut c);
    let t = s.update(&mut c, &act(&[Cancel]));
    assert_eq!(format!("{t:?}"), "None");
    assert_eq!(s.message(), Some("Give Cursor down a key first"));
    assert_eq!(sounds(&mut c), [cue(MenuSound::Denied)]);
    assert!(!c.player_keys().is_custom(RIGHT_HANDED));
    assert_eq!(
        s.view(&c).message.as_deref(),
        Some("Give Cursor down a key first")
    );
    // The two required actions with nothing in them say so.
    let blocking: Vec<_> = s
        .view(&c)
        .rows()
        .filter(|r| r.blocks_leaving)
        .map(|r| r.action)
        .collect();
    assert_eq!(blocking, [CursorDown, Cancel]);
    // Moving on clears the message; the next try names what is left.
    bind(&mut s, &mut c, CursorDown, 0, "j");
    assert_eq!(s.message(), None);
    s.update(&mut c, &act(&[Cancel]));
    assert_eq!(s.message(), Some("Give Cancel a key first"));
    s.update(&mut c, &act(&[CursorRight]));
    assert_eq!(s.message(), None);
    // Escape doesn't count as Cancel's key, but still works as Cancel:
    // with a key of its own again, leaving works.
    bind(&mut s, &mut c, Cancel, 0, "k");
    let t = s.update(&mut c, &act(&[Cancel]));
    assert_eq!(format!("{t:?}"), "Pop");
    assert_eq!(c.keymap.action(chord("k")), Some(Cancel));
    // An optional action with no key never blocks.
    let mut s = KeyBindingsScreen::new(&c, &KEY_ROWS);
    focus_on(&mut s, &mut c, Action::Rewind, 0);
    s.update(&mut c, &press("Delete"));
    let t = s.update(&mut c, &act(&[Cancel]));
    assert_eq!(format!("{t:?}"), "Pop");
    assert_eq!(c.keymap.primary(Action::Rewind), None);
}

#[test]
fn restore_defaults_resets_only_this_layout() {
    let mut c = ctx();
    let mut left = c.layout_bindings(LEFT_HANDED);
    left.bind(Info, 0, chord("g")).unwrap();
    c.set_layout_bindings(LEFT_HANDED, left.clone()).unwrap();
    let mut s = KeyBindingsScreen::new(&c, &KEY_ROWS);
    bind(&mut s, &mut c, Confirm, 1, "e");
    assert!(s.moved.is_some());
    s.update(&mut c, &act(&[Cancel]));
    assert!(c.player_keys().is_custom(RIGHT_HANDED));

    let mut s = KeyBindingsScreen::new(&c, &KEY_ROWS);
    focus_on_restore(&mut s, &mut c);
    s.moved = Some((Info, 1.0));
    sounds(&mut c);
    // It asks first; a second Confirm in the same frame isn't the answer.
    let custom = s.bindings().clone();
    s.update(&mut c, &act(&[Confirm, Confirm]));
    assert!(!s.is_capturing());
    assert!(s.is_asking_restore());
    assert_eq!(s.bindings(), &custom);
    assert_eq!(sounds(&mut c), [cue(MenuSound::Select)]);
    s.update(&mut c, &act(&[Confirm, Cancel]));
    assert!(!s.is_asking_restore());
    assert_eq!(s.moved, None);
    assert_eq!(sounds(&mut c), [cue(MenuSound::Select)]);
    let defaults = LayoutBindings::defaults(c.game.keymap_def(), RIGHT_HANDED);
    assert_eq!(s.bindings(), &defaults);
    let t = s.update(&mut c, &act(&[Cancel]));
    assert_eq!(format!("{t:?}"), "Pop");
    assert!(!c.player_keys().is_custom(RIGHT_HANDED));
    assert_eq!(c.layout_bindings(LEFT_HANDED), left);
}

#[test]
fn the_restore_question_can_be_backed_out_of() {
    let mut c = ctx();
    let mut s = KeyBindingsScreen::new(&c, &KEY_ROWS);
    bind(&mut s, &mut c, Confirm, 1, "e");
    let custom = s.bindings().clone();
    focus_on_restore(&mut s, &mut c);
    s.update(&mut c, &act(&[Confirm]));
    assert!(s.is_asking_restore());
    assert_eq!(
        s.restore_question(&c),
        "Restore the default keys for Right-handed?"
    );
    assert_eq!(s.help(&c), "f yes · d no");
    sounds(&mut c);
    // Nothing but Confirm and Cancel answers; the clear-slot key and the
    // cursor do nothing.
    let other = act(&[CursorUp, Info, CursorLeft]).with_typing(vec![chord("Delete")], vec![]);
    s.update(&mut c, &other);
    assert!(s.is_asking_restore());
    assert_eq!(s.focus(), None);
    let found = sounds(&mut c);
    assert!(found.is_empty(), "{found:?}");
    let question = s.view(&c).question.unwrap();
    assert_eq!(question.text, "Restore the default keys for Right-handed?");
    assert_eq!(question.keys.as_deref(), Some("f yes / d no"));
    // Cancel closes the question, not the screen, and changes nothing;
    // what follows it that frame is dropped.
    let t = s.update(&mut c, &act(&[Cancel, Confirm, Cancel]));
    assert_eq!(format!("{t:?}"), "None");
    assert!(!s.is_asking_restore());
    assert_eq!(s.bindings(), &custom);
    assert_eq!(sounds(&mut c), [cue(MenuSound::Cancel)]);
    assert_eq!(s.help(&c), "arrows move · f restore · d back");
    assert_eq!(s.view(&c).question, None);
    // The left-handed question names its layout and keys.
    let left = ctx().with_layout(LEFT_HANDED);
    let mut s = KeyBindingsScreen::new(&left, &KEY_ROWS);
    s.restoring = Some(s.restore_ask());
    assert_eq!(
        s.restore_question(&c),
        "Restore the default keys for Left-handed?"
    );
    assert_eq!(s.help(&c), "j yes · k no");
    let question = s.view(&left).question.unwrap();
    assert_eq!(question.text, "Restore the default keys for Left-handed?");
    assert_eq!(question.keys.as_deref(), Some("j yes / k no"));
}

#[test]
fn confirm_clears_the_message() {
    let mut c = ctx();
    let mut s = KeyBindingsScreen::new(&c, &KEY_ROWS);
    focus_on(&mut s, &mut c, Confirm, 0);
    s.update(&mut c, &press("Delete"));
    s.update(&mut c, &act(&[Cancel]));
    assert!(s.message().is_some());
    s.update(&mut c, &act(&[Confirm]));
    assert_eq!(s.message(), None);
}

#[test]
fn other_actions_do_nothing() {
    let mut c = ctx();
    let mut s = screen(&c);
    let before = s.bindings().clone();
    let t = s.update(
        &mut c,
        &act(&[Info, Action::EndTurn, Action::Debug, Action::Menu]),
    );
    assert_eq!(format!("{t:?}"), "None");
    assert_eq!(s.focus(), Some((CursorUp, 0)));
    assert_eq!(s.bindings(), &before);
    let found = sounds(&mut c);
    assert!(found.is_empty(), "{found:?}");
    // An unbound key pressed while not capturing does nothing either.
    s.update(&mut c, &press("g"));
    assert_eq!(s.bindings(), &before);
}

#[test]
fn help_names_the_keys_the_screen_was_opened_with() {
    let mut c = ctx();
    let mut s = screen(&c);
    assert_eq!(s.help(&c), "arrows move · f bind · Delete clear · d back");
    // Edits don't change the help: the old keys still steer the screen.
    bind(&mut s, &mut c, Info, 0, "f");
    bind(&mut s, &mut c, Info, 1, "d");
    assert_eq!(s.help(&c), "arrows move · f bind · Delete clear · d back");
    s.update(&mut c, &act(&[Confirm]));
    assert_eq!(s.help(&c), "Press the key to put here · Escape back");
    s.update(&mut c, &press("Escape"));
    s.update(&mut c, &act(&[CursorUp, CursorUp, CursorUp, CursorUp]));
    focus_on_restore(&mut s, &mut c);
    assert_eq!(s.help(&c), "arrows move · f restore · d back");
    let left = ctx().with_layout(LEFT_HANDED);
    assert_eq!(
        KeyBindingsScreen::new(&left, &KEY_ROWS).help(&left),
        "wasd move · j bind · Delete clear · k back"
    );
}

#[test]
fn the_view_lists_slots_fixed_keys_and_not_mapped_notes() {
    let mut c = ctx();
    let mut s = screen(&c);
    let view = s.view(&c);
    assert_eq!(shown(&view, CursorUp), ["Up", "-", "-"]);
    assert_eq!(shown(&view, Confirm), ["f", "-", "-"]);
    assert_eq!(row_of(&view, Cancel).fixed, ["Escape"]);
    assert_eq!(shown(&view, Action::EndTurn)[0], "Space");
    assert_eq!(shown(&view, Action::ToggleAutoEnd)[0], "Shift+Space");
    let found = &row_of(&view, Action::EndTurn).fixed;
    assert!(found.is_empty(), "{found:?}");
    let found = &row_of(&view, Confirm).fixed;
    assert!(found.is_empty(), "{found:?}");
    assert_eq!(view.not_mapped, crate::input::help_keys::NOT_MAPPED);
    // Optional and unmapped: a note that doesn't block leaving. Required and
    // unmapped: one that does.
    let flags = |view: &KeyBindingsView, a| {
        let r = row_of(view, a);
        (r.unmapped, r.blocks_leaving)
    };
    assert_eq!(flags(&view, Action::Select), (true, false));
    assert_eq!(flags(&view, Confirm), (false, false));
    focus_on(&mut s, &mut c, Confirm, 0);
    s.update(&mut c, &press("Delete"));
    let view = s.view(&c);
    assert_eq!(flags(&view, Confirm), (true, true));
    assert_eq!(shown(&view, Confirm), ["-", "-", "-"]);
    // The focused slot is the one the cursor is on; while capturing it
    // holds the prompt.
    assert_eq!(row_of(&view, Confirm).focus, Some(0));
    assert_eq!(view.rows().filter(|r| r.focused()).count(), 1);
    s.update(&mut c, &act(&[CursorRight, Confirm]));
    let view = s.view(&c);
    assert_eq!(row_of(&view, Confirm).focus, Some(1));
    assert_eq!(shown(&view, Confirm)[1], "…Press a key…");
    assert_eq!(shown(&view, Confirm)[0], "-");
}

#[test]
fn the_groups_are_the_required_actions_then_the_optional_ones() {
    let c = ctx();
    let view = screen(&c).view(&c);
    assert!(view.groups[0].required && !view.groups[1].required);
    assert!(view.groups[0].rows.iter().all(|r| r.action.is_required()));
    assert!(view.groups[1].rows.iter().all(|r| !r.action.is_required()));
    let listed: Vec<_> = view.rows().map(|r| (r.action, r.label.as_str())).collect();
    let want: Vec<_> = KEY_ROWS
        .iter()
        .map(|&(a, key, _)| (a, c.text(key)))
        .collect();
    assert_eq!(listed, want);
}

// --- The controller side (ticket 0816) --------------------------------------

const XBOX: Device = Device::Pad(PadKind::Xbox);

/// A context whose player pressed a controller button last.
fn pad_ctx() -> Ctx {
    let mut c = ctx();
    c.device = XBOX;
    c
}

/// The button watched during a capture, and for how many seconds it has
/// been held.
fn held(s: &KeyBindingsScreen) -> Option<(Button, f32)> {
    s.held.map(|h| (h.first, h.secs))
}

/// A frame of `dt` seconds where the buttons `down` went down and `up` went
/// up (and did nothing as actions).
fn buttons(down: &[Button], up: &[Button], dt: f32) -> FrameInput {
    FrameInput::new(vec![], dt, vec![]).with_buttons(down.to_vec(), up.to_vec())
}

/// Presses and lets go of `button` in two frames.
fn tap(s: &mut KeyBindingsScreen, c: &mut Ctx, button: Button) {
    s.update(c, &buttons(&[button], &[], 0.0));
    s.update(c, &buttons(&[], &[button], 0.0));
}

/// Focuses `action`'s slot `slot` and starts capturing through the choices.
fn change(s: &mut KeyBindingsScreen, c: &mut Ctx, action: Action, slot: usize) {
    focus_on(s, c, action, slot);
    s.update(c, &act(&[Confirm]));
    assert_eq!(s.choice(), Some(0));
    s.update(c, &act(&[Confirm]));
    assert!(s.is_capturing());
}

fn pad_names(s: &KeyBindingsScreen, action: Action) -> [String; SLOTS] {
    s.pad_bindings()
        .slots(action)
        .map(|b| b.map_or_else(|| "-".to_owned(), |b| b.to_string()))
}

#[test]
fn opens_on_the_side_pressed_last() {
    let c = ctx();
    assert_eq!(KeyBindingsScreen::new(&c, &KEY_ROWS).side(), Side::Keyboard);
    let c = pad_ctx();
    let s = KeyBindingsScreen::new(&c, &KEY_ROWS);
    assert_eq!(s.side(), Side::Controller);
    assert_eq!(s.focus(), Some((CursorUp, 0)));
    assert_eq!(s.pad_bindings(), &c.pad_bindings());
    assert_eq!(s.choice(), None);
    assert!(!s.is_on_switch());
}

#[test]
fn the_switch_row_shows_the_keyboard_or_the_controller() {
    let mut c = ctx();
    let mut s = screen(&c);
    s.update(&mut c, &act(&[CursorUp]));
    assert!(s.is_on_switch());
    s.message = Some("old".to_owned());
    s.moved = Some((Info, 1.0));
    sounds(&mut c);
    // Left on the keyboard side: already there.
    s.update(&mut c, &act(&[CursorLeft]));
    assert_eq!(s.side(), Side::Keyboard);
    let found = sounds(&mut c);
    assert!(found.is_empty(), "{found:?}");
    assert_eq!(s.message(), Some("old"));
    s.update(&mut c, &act(&[CursorRight]));
    assert_eq!(s.side(), Side::Controller);
    assert_eq!(sounds(&mut c), [cue(MenuSound::Move)]);
    assert_eq!((s.message(), s.moved), (None, None));
    s.update(&mut c, &act(&[CursorRight]));
    assert_eq!(s.side(), Side::Controller);
    let found = sounds(&mut c);
    assert!(found.is_empty(), "{found:?}");
    s.update(&mut c, &act(&[CursorLeft]));
    assert_eq!(s.side(), Side::Keyboard);
    // Confirm flips it, either way, and opens nothing.
    s.update(&mut c, &act(&[Confirm]));
    assert_eq!(s.side(), Side::Controller);
    assert_eq!(sounds(&mut c), vec![cue(MenuSound::Move); 2]);
    s.update(&mut c, &act(&[Confirm]));
    assert_eq!(s.side(), Side::Keyboard);
    assert!(!s.is_capturing() && !s.is_asking_restore() && s.choice().is_none());
    // The slot is kept for the way back down.
    assert_eq!(s.slot, 0);
    s.update(&mut c, &act(&[CursorDown]));
    assert_eq!(s.focus(), Some((CursorUp, 0)));
    // On the controller too, where Confirm doesn't open the choices.
    let mut c = pad_ctx();
    let mut s = KeyBindingsScreen::new(&c, &KEY_ROWS);
    s.update(&mut c, &act(&[CursorUp, Confirm]));
    assert_eq!((s.side(), s.choice()), (Side::Keyboard, None));
    // The clear-slot key does nothing there.
    s.update(&mut c, &press("Delete"));
    assert_eq!(s.bindings(), &c.layout_bindings(RIGHT_HANDED));
}

#[test]
fn the_controller_side_shows_buttons_as_the_pad_names_them() {
    let mut c = pad_ctx();
    let mut s = KeyBindingsScreen::new(&c, &KEY_ROWS);
    let view = s.view(&c);
    assert_eq!(view.columns, ["Button 1", "Button 2", "Button 3"]);
    let headings: Vec<_> = view.groups.iter().map(|g| g.heading.as_str()).collect();
    assert_eq!(
        headings,
        ["Must have a button", "Optional", "In the dialogue log"]
    );
    assert_eq!(shown(&view, CursorUp), ["↑", "L-stick ↑", "-"]);
    assert_eq!(shown(&view, Confirm)[0], "A");
    assert!(row_of(&view, Cancel).fixed.is_empty(), "no fixed button");
    assert_eq!(shown(&view, Action::EndTurn)[0], "Start");
    assert!(row_of(&view, Action::Select).unmapped);
    assert!(!row_of(&view, Action::Rewind).unmapped);
    // The shown side is the controller.
    assert_eq!(view.switch.as_ref().unwrap().shown, Side::Controller);
    // A Sony pad's names; on the keyboard, an Xbox pad's.
    c.device = Device::Pad(PadKind::PlayStation);
    assert_eq!(shown(&s.view(&c), Action::EndTurn)[0], "Options");
    c.device = Device::Keyboard;
    assert_eq!(shown(&s.view(&c), Action::EndTurn)[0], "Start");
    // On the switch, it is the focus.
    s.update(&mut c, &act(&[CursorUp]));
    let view = s.view(&c);
    assert!(view.switch.as_ref().unwrap().focused);
    assert_eq!(view.focused(), None);
    s.update(&mut c, &act(&[CursorLeft]));
    let view = s.view(&c);
    assert_eq!(view.switch.as_ref().unwrap().shown, Side::Keyboard);
    assert!(view.switch.as_ref().unwrap().focused);
    assert_eq!(view.columns[0], "Key 1");
    assert_eq!(row_of(&view, Cancel).fixed, ["Escape"]);
}

#[test]
fn with_the_keyboard_confirm_captures_a_button_let_go() {
    let mut c = ctx();
    let mut s = screen(&c);
    s.update(&mut c, &act(&[CursorUp, CursorRight]));
    focus_on(&mut s, &mut c, Info, 1);
    sounds(&mut c);
    s.update(&mut c, &act(&[Confirm]));
    assert!(s.is_capturing());
    assert_eq!(s.choice(), None);
    let waiting = shown(&s.view(&c), Info);
    assert_eq!(waiting[1], "…Press a button…");
    sounds(&mut c);
    // Going down doesn't bind; nor does a key.
    s.update(&mut c, &buttons(&[Button::RightStickPress], &[], 0.0));
    s.update(&mut c, &press("g"));
    assert!(s.is_capturing());
    assert_eq!(pad_names(&s, Info), ["North", "-", "-"]);
    assert_eq!(s.bindings(), &screen(&c).bindings);
    let found = sounds(&mut c);
    assert!(found.is_empty(), "{found:?}");
    // Letting go does.
    s.update(&mut c, &buttons(&[], &[Button::RightStickPress], 0.0));
    assert!(!s.is_capturing());
    assert_eq!(pad_names(&s, Info), ["North", "RightStickPress", "-"]);
    assert_eq!((s.moved, s.held), (None, None));
    assert_eq!(sounds(&mut c), [cue(MenuSound::Select)]);
    // The keys are untouched.
    assert_eq!(s.bindings(), &screen(&c).bindings);
}

#[test]
fn a_button_taken_from_another_action_moves_and_that_row_lights_up() {
    let mut c = pad_ctx();
    let mut s = KeyBindingsScreen::new(&c, &KEY_ROWS);
    change(&mut s, &mut c, Info, 0);
    s.message = Some("old".to_owned());
    tap(&mut s, &mut c, Button::South);
    assert_eq!(pad_names(&s, Info), ["South", "-", "-"]);
    assert_eq!(pad_names(&s, Confirm), ["-", "-", "-"]);
    assert_eq!(s.moved, Some((Confirm, MOVED_FLASH_SECS)));
    assert_eq!(s.message(), None);
    assert!(row_of(&s.view(&c), Confirm).unmapped);
    assert!(row_of(&s.view(&c), Confirm).lost_key);
    // Its own button again: nothing moves.
    change(&mut s, &mut c, Info, 0);
    tap(&mut s, &mut c, Button::South);
    assert_eq!(s.moved, None);
    assert!(!s.is_capturing());
}

#[test]
fn a_button_pressed_and_let_go_in_one_frame_binds() {
    let mut c = pad_ctx();
    let mut s = KeyBindingsScreen::new(&c, &KEY_ROWS);
    change(&mut s, &mut c, Info, 2);
    s.update(&mut c, &buttons(&[Button::West], &[Button::West], 5.0));
    assert!(!s.is_capturing());
    assert_eq!(pad_names(&s, Info), ["North", "-", "West"]);
}

#[test]
fn holding_a_button_backs_out_with_nothing_changed() {
    let mut c = pad_ctx();
    let mut s = KeyBindingsScreen::new(&c, &KEY_ROWS);
    change(&mut s, &mut c, Info, 0);
    sounds(&mut c);
    // The frame it goes down in doesn't count, however long.
    s.update(&mut c, &buttons(&[Button::West], &[], 5.0));
    assert_eq!(held(&s), Some((Button::West, 0.0)));
    s.update(&mut c, &buttons(&[], &[], 0.5));
    s.update(&mut c, &buttons(&[], &[], 0.25));
    assert!(s.is_capturing(), "not held long enough yet");
    assert_eq!(held(&s), Some((Button::West, 0.75)));
    let found = sounds(&mut c);
    assert!(found.is_empty(), "{found:?}");
    s.update(&mut c, &buttons(&[], &[], 0.25));
    assert!(!s.is_capturing(), "held for exactly the time");
    assert_eq!(held(&s), None);
    assert!(!s.await_release, "no cursor button is held");
    assert_eq!(sounds(&mut c), [cue(MenuSound::Cancel)]);
    // Letting go afterwards does nothing.
    s.update(&mut c, &buttons(&[], &[Button::West], 0.0));
    assert_eq!(s.pad_bindings(), &c.pad_bindings());
    assert_eq!(s.focus(), Some((Info, 0)));
    let found = sounds(&mut c);
    assert!(found.is_empty(), "{found:?}");
    assert_eq!(HOLD_TO_CANCEL_SECS.to_bits(), 1.0_f32.to_bits());

    // Let go just before the time: it binds.
    change(&mut s, &mut c, Info, 0);
    s.update(&mut c, &buttons(&[Button::West], &[], 0.0));
    s.update(&mut c, &buttons(&[], &[], 0.75));
    s.update(&mut c, &buttons(&[], &[Button::West], 0.5));
    assert_eq!(pad_names(&s, Info), ["West", "-", "-"]);
}

#[test]
fn a_bad_frame_time_neither_backs_out_nor_breaks_the_hold() {
    let mut c = pad_ctx();
    let mut s = KeyBindingsScreen::new(&c, &KEY_ROWS);
    change(&mut s, &mut c, Info, 0);
    s.update(&mut c, &buttons(&[Button::West], &[], 0.0));
    s.update(&mut c, &buttons(&[], &[], f32::NAN));
    s.update(&mut c, &buttons(&[], &[], -3.0));
    assert_eq!(held(&s), Some((Button::West, 0.0)));
    s.update(&mut c, &buttons(&[], &[], 1.5));
    assert!(!s.is_capturing());
}

/// Ticket 1045: with no keyboard the screen is the controller side alone:
/// no switch (up from the first row is Restore defaults), and a keyboard
/// key missing never blocks leaving.
#[test]
fn with_no_keyboard_only_the_controller_side_shows() {
    let mut c = pad_ctx();
    c.platform.keyboard = Keyboard::Absent;
    c.device = Device::Keyboard;
    let layout = c.shown_layout().unwrap();
    let mut keys = c.layout_bindings(layout);
    for slot in 0..SLOTS {
        keys.clear(Cancel, slot);
    }
    let mut s = KeyBindingsScreen::new(&c, &KEY_ROWS).with_bindings(keys);
    assert!(s.is_controller_only());
    assert_eq!(s.side(), Side::Controller);
    assert_eq!(s.view(&c).switch, None);
    s.update(&mut c, &act(&[CursorUp]));
    assert!(!s.is_on_switch());
    assert_eq!(s.focus(), None, "Restore defaults");
    assert!(s.view(&c).restore.focused);
    s.update(&mut c, &act(&[CursorDown]));
    assert_eq!(s.focus(), Some((CursorUp, 0)));
    // Left and right on the first row move between slots, never sides.
    s.update(&mut c, &act(&[CursorLeft]));
    assert_eq!(s.side(), Side::Controller);
    let t = s.update(&mut c, &act(&[Cancel]));
    assert_eq!(format!("{t:?}"), "Pop");
    // With a keyboard the same screen has the switch and is blocked.
    let mut c = pad_ctx();
    let mut keys = c.layout_bindings(layout);
    for slot in 0..SLOTS {
        keys.clear(Cancel, slot);
    }
    let mut s = KeyBindingsScreen::new(&c, &KEY_ROWS).with_bindings(keys);
    assert!(!s.is_controller_only() && s.view(&c).switch.is_some());
    let t = s.update(&mut c, &act(&[Cancel]));
    assert_eq!(format!("{t:?}"), "None");
    assert_eq!(s.side(), Side::Keyboard);
}

/// Ticket 1045: a game's own painter paints the same view instead.
#[test]
fn a_game_may_paint_the_view_its_own_way() {
    use crate::frame::color::Rgb;
    use crate::frame::glyph_buffer::Cell;
    /// Writes the title at the top left.
    fn title(_: &Ctx, view: &KeyBindingsView, buf: &mut GlyphBuffer) {
        let black = Rgb::new(0, 0, 0);
        buf.fill_rect(buf.bounds(), Cell::new(' ', black, black));
        buf.print(0, 0, &view.title, black, black);
    }
    let c = ctx();
    let s = KeyBindingsScreen::new(&c, &KEY_ROWS).painted_by(title);
    let ink = Rgb::new(0, 0, 0);
    let stale = Cell::new('x', ink, ink);
    let mut buf = GlyphBuffer::with_spec(ConsoleSpec::text(20, 2), stale);
    s.draw(&c, &mut buf);
    let mut expected = GlyphBuffer::with_spec(ConsoleSpec::text(20, 2), stale);
    title(&c, &s.view(&c), &mut expected);
    assert_eq!(buf, expected);
    assert_eq!(buf.get(0, 0).map(|c| c.glyph), Some('K'));
}

/// Ticket 1045: two buttons down together make a chord of the two; a
/// third is ignored, and the chord goes in when one of the two is let go.
#[test]
fn two_buttons_down_together_make_a_chord() {
    let mut c = pad_ctx();
    let mut s = KeyBindingsScreen::new(&c, &KEY_ROWS);
    change(&mut s, &mut c, Info, 1);
    s.update(&mut c, &buttons(&[Button::West, Button::Start], &[], 0.0));
    s.update(&mut c, &buttons(&[Button::RightStickPress], &[], 0.25));
    s.update(&mut c, &buttons(&[], &[Button::RightStickPress], 0.25));
    assert!(s.is_capturing());
    // Held together, however long, they don't back out.
    s.update(&mut c, &buttons(&[], &[], 5.0));
    assert!(s.is_capturing());
    s.update(&mut c, &buttons(&[], &[Button::Start], 0.0));
    assert!(!s.is_capturing());
    assert_eq!(pad_names(&s, Info), ["North", "West+Start", "-"]);
}

/// Ticket 1045: a button of the capture still held when it ends (B of a
/// chord, or one held to back out) does nothing when it is let go, even
/// if its action comes then (the Playdate's B is Cancel on release).
#[test]
fn a_button_still_held_when_the_capture_ends_does_nothing_when_let_go() {
    let mut c = pad_ctx();
    let mut s = KeyBindingsScreen::new(&c, &KEY_ROWS);
    change(&mut s, &mut c, Info, 0);
    s.update(&mut c, &buttons(&[Button::East], &[], 0.0));
    s.update(&mut c, &buttons(&[Button::DpadRight], &[], 0.1));
    s.update(&mut c, &buttons(&[], &[Button::DpadRight], 0.1));
    assert!(!s.is_capturing());
    assert_eq!(pad_names(&s, Info)[0], "East+DpadRight");
    let let_go = |s: &mut KeyBindingsScreen, c: &mut Ctx| {
        let frame = FrameInput::new(vec![Cancel], 0.0, vec![]);
        let t = s.update(c, &frame.with_buttons(vec![], vec![Button::East]));
        format!("{t:?}")
    };
    assert_eq!(let_go(&mut s, &mut c), "None");
    assert_eq!(s.focus(), Some((Info, 0)));
    // Then Cancel is Cancel again.
    assert_eq!(format!("{:?}", s.update(&mut c, &act(&[Cancel]))), "Pop");
    // Held alone to back out: the same.
    let mut s = KeyBindingsScreen::new(&c, &KEY_ROWS);
    change(&mut s, &mut c, Info, 0);
    s.update(&mut c, &buttons(&[Button::East], &[], 0.0));
    s.update(&mut c, &buttons(&[], &[], 1.5));
    assert!(!s.is_capturing());
    assert_eq!(let_go(&mut s, &mut c), "None");
    assert_eq!(format!("{:?}", s.update(&mut c, &act(&[Cancel]))), "Pop");
}

/// Ticket 1045: a second button pressed while the first is held (B held,
/// then right) makes the chord; let go of the first, it goes in.
#[test]
fn a_button_pressed_while_another_is_held_makes_a_chord() {
    let mut c = pad_ctx();
    let mut s = KeyBindingsScreen::new(&c, &KEY_ROWS);
    change(&mut s, &mut c, Info, 0);
    s.update(&mut c, &buttons(&[Button::East], &[], 0.0));
    s.update(&mut c, &buttons(&[], &[], 0.5));
    s.update(&mut c, &buttons(&[Button::DpadRight], &[], 0.25));
    assert_eq!(held(&s), Some((Button::East, 0.75)));
    s.update(&mut c, &buttons(&[], &[Button::East], 0.0));
    assert!(!s.is_capturing());
    assert_eq!(pad_names(&s, Info)[0], "East+DpadRight");
    s.update(&mut c, &buttons(&[], &[Button::DpadRight], 0.0));
    // One let go before the next goes down: that one alone.
    change(&mut s, &mut c, Info, 2);
    s.update(&mut c, &buttons(&[Button::West], &[], 0.0));
    s.update(&mut c, &buttons(&[], &[Button::West], 0.1));
    assert!(!s.is_capturing());
    assert_eq!(pad_names(&s, Info)[2], "West");
}

#[test]
fn a_button_held_since_before_the_capture_is_ignored() {
    let mut c = pad_ctx();
    let mut s = KeyBindingsScreen::new(&c, &KEY_ROWS);
    // Confirm's button went down to pick Change, and is still down.
    change(&mut s, &mut c, Info, 1);
    s.update(&mut c, &buttons(&[], &[], 5.0));
    assert!(s.is_capturing(), "a hold from before doesn't back out");
    s.update(&mut c, &buttons(&[], &[Button::South], 0.0));
    assert!(s.is_capturing(), "letting go of it doesn't bind it");
    assert_eq!(s.pad_bindings(), &c.pad_bindings());
}

#[test]
fn on_a_controller_confirm_offers_change_or_clear() {
    let mut c = pad_ctx();
    let mut s = KeyBindingsScreen::new(&c, &KEY_ROWS);
    focus_on(&mut s, &mut c, Info, 0);
    s.message = Some("old".to_owned());
    sounds(&mut c);
    // A second Confirm in the same frame isn't the answer.
    s.update(&mut c, &act(&[Confirm, Confirm, CursorDown]));
    assert_eq!(s.choice(), Some(0));
    assert!(!s.is_capturing());
    assert_eq!(s.message(), None);
    assert_eq!(s.focus(), Some((Info, 0)));
    assert_eq!(sounds(&mut c), [cue(MenuSound::Select)]);
    // Up and down move between the two; left, right and the rest don't.
    s.update(&mut c, &act(&[CursorDown]));
    assert_eq!(s.choice(), Some(1));
    s.update(&mut c, &act(&[CursorDown]));
    assert_eq!(s.choice(), Some(0));
    s.update(&mut c, &act(&[CursorUp]));
    assert_eq!(s.choice(), Some(1));
    assert_eq!(sounds(&mut c), vec![cue(MenuSound::Move); 3]);
    s.update(
        &mut c,
        &act(&[CursorLeft, CursorRight, Info, Action::EndTurn]),
    );
    s.update(&mut c, &press("Delete"));
    assert_eq!(s.choice(), Some(1));
    assert_eq!(s.focus(), Some((Info, 0)));
    let found = sounds(&mut c);
    assert!(found.is_empty(), "{found:?}");
    // A button going down while they are open isn't captured.
    tap(&mut s, &mut c, Button::West);
    assert_eq!(s.pad_bindings(), &c.pad_bindings());
    // Cancel closes them and stays on the screen.
    let t = s.update(&mut c, &act(&[Cancel, Cancel]));
    assert_eq!(format!("{t:?}"), "None");
    assert_eq!(s.choice(), None);
    assert_eq!(sounds(&mut c), [cue(MenuSound::Cancel)]);
    // Clear empties the slot.
    s.update(&mut c, &act(&[Confirm]));
    s.update(&mut c, &act(&[CursorDown, Confirm, Confirm]));
    assert_eq!(s.choice(), None);
    assert!(!s.is_capturing());
    assert_eq!(pad_names(&s, Info), ["-", "-", "-"]);
    sounds(&mut c);
    // Clear on an empty slot closes them too, with one sound.
    s.update(&mut c, &act(&[Confirm]));
    s.update(&mut c, &act(&[CursorDown, Confirm]));
    assert_eq!(s.choice(), None);
    assert_eq!(
        sounds(&mut c),
        [
            cue(MenuSound::Select),
            cue(MenuSound::Move),
            cue(MenuSound::Select)
        ]
    );
    // Change waits for a button.
    s.update(&mut c, &act(&[Confirm]));
    sounds(&mut c);
    s.update(&mut c, &act(&[Confirm, CursorDown]));
    assert!(s.is_capturing());
    assert_eq!(s.choice(), None);
    assert_eq!(sounds(&mut c), [cue(MenuSound::Select)]);
    // On Restore defaults Confirm still asks; no choices there.
    tap(&mut s, &mut c, Button::West);
    focus_on_restore(&mut s, &mut c);
    s.update(&mut c, &act(&[Confirm]));
    assert!(s.is_asking_restore());
    assert_eq!(s.choice(), None);
}

#[test]
fn the_view_offers_the_choices_for_the_slot_with_the_cursor() {
    let mut c = pad_ctx();
    let mut s = KeyBindingsScreen::new(&c, &KEY_ROWS);
    assert_eq!(s.view(&c).choices, None);
    focus_on(&mut s, &mut c, Info, 1);
    s.update(&mut c, &act(&[Confirm]));
    let view = s.view(&c);
    let choices = view.choices.clone().unwrap();
    assert_eq!(choices.lines, ["Change", "Clear"]);
    assert_eq!((choices.focused, choices.slot), (0, 1));
    assert_eq!(
        view.focused().map(|r| (r.action, r.focus)),
        Some((Info, Some(1)))
    );
    s.update(&mut c, &act(&[CursorDown]));
    assert_eq!(s.view(&c).choices.unwrap().focused, 1);
    // Cancel closes them.
    s.update(&mut c, &act(&[Cancel]));
    assert_eq!(s.view(&c).choices, None);
}

#[test]
fn on_a_controller_the_keyboard_side_still_takes_only_keys() {
    let mut c = pad_ctx();
    let mut s = screen(&c);
    s.update(&mut c, &act(&[CursorUp, CursorLeft]));
    assert_eq!(s.side(), Side::Keyboard);
    change(&mut s, &mut c, Info, 1);
    assert_eq!(shown(&s.view(&c), Info)[1], "…Press a key…");
    // A button let go isn't a key: still waiting.
    tap(&mut s, &mut c, Button::West);
    assert!(s.is_capturing());
    assert_eq!(held(&s), None);
    assert_eq!(s.pad_bindings(), &c.pad_bindings());
    // Holding one backs out.
    s.update(&mut c, &buttons(&[Button::West], &[], 0.0));
    s.update(&mut c, &buttons(&[], &[], 1.0));
    assert!(!s.is_capturing());
    s.update(&mut c, &buttons(&[], &[Button::West], 0.0));
    // And a key goes in.
    change(&mut s, &mut c, Info, 1);
    s.update(&mut c, &press("g"));
    assert_eq!(s.bindings().slots(Info)[1], Some(chord("g")));
    // Clear empties the key slot, not a button.
    s.update(&mut c, &act(&[Confirm]));
    s.update(&mut c, &act(&[CursorDown, Confirm]));
    assert_eq!(s.bindings().slots(Info)[1], None);
    assert_eq!(s.pad_bindings(), &c.pad_bindings());
}

#[test]
fn on_the_controller_side_the_keyboards_fixed_keys_still_work() {
    let mut c = ctx();
    let mut s = screen(&c);
    s.update(&mut c, &act(&[CursorUp, CursorRight]));
    focus_on(&mut s, &mut c, Info, 0);
    // The clear-slot key empties the button slot.
    sounds(&mut c);
    s.update(&mut c, &press("Delete"));
    assert_eq!(pad_names(&s, Info), ["-", "-", "-"]);
    assert_eq!(s.bindings(), &screen(&c).bindings);
    assert_eq!(sounds(&mut c), [cue(MenuSound::Select)]);
    s.update(&mut c, &press("Delete"));
    assert!(sounds(&mut c).is_empty(), "already empty");
    // The back-out key ends a capture, even with a button held; the
    // clear-slot key does nothing during one.
    s.update(&mut c, &act(&[Confirm]));
    s.update(&mut c, &buttons(&[Button::West], &[], 0.0));
    s.update(&mut c, &press("Delete"));
    assert!(s.is_capturing());
    let escape = press("Escape").with_buttons(vec![], vec![Button::West]);
    s.update(&mut c, &escape);
    assert!(!s.is_capturing());
    assert_eq!(held(&s), None);
    assert_eq!(pad_names(&s, Info), ["-", "-", "-"]);
}

#[test]
fn cursor_buttons_held_after_a_capture_dont_move_the_focus() {
    let mut c = pad_ctx();
    let mut s = KeyBindingsScreen::new(&c, &KEY_ROWS);
    change(&mut s, &mut c, Info, 0);
    s.update(&mut c, &buttons(&[Button::DpadDown], &[], 0.0));
    s.update(&mut c, &FrameInput::new(vec![], 1.0, vec![CursorDown]));
    assert!(!s.is_capturing());
    assert!(s.await_release);
    // Still held: its repeats are ignored until it is let go.
    let held = FrameInput::new(vec![CursorDown], 0.1, vec![CursorDown]);
    s.update(&mut c, &held);
    assert_eq!(s.focus(), Some((Info, 0)));
    s.update(&mut c, &buttons(&[], &[Button::DpadDown], 0.0));
    s.update(&mut c, &held);
    assert_eq!(s.focus(), Some((Action::DangerZone, 0)));
    // A button tapped into a slot is up again: the next press moves.
    change(&mut s, &mut c, Action::DangerZone, 0);
    tap(&mut s, &mut c, Button::RightStickPress);
    s.update(&mut c, &held);
    assert_eq!(s.focus(), Some((Action::ToggleAutoEnd, 0)));
}

#[test]
fn leaving_is_blocked_while_a_required_action_has_no_button() {
    let mut c = pad_ctx();
    let mut s = KeyBindingsScreen::new(&c, &KEY_ROWS);
    change(&mut s, &mut c, Info, 0);
    tap(&mut s, &mut c, Button::South);
    change(&mut s, &mut c, Info, 1);
    tap(&mut s, &mut c, Button::Start);
    sounds(&mut c);
    let t = s.update(&mut c, &act(&[Cancel]));
    assert_eq!(format!("{t:?}"), "None");
    // The one listed higher on the screen is named.
    assert_eq!(s.message(), Some("Give Confirm a button first"));
    assert_eq!(sounds(&mut c), [cue(MenuSound::Denied)]);
    assert_eq!(c.pad_bindings(), PadBindings::defaults(c.game.keymap_def()));
    // From the keyboard side too: the screen shows the side that lacks one.
    s.update(&mut c, &act(&[CursorDown]));
    assert_eq!(s.message(), None);
    s.side = Side::Keyboard;
    s.update(&mut c, &act(&[Cancel]));
    assert_eq!(s.side(), Side::Controller);
    assert_eq!(s.message(), Some("Give Confirm a button first"));
    // When both sides lack one, the shown side's is named and it stays.
    s.bindings.clear(Action::EndTurn, 0);
    s.update(&mut c, &act(&[Cancel]));
    assert_eq!(s.side(), Side::Controller);
    s.side = Side::Keyboard;
    s.update(&mut c, &act(&[Cancel]));
    assert_eq!(s.side(), Side::Keyboard);
    assert_eq!(s.message(), Some("Give End turn a key first"));
    // An optional action may be left with no button.
    let mut s = KeyBindingsScreen::new(&c, &KEY_ROWS);
    focus_on(&mut s, &mut c, Info, 0);
    s.update(&mut c, &act(&[Confirm]));
    s.update(&mut c, &act(&[CursorDown, Confirm]));
    let t = s.update(&mut c, &act(&[Cancel]));
    assert_eq!(format!("{t:?}"), "Pop");
}

#[test]
fn leaving_saves_the_buttons_for_both_layouts_and_leaves_the_keys() {
    let mut c = pad_ctx();
    let mut s = KeyBindingsScreen::new(&c, &KEY_ROWS);
    change(&mut s, &mut c, Info, 1);
    tap(&mut s, &mut c, Button::RightStickPress);
    // Nothing changes in the game until the screen closes.
    assert_eq!(c.keymap.pad_action(Button::RightStickPress), None);
    assert_eq!(c.storage.read(crate::ctx::KEYBINDINGS_KEY), Ok(None));
    sounds(&mut c);
    let t = s.update(&mut c, &act(&[Cancel]));
    assert_eq!(format!("{t:?}"), "Pop");
    assert_eq!(sounds(&mut c), [cue(MenuSound::Cancel)]);
    assert_eq!(&c.pad_bindings(), s.pad_bindings());
    assert_eq!(c.keymap.pad_action(Button::RightStickPress), Some(Info));
    assert!(c.player_keys().is_custom_pad());
    for layout in [RIGHT_HANDED, LEFT_HANDED] {
        assert!(!c.player_keys().is_custom(layout));
        let km = c.keymap_for(layout);
        assert_eq!(
            km.pad_action(Button::RightStickPress),
            Some(Info),
            "{layout}"
        );
    }
    let saved = c.storage.read(crate::ctx::KEYBINDINGS_KEY);
    assert_eq!(saved, Ok(Some(c.player_keys().to_ron())));
    // Leaving with nothing changed writes nothing.
    let mut c = pad_ctx();
    let mut s = KeyBindingsScreen::new(&c, &KEY_ROWS);
    s.update(&mut c, &act(&[Cancel]));
    assert_eq!(c.storage.read(crate::ctx::KEYBINDINGS_KEY), Ok(None));
}

#[test]
fn restore_defaults_on_the_controller_side_resets_only_the_buttons() {
    let mut c = pad_ctx();
    let mut s = screen(&c);
    // A key change on the other side stays.
    s.bindings.bind(Info, 1, chord("g")).unwrap();
    let keys = s.bindings().clone();
    change(&mut s, &mut c, Info, 0);
    tap(&mut s, &mut c, Button::South);
    assert!(s.moved.is_some());
    focus_on_restore(&mut s, &mut c);
    s.update(&mut c, &act(&[Confirm]));
    assert!(s.is_asking_restore());
    assert_eq!(s.restore_question(&c), "Restore the default buttons?");
    let question = s.view(&c).question.unwrap();
    assert_eq!(question.text, "Restore the default buttons?");
    assert_eq!(question.keys.as_deref(), Some("A yes / B no"));
    s.update(&mut c, &act(&[Confirm]));
    assert_eq!(
        s.pad_bindings(),
        &PadBindings::defaults(c.game.keymap_def())
    );
    assert_eq!(s.bindings(), &keys);
    assert_eq!(s.moved, None);
    // And on the keyboard side, only the keys.
    change(&mut s, &mut c, Info, 0);
    tap(&mut s, &mut c, Button::West);
    let pad = s.pad_bindings().clone();
    s.side = Side::Keyboard;
    assert_eq!(
        s.restore_question(&c),
        "Restore the default keys for Right-handed?"
    );
    focus_on_restore(&mut s, &mut c);
    s.update(&mut c, &act(&[Confirm]));
    s.update(&mut c, &act(&[Confirm]));
    assert_ne!(s.bindings(), &keys);
    assert_eq!(s.pad_bindings(), &pad);
}

#[test]
fn help_follows_the_side_and_the_device() {
    let mut c = pad_ctx();
    let mut s = KeyBindingsScreen::new(&c, &KEY_ROWS);
    assert_eq!(s.help(&c), "D-pad/L-stick move · A change · B back");
    s.update(&mut c, &act(&[Confirm]));
    assert_eq!(s.help(&c), "D-pad/L-stick move · A select · B back");
    s.update(&mut c, &act(&[Confirm]));
    assert_eq!(s.help(&c), "Press a button… · hold any button to cancel");
    // The keyboard used last: its back-out key is named too.
    c.device = Device::Keyboard;
    assert_eq!(
        s.help(&c),
        "Press a button… · hold any button to cancel · Escape back"
    );
    s.update(&mut c, &press("Escape"));
    assert_eq!(s.help(&c), "arrows move · f bind · Delete clear · d back");
    // A key slot waiting, on a controller and on the keyboard.
    s.side = Side::Keyboard;
    s.update(&mut c, &act(&[Confirm]));
    assert_eq!(s.help(&c), "Press the key to put here · Escape back");
    c.device = XBOX;
    assert_eq!(
        s.help(&c),
        "Press the key to put here · hold any button to cancel"
    );
    s.update(&mut c, &press("Escape"));
    // The switch and Restore defaults.
    s.update(&mut c, &act(&[CursorUp]));
    assert!(s.is_on_switch());
    assert_eq!(
        s.help(&c),
        "←/→ keyboard or controller · D-pad/L-stick move · B back"
    );
    c.device = Device::Keyboard;
    assert_eq!(
        s.help(&c),
        "Left/Right keyboard or controller · arrows move · d back"
    );
    s.update(&mut c, &act(&[CursorUp]));
    assert_eq!(s.help(&c), "arrows move · f restore · d back");
    c.device = XBOX;
    assert_eq!(s.help(&c), "D-pad/L-stick move · A restore · B back");
    s.update(&mut c, &act(&[Confirm]));
    assert_eq!(s.help(&c), "A yes · B no");
    // Every line fits the console on every pad.
    for kind in [PadKind::PlayStation, PadKind::Nintendo, PadKind::Generic] {
        c.device = Device::Pad(kind);
        let mut s = KeyBindingsScreen::new(&c, &KEY_ROWS);
        let mut lines = vec![s.help(&c)];
        s.update(&mut c, &act(&[CursorUp]));
        lines.push(s.help(&c));
        for line in lines {
            assert!(line.chars().count() <= usize::from(PC.cols), "{line}");
        }
    }
}

#[test]
fn the_screen_says_what_it_is_and_is_found_by_type() {
    let c = ctx();
    let s = KeyBindingsScreen::new(&c, &KEY_ROWS);
    let shown = format!("{s:?}");
    assert!(shown.starts_with("KeyBindingsScreen {"), "{shown}");
    assert!(
        shown.contains("side: Keyboard") && shown.contains("capturing: false"),
        "{shown}"
    );
    let any = s.as_any().unwrap();
    assert!(any.downcast_ref::<KeyBindingsScreen>().is_some());
}

#[test]
fn only_the_row_after_the_actions_is_restore_defaults() {
    let mut c = ctx();
    let mut s = KeyBindingsScreen::new(&c, &KEY_ROWS);
    // On an action, the second say: not Restore defaults.
    s.update(&mut c, &act(&[CursorDown]));
    assert_eq!(s.focus(), Some((CursorDown, 0)));
    assert!(!s.view(&c).restore.focused);
    // Up from the first action is the switch, up again Restore defaults.
    s.update(&mut c, &act(&[CursorUp, CursorUp, CursorUp]));
    assert_eq!(s.focus(), None);
    assert!(!s.is_on_switch());
    assert!(s.view(&c).restore.focused);
}
