//! The engine's profiles screen, opened from the title: a profile added
//! by typing its name is the one in use, with options of its own.

use hello_grid::Hello;
use shuyi_platform::harness::Harness;
use shuyi_ui::profiles::ProfileId;
use shuyi_ui::screens::profiles::{ProfilesScreen, RowKind};
use shuyi_ui::settings::TextSpeed;

use crate::title;

fn profiles(h: &Harness<Hello>) -> &ProfilesScreen<Hello> {
    h.screen::<ProfilesScreen<Hello>>()
        .unwrap_or_else(|| panic!("no profiles: {:?}", h.screens()))
}

fn names(h: &Harness<Hello>) -> Vec<String> {
    let all = h.ctx().profiles().all().iter();
    all.map(|p| p.name.clone()).collect()
}

#[test]
fn the_title_opens_the_profiles_screen_on_the_one_in_use() {
    let mut h = title(0);
    h.keys("Down Down Enter");
    assert_eq!(h.screens(), ["title", "profiles"]);
    let screen = profiles(&h);
    assert_eq!(screen.focused(h.ctx()), RowKind::Profile(ProfileId::FIRST));
    let snapshot = h.snapshot();
    for text in ["Profiles", "Player 1", "in use", "Add a profile"] {
        assert!(snapshot.contains(text), "{text}: {snapshot}");
    }
    h.keys("Backspace");
    assert_eq!(h.screens(), ["title"]);
}

#[test]
fn a_profile_added_by_typing_is_in_use_with_its_own_options() {
    let mut h = title(0);
    // The row that adds one, then type a name.
    h.keys("Down Down Enter Down Enter");
    assert!(h.snapshot().contains("Profile name"), "{}", h.snapshot());
    h.type_text("Kai");
    assert_eq!(profiles(&h).typed(), Some("Kai"));
    h.keys("Enter");
    assert_eq!(h.screens(), ["title"]);
    assert_eq!(names(&h), ["Player 1", "Kai"]);
    let kai = h.ctx().profiles().current();
    assert_ne!(kai, ProfileId::FIRST);
    // Kai's text speed, kept on leaving the options, is Kai's alone.
    h.keys("Up Enter Right Backspace Left Left Enter");
    assert_eq!(h.ctx().settings().text_speed, TextSpeed::Fast);
    assert_eq!(h.screens(), ["title"]);
    h.keys("Down Enter Up Enter Enter");
    assert_eq!(h.screens(), ["title"]);
    assert_eq!(h.ctx().profiles().current(), ProfileId::FIRST);
    assert_eq!(h.ctx().settings().text_speed, TextSpeed::Normal);
}
