//! The answer every operation gives, and the operations' names.

use super::*;

#[test]
fn every_op_has_a_name_it_is_found_by() {
    for op in Op::ALL {
        assert_eq!(Op::named(op.name()), Some(op));
        let json = serde_json::to_string(&op).unwrap();
        assert_eq!(json, format!("\"{}\"", op.name()));
    }
    assert_eq!(Op::named("frame-png"), None);
}

#[test]
fn an_answer_carries_the_version_its_verdict_and_its_limits() {
    let answer = Answer::new(Op::Check, true, 7u8, Vec::new()).limited("no line");
    assert_eq!(answer.contract, VERSION);
    assert_eq!(answer.exit_code(), 0);
    assert_eq!(answer.limits, ["no line"]);
    assert_eq!(
        answer.to_text(|n| format!("{n} found\n")),
        "7 found\nlimits:\n  - no line\n"
    );
    let json: serde_json::Value = serde_json::from_str(&answer.to_json()).unwrap();
    assert_eq!(json["contract"], VERSION);
    assert_eq!(json["op"], "check");
    assert_eq!(json["ok"], true);
    assert_eq!(json["result"], 7);
    assert!(json["error"].is_null());
    let back: Answer<u8> = serde_json::from_str(&answer.to_json()).unwrap();
    assert_eq!(back, answer);
}

#[test]
fn an_answer_that_could_not_run_says_why() {
    let answer = Answer::<u8>::failed(Op::Run, "no game");
    assert!(!answer.ok);
    assert_eq!(answer.exit_code(), 1);
    assert_eq!(answer.result, None);
    assert_eq!(answer.to_text(|_| unreachable!()), "run: no game\n");
    let empty = Answer::<u8> {
        error: None,
        ..answer
    };
    assert_eq!(empty.to_text(|_| unreachable!()), "");
}

#[test]
fn a_problem_shows_where_it_is() {
    let error = ContentError::new("a.ron", "bad").at(3, Some(4));
    let problem = Problem::from(&error);
    assert_eq!(problem.to_string(), "a.ron:3:4: bad");
    assert_eq!(problem.to_string(), error.to_string());
    let line = Problem::from(&ContentError::new("b.map", "worse").at(2, None));
    assert_eq!(line.to_string(), "b.map:2: worse");
    assert_eq!(
        Problem::from(&ContentError::new("c", "whole")).to_string(),
        "c: whole"
    );
}

#[test]
fn every_op_has_an_input_and_an_output_schema() {
    let files = schemas();
    let names: Vec<&str> = files.iter().map(|f| f.name.as_str()).collect();
    assert_eq!(names.len(), Op::ALL.len() * 2);
    for op in Op::ALL {
        for side in ["input", "output"] {
            let name = format!("{}.{side}.json", op.name());
            assert!(names.contains(&name.as_str()), "{name}");
        }
    }
    for file in &files {
        assert!(file.text.ends_with("}\n"), "{}", file.name);
        let schema: serde_json::Value = serde_json::from_str(&file.text).unwrap();
        assert!(schema["$schema"].is_string(), "{}", file.name);
    }
    // The fingerprint follows the text.
    let mut changed = files.clone();
    changed[0].text.push(' ');
    assert_ne!(fingerprint(&changed), fingerprint(&files));
    assert_eq!(fingerprint(&files).len(), 16);
}
