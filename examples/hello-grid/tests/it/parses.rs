//! No frame parses a text (ticket 2266): the game's language learns each
//! text's form when it loads, and every screen fills its text from it.
//! The count is the thread's, so tests beside it don't count here.

use shuyi_assets::message::parse_count;

use crate::title;

#[test]
fn no_frame_of_the_games_screens_parses_a_text() {
    let mut h = title(0);
    for n in 2..=8 {
        let added = h.ctx_mut().add_profile(&format!("P{n}"));
        assert!(added.is_ok(), "{added:?}");
    }
    parse_count::take();
    // The profiles screen, its add row refused with a plural.
    h.keys("Down Down Enter Up Enter");
    assert!(h.snapshot().contains("8 profiles"), "{}", h.snapshot());
    h.keys("Backspace Backspace");
    assert_eq!(h.screens(), ["title"]);
    // The options, a row changed, and its question on leaving.
    h.keys("Up Enter Down Left");
    assert_eq!(h.screens(), ["title", "options"]);
    h.keys("Backspace Left Left Enter");
    assert_eq!(h.screens(), ["title"]);
    // The stage, its talk read in the text box, its cursor moved.
    h.keys("Up Enter");
    assert_eq!(h.screens(), ["title", "stage", "talk"]);
    h.keys("Enter Enter Enter Enter Enter Enter Right Right Down");
    assert_eq!(parse_count::take(), 0, "a frame parsed a text");
}
