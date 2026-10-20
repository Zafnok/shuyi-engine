//! The game suspended and resumed, as a device that sleeps does it, through
//! the Harness on the engine's headless backend: while it sleeps nothing
//! steps, nothing is shown and keys do nothing; once resumed it goes on
//! where it was.

use hello_grid::screens::Talk;
use shuyi_platform::SystemEvent;

use crate::title;

#[test]
fn a_suspended_game_stands_still_and_goes_on_where_it_was() {
    let mut h = title(1);
    h.keys("Enter");
    // Half the first line typed out.
    h.wait(0.2);
    let typed = |h: &shuyi_platform::harness::Harness<hello_grid::Hello>| {
        let talk = h.screen::<Talk>().unwrap_or_else(|| panic!("no talk"));
        talk.text.is_revealed()
    };
    assert!(!typed(&h));
    let before = h.snapshot();
    let shown = h.platform().presented();
    h.system(SystemEvent::Suspend);
    assert!(h.app().is_suspended());
    // Keys and minutes of frames while it sleeps change nothing.
    h.keys("Enter Enter Down").wait(20.0);
    assert_eq!(h.snapshot(), before);
    assert_eq!(h.screens(), ["title", "stage", "talk"]);
    assert!(!typed(&h));
    assert_eq!(
        h.platform().presented(),
        shown,
        "nothing shown while asleep"
    );
    h.system(SystemEvent::Resume);
    assert!(!h.app().is_suspended());
    // It goes on typing from where it was, then takes Confirm.
    h.wait(5.0);
    assert!(typed(&h));
    h.keys("Enter");
    let talk = h.screen::<Talk>().unwrap_or_else(|| panic!("no talk"));
    assert!(!talk.text.is_revealed(), "on to the next line");
}
