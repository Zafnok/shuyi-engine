//! The graph walk, the texts and `NOTICES.txt` on a small hand-written
//! `cargo metadata` answer (`metadata.json`) whose crates' folders are made
//! here, and the walk on Hello, Grid's real graph.

use super::*;

/// The fixture's `cargo metadata` answer.
const METADATA: &str = include_str!("metadata.json");

/// The MIT text the fixture's crates ship in their folders.
const LIB_MIT: &str = "MIT License\n\nCopyright (c) Al Able\n\nPermission is granted.\n";

/// An empty folder of its own for test `name`.
fn folder(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("shuyi-notices-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// `files` written under `dir`.
fn write_files(dir: &Path, files: &[(&str, &str)]) {
    for (path, text) in files {
        let to = dir.join(path);
        fs::create_dir_all(to.parent().unwrap()).unwrap();
        fs::write(to, text).unwrap();
    }
}

/// The fixture's crates' folders under a folder for test `name`, and the
/// works [`crates_of`] finds there.
fn fixture(name: &str) -> (PathBuf, Vec<Work>) {
    let root = folder(name);
    // lib-a's Apache text is the standard one, with Windows line ends.
    let apache = ENGINE_APACHE.replace('\n', "\r\n");
    write_files(
        &root,
        &[
            ("game-web/Cargo.toml", ""),
            ("lib-a/Cargo.toml", ""),
            ("lib-a/LICENSE-MIT", LIB_MIT),
            ("lib-a/license-apache.md", &apache),
            ("lib-a/COPYING", "See the LICENSE files.\n\n\n"),
            ("lib-a/docs/TERMS.txt", "Terms of lib-a.\n"),
            ("lib-a/README.md", "Not a license.\n"),
            ("lib-a/LICENSES/MIT.txt", "In a folder: not read.\n"),
            ("old-b/Cargo.toml", ""),
            ("odd-e/Cargo.toml", ""),
        ],
    );
    let slashed = root.display().to_string().replace('\\', "/");
    let json = METADATA.replace("%ROOT%", &slashed);
    let works = crates_of(&json, "game-web").unwrap();
    (root, works)
}

fn names(works: &[Work]) -> Vec<&str> {
    works.iter().map(|work| work.name.as_str()).collect()
}

fn labels(work: &Work) -> Vec<&str> {
    work.texts.iter().map(|text| text.label.as_str()).collect()
}

fn find<'a>(works: &'a [Work], name: &str) -> &'a Work {
    works.iter().find(|work| work.name == name).unwrap()
}

#[test]
fn the_walk_follows_normal_edges_lists_each_crate_once_sorted_and_leaves_out_the_games_own() {
    let (root, works) = fixture("walk");
    // Not dev-c (a dev-dependency), build-d (a build-dependency) or
    // the-rules (the game's own member), but odd-e and old-b through it.
    assert_eq!(names(&works), ["game-web", "lib-a", "odd-e", "old-b"]);
    assert!(works.iter().all(|work| work.section == Section::Crates));
    let lib = find(&works, "lib-a");
    assert_eq!(lib.version, "1.2.0");
    assert_eq!(lib.license, "MIT OR Apache-2.0");
    assert_eq!(lib.source, "https://example.com/lib-a");
    // No repository: crates.io's page.
    assert_eq!(
        find(&works, "old-b").source,
        "https://crates.io/crates/old-b"
    );
    let missing = crates_of(&METADATA.replace("%ROOT%", "/x"), "nope").unwrap_err();
    assert!(missing.contains("`nope`"), "{missing}");
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn license_files_are_found_by_name_in_any_case_with_the_license_file() {
    let (root, works) = fixture("files");
    let lib = find(&works, "lib-a");
    assert_eq!(
        labels(lib),
        [
            "COPYING",
            "LICENSE-MIT",
            "docs/TERMS.txt",
            "license-apache.md"
        ]
    );
    assert_eq!(lib.texts[1].body, LIB_MIT);
    assert_eq!(lib.texts[2].body, "Terms of lib-a.\n");
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn the_engines_crates_with_no_file_get_the_engines_texts() {
    let (root, works) = fixture("engine");
    assert_eq!(ENGINE_REPOSITORY, "https://github.com/Zafnok/shuyi-engine");
    let game = find(&works, "game-web");
    assert_eq!(game.texts, engine_texts());
    assert!(game.texts[0].body.starts_with("MIT License"));
    assert!(game.texts[1].body.contains("Apache License"));
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn a_crate_with_no_file_gets_the_texts_of_its_declared_licenses() {
    let (root, works) = fixture("written");
    let old = find(&works, "old-b");
    assert_eq!(old.license, "MIT/Apache-2.0");
    assert_eq!(
        labels(old),
        [
            "MIT, written from the declared license: the package has no license file",
            "Apache-2.0, written from the declared license: the package has no license file",
        ]
    );
    let mit = &old.texts[0].body;
    assert!(
        mit.contains("\nCopyright (c) Ann Author <ann@example.com>, Bo Builder\n"),
        "{mit}"
    );
    assert!(!mit.contains("Nick Wentz"), "{mit}");
    assert_eq!(mit.lines().count(), ENGINE_MIT.lines().count());
    assert_eq!(old.texts[1].body, ENGINE_APACHE);
    // Zlib isn't one this tool writes: listed, with no text.
    assert_eq!(find(&works, "odd-e").texts, []);
    // MIT names its holders: none, no text.
    assert_eq!(written_texts("MIT", &[]), []);
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn license_expressions_split_into_their_ids_once_each() {
    assert_eq!(license_ids("MIT OR Apache-2.0"), ["MIT", "Apache-2.0"]);
    assert_eq!(license_ids("MIT/Apache-2.0"), ["MIT", "Apache-2.0"]);
    assert_eq!(
        license_ids("(MIT OR Apache-2.0) AND Unicode-3.0"),
        ["MIT", "Apache-2.0", "Unicode-3.0"]
    );
    assert_eq!(
        license_ids("Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT"),
        ["Apache-2.0", "LLVM-exception", "MIT"]
    );
    assert_eq!(license_ids(""), Vec::<&str>::new());
}

#[test]
fn texts_are_compared_with_their_line_ends_and_trailing_blank_lines_made_alike() {
    assert_eq!(normalized("a\r\nb\r\n\r\n  \n"), "a\nb\n");
    assert_eq!(normalized("a\nb"), "a\nb\n");
    assert_eq!(normalized(""), "\n");
    let mut one = Work::new(Section::Crates, "one");
    one.texts = vec![Text::new("LICENSE", "same\r\ntext\r\n\r\n")];
    let mut two = Work::new(Section::Shell, "two");
    two.texts = vec![Text::new("COPYING", "same\ntext")];
    let text = render("T", &[two, one]);
    assert_eq!(text.matches("same\ntext\n").count(), 1, "{text}");
    assert!(
        text.contains("  one (LICENSE)\n  two (COPYING)\n"),
        "{text}"
    );
    assert!(!text.contains('\r'));
}

#[test]
fn the_games_texts_are_read_whole_and_a_missing_one_is_named() {
    let root = folder("game");
    write_files(&root, &[("files/font/OFL.txt", "Open Font License\r\n")]);
    let mut notices = Notices {
        texts: vec!["files/font/OFL.txt".to_owned()],
    };
    let works = game_texts(&root, &notices).unwrap();
    assert_eq!(works.len(), 1);
    assert_eq!(works[0].section, Section::Game);
    assert_eq!(works[0].name, "files/font/OFL.txt");
    assert_eq!(
        works[0].texts,
        [Text::new("files/font/OFL.txt", "Open Font License\r\n")]
    );
    notices.texts.push("files/gone.txt".to_owned());
    let error = game_texts(&root, &notices).unwrap_err();
    assert!(error.contains("files/gone.txt"), "{error}");
    assert_eq!(game_texts(&root, &Notices::default()), Ok(Vec::new()));
    fs::remove_dir_all(&root).unwrap();
}

/// The fixture's crates, a shell file and a game text.
fn all_works(crates: Vec<Work>) -> Vec<Work> {
    let mut shell = Work::new(Section::Shell, "loader.js, from lib-a");
    shell.version = "1.2.0".to_owned();
    shell.license = "MIT".to_owned();
    shell.source = "https://example.com/lib-a".to_owned();
    shell.texts = vec![Text::new("loader-LICENSE-MIT.txt", LIB_MIT)];
    let mut font = Work::new(Section::Game, "files/font/OFL.txt");
    font.texts = vec![Text::new("files/font/OFL.txt", "Open Font License\n")];
    let mut works = vec![font, shell];
    works.extend(crates);
    works
}

#[test]
fn notices_txt_from_the_fixture() {
    let (root, crates) = fixture("snapshot");
    let works = all_works(crates);
    let dist = root.join("dist");
    fs::create_dir_all(&dist).unwrap();
    write(&dist, "The Game", &works).unwrap();
    let text = fs::read_to_string(dist.join(FILE)).unwrap();
    // The Apache text four works share, once.
    assert_eq!(text.matches("Apache License").count(), 1);
    insta::assert_snapshot!("notices_txt", text);
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn the_same_works_give_the_same_bytes() {
    let (root, crates) = fixture("same");
    let works = all_works(crates);
    for dir in ["a", "b"] {
        fs::create_dir_all(root.join(dir)).unwrap();
        write(&root.join(dir), "The Game", &works).unwrap();
    }
    let a = fs::read(root.join("a").join(FILE)).unwrap();
    assert_eq!(a, fs::read(root.join("b").join(FILE)).unwrap());
    let mut reversed = works.clone();
    reversed.reverse();
    // Sections keep their order; works within one keep theirs.
    assert_ne!(render("The Game", &reversed), String::from_utf8(a).unwrap());
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn hello_grids_web_build_holds_macroquad_and_quad_snd_and_no_dev_or_build_crate() {
    let example = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/hello-grid");
    let works = crates(&example, "hello-grid", "wasm32-unknown-unknown", &[]).unwrap();
    let listed = names(&works);
    for shipped in ["hello-grid", "macroquad", "quad-snd", "shuyi-core"] {
        assert!(listed.contains(&shipped), "{shipped} in {listed:?}");
    }
    for left_out in ["insta", "autocfg"] {
        assert!(!listed.contains(&left_out), "{left_out} in {listed:?}");
    }
    let snd = find(&works, "quad-snd");
    assert_eq!(snd.license, "MIT/Apache-2.0");
    assert!(snd.texts[0].body.contains("Copyright (c) Fedor Logachev"));
    assert_eq!(find(&works, "shuyi-core").texts, engine_texts());
    // A feature of the package is named through it.
    let tools = crates(
        &example,
        "hello-grid",
        "wasm32-unknown-unknown",
        &["tools".to_owned()],
    );
    assert!(tools.is_ok(), "{tools:?}");
    let error = crates(
        &example,
        "hello-grid",
        "wasm32-unknown-unknown",
        &["nope".to_owned()],
    );
    assert!(error.unwrap_err().starts_with("cargo metadata failed"));
}
