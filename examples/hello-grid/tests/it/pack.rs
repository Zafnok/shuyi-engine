//! Hello, Grid baked into a pack (ticket 2204): `cargo shuyi bake` makes
//! one, the game loads from it what it loads from its files, and `cargo
//! shuyi check --baked` proves the two equal.

use std::fs;

use hello_grid::tools::HelloTools;
use hello_grid::{Hello, PACK_STAMP};
use shuyi::assets::bundle::{Bundle, Layer};
use shuyi::assets::pack::{Pack, PackError};
use shuyi_cli::contract::bake::{BakeInput, bake};
use shuyi_cli::contract::check::{CheckInput, check};
use shuyi_cli::project::Config;
use shuyi_cli::{GameTools, Project};

/// The game's pack, as `cargo shuyi bake` makes it, kept for the run as
/// a game that embeds it keeps it.
fn baked() -> &'static [u8] {
    let game = HelloTools.bake(None).unwrap_or_else(|e| panic!("{e}"));
    let baked = game
        .bake
        .bake(&game.formats, &game.bundle, None)
        .unwrap_or_else(|e| panic!("{e:?}"));
    baked.bytes.leak()
}

#[test]
fn the_game_loads_from_its_pack_what_it_loads_from_its_files() {
    let bytes = baked();
    let from_files = Hello::load().unwrap_or_else(|e| panic!("{e:?}"));
    let from_pack = Hello::from_pack(bytes).unwrap_or_else(|e| panic!("{e:?}"));
    assert_eq!(format!("{from_pack:?}"), format!("{from_files:?}"));
    // Its text went in baked, its pictures as they are.
    let pack = Pack::open(bytes, PACK_STAMP).unwrap();
    let kinds = |path: &str| {
        let file = pack.files().iter().find(|f| f.path == path).unwrap();
        file.kinds.clone()
    };
    assert_eq!(kinds("palette.ron"), ["palette"]);
    assert_eq!(kinds("stage.map"), ["map"]);
    let found = kinds("tiles.png");
    assert!(found.is_empty(), "{found:?}");
    assert_eq!(pack.get("tiles.png"), hello_grid::file("tiles.png"));
}

/// A mod's files, laid over the game's.
static MOD: &[(&str, &[u8])] = &[("stage.map", b"a mod's map"), ("new.txt", b"new")];

#[test]
fn the_pack_has_the_fingerprint_of_the_files_it_was_baked_from() {
    let loose = Bundle::new(vec![hello_grid::layer()]);
    let pack = Pack::open(baked(), PACK_STAMP).unwrap();
    assert_eq!(pack.info().content, loose.fingerprint());
    let layer = pack.into_layer("pack");
    let packed = Bundle::new(vec![layer]);
    assert_eq!(packed.fingerprint(), loose.fingerprint());
    assert_eq!(hello_grid::made().content, packed.fingerprint());
    // A loose layer over the pack counts as it does over the files.
    let over = |under: Layer| Bundle::new(vec![Layer::table("mod", MOD), under]).fingerprint();
    assert_eq!(over(layer), over(hello_grid::layer()));
    assert_ne!(over(layer), loose.fingerprint());
}

#[test]
fn a_pack_of_another_game_or_version_is_refused() {
    let mut bytes = baked().to_vec();
    let error = Hello::from_pack(b"not a pack").unwrap_err();
    assert_eq!(error[0].file, "pack");
    assert_eq!(error[0].message, PackError::NotAPack.to_string());
    bytes[8] ^= 1;
    let error = Hello::from_pack(bytes.leak()).unwrap_err();
    assert!(error[0].message.contains("bake it again"), "{error:?}");
}

#[test]
fn check_baked_passes_on_the_game() {
    let mut input = CheckInput::default();
    input.baked = true;
    let answer = check(&HelloTools, &input);
    assert!(answer.ok, "{answer:?}");
}

#[test]
fn bake_writes_the_games_pack() {
    let dir = std::env::temp_dir().join(format!("hello-grid-bake-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    let project = Project::new(dir.clone(), Config::default());
    let answer = bake(&HelloTools, &project, &BakeInput::default());
    assert!(answer.ok, "{answer:?}");
    let made = answer.result.unwrap().pack.unwrap();
    assert_eq!(made.files, 11);
    assert_eq!(made.baked, 6);
    let bytes = fs::read(dir.join(&made.file)).unwrap();
    assert!(Hello::from_pack(bytes.leak()).is_ok());
    fs::remove_dir_all(&dir).unwrap();
}
