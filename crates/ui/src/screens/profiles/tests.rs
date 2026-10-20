//! The profiles screen on the toy game: its first profile is `Player 1`.

use shuyi_assets::keymap::{Chord, Key};

use super::*;
use crate::frame::color::EngineColor;
use crate::frame::console::ConsoleSpec;
use crate::frame::glyph_buffer::Cell;
use crate::settings::{Settings, TextSpeed};
use crate::toy::{Action, Picker, Toy, ToyCtx, ctx};

use Action::{Cancel, Confirm, CursorDown, CursorLeft, CursorRight, CursorUp};

type Ctx = ToyCtx;
type ProfilesScreen = super::ProfilesScreen<Toy>;
type FrameInput = crate::screen::FrameInput<Action>;

fn press(s: &mut ProfilesScreen, c: &mut Ctx, actions: &[Action]) -> String {
    let input = FrameInput::new(actions.to_vec(), 0.0, vec![]);
    format!("{:?}", s.update(c, &input))
}

/// `actions` from a controller.
fn pad(s: &mut ProfilesScreen, c: &mut Ctx, actions: &[Action]) -> String {
    let input = FrameInput::new(actions.to_vec(), 0.0, vec![]).with_pad(true);
    format!("{:?}", s.update(c, &input))
}

/// `text` typed and the keys `keys` pressed, in one frame.
fn typing(s: &mut ProfilesScreen, c: &mut Ctx, text: &str, keys: &[Key]) -> String {
    let chords = keys.iter().map(|&k| Chord::plain(k)).collect();
    let input = FrameInput::new(vec![], 0.0, vec![]).with_typing(chords, text.chars().collect());
    format!("{:?}", s.update(c, &input))
}

fn ok<T, E: core::fmt::Display>(result: Result<T, E>) -> T {
    result.unwrap_or_else(|e| panic!("{e}"))
}

/// The toy's context with `Player 1` (in use), `Mei` and `Odo`.
fn three() -> Ctx {
    let mut c = ctx();
    ok(c.add_profile("Mei"));
    ok(c.add_profile("Odo"));
    c
}

fn names(c: &Ctx) -> Vec<String> {
    c.profiles().all().iter().map(|p| p.name.clone()).collect()
}

fn id_of(c: &Ctx, name: &str) -> ProfileId {
    let profile = c.profiles().all().iter().find(|p| p.name == name);
    profile.map_or_else(|| panic!("no {name}"), |p| p.id)
}

/// The labels of the open menu of what can be done to a profile.
fn actions(s: &ProfilesScreen, c: &Ctx) -> Vec<String> {
    let view = s.view(c).actions.unwrap_or_else(|| panic!("no menu"));
    view.menu.items.iter().map(|i| c.label(&i.label)).collect()
}

/// The focused item of the open menu.
fn focused_action(s: &ProfilesScreen, c: &Ctx) -> Option<String> {
    let view = s.view(c).actions?;
    view.menu.focused().map(|i| c.label(&i.label))
}

/// The answer in focus of the question asked before deleting.
fn delete_answer(s: &ProfilesScreen, c: &Ctx) -> Option<DeleteChoice> {
    s.view(c).deleting.and_then(|q| q.focused())
}

#[test]
fn the_view_lists_the_profiles_the_one_in_use_marked_and_focused_then_add() {
    let mut c = three();
    ok(c.switch_profile(id_of(&c, "Mei")));
    let s = ProfilesScreen::new(&c);
    let view = s.view(&c);
    let labels: Vec<&str> = view.rows.iter().map(|r| r.label.as_str()).collect();
    assert_eq!(labels, ["Player 1", "Mei", "Odo", "Add a profile"]);
    assert_eq!(view.current().map(|r| r.label.as_str()), Some("Mei"));
    assert_eq!(view.focused().map(|r| r.label.as_str()), Some("Mei"));
    assert_eq!(view.current_mark, "in use");
    assert_eq!(view.title, "Profile");
    assert_eq!(view.rows.last().map(|r| r.kind), Some(RowKind::Add));
    assert!(view.rows.iter().all(|r| r.enabled));
}

#[test]
fn the_rows_wrap_and_cancel_goes_back() {
    let mut c = three();
    let mut s = ProfilesScreen::new(&c);
    assert_eq!(s.help(&c), "arrows move · f choose · d back");
    press(&mut s, &mut c, &[CursorUp]);
    assert_eq!(s.focused(&c), RowKind::Add);
    assert_eq!(s.help(&c), "arrows move · f add · d back");
    press(&mut s, &mut c, &[CursorDown]);
    assert_eq!(s.focused(&c), RowKind::Profile(ProfileId::FIRST));
    assert_eq!(press(&mut s, &mut c, &[Cancel]), "Pop");
}

#[test]
fn a_profile_offers_switch_unless_in_use_and_delete_unless_the_only_one() {
    let mut c = ctx();
    let mut s = ProfilesScreen::new(&c);
    press(&mut s, &mut c, &[Confirm]);
    assert_eq!(actions(&s, &c), ["Rename"]);
    let mut c = three();
    let mut s = ProfilesScreen::new(&c);
    press(&mut s, &mut c, &[Confirm]);
    assert_eq!(actions(&s, &c), ["Rename", "Delete"]);
    assert_eq!(s.view(&c).actions.map(|a| a.title), Some("Player 1".into()));
    // Cancel closes the menu, not the screen.
    assert_eq!(press(&mut s, &mut c, &[Cancel]), "None");
    assert!(s.view(&c).actions.is_none());
    press(&mut s, &mut c, &[CursorDown, Confirm]);
    assert_eq!(actions(&s, &c), ["Use this profile", "Rename", "Delete"]);
    assert_eq!(s.help(&c), "arrows move · f select · d back");
    // The cursor moves over the items and wraps both ways.
    assert_eq!(focused_action(&s, &c).as_deref(), Some("Use this profile"));
    press(&mut s, &mut c, &[CursorUp]);
    assert_eq!(focused_action(&s, &c).as_deref(), Some("Delete"));
    press(&mut s, &mut c, &[CursorDown]);
    assert_eq!(focused_action(&s, &c).as_deref(), Some("Use this profile"));
    press(&mut s, &mut c, &[CursorDown]);
    assert_eq!(focused_action(&s, &c).as_deref(), Some("Rename"));
}

/// The press that opens the menu or the name box isn't its first: the
/// rest of that frame's presses are dropped.
#[test]
fn the_press_that_opens_something_ends_the_frame() {
    let mut c = three();
    let mut s = ProfilesScreen::new(&c);
    press(&mut s, &mut c, &[CursorDown, Confirm, CursorDown]);
    assert_eq!(focused_action(&s, &c).as_deref(), Some("Use this profile"));
    press(&mut s, &mut c, &[Cancel]);
    // The row that adds one: the box opens and the focus stays there.
    press(&mut s, &mut c, &[CursorDown, CursorDown, Confirm, CursorUp]);
    assert_eq!(s.naming(), Some(NamePurpose::Add));
    assert_eq!(s.focused(&c), RowKind::Add);
}

#[test]
fn switching_uses_that_profiles_options_and_closes() {
    let mut c = three();
    let mei = id_of(&c, "Mei");
    ok(c.switch_profile(mei));
    ok(c.change_settings(|s| s.text_speed = TextSpeed::Slow));
    ok(c.switch_profile(ProfileId::FIRST));
    assert_ne!(c.settings().text_speed, TextSpeed::Slow);
    let mut s = ProfilesScreen::new(&c);
    // Mei, the menu, `Use this profile`.
    assert_eq!(press(&mut s, &mut c, &[CursorDown, Confirm]), "None");
    assert_eq!(press(&mut s, &mut c, &[Confirm]), "Pop");
    assert_eq!(c.profiles().current(), mei);
    assert_eq!(c.settings().text_speed, TextSpeed::Slow);
}

#[test]
fn deleting_asks_first_focused_on_no() {
    let mut c = three();
    let mut s = ProfilesScreen::new(&c);
    // Mei, the menu, down past Rename to Delete.
    press(&mut s, &mut c, &[CursorDown, Confirm]);
    press(&mut s, &mut c, &[CursorDown, CursorDown, Confirm]);
    let question = s.view(&c).deleting.unwrap_or_else(|| panic!("no question"));
    assert_eq!(
        question.text,
        "Delete Mei? Its options and saves go with it."
    );
    assert_eq!(question.focused(), Some(DeleteChoice::No));
    assert_eq!(s.help(&c), "arrows choose · f pick · d back");
    // Confirm at once is No: nothing is deleted.
    press(&mut s, &mut c, &[Confirm]);
    assert!(s.view(&c).deleting.is_none());
    assert_eq!(names(&c), ["Player 1", "Mei", "Odo"]);
    // Cancel is No too.
    press(&mut s, &mut c, &[Confirm]);
    press(&mut s, &mut c, &[CursorDown, CursorDown, Confirm]);
    assert_eq!(delete_answer(&s, &c), Some(DeleteChoice::No));
    press(&mut s, &mut c, &[Cancel]);
    assert_eq!(names(&c), ["Player 1", "Mei", "Odo"]);
    // Left to Yes, Confirm: deleted.
    press(&mut s, &mut c, &[Confirm]);
    press(&mut s, &mut c, &[CursorDown, CursorDown, Confirm]);
    press(&mut s, &mut c, &[CursorLeft]);
    assert_eq!(delete_answer(&s, &c), Some(DeleteChoice::Yes));
    // The question widget goes round both ways.
    press(&mut s, &mut c, &[CursorRight]);
    assert_eq!(delete_answer(&s, &c), Some(DeleteChoice::No));
    press(&mut s, &mut c, &[CursorRight]);
    assert_eq!(delete_answer(&s, &c), Some(DeleteChoice::Yes));
    assert_eq!(press(&mut s, &mut c, &[Confirm]), "None");
    assert_eq!(names(&c), ["Player 1", "Odo"]);
}

#[test]
fn deleting_the_profile_in_use_uses_the_first_one_left_and_keeps_the_focus_in_the_list() {
    let mut c = three();
    let odo = id_of(&c, "Odo");
    ok(c.switch_profile(odo));
    let mut s = ProfilesScreen::new(&c);
    assert_eq!(s.focused(&c), RowKind::Profile(odo));
    // In use: Rename, Delete.
    press(&mut s, &mut c, &[Confirm]);
    press(&mut s, &mut c, &[CursorDown, Confirm]);
    press(&mut s, &mut c, &[CursorLeft]);
    press(&mut s, &mut c, &[Confirm]);
    assert_eq!(names(&c), ["Player 1", "Mei"]);
    assert_eq!(c.profiles().current(), ProfileId::FIRST);
    assert_eq!(s.focused(&c), RowKind::Profile(id_of(&c, "Mei")));
}

#[test]
fn adding_past_the_most_profiles_is_refused_with_a_message() {
    let mut c = ctx();
    for i in 2..=MAX_PROFILES {
        ok(c.add_profile(&format!("P{i}")));
    }
    let mut s = ProfilesScreen::new(&c);
    press(&mut s, &mut c, &[CursorUp]);
    let view = s.view(&c);
    assert_eq!(view.focused().map(|r| r.enabled), Some(false));
    assert_eq!(s.help(&c), "arrows move · d back");
    press(&mut s, &mut c, &[Confirm]);
    assert!(s.typed().is_none());
    assert_eq!(s.message(), Some(TOO_MANY_MESSAGE));
    assert_eq!(
        s.view(&c).message.as_deref(),
        Some("There can be at most 8 profiles.")
    );
    // The message goes with the next move.
    press(&mut s, &mut c, &[CursorUp]);
    assert_eq!(s.message(), None);
}

#[test]
fn on_the_keyboard_add_types_the_name_and_uses_the_new_profile() {
    let mut c = three();
    let mut s = ProfilesScreen::new(&c);
    press(&mut s, &mut c, &[CursorUp, Confirm]);
    assert_eq!(s.naming(), Some(NamePurpose::Add));
    let view = s.view(&c);
    assert_eq!(view.typing.map(|t| t.title), Some("Profile name".into()));
    assert_eq!(s.help(&c), "Enter done · Backspace delete · Escape cancel");
    // The game's keys do nothing while the box is open.
    press(&mut s, &mut c, &[Cancel]);
    assert_eq!(s.typed(), Some(""));
    typing(&mut s, &mut c, "Kai", &[]);
    assert_eq!(s.typed(), Some("Kai"));
    assert_eq!(typing(&mut s, &mut c, "", &[Key::Enter]), "Pop");
    assert_eq!(names(&c), ["Player 1", "Mei", "Odo", "Kai"]);
    assert_eq!(c.profiles().current(), id_of(&c, "Kai"));
}

#[test]
fn renaming_to_another_profiles_name_is_refused_and_the_box_stays() {
    let mut c = three();
    let mut s = ProfilesScreen::new(&c);
    // Player 1, the menu, Rename.
    press(&mut s, &mut c, &[Confirm]);
    press(&mut s, &mut c, &[Confirm]);
    assert_eq!(s.naming(), Some(NamePurpose::Rename(ProfileId::FIRST)));
    assert_eq!(s.typed(), Some("Player 1"));
    let delete = [Key::Backspace; 8];
    typing(&mut s, &mut c, "", &delete);
    typing(&mut s, &mut c, "mei", &[Key::Enter]);
    assert_eq!(s.message(), Some(NAME_TAKEN_MESSAGE));
    assert_eq!(s.typed(), Some("mei"));
    typing(&mut s, &mut c, "", &[Key::Backspace, Key::Backspace]);
    assert_eq!(typing(&mut s, &mut c, "ra", &[Key::Enter]), "None");
    assert_eq!(s.typed(), None);
    assert_eq!(s.message(), None);
    assert_eq!(names(&c), ["mra", "Mei", "Odo"]);
}

#[test]
fn backing_out_of_a_typed_name_asks_whether_to_keep_it() {
    let mut c = three();
    let mut s = ProfilesScreen::new(&c);
    press(&mut s, &mut c, &[CursorUp, Confirm]);
    typing(&mut s, &mut c, "Kai", &[Key::Escape]);
    let question = s.view(&c).leaving.unwrap_or_else(|| panic!("no question"));
    assert_eq!(question.text, "Keep this name?");
    assert_eq!(question.focused(), Some(LeaveChoice::Back));
    assert_eq!(s.help(&c), "arrows choose · f pick · d back");
    // Back: the box again, the name as it was.
    press(&mut s, &mut c, &[Cancel]);
    assert!(s.view(&c).leaving.is_none());
    assert_eq!(s.typed(), Some("Kai"));
    // Discard: the box closes, nothing added.
    typing(&mut s, &mut c, "", &[Key::Escape]);
    press(&mut s, &mut c, &[CursorLeft, Confirm]);
    assert_eq!(s.typed(), None);
    assert_eq!(names(&c), ["Player 1", "Mei", "Odo"]);
    // Keep: added.
    press(&mut s, &mut c, &[Confirm]);
    typing(&mut s, &mut c, "Kai", &[Key::Escape]);
    assert_eq!(press(&mut s, &mut c, &[CursorRight, Confirm]), "Pop");
    assert_eq!(names(&c), ["Player 1", "Mei", "Odo", "Kai"]);
}

#[test]
fn on_a_controller_add_spells_the_name_on_the_grid_and_opens_what_the_game_gives() {
    let mut c = three();
    let mut s = ProfilesScreen::new(&c).opening_after_add(|_| Box::new(Picker::default()));
    pad(&mut s, &mut c, &[CursorUp]);
    pad(&mut s, &mut c, &[Confirm]);
    assert_eq!(s.naming(), Some(NamePurpose::Add));
    let grid = s.view(&c).grid.unwrap_or_else(|| panic!("no grid"));
    assert_eq!(grid.rows.len(), 6, "a row of digits");
    assert_eq!(s.help(&c), "arrows choose · f type · d delete");
    // "A", then up round to the last row and left round to Done.
    pad(&mut s, &mut c, &[Confirm]);
    assert_eq!(s.typed(), Some("A"));
    pad(&mut s, &mut c, &[CursorUp, CursorLeft]);
    let transition = pad(&mut s, &mut c, &[Confirm]);
    assert_eq!(transition, "Replace(picker)");
    assert_eq!(names(&c), ["Player 1", "Mei", "Odo", "A"]);
    assert_eq!(c.profiles().current(), id_of(&c, "A"));
    // The new profile starts from the default options, with no layout.
    assert_eq!(*c.settings(), Settings::default());
    assert_eq!(c.layout(), None);
}

/// The list filling up while the name box is open (not something the
/// screen does): keeping the name is refused with the same message.
#[test]
fn a_full_list_refuses_the_name_kept() {
    let mut c = ctx();
    for i in 2..MAX_PROFILES {
        ok(c.add_profile(&format!("P{i}")));
    }
    let mut s = ProfilesScreen::new(&c);
    pad(&mut s, &mut c, &[CursorUp]);
    pad(&mut s, &mut c, &[Confirm]);
    ok(c.add_profile("Last"));
    pad(&mut s, &mut c, &[Confirm]);
    pad(&mut s, &mut c, &[CursorUp, CursorLeft]);
    assert_eq!(pad(&mut s, &mut c, &[Confirm]), "None");
    assert_eq!(s.typed(), Some("A"));
    assert_eq!(s.message(), Some(TOO_MANY_MESSAGE));
    assert_eq!(c.profiles().all().len(), MAX_PROFILES);
}

#[test]
fn cancel_on_an_empty_grid_closes_it_without_asking() {
    let mut c = three();
    let mut s = ProfilesScreen::new(&c);
    pad(&mut s, &mut c, &[CursorUp]);
    pad(&mut s, &mut c, &[Confirm]);
    pad(&mut s, &mut c, &[Cancel]);
    assert!(s.typed().is_none());
    assert!(s.view(&c).leaving.is_none());
    assert_eq!(names(&c), ["Player 1", "Mei", "Odo"]);
}

#[test]
fn the_screen_is_named_and_found_by_its_type() {
    let c = ctx();
    let s = ProfilesScreen::new(&c);
    assert_eq!(s.name(), "profiles");
    let any = s.as_any().unwrap_or_else(|| panic!("no Any"));
    assert!(any.downcast_ref::<ProfilesScreen>().is_some());
    assert!(format!("{s:?}").starts_with("ProfilesScreen {"));
}

#[test]
fn a_game_paints_the_view_its_own_way() {
    fn titled(c: &Ctx, view: &ProfilesView, buf: &mut GlyphBuffer) {
        let text = c.palette.engine(EngineColor::Text);
        buf.print(0, 0, &view.title, text, text);
    }
    let c = ctx();
    let s = ProfilesScreen::new(&c).painted_by(titled);
    let black = c.palette.engine(EngineColor::Background);
    let spec = ConsoleSpec::text(10, 2);
    let mut buf = GlyphBuffer::with_spec(spec, Cell::new(' ', black, black));
    s.draw(&c, &mut buf);
    assert_eq!(buf.get(0, 0).map(|c| c.glyph), Some('P'));
}

/// The screen painted on a 100 × 32 console.
fn snapshot(s: &ProfilesScreen, c: &Ctx) -> String {
    let black = c.palette.engine(EngineColor::Background);
    let spec = ConsoleSpec::text(100, 32);
    let mut buf = GlyphBuffer::with_spec(spec, Cell::new(' ', black, black));
    s.draw(c, &mut buf);
    buf.to_snapshot(&c.palette)
}

/// The screen with its profiles.
#[test]
fn profiles_screen_snapshot() {
    let mut c = three();
    let mut s = ProfilesScreen::new(&c);
    press(&mut s, &mut c, &[CursorDown]);
    insta::assert_snapshot!(snapshot(&s, &c));
}

/// The screen with only the first profile.
#[test]
fn one_profile_snapshot() {
    let c = ctx();
    let s = ProfilesScreen::new(&c);
    insta::assert_snapshot!(snapshot(&s, &c));
}

/// Deleting Mei: the question over the list, focused on No.
#[test]
fn delete_question_snapshot() {
    let mut c = ctx();
    ok(c.add_profile("Mei"));
    let mut s = ProfilesScreen::new(&c);
    press(&mut s, &mut c, &[CursorDown, Confirm]);
    press(&mut s, &mut c, &[CursorDown, CursorDown, Confirm]);
    insta::assert_snapshot!(snapshot(&s, &c));
}

/// Spelling a name on the grid, over the list.
#[test]
fn name_grid_snapshot() {
    let mut c = ctx();
    let mut s = ProfilesScreen::new(&c);
    pad(&mut s, &mut c, &[CursorUp]);
    pad(&mut s, &mut c, &[Confirm]);
    pad(&mut s, &mut c, &[Confirm, CursorDown]);
    insta::assert_snapshot!(snapshot(&s, &c));
}
