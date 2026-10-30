//! Pictures of the stage as `cargo shuyi frame-png` paints them, with the
//! game's font and tileset, checked as snapshots: your piece picked up and
//! its path steered, in each look. Review a change with `cargo insta
//! review`; the `.png` beside each `.snap` is the picture.

use hello_grid::MapLook;
use hello_grid::tools::{HelloTools, Start};
use shuyi_cli::GameTools;
use shuyi_cli::codec::encode_png;
use shuyi_cli::frame_png::{Script, Step};
use shuyi_cli::{GameArg, frame_png};

/// The stage in `look`, your piece picked up and steered up and right.
fn picture(look: MapLook) -> Vec<u8> {
    let mut h = Start {
        stage: true,
        look,
        seed: 0,
    }
    .harness();
    h.keys("Enter Up Right Right");
    let image = hello_grid::tools::picture(&h, 1).unwrap_or_else(|e| panic!("{e}"));
    encode_png(image.width, image.height, &image.rgba).unwrap_or_else(|e| panic!("{e}"))
}

#[test]
fn the_stage_in_glyphs() {
    insta::assert_binary_snapshot!("stage_glyphs.png", picture(MapLook::Glyphs));
}

#[test]
fn the_stage_in_tiles() {
    insta::assert_binary_snapshot!("stage_tiles.png", picture(MapLook::Tiles));
}

#[test]
fn the_title_and_the_end_as_text() {
    let h = crate::title(0);
    insta::assert_snapshot!("title", h.snapshot());
    assert_eq!(crate::title_on("desktop", 0).snapshot(), h.snapshot());
    // On the web, where the tab closes the game, the menu has no Quit.
    insta::assert_snapshot!("title_web", crate::title_on("web", 0).snapshot());
    let mut h = crate::title(7);
    h.keys("Enter Backspace");
    crate::play_out(&mut h, crate::to_the_goal);
    insta::assert_snapshot!("end", h.snapshot());
}

#[test]
fn frame_png_paints_what_the_harness_shows() {
    let script = Script::new(vec![Step::Keys("Enter Up Right Right".to_owned())]);
    let options = [GameArg {
        flag: "--stage".to_owned(),
        value: None,
    }];
    let shot = HelloTools.frame(&options, &script, None).unwrap();
    assert_eq!(shot.shows, "stage");
    let png = encode_png(shot.image.width, shot.image.height, &shot.image.rgba).unwrap();
    assert_eq!(png, picture(MapLook::Glyphs));
    // Twice the size with a scale of 2; the game's options are checked.
    let big = HelloTools.frame(&options, &script, Some(2)).unwrap();
    assert_eq!((big.image.width, big.image.height), (768, 768));
    let seed = |value: &str| GameArg {
        flag: "--seed".to_owned(),
        value: Some(value.to_owned()),
    };
    assert!(HelloTools.frame(&[seed("x")], &script, None).is_err());
    assert!(
        HelloTools
            .frame(&[seed("3")], &Script::default(), None)
            .is_ok()
    );
    let unknown = GameArg {
        flag: "--light".to_owned(),
        value: None,
    };
    assert!(HelloTools.frame(&[unknown], &script, None).is_err());
    assert_eq!(HelloTools.frame_options().len(), 3);
    let usage = frame_png::usage("cargo shuyi", HelloTools.frame_options(), "");
    assert!(usage.contains("--tiles"), "{usage}");
    let lang = HelloTools.lang().unwrap();
    assert_eq!(lang.lang.english("title.name"), Some("Hello, Grid"));
}
