use super::*;
use crate::error::Code;
use crate::format::Format;
use crate::format::builtin_tests::{
    TERRAIN_TILES as FARM, TILE_LOOKS as LOOKS, TILE_RULES as RULES,
    terrain_tiles_images as images, tile_terrains as terrains,
};
use crate::image::ImageId;
use proptest::prelude::*;

fn read(source: &str) -> TerrainTilesDef {
    parse_ron("farm.ron", source).unwrap_or_else(|e| panic!("{e}"))
}

fn problems(source: &str) -> Vec<String> {
    read(source).problems(&RULES)
}

/// A file of the farm's own tiles with `layers` and `looks`, its tiles
/// `tile_px`.
fn file(tile_px: &str, layers: &str, looks: &str) -> String {
    format!(
        "(image: \"tiles/farm.png\", tile_px: {tile_px}, terrain: {{\"soil\": (0, 0), \
         \"tilled\": (1, 0), \"water\": (2, 0)}}, layers: [{layers}], looks: {{{looks}}})"
    )
}

/// The problems of the farm's own tiles with `layers`.
fn layer_problems(layers: &str) -> Vec<String> {
    problems(&file("(16, 16)", layers, ""))
}

/// What `source` resolves to, or its errors as shown.
fn resolved(source: &str) -> Result<TerrainTiles, Vec<String>> {
    read(source)
        .resolve("farm.ron", &images(), &terrains())
        .map_err(|errors| errors.iter().map(ToString::to_string).collect())
}

/// The errors of `source` resolved, as shown.
fn errors(source: &str) -> Vec<String> {
    resolved(source).err().unwrap_or_default()
}

/// The codes of `source`'s errors when resolved.
fn codes_of(source: &str) -> Vec<Option<Code>> {
    let errors = read(source)
        .resolve("farm.ron", &images(), &terrains())
        .err()
        .unwrap_or_default();
    errors.iter().map(|e| e.code).collect()
}

fn image() -> ImageId {
    images().id("tiles/farm.png").unwrap()
}

/// Cell `(column, row)` of the farm's 16 px tiles.
fn cell(at: (u32, u32)) -> ImageRect {
    ImageRect::cell(at, (16, 16), (0, 0))
}

fn ids<const N: usize>(ids: [u16; N]) -> BTreeSet<TerrainId> {
    ids.into_iter().map(TerrainId).collect()
}

/// Pictures by mix.
fn mixes<const N: usize>(pictures: [(u8, ImageRect); N]) -> [Option<ImageRect>; MIXES] {
    let mut tiles = [None; MIXES];
    for (m, rect) in pictures {
        tiles[usize::from(m)] = Some(rect);
    }
    tiles
}

#[test]
fn the_farm_resolves_to_the_tiles_built_by_hand() {
    assert_eq!(problems(FARM), Vec::<String>::new());
    let own = Look::new(
        [
            (TerrainId(0), cell((0, 0))),
            (TerrainId(1), cell((1, 0))),
            (TerrainId(2), cell((2, 0))),
        ]
        .into(),
        vec![
            Layer::Corners(CornerLayer::new(
                ids([2]),
                mixes([(1, cell((0, 1))), (3, cell((1, 1)))]),
            )),
            Layer::Tiles(TileLayer {
                of: ids([1]),
                tile: cell((3, 0)),
                beside: ids([2]),
                sides: mixes([(5, cell((3, 1)))]),
            }),
        ],
    );
    let winter = Look::new(
        [
            (TerrainId(0), cell((0, 2))),
            (TerrainId(1), cell((1, 2))),
            (TerrainId(2), cell((2, 2))),
        ]
        .into(),
        vec![Layer::Corners(CornerLayer::new(
            ids([1, 2]),
            mixes([(8, cell((3, 2)))]),
        ))],
    );
    let mut own = own;
    own.props = [
        (
            "lamp".to_owned(),
            ImageRect {
                x: 48,
                y: 16,
                w: 16,
                h: 32,
            },
        ),
        ("sign".to_owned(), cell((0, 2))),
    ]
    .into();
    // The own look's pictures are not winter's: it places none.
    let mut tiles = TerrainTiles::new(image(), (16, 16), own);
    tiles.looks = [("winter".to_owned(), winter)].into();
    assert_eq!(resolved(FARM), Ok(tiles));
}

#[test]
fn a_corner_layers_size_offset_and_skip_go_onto_it() {
    let source = file(
        "(16, 16)",
        r#"Corners(of: ["water"], tiles: [(corners: ".##.", at: (0, 1))],
                   size: (24, 32), offset: (1, -3), skip: ["soil", "tilled"]),
           Corners(of: ["water"], tiles: [(corners: ".##.", at: (1, 1))])"#,
        "",
    );
    assert_eq!(problems(&source), Vec::<String>::new());
    let tiles = resolved(&source).unwrap();
    let [Layer::Corners(sized), Layer::Corners(plain)] = tiles.look.layers.as_slice() else {
        panic!("{:?}", tiles.look.layers);
    };
    let mut want = CornerLayer::new(
        ids([2]),
        mixes([(
            6,
            ImageRect {
                x: 0,
                y: 16,
                w: 24,
                h: 32,
            },
        )]),
    );
    want.offset = (1, -3);
    want.skip = ids([0, 1]);
    assert_eq!(sized, &want);
    // Left out: a tile's size, no offset, nothing skipped.
    assert_eq!(
        plain,
        &CornerLayer::new(ids([2]), mixes([(6, cell((1, 1)))]))
    );
}

#[test]
fn a_side_of_the_tiles_outside_the_rules_is_a_problem() {
    for (tile_px, problem) in [
        (
            "(7, 16)",
            "tile_px is (7, 16); each side must be 8 to 64 px",
        ),
        (
            "(16, 65)",
            "tile_px is (16, 65); each side must be 8 to 64 px",
        ),
    ] {
        assert_eq!(problems(&file(tile_px, "", "")), [problem]);
    }
    for edge in ["(8, 8)", "(64, 64)", "(8, 64)"] {
        assert_eq!(problems(&file(edge, "", "")), Vec::<String>::new());
    }
}

#[test]
fn a_look_is_named_only_as_a_map_can_name_one_and_not_as_the_own_one() {
    let look = |name: &str| format!("\"{name}\": (terrain: {{}})");
    assert_eq!(
        problems(&file("(16, 16)", "", &look("cave"))),
        Vec::<String>::new()
    );
    assert_eq!(
        problems(&file("(16, 16)", "", &look("dungeon"))),
        ["look \"dungeon\" is not a look a map can name (\"outdoor\", \"winter\", \"cave\")"]
    );
    assert_eq!(
        problems(&file("(16, 16)", "", &look(DEFAULT_TILES))),
        ["look \"outdoor\" is the tileset's own `terrain` and `layers`: write it there"]
    );
}

#[test]
fn a_layer_must_name_a_terrain() {
    assert_eq!(
        layer_problems("Tiles(of: [], at: (3, 0))"),
        ["layer 1: `of` names no terrain"]
    );
    assert_eq!(
        layer_problems(
            r#"Tiles(of: ["water"], at: (3, 0)), Corners(of: [], tiles: [(corners: "...#", at: (0, 1))])"#
        ),
        ["layer 2: `of` names no terrain"]
    );
}

#[test]
fn a_mix_is_four_of_in_and_out() {
    assert_eq!(
        layer_problems(
            r#"Corners(of: ["water"], tiles: [(corners: "..#", at: (0, 1)), (corners: "...#", at: (0, 1))])"#
        ),
        ["layer 1: corners \"..#\" must be four of `#` (in) and `.` (out)"]
    );
    assert_eq!(
        layer_problems(
            r#"Tiles(of: ["tilled"], at: (3, 0), sides: [(sides: ".x.#", at: (3, 1))])"#
        ),
        ["layer 1: sides \".x.#\" must be four of `#` (in) and `.` (out)"]
    );
}

#[test]
fn a_mix_is_listed_once() {
    assert_eq!(
        layer_problems(
            r#"Corners(of: ["water"], tiles: [(corners: "...#", at: (0, 1)), (corners: "...#", at: (1, 1))])"#
        ),
        ["layer 1: corners \"...#\" is listed twice"]
    );
    assert_eq!(
        layer_problems(
            r#"Tiles(of: ["tilled"], at: (3, 0), sides: [(sides: ".#.#", at: (3, 1)), (sides: ".#.#", at: (3, 2))])"#
        ),
        ["layer 1: sides \".#.#\" is listed twice"]
    );
}

#[test]
fn a_corner_layer_has_no_picture_for_no_corner_in_it() {
    assert_eq!(
        layer_problems(
            r#"Corners(of: ["water"], tiles: [(corners: "....", at: (0, 1)), (corners: "...#", at: (1, 1))])"#
        ),
        ["layer 1: corners \"....\" has no corner in the layer, so there is nothing to draw"]
    );
}

#[test]
fn a_corner_layer_has_a_picture() {
    assert_eq!(
        layer_problems(r#"Corners(of: ["water"], tiles: [])"#),
        ["layer 1: it has no pictures"]
    );
}

#[test]
fn a_corner_layers_size_is_inside_the_rules() {
    assert_eq!(
        layer_problems(
            r#"Corners(of: ["water"], tiles: [(corners: "...#", at: (0, 1))], size: (16, 65))"#
        ),
        ["layer 1: size is (16, 65); each side must be 8 to 64 px"]
    );
    assert_eq!(
        layer_problems(
            r#"Corners(of: ["water"], tiles: [(corners: "...#", at: (0, 0))], size: (64, 8))"#
        ),
        Vec::<String>::new()
    );
}

#[test]
fn a_problem_of_another_look_names_it() {
    let looks = r#""winter": (terrain: {}, layers: [Tiles(of: [], at: (0, 0))])"#;
    assert_eq!(
        problems(&file("(16, 16)", "", looks)),
        ["look \"winter\": layer 1: `of` names no terrain"]
    );
}

#[test]
fn every_problem_of_the_file_is_reported() {
    let source = file(
        "(7, 7)",
        r#"Corners(of: [], tiles: [(corners: "....", at: (0, 0))], size: (99, 8))"#,
        r#""dungeon": (terrain: {})"#,
    );
    assert_eq!(
        problems(&source),
        [
            "tile_px is (7, 7); each side must be 8 to 64 px",
            "look \"dungeon\" is not a look a map can name (\"outdoor\", \"winter\", \"cave\")",
            "layer 1: `of` names no terrain",
            "layer 1: size is (99, 8); each side must be 8 to 64 px",
            "layer 1: corners \"....\" has no corner in the layer, so there is nothing to draw",
        ]
    );
}

#[test]
fn the_image_is_one_of_the_table() {
    let source = FARM.replace("tiles/farm.png", "tiles/gone.png");
    assert_eq!(
        errors(&source),
        ["farm.ron: image \"tiles/gone.png\" is not in the bundle's image table"]
    );
    assert_eq!(codes_of(&source), [Some(codes::MISSING_FILE)]);
}

#[test]
fn every_picture_lies_inside_the_image() {
    let outside = |what: &str, size: &str, at: &str| {
        format!("farm.ron: {what}: its {size} px at {at} lies outside the 64×48 px image")
    };
    let terrain = FARM.replace("\"soil\": (0, 0)", "\"soil\": (4, 0)");
    assert_eq!(
        errors(&terrain),
        [outside("terrain \"soil\"", "16×16", "(64, 0)")]
    );
    assert_eq!(codes_of(&terrain), [Some(codes::OUT_OF_RANGE)]);
    let winter = FARM.replace("\"soil\": (0, 2)", "\"soil\": (0, 3)");
    assert_eq!(
        errors(&winter),
        [outside(
            "look \"winter\": terrain \"soil\"",
            "16×16",
            "(0, 48)"
        )]
    );
    let layers = file(
        "(16, 16)",
        r#"Corners(of: ["water"], tiles: [(corners: "...#", at: (0, 1))], size: (16, 48)),
           Tiles(of: ["tilled"], at: (0, 3), sides: [(sides: ".#.#", at: (5, 0))])"#,
        "",
    );
    assert_eq!(
        errors(&layers),
        [
            outside("layer 1, corners \"...#\"", "16×48", "(0, 16)"),
            outside("layer 2", "16×16", "(0, 48)"),
            outside("layer 2, sides \".#.#\"", "16×16", "(80, 0)"),
        ]
    );
    // At its size, a corner layer's picture may reach the image's edge.
    let edge = file(
        "(16, 16)",
        r#"Corners(of: ["water"], tiles: [(corners: "...#", at: (0, 1))], size: (16, 32))"#,
        "",
    );
    assert!(resolved(&edge).is_ok());
}

#[test]
fn every_terrain_named_is_one_of_the_games() {
    let unknown = |what: &str, name: &str| {
        format!("farm.ron: {what}terrain \"{name}\" is not one of the game's terrains")
    };
    let terrain = FARM.replace("\"water\": (2, 2)", "\"water\": (2, 2), \"rock\": (3, 2)");
    assert_eq!(errors(&terrain), [unknown("look \"winter\": ", "rock")]);
    assert_eq!(codes_of(&terrain), [Some(codes::UNKNOWN_NAME)]);
    let own = FARM.replacen(
        "\"soil\": (0, 0),",
        "\"soil\": (0, 0), \"sand\": (3, 2),",
        1,
    );
    assert_eq!(errors(&own), [unknown("", "sand")]);
    let layers = file(
        "(16, 16)",
        r#"Corners(of: ["lava"], tiles: [(corners: "...#", at: (0, 1))], skip: ["wall"]),
           Tiles(of: ["tilled"], at: (3, 0), beside: ["sea"])"#,
        "",
    );
    assert_eq!(
        errors(&layers),
        [
            unknown("layer 1: ", "lava"),
            unknown("layer 1: skip: ", "wall"),
            unknown("layer 2: ", "sea"),
        ]
    );
}

#[test]
fn every_terrain_of_the_game_has_a_tile_in_each_look() {
    let own = FARM.replacen(", \"water\": (2, 0)", "", 1);
    assert_eq!(errors(&own), ["farm.ron: terrain \"water\" has no tile"]);
    let winter = FARM.replace(", \"water\": (2, 2)", "");
    assert_eq!(
        errors(&winter),
        ["farm.ron: look \"winter\": terrain \"water\" has no tile"]
    );
    assert_eq!(codes_of(&winter), [None]);
}

#[test]
fn a_mix_the_file_alone_refuses_is_left_out_when_resolving() {
    let source = file(
        "(16, 16)",
        r#"Corners(of: ["water"], tiles: [(corners: "bad", at: (9, 9)), (corners: "...#", at: (0, 1))])"#,
        "",
    );
    let tiles = resolved(&source).unwrap();
    assert_eq!(
        tiles.look.layers,
        [Layer::Corners(CornerLayer::new(
            ids([2]),
            mixes([(1, cell((0, 1)))])
        ))]
    );
}

#[test]
fn the_fields_left_out_are_their_defaults() {
    let bare = read(r#"(image: "a.png", tile_px: (8, 8), terrain: {})"#);
    assert_eq!(
        bare,
        TerrainTilesDef::new("a.png".to_owned(), (8, 8), BTreeMap::new())
    );
    let layer: LayerDef = parse_ron(
        "l.ron",
        r#"Corners(of: ["water"], tiles: [(corners: "...#", at: (0, 1))])"#,
    )
    .unwrap();
    assert_eq!(
        layer,
        LayerDef::Corners {
            of: vec!["water".to_owned()],
            tiles: vec![CornersAt {
                corners: "...#".to_owned(),
                at: (0, 1),
            }],
            size: None,
            offset: (0, 0),
            skip: Vec::new(),
            image: None,
        }
    );
    let tiles: LayerDef = parse_ron("l.ron", r#"Tiles(of: ["fort"], at: (0, 4))"#).unwrap();
    assert_eq!(
        tiles,
        LayerDef::Tiles {
            of: vec!["fort".to_owned()],
            at: (0, 4),
            beside: Vec::new(),
            sides: Vec::new(),
        }
    );
    // An unknown field is refused, in a layer too.
    for bad in [
        r#"(image: "a.png", tile_px: (8, 8), terrain: {}, colour: 3)"#,
        r#"(image: "a.png", tile_px: (8, 8), terrain: {}, layers: [Tiles(of: ["a"], at: (0, 0), skip: [])])"#,
    ] {
        assert!(parse_ron::<TerrainTilesDef>("t.ron", bad).is_err(), "{bad}");
    }
}

#[test]
fn a_corner_layers_own_image_holds_its_pictures() {
    let source = file(
        "(16, 16)",
        r#"Corners(of: ["water"], tiles: [(corners: "...#", at: (3, 1))],
                   image: "tiles/water-corners.png"),
           Corners(of: ["water"], tiles: [(corners: "...#", at: (3, 1))])"#,
        "",
    );
    assert_eq!(problems(&source), Vec::<String>::new());
    let tiles = resolved(&source).unwrap();
    let [Layer::Corners(own), Layer::Corners(shared)] = tiles.look.layers.as_slice() else {
        panic!("{:?}", tiles.look.layers);
    };
    let sheet = images().id("tiles/water-corners.png").unwrap();
    let pictures = mixes([(1, cell((3, 1)))]);
    assert_eq!(own, &CornerLayer::new(ids([2]), pictures).with_image(sheet));
    // Left out, the tileset's image.
    assert_eq!(shared, &CornerLayer::new(ids([2]), pictures));
    assert_eq!(tiles.image, image());
}

#[test]
fn a_corner_layers_own_image_is_in_the_table_and_holds_each_picture() {
    let gone = file(
        "(16, 16)",
        r#"Corners(of: ["water"], tiles: [(corners: "...#", at: (9, 9))], image: "tiles/gone.png")"#,
        "",
    );
    // Not in the table: that alone, no picture checked against it.
    assert_eq!(
        errors(&gone),
        ["farm.ron: layer 1: image \"tiles/gone.png\" is not in the bundle's image table"]
    );
    assert_eq!(codes_of(&gone), [Some(codes::MISSING_FILE)]);
    // The sheet is 64 × 32: row 2 lies outside it, though the farm's
    // 64 × 48 picture would hold it.
    let in_winter = |image: &str| {
        let layer = format!(
            r##", layers: [Corners(of: ["water"], tiles: [(corners: "#...", at: (0, 2))]{image})]"##
        );
        file("(16, 16)", "", &winter(&layer))
    };
    let outside = in_winter(r#", image: "tiles/water-corners.png""#);
    assert_eq!(
        errors(&outside),
        [
            "farm.ron: look \"winter\": layer 1, corners \"#...\": its 16×16 px at (0, 32) lies \
          outside the 64×32 px image"
        ]
    );
    assert_eq!(codes_of(&outside), [Some(codes::OUT_OF_RANGE)]);
    assert!(resolved(&in_winter("")).is_ok());
}

#[test]
fn a_file_written_back_reads_as_itself() {
    let sized = file(
        "(16, 16)",
        r#"Corners(of: ["water"], tiles: [(corners: ".##.", at: (0, 1))], size: (16, 32), offset: (1, -3), skip: ["soil"], image: "tiles/water-corners.png")"#,
        "",
    );
    for source in [FARM, &sized] {
        let def = read(source);
        let text = ::ron::to_string(&def).unwrap();
        assert_eq!(read(&text), def, "{text}");
    }
}

#[test]
fn the_constructors_give_the_literals() {
    assert_eq!(
        RULES,
        TileRules {
            looks: LOOKS,
            side_px: 8..=64,
        }
    );
    let terrain = BTreeMap::from([("soil".to_owned(), (0, 0))]);
    assert_eq!(
        LookDef::new(terrain.clone()),
        LookDef {
            terrain: terrain.clone(),
            layers: Vec::new(),
            props: BTreeMap::new(),
        }
    );
    assert_eq!(
        PropAt::new((4, 0)),
        PropAt {
            at: (4, 0),
            size: None,
        }
    );
    assert_eq!(
        PropAt::new((4, 0)).with_size((16, 32)),
        PropAt {
            at: (4, 0),
            size: Some((16, 32)),
        }
    );
    assert_eq!(
        TerrainTilesDef::new("a.png".to_owned(), (16, 24), terrain.clone()),
        TerrainTilesDef {
            image: "a.png".to_owned(),
            tile_px: (16, 24),
            terrain,
            layers: Vec::new(),
            props: BTreeMap::new(),
            looks: BTreeMap::new(),
        }
    );
}

/// A file of the farm's own tiles with the pictures to place `props` and
/// `looks`.
fn with_props(props: &str, looks: &str) -> String {
    format!(
        "(image: \"tiles/farm.png\", tile_px: (16, 16), terrain: {{\"soil\": (0, 0), \
         \"tilled\": (1, 0), \"water\": (2, 0)}}, props: {{{props}}}, looks: {{{looks}}})"
    )
}

/// The farm's tiles' own look, a winter look with `winter` written in it.
fn winter(winter: &str) -> String {
    format!(
        "\"winter\": (terrain: {{\"soil\": (0, 2), \"tilled\": (1, 2), \"water\": (2, 2)}}{winter})"
    )
}

#[test]
fn a_picture_to_place_is_its_cell_at_its_size() {
    // A wider image than the farm's, for a picture at column 4.
    let images = ImageTable::new([("tiles/farm.png", ImageInfo::new(96, 48))].into());
    let source = with_props(r#""lamp": (at: (4, 0), size: (16, 32))"#, "");
    assert_eq!(problems(&source), Vec::<String>::new());
    let tiles = read(&source)
        .resolve("farm.ron", &images, &terrains())
        .unwrap();
    let lamp = ImageRect {
        x: 4 * 16,
        y: 0,
        w: 16,
        h: 32,
    };
    assert_eq!(tiles.look.props, [("lamp".to_owned(), lamp)].into());
}

#[test]
fn a_picture_to_place_with_no_size_is_a_tile() {
    let source = with_props(r#""sign": (at: (1, 2))"#, "");
    assert_eq!(read(&source).props["sign"], PropAt::new((1, 2)));
    let tiles = resolved(&source).unwrap();
    assert_eq!(tiles.look.props, [("sign".to_owned(), cell((1, 2)))].into());
    // Left out, the size is `tile_px`, a tile not square too.
    let tall = with_props(r#""sign": (at: (1, 0))"#, "").replace("(16, 16)", "(16, 24)");
    let look = resolved(&tall).unwrap().look;
    assert_eq!(
        look.props["sign"],
        ImageRect {
            x: 16,
            y: 0,
            w: 16,
            h: 24,
        }
    );
}

#[test]
fn a_file_without_pictures_to_place_places_none() {
    let source = file("(16, 16)", "", "");
    let def = read(&source);
    assert_eq!(def.props, BTreeMap::new());
    assert_eq!(resolved(&source).unwrap().look.props, BTreeMap::new());
}

#[test]
fn another_looks_pictures_to_place_are_its_own() {
    let source = with_props(
        r#""lamp": (at: (3, 1), size: (16, 32))"#,
        &[
            winter(r#", props: {"lamp": (at: (0, 1)), "well": (at: (1, 1))}"#),
            r#""cave": (terrain: {"soil": (0, 0), "tilled": (1, 0), "water": (2, 0)})"#.to_owned(),
        ]
        .join(", "),
    );
    assert_eq!(problems(&source), Vec::<String>::new());
    let tiles = resolved(&source).unwrap();
    assert_eq!(
        tiles.looks["winter"].props,
        [
            ("lamp".to_owned(), cell((0, 1))),
            ("well".to_owned(), cell((1, 1))),
        ]
        .into()
    );
    // A look without any places nothing, though the own look has some.
    assert_eq!(tiles.looks["cave"].props, BTreeMap::new());
    assert_eq!(tiles.look.props.keys().collect::<Vec<_>>(), ["lamp"]);
}

#[test]
fn a_picture_to_place_has_a_name_and_a_size_inside_the_rules() {
    assert_eq!(
        problems(&with_props(r#""": (at: (0, 0))"#, "")),
        ["prop \"\": its name is empty"]
    );
    for (size, problem) in [
        (
            "(0, 32)",
            "prop \"lamp\": size is (0, 32); a side of 0 px draws nothing",
        ),
        (
            "(16, 0)",
            "prop \"lamp\": size is (16, 0); a side of 0 px draws nothing",
        ),
        (
            "(16, 65)",
            "prop \"lamp\": size is (16, 65); each side must be 8 to 64 px",
        ),
        (
            "(7, 16)",
            "prop \"lamp\": size is (7, 16); each side must be 8 to 64 px",
        ),
    ] {
        let source = with_props(&format!("\"lamp\": (at: (0, 0), size: {size})"), "");
        assert_eq!(problems(&source), [problem], "{size}");
    }
    for edge in ["(8, 8)", "(64, 64)"] {
        let source = with_props(&format!("\"lamp\": (at: (0, 0), size: {edge})"), "");
        assert_eq!(problems(&source), Vec::<String>::new(), "{edge}");
    }
}

#[test]
fn a_side_of_0_is_reported_when_the_rules_allow_0() {
    let rules = TileRules::new(LOOKS, 0..=64);
    let source = with_props(r#""lamp": (at: (0, 0), size: (0, 8))"#, "");
    assert_eq!(
        read(&source).problems(&rules),
        ["prop \"lamp\": size is (0, 8); a side of 0 px draws nothing"]
    );
}

#[test]
fn a_problem_of_a_picture_to_place_names_its_look_in_the_files_order() {
    let source = with_props(
        r#""": (at: (0, 0))"#,
        &winter(
            r#", layers: [Tiles(of: [], at: (0, 0))], props: {"lamp": (at: (0, 0), size: (0, 0))}"#,
        ),
    );
    assert_eq!(
        problems(&source),
        [
            "prop \"\": its name is empty",
            "look \"winter\": layer 1: `of` names no terrain",
            "look \"winter\": prop \"lamp\": size is (0, 0); a side of 0 px draws nothing",
        ]
    );
}

#[test]
fn a_picture_to_place_lies_inside_the_image() {
    let outside = |what: &str, size: &str, at: &str| {
        format!("farm.ron: {what}: its {size} px at {at} lies outside the 64×48 px image")
    };
    let own = with_props(r#""lamp": (at: (3, 2), size: (16, 32))"#, "");
    assert_eq!(
        errors(&own),
        [outside("prop \"lamp\"", "16×32", "(48, 32)")]
    );
    assert_eq!(codes_of(&own), [Some(codes::OUT_OF_RANGE)]);
    let other = with_props("", &winter(r#", props: {"well": (at: (4, 0))}"#));
    assert_eq!(
        errors(&other),
        [outside(
            "look \"winter\": prop \"well\"",
            "16×16",
            "(64, 0)"
        )]
    );
    // At its size, a picture may reach the image's edge.
    let edge = with_props(r#""lamp": (at: (3, 1), size: (16, 32))"#, "");
    assert!(resolved(&edge).is_ok());
}

#[test]
fn a_picture_to_place_refuses_an_unknown_field() {
    let own = with_props(r#""lamp": (at: (0, 0), offset: (1, 1))"#, "");
    let other = with_props(
        "",
        &winter(r#", props: {"well": (at: (1, 1), name: "well")}"#),
    );
    for source in [own, other] {
        let bad = &source;
        assert!(
            parse_ron::<TerrainTilesDef>("t.ron", &source).is_err(),
            "{bad}"
        );
    }
}

/// A picture to place, with a size or without.
fn prop_at() -> impl Strategy<Value = PropAt> {
    (
        (0..8_u32, 0..8_u32),
        proptest::option::of((1..65_u32, 1..65_u32)),
    )
        .prop_map(|(at, size)| {
            let prop = PropAt::new(at);
            size.map_or(prop, |size| prop.with_size(size))
        })
}

/// Pictures to place, by name.
fn props() -> impl Strategy<Value = BTreeMap<String, PropAt>> {
    proptest::collection::btree_map("[a-z ]{0,6}", prop_at(), 0..4)
}

proptest! {
    #[test]
    fn a_file_with_pictures_to_place_written_back_reads_as_itself(
        own in props(),
        other in props(),
    ) {
        let mut def = read(FARM);
        def.props = own;
        if let Some(winter) = def.looks.get_mut("winter") {
            winter.props = other;
        }
        let text = ::ron::to_string(&def).unwrap();
        prop_assert_eq!(read(&text), def, "{}", text);
    }
}

#[test]
fn the_format_reads_a_file_and_reports_its_problems() {
    let format = TerrainTilesRon::new(RULES);
    let decode =
        |bytes: &[u8]| format.decode(&AssetPath::new("t/farm.ron"), bytes, &DecodeCx::new());
    assert_eq!(decode(FARM.as_bytes()), Ok(read(FARM)));
    let bad = file("(7, 16)", "Tiles(of: [], at: (0, 0))", "");
    let shown: Vec<String> = decode(bad.as_bytes())
        .err()
        .unwrap_or_default()
        .iter()
        .map(ToString::to_string)
        .collect();
    assert_eq!(
        shown,
        [
            "t/farm.ron: tile_px is (7, 16); each side must be 8 to 64 px",
            "t/farm.ron: layer 1: `of` names no terrain",
        ]
    );
    let syntax = decode(b"(image: ").err().unwrap_or_default();
    assert_eq!(syntax.len(), 1);
    assert_eq!(syntax[0].code, Some(codes::PARSE));
}
