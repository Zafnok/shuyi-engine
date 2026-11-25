//! The game's tools beyond the contract's operations: its `--seed`, its
//! stamp, the rules' events of a session, and a check that finds what its
//! files lack.

use shuyi::assets::{ContentError, Layer};
use shuyi::prelude::{BasicGame, UiAction};
use shuyi_cli::{GameArg, GameTools};
use snake::tools::{Tools, events, lines, seed_of};

use crate::{at_title, tools};

/// The option `flag` with `value`.
fn arg(flag: &str, value: Option<&str>) -> GameArg {
    GameArg {
        flag: flag.to_owned(),
        value: value.map(str::to_owned),
    }
}

#[test]
fn the_seed_is_1_unless_said_and_a_wrong_option_is_named() {
    assert_eq!(seed_of(&[]), Ok(1));
    assert_eq!(seed_of(&[arg("--seed", Some("5"))]), Ok(5));
    let wrong = seed_of(&[arg("--seed", Some("five"))]).unwrap_err();
    assert_eq!(wrong, "--seed five: not a number");
    let unknown = seed_of(&[arg("--stage", None)]).unwrap_err();
    assert_eq!(unknown, "unknown option --stage");
}

#[test]
fn its_one_option_is_the_seed() {
    let flags: Vec<&str> = tools().frame_options().iter().map(|o| o.flag).collect();
    assert_eq!(flags, ["--seed"]);
}

#[test]
fn the_tools_stamp_is_the_builds() {
    assert_eq!(tools().made(), Some(snake::made()));
}

#[test]
fn its_one_bit_look_is_the_palette_and_its_pattern_map() {
    let skin = tools()
        .one_bit("playdate")
        .unwrap_or_else(|| panic!("no one-bit look"))
        .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(skin.palette_file, "files/palette.ron");
    assert!(
        skin.map_file.ends_with("palette@onebit.ron"),
        "{}",
        skin.map_file
    );
}

#[test]
fn a_sessions_events_are_the_rules_events_in_order() {
    let mut h = at_title(1);
    let none = events(&h);
    assert!(none.is_empty(), "{none:?}");
    h.act(UiAction::Confirm);
    h.wait(0.3);
    let said = events(&h);
    assert_eq!(said.len(), 2, "{said:?}");
    assert_eq!(said[0], "Moved { head: Pos { x: 7, y: 7 } }");
    assert_eq!(said[1], "Moved { head: Pos { x: 8, y: 7 } }");
}

#[test]
fn check_finds_each_file_a_game_lacks_one_a_line() {
    // check-text: not player text, a folder
    let empty = Layer::table("files", &[]);
    let broken = BasicGame::with_own(snake::app_info::APP, empty, snake::own, |_| {
        Box::new(snake::screens::Title)
    });
    let errors = Tools::new(broken).check().unwrap_or_else(|e| panic!("{e}"));
    let files: Vec<&str> = errors.iter().map(|e| e.file.as_str()).collect();
    for file in ["files/board.map", "files/tuning.ron", "files/keymap.ron"] {
        assert!(files.contains(&file), "{files:?}");
    }
    let shown = lines(&[
        ContentError::new("a.ron", "one"),
        ContentError::new("b.ron", "two"),
    ]);
    assert_eq!(shown, "a.ron: one\nb.ron: two");
}
