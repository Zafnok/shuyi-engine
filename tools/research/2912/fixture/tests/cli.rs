//! The public batch and line-oriented oracle protocol used by the pilot.

use std::io::Write;
use std::process::{Command, Stdio};

use serde_json::Value;

fn binary() -> Command {
    Command::new(env!("CARGO_BIN_EXE_cloning-fixture"))
}

#[test]
fn batch_has_initial_and_final_records_with_an_atomic_refusal() {
    let output = binary()
        .args(["courier", "11", "left,right"])
        .output()
        .unwrap_or_else(|error| panic!("binary: {error}"));
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let answer: Value =
        serde_json::from_slice(&output.stdout).unwrap_or_else(|error| panic!("JSON: {error}"));
    assert_eq!(answer["game"], "courier");
    assert_eq!(answer["trace"].as_array().map(Vec::len), Some(3));
    assert_eq!(answer["trace"][1]["ok"], false);
    assert_eq!(answer["trace"][0]["hash"], answer["trace"][1]["hash"]);
    assert_eq!(answer["trace"][2], answer["final"]);
    assert_ne!(answer["trace"][1]["hash"], answer["final"]["hash"]);
}

#[test]
fn oracle_answers_one_line_per_query_and_preserves_state_on_refusal() {
    let mut child = binary()
        .args(["workshop", "11", "--oracle"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap_or_else(|error| panic!("spawn: {error}"));
    child
        .stdin
        .take()
        .unwrap_or_else(|| panic!("stdin pipe missing"))
        .write_all(b"state\ncraft\ngather\n")
        .unwrap_or_else(|error| panic!("write: {error}"));
    let output = child
        .wait_with_output()
        .unwrap_or_else(|error| panic!("wait: {error}"));
    assert!(output.status.success());
    let lines: Vec<Value> = output
        .stdout
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.is_empty())
        .map(|line| serde_json::from_slice(line).unwrap_or_else(|error| panic!("JSON: {error}")))
        .collect();
    assert_eq!(lines.len(), 3);
    assert_eq!(lines[0]["hash"], lines[1]["hash"]);
    assert_eq!(lines[1]["error"], "NoIngot");
    assert_ne!(lines[1]["hash"], lines[2]["hash"]);
}
