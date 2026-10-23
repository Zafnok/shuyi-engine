//! Tests of the `.map` format over fixture maps, with a fixture feature.

use proptest::prelude::*;

use super::*;

/// The fixture's terrains, by id.
const TERRAINS: [&str; 4] = ["plain", "forest", "water", "wall"];

fn terrain(id: &str) -> Option<u16> {
    TERRAINS
        .iter()
        .position(|&t| t == id)
        .and_then(|i| u16::try_from(i).ok())
}

const FORMAT: MapFormat<'static> = MapFormat {
    looks: &["outdoor", "indoor"],
    max_size: 64,
};

/// The fixture's one feature: a sign with words on it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
enum Thing {
    Sign(String),
}

impl MapFeature for Thing {
    fn problem(&self) -> Option<String> {
        match self {
            Thing::Sign(words) if words.is_empty() => Some("the sign is blank".to_owned()),
            Thing::Sign(_) => None,
        }
    }
}

type Map = GridMap<u16, Thing>;

const HEADER: &str = "(\n  name: \"Test\",\n  legend: { '.': \"plain\", 'T': \"forest\", '~': \"water\" },\n)\n---\n";

fn parse_map(src: &str) -> Result<Map, Vec<ContentError>> {
    parse("m.map", src, &FORMAT, terrain)
}

/// The codes of `src`'s errors, in order.
fn codes_of(src: &str) -> Vec<Option<crate::error::Code>> {
    parse_map(src)
        .err()
        .unwrap_or_default()
        .iter()
        .map(|e| e.code)
        .collect()
}

fn errors(src: &str) -> Vec<String> {
    parse_map(src)
        .err()
        .unwrap_or_default()
        .iter()
        .map(ToString::to_string)
        .collect()
}

#[test]
fn parses_valid_map() {
    let map = parse_map(&format!("{HEADER}.T~\n~T.\n\n")).unwrap();
    assert_eq!(map.name, "Test");
    assert_eq!((map.tiles.width(), map.tiles.height()), (3, 2));
    let at = |x, y| map.tiles.get(Pos::new(x, y)).copied();
    assert_eq!(at(0, 0), Some(0));
    assert_eq!(at(1, 0), Some(1));
    assert_eq!(at(2, 0), Some(2));
    assert_eq!(at(0, 1), Some(2));
    assert_eq!(map.legend.terrains.get(&'T'), Some(&1));
    assert_eq!(
        map.legend.names.get(&'~').map(String::as_str),
        Some("water")
    );
    assert!(map.features.is_empty(), "{:?}", map.features);
}

#[test]
fn crlf_line_endings_are_accepted() {
    let src = format!("{HEADER}.T\n~.\n").replace('\n', "\r\n");
    let map = parse_map(&src).unwrap();
    assert_eq!((map.tiles.width(), map.tiles.height()), (2, 2));
}

#[test]
fn missing_separator() {
    assert_eq!(
        errors("(name: \"x\", legend: {})\n..\n"),
        ["m.map: missing the \"---\" line between the header and the tiles"]
    );
}

#[test]
fn header_syntax_error_is_positioned() {
    let errs = parse_map("(\n  name: \"x\"\n  legend: {},\n)\n---\n..\n").unwrap_err();
    assert_eq!(errs.len(), 1);
    assert_eq!(errs[0].line, Some(3));
}

#[test]
fn unknown_legend_char() {
    assert_eq!(
        errors(&format!("{HEADER}..\n.x\nx.\n")),
        [
            "m.map:7:2: 'x' is not in the legend",
            "m.map:8:1: 'x' is not in the legend"
        ]
    );
    assert_eq!(
        codes_of(&format!("{HEADER}..\n.x\nx.\n")),
        [Some(codes::UNKNOWN_NAME); 2]
    );
}

#[test]
fn ragged_rows() {
    assert_eq!(
        errors(&format!("{HEADER}...\n..\n....\n\n...\n")),
        [
            "m.map:7:3: row is 2 tiles wide; expected 3 like the first row",
            "m.map:8:4: row is 4 tiles wide; expected 3 like the first row",
            "m.map:9:1: row is 0 tiles wide; expected 3 like the first row",
        ]
    );
}

#[test]
fn unknown_terrain_in_legend() {
    let src = "(\n  name: \"x\",\n  legend: {\n    '.': \"plain\",\n    'L': \"lava\",\n  },\n)\n---\n.L\n";
    assert_eq!(
        errors(src),
        ["m.map:5:5: legend 'L': unknown terrain \"lava\""]
    );
    assert_eq!(codes_of(src), [Some(codes::UNKNOWN_NAME)]);
}

#[test]
fn legend_position_fallbacks() {
    let header = ["legend: { '\\'': \"lava\" }"];
    assert_eq!(legend_position(&header, '\'', "lava"), (1, 17));
    assert_eq!(legend_position(&["a", " 'q'"], 'q', "x"), (2, 2));
    assert_eq!(legend_position(&["a"], 'q', "x"), (1, 1));
    assert_eq!(legend_position(&["é'q'"], 'q', "x"), (1, 2));
}

#[test]
fn empty_map() {
    assert_eq!(
        errors(&format!("{HEADER}\n\n")),
        ["m.map:6:1: map has no tiles"]
    );
    assert_eq!(errors(HEADER), ["m.map:6:1: map has no tiles"]);
    assert_eq!(
        errors(&format!("{HEADER}\n..\n")),
        ["m.map:6:1: first tile row is empty"]
    );
}

#[test]
fn size_limits() {
    let wide = ".".repeat(65);
    assert_eq!(
        errors(&format!("{HEADER}{wide}\n")),
        ["m.map:6:65: map is 65 tiles wide; the maximum is 64"]
    );
    assert_eq!(
        codes_of(&format!("{HEADER}{wide}\n")),
        [Some(codes::OUT_OF_RANGE)]
    );
    let tall = ".\n".repeat(65);
    assert_eq!(
        errors(&format!("{HEADER}{tall}")),
        ["m.map:70:1: map is 65 tiles tall; the maximum is 64"]
    );
    assert_eq!(
        codes_of(&format!("{HEADER}{tall}")),
        [Some(codes::OUT_OF_RANGE)]
    );
    let max = format!("{}\n", ".".repeat(64)).repeat(64);
    let map = parse_map(&format!("{HEADER}{max}")).unwrap();
    assert_eq!((map.tiles.width(), map.tiles.height()), (64, 64));
    // The game picks the limit.
    let small = MapFormat {
        max_size: 2,
        ..FORMAT
    };
    let three = parse::<u16, Thing>("m.map", &format!("{HEADER}...\n"), &small, terrain);
    assert_eq!(
        three.unwrap_err()[0].message,
        "map is 3 tiles wide; the maximum is 2"
    );
}

#[test]
fn all_errors_reported_together() {
    let src = "(name: \"x\", legend: { '.': \"plain\", 'L': \"lava\" })\n---\n.L\n.q.\n";
    assert_eq!(errors(src).len(), 3);
}

#[test]
fn print_round_trips_and_escapes_name() {
    let src = format!("{HEADER}.T~\n~T.\n").replace("\"Test\"", "\"Say \\\"hi\\\"\"");
    let map = parse_map(&src).unwrap();
    let printed = print(&map).unwrap();
    assert_eq!(parse_map(&printed), Ok(map.clone()));
    assert_eq!(map.name, "Say \"hi\"");
    assert!(printed.ends_with("\n---\n.T~\n~T.\n"), "{printed}");
}

#[test]
fn a_map_names_its_look_or_is_outdoor() {
    // No `look`: outdoor, and printing it writes none.
    let plain = parse_map(&format!("{HEADER}.T\n")).unwrap();
    assert_eq!(plain.look, MapLook::default());
    assert_eq!(plain.look.tiles, "outdoor");
    assert!(plain.look.is_default());
    let printed = print(&plain).unwrap();
    assert!(!printed.contains("look"), "{printed}");
    // Named: kept beside the tiles, and printed back.
    let header = HEADER.replace(",\n)", ",\n  look: (tiles: \"indoor\"),\n)");
    let indoor = parse_map(&format!("{header}.T\n")).unwrap();
    assert_eq!(indoor.look.tiles, "indoor");
    assert!(!indoor.look.is_default());
    assert_eq!(indoor.tiles, plain.tiles);
    let printed = print(&indoor).unwrap();
    assert!(printed.contains("look:(tiles:\"indoor\")"), "{printed}");
    assert_eq!(parse_map(&printed), Ok(indoor));
    // `look: ()` is the default look too.
    let empty = HEADER.replace(",\n)", ",\n  look: (),\n)");
    assert_eq!(parse_map(&format!("{empty}.T\n")).unwrap().look, plain.look);
}

#[test]
fn a_map_names_its_light_or_is_at_noon() {
    let small = format!("{HEADER}.T\n");
    let plain = parse_map(&small).unwrap();
    assert_eq!(plain.look.light, "noon");
    let source = small.replacen("legend:", "look: (light: \"dusk\"),\n  legend:", 1);
    let dusk = parse_map(&source).unwrap();
    assert_eq!(
        (dusk.look.tiles.as_str(), dusk.look.light.as_str()),
        ("outdoor", "dusk")
    );
    assert!(!dusk.look.is_default());
    assert_eq!(dusk.tiles, plain.tiles);
    let printed = print(&dusk).unwrap();
    assert!(
        printed.contains("look:(tiles:\"outdoor\",light:\"dusk\")"),
        "{printed}"
    );
    assert_eq!(parse_map(&printed).map(|m| m.look), Ok(dusk.look.clone()));
    // Both, in either order.
    let both = small.replacen(
        "legend:",
        "look: (light: \"night\", tiles: \"indoor\"),\n  legend:",
        1,
    );
    let look = parse_map(&both).unwrap().look;
    assert_eq!(
        (look.tiles.as_str(), look.light.as_str()),
        ("indoor", "night")
    );
    // Another light replaces the map's; none leaves it.
    assert_eq!(look.clone().lit(None), look);
    let lit = look.lit(Some("dusk"));
    assert_eq!((lit.tiles.as_str(), lit.light.as_str()), ("indoor", "dusk"));
}

#[test]
fn an_unknown_look_is_reported_where_it_is_written() {
    let header = HEADER.replace(",\n)", ",\n  look: (tiles: \"cave\"),\n)");
    assert_eq!(
        errors(&format!("{header}.T\n")),
        ["m.map:4:17: look: tiles \"cave\" is not a look; the looks are \"outdoor\", \"indoor\""]
    );
    // With the map's other problems, not instead of them.
    assert_eq!(errors(&format!("{header}.x\n")).len(), 2);
    // A field a look doesn't have is a syntax error.
    let header = HEADER.replace(",\n)", ",\n  look: (tile: \"indoor\"),\n)");
    let errs = parse_map(&format!("{header}.T\n")).unwrap_err();
    assert_eq!(errs.len(), 1);
    assert!(errs[0].line.is_some());
}

#[test]
fn positions_are_found_in_the_header() {
    let header = ["(", "  name: \"é x\",", "  x: \"x\",", ")"];
    assert_eq!(header_position(&header, "\"x\""), Some((3, 6)));
    // Columns count characters, not bytes.
    assert_eq!(header_position(&header, " x\""), Some((2, 11)));
    assert_eq!(header_position(&header, "nowhere"), None);
    assert_eq!(header_position(&[], "x"), None);
}

#[test]
fn header_lines_stop_at_the_separator() {
    assert_eq!(header_lines("(a)\nb\n---\nc\n"), ["(a)", "b"]);
    assert_eq!(header_lines("(a)\n\"x\"\n"), ["(a)", "\"x\""]);
}

#[test]
fn print_needs_every_terrain_in_legend() {
    let legend = Legend::resolve(BTreeMap::from([('.', "plain".to_owned())]), terrain).unwrap();
    let map: Map = GridMap {
        name: "m".to_owned(),
        tiles: Grid::from_cells(2, 1, vec![0, 3]).unwrap(),
        features: BTreeMap::new(),
        legend,
        look: MapLook::default(),
    };
    assert_eq!(print(&map), None);
}

#[test]
fn legend_resolve_and_char_for() {
    let names = BTreeMap::from([('b', "forest".to_owned()), ('a', "forest".to_owned())]);
    let legend = Legend::resolve(names, terrain).unwrap();
    assert_eq!(legend.char_for(1), Some('a'));
    assert_eq!(legend.char_for(0), None);
    let bad = BTreeMap::from([('a', "lava".to_owned())]);
    assert_eq!(Legend::resolve(bad, terrain), None);
}

const FEATURES: &str = "(
  name: \"F\",
  legend: { '.': \"plain\", '#': \"wall\" },
  features: {
    (0, 0): Sign(\"North\"),
    (2, 1): Sign(\"Keep out\"),
  },
)
---
..#
..#
";

#[test]
fn features_are_the_games() {
    let map = parse_map(FEATURES).unwrap();
    assert_eq!(map.features.len(), 2);
    assert_eq!(
        map.features.get(&Pos::new(2, 1)),
        Some(&Thing::Sign("Keep out".to_owned()))
    );
    let printed = print(&map).unwrap();
    assert!(printed.contains("(2,1):Sign(\"Keep out\")"), "{printed}");
    assert_eq!(parse_map(&printed), Ok(map));
}

#[test]
fn feature_problems_are_reported() {
    let src = FEATURES
        .replace("(0, 0)", "(5, 0)")
        .replace("\"Keep out\"", "\"\"");
    assert_eq!(
        errors(&src),
        [
            "m.map: feature at (2, 1): the sign is blank",
            "m.map: feature at (5, 0): outside the map",
        ]
    );
    // A feature the game doesn't have is a syntax error.
    let errs = parse_map(&FEATURES.replace("Sign(\"North\")", "Gate")).unwrap_err();
    assert_eq!(errs.len(), 1);
    assert!(errs[0].line.is_some());
}

/// A feature with no problems of its own: the hook's default.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Mark(u8);

impl MapFeature for Mark {}

#[test]
fn a_feature_has_no_problem_unless_the_game_says() {
    assert_eq!(Mark(0).problem(), None);
    let src = HEADER.replace(",\n)", ",\n  features: { (0, 0): (0) },\n)");
    let map = parse::<u16, Mark>("m.map", &format!("{src}..\n"), &FORMAT, terrain).unwrap();
    assert_eq!(map.features.get(&Pos::new(0, 0)), Some(&Mark(0)));
}

#[test]
fn a_game_without_features_has_none() {
    let map = parse::<u16, NoFeature>("m.map", &format!("{HEADER}..\n"), &FORMAT, terrain);
    let found = map.unwrap().features;
    assert!(found.is_empty(), "{found:?}");
    let errs = parse::<u16, NoFeature>("m.map", FEATURES, &FORMAT, terrain).unwrap_err();
    assert_eq!(errs.len(), 1);
}

const LEGEND_CHARS: &str = ".T~#^=F";

fn arb_map() -> impl Strategy<Value = Map> {
    let size = (1u16..=12, 1u16..=12, 1usize..=4, "[ -~]{0,12}");
    size.prop_flat_map(|(w, h, n, name)| {
        let cells = proptest::collection::vec(0..n, usize::from(w) * usize::from(h));
        let features =
            proptest::collection::btree_map((0..i32::from(w), 0..i32::from(h)), "[ -~]{1,8}", 0..4);
        (Just((w, h, n, name)), cells, features, 0usize..2, 0usize..2)
    })
    .prop_map(|((w, h, n, name), cells, features, tiles, light)| {
        let names: BTreeMap<char, String> = LEGEND_CHARS
            .chars()
            .zip(TERRAINS)
            .take(n)
            .map(|(c, t)| (c, t.to_owned()))
            .collect();
        let cells = cells
            .into_iter()
            .map(|i| u16::try_from(i).unwrap_or(0))
            .collect();
        GridMap {
            name,
            tiles: Grid::from_cells(w, h, cells).unwrap(),
            features: features
                .into_iter()
                .map(|((x, y), words)| (Pos::new(x, y), Thing::Sign(words)))
                .collect(),
            legend: Legend::resolve(names, terrain).unwrap(),
            look: MapLook {
                tiles: FORMAT.looks[tiles].to_owned(),
                light: ["noon", "dusk"][light].to_owned(),
            },
        }
    })
}

proptest! {
    #[test]
    fn parse_print_round_trip(map in arb_map()) {
        let printed = print(&map);
        prop_assert!(printed.is_some());
        let printed = printed.unwrap_or_default();
        let parsed = parse_map(&printed);
        prop_assert_eq!(parsed.as_ref(), Ok(&map));
        // And printing it again gives the same text.
        prop_assert_eq!(parsed.ok().and_then(|m| print(&m)), Some(printed));
    }
}

/// A format as games build theirs, in a `const` item.
const SMALL: MapFormat<'static> = MapFormat::new(&[DEFAULT_TILES], 8);

#[test]
fn a_new_format_is_the_literal() {
    assert_eq!(
        SMALL,
        MapFormat {
            looks: &[DEFAULT_TILES],
            max_size: 8,
        }
    );
}
