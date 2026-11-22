//! The menu and the pointer (ticket 2405): hover, a still pointer, clicks
//! on enabled and disabled rows, two menus told apart by their keys, and
//! the glyph look's hit areas.

use shuyi_assets::keymap::sample::Action;

use super::*;
use crate::audio::AudioConfig;
use crate::frame::glyph_buffer::{Cell, GlyphBuffer, Rect};
use crate::frame::hit::{Hit, WidgetKey};
use crate::frame::{EngineColor, Rgb};
use crate::input::pointer::PointerInput;
use crate::toy;

/// A menu of items standing for their indexes, enabled as `enabled` says.
fn menu(enabled: &[bool]) -> Menu<usize> {
    Menu::new(
        enabled
            .iter()
            .enumerate()
            .map(|(i, &on)| {
                MenuItem::new(format!("item {i}"))
                    .with_value(i)
                    .with_enabled(on)
            })
            .collect(),
    )
}

/// Row `item` of the menu keyed `key`.
fn row(key: u16, item: u32) -> Hit {
    Hit::new(Rect::new(0, 0, 8, 16), WidgetKey(key), item)
}

/// A pointer that moved over `hit`.
fn hover(hit: Hit) -> PointerInput {
    let mut p = PointerInput::at(1, 1);
    p.moved = true;
    p.over = Some(hit);
    p
}

/// A pointer still over `hit`.
fn still(hit: Hit) -> PointerInput {
    PointerInput {
        moved: false,
        ..hover(hit)
    }
}

/// A click on `hit`.
fn click(hit: Hit) -> PointerInput {
    let mut p = still(hit);
    p.released = true;
    p.clicked = Some(hit);
    p
}

/// The sound cues the pointers play on `m`, in order.
fn sounds(m: &mut Menu<usize>, pointers: &[PointerInput]) -> Vec<String> {
    const CUES: AudioConfig = AudioConfig {
        move_cue: "tick",
        select_cue: "ding",
        cancel_cue: "dong",
        denied_cue: "buzz",
        cursor_cue: "step",
    };
    let mut audio = AudioQueue::<()>::new(CUES);
    for p in pointers {
        m.point_with_sound(Some(p), &mut audio);
    }
    audio
        .take()
        .iter()
        .filter_map(|r| r.cue().map(str::to_owned))
        .collect()
}

#[test]
fn hover_moves_the_focus_on_a_frame_the_pointer_moved() {
    let mut m = menu(&[true, true, true, true]);
    assert_eq!(m.point(Some(&hover(row(0, 2)))), None);
    assert_eq!(m.focus(), 2);
    // With the pointer still over row 2, a key moves the focus on, and
    // it stays there.
    m.handle(Action::CursorDown);
    assert_eq!(m.point(Some(&still(row(0, 2)))), None);
    assert_eq!(m.focus(), 3);
    // Over the box but no row, or no pointer: nothing.
    m.point(Some(&hover(row(0, Hit::AREA))));
    m.point(None);
    assert_eq!(m.focus(), 3);
    // A row past the items is no row of this menu.
    m.point(Some(&hover(row(0, 9))));
    assert_eq!(m.focus(), 3);
}

#[test]
fn hover_skips_a_row_the_focus_may_not_stop_on() {
    let mut m = menu(&[true, false, true]);
    m.point(Some(&hover(row(0, 1))));
    assert_eq!(m.focus(), 0);
    let mut browsing = menu(&[true, false, true]).browsing_disabled();
    browsing.point(Some(&hover(row(0, 1))));
    assert_eq!(browsing.focus(), 1);
}

#[test]
fn a_click_chooses_an_enabled_row_and_not_a_disabled_one() {
    let mut m = menu(&[true, false, true]);
    assert_eq!(m.point(Some(&click(row(0, 2)))), Some(MenuEvent::Chosen(2)));
    assert_eq!(m.focus(), 2, "the focus went there first");
    assert_eq!(m.point(Some(&click(row(0, 1)))), None);
    assert_eq!(m.focus(), 2, "a disabled row takes no focus");
    // A click on the box but no row chooses nothing.
    assert_eq!(m.point(Some(&click(row(0, Hit::AREA)))), None);
}

#[test]
fn pointing_plays_the_menus_sounds() {
    let mut m = menu(&[true, false, true]);
    assert_eq!(sounds(&mut m, &[hover(row(0, 2))]), ["tick"]);
    assert_eq!(sounds(&mut m, &[hover(row(0, 2))]), Vec::<String>::new());
    assert_eq!(sounds(&mut m, &[click(row(0, 2))]), ["ding"]);
    assert_eq!(sounds(&mut m, &[click(row(0, 1))]), ["buzz"]);
    // A click on another menu's row plays nothing here.
    assert_eq!(sounds(&mut m, &[click(row(1, 0))]), Vec::<String>::new());
}

#[test]
fn two_menus_are_told_apart_by_their_keys() {
    let mut left = menu(&[true, true]);
    let mut right = menu(&[true, true]).keyed(WidgetKey(1));
    assert_eq!(
        (left.widget(), right.widget()),
        (WidgetKey(0), WidgetKey(1))
    );
    assert_eq!(right.view().widget, WidgetKey(1));
    let on_right = click(row(1, 1));
    assert_eq!(left.point(Some(&on_right)), None);
    assert_eq!(left.focus(), 0);
    assert_eq!(right.point(Some(&on_right)), Some(MenuEvent::Chosen(1)));
}

#[test]
fn the_glyph_look_records_its_box_and_each_row_shown() {
    let c = toy::ctx();
    let m = menu(&[true, true, true, true])
        .showing(2, crate::widgets::list::Scroll::KeepInView)
        .focused(3)
        .keyed(WidgetKey(5));
    let view = m.view();
    let ink = c.palette.engine(EngineColor::Text);
    let mut buf = GlyphBuffer::new(20, 8, Cell::new(' ', ink, Rgb::new(0, 0, 0)));
    glyph::paint(&c, &view, &mut buf, 1, 1);
    let (w, h) = glyph::size(&c, &view);
    let hits = buf.hits();
    assert_eq!(
        hits[0],
        Hit::new(Rect::new(8, 16, w * 8, h * 16), WidgetKey(5), Hit::AREA)
    );
    // The rows shown are items 2 and 3, a cell in from each side.
    let rows: Vec<_> = hits[1..].iter().map(|h| (h.rect, h.item)).collect();
    assert_eq!(
        rows,
        [
            (Rect::new(16, 32, (w - 2) * 8, 16), 2),
            (Rect::new(16, 48, (w - 2) * 8, 16), 3),
        ]
    );
    // A pixel of a row gives its item; the border gives the box.
    assert_eq!(buf.hit_at(20, 50).and_then(|h| h.item()), Some(3));
    assert_eq!(buf.hit_at(9, 50).and_then(|h| h.item()), None);
}
