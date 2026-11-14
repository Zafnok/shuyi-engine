use std::collections::BTreeMap;

use shuyi_assets::pack::{Content, PackInfo};

use super::*;
use crate::project::Config;

/// Hello, Grid's folder.
fn hello_grid() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/hello-grid")
}

/// Hello, Grid's tiles: the picture its `shuyi.ron` names for the overlay.
fn tiles() -> Vec<u8> {
    fs::read(hello_grid().join("files/tiles.png")).expect("Hello, Grid's tiles")
}

/// A new empty folder for test `name`.
fn temp(name: &str) -> PathBuf {
    ledger::tests::temp(&format!("recipient-{name}"))
}

/// A program of `len` bytes that holds a public build's field.
fn program(len: usize) -> Vec<u8> {
    let mut bytes: Vec<u8> = (0..len)
        .map(|i| u8::try_from(i * 37 % 241).expect("under 241"))
        .collect();
    bytes[len / 2..len / 2 + code::FIELD_BYTES].copy_from_slice(&code::FIELD);
    bytes
}

/// A pack of Hello, Grid's files, as their own bytes.
fn pack() -> Vec<u8> {
    let mut files = BTreeMap::new();
    for name in [
        "tiles.png",
        "font/atlas.png",
        "palette.ron",
        "stage.map",
        "keymap.ron",
        "en.ron",
    ] {
        let bytes = fs::read(hello_grid().join("files").join(name)).expect("a file of Hello, Grid");
        files.insert(name.to_owned(), Content::Raw(bytes));
    }
    let info = PackInfo {
        stamp: "hello-grid 0.1.0".to_owned(),
        ..PackInfo::default()
    };
    pack::write(&info, &files)
}

fn the(bits: u32) -> Code {
    Code::new(bits).expect("not zero")
}

/// The codes the marks of one kind name, as text, in order.
fn named(marks: &[FoundMark], kind: MarkKind) -> Vec<Option<String>> {
    marks
        .iter()
        .filter(|mark| mark.mark == kind)
        .map(|mark| mark.code.clone())
        .collect()
}

/// A copy's program and pack for `code`, and the public ones.
struct Copy {
    program: Vec<u8>,
    pack: Vec<u8>,
}

/// Marks a program and a pack for `code`, as the overlay does, in folder
/// `dir`.
fn copy_for(dir: &Path, code: Code) -> Copy {
    let program_file = dir.join("game.exe");
    let pack_file = dir.join("game.pack");
    fs::write(&program_file, program(5000)).expect("written");
    fs::write(&pack_file, pack()).expect("written");
    assert_eq!(mark_program(&program_file, code).expect("marked"), 1);
    mark_pack(&pack_file, "tiles.png", code).expect("marked");
    Copy {
        program: fs::read(&program_file).expect("read"),
        pack: fs::read(&pack_file).expect("read"),
    }
}

/// Every mark of a copy's program and pack, as the detector reads them.
fn marks_of(copy: &Copy) -> Vec<FoundMark> {
    let mut marks = program_marks("game.exe", &copy.program);
    marks.extend(pack_marks("game.pack", &copy.pack, Some("tiles.png"), None));
    marks
}

#[test]
fn an_id_is_plain() {
    for id in ["jane", "press-01", "Example_Press.jane", &"a".repeat(64)] {
        assert!(plain_id(id), "{id}");
    }
    for id in ["", ".hidden", "a b", "a/b", "a\\b", "é", &"a".repeat(65)] {
        assert!(!plain_id(id), "{id}");
    }
}

#[test]
fn marking_the_program_changes_only_its_field() {
    let dir = temp("program");
    let file = dir.join("game.exe");
    let before = program(5000);
    fs::write(&file, &before).expect("written");
    assert_eq!(mark_program(&file, the(7)).expect("marked"), 1);
    let after = fs::read(&file).expect("read");
    assert_eq!(after.len(), before.len());
    let changed: Vec<usize> = (0..before.len())
        .filter(|&i| before[i] != after[i])
        .collect();
    assert!(
        changed.iter().all(|&i| (2516..2532).contains(&i)),
        "{changed:?}"
    );
    let none = dir.join("other.exe");
    fs::write(&none, b"no field here").expect("written");
    assert!(
        mark_program(&none, the(7))
            .unwrap_err()
            .message()
            .contains("no recipient's field")
    );
}

#[test]
fn each_mark_alone_names_the_code() {
    let dir = temp("each");
    let code = the(0x1234_5678);
    let copy = copy_for(&dir, code);
    let text = Some(code.to_string());
    assert_eq!(
        named(&program_marks("game.exe", &copy.program), MarkKind::Program),
        std::slice::from_ref(&text)
    );
    let pack_only = pack_marks("game.pack", &copy.pack, Some("tiles.png"), None);
    assert_eq!(
        named(&pack_only, MarkKind::Pack),
        std::slice::from_ref(&text)
    );
    assert_eq!(
        named(&pack_only, MarkKind::Picture),
        std::slice::from_ref(&text)
    );
    let picture = Raw::read(&copy.pack)
        .expect("reads")
        .file("tiles.png")
        .expect("there");
    assert_eq!(picture_mark("tiles.png", &picture, None).code, text);
}

#[test]
fn the_copy_is_the_public_build_but_for_its_marks() {
    let dir = temp("same");
    let copy = copy_for(&dir, the(99));
    let public = Pack::open_files(&pack());
    let marked = Pack::open_files(&copy.pack);
    assert_eq!(public.len(), marked.len());
    for ((path, a), (other, b)) in public.iter().zip(&marked) {
        assert_eq!(path, other);
        if path == "tiles.png" {
            let (a, b) = (pixels(a).expect("a PNG"), pixels(b).expect("a PNG"));
            assert_eq!((a.width, a.height), (b.width, b.height));
            assert_eq!(mark::palette_count(&a), mark::palette_count(&b));
            assert!(a.rgba.iter().zip(&b.rgba).all(|(x, y)| x.abs_diff(*y) <= 3));
        } else {
            assert_eq!(a, b, "{path}");
        }
    }
}

/// What a pack opens to, by path, whatever its stamp.
trait OpenFiles {
    fn open_files(bytes: &[u8]) -> Vec<(String, Vec<u8>)>;
}

impl OpenFiles for Pack<'_> {
    fn open_files(bytes: &[u8]) -> Vec<(String, Vec<u8>)> {
        let raw = Raw::read(bytes).expect("reads");
        let stamp = raw.info().stamp.clone();
        let pack = Pack::open(bytes, &stamp).expect("opens");
        pack.files()
            .iter()
            .map(|file| (file.path.clone(), file.bytes.to_vec()))
            .collect()
    }
}

use shuyi_assets::pack::Pack;

#[test]
fn the_public_build_has_no_mark() {
    let mut marks = program_marks("game.exe", &program(5000));
    let own = pixels(&tiles()).expect("a PNG");
    marks.extend(pack_marks(
        "game.pack",
        &pack(),
        Some("tiles.png"),
        Some(&own),
    ));
    assert_eq!(marks.len(), 3);
    assert!(marks.iter().all(|mark| mark.code.is_none()), "{marks:?}");
    let codes: Vec<String> = Vec::new();
    assert_eq!(verdict(&marks, &codes), "no mark found");
    // Without the game's own picture to compare, its bits alone hold no code
    // either.
    assert_eq!(picture_mark("tiles.png", &tiles(), None).code, None);
}

/// The marks that still name `code` after an attack, as `program`, `pack`
/// and `picture`.
fn surviving(marks: &[FoundMark], code: Code) -> Vec<&'static str> {
    let text = code.to_string();
    let mut kinds: Vec<&'static str> = marks
        .iter()
        .filter(|mark| mark.code.as_deref() == Some(text.as_str()))
        .map(|mark| match mark.mark {
            MarkKind::Program => "program",
            MarkKind::Pack => "pack",
            MarkKind::Picture => "picture",
        })
        .collect();
    kinds.dedup();
    kinds
}

/// The picture of a copy's pack, re-encoded with each channel's three lowest
/// bits dropped: what a lossy save at a lower quality, or a palette reduced,
/// does to them.
fn reencoded(png: &[u8]) -> Vec<u8> {
    let mut image = pixels(png).expect("a PNG");
    for byte in &mut image.rgba {
        *byte &= !0b111;
    }
    crate::codec::encode_png(image.width, image.height, &image.rgba).expect("encoded")
}

/// The rows `from..to` of the PNG `png`.
fn cropped(png: &[u8], from: u32, to: u32, columns: u32) -> Vec<u8> {
    let image = pixels(png).expect("a PNG");
    let mut rgba = Vec::new();
    for y in from..to {
        for x in 0..columns {
            rgba.extend(image.pixel(x, y).expect("in the picture"));
        }
    }
    crate::codec::encode_png(columns, to - from, &rgba).expect("encoded")
}

// The limits the book states (`site/content/learn/book/targets-and-export.md`,
// "Closed builds"), on Hello, Grid's files: which marks still name the
// recipient after each attack. A change here changes the book.
#[test]
fn the_limits_hold_as_the_book_states() {
    let dir = temp("limits");
    let code = the(0x00c0_ffee);
    let copy = copy_for(&dir, code);
    let picture = Raw::read(&copy.pack)
        .expect("reads")
        .file("tiles.png")
        .expect("there");
    assert_eq!(
        surviving(&marks_of(&copy), code),
        ["program", "pack", "picture"]
    );

    // The program's field overwritten (its data stripped, or zeroed).
    let mut stripped = copy.program.clone();
    let at = code::find_fields(&stripped)[0];
    stripped[at + 16..at + code::FIELD_BYTES].fill(0);
    let attacked = Copy {
        program: stripped,
        pack: copy.pack.clone(),
    };
    assert_eq!(surviving(&marks_of(&attacked), code), ["pack", "picture"]);
    let removed: Vec<u8> = [&copy.program[..at], &copy.program[at + code::FIELD_BYTES..]].concat();
    assert_eq!(program_marks("game.exe", &removed), []);

    // The pack repacked: its files laid out again in path order.
    let repacked = Copy {
        program: copy.program.clone(),
        pack: Raw::read(&copy.pack).expect("reads").write(),
    };
    assert_eq!(
        surviving(&marks_of(&repacked), code),
        ["program", "picture"]
    );

    // The picture re-encoded at a lower quality, in a repacked pack.
    let mut raw = Raw::read(&copy.pack).expect("reads");
    raw.replace("tiles.png", &reencoded(&picture));
    let both = Copy {
        program: copy.program.clone(),
        pack: raw.write(),
    };
    assert_eq!(surviving(&marks_of(&both), code), ["program"]);
    assert_eq!(
        picture_mark("tiles.png", &reencoded(&picture), None).code,
        None
    );

    // The picture cropped: Hello, Grid's tiles are one row of 8 tiles of 24
    // pixels. A crop that keeps a piece of every tile keeps every color, and
    // the mark; one of the first four tiles drops colors, and the mark.
    let every_tile = cropped(&picture, 4, 20, 192);
    assert_eq!(
        picture_mark("tiles.png", &every_tile, None).code,
        Some(code.to_string())
    );
    let four_tiles = cropped(&picture, 0, 24, 96);
    assert_eq!(picture_mark("tiles.png", &four_tiles, None).code, None);

    // Two copies compared: every byte that differs is a mark's.
    let other = copy_for(&temp("limits-other"), the(0x0bad_f00d));
    let differ: Vec<usize> = (0..copy.program.len())
        .filter(|&i| copy.program[i] != other.program[i])
        .collect();
    assert!(
        differ
            .iter()
            .all(|&i| (at + 16..at + code::FIELD_BYTES).contains(&i))
    );
}

#[test]
fn a_copy_whose_marks_disagree_says_so() {
    let dir = temp("disagree");
    let a = copy_for(&dir, the(1111));
    let b = copy_for(&temp("disagree-b"), the(2222));
    let mixed = Copy {
        program: a.program,
        pack: b.pack,
    };
    let marks = marks_of(&mixed);
    let mut codes: Vec<String> = marks.iter().filter_map(|m| m.code.clone()).collect();
    codes.sort();
    codes.dedup();
    assert_eq!(codes.len(), 2);
    assert!(verdict(&marks, &codes).contains("2 different codes"));
}

#[test]
fn every_verdict_that_names_a_code_says_circumstantial() {
    let dir = temp("verdict");
    let marks = marks_of(&copy_for(&dir, the(5)));
    let codes = vec![the(5).to_string()];
    let said = verdict(&marks, &codes);
    assert!(said.contains("3 of 3 mark(s) name the code"), "{said}");
    assert!(
        said.contains("circumstantial evidence, never proof"),
        "{said}"
    );
    assert!(CIRCUMSTANTIAL.contains("circumstantial evidence, never proof"));
}

/// A project in `dir` that names Hello, Grid's tiles for the overlay, its
/// files in `files/`.
fn project(dir: &Path) -> Project {
    fs::create_dir_all(dir.join("files")).expect("a folder");
    fs::write(dir.join("files/tiles.png"), tiles()).expect("written");
    let config = Config::parse("(assets: \"files\", export: (overlay_picture: \"tiles.png\"))")
        .expect("a project file");
    Project::new(dir, config)
}

#[test]
fn the_detector_reads_a_folder_an_archive_and_one_file_and_the_ledger() {
    let dir = temp("detect");
    let project = project(&dir);
    let code = the(0x5eed_5eed);
    let copy = copy_for(&dir.join("files"), code);
    let package = dir.join("dist/recipients/jane/windows/game-0.1.0-windows");
    fs::create_dir_all(&package).expect("a folder");
    fs::write(package.join("game.exe"), &copy.program).expect("written");
    fs::write(package.join("game.pack"), &copy.pack).expect("written");
    let entries = archive::entries(&package, "game-0.1.0-windows/", &|_| false).expect("listed");
    let zip = dir.join("dist/recipients/jane/windows/game-0.1.0-windows.zip");
    archive::zip(&zip, &entries).expect("zipped");

    let (key_dir, key) = ledger::tests::new_key("detect");
    let kit = kit_of(&project);
    let ledger_path = ledger::path_beside(&kit);
    let entry = ledger::Entry {
        recipient: "jane".to_owned(),
        label: "Jane, Example Press".to_owned(),
        code: code.to_string(),
        ..ledger::Entry::default()
    };
    let kept = ledger::Ledger {
        about: ledger::ABOUT.to_owned(),
        signer: ledger::public_key(&key).expect("its public key"),
        entries: vec![entry],
    };
    ledger::save(&ledger_path, &kept, &key).expect("saved");
    let _ = key_dir;

    for file in [
        "dist/recipients/jane/windows/game-0.1.0-windows",
        "dist/recipients/jane/windows/game-0.1.0-windows.zip",
        "dist/recipients/jane/windows/game-0.1.0-windows/game.exe",
        "dist/recipients/jane/windows/game-0.1.0-windows/game.pack",
    ] {
        let found = detect(&project, file).expect("read");
        assert_eq!(found.codes, [code.to_string()], "{file}");
        assert_eq!(found.recipients.len(), 1, "{file}");
        assert_eq!(found.recipients[0].recipient, "jane");
        assert_eq!(found.signed, Some(true));
        assert!(found.note.contains("circumstantial"));
    }
    // The game's own picture names no one.
    let found = detect(&project, "files/tiles.png").expect("read");
    assert_eq!(found.verdict, "no mark found");
    assert!(found.marks[0].said.contains("the game's own picture"));
    // A file with no mark at all.
    fs::write(dir.join("notes.txt"), "nothing").expect("written");
    let found = detect(&project, "notes.txt").expect("read");
    assert!(
        found.verdict.starts_with("no mark found"),
        "{}",
        found.verdict
    );
}

#[test]
fn an_overlay_is_refused_before_anything_is_built() {
    let root = hello_grid();
    let mut project = Project::load(&root).expect("Hello, Grid's project");
    // A kit that is nowhere, whatever Hello, Grid's `dist` holds.
    project.config.reveal = Some(crate::project::Reveal {
        out: Some("dist/no-kit-here".to_owned()),
        ..crate::project::Reveal::default()
    });
    let cx = crate::export::tests::cx("recipient-refused", "windows");
    let asked = Asked {
        recipient: "jane".to_owned(),
        label: "jane".to_owned(),
        key: PathBuf::from("key"),
    };
    let refused = |cx: &ExportCx, asked: &Asked| {
        refuse(cx, &super::super::desktop::WINDOWS, &project, asked)
            .unwrap_err()
            .message()
    };
    assert!(refused(&cx, &asked).contains("add --release"));
    let mut release = cx.clone();
    release.release = true;
    release.features.push(super::super::PACKED.to_owned());
    let bad = Asked {
        recipient: "a b".to_owned(),
        ..asked.clone()
    };
    assert!(refused(&release, &bad).contains("an id is 1 to 64"));
    let playdate = refuse(
        &release,
        &super::super::playdate::PLAYDATE,
        &project,
        &asked,
    )
    .unwrap_err()
    .message();
    assert!(playdate.contains("has no overlay"), "{playdate}");
    let mut embedded = release.clone();
    embedded.features.clear();
    assert!(refused(&embedded, &asked).contains("needs a `packed` feature"));
    // Hello, Grid names its picture; its kit isn't prepared here.
    let no_kit = refused(&release, &asked);
    assert!(no_kit.contains("reveal prepare"), "{no_kit}");
}

#[test]
fn a_signing_key_is_named_or_refused() {
    let dir = temp("key");
    fs::write(dir.join("key"), "a key").expect("written");
    assert_eq!(key_of(&dir, Some("key")).expect("there"), dir.join("key"));
    assert!(
        key_of(&dir, Some("nope"))
            .unwrap_err()
            .message()
            .contains("isn't there")
    );
}

#[test]
fn a_folders_digest_is_of_its_files_by_path() {
    let dir = temp("digest");
    fs::create_dir_all(dir.join("a/b")).expect("a folder");
    fs::write(dir.join("a/b/c.txt"), "c").expect("written");
    fs::write(dir.join("a/d.txt"), "d").expect("written");
    let lines = format!(
        "{}  b/c.txt\n{}  d.txt\n",
        sha256::hex(b"c"),
        sha256::hex(b"d")
    );
    assert_eq!(
        digest(&dir.join("a")).expect("read"),
        sha256::hex(lines.as_bytes())
    );
    assert_eq!(
        digest(&dir.join("a/d.txt")).expect("read"),
        sha256::hex(b"d")
    );
}
