use std::path::Path;

use super::*;
use crate::api::Ran;

/// The index of `fixture/lib.rs`, from the JSON the pinned nightly wrote.
fn fixture() -> Index {
    Index::read(include_str!("fixture/fixture.json")).unwrap()
}

#[test]
fn the_reader_names_the_crate_and_finds_an_item_with_its_docs() {
    let index = fixture();
    assert_eq!(index.krate, "fixture");
    let countdown = index.get("fixture::timer::Countdown").unwrap();
    assert_eq!(countdown.kind, "struct");
    assert!(
        countdown
            .docs
            .starts_with("A timer: it fires once, after its length.")
    );
    assert_eq!(countdown.deprecated, None);
}

#[test]
fn an_item_is_found_at_each_path_it_is_reexported_at() {
    let index = fixture();
    let countdown = index.get("fixture::timer::Countdown").unwrap();
    assert_eq!(
        countdown.paths,
        ["fixture::Countdown", "fixture::timer::Countdown"]
    );
    assert_eq!(index.get("fixture::Countdown"), Some(countdown));
    // A glob re-export, and a private module's item re-exported.
    assert!(index.get("fixture::Align").is_some());
    assert!(index.get("fixture::Tick").is_some());
    assert_eq!(
        index.get("fixture::Inner").unwrap().paths,
        ["fixture::Inner"]
    );
    assert!(index.get("fixture::hidden::Inner").is_none());
}

#[test]
fn the_aliases_are_read_in_both_forms() {
    let countdown = fixture().get("fixture::Countdown").unwrap().clone();
    assert_eq!(countdown.aliases, ["Timer", "Alarm", "Delay"]);
}

#[test]
fn a_deprecation_is_read_with_its_note() {
    let index = fixture();
    let reset = index.get("fixture::timer::Countdown::reset").unwrap();
    assert_eq!(reset.kind, "function");
    assert_eq!(reset.deprecated.as_deref(), Some("renamed `restart`"));
}

#[test]
fn members_are_found_and_a_traits_impl_and_private_items_are_not() {
    let index = fixture();
    for path in [
        "fixture::timer::Countdown::restart",
        "fixture::timer::Countdown::length",
        "fixture::Countdown::restart",
        "fixture::timer::Align::Centered",
        "fixture::timer::Tick::tick",
    ] {
        assert!(index.get(path).is_some(), "{path}");
    }
    for path in [
        "fixture::timer::Countdown::clone",
        "fixture::timer::Countdown::hidden",
        "fixture::timer::Countdown::left",
    ] {
        assert!(index.get(path).is_none(), "{path}");
    }
    assert_eq!(
        index.get("fixture::timer::Align::Centered").unwrap().kind,
        "variant"
    );
}

#[test]
fn doc_aliases_reads_one_word_or_a_list_and_nothing_else() {
    assert_eq!(doc_aliases("#[doc(alias = \"Timer\")]"), ["Timer"]);
    assert_eq!(doc_aliases("#[doc(alias(\"a\", \"b\"))]"), ["a", "b"]);
    assert!(doc_aliases("#[doc(hidden)]").is_empty());
    assert!(doc_aliases("#[must_use]").is_empty());
}

#[test]
fn read_fails_on_json_without_a_root() {
    assert!(Index::read("{").unwrap_err().contains("rustdoc JSON"));
    assert!(Index::read("{\"index\": {}}").unwrap_err().contains("root"));
}

/// A runner that answers one command and records it.
struct Fake {
    ok: bool,
    seen: std::cell::RefCell<Vec<String>>,
}

impl Runner for Fake {
    fn run(&self, _dir: &Path, program: &str, args: &[&str]) -> Result<Ran, String> {
        self.seen
            .borrow_mut()
            .push(format!("{program} {}", args.join(" ")));
        Ok(Ran {
            ok: self.ok,
            stdout: String::new(),
            stderr: "no".to_owned(),
        })
    }
}

#[test]
fn build_runs_rustdoc_on_the_nightly_with_the_features_and_reads_its_file() {
    let dir = std::env::temp_dir().join(format!("rustdoc-index-{}", std::process::id()));
    let doc = dir.join("doc");
    fs::create_dir_all(&doc).unwrap();
    fs::copy(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("src/rustdoc_index/fixture/fixture.json"),
        doc.join("fixture.json"),
    )
    .unwrap();
    let fake = Fake {
        ok: true,
        seen: std::cell::RefCell::default(),
    };
    let index = build(
        Path::new("."),
        "fixture",
        &["--features", "std"],
        &dir,
        &fake,
    )
    .unwrap();
    assert_eq!(index.krate, "fixture");
    let seen = fake.seen.borrow();
    assert_eq!(seen.len(), 1);
    assert!(
        seen[0].starts_with(&format!(
            "cargo +{NIGHTLY} rustdoc -p fixture --lib --features std --target-dir "
        )),
        "{}",
        seen[0]
    );
    assert!(seen[0].ends_with("-- -Z unstable-options --output-format json"));
    let failing = Fake {
        ok: false,
        seen: std::cell::RefCell::default(),
    };
    let error = build(Path::new("."), "fixture", &[], &dir, &failing).unwrap_err();
    assert!(error.contains("failed") && error.contains("no"), "{error}");
    fs::remove_dir_all(&dir).unwrap();
}
