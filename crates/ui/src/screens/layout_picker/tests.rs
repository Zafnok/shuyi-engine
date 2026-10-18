//! The layout picker on the toy game, whose keymap file has a
//! right-handed and a left-handed layout.

use shuyi_assets::keymap::KeymapDef;

use super::*;
use crate::input::help_keys::NOT_MAPPED;
use crate::toy::{Action, LEFT_HANDED, LEGEND, RIGHT_HANDED, Toy, ToyCtx, ctx};

type LayoutPickerScreen = super::LayoutPickerScreen<Toy>;

fn input(actions: &[Action]) -> FrameInput<Action> {
    FrameInput::new(actions.to_vec(), 0.0, vec![])
}

fn first_launch_ctx() -> ToyCtx {
    crate::toy::first_launch()
}

fn picker(c: &ToyCtx) -> LayoutPickerScreen {
    LayoutPickerScreen::new(c, &LEGEND)
}

#[test]
fn down_and_confirm_pick_left_handed_and_save_it() {
    let mut c = first_launch_ctx();
    let mut p = picker(&c);
    assert_eq!(p.name(), "layout_picker");
    assert_eq!(p.focused(&c), Some(RIGHT_HANDED));
    let t = p.update(&mut c, &input(&[Action::CursorDown]));
    assert_eq!(format!("{t:?}"), "None");
    assert_eq!(p.focused(&c), Some(LEFT_HANDED));
    assert_eq!(c.layout(), None);
    let t = p.update(&mut c, &input(&[Action::Confirm]));
    assert_eq!(format!("{t:?}"), "Pop");
    assert_eq!(c.layout(), Some(LEFT_HANDED));
    assert_eq!(c.saved_layout(), Some(LEFT_HANDED));
}

#[test]
fn confirm_picks_the_focused_layout() {
    let mut c = first_launch_ctx();
    let mut p = picker(&c);
    let t = p.update(
        &mut c,
        &input(&[Action::CursorUp, Action::CursorUp, Action::Confirm]),
    );
    assert_eq!(format!("{t:?}"), "Pop");
    assert_eq!(c.saved_layout(), Some(RIGHT_HANDED));
}

#[test]
fn cancel_and_other_actions_do_nothing() {
    let mut c = first_launch_ctx();
    let mut p = picker(&c);
    let t = p.update(
        &mut c,
        &input(&[Action::Cancel, Action::Info, Action::CursorLeft]),
    );
    assert_eq!(format!("{t:?}"), "None");
    assert_eq!(c.layout(), None);
    assert_eq!(p.focused(&c), Some(RIGHT_HANDED));
}

/// Ticket 0226: any controller button closes the asked-for picker
/// without choosing; opened from Options, the pad steers it.
#[test]
fn a_pad_press_closes_only_the_asked_for_picker() {
    use shuyi_assets::keymap::Button;
    let pad = input(&[Action::CursorDown, Action::Confirm])
        .with_buttons(vec![Button::DpadDown, Button::South], vec![]);
    let mut c = first_launch_ctx();
    let mut p = picker(&c);
    assert_eq!(format!("{:?}", p.update(&mut c, &pad)), "Pop");
    assert_eq!(c.layout(), None);
    assert_eq!(p.focused(&c), Some(RIGHT_HANDED));
    // A release alone isn't a press.
    let up = input(&[]).with_buttons(vec![], vec![Button::South]);
    assert_eq!(format!("{:?}", p.update(&mut c, &up)), "None");
    let mut c = ctx();
    let mut p = LayoutPickerScreen::change(&c, &LEGEND);
    assert_eq!(format!("{:?}", p.update(&mut c, &pad)), "Pop");
    assert_eq!(c.layout(), Some(LEFT_HANDED));
}

#[test]
fn help_names_the_picker_keys() {
    assert_eq!(
        LayoutPickerScreen::help(&first_launch_ctx()),
        "w/Up s/Down choose · f/j/Enter/Space pick"
    );
    // Later (from Options) it names the chosen layout's keys.
    assert_eq!(LayoutPickerScreen::help(&ctx()), "Up Down choose · f pick");
}

#[test]
fn legend_comes_from_each_layout() {
    let c = first_launch_ctx();
    let def = c.game.keymap_def();
    let legend = |l| {
        picker(&c)
            .legend(&c, &Keymap::for_layout(def, l))
            .into_iter()
            .map(|row| format!("{} {}", row.keys, row.what))
            .collect::<Vec<_>>()
    };
    assert_eq!(
        legend(RIGHT_HANDED),
        [
            "arrows move",
            "f select",
            "d/Escape back",
            "a prev unit",
            "s next unit",
            "e unit info",
            "w danger zone",
            "Space end turn",
            "Shift+Space auto-end",
            "r rewind",
        ]
    );
    assert_eq!(
        legend(LEFT_HANDED),
        [
            "wasd move",
            "j select",
            "k/Escape back",
            "; prev unit",
            "l next unit",
            "i unit info",
            "o danger zone",
            "Space end turn",
            "Shift+Space auto-end",
            "u rewind",
        ]
    );
    let unbound = picker(&c).legend(
        &c,
        &Keymap::for_layout(&KeymapDef::<Action>::default(), LEFT_HANDED),
    );
    assert_eq!(unbound.len(), 10);
    assert!(unbound.iter().all(|r| r.keys == NOT_MAPPED), "{unbound:?}");
}

#[test]
fn key_roles() {
    let c = first_launch_ctx();
    let left = Keymap::for_layout(c.game.keymap_def(), LEFT_HANDED);
    assert_eq!(key_role(&left, Key::W), KeyRole::Movement);
    assert_eq!(key_role(&left, Key::J), KeyRole::Bound);
    assert_eq!(key_role(&left, Key::Up), KeyRole::Unbound);
}

#[test]
fn only_the_cursor_keys_are_movement_in_the_legend() {
    let c = first_launch_ctx();
    let km = Keymap::for_layout(c.game.keymap_def(), RIGHT_HANDED);
    let legend = picker(&c).legend(&c, &km);
    assert!(legend[0].movement);
    assert_eq!(legend.iter().filter(|r| r.movement).count(), 1);
}

#[test]
fn labels() {
    let c = ctx();
    assert_eq!(label(&c, RIGHT_HANDED), "Right-handed");
    assert_eq!(label(&c, LEFT_HANDED), "Left-handed");
}

/// The view of the picker as it opens on first launch: both layouts in
/// order, the first one focused, the picker's own help.
#[test]
fn the_first_frame_offers_both_layouts_with_the_first_focused() {
    let c = first_launch_ctx();
    let view = picker(&c).view(&c);
    assert_eq!(view.title, "Pick your layout");
    let names: Vec<_> = view
        .layouts
        .iter()
        .map(|l| (l.layout, l.name.as_str()))
        .collect();
    assert_eq!(
        names,
        [(RIGHT_HANDED, "Right-handed"), (LEFT_HANDED, "Left-handed")]
    );
    assert_eq!(
        view.layouts.iter().map(|l| l.focused).collect::<Vec<_>>(),
        [true, false]
    );
    assert_eq!(view.focused().map(|l| l.layout), Some(RIGHT_HANDED));
    assert_eq!(view.help, "w/Up s/Down choose · f/j/Enter/Space pick");
    // Each layout's legend is read from its own keys.
    let legend = |l: usize| view.layouts[l].legend[1].keys.clone();
    assert_eq!(legend(0), "f");
    assert_eq!(legend(1), "j");
}

#[test]
fn moving_the_cursor_moves_the_focus_in_the_view() {
    let mut c = first_launch_ctx();
    let mut p = picker(&c);
    p.update(&mut c, &input(&[Action::CursorDown]));
    let view = p.view(&c);
    assert_eq!(view.focused().map(|l| l.layout), Some(LEFT_HANDED));
    assert_eq!(
        view.layouts.iter().map(|l| l.focused).collect::<Vec<_>>(),
        [false, true]
    );
    // Up wraps back.
    p.update(&mut c, &input(&[Action::CursorUp]));
    assert_eq!(p.view(&c).focused().map(|l| l.layout), Some(RIGHT_HANDED));
}

/// Opened from Options: the layout in use is focused and the help names
/// Cancel too.
#[test]
fn opened_from_options_it_focuses_the_layout_in_use() {
    let c = ctx().with_layout(LEFT_HANDED);
    let view = LayoutPickerScreen::change(&c, &LEGEND).view(&c);
    assert_eq!(view.focused().map(|l| l.layout), Some(LEFT_HANDED));
    assert_eq!(view.help, c.text_with("layout_picker.help_change", &[]));
    assert_ne!(view.help, picker(&c).view(&c).help);
}

#[test]
fn the_keyboard_lists_every_key_with_its_role_in_each_layout() {
    let c = first_launch_ctx();
    let view = picker(&c).view(&c);
    let (right, left) = (&view.layouts[0], &view.layouts[1]);
    assert_eq!(right.keys.len(), Key::ALL.len());
    // Right-handed: the arrows move, `f` selects, `w` does the danger zone.
    assert_eq!(right.role(Key::Up), KeyRole::Movement);
    assert_eq!(right.role(Key::F), KeyRole::Bound);
    assert_eq!(right.role(Key::W), KeyRole::Bound);
    assert_eq!(right.role(Key::J), KeyRole::Unbound);
    // Left-handed: `wasd` move, `j` selects, the arrows do nothing.
    assert_eq!(left.role(Key::W), KeyRole::Movement);
    assert_eq!(left.role(Key::J), KeyRole::Bound);
    assert_eq!(left.role(Key::Up), KeyRole::Unbound);
    // A key that isn't listed is unbound.
    let none = LayoutView {
        keys: vec![],
        ..left.clone()
    };
    assert_eq!(none.role(Key::W), KeyRole::Unbound);
}

#[test]
fn the_picker_says_what_it_is_and_is_found_by_type() {
    let c = first_launch_ctx();
    let p = picker(&c);
    let shown = format!("{p:?}");
    assert!(shown.starts_with("LayoutPickerScreen {"), "{shown}");
    assert!(shown.contains("cancellable: false"), "{shown}");
    let any = p.as_any().unwrap();
    assert!(any.downcast_ref::<LayoutPickerScreen>().is_some());
    assert_eq!(p.clone().focused(&c), p.focused(&c));
}
