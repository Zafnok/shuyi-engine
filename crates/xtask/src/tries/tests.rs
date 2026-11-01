use std::collections::BTreeMap;

use super::*;

/// A list with a picture and a before and after.
const LIST: &str = r##"
[title]
step = "01-new"
alt = "The title"

[title-blue]
step = "01-new"
keys = "Enter"
alt = "The title"
after = "The title, in blue"
[[title-blue.edits]]
file = "files/palette.ron"
find = "#ffd75a"
replace = "#5ac8ff"
"##;

fn list() -> BTreeMap<String, Try> {
    parse(LIST).unwrap()
}

fn edit(find: &str, replace: &str) -> Edit {
    Edit {
        file: "f.ron".to_owned(),
        find: find.to_owned(),
        replace: replace.to_owned(),
    }
}

#[test]
fn the_list_parses_with_its_keys_and_edits() {
    let tries = list();
    assert_eq!(tries["title"].keys, "");
    assert_eq!(pictures(&tries["title"]), [BEFORE]);
    assert_eq!(tries["title-blue"].keys, "Enter");
    assert_eq!(tries["title-blue"].edits[0].replace, "#5ac8ff");
    assert_eq!(pictures(&tries["title-blue"]), [BEFORE, AFTER]);
}

#[test]
fn an_unknown_field_a_bad_name_and_a_lone_after_fail() {
    let unknown = parse("[a]\nstep = \"s\"\nalt = \"a\"\ncolor = 1\n").unwrap_err();
    assert!(unknown[0].contains("color"), "{unknown:?}");
    let name = parse("[Bad_Name]\nstep = \"s\"\nalt = \"a\"\n").unwrap_err();
    assert!(name[0].contains("Bad_Name"), "{name:?}");
    let after = parse("[a]\nstep = \"s\"\nalt = \"a\"\nafter = \"b\"\n").unwrap_err();
    assert!(after[0].contains("needs edits"), "{after:?}");
    let edits = parse(
        "[a]\nstep = \"s\"\nalt = \"a\"\n[[a.edits]]\nfile = \"f\"\nfind = \"x\"\nreplace = \"y\"\n",
    )
    .unwrap_err();
    assert!(edits[0].contains("needs edits"), "{edits:?}");
}

#[test]
fn an_edit_needs_its_text_exactly_once() {
    assert_eq!(apply("a b c", &edit("b", "z")).unwrap(), "a z c");
    let none = apply("a b c", &edit("q", "z")).unwrap_err();
    assert!(none.contains("0 times"), "{none}");
    let twice = apply("b b", &edit("b", "z")).unwrap_err();
    assert!(twice.contains("2 times"), "{twice}");
}

#[test]
fn an_edit_of_several_lines_matches_either_line_ending() {
    let edited = apply("one\r\ntwo\r\nthree\r\n", &edit("one\ntwo", "both")).unwrap();
    assert_eq!(edited, "both\nthree\n");
}

#[test]
fn a_try_no_page_shows_is_named() {
    let tries = list();
    let pages = "{{ try_it(name=\"title-blue\") }}";
    assert_eq!(unused(&tries, pages), ["title"]);
    let both = "{{ try_it(name=\"title\") }} {{ try_it(name=\"title-blue\") }}";
    assert_eq!(unused(&tries, both), [] as [&str; 0]);
}

#[test]
fn a_folder_no_try_names_is_an_orphan() {
    let tries = list();
    let found = ["title".to_owned(), "gone".to_owned()];
    assert_eq!(orphans(&tries, &found), ["gone"]);
}

#[test]
fn a_stale_or_missing_picture_says_how_to_remake_it() {
    assert_eq!(compare("a", BEFORE, Some(b"same"), b"same"), None);
    let stale = compare("a", BEFORE, Some(b"old"), b"new").unwrap();
    assert!(stale.contains("stale"), "{stale}");
    assert!(stale.contains("cargo xtask tutorial-tries a"), "{stale}");
    let missing = compare("a", AFTER, None, b"new").unwrap();
    assert!(missing.contains("missing"), "{missing}");
    assert!(
        missing.contains("cargo xtask tutorial-tries a"),
        "{missing}"
    );
}

#[test]
fn the_options_take_check_and_names_and_refuse_other_flags() {
    let args = |list: &[&str]| list.iter().map(ToString::to_string).collect::<Vec<_>>();
    let options = Options::parse(&args(&["--check", "a", "b"])).unwrap();
    assert!(options.check);
    assert_eq!(options.names, ["a", "b"]);
    assert_eq!(Options::parse(&[]).unwrap(), Options::default());
    assert_eq!(Options::parse(&args(&["--nope"])), None);
}

#[test]
fn a_copy_leaves_out_what_a_build_writes() {
    let root = std::env::temp_dir().join(format!("xtask-tries-{}", std::process::id()));
    let from = root.join("from");
    fs::create_dir_all(from.join("target/debug")).unwrap();
    fs::create_dir_all(from.join("crates/game")).unwrap();
    fs::write(from.join("target/debug/game"), "built").unwrap();
    fs::write(from.join("crates/game/a.rs"), "kept").unwrap();
    let to = root.join("to");
    copy_tree(&from, &to).unwrap();
    assert_eq!(
        fs::read_to_string(to.join("crates/game/a.rs")).unwrap(),
        "kept"
    );
    assert!(!to.join("target").exists());
    fs::remove_dir_all(&root).unwrap();
}
