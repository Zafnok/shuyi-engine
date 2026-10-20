//! The contract's JSON schemas (ADR-0022): the files checked in under
//! `schemas/` are what the types generate, and they change only with the
//! contract's version.

use std::fs;
use std::path::{Path, PathBuf};

use shuyi_cli::contract::{VERSION, VersionLine, fingerprint, schemas};

fn dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("schemas")
}

/// What to do when a schema changed.
const REGENERATE: &str = "regenerate them with `cargo shuyi schema --out crates/cli/schemas` \
     from the repository's root, raise `contract::VERSION`, and add the line the version test \
     asks for to schemas/versions.json";

#[test]
fn the_checked_in_schemas_are_what_the_types_generate() {
    let files = schemas();
    for file in &files {
        let on_disk = fs::read_to_string(dir().join(&file.name))
            .unwrap_or_default()
            .replace("\r\n", "\n");
        assert!(
            on_disk == file.text,
            "schemas/{} is not what the contract's types generate: {REGENERATE}",
            file.name
        );
    }
    let mut checked_in: Vec<String> = fs::read_dir(dir())
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|name| name != "versions.json")
        .collect();
    checked_in.sort();
    let mut generated: Vec<String> = files.into_iter().map(|f| f.name).collect();
    generated.sort();
    assert_eq!(checked_in, generated, "a schema file of no operation");
}

#[test]
fn the_schemas_change_only_with_the_contracts_version() {
    let text = fs::read_to_string(dir().join("versions.json")).unwrap();
    let versions: Vec<VersionLine> = serde_json::from_str(&text).unwrap();
    let current = fingerprint(&schemas());
    let last = versions.last().expect("versions.json lists no version");
    assert_eq!(
        last.version, VERSION,
        "versions.json's last line is version {}, and contract::VERSION is {VERSION}",
        last.version
    );
    assert!(
        last.fingerprint == current,
        "the schemas changed, and contract::VERSION is still {VERSION}: raise it to {} and add \
         {{\"version\": {}, \"fingerprint\": \"{current}\"}} to schemas/versions.json",
        VERSION + 1,
        VERSION + 1
    );
    for pair in versions.windows(2) {
        assert_eq!(
            pair[1].version,
            pair[0].version + 1,
            "versions go up by one"
        );
        assert_ne!(
            pair[1].fingerprint, pair[0].fingerprint,
            "version {} changed no schema",
            pair[1].version
        );
    }
}
