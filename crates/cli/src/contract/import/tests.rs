//! `import` on files of a scratch folder.

use std::path::PathBuf;

use super::*;

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("shuyi-import-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn input(file: &str, kind: Option<&str>, print: bool) -> ImportInput {
    ImportInput {
        file: file.to_owned(),
        kind: kind.map(str::to_owned),
        print,
    }
}

fn args(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| (*s).to_owned()).collect()
}

#[test]
fn a_count_is_one_thing_or_many() {
    assert_eq!(count(1, "color", "colors"), "1 color");
    assert_eq!(count(0, "color", "colors"), "0 colors");
    assert_eq!(count(3, "color", "colors"), "3 colors");
}

#[test]
fn the_standard_kinds_are_the_engines() {
    let kinds = Importers::standard().kinds();
    for kind in [
        "image",
        "sound",
        "music track",
        "palette",
        "font atlas",
        "dialogue",
    ] {
        assert!(kinds.contains(&kind), "{kind}: {kinds:?}");
    }
    let found = Importers::new(Formats::new()).kinds();
    assert!(found.is_empty(), "{found:?}");
    assert!(format!("{:?}", Importers::standard()).contains("palette"));
}

#[test]
fn a_folder_bundle_holds_every_file_below_it_up_to_its_limits() {
    let dir = scratch("bundle");
    fs::create_dir_all(dir.join("sub")).unwrap();
    fs::write(dir.join("a.txt"), "aa").unwrap();
    fs::write(dir.join("sub/b.txt"), "bbb").unwrap();
    let bundle = folder_bundle(&dir, 2, 5).unwrap();
    assert_eq!(bundle.bytes("a.txt"), Some(&b"aa"[..]));
    assert_eq!(bundle.bytes("sub/b.txt"), Some(&b"bbb"[..]));
    let error = folder_bundle(&dir, 1, 5).unwrap_err();
    assert_eq!(error, "its folder holds more than 1 files or 5 bytes");
    assert!(folder_bundle(&dir, 2, 4).is_err());
    assert!(folder_bundle(&dir.join("nowhere"), 2, 5).is_err());
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_file_is_read_as_every_kind_that_knows_it() {
    let dir = scratch("kinds");
    fs::write(
        dir.join("colors.ron"),
        "{ \"text\": \"#ffffff\", \"panel\": \"#000000\" }",
    )
    .unwrap();
    let importers = Importers::standard();
    let answer = import(
        &importers,
        &dir,
        &input("colors.ron", Some("palette"), false),
    );
    assert!(answer.ok, "{answer:?}");
    assert_eq!(answer.op, Op::Import);
    let out = answer.result.unwrap();
    assert_eq!(out.file, "colors.ron");
    assert_eq!(
        out.read_as,
        [Imported {
            kind: "palette".into(),
            summary: "palette: 2 colors".into(),
            text: None,
        }]
    );
    assert!(out.errors.is_empty(), "{:?}", out.errors);
    assert!(answer.limits.is_empty(), "{:?}", answer.limits);
    // Every kind: a palette's file is a names table too.
    let answer = import(&importers, &dir, &input("colors.ron", None, false));
    let kinds: Vec<&str> = answer
        .result
        .as_ref()
        .unwrap()
        .read_as
        .iter()
        .map(|r| r.kind.as_str())
        .collect();
    assert_eq!(kinds, ["palette", "names table"]);
    assert_eq!(
        answer.limits,
        ["it reads as 2 kinds: name the one meant with --kind"]
    );
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn print_gives_our_text_format_or_the_asset_as_read() {
    let dir = scratch("print");
    fs::write(dir.join("colors.ron"), "{ \"text\": \"#ffffff\" }").unwrap();
    let as_read = import(
        &Importers::standard(),
        &dir,
        &input("colors.ron", Some("palette"), true),
    );
    let text = as_read.result.as_ref().unwrap().read_as[0]
        .text
        .clone()
        .unwrap();
    assert!(text.starts_with("PaletteDef {"), "{text}");
    assert_eq!(
        as_read.limits,
        [
            "no writer of our text format for the palette kind yet: it is printed as read, in \
             Rust's debug form"
        ]
    );
    let written = Importers::new(Formats::standard())
        .kind_with_text::<PaletteDef>(|_| "one".into(), |p| format!("{} colors", p.colors.len()));
    let answer = import(&written, &dir, &input("colors.ron", None, true));
    assert_eq!(
        answer.result.unwrap().read_as[0].text.as_deref(),
        Some("1 colors")
    );
    assert!(answer.limits.is_empty(), "{:?}", answer.limits);
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_file_no_kind_reads_gives_the_errors_of_those_that_claim_it() {
    let dir = scratch("errors");
    fs::write(dir.join("broken.ron"), "{ \"text\": 3 }").unwrap();
    fs::write(dir.join("notes.txt"), "hello").unwrap();
    let importers = Importers::standard();
    let broken = import(
        &importers,
        &dir,
        &input("broken.ron", Some("palette"), false),
    );
    assert!(!broken.ok);
    let errors = &broken.result.as_ref().unwrap().errors;
    assert_eq!(errors[0].file, "broken.ron");
    assert!(broken.limits.is_empty(), "{:?}", broken.limits);
    // Every kind that claims `.ron` says why: none says no format knows it.
    let every = import(&importers, &dir, &input("broken.ron", None, false));
    let errors = &every.result.as_ref().unwrap().errors;
    assert!(errors.len() > 1, "{errors:?}");
    assert!(errors.iter().all(|e| !e.message.starts_with("no format")));
    assert!(every.limits.is_empty(), "{:?}", every.limits);
    let unknown = import(&importers, &dir, &input("notes.txt", None, false));
    assert!(!unknown.ok);
    let error = &unknown.result.as_ref().unwrap().errors[0];
    assert!(
        error
            .message
            .starts_with("no format of these kinds knows its bytes")
    );
    assert_eq!(error.code.as_deref(), Some("SH2005"));
    assert_eq!(error.line, None);
    assert_eq!(
        unknown.limits,
        ["the importers of Tiled, LDtk and Aseprite files come with tickets 1123 and 1124"]
    );
    let missing = import(&importers, &dir, &input("gone.ron", None, false));
    assert_eq!(missing.code.as_deref(), Some("SH3004"));
    assert!(missing.error.unwrap().starts_with("gone.ron: "));
    let no_kind = import(&importers, &dir, &input("notes.txt", Some("tune"), false));
    assert_eq!(no_kind.exit_code(), 2);
    assert!(
        no_kind
            .error
            .unwrap()
            .starts_with("no kind `tune`: the kinds are image, ")
    );
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_folder_too_big_to_read_is_a_limit() {
    let dir = scratch("big");
    for i in 0..=MAX_FOLDER_FILES {
        fs::write(dir.join(format!("{i}.txt")), "").unwrap();
    }
    fs::write(dir.join("colors.ron"), "{ \"text\": \"#ffffff\" }").unwrap();
    let answer = import(
        &Importers::standard(),
        &dir,
        &input("colors.ron", Some("palette"), false),
    );
    assert!(answer.ok);
    assert_eq!(
        answer.limits,
        [
            "the files it names were not read: its folder holds more than 1000 files or 67108864 bytes"
        ]
    );
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn the_arguments_are_a_file_a_kind_and_print() {
    let read = parse_args(&args(&["a.ron", "--kind", "palette", "--print"])).unwrap();
    assert_eq!(read, input("a.ron", Some("palette"), true));
    assert_eq!(
        parse_args(&args(&["a.ron"])).unwrap(),
        input("a.ron", None, false)
    );
    for bad in [&[][..], &["a", "b"], &["a", "--kind"], &["a", "--bogus"]] {
        assert!(parse_args(&args(bad)).is_err(), "{bad:?}");
    }
    // A flag is never taken for the file.
    for flags in [&["--bogus"][..], &["a", "--bogus"]] {
        assert_eq!(
            parse_args(&args(flags)).unwrap_err(),
            "unknown option: --bogus"
        );
    }
}

#[test]
fn what_import_found_reads_for_people() {
    let out = ImportOutput {
        file: "a.ron".into(),
        read_as: vec![
            Imported {
                kind: "palette".into(),
                summary: "palette: 1 color".into(),
                text: Some("PaletteDef".into()),
            },
            Imported {
                kind: "names table".into(),
                summary: "names table: 1 name".into(),
                text: Some("Names\n".into()),
            },
        ],
        errors: vec![Problem::from(
            &ContentError::new("a.ron", "bad").at(1, None),
        )],
    };
    assert_eq!(
        text(&out),
        "import: a.ron is palette: 1 color\nPaletteDef\nimport: a.ron is names table: 1 \
         name\nNames\nimport: 1 error(s)\n  a.ron:1: bad\n"
    );
    // A problem with a code says it, and its help follows the list.
    let coded = ImportOutput {
        read_as: Vec::new(),
        errors: vec![Problem::from(
            &ContentError::new("a.ron", "Expected comma")
                .at(3, Some(4))
                .with_code(shuyi_assets::error::codes::PARSE),
        )],
        ..out
    };
    let help = crate::contract::codes::help_of("SH2001").unwrap();
    assert_eq!(
        text(&coded),
        format!(
            "import: 1 error(s)\n  a.ron:3:4: error[SH2001]: Expected comma\nhelp[SH2001]: {help}\n"
        )
    );
}

#[test]
fn a_ron_syntax_error_is_a_parse_problem_with_its_help() {
    let dir = scratch("syntax");
    fs::write(dir.join("colors.ron"), "{ \"text\" \"#ffffff\" }").unwrap();
    let answer = import(
        &Importers::standard(),
        &dir,
        &input("colors.ron", Some("palette"), false),
    );
    assert!(!answer.ok);
    assert_eq!(answer.exit_code(), 1);
    let json: serde_json::Value = serde_json::from_str(&answer.to_json()).unwrap();
    let problem = &json["result"]["errors"][0];
    assert_eq!(problem["code"], "SH2001", "{problem}");
    assert_eq!(
        problem["help"],
        crate::contract::codes::help_of("SH2001").unwrap()
    );
    assert_eq!(problem["line"], 1);
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_new_input_reads_every_kind_and_prints_nothing() {
    let literal = ImportInput {
        file: "a.map".to_owned(),
        kind: None,
        print: false,
    };
    assert_eq!(ImportInput::new("a.map"), literal);
}
