//! The pack's tests (ADR-0023): every kind of the built-in formats'
//! fixtures reads the same from a pack as from its source; a pack holds
//! what was written in it; a pack of another version or stamp, cut short
//! or changed, is refused and never panics; and the binary form of every
//! engine kind is pinned, so a change to one makes [`VERSION`] the next
//! number.
use std::fmt::Write as _;
use std::sync::OnceLock;

use proptest::prelude::*;

use super::*;
use crate::audio::{AudioManifest, MusicTrack, Sound};
use crate::bundle::Bundle;
use crate::cinematic::{CinematicFile, CinematicRon};
use crate::credits::CreditsFile;
use crate::dialogue::tests::Cue;
use crate::dialogue::{DialogueFile, Dlg};
use crate::font::FontAtlasDef;
use crate::format::builtin_tests::{
    ATLAS, CREDITS, ENGLISH_UI, FILM, LIGHTS, MANIFEST, MAP, NAMES, PACK_INFO, PACK_UI, PALETTE,
    PATTERN_MAP, SCRIPT, Shot, TABLE, Table, bundle, lang_pack, map_text, ogg_file, png_file, wav,
};
use crate::format::{Formats, RegisterError};
use crate::grid_map::{GridMap, NoFeature};
use crate::image::{Image, ImageInfo, Png, png_header};
use crate::keymap::{KeymapDef, KeymapRon, sample};
use crate::lang::LangPack;
use crate::lighting::Lighting;
use crate::names::Names;
use crate::onebit::PatternMapDef;
use crate::palette::PaletteDef;
use crate::ron::Ron;
use crate::target::TargetProfile;

/// The stamp the tests bake with.
const STAMP: &str = "test 1";

/// The engine's Playdate profile.
const PLAYDATE: &str = include_str!("../../../../targets/playdate.ron");

/// A game of one file of every kind the built-in formats read, and the
/// files they name.
fn sources() -> &'static Bundle {
    static SOURCES: OnceLock<&'static Bundle> = OnceLock::new();
    SOURCES.get_or_init(|| {
        bundle(vec![
            ("images/card.png", png_file([1, 2, 3, 255, 4, 5, 6, 255])),
            ("audio/audio.ron", MANIFEST.into()),
            ("audio/sfx/tick.wav", wav(1, crate::audio::SAMPLE_RATE, 1)),
            ("audio/sfx/gone.wav", wav(1, crate::audio::SAMPLE_RATE, 1)),
            ("audio/sfx/trim.wav", wav(2, crate::audio::SAMPLE_RATE, 1)),
            (
                "music/title.ogg",
                ogg_file(crate::audio::SAMPLE_RATE, 88_200),
            ),
            ("data/credits.ron", CREDITS.into()),
            ("fonts/atlas.ron", ATLAS.into()),
            ("fonts/atlas.png", png_header(128, 16)),
            ("data/palette.ron", PALETTE.into()),
            ("data/palette@onebit.ron", PATTERN_MAP.into()),
            ("data/lights.ron", LIGHTS.into()),
            ("data/names.ron", NAMES.into()),
            ("dialogue/gate.dlg", SCRIPT.into()),
            ("maps/field.map", MAP.into()),
            ("data/keymap.ron", sample::SOURCE.into()),
            ("films/title.ron", FILM.into()),
            ("lang/en/ui.ron", ENGLISH_UI.into()),
            ("lang/xx/lang.ron", PACK_INFO.into()),
            ("lang/xx/ui.ron", PACK_UI.into()),
            ("data/table.ron", TABLE.into()),
            ("targets/playdate.ron", PLAYDATE.into()),
            ("readme.txt", b"kept as it is".to_vec()),
        ])
    })
}

/// The tests' game's formats: the standard ones, with pixels, and its own.
fn formats() -> Formats {
    let mut formats = Formats::standard();
    let own: [Result<(), RegisterError>; 7] = [
        formats.register(Png).or(Ok(())),
        formats.replace(Dlg::<Cue>::new()).map(drop),
        formats.register(map_text()),
        formats.register(KeymapRon::<sample::Action>::new()),
        formats.register(CinematicRon::<Shot, ()>::new()),
        formats.register(lang_pack()),
        formats.register(Ron::<Table>::new()),
    ];
    assert!(own.iter().all(Result::is_ok), "{own:?}");
    formats
}

/// The tests' game's bake: every kind, each asset as its loader asks for
/// it.
fn every_kind() -> Bake {
    Bake::new(STAMP)
        .under::<ImageInfo>("images")
        .under::<Image>("images")
        .under::<Sound>("audio/sfx")
        .under::<MusicTrack>("music")
        .asset::<AudioManifest>("audio/audio")
        .asset::<CreditsFile>("data/credits")
        .asset::<FontAtlasDef>("fonts/atlas.ron")
        .asset::<PaletteDef>("data/palette")
        .asset::<PatternMapDef>("data/palette@onebit")
        .asset::<Lighting>("data/lights")
        .asset::<Names>("data/names")
        .under::<DialogueFile<Cue>>("dialogue")
        .under::<GridMap<u8, NoFeature>>("maps")
        .asset::<KeymapDef<sample::Action>>("data/keymap")
        .asset::<CinematicFile<Shot, ()>>("films/title")
        .asset::<LangPack>("lang/xx/lang")
        .asset::<Table>("data/table")
        .asset::<TargetProfile>("targets/playdate")
}

/// The pack of the tests' game, baked once.
fn baked() -> &'static Baked {
    static BAKED: OnceLock<Baked> = OnceLock::new();
    BAKED.get_or_init(|| {
        every_kind()
            .bake(&formats(), sources(), Some("desk"))
            .unwrap_or_else(|e| panic!("{e:?}"))
    })
}

/// A bundle of `pack` alone.
fn pack_bundle(pack: Pack<'static>) -> Bundle {
    Bundle::new(vec![pack.into_layer("pack")])
}

/// The formats of the tests' game, with the pack's.
fn pack_formats(bake: &Bake) -> Formats {
    let mut formats = formats();
    bake.register(&mut formats)
        .unwrap_or_else(|e| panic!("{e}"));
    formats
}

#[test]
fn every_fixture_kind_reads_the_same_from_the_pack_as_from_its_source() {
    let bake = every_kind();
    let formats = pack_formats(&bake);
    let pack = Pack::open(&baked().bytes, STAMP).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(pack.info().target.as_deref(), Some("desk"));
    let pack = pack_bundle(pack);
    assert_eq!(bake.check(&formats, sources(), &pack), []);
    assert_eq!(bake.load(&formats, sources()), []);
    assert_eq!(bake.load(&formats, &pack), []);
    let unbaked = bundle(vec![("data/names.ron", b"[".to_vec())]);
    // One error for each asset asked for by id: eleven missing, one broken.
    assert_eq!(bake.load(&formats, unbaked).len(), 12);
    // Each kind, by hand, read through the registry as a loader reads it.
    let palette = formats.load::<PaletteDef>(&pack, "data/palette").unwrap();
    assert_eq!(palette.colors.get("grass"), Some(&[0, 255, 0]));
    let film = formats
        .load::<CinematicFile<Shot, ()>>(&pack, "films/title")
        .unwrap();
    assert_eq!(film.shots.len(), 2);
    let scenes = formats
        .load_under::<DialogueFile<Cue>>(&pack, "dialogue")
        .unwrap();
    assert_eq!(scenes["dialogue/gate.dlg"].scenes[0].scene.id, "gate");
    let image = formats.load::<Image>(&pack, "images/card").unwrap();
    assert_eq!(image.rgba, [1, 2, 3, 255, 4, 5, 6, 255]);
    // The kinds go in the file of each, and every other file as it was.
    let kinds = |path: &str| {
        let baked_file = baked().files.iter().find(|f| f.path == path).unwrap();
        baked_file.kinds.clone()
    };
    assert_eq!(kinds("images/card.png"), ["image", "image pixels"]);
    assert_eq!(kinds("fonts/atlas.png"), Vec::<&str>::new());
    assert_eq!(pack.bytes("readme.txt"), Some(&b"kept as it is"[..]));
    assert_eq!(baked().files.len(), sources().files_under("").len());
    assert_eq!(
        baked().source_bytes(),
        sources()
            .files_under("")
            .iter()
            .map(|path| sources().bytes(path).unwrap().len())
            .sum::<usize>()
    );
}

#[test]
fn a_pack_under_loose_files_gives_way_to_them() {
    static LOOSE: &[(&str, &[u8])] = &[("data/names.ron", br#"{"hero": "Bea"}"#)];
    let bake = every_kind();
    let formats = pack_formats(&bake);
    let pack = Pack::open(&baked().bytes, STAMP)
        .unwrap()
        .into_layer("pack");
    let both = Bundle::new(vec![Layer::table("loose", LOOSE), pack]);
    let names = formats.load::<Names>(&both, "data/names").unwrap();
    assert_eq!(names.names["hero"], "Bea");
    let palette = formats.load::<PaletteDef>(&both, "data/palette").unwrap();
    assert_eq!(
        both.display_path("data/palette.ron"),
        "pack/data/palette.ron"
    );
    assert_eq!(palette.colors.len(), 2);
}

#[test]
fn the_check_finds_every_way_a_pack_differs_from_its_sources() {
    let bake = every_kind();
    let formats = pack_formats(&bake);
    let pack = pack_bundle(Pack::open(&baked().bytes, STAMP).unwrap());
    let mut files: Vec<(&'static str, Vec<u8>)> = sources()
        .files_under("")
        .into_iter()
        .filter(|path| !matches!(*path, "data/table.ron" | "readme.txt"))
        .map(|path| (path, sources().bytes(path).unwrap().to_vec()))
        .collect();
    for (path, bytes) in &mut files {
        match *path {
            "data/palette.ron" => {
                *bytes = br##"{"black": "#000000", "grass": "#00ff01"}"##.to_vec();
            }
            "fonts/atlas.png" => *bytes = png_header(128, 32),
            _ => {}
        }
    }
    files.push(("data/table.ron", b"(rows: [])".to_vec()));
    files.push(("extra.txt", b"new".to_vec()));
    let changed = bundle(files);
    let errors: Vec<String> = bake
        .check(&formats, changed, &pack)
        .iter()
        .map(ToString::to_string)
        .collect();
    assert_eq!(
        errors,
        [
            "pack/data/palette.ron: the palette read from the pack differs from the one read from \
             its source",
            "pack/data/table.ron: the table read from the pack differs from the one read from its \
             source",
            "pack/extra.txt: the pack lacks this file",
            // The atlas's picture is read raw by the platform, and by the
            // atlas's format beside it: its source changed.
            "pack/fonts/atlas.png: the pack's copy differs from the file",
            "pack/readme.txt: the pack has this file and the sources don't",
        ]
    );
    assert!(
        bake.check(&formats, changed, &pack)
            .iter()
            .all(|e| e.code == Some(crate::error::codes::PACK_DIFFERS))
    );
}

#[test]
fn the_check_says_when_the_packs_formats_are_missing() {
    let bake = Bake::new(STAMP).asset::<Names>("data/names");
    let baked = bake.bake(&formats(), sources(), None).unwrap();
    let pack = pack_bundle(Pack::open(&baked.bytes, STAMP).unwrap().into_owned());
    let errors = bake.check(&formats(), sources(), &pack);
    assert_eq!(
        errors[0].to_string(),
        "pack/data/names.ron: no format reads a baked names table: register the pack's formats \
         (Bake::register)"
    );
    // Without them, the names' own format can't read the baked file.
    assert!(formats().load::<Names>(&pack, "data/names").is_err());
}

#[test]
fn a_source_with_errors_is_refused_with_every_error() {
    let files = bundle(vec![
        ("data/palette.ron", b"{".to_vec()),
        ("data/names.ron", b"[".to_vec()),
    ]);
    let bake = Bake::new(STAMP)
        .asset::<PaletteDef>("data/palette")
        .asset::<Names>("data/names")
        .asset::<Lighting>("data/lights");
    let errors = bake.bake(&formats(), files, None).unwrap_err();
    let errors: Vec<String> = errors.iter().map(ToString::to_string).collect();
    assert_eq!(errors.len(), 3, "{errors:?}");
    assert!(
        errors[0].starts_with("base/data/lights: file not found"),
        "{errors:?}"
    );
    assert!(errors[1].starts_with("base/data/names.ron:1"), "{errors:?}");
    assert!(
        errors[2].starts_with("base/data/palette.ron:1"),
        "{errors:?}"
    );
}

#[test]
fn two_kinds_of_one_name_cant_be_baked_together() {
    let bake = Bake::new(STAMP)
        .asset::<DialogueFile<Cue>>("dialogue/gate.dlg")
        .asset::<DialogueFile>("dialogue/gate.dlg");
    assert_eq!(bake.kinds(), ["dialogue"]);
    let errors = bake.bake(&formats(), sources(), None).unwrap_err();
    assert_eq!(
        errors[0].to_string(),
        "bake: the bake has two kinds called \"dialogue\": a pack tells kinds by name"
    );
}

#[test]
fn a_kind_named_twice_is_baked_and_registered_once() {
    let bake = Bake::new(STAMP)
        .asset::<Names>("data/names")
        .asset::<Names>("data/names.ron");
    let mut formats = formats();
    bake.register(&mut formats).unwrap();
    bake.register(&mut formats).unwrap();
    assert_eq!(formats.ids::<Names>(), ["names", "baked"]);
    let baked = bake.bake(&formats, sources(), None).unwrap();
    let names = baked
        .files
        .iter()
        .find(|f| f.path == "data/names.ron")
        .unwrap();
    assert_eq!(names.kinds, ["names table"]);
    let debug = format!("{bake:?}");
    assert!(debug.contains("stamp: \"test 1\""), "{debug}");
    assert_eq!(bake.stamp(), STAMP);
}

#[test]
fn a_baked_file_says_which_kinds_it_holds() {
    let bake = Bake::new(STAMP).asset::<Names>("data/names");
    let baked = bake.bake(&formats(), sources(), None).unwrap();
    let pack = Pack::open(&baked.bytes, STAMP).unwrap();
    let bytes = pack.get("data/names.ron").unwrap();
    let file = AssetPath::new("data/names.ron");
    let cx = DecodeCx::new();
    let error = Packed::<PaletteDef>::new()
        .decode(&file, bytes, &cx)
        .unwrap_err();
    assert_eq!(
        error[0].to_string(),
        "data/names.ron: baked as names table and not as a palette: bake the palette kind too"
    );
    let wrong = Packed::<Lighting>::new().decode(&file, bytes, &cx);
    assert!(wrong.is_err(), "names don't read as lights");
    let mut cut = bytes.to_vec();
    cut.pop();
    assert_eq!(
        Packed::<Names>::new().decode(&file, &cut, &cx).unwrap_err()[0].to_string(),
        "data/names.ron: a baked file that is broken"
    );
    let format = Packed::<Names>::default();
    assert_eq!(format!("{format:?}"), "Packed");
    assert_eq!(format.extensions(), [] as [&str; 0]);
    assert_eq!(format.max_bytes(), MAX_BAKED_BYTES);
    assert!(format.sniff(bytes));
    assert!(!format.sniff(b"{\"hero\": \"Ana\"}"));
}

#[test]
fn a_baked_kind_whose_shape_changed_is_refused_with_a_message_to_bake_again() {
    let mut sections = BTreeMap::new();
    sections.insert("names table", vec![1, 0xff]);
    let bytes = baked_file(&sections);
    let error = Packed::<Names>::new()
        .decode(&AssetPath::new("n.ron"), &bytes, &DecodeCx::new())
        .unwrap_err();
    assert_eq!(
        error[0].to_string(),
        "n.ron: its baked names table doesn't read as this game's: bake it again with `shuyi bake`"
    );
    // Bytes left over are as wrong as bytes missing.
    let mut sections = BTreeMap::new();
    sections.insert("names table", vec![0, 0]);
    let bytes = baked_file(&sections);
    assert!(
        Packed::<Names>::new()
            .decode(&AssetPath::new("n.ron"), &bytes, &DecodeCx::new())
            .is_err()
    );
}

/// The pack of `files` (path → raw bytes), with `info`.
fn raw_pack(info: &PackInfo, files: &[(String, Vec<u8>)]) -> Vec<u8> {
    let files = files
        .iter()
        .map(|(path, bytes)| (path.clone(), Content::Raw(bytes.clone())))
        .collect();
    write(info, &files)
}

/// The info of the tests' raw packs.
fn test_info() -> PackInfo {
    PackInfo {
        stamp: STAMP.to_owned(),
        target: None,
    }
}

#[test]
fn a_pack_of_another_version_or_stamp_is_refused_with_a_message_to_bake_again() {
    let bytes = raw_pack(&test_info(), &[("a.txt".to_owned(), b"a".to_vec())]);
    assert_eq!(info(&bytes), Ok(test_info()));
    let mut old = bytes.clone();
    old[8..12].copy_from_slice(&0_u32.to_le_bytes());
    assert_eq!(
        Pack::open(&old, STAMP),
        Err(PackError::Version { found: 0 })
    );
    assert_eq!(
        PackError::Version { found: 0 }.to_string(),
        "the pack is of pack format version 0 and this engine reads version 2: bake it again \
         with `shuyi bake`"
    );
    let error = Pack::open(&bytes, "test 2").unwrap_err();
    assert_eq!(
        error.to_string(),
        "the pack was baked for \"test 1\" and this game is \"test 2\": bake it again with \
         `shuyi bake`"
    );
    assert_eq!(Pack::open(b"PNG", STAMP), Err(PackError::NotAPack));
    assert_eq!(Pack::open(b"SHUYI", STAMP), Err(PackError::Truncated));
    assert_eq!(Pack::open(b"", STAMP), Err(PackError::Truncated));
    assert_eq!(info(b"nope"), Err(PackError::NotAPack));
}

#[test]
fn a_pack_keeps_a_file_as_it_is_when_compressing_doesnt_make_it_smaller() {
    let repeated = vec![7; 4096];
    let noise: Vec<u8> = (0..64_u8).map(|i| i.wrapping_mul(73) ^ 0x5a).collect();
    let files = [
        ("a.bin".to_owned(), repeated.clone()),
        ("b.bin".to_owned(), noise.clone()),
        ("c.bin".to_owned(), Vec::new()),
    ];
    let bytes = raw_pack(&test_info(), &files);
    assert!(bytes.len() < 4096, "{}", bytes.len());
    let pack = Pack::open(&bytes, STAMP).unwrap();
    let kept: Vec<(&str, usize, usize)> = pack
        .files()
        .iter()
        .map(|f| (f.path.as_str(), f.stored, f.bytes.len()))
        .collect();
    assert!(kept[0].1 < 100, "{kept:?}");
    assert_eq!(kept[1..], [("b.bin", 64, 64), ("c.bin", 0, 0)]);
    assert_eq!(pack.get("a.bin"), Some(&repeated[..]));
    assert_eq!(pack.get("b.bin"), Some(&noise[..]));
    assert_eq!(pack.get("d.bin"), None);
    let got = &pack.files()[0].kinds;
    assert!(got.is_empty(), "{got:?}");
}

/// `bytes`, its body changed by `change`, with the checksum made right
/// again: what reaches the index and the files' checks.
fn resummed(bytes: &[u8], change: impl FnOnce(&mut Vec<u8>)) -> Vec<u8> {
    let mut bytes = bytes.to_vec();
    change(&mut bytes);
    let sum = checksum(bytes.get(HEADER..).unwrap_or_default());
    if let Some(at) = bytes.get_mut(12..20) {
        at.copy_from_slice(&sum.to_le_bytes());
    }
    bytes
}

/// The pack of `index` and `data`, with a right header.
fn assembled(index: &Index, data: &[u8]) -> Vec<u8> {
    let index = postcard::to_allocvec(index).unwrap();
    let body = [index.as_slice(), data].concat();
    let length = u32::try_from(index.len()).unwrap();
    [
        &MAGIC[..],
        &VERSION.to_le_bytes(),
        &checksum(&body).to_le_bytes(),
        &length.to_le_bytes(),
        &body,
    ]
    .concat()
}

/// An entry of `stored` bytes at `offset`, of `size` unpacked.
fn entry(path: &str, codec: Codec, offset: u64, stored: u64, size: u64) -> Entry {
    Entry {
        path: path.to_owned(),
        kinds: Vec::new(),
        codec,
        offset,
        stored,
        size,
    }
}

/// What opening a pack of `entries` over `data` says, as text.
fn opened(entries: Vec<Entry>, data: &[u8]) -> String {
    let index = Index {
        info: test_info(),
        entries,
    };
    match Pack::open(&assembled(&index, data), STAMP) {
        Ok(pack) => format!("ok: {} file(s)", pack.files().len()),
        Err(e) => e.to_string(),
    }
}

#[test]
fn an_index_that_doesnt_hold_together_is_refused() {
    let stored = Codec::Stored;
    assert_eq!(
        opened(vec![entry("a", stored, 0, 2, 2)], b"ab"),
        "ok: 1 file(s)"
    );
    assert_eq!(
        opened(vec![entry("a", stored, 1, 1, 1)], b"ab"),
        "the pack is broken: a starts at 1 and the file before it ends at 0"
    );
    assert_eq!(
        opened(vec![entry("a", stored, 0, 3, 3)], b"ab"),
        "the pack is broken: a goes past the end of the pack"
    );
    assert_eq!(
        opened(vec![entry("a", stored, 0, 1, 2)], b"a"),
        "the pack is broken: a is stored with two sizes"
    );
    assert_eq!(
        opened(vec![entry("a", stored, 0, 1, 1)], b"ab"),
        "the pack is broken: bytes follow its last file"
    );
    assert_eq!(
        opened(
            vec![entry("b", stored, 0, 1, 1), entry("a", stored, 1, 1, 1)],
            b"ab"
        ),
        "the pack is broken: a is out of path order"
    );
    assert_eq!(
        opened(vec![entry("a", Codec::Lz4, 0, 1, 1000)], b"a"),
        "the pack is broken: a claims 1000 bytes from 1 stored"
    );
    assert_eq!(
        opened(vec![entry("a", Codec::Lz4, 0, 2, 3)], b"\xff\xff"),
        "the pack is broken: a doesn't unpack"
    );
    assert_eq!(
        opened(vec![entry("a", stored, 0, 0, MAX_UNPACKED_BYTES + 1)], b""),
        format!("the pack is broken: its files add up to more than {MAX_UNPACKED_BYTES} bytes")
    );
    let mut baked = entry("a", stored, 0, 2, 2);
    baked.kinds = vec!["palette".to_owned()];
    assert_eq!(
        opened(vec![baked], b"ab"),
        "the pack is broken: a isn't a whole baked file"
    );
    let many = (0..=MAX_FILES)
        .map(|i| entry(&format!("{i:06}"), stored, 0, 0, 0))
        .collect();
    assert_eq!(
        opened(many, b""),
        format!("the pack is broken: it has more than {MAX_FILES} files")
    );
    let bytes = raw_pack(&test_info(), &[]);
    let unreadable = resummed(&bytes, |b| b.truncate(HEADER));
    let unreadable = resummed(&unreadable, |b| b.push(0xff));
    let mut unreadable = unreadable;
    unreadable[20..24].copy_from_slice(&1_u32.to_le_bytes());
    let unreadable = resummed(&unreadable, |_| {});
    assert_eq!(
        Pack::open(&unreadable, STAMP).unwrap_err().to_string(),
        "the pack is broken: its index doesn't read"
    );
    let mut long = bytes.clone();
    long[20..24].copy_from_slice(&u32::MAX.to_le_bytes());
    assert_eq!(Pack::open(&long, STAMP), Err(PackError::Truncated));
}

/// Each byte of `asset`'s binary form, as hex, after its kind's name.
fn binary_form<K: Bakeable>(out: &mut String, asset: &K) {
    let bytes = postcard::to_allocvec(asset).unwrap();
    let back: K = postcard::from_bytes(&bytes).unwrap();
    assert_eq!(&back, asset, "{} round-trips", K::NAME);
    let _ = write!(out, "{}:", K::NAME);
    for (i, b) in bytes.iter().enumerate() {
        if i % 32 == 0 {
            out.push_str("\n   ");
        }
        let _ = write!(out, " {b:02x}");
    }
    out.push('\n');
}

#[test]
fn the_binary_form_of_every_engine_kind_is_pinned_by_the_pack_version() {
    // A change here changes what a pack holds: make `VERSION` the next
    // number, then accept the new snapshot.
    let formats = formats();
    let bundle = sources();
    let mut out = format!("pack format version {VERSION}\n");
    macro_rules! form {
        ($kind:ty, $id:expr) => {
            binary_form(&mut out, &formats.load::<$kind>(bundle, $id).unwrap())
        };
    }
    form!(ImageInfo, "images/card.png");
    form!(Image, "images/card.png");
    form!(Sound, "audio/sfx/tick.wav");
    form!(MusicTrack, "music/title.ogg");
    form!(AudioManifest, "audio/audio.ron");
    form!(CreditsFile, "data/credits.ron");
    form!(FontAtlasDef, "fonts/atlas.ron");
    form!(PaletteDef, "data/palette.ron");
    form!(PatternMapDef, "data/palette@onebit.ron");
    form!(Lighting, "data/lights.ron");
    form!(Names, "data/names.ron");
    form!(DialogueFile<Cue>, "dialogue/gate.dlg");
    form!(GridMap<u8, NoFeature>, "maps/field.map");
    form!(KeymapDef<sample::Action>, "data/keymap.ron");
    form!(CinematicFile<Shot, ()>, "films/title.ron");
    form!(LangPack, "lang/xx/lang.ron");
    form!(TargetProfile, "targets/playdate.ron");
    insta::assert_snapshot!(out);
}

#[test]
fn baking_is_deterministic() {
    let again = every_kind()
        .bake(&formats(), sources(), Some("desk"))
        .unwrap();
    assert_eq!(again, *baked());
}

/// Paths and bytes for a pack: distinct paths in order.
fn pack_files() -> impl Strategy<Value = Vec<(String, Vec<u8>)>> {
    proptest::collection::btree_map(
        "[a-z]{1,8}(/[a-z]{1,8}){0,2}\\.[a-z]{1,3}",
        prop_oneof![
            proptest::collection::vec(any::<u8>(), 0..300),
            (any::<u8>(), 0..2000_usize).prop_map(|(b, n)| vec![b; n]),
        ],
        0..12,
    )
    .prop_map(|files| files.into_iter().collect())
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    #[test]
    fn a_pack_holds_what_was_written_in_it(
        files in pack_files(),
        stamp in "[ -~]{0,20}",
        target in proptest::option::of("[a-z]{1,10}"),
    ) {
        let info = PackInfo { stamp: stamp.clone(), target };
        let bytes = raw_pack(&info, &files);
        let pack = Pack::open(&bytes, &stamp).unwrap();
        prop_assert_eq!(pack.info(), &info);
        let read: Vec<(String, Vec<u8>)> = pack
            .files()
            .iter()
            .map(|f| (f.path.clone(), f.bytes.to_vec()))
            .collect();
        prop_assert_eq!(&read, &files);
        let bundle = pack_bundle(pack.into_owned());
        for (path, bytes) in &files {
            prop_assert_eq!(bundle.bytes(path), Some(bytes.as_slice()));
        }
    }

    #[test]
    fn a_pack_cut_short_is_refused(at in any::<prop::sample::Index>()) {
        let bytes = &baked().bytes;
        prop_assert!(Pack::open(&bytes[..at.index(bytes.len())], STAMP).is_err());
    }

    #[test]
    fn a_pack_with_a_byte_changed_is_refused(at in any::<prop::sample::Index>(), by in 1..=255_u8) {
        let mut bytes = baked().bytes.clone();
        let at = at.index(bytes.len());
        bytes[at] ^= by;
        prop_assert!(Pack::open(&bytes, STAMP).is_err());
    }

    #[test]
    fn a_pack_changed_under_a_right_checksum_never_panics(
        at in any::<prop::sample::Index>(),
        by in 1..=255_u8,
        tail in proptest::collection::vec(any::<u8>(), 0..8),
    ) {
        let bytes = resummed(&baked().bytes, |b| {
            let at = HEADER + at.index(b.len() - HEADER);
            b[at] ^= by;
            b.extend_from_slice(&tail);
        });
        if let Ok(pack) = Pack::open(&bytes, STAMP) {
            // Whatever opened, the formats read it without a panic.
            let bake = every_kind();
            let formats = pack_formats(&bake);
            let _ = bake.check(&formats, sources(), &pack_bundle(pack.into_owned()));
        }
    }

    #[test]
    fn no_bytes_panic_the_pack_or_a_baked_file(
        random in proptest::collection::vec(any::<u8>(), 0..400),
        after in proptest::collection::vec(any::<u8>(), 0..400),
    ) {
        let _ = Pack::open(&random, STAMP);
        let _ = info(&random);
        let headed = [&MAGIC[..], &VERSION.to_le_bytes(), &after].concat();
        let _ = Pack::open(&resummed(&headed, |_| {}), STAMP);
        let baked = [&BAKED_MAGIC[..], &after].concat();
        let file = AssetPath::new("a.ron");
        let cx = DecodeCx::new();
        let _ = Packed::<DialogueFile<Cue>>::new().decode(&file, &baked, &cx);
        let _ = Packed::<KeymapDef<sample::Action>>::new().decode(&file, &baked, &cx);
        let _ = Packed::<LangPack>::new().decode(&file, &baked, &cx);
    }
}

#[test]
fn the_checksum_is_pinned_and_folds_its_high_bits_down() {
    // A change to the checksum changes every pack: make `VERSION` the next
    // number with it.
    assert_eq!(checksum(b""), 0xaf63_bd4c_2962_0a93);
    assert_eq!(checksum(b"shuyi"), 0x5604_ea22_e166_71ac);
    assert_eq!(checksum(b"0123456789abcdef!"), 0xa2b0_353e_3b2a_fb8a);
    // Without the fold, flipping the top bit of two words would cancel.
    let mut flipped = *b"0123456789abcdef";
    flipped[7] ^= 0x80;
    flipped[15] ^= 0x80;
    assert_ne!(checksum(&flipped), checksum(b"0123456789abcdef"));
}

#[test]
fn a_file_lz4_cant_make_smaller_is_stored_and_borrowed() {
    // LZ4 makes these 15 bytes into 15: no smaller, so they are stored.
    let same = vec![
        114, 46, 97, 97, 97, 97, 97, 148, 80, 97, 200, 132, 63, 251, 183,
    ];
    assert_eq!(lz4_flex::block::compress(&same).len(), same.len());
    let bytes = raw_pack(&test_info(), &[("a.bin".to_owned(), same.clone())]);
    let pack = Pack::open(&bytes, STAMP).unwrap();
    let file = &pack.files()[0];
    assert!(matches!(file.bytes, Cow::Borrowed(_)), "{file:?}");
    assert_eq!((file.stored, &*file.bytes), (15, same.as_slice()));
    // A compressed one is unpacked into memory of its own.
    let bytes = raw_pack(&test_info(), &[("b.bin".to_owned(), vec![0; 100])]);
    let pack = Pack::open(&bytes, STAMP).unwrap();
    assert!(matches!(pack.files()[0].bytes, Cow::Owned(_)));
}

#[test]
fn the_limits_hold_at_their_edges() {
    let stored = Codec::Stored;
    // Exactly the most bytes passes that check (and fails the next).
    assert_eq!(
        opened(vec![entry("a", stored, 0, 0, MAX_UNPACKED_BYTES)], b""),
        "the pack is broken: a is stored with two sizes"
    );
    // Exactly LZ4's most is tried, and these bytes don't unpack.
    assert_eq!(
        opened(vec![entry("a", Codec::Lz4, 0, 1, MAX_RATIO)], b"\x00"),
        "the pack is broken: a doesn't unpack"
    );
    // Fewer bytes than the entry says is as broken as bytes that don't
    // unpack at all.
    let short = lz4_flex::block::compress(b"abcabcabcabcabcabc");
    let length = short.len() as u64;
    assert_eq!(
        opened(vec![entry("a", Codec::Lz4, 0, length, 30)], &short),
        "the pack is broken: a doesn't unpack"
    );
    assert_eq!(
        opened(vec![entry("a", Codec::Lz4, 0, length, 18)], &short),
        "ok: 1 file(s)"
    );
    // Exactly the most files opens.
    let most = (0..MAX_FILES)
        .map(|i| entry(&format!("{i:06}"), stored, 0, 0, 0))
        .collect();
    assert_eq!(opened(most, b""), format!("ok: {MAX_FILES} file(s)"));
}
