use std::cell::RefCell;

use super::*;

/// A temporary folder for one test, empty.
fn temp(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("xtask-script-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// A fake runner: each command's answer from `answer`, by its arguments
/// joined with spaces; every call kept.
struct Fake<F: Fn(&str) -> Ran> {
    answer: F,
    calls: RefCell<Vec<(PathBuf, String, String)>>,
}

impl<F: Fn(&str) -> Ran> Fake<F> {
    fn new(answer: F) -> Self {
        Self {
            answer,
            calls: RefCell::new(Vec::new()),
        }
    }
}

impl<F: Fn(&str) -> Ran> Runner for Fake<F> {
    fn run(&self, dir: &Path, program: &str, args: &[String]) -> Result<Ran, String> {
        let joined = args.join(" ");
        self.calls
            .borrow_mut()
            .push((dir.to_path_buf(), program.to_owned(), joined.clone()));
        Ok((self.answer)(&joined))
    }
}

/// What a command that printed `stdout` and exited with `code` did.
fn ran(code: i32, stdout: &str) -> Ran {
    Ran {
        code: Some(code),
        stdout: stdout.to_owned(),
        stderr: String::new(),
    }
}

fn shuyi(args: &[&str], ok: bool, expect: Vec<Expect>) -> Step {
    Step::Shuyi {
        args: args.iter().map(|a| (*a).to_owned()).collect(),
        ok,
        expect,
        save: None,
    }
}

const TITLE: &str = r#"{"contract": 17, "op": "run", "ok": true, "result": {"frames": 2, "top_screen": "title", "scene": null, "screens": ["title"], "quit": false}, "limits": []}"#;

#[test]
fn a_contract_step_runs_cargo_shuyi_with_json_in_the_game() {
    let game = temp("contract");
    let fake = Fake::new(|_| ran(0, TITLE));
    let step = shuyi(
        &["run", "--script", "Enter Right", "--frames", "1"],
        true,
        vec![Expect::Is(
            "/result/top_screen".to_owned(),
            "title".to_owned(),
        )],
    );
    let verdicts = run(&[step], &game, &game, &fake);
    assert_eq!(verdicts.len(), 1);
    assert!(verdicts[0].passed, "{verdicts:?}");
    assert_eq!(verdicts[0].step, 1);
    assert_eq!(
        verdicts[0].what,
        "cargo shuyi run --script \"Enter Right\" --frames 1 --json"
    );
    let calls = fake.calls.borrow();
    assert_eq!(
        calls[0],
        (
            game.clone(),
            "cargo".to_owned(),
            "shuyi run --script Enter Right --frames 1 --json".to_owned()
        )
    );
    let _ = fs::remove_dir_all(&game);
}

#[test]
fn an_answer_that_isnt_ok_fails_with_its_error_and_code() {
    let game = temp("not-ok");
    let fake = Fake::new(|_| {
        ran(
            3,
            r#"{"ok": false, "error": "shuyi.ron names no rules", "code": "SH1002", "limits": ["a limit"]}"#,
        )
    });
    let verdicts = run(&[shuyi(&["lint"], true, Vec::new())], &game, &game, &fake);
    let detail = verdicts[0].detail.clone().unwrap();
    assert!(!verdicts[0].passed);
    assert!(
        detail.starts_with("the answer's ok is false, not true"),
        "{detail}"
    );
    assert!(
        detail.contains("error: shuyi.ron names no rules"),
        "{detail}"
    );
    assert!(detail.contains("code: SH1002"), "{detail}");
    assert!(detail.contains(r#"limits: ["a limit"]"#), "{detail}");
    // A step that expects the failure passes, and reads its code.
    let expected = shuyi(
        &["lint"],
        false,
        vec![Expect::Is("/code".to_owned(), "SH1002".to_owned())],
    );
    assert!(run(&[expected], &game, &game, &fake)[0].passed);
    let _ = fs::remove_dir_all(&game);
}

#[test]
fn the_exit_code_must_agree_with_the_answer() {
    let game = temp("exit");
    let fake = Fake::new(|_| ran(1, TITLE));
    let verdicts = run(&[shuyi(&["run"], true, Vec::new())], &game, &game, &fake);
    assert_eq!(
        verdicts[0].detail.as_deref(),
        Some("the answer's ok is true, but it exited with 1")
    );
    let _ = fs::remove_dir_all(&game);
}

#[test]
fn no_json_fails_with_the_last_lines_of_the_output() {
    let game = temp("no-json");
    let fake = Fake::new(|_| Ran {
        code: Some(101),
        stdout: String::new(),
        stderr: (1..=30).fold(String::new(), |mut lines, n| {
            let _ = writeln!(lines, "line {n}");
            lines
        }),
    });
    let verdicts = run(&[shuyi(&["check"], true, Vec::new())], &game, &game, &fake);
    let detail = verdicts[0].detail.clone().unwrap();
    assert!(detail.starts_with("no JSON answer"), "{detail}");
    assert!(detail.contains("it exited with 101"), "{detail}");
    assert!(
        detail.contains("line 11\n") && detail.ends_with("line 30"),
        "{detail}"
    );
    assert!(!detail.contains("line 10\n"), "{detail}");
    let _ = fs::remove_dir_all(&game);
}

#[test]
fn what_is_and_has_and_holds_compare() {
    let answer: Value = serde_json::from_str(
        r#"{"ok": true, "result": {"name": "title", "frames": 2, "quit": false,
            "scene": null, "files": ["AGENTS.md", "shuyi.ron"], "text": "a long line"}}"#,
    )
    .unwrap();
    let is = |p: &str, v: &str| holds(&answer, &Expect::Is(p.to_owned(), v.to_owned()));
    assert_eq!(is("/result/name", "title"), Ok(()));
    assert_eq!(is("/result/frames", "2"), Ok(()));
    assert_eq!(is("/result/quit", "false"), Ok(()));
    assert_eq!(is("/result/scene", "null"), Ok(()));
    assert_eq!(
        is("/result/name", "\"title\""),
        Err("/result/name is \"title\", not \"title\"".to_owned())
    );
    assert_eq!(
        is("/result/nope", "1"),
        Err("no value at /result/nope".to_owned())
    );
    let has = |p: &str| holds(&answer, &Expect::Has(p.to_owned()));
    assert_eq!(has("/result/files"), Ok(()));
    assert_eq!(
        has("/result/scene"),
        Err("/result/scene is null".to_owned())
    );
    let held = |p: &str, v: &str| holds(&answer, &Expect::Holds(p.to_owned(), v.to_owned()));
    assert_eq!(held("/result/files", "shuyi.ron"), Ok(()));
    assert_eq!(held("/result/text", "long"), Ok(()));
    assert_eq!(
        held("/result/files", "README.md"),
        Err(r#"/result/files doesn't hold README.md: it is ["AGENTS.md","shuyi.ron"]"#.to_owned())
    );
    assert_eq!(
        held("/result/frames", "2"),
        Err("/result/frames is no array or string to hold 2: it is 2".to_owned())
    );
}

#[test]
fn a_saved_value_is_written_to_the_game() {
    let game = temp("save");
    let fake = Fake::new(|_| {
        ran(
            0,
            r#"{"ok": true, "result": {"plays": [{"record": "(seed: 3, commands: [])", "turns": 4}]}}"#,
        )
    });
    let step = |pointer: &str, file: &str| Step::Shuyi {
        args: vec!["bots".to_owned(), "1".to_owned()],
        ok: true,
        expect: Vec::new(),
        save: Some((pointer.to_owned(), file.to_owned())),
    };
    let verdicts = run(
        &[
            step("/result/plays/0/record", "records/one.ron"),
            step("/result/plays/0/turns", "turns.txt"),
            step("/result/nope", "nope.txt"),
        ],
        &game,
        &game,
        &fake,
    );
    assert!(verdicts[0].passed && verdicts[1].passed, "{verdicts:?}");
    assert_eq!(
        fs::read_to_string(game.join("records/one.ron")).unwrap(),
        "(seed: 3, commands: [])"
    );
    assert_eq!(fs::read_to_string(game.join("turns.txt")).unwrap(), "4");
    assert_eq!(
        verdicts[2].detail.as_deref(),
        Some("save: no value at /result/nope")
    );
    let _ = fs::remove_dir_all(&game);
}

#[test]
fn a_cargo_step_succeeds_or_fails_as_it_says() {
    let game = temp("cargo");
    let fake = Fake::new(|args| {
        if args.starts_with("test") {
            ran(0, "")
        } else {
            Ran {
                code: Some(101),
                stdout: String::new(),
                stderr: "error: a test failed".to_owned(),
            }
        }
    });
    let cargo = |args: &str, ok: bool| Step::Cargo {
        args: args.split(' ').map(str::to_owned).collect(),
        ok,
    };
    let verdicts = run(
        &[
            cargo("test --workspace", true),
            cargo("clippy -- -D warnings", false),
            cargo("clippy -- -D warnings", true),
            cargo("test --workspace", false),
        ],
        &game,
        &game,
        &fake,
    );
    let passed: Vec<bool> = verdicts.iter().map(|v| v.passed).collect();
    assert_eq!(passed, [true, true, false, false]);
    assert_eq!(verdicts[1].what, "cargo clippy -- -D warnings (fails)");
    assert_eq!(
        verdicts[2].detail.as_deref(),
        Some("it exited with 101, and should succeed\nerror: a test failed")
    );
    assert_eq!(
        verdicts[3].detail.as_deref(),
        Some("it exited with 0, and should fail\n")
    );
    let _ = fs::remove_dir_all(&game);
}

#[test]
fn files_are_written_edited_and_read_inside_the_game() {
    let files = temp("files-from");
    let game = temp("files-game");
    fs::create_dir_all(files.join("example")).unwrap();
    fs::write(files.join("example/about.rs"), "//! About.\n").unwrap();
    fs::write(game.join("lib.rs"), "pub mod title;\n").unwrap();
    let fake = Fake::new(|_| ran(0, ""));
    let steps = [
        Step::Write {
            to: "crates/game/src/about.rs".to_owned(),
            from: "example/about.rs".to_owned(),
        },
        Step::Edit {
            file: "lib.rs".to_owned(),
            find: "pub mod title;".to_owned(),
            replace: "pub mod about;\npub mod title;".to_owned(),
        },
        Step::Holds {
            file: "lib.rs".to_owned(),
            text: "pub mod about;".to_owned(),
        },
        Step::Holds {
            file: "crates/game/src/about.rs".to_owned(),
            text: String::new(),
        },
        Step::Edit {
            file: "lib.rs".to_owned(),
            find: "pub mod nope;".to_owned(),
            replace: String::new(),
        },
        Step::Edit {
            file: "lib.rs".to_owned(),
            find: "pub mod".to_owned(),
            replace: String::new(),
        },
        Step::Holds {
            file: "lib.rs".to_owned(),
            text: "pub mod nope;".to_owned(),
        },
        Step::Write {
            to: "../outside.rs".to_owned(),
            from: "example/about.rs".to_owned(),
        },
        Step::Holds {
            file: "missing.rs".to_owned(),
            text: String::new(),
        },
    ];
    let verdicts = run(&steps, &files, &game, &fake);
    let passed: Vec<bool> = verdicts.iter().map(|v| v.passed).collect();
    assert_eq!(
        passed,
        [true, true, true, true, false, false, false, false, false]
    );
    assert_eq!(
        fs::read_to_string(game.join("crates/game/src/about.rs")).unwrap(),
        "//! About.\n"
    );
    assert_eq!(
        verdicts[4].detail.as_deref(),
        Some("lib.rs doesn't hold the text to replace: \"pub mod nope;\"")
    );
    assert_eq!(
        verdicts[5].detail.as_deref(),
        Some("lib.rs holds the text to replace 2 times, not once: \"pub mod\"")
    );
    assert_eq!(
        verdicts[6].detail.as_deref(),
        Some("lib.rs doesn't hold \"pub mod nope;\"")
    );
    assert_eq!(
        verdicts[7].detail.as_deref(),
        Some("\"../outside.rs\": a path from the folder, with no `..`, is needed")
    );
    assert!(!game.join("../outside.rs").exists());
    assert_eq!(verdicts[3].what, "crates/game/src/about.rs exists");
    assert_eq!(verdicts[2].what, "lib.rs holds its text");
    assert!(fake.calls.borrow().is_empty());
    let _ = fs::remove_dir_all(&files);
    let _ = fs::remove_dir_all(&game);
}

#[test]
fn it_goes_on_after_a_failed_step() {
    let game = temp("goes-on");
    let fake = Fake::new(|args| {
        if args.starts_with("shuyi check") {
            ran(1, r#"{"ok": false, "error": "two goals"}"#)
        } else {
            ran(0, TITLE)
        }
    });
    let steps = [
        shuyi(&["check"], true, Vec::new()),
        shuyi(&["run"], true, Vec::new()),
    ];
    let verdicts = run(&steps, &game, &game, &fake);
    assert_eq!(
        verdicts
            .iter()
            .map(|v| (v.step, v.passed))
            .collect::<Vec<_>>(),
        [(1, false), (2, true)]
    );
    let text = report(&verdicts);
    assert!(
        text.contains(" 1. FAILED cargo shuyi check --json ("),
        "{text}"
    );
    assert!(
        text.contains("the answer's ok is false, not true; error: two goals"),
        "{text}"
    );
    assert!(
        text.contains(" 2. ok     cargo shuyi run --json ("),
        "{text}"
    );
    let _ = fs::remove_dir_all(&game);
}

#[test]
fn steps_read_from_ron_with_their_defaults() {
    let steps: Vec<Step> = ron::from_str(
        r#"[
            Shuyi(args: ["run", "--frames", "1"], expect: [Is("/result/top_screen", "title"), Has("/result"), Holds("/limits", "x")]),
            Shuyi(args: ["lint"], ok: false, save: Some(("/result", "lint.json"))),
            Cargo(args: ["test"]),
            Write(to: "a.rs", from: "example/a.rs"),
            Edit(file: "a.rs", find: "x", replace: "y"),
            Holds(file: "a.rs"),
        ]"#,
    )
    .unwrap();
    assert_eq!(
        steps[0],
        Step::Shuyi {
            args: vec!["run".to_owned(), "--frames".to_owned(), "1".to_owned()],
            ok: true,
            expect: vec![
                Expect::Is("/result/top_screen".to_owned(), "title".to_owned()),
                Expect::Has("/result".to_owned()),
                Expect::Holds("/limits".to_owned(), "x".to_owned()),
            ],
            save: None,
        }
    );
    assert!(matches!(
        &steps[1],
        Step::Shuyi {
            ok: false,
            save: Some(_),
            ..
        }
    ));
    assert_eq!(
        steps[2],
        Step::Cargo {
            args: vec!["test".to_owned()],
            ok: true
        }
    );
    assert_eq!(
        steps[5],
        Step::Holds {
            file: "a.rs".to_owned(),
            text: String::new()
        }
    );
    assert!(ron::from_str::<Vec<Step>>("[Cargo(args: [], nope: 1)]").is_err());
}
