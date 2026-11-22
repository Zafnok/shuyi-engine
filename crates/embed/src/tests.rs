use std::fs;
use std::path::{Path, PathBuf};

use super::{Entry, Error, Table, matches, paths_source, same_but_case, table_source, walk};

/// An empty folder of its own for test `name`.
fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("shuyi-embed-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap_or_else(|e| panic!("{}: {e}", dir.display()));
    dir
}

/// Writes `text` at `path` under `root`, making its folders.
fn put(root: &Path, path: &str, text: &str) {
    let file = root.join(path);
    if let Some(parent) = file.parent() {
        fs::create_dir_all(parent).unwrap_or_else(|e| panic!("{}: {e}", parent.display()));
    }
    fs::write(&file, text).unwrap_or_else(|e| panic!("{}: {e}", file.display()));
}

fn paths(walk: &super::Walk) -> Vec<&str> {
    walk.files.iter().map(|entry| entry.path.as_str()).collect()
}

#[test]
fn paths_are_sorted_and_slash_separated_at_any_depth() {
    let root = scratch("sorted");
    put(&root, "zebra.txt", "zebra");
    put(&root, "data/levels/one.ron", "one");
    put(&root, "data/alpha.ron", "alpha");
    put(&root, "Big.txt", "big");
    let walk = walk(&root, &[]).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(
        paths(&walk),
        [
            "Big.txt",
            "data/alpha.ron",
            "data/levels/one.ron",
            "zebra.txt"
        ]
    );
    for entry in &walk.files {
        assert!(!entry.path.contains('\\'), "{}", entry.path);
        assert!(entry.file.is_absolute(), "{}", entry.file.display());
        assert!(entry.file.is_file(), "{}", entry.file.display());
    }
    assert_eq!(
        walk.folders,
        [root.clone(), root.join("data"), root.join("data/levels")]
    );
}

#[test]
fn an_empty_folder_is_an_empty_table() {
    let root = scratch("empty");
    fs::create_dir_all(root.join("nothing/here")).unwrap_or_else(|e| panic!("{e}"));
    let walk = walk(&root, &[]).unwrap_or_else(|e| panic!("{e}"));
    assert!(walk.files.is_empty());
    assert_eq!(table_source(&walk.files), "&[\n]");
}

#[test]
fn a_missing_folder_or_a_file_is_an_error_naming_it() {
    let root = scratch("missing");
    let missing = root.join("gone");
    let error = walk(&missing, &[]).err().map(|e| e.to_string());
    assert!(
        error.as_deref().is_some_and(|e| e.contains("gone")),
        "{error:?}"
    );
    put(&root, "file.txt", "file");
    assert!(matches!(
        walk(&root.join("file.txt"), &[]),
        Err(Error::NotAFolder(_))
    ));
}

#[test]
fn skip_patterns_match_whole_paths() {
    assert!(matches("README.md", "README.md"));
    assert!(!matches("README.md", "data/README.md"));
    assert!(matches("**/README.md", "README.md"));
    assert!(matches("**/README.md", "data/deep/README.md"));
    assert!(!matches("**/README.md", "data/README.md.bak"));
    assert!(matches("*.wav", "beep.wav"));
    assert!(!matches("*.wav", "sounds/beep.wav"));
    assert!(matches("sounds/*.wav", "sounds/beep.wav"));
    assert!(matches("sounds/**", "sounds/loud/beep.wav"));
    assert!(matches("sounds/**", "sounds/beep.wav"));
    assert!(!matches("sounds/**", "music/beep.wav"));
    assert!(matches("level-?.ron", "level-2.ron"));
    assert!(!matches("level-?.ron", "level-10.ron"));
    assert!(matches("*", "anything"));
    assert!(!matches("*", "two/parts"));
}

#[test]
fn skipped_files_are_left_out_and_listed() {
    let root = scratch("skip");
    put(&root, "README.md", "notes");
    put(&root, "data/README.md", "notes");
    put(&root, "data/one.ron", "one");
    let skip = ["**/README.md".to_owned()];
    let walk = walk(&root, &skip).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(paths(&walk), ["data/one.ron"]);
    assert_eq!(walk.skipped, ["README.md", "data/README.md"]);
}

#[test]
fn two_paths_differing_only_by_case_are_an_error_with_both() {
    let paths = ["data/Hero.png", "data/hero.png", "other.png"];
    let error = same_but_case(paths.into_iter())
        .err()
        .map(|e| e.to_string());
    let error = error.unwrap_or_default();
    assert!(
        error.contains("`data/Hero.png`") && error.contains("`data/hero.png`"),
        "{error}"
    );
    assert!(same_but_case(["data/hero.png", "data/hero2.png"].into_iter()).is_ok());
}

#[test]
fn a_folder_with_files_differing_only_by_case_fails_where_the_disk_allows_them() {
    let root = scratch("case");
    put(&root, "Hero.png", "upper");
    put(&root, "hero.png", "lower");
    // On a disk that ignores case (Windows, macOS) the second write was the
    // first file: there is one file, and nothing to refuse.
    if fs::read_dir(&root).map_or(0, Iterator::count) < 2 {
        return;
    }
    let error = walk(&root, &[])
        .err()
        .map(|e| e.to_string())
        .unwrap_or_default();
    assert!(
        error.contains("`Hero.png`") && error.contains("`hero.png`"),
        "{error}"
    );
}

/// A file name that isn't UTF-8, where the disk allows one.
#[cfg(unix)]
fn not_utf8() -> std::ffi::OsString {
    use std::os::unix::ffi::OsStringExt as _;
    std::ffi::OsString::from_vec(vec![b'b', b'a', b'd', 0xff])
}

/// A file name that isn't UTF-8 (a lone surrogate), which NTFS allows.
#[cfg(windows)]
fn not_utf8() -> std::ffi::OsString {
    use std::os::windows::ffi::OsStringExt as _;
    std::ffi::OsString::from_wide(&[u16::from(b'b'), 0xd800])
}

#[cfg(any(unix, windows))]
#[test]
fn a_name_that_is_not_utf8_is_an_error_naming_it() {
    let root = scratch("utf8");
    fs::create_dir_all(root.join("data")).unwrap_or_else(|e| panic!("{e}"));
    let file = root.join("data").join(not_utf8());
    // A disk that refuses the name (macOS's APFS) has nothing to refuse.
    if fs::write(&file, "bytes").is_err() {
        return;
    }
    match walk(&root, &[]) {
        Err(Error::NotUtf8(path)) => assert_eq!(path, file),
        other => panic!("{other:?}"),
    }
}

#[test]
fn the_sources_are_rust_with_absolute_escaped_paths() {
    let files = [Entry {
        path: "data/one \"quoted\".ron".to_owned(),
        file: PathBuf::from(r"C:\game\files\data\one.ron"),
    }];
    assert_eq!(
        table_source(&files),
        "&[\n    (\"data/one \\\"quoted\\\".ron\", \
         include_bytes!(\"C:\\\\game\\\\files\\\\data\\\\one.ron\") as &[u8]),\n]"
    );
    assert_eq!(
        paths_source(&["README.md".to_owned()]),
        "&[\n    \"README.md\",\n]"
    );
}

#[test]
fn a_table_writes_its_two_files_and_watches_everything() {
    let game = scratch("table");
    let out = game.join("out");
    fs::create_dir_all(&out).unwrap_or_else(|e| panic!("{e}"));
    put(&game, "files/data/one.ron", "one");
    put(&game, "files/README.md", "notes");
    let written = Table::new("files")
        .skip("**/README.md")
        .write_to(&game, &out)
        .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(written.table, out.join("shuyi_embed_files.rs"));
    assert_eq!(written.skipped, out.join("shuyi_embed_files_skipped.rs"));
    let table = fs::read_to_string(&written.table).unwrap_or_default();
    assert!(
        table.contains("(\"data/one.ron\", include_bytes!("),
        "{table}"
    );
    assert!(!table.contains("README"), "{table}");
    let skipped = fs::read_to_string(&written.skipped).unwrap_or_default();
    assert_eq!(skipped, "&[\n    \"README.md\",\n]");
    let files = game.join("files");
    assert_eq!(
        written.watched,
        [
            files.clone(),
            files.join("README.md"),
            files.join("data"),
            files.join("data/one.ron"),
        ]
    );
}

#[test]
fn a_table_reads_the_folder_it_is_given() {
    let game = scratch("dir");
    put(&game, "assets/one.ron", "one");
    let written = Table::new("art")
        .dir("assets")
        .write_to(&game, &game)
        .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(written.table, game.join("shuyi_embed_art.rs"));
}

#[test]
fn a_table_name_that_cannot_be_a_file_name_is_refused() {
    let game = scratch("name");
    for name in ["", "two words", "../up", "dots.rs"] {
        assert!(
            matches!(
                Table::new(name).dir(".").write_to(&game, &game),
                Err(Error::BadName(_))
            ),
            "{name:?}"
        );
    }
}
