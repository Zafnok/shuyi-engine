//! The menu on the sample game's actions.

use proptest::prelude::*;
use shuyi_assets::keymap::sample::Action;

use super::*;
use crate::audio::AudioConfig;
use Action::{Cancel, Confirm, CursorDown, CursorLeft, CursorUp, Info};

fn menu(enabled: &[bool]) -> Menu {
    Menu::new(
        enabled
            .iter()
            .enumerate()
            .map(|(i, &on)| MenuItem {
                label: format!("item {i}"),
                enabled: on,
                suffix: None,
                note: None,
            })
            .collect(),
    )
}

/// The sound cues `actions` play on `m`, in order.
fn sounds(m: &mut Menu, actions: &[Action]) -> Vec<String> {
    let mut audio = AudioQueue::<()>::new(CUES);
    for &a in actions {
        m.handle_with_sound(a, &mut audio);
    }
    audio
        .take()
        .iter()
        .filter_map(|r| r.cue().map(str::to_owned))
        .collect()
}

/// Menu sounds that aren't the default ones.
const CUES: AudioConfig = AudioConfig {
    move_cue: "tick",
    select_cue: "ding",
    cancel_cue: "dong",
    denied_cue: "buzz",
    cursor_cue: "step",
};

#[test]
fn move_select_and_cancel_sound() {
    let mut m = menu(&[true, true, true]);
    assert_eq!(sounds(&mut m, &[CursorDown]), ["tick"]);
    assert_eq!(sounds(&mut m, &[CursorUp, CursorUp]), ["tick"; 2]);
    assert_eq!(m.focus(), 2, "wrapped");
    assert_eq!(sounds(&mut m, &[Confirm]), ["ding"]);
    assert_eq!(sounds(&mut m, &[Cancel]), ["dong"]);
    assert_eq!(sounds(&mut m, &[CursorLeft, Info]), Vec::<String>::new());
}

#[test]
fn handle_with_sound_reports_the_same_events() {
    let mut m = menu(&[true, true]);
    let mut audio = AudioQueue::<()>::default();
    assert_eq!(m.handle_with_sound(CursorDown, &mut audio), None);
    assert_eq!(
        m.handle_with_sound(Confirm, &mut audio),
        Some(MenuEvent::Chosen(1))
    );
    assert_eq!(
        m.handle_with_sound(Cancel, &mut audio),
        Some(MenuEvent::Cancelled)
    );
}

#[test]
fn a_disabled_item_is_denied() {
    // Nothing is enabled, so a disabled item is focused.
    let mut m = menu(&[false, false]);
    assert_eq!(sounds(&mut m, &[Confirm]), ["buzz"]);
    // An empty menu has nothing to deny.
    assert!(sounds(&mut Menu::new(vec![]), &[Confirm]).is_empty());
}

#[test]
fn doing_nothing_is_silent() {
    let mut m = menu(&[false, false]);
    assert!(sounds(&mut m, &[CursorDown, CursorUp]).is_empty());
    // A single enabled item: the focus has nowhere to go.
    let mut m = menu(&[false, true, false]);
    assert_eq!(
        sounds(&mut m, &[CursorDown, CursorUp]),
        Vec::<String>::new()
    );
    assert_eq!(sounds(&mut m, &[Confirm]), ["ding"]);
}

#[test]
fn without_cancel_ignores_cancel() {
    let mut m = menu(&[true, true]).without_cancel();
    assert_eq!(m.handle(Cancel), None);
    assert_eq!(sounds(&mut m, &[Cancel]), Vec::<String>::new());
    assert_eq!(m.handle(Confirm), Some(MenuEvent::Chosen(0)));
}

fn focus_after(m: &mut Menu, actions: &[Action]) -> usize {
    for &a in actions {
        m.handle(a);
    }
    m.focus()
}

#[test]
fn down_and_up_wrap() {
    let mut m = menu(&[true, true, true]);
    assert_eq!(m.focus(), 0);
    assert_eq!(focus_after(&mut m, &[CursorDown]), 1);
    assert_eq!(focus_after(&mut m, &[CursorDown]), 2);
    assert_eq!(focus_after(&mut m, &[CursorDown]), 0);
    assert_eq!(focus_after(&mut m, &[CursorUp]), 2);
    assert_eq!(focus_after(&mut m, &[CursorUp]), 1);
}

#[test]
fn disabled_items_are_skipped() {
    let mut m = menu(&[false, true, false, true, false]);
    assert_eq!(m.focus(), 1, "starts on the first enabled item");
    assert_eq!(focus_after(&mut m, &[CursorDown]), 3);
    assert_eq!(focus_after(&mut m, &[CursorDown]), 1);
    assert_eq!(focus_after(&mut m, &[CursorUp]), 3);
    assert_eq!(focus_after(&mut m, &[CursorUp]), 1);
}

#[test]
fn focused_moves_to_an_enabled_item_only() {
    let m = menu(&[true, false, true]);
    assert_eq!(m.clone().focused(2).focus(), 2);
    assert_eq!(m.clone().focused(1).focus(), 0, "disabled");
    assert_eq!(m.focused(9).focus(), 0, "no such item");
}

#[test]
fn a_single_enabled_item_keeps_focus() {
    let mut m = menu(&[false, true, false]);
    assert_eq!(focus_after(&mut m, &[CursorDown, CursorUp, CursorDown]), 1);
}

#[test]
fn nothing_enabled_nothing_chosen() {
    let mut m = menu(&[false, false]);
    assert_eq!(m.focus(), 0);
    assert_eq!(m.handle(CursorDown), None);
    assert_eq!(m.focus(), 0);
    assert_eq!(m.handle(Confirm), None);
    assert_eq!(m.handle(Cancel), Some(MenuEvent::Cancelled));
}

#[test]
fn empty_menu_is_harmless() {
    let mut m = Menu::new(vec![]);
    for &a in <Action as ActionSet>::ALL {
        m.handle(a);
    }
    assert_eq!(m.handle(Confirm), None);
    assert_eq!(glyph::size(&m.view()), (4, 2));
}

#[test]
fn a_browsing_menu_stops_on_disabled_items_but_never_chooses_them() {
    let mut m = menu(&[true, false, true]).browsing_disabled();
    assert_eq!(m.focus(), 0);
    m.handle(CursorDown);
    assert_eq!(m.focus(), 1, "stops on the disabled item");
    assert_eq!(m.handle(Confirm), None);
    let mut audio = AudioQueue::<()>::new(CUES);
    m.handle_with_sound(Confirm, &mut audio);
    assert_eq!(audio.take()[0].cue(), Some("buzz"), "the denied sound");
    m.handle(CursorDown);
    assert_eq!(m.focus(), 2);
    m.handle(CursorUp);
    assert_eq!(m.focus(), 1);
    // Focusing it by index works too; a plain menu refuses it.
    assert_eq!(
        menu(&[true, false]).browsing_disabled().focused(1).focus(),
        1
    );
    assert_eq!(menu(&[true, false]).focused(1).focus(), 0);
    // It still opens on the first enabled item.
    assert_eq!(menu(&[false, true]).browsing_disabled().focus(), 1);
}

#[test]
fn a_note_goes_to_the_view() {
    let m = Menu::new(vec![
        MenuItem::new("a").with_note("does a"),
        MenuItem::new("b"),
    ]);
    let view = m.view();
    assert_eq!(view.items[0].note.as_deref(), Some("does a"));
    assert_eq!(view.items[1].note, None);
}

#[test]
fn events() {
    let mut m = menu(&[true, true]);
    assert_eq!(m.handle(Confirm), Some(MenuEvent::Chosen(0)));
    assert_eq!(m.handle(CursorDown), None);
    assert_eq!(m.handle(Confirm), Some(MenuEvent::Chosen(1)));
    assert_eq!(m.handle(Cancel), Some(MenuEvent::Cancelled));
    assert_eq!(m.handle(CursorLeft), None);
    assert_eq!(m.handle(Info), None);
    assert_eq!(m.focus(), 1, "other actions don't move the focus");
}

#[test]
fn item_constructors() {
    assert_eq!(
        MenuItem::new("a"),
        MenuItem {
            label: "a".into(),
            enabled: true,
            suffix: None,
            note: None,
        }
    );
    assert!(!MenuItem::disabled("b").enabled);
    let m = Menu::new(vec![MenuItem::new("a"), MenuItem::disabled("bcd")]);
    assert_eq!(m.items()[1].label, "bcd");
    assert_eq!(glyph::size(&m.view()), (7, 4));
}

#[test]
fn the_view_lists_the_items_and_the_focus() {
    let mut m = Menu::new(vec![
        MenuItem::new("Move"),
        MenuItem::disabled("Attack").with_suffix(" (no foe)", SuffixKind::Warning),
        MenuItem::new("Wait"),
    ]);
    let view = m.view();
    let shown: Vec<_> = view
        .items
        .iter()
        .map(|i| (i.label.as_str(), i.enabled, i.focused))
        .collect();
    assert_eq!(
        shown,
        [
            ("Move", true, true),
            ("Attack", false, false),
            ("Wait", true, false)
        ]
    );
    assert_eq!(
        view.items[1].suffix,
        Some((" (no foe)".to_owned(), SuffixKind::Warning))
    );
    assert_eq!(view.items[0].suffix, None);
    assert_eq!(view.focused().map(|i| i.label.as_str()), Some("Move"));
    // The focus follows the menu, and an empty menu has none.
    m.handle(CursorDown);
    assert_eq!(m.view().focused().map(|i| i.label.as_str()), Some("Wait"));
    assert_eq!(Menu::new(vec![]).view().focused(), None);
}

proptest! {
    #[test]
    fn focus_always_lands_on_an_enabled_item(
        enabled in prop::collection::vec(any::<bool>(), 1..8),
        moves in prop::collection::vec(any::<bool>(), 0..20),
    ) {
        let mut m = menu(&enabled);
        for down in moves {
            m.handle(if down { CursorDown } else { CursorUp });
            prop_assert!(m.focus() < enabled.len());
            if enabled.iter().any(|&e| e) {
                prop_assert!(enabled[m.focus()]);
                prop_assert_eq!(m.handle(Confirm), Some(MenuEvent::Chosen(m.focus())));
            }
        }
    }

    #[test]
    fn down_then_up_returns(enabled in prop::collection::vec(any::<bool>(), 1..8)) {
        let mut m = menu(&enabled);
        let start = m.focus();
        m.handle(CursorDown);
        m.handle(CursorUp);
        prop_assert_eq!(m.focus(), start);
    }
}
