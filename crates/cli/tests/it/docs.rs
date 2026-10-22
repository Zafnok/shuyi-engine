//! `docs/contract.md` and the README against the code (ticket 2017): the
//! example answer is one the contract writes, and every operation has its
//! row in the operations table and its name in the README.

use std::fs;
use std::path::{Path, PathBuf};

use shuyi_cli::contract::run::SceneOutput;
use shuyi_cli::contract::{Answer, Op, VERSION};

/// The repository's root: `crates/cli/../..`.
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// A file of the repository, with LF line endings.
fn read(path: &str) -> String {
    fs::read_to_string(root().join(path))
        .unwrap_or_else(|e| panic!("{path}: {e}"))
        .replace("\r\n", "\n")
}

/// The lines of `text` after the line `heading`, up to the next `## `.
fn section<'a>(text: &'a str, heading: &str) -> Vec<&'a str> {
    text.lines()
        .skip_while(|l| *l != heading)
        .skip(1)
        .take_while(|l| !l.starts_with("## "))
        .collect()
}

/// The first fenced `json` block under `## The answer`.
fn example_answer(contract: &str) -> String {
    section(contract, "## The answer")
        .into_iter()
        .skip_while(|l| *l != "```json")
        .skip(1)
        .take_while(|l| *l != "```")
        .collect::<Vec<_>>()
        .join("\n")
}

/// The first word inside a line's first backticks.
fn first_name(line: &str) -> Option<&str> {
    line.split('`').nth(1)?.split_whitespace().next()
}

#[test]
fn the_example_answer_is_one_the_contract_writes() {
    let json = example_answer(&read("docs/contract.md"));
    assert!(
        !json.is_empty(),
        "docs/contract.md has no ```json block under `## The answer`: put the example answer there"
    );
    let answer: Answer<SceneOutput> = serde_json::from_str(&json).unwrap_or_else(|e| {
        panic!(
            "docs/contract.md's example answer is not an `Answer<SceneOutput>` ({e}): change \
             the example to what `shuyi scene --json` writes"
        )
    });
    let written: serde_json::Value = serde_json::to_value(&answer).unwrap();
    let shown: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert!(
        written == shown,
        "docs/contract.md's example answer leaves out a field the contract writes: show every \
         field, as in {written}"
    );
    assert_eq!(
        answer.op,
        Op::Scene,
        "docs/contract.md's example answer is a `scene` answer: set its `op` to \"scene\""
    );
    assert_eq!(
        answer.contract, VERSION,
        "docs/contract.md's example answer shows contract {}: set it to {VERSION}, \
         contract::VERSION",
        answer.contract
    );
}

#[test]
fn every_operation_has_its_row_in_the_operations_table() {
    let contract = read("docs/contract.md");
    let rows: Vec<&str> = section(&contract, "## The operations")
        .into_iter()
        .filter(|l| l.starts_with("| `"))
        .filter_map(first_name)
        .collect();
    for op in Op::ALL {
        assert!(
            rows.contains(&op.name()),
            "docs/contract.md's table under `## The operations` has no row for `{}`: add one",
            op.name()
        );
    }
    for row in rows {
        assert!(
            row == "schema" || Op::named(row).is_some(),
            "docs/contract.md's table under `## The operations` has a row for `{row}`, which is \
             no operation: remove it, or add the operation to `Op`"
        );
    }
}

#[test]
fn the_readme_names_every_operation() {
    let readme = read("README.md");
    let row = readme
        .lines()
        .find(|l| l.starts_with("| `shuyi-cli`"))
        .expect("README.md has no `shuyi-cli` row in its crates table");
    for op in Op::ALL {
        assert!(
            row.contains(&format!("`{}`", op.name())),
            "README.md's `shuyi-cli` row doesn't name the operation `{}`: add it to the list",
            op.name()
        );
    }
}
