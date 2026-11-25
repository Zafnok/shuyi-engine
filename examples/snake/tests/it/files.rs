//! The game's files: the preview is what the engine's `frame-png` makes of
//! its `shuyi.ron` part, so an edit that forgets to make it again fails
//! here, and its web build is its binary.

use shuyi_cli::Project;
use shuyi_cli::frame_png;

use crate::{root, tools};

fn project() -> Project {
    Project::load(root()).unwrap_or_else(|e| panic!("{e}"))
}

#[test]
fn the_preview_is_what_frame_png_makes_of_its_part() {
    frame_png::preview::check(&project(), &tools()).unwrap_or_else(|e| panic!("{e}"));
}

#[test]
fn the_web_build_is_the_games_binary() {
    let web = project().config.web.unwrap_or_else(|| panic!("no web part"));
    assert_eq!((web.package.as_str(), web.bin.as_str()), ("snake", "snake"));
    assert_eq!(web.dist, "dist/web");
}
