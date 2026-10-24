//! The about screen through the engine's Harness: tests of what it shows
//! read its view, not cells. Key names here are the Harness's test-script
//! names, input to a test, not game code.

use game::about::About;

#[test]
fn confirm_on_the_title_opens_the_about_screen_and_cancel_closes_it() {
    let mut h = game::game().harness();
    h.keys("Enter");
    assert_eq!(h.screens(), ["title", "about"]);
    assert!(h.screen::<About>().is_some());
    let view = About::view(h.ctx());
    assert!(h.snapshot().contains(&view.heading), "{}", h.snapshot());
    assert!(h.snapshot().contains(&view.line), "{}", h.snapshot());
    assert!(!view.help.is_empty(), "{view:?}");
    h.keys("Backspace");
    assert_eq!(h.screens(), ["title"]);
}
