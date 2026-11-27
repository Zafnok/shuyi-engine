use super::*;

use proptest::prelude::*;
use shuyi_core::state_hash;

use crate::format::builtin_tests::{bytes, edited};
use crate::format::{Format, Formats};
use crate::image::Png;

/// The ticket's example, completed with the parts it names.
const EXAMPLE: &str = r##"(
    size: (32, 32),
    // Draw order, first undermost.
    slots: ["back_hair", "head", "ears", "eyes", "hair", "beard", "helmet"],
    // A grey (red = green = blue) and what it stands for.
    keys: { 200: "skin", 150: "skin_shadow", 120: "hair", 90: "eyes" },
    // Keys coloured together: one roll picks a row.
    colours: {
        "skin": (keys: ["skin", "skin_shadow"], rows: [
            ["#F1C27D", "#C68642"], ["#8D5524", "#5C3A1A"],
        ]),
        "hair": (keys: ["hair"], rows: [["#2B1B10"], ["#D8B060"]]),
        "eyes": (keys: ["eyes"], rows: [["#3A6EA5"], ["#4A3020"]]),
    },
    parts: {
        "head_round": (slot: "head", image: "faces/heads.png", at: (0, 0)),
        "head_long": (slot: "head", image: "faces/heads.png", at: (32, 0)),
        "ears_long": (slot: "ears", image: "faces/ears.png", at: (0, 0)),
        "hair_short": (slot: "hair", image: "faces/hair.png", at: (0, 0)),
        "hair_long": (slot: "hair", image: "faces/hair.png", at: (32, 0)),
        "beard_full": (slot: "beard", image: "faces/beards.png", at: (0, 0)),
        "helm_iron": (slot: "helmet", image: "faces/gear.png", at: (0, 0),
            hides: ["hair", "back_hair", "ears"]),
    },
    kinds: {
        "plain": (slots: {
            "head": ["head_round", "head_long"],
            "hair": ["hair_short", "hair_long", None],
            "beard": [None, "beard_full"],
        }),
        "elf": (base: Some("plain"), slots: { "ears": ["ears_long"] }),
    },
)"##;

/// The example's images, by size.
fn example_images() -> ImageTable {
    ImageTable::new(
        [
            ("faces/heads.png", ImageInfo::new(64, 32)),
            ("faces/ears.png", ImageInfo::new(32, 32)),
            ("faces/hair.png", ImageInfo::new(64, 32)),
            ("faces/beards.png", ImageInfo::new(32, 32)),
            ("faces/gear.png", ImageInfo::new(32, 32)),
        ]
        .into(),
    )
}

/// The example, checked and resolved.
fn example() -> PartSet {
    let def = PartsDef::from_source("faces.ron", EXAMPLE).unwrap();
    def.resolve("faces.ron", "", &example_images()).unwrap()
}

/// The fixture's file.
pub(crate) const FACES: &str = include_str!("../../tests/fixtures/parts/faces.ron");

/// The fixture's picture.
pub(crate) const FACES_PNG: &[u8] = include_bytes!("../../tests/fixtures/parts/faces.png");

/// The fixture's picture, decoded.
fn faces_image() -> Image {
    Png.decode(
        &AssetPath::new("parts/faces.png"),
        FACES_PNG,
        &DecodeCx::new(),
    )
    .unwrap()
}

/// The fixture, checked and resolved, its file in `parts/`.
fn faces() -> PartSet {
    let image = faces_image();
    let images = ImageTable::new([("parts/faces.png", image.info())].into());
    let def = PartsDef::from_source("parts/faces.ron", FACES).unwrap();
    def.resolve("parts/faces.ron", "parts", &images).unwrap()
}

/// Each error of `result` as `file:line: message`.
fn shown<T: core::fmt::Debug>(result: Result<T, Vec<ContentError>>) -> Vec<String> {
    result
        .unwrap_err()
        .iter()
        .map(ToString::to_string)
        .collect()
}

/// The 1-based line of `source` that first holds `needle`.
fn line_of(source: &str, needle: &str) -> usize {
    source.lines().position(|l| l.contains(needle)).unwrap() + 1
}

#[test]
fn the_example_file_loads() {
    let set = example();
    assert_eq!(set.size, (32, 32));
    assert_eq!(set.slots.len(), 7);
    assert_eq!(set.kinds().collect::<Vec<_>>(), ["elf", "plain"]);
    let helm = &set.parts["helm_iron"];
    assert_eq!(helm.slot, 6);
    assert_eq!(helm.hides, [4, 0, 2]);
    assert_eq!(helm.image.path(), "faces/gear.png");
    assert_eq!(set.keys[&150], "skin_shadow");
    // The fixture loads too, its image beside it.
    assert_eq!(faces().parts["hat_cap"].image.path(), "parts/faces.png");
}

#[test]
fn the_format_is_parts_and_reads_part_sets() {
    assert_eq!(PartsRon.id(), "parts");
    assert_eq!(PartsRon.extensions(), ["ron"]);
    assert_eq!(PartsRon.max_bytes(), TEXT_MAX_BYTES);
    assert_eq!(<PartsDef as AssetKind>::NAME, "part set");
    let mut formats = Formats::new();
    formats.register(PartsRon).unwrap();
    let path = AssetPath::new("parts/faces.ron");
    let read = formats.decode::<PartsDef>(&path, FACES.as_bytes(), &DecodeCx::new());
    assert_eq!(read, PartsDef::from_source("parts/faces.ron", FACES));
    let not_text = PartsRon.decode(&path, &[0xff, 0xfe], &DecodeCx::new());
    assert_eq!(
        shown(not_text),
        ["parts/faces.ron: file is not valid UTF-8"]
    );
    // An unknown field is refused.
    let extra = EXAMPLE.replacen("size: (32, 32),", "size: (32, 32), scale: 2,", 1);
    let errors = shown(PartsDef::from_source("faces.ron", &extra));
    assert!(errors[0].starts_with("faces.ron:2:"), "{errors:?}");
}

/// Each check of a file alone: the example edited (`from` → `to`), the
/// line that shows the problem (the first that holds `at`), and its
/// message.
const CHECKS: &[(&str, &str, &str, &str)] = &[
    (
        r#"slots: ["back_hair", "head", "ears", "eyes", "hair", "beard", "helmet"],"#,
        "slots: [],",
        "slots: [],",
        "the set has no slot",
    ),
    (
        r#""beard", "helmet"],"#,
        r#""beard", "helmet",
        "head"],"#,
        r#""head"],"#,
        "slot \"head\" is named twice",
    ),
    (
        r#""helm_iron": (slot: "helmet","#,
        r#""helm_iron": (slot: "hat","#,
        "\"helm_iron\"",
        "part \"helm_iron\": slot \"hat\" is not one of `slots`",
    ),
    (
        r#"hides: ["hair", "back_hair", "ears"]"#,
        r#"hides: ["hair", "mane", "ears"]"#,
        "\"helm_iron\"",
        "part \"helm_iron\": it hides \"mane\", which is not one of `slots`",
    ),
    (
        r#""hair_short": (slot: "hair", image: "faces/hair.png", at: (0, 0)),"#,
        r#""hair_short": (slot: "hair", image: "faces/hair.png", at: (0, 0), hides: ["helmet"]),"#,
        "\"hair_short\": (",
        "part \"hair_short\": it hides \"helmet\", which is not under its own slot \"hair\"",
    ),
    (
        r#""beard": [None, "beard_full"],"#,
        r#""chin": [None],"#,
        "\"plain\": (",
        "kind \"plain\": slot \"chin\" is not one of `slots`",
    ),
    (
        r#""head": ["head_round", "head_long"],"#,
        r#""head": ["head_round", "hair_short"],"#,
        "\"plain\": (",
        "kind \"plain\": slot \"head\": part \"hair_short\" goes in slot \"hair\"",
    ),
    (
        r#""head": ["head_round", "head_long"],"#,
        r#""head": ["head_round", "head_flat"],"#,
        "\"plain\": (",
        "kind \"plain\": slot \"head\": \"head_flat\" is not a part",
    ),
    (
        r#""beard": [None, "beard_full"],"#,
        r#""beard": [],"#,
        "\"plain\": (",
        "kind \"plain\": the list of slot \"beard\" is empty",
    ),
    (
        r#"base: Some("plain")"#,
        r#"base: Some("human")"#,
        "\"elf\": (",
        "kind \"elf\": its base \"human\" is not a kind",
    ),
    (
        r#""plain": (slots: {"#,
        r#""plain": (base: Some("elf"), slots: {"#,
        "\"elf\": (",
        "kinds \"elf\", \"plain\" make a cycle of bases",
    ),
    (
        r#"90: "eyes" },"#,
        r#"90: "eyes",
        200: "paint" },"#,
        "200: \"paint\"",
        "key grey 200 is named twice",
    ),
    (
        r#"90: "eyes" },"#,
        r#"90: "eyes", 80: "hair" },"#,
        "80:",
        "two key greys stand for \"hair\"",
    ),
    (
        r#""hair": (keys: ["hair"],"#,
        r#""hair": (keys: ["fur"],"#,
        "\"hair\": (keys",
        "colour group \"hair\": \"fur\" is not a key",
    ),
    (
        r##""eyes": (keys: ["eyes"], rows: [["#3A6EA5"], ["#4A3020"]]),"##,
        r##""eyes": (keys: ["eyes", "hair"], rows: [["#3A6EA5", "#000000"]]),"##,
        // Groups are read by name: "eyes" took the key first.
        "\"hair\": (keys",
        "colour group \"hair\": key \"hair\" is in group \"eyes\" too",
    ),
    (
        r##""eyes": (keys: ["eyes"], rows: [["#3A6EA5"], ["#4A3020"]]),"##,
        r#""eyes": (keys: ["eyes"], rows: []),"#,
        "\"eyes\": (keys",
        "colour group \"eyes\" has no rows",
    ),
    (
        r##"["#F1C27D", "#C68642"], ["#8D5524", "#5C3A1A"],"##,
        r##"["#F1C27D", "#C68642"], ["#8D5524"],"##,
        "\"skin\": (keys",
        "colour group \"skin\": row 2 has 1 colours; it needs one per key, 2",
    ),
    (
        r##"rows: [["#2B1B10"], ["#D8B060"]]"##,
        r##"rows: [["#2B1B10"], ["D8B060"]]"##,
        "\"hair\": (keys",
        "colour group \"hair\": row 2: \"D8B060\" must start with '#' (format \"#RRGGBB\")",
    ),
    (
        "size: (32, 32),",
        "size: (257, 32),",
        "size:",
        "size is 257×32 px; each side must be 1 to 256 px",
    ),
    (
        "size: (32, 32),",
        "size: (32, 0),",
        "size:",
        "size is 32×0 px; each side must be 1 to 256 px",
    ),
];

#[test]
fn each_check_of_a_file_alone_has_a_file_that_fails_it_at_its_line() {
    for &(from, to, at, message) in CHECKS {
        assert!(EXAMPLE.contains(from), "{from}");
        let source = EXAMPLE.replacen(from, to, 1);
        let want = format!("faces.ron:{}: {message}", line_of(&source, at));
        let errors = shown(PartsDef::from_source("faces.ron", &source));
        assert!(errors.contains(&want), "{want}\nin {errors:#?}");
    }
}

#[test]
fn a_name_on_a_line_is_found_as_a_whole_word() {
    assert!(check::holds("    200: \"skin\",", "200:"));
    assert!(check::holds("{20: \"a\"}", "20:"));
    // Not as the end of a longer number or name.
    assert!(!check::holds("    120: \"hair\",", "20:"));
    assert!(!check::holds("    a_20: 1,", "20:"));
    assert!(!check::holds("    x20: 1,", "20:"));
    // A later one counts when an earlier one doesn't.
    assert!(check::holds("120: \"a\", 20: \"b\"", "20:"));
}

#[test]
fn keys_are_a_map_and_say_so() {
    use serde::de::IntoDeserializer;
    use serde::de::value::{Error, U32Deserializer};
    let number: U32Deserializer<Error> = 5_u32.into_deserializer();
    let read: Result<Vec<(u8, String)>, Error> = entries::deserialize(number);
    let error = read.unwrap_err().to_string();
    assert!(error.contains("expected a map"), "{error}");
}

#[test]
fn a_set_read_without_its_checks_stops_at_a_cycle_of_bases() {
    let cycle = EXAMPLE.replacen(
        r#""plain": (slots: {"#,
        r#""plain": (base: Some("elf"), slots: {"#,
        1,
    );
    // Read straight into the type, as a game could, without the checks.
    let def: PartsDef = crate::ron::parse_ron_implicit_some("faces.ron", &cycle).unwrap();
    let set = def.resolve("faces.ron", "", &example_images()).unwrap();
    for seed in 0..20 {
        let elf = set.appearance("elf", seed, &[]).unwrap();
        let plain = set.appearance("plain", seed, &[]).unwrap();
        // Each kind's own lists first, then its base's, once.
        assert_eq!(elf.parts[2].as_deref(), Some("ears_long"));
        assert_eq!(plain.parts[2].as_deref(), Some("ears_long"));
        assert_eq!(elf.parts[1], plain.parts[1]);
    }
}

#[test]
fn a_kind_may_replace_a_groups_rows_and_its_rows_are_checked() {
    let green = EXAMPLE.replacen(
        r#"slots: { "ears": ["ears_long"] }"#,
        r##"slots: { "ears": ["ears_long"] },
            colours: { "skin": [["#5A8A3A", "#3A6A2A"]] }"##,
        1,
    );
    let def = PartsDef::from_source("faces.ron", &green).unwrap();
    let set = def.resolve("faces.ron", "", &example_images()).unwrap();
    for seed in 0..20 {
        let elf = set.appearance("elf", seed, &[]).unwrap();
        assert_eq!(elf.colours[&200], [0x5a, 0x8a, 0x3a]);
        assert_eq!(elf.colours[&150], [0x3a, 0x6a, 0x2a]);
        // The base keeps the set's rows, and the hair the set's too.
        let plain = set.appearance("plain", seed, &[]).unwrap();
        assert_eq!(elf.colours[&120], plain.colours[&120]);
    }
    let wrong = green.replacen(r##"[["#5A8A3A", "#3A6A2A"]]"##, r##"[["#5A8A3A"]]"##, 1);
    let errors = shown(PartsDef::from_source("faces.ron", &wrong));
    let line = line_of(&wrong, "\"elf\": (");
    assert_eq!(
        errors,
        [format!(
            "faces.ron:{line}: kind \"elf\": colour group \"skin\": row 1 has 1 colours; it needs one per key, 2"
        )]
    );
    let unknown = green.replacen(r#"colours: { "skin""#, r#"colours: { "fur""#, 1);
    let errors = shown(PartsDef::from_source("faces.ron", &unknown));
    assert_eq!(
        errors,
        [format!(
            "faces.ron:{line}: kind \"elf\": \"fur\" is not a colour group"
        )]
    );
}

/// A set with `slots` slots, `parts` parts, `kinds` kinds and `rows` rows
/// in its one group, each section on a line of its own.
fn many(slots: usize, parts: usize, kinds: usize, rows: usize) -> String {
    let slot_names: Vec<String> = (0..slots).map(|i| format!("\"s{i}\"")).collect();
    let part_list: Vec<String> = (0..parts)
        .map(|i| format!("\"p{i}\": (slot: \"s0\", image: \"a.png\", at: (0, 0))"))
        .collect();
    let kind_list: Vec<String> = (0..kinds).map(|i| format!("\"k{i}\": ()")).collect();
    let row_list = vec!["[\"#000000\"]"; rows];
    format!(
        "(\n    size: (8, 8),\n    slots: [{}],\n    keys: {{ 200: \"skin\" }},\n    colours: {{\n        \"skin\": (keys: [\"skin\"], rows: [{}]),\n    }},\n    parts: {{\n        {}\n    }},\n    kinds: {{\n        {}\n    }},\n)",
        slot_names.join(", "),
        row_list.join(", "),
        part_list.join(",\n        "),
        kind_list.join(",\n        "),
    )
}

#[test]
fn the_caps_are_64_slots_1024_parts_256_kinds_and_64_rows() {
    let at_caps = many(MAX_SLOTS, MAX_PARTS, MAX_KINDS, MAX_ROWS);
    assert!(PartsDef::from_source("a.ron", &at_caps).is_ok());
    let cases = [
        (
            many(65, 1, 1, 1),
            "slots:",
            "65 slots; a set may have at most 64",
        ),
        (
            many(1, 1025, 1, 1),
            "parts:",
            "1025 parts; a set may have at most 1024",
        ),
        (
            many(1, 1, 257, 1),
            "kinds:",
            "257 kinds; a set may have at most 256",
        ),
        (
            many(1, 1, 1, 65),
            "\"skin\": (",
            "colour group \"skin\" has 65 rows; a group may have at most 64",
        ),
    ];
    for (source, at, message) in cases {
        let want = format!("a.ron:{}: {message}", line_of(&source, at));
        assert_eq!(shown(PartsDef::from_source("a.ron", &source)), [want]);
    }
}

#[test]
fn a_part_whose_image_is_missing_or_too_small_does_not_resolve() {
    let def = PartsDef::from_source("faces.ron", EXAMPLE).unwrap();
    let mut images = example_images();
    images.images.remove("faces/gear.png");
    images
        .images
        .insert("faces/hair.png", ImageInfo::new(63, 32));
    let errors = shown(def.resolve("faces.ron", "", &images));
    assert_eq!(
        errors,
        [
            "faces.ron: part \"hair_long\": 32×32 px at (32, 0) is outside faces/hair.png, which is 63×32 px",
            "faces.ron: part \"helm_iron\": faces/gear.png is not an image in the assets",
        ]
    );
    // Paths are relative to the file's folder.
    let moved = def.resolve("art/faces.ron", "art", &example_images());
    assert!(shown(moved).iter().all(|e| e.contains("art/faces/")));
}

#[test]
fn a_look_is_the_same_for_the_same_seed_and_varies_over_seeds() {
    let set = example();
    let looks: BTreeSet<Appearance> = (0..100)
        .map(|seed| {
            let look = set.appearance("plain", seed, &[]).unwrap();
            assert_eq!(set.appearance("plain", seed, &[]), Ok(look.clone()));
            look
        })
        .collect();
    assert!(looks.len() >= 2, "{looks:?}");
    let first = looks.first().unwrap();
    assert_eq!(first.kind, "plain");
    assert_eq!(first.parts.len(), 7);
    // Every key a group colours has its colour.
    assert_eq!(
        first.colours.keys().copied().collect::<Vec<_>>(),
        [90, 120, 150, 200]
    );
    // Each slot's roll is its own: the head rolls both heads, the hair
    // each of its three.
    let heads: BTreeSet<_> = looks.iter().map(|l| l.parts[1].clone()).collect();
    let hair: BTreeSet<_> = looks.iter().map(|l| l.parts[4].clone()).collect();
    assert_eq!(heads.len(), 2);
    assert_eq!(hair.len(), 3);
    // A slot no kind names is empty.
    assert!(
        looks
            .iter()
            .all(|l| l.parts[0].is_none() && l.parts[3].is_none())
    );
}

#[test]
fn a_slot_or_a_group_added_leaves_every_other_roll_as_it_was() {
    let set = example();
    let added = EXAMPLE
        .replacen(r#""eyes", "hair""#, r#""eyes", "earring", "hair""#, 1)
        .replacen("90: \"eyes\" }", "90: \"eyes\", 60: \"metal\" }", 1)
        .replacen(
            "colours: {",
            "colours: {\n        \"metal\": (keys: [\"metal\"], rows: [[\"#C0C0C0\"], [\"#D4AF37\"]]),",
            1,
        )
        .replacen(
            "parts: {",
            "parts: {\n        \"ring\": (slot: \"earring\", image: \"faces/gear.png\", at: (0, 0)),",
            1,
        )
        .replacen(r#""beard": [None, "beard_full"],"#, r#""beard": [None, "beard_full"], "earring": ["ring", None],"#, 1);
    let def = PartsDef::from_source("faces.ron", &added).unwrap();
    let more = def.resolve("faces.ron", "", &example_images()).unwrap();
    assert_eq!(more.slots.len(), 8);
    let mut rings = BTreeSet::new();
    for seed in 0..1000 {
        for kind in ["plain", "elf"] {
            let was = set.appearance(kind, seed, &[]).unwrap();
            let now = more.appearance(kind, seed, &[]).unwrap();
            for (slot, part) in set.slots.iter().zip(&was.parts) {
                let at = more.slots.iter().position(|s| s == slot).unwrap();
                assert_eq!(&now.parts[at], part, "seed {seed} slot {slot}");
            }
            for (key, colour) in &was.colours {
                assert_eq!(now.colours.get(key), Some(colour), "seed {seed} key {key}");
            }
            rings.insert(now.parts[4].clone());
        }
    }
    assert_eq!(rings.len(), 2);
}

#[test]
fn a_kind_with_a_base_rolls_the_bases_lists_and_its_own() {
    let set = example();
    for seed in 0..100 {
        let plain = set.appearance("plain", seed, &[]).unwrap();
        let elf = set.appearance("elf", seed, &[]).unwrap();
        assert_eq!(elf.parts[2].as_deref(), Some("ears_long"));
        assert_eq!(plain.parts[2], None);
        // The same streams: the same heads, hair, beards and colours.
        for slot in [1, 4, 5] {
            assert_eq!(elf.parts[slot], plain.parts[slot], "seed {seed}");
        }
        assert_eq!(elf.colours, plain.colours);
    }
}

#[test]
fn a_worn_part_takes_its_slot_and_hides_what_it_names() {
    let set = example();
    for seed in 0..50 {
        let look = set.appearance("elf", seed, &["helm_iron"]).unwrap();
        assert_eq!(look.parts[6].as_deref(), Some("helm_iron"));
        // Hair, back hair and ears emptied; the head and beard rolled.
        assert_eq!(
            (&look.parts[0], &look.parts[2], &look.parts[4]),
            (&None, &None, &None)
        );
        let bare = set.appearance("elf", seed, &[]).unwrap();
        assert_eq!(
            (&look.parts[1], &look.parts[5]),
            (&bare.parts[1], &bare.parts[5])
        );
        // A worn part replaces the roll.
        let long = set.appearance("plain", seed, &["hair_long"]).unwrap();
        assert_eq!(long.parts[4].as_deref(), Some("hair_long"));
    }
    assert_eq!(
        set.appearance("plain", 0, &["crown"]),
        Err(PartsError::UnknownPart("crown".into()))
    );
    let two = set.appearance("plain", 0, &["hair_short", "hair_long"]);
    assert_eq!(
        two,
        Err(PartsError::TwoInOneSlot {
            slot: "hair".into(),
            first: "hair_short".into(),
            second: "hair_long".into(),
        })
    );
    assert_eq!(
        two.unwrap_err().to_string(),
        "\"hair_short\" and \"hair_long\" are both worn in slot \"hair\""
    );
    // The same part twice is worn once.
    assert!(
        set.appearance("plain", 0, &["helm_iron", "helm_iron"])
            .is_ok()
    );
    assert_eq!(
        set.appearance("orc", 0, &[]).unwrap_err().to_string(),
        "the part set has no kind \"orc\""
    );
}

#[test]
fn a_hidden_parts_own_hides_do_not_count() {
    let source = r#"(
        size: (8, 8),
        slots: ["a", "b", "c"],
        keys: {},
        parts: {
            "a1": (slot: "a", image: "x.png", at: (0, 0)),
            "b1": (slot: "b", image: "x.png", at: (0, 0), hides: ["a"]),
            "c1": (slot: "c", image: "x.png", at: (0, 0), hides: ["b"]),
        },
        kinds: { "k": (slots: { "a": ["a1"] }) },
    )"#;
    let images = ImageTable::new([("x.png", ImageInfo::new(8, 8))].into());
    let set = PartsDef::from_source("x.ron", source)
        .unwrap()
        .resolve("x.ron", "", &images)
        .unwrap();
    let look = set.appearance("k", 3, &["b1", "c1"]).unwrap();
    assert_eq!(look.parts, [Some("a1".into()), None, Some("c1".into())]);
    // Shown, `b1` hides `a1`.
    let look = set.appearance("k", 3, &["b1"]).unwrap();
    assert_eq!(look.parts, [None, Some("b1".into()), None]);
    assert!(look.colours.is_empty());
}

#[test]
fn a_grey_near_a_key_but_not_on_it_is_reported_with_its_file_and_pixel() {
    let source = r#"(
        size: (4, 4),
        slots: ["head"],
        keys: { 200: "skin" },
        parts: { "head": (slot: "head", image: "x.png", at: (4, 0)) },
        kinds: { "k": (slots: { "head": ["head"] }) },
    )"#;
    let images = ImageTable::new([("x.png", ImageInfo::new(12, 4))].into());
    let set = PartsDef::from_source("x.ron", source)
        .unwrap()
        .resolve("x.ron", "", &images)
        .unwrap();
    let mut image = Image::filled(12, 4, [200, 200, 200, 255]);
    let mut put = |(x, y): (u32, u32), pixel: [u8; 4]| {
        let i = ((y * 12 + x) * 4) as usize;
        image.rgba[i..i + 4].copy_from_slice(&pixel);
    };
    put((5, 1), [198, 198, 198, 255]);
    put((6, 1), [190, 190, 190, 255]);
    put((7, 2), [204, 204, 204, 255]);
    // Drawn from alpha 128.
    put((4, 3), [198, 198, 198, 128]);
    // Not grey, clear, or outside the part (left of it or right of it):
    // never reported.
    put((5, 2), [198, 199, 198, 255]);
    put((5, 3), [198, 198, 199, 255]);
    put((6, 2), [198, 198, 198, 127]);
    put((1, 1), [198, 198, 198, 255]);
    put((9, 1), [198, 198, 198, 255]);
    let errors: Vec<String> = set
        .check_pixels(|_| Some(&image))
        .iter()
        .map(ToString::to_string)
        .collect();
    assert_eq!(
        errors,
        [
            "x.png: pixel (5, 1): grey 198 is 2 from key 200 (\"skin\") but not on it",
            "x.png: pixel (7, 2): grey 204 is 4 from key 200 (\"skin\") but not on it",
            "x.png: pixel (4, 3): grey 198 is 2 from key 200 (\"skin\") but not on it",
        ]
    );
    // An image with no pixels, or too few.
    let missing: Vec<String> = set
        .check_pixels(|_| None)
        .iter()
        .map(ToString::to_string)
        .collect();
    assert_eq!(
        missing,
        ["x.png: part \"head\": the image's pixels aren't there"]
    );
    let small = Image::filled(6, 4, [0; 4]);
    assert_eq!(set.check_pixels(|_| Some(&small)).len(), 1);
    // An image whose bytes are fewer than its size says.
    let mut short = Image::filled(12, 4, [0; 4]);
    short.rgba.pop();
    assert_eq!(set.check_pixels(|_| Some(&short)).len(), 1);
    // The fixture's greys are all on their keys.
    let faces_image = faces_image();
    assert_eq!(faces().check_pixels(|_| Some(&faces_image)), []);
}

#[test]
fn an_appearance_is_plain_data_with_a_fingerprint() {
    let set = faces();
    let look = set.appearance("capped", 7, &[]).unwrap();
    let text = ::ron::to_string(&look).unwrap();
    let back: Appearance = ::ron::from_str(&text).unwrap();
    assert_eq!(back, look);
    assert_eq!(back.fingerprint(), look.fingerprint());
    let mut other = look.clone();
    other.colours.insert(200, [0, 0, 0]);
    assert_ne!(other.fingerprint(), look.fingerprint());
    let none = Appearance::new("capped".into(), vec![None; 4], BTreeMap::new());
    assert_ne!(none.fingerprint(), look.fingerprint());
    // The hat hides the hair.
    assert_eq!(look.parts[3].as_deref(), Some("hat_cap"));
    assert_eq!(look.parts[2], None);
}

#[test]
fn golden_a_part_set_and_a_look_are_the_same_on_every_platform() {
    let mut golden = crate::golden!("tests/golden/parts.ron");
    let def = PartsDef::from_source("parts/faces.ron", FACES).unwrap();
    golden.check("part set: parts", state_hash(&def).unwrap());
    let look = faces().appearance("plain", 7, &[]).unwrap();
    golden.check("appearance", format!("{:016x}", look.fingerprint()));
    golden.assert();
}

/// A set of `lists` (a slot each, with that many parts, `None` among
/// them when the flag says) and a colour group of `rows` rows.
fn generated(lists: &[(u8, bool)], rows: u8, extra: Option<(usize, u8)>) -> PartSet {
    let mut slots: Vec<String> = (0..lists.len()).map(|i| format!("s{i}")).collect();
    let mut all: Vec<(String, u8, bool)> = slots
        .iter()
        .cloned()
        .zip(lists.iter().copied())
        .map(|(s, (n, e))| (s, n, e))
        .collect();
    if let Some((at, n)) = extra {
        let at = at % (slots.len() + 1);
        slots.insert(at, "new".into());
        all.insert(at, ("new".into(), n, false));
    }
    let mut parts = BTreeMap::new();
    let mut lists = BTreeMap::new();
    for (slot, n, empty) in &all {
        let mut list: Vec<Option<String>> = (0..*n)
            .map(|i| {
                let name = format!("{slot}p{i}");
                parts.insert(
                    name.clone(),
                    PartDef {
                        slot: slot.clone(),
                        image: "x.png".into(),
                        at: (0, 0),
                        hides: Vec::new(),
                    },
                );
                Some(name)
            })
            .collect();
        if *empty {
            list.push(None);
        }
        lists.insert(slot.clone(), list);
    }
    let row = |i: u8| vec![format!("#{i:02x}0000")];
    let mut colours = BTreeMap::from([(
        "g".to_owned(),
        ColourGroupDef {
            keys: vec!["k".into()],
            rows: (0..rows).map(row).collect(),
        },
    )]);
    let mut keys = vec![(200, "k".to_owned())];
    if extra.is_some() {
        keys.push((100, "new".to_owned()));
        let rows = (0..3).map(row).collect();
        colours.insert(
            "a".into(),
            ColourGroupDef {
                keys: vec!["new".into()],
                rows,
            },
        );
    }
    let kind = KindDef {
        base: None,
        slots: lists,
        colours: BTreeMap::new(),
    };
    let def = PartsDef {
        size: (8, 8),
        slots,
        keys,
        colours,
        parts,
        kinds: BTreeMap::from([("k".to_owned(), kind)]),
    };
    assert_eq!(check::problems(&def, "x.ron", ""), []);
    let images = ImageTable::new([("x.png", ImageInfo::new(8, 8))].into());
    def.resolve("x.ron", "", &images).unwrap()
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    #[test]
    fn a_slot_and_a_group_added_to_any_set_change_no_other_roll(
        lists in proptest::collection::vec((1..6_u8, any::<bool>()), 1..8),
        rows in 1..6_u8,
        at in any::<usize>(),
        n in 1..6_u8,
        seed in any::<u64>(),
    ) {
        let set = generated(&lists, rows, None);
        let more = generated(&lists, rows, Some((at, n)));
        let was = set.appearance("k", seed, &[]).unwrap();
        let now = more.appearance("k", seed, &[]).unwrap();
        let kept: Vec<_> = more.slots.iter().zip(&now.parts).filter(|(s, _)| *s != "new").map(|(_, p)| p.clone()).collect();
        prop_assert_eq!(kept, was.parts);
        prop_assert_eq!(now.colours.get(&200), was.colours.get(&200));
    }

    #[test]
    fn no_seed_or_worn_part_panics_a_roll(seed in any::<u64>(), worn in 0..9_usize) {
        let set = faces();
        let names: Vec<&str> = set.parts.keys().map(String::as_str).chain(["nothing", "hat_cap"]).collect();
        for kind in ["plain", "capped", "none"] {
            let _ = set.appearance(kind, seed, &[]);
            let _ = set.appearance(kind, seed, &names[worn..=worn]);
        }
    }

    #[test]
    fn no_bytes_panic_the_parts_format(
        random in bytes(),
        text in "[ -~\n]{0,300}",
        faces in edited(FACES),
        example in edited(EXAMPLE),
    ) {
        let mut images = example_images();
        images.images.insert("faces.png", ImageInfo::new(56, 8));
        let path = AssetPath::new("faces.ron");
        for bytes in [random.as_slice(), text.as_bytes(), faces.as_bytes(), example.as_bytes()] {
            // What the file alone passes resolves and rolls without a panic too.
            if let Ok(def) = PartsRon.decode(&path, bytes, &DecodeCx::new())
                && let Ok(set) = def.resolve("faces.ron", "", &images)
            {
                for kind in set.kinds() {
                    let _ = set.appearance(kind, 7, &["helm_iron"]);
                }
                let _ = set.check_pixels(|_| None);
            }
        }
    }
}
