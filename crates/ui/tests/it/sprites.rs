//! Pictures through the Harness (ticket 1111, game ADR-0038): the toy's
//! `Gallery`, opened with the Debug key, puts sprite items in the frame,
//! and a snapshot lists each as one line.

use shuyi_platform::harness::Harness;
use shuyi_ui::toy::{self, CARD, Toy};

use crate::toy as home;

fn gallery() -> Harness<Toy> {
    let mut h = home();
    h.keys("F2");
    assert_eq!(h.screens(), ["home", "gallery"]);
    h
}

#[test]
fn the_debug_key_opens_the_gallery_once_and_cancel_closes_it() {
    let mut h = gallery();
    let sprites = h.game().buffer().sprites();
    assert_eq!(sprites.len(), 2);
    assert!(sprites.iter().all(|s| s.image.path() == CARD));
    // The Debug key does nothing over a debug tool.
    h.keys("F2");
    assert_eq!(h.screens(), ["home", "gallery"]);
    h.keys("d");
    assert_eq!(h.screens(), ["home"]);
    // The home screen is drawn without the gallery's pictures.
    let found = h.game().buffer().sprites();
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn without_debug_tools_the_debug_key_does_nothing() {
    let mut h = Harness::new(toy::game().with_debug_screens(false));
    h.keys("F2");
    assert_eq!(h.screens(), ["home"]);
}

#[test]
fn every_sprite_is_one_snapshot_line() {
    let h = gallery();
    let snapshot = h.snapshot();
    let lines: Vec<&str> = snapshot
        .lines()
        .skip_while(|&l| l != "--- overlays ---")
        .skip(1)
        .collect();
    assert_eq!(lines.len(), h.game().buffer().items().len());
    assert_eq!(lines.len(), 2, "{snapshot}");
    assert!(lines.iter().all(|l| l.contains(" sprite ")), "{lines:?}");
    // The card half off the right edge is listed by what is left of it.
    assert!(!lines[0].contains(" clip="), "{}", lines[0]);
    assert!(lines[1].ends_with(" clip=304,0 16x32"), "{}", lines[1]);
}
