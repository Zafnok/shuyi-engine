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
    assert!(json["code"].is_null());
    assert!(json["help"].is_null());
    let back: Answer<u8> = serde_json::from_str(&answer.to_json()).unwrap();
    assert_eq!(back, answer);
}

#[test]
fn an_answer_that_could_not_run_says_why_with_its_code_and_help() {
    let answer = Answer::<u8>::failed(Op::Run, codes::GAME_ERROR, "no game");
    assert!(!answer.ok);
    assert_eq!(answer.exit_code(), 1);
    assert_eq!(answer.result, None);
    assert_eq!(answer.code.as_deref(), Some("SH3002"));
    let help = codes::help_of("SH3002").unwrap();
    assert_eq!(answer.help.as_deref(), Some(help));
    assert_eq!(
        answer.to_text(|_| unreachable!()),
        format!("run: error[SH3002]: no game\n  help: {help}\n  see: shuyi explain SH3002\n")
    );
    let json: serde_json::Value = serde_json::from_str(&answer.to_json()).unwrap();
    assert_eq!(json["code"], "SH3002");
    assert_eq!(json["help"], help);
    let back: Answer<u8> = serde_json::from_str(&answer.to_json()).unwrap();
    assert_eq!(back, answer);
    // Without a code, as before.
    let bare = Answer::<u8> {
        code: None,
        help: None,
        ..answer.clone()
    };
    assert_eq!(bare.exit_code(), 1);
    assert_eq!(bare.to_text(|_| unreachable!()), "run: no game\n");
    let empty = Answer::<u8> {
        error: None,
        ..bare
    };
    assert_eq!(empty.to_text(|_| unreachable!()), "");
    // A sharper help.
    let usage = answer.with_help("usage: run");
    assert_eq!(usage.help.as_deref(), Some("usage: run"));
}

#[test]
fn the_exit_code_is_the_codes_class() {
    let failed = |code| Answer::<u8>::failed(Op::Check, code, "x").exit_code();
    assert_eq!(failed(codes::WRONG_ARGUMENTS), 2);
    assert_eq!(failed(codes::NO_PROJECT), 3);
    assert_eq!(failed(codes::PROJECT_FILE), 3);
    assert_eq!(failed(codes::TOOLCHAIN), 3);
    assert_eq!(failed(shuyi_assets::error::codes::PARSE), 1);
    assert_eq!(failed(codes::FILE), 1);
    // An answer that ran and failed on the setup; a pass is 0 whatever.
    let ran = Answer::new(Op::Doctor, false, 1u8, Vec::new()).with_code(codes::TOOLCHAIN);
    assert_eq!(ran.exit_code(), 3);
    assert!(
        ran.to_text(|_| "checks\n".to_owned())
            .starts_with("checks\ndoctor: error[SH1003]: A piece of the toolchain is missing\n")
    );
    let passed = Answer::new(Op::Doctor, true, 1u8, Vec::new()).with_code(codes::TOOLCHAIN);
    assert_eq!(passed.exit_code(), 0);
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
    assert_eq!(problem.code, None);
    let json: serde_json::Value = serde_json::to_value(&problem).unwrap();
    assert!(json["code"].is_null() && json["help"].is_null());
}

#[test]
fn a_problem_with_a_code_says_it_with_its_rows_help() {
    let error = ContentError::new("files/stage.map", "expected ','")
        .at(3, Some(4))
        .with_code(shuyi_assets::error::codes::PARSE);
    let problem = Problem::from(&error);
    assert_eq!(
        problem.to_string(),
        "files/stage.map:3:4: error[SH2001]: expected ','"
    );
    assert_eq!(problem.code.as_deref(), Some("SH2001"));
    assert_eq!(problem.help.as_deref(), codes::help_of("SH2001"));
    let json: serde_json::Value = serde_json::to_value(&problem).unwrap();
    assert_eq!(json["code"], "SH2001");
    let back: Problem = serde_json::from_value(json).unwrap();
    assert_eq!(back, problem);
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
