//! The contract's JSON schemas (ADR-0022 §5, ADR-0150): the files checked
//! in under `schemas/` are what the types generate, and each set of them
//! has its line in `schemas/versions.json`. A set that breaks a client
//! raises the contract's version by one; any other keeps it (`cargo xtask
//! contract` tells which, and CI's `api` job holds it).

use std::fs;
use std::path::{Path, PathBuf};

use shuyi_cli::contract::{VERSION, VersionLine, fingerprint, schemas};

fn dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("schemas")
}

/// What to do when a schema changed.
const REGENERATE: &str = "regenerate them with `cargo shuyi schema --out crates/cli/schemas` \
     from the repository's root, and add the line the version test asks for to \
     schemas/versions.json";

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
fn each_set_of_schemas_has_its_line() {
    let text = fs::read_to_string(dir().join("versions.json")).unwrap();
    let versions: Vec<VersionLine> = serde_json::from_str(&text).unwrap();
    let current = fingerprint(&schemas());
    let last = versions.last().expect("versions.json lists no version");
    let (kept, raised) = (last.version, last.version + 1);
    assert!(
        last.fingerprint == current,
        "the schemas changed: if they break no client, keep contract::VERSION at {kept} and \
         add {{\"version\": {kept}, \"fingerprint\": \"{current}\"}} to schemas/versions.json; \
         if they break one, raise it to {raised} and add {{\"version\": {raised}, \
         \"fingerprint\": \"{current}\"}}. `cargo xtask contract` lists what breaks (ADR-0150)"
    );
    assert_eq!(
        last.version, VERSION,
        "contract::VERSION is {VERSION}, and the schemas' line is version {kept}: a version \
         rises only with a line of changed schemas, which names it"
    );
    for pair in versions.windows(2) {
        assert!(
            pair[1].version == pair[0].version || pair[1].version == pair[0].version + 1,
            "versions.json's version {} follows {}: a line keeps the version, or raises it by one",
            pair[1].version,
            pair[0].version
        );
        assert_ne!(
            pair[1].fingerprint, pair[0].fingerprint,
            "versions.json's line after version {}'s changed no schema",
            pair[0].version
        );
    }
}
