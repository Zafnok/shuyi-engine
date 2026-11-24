use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use super::grade::{self, Lines, Over, left_out, numstat, web_dist, web_folder_holds};
use super::reference::{self, done_waits, places_engine_crate, refuse_overlay};
use super::task::{After, GATES, Input, Kind, Reference, Task, input_args};
use crate::script::{self, Expect, Ran, Runner, Step};

/// The repository's root.
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// A temporary folder for one test, empty.
fn temp(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("xtask-eval-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

const TASK: &str = r#"(
    id: "t",
    prompt: "Make me a game.",
    template: "grid",
    name: "T",
    time_box_minutes: 60,
    targets: ["desktop", "web"],
    checks: [
        Gates,
        Check,
        Targets,
        Web,
        Export("linux"),
        Responds(steps: [Act("Right"), Wait(0.5)]),
        Replay,
        Scene(steps: [Act("Confirm")], has: [("/turn", Some("o")), ("/winner", None)]),
        Scene(steps: [], has: []),
    ],
    reference: Overlay("evals/references/t"),
    unmeasured: [("a Playdate build", ["9999"])],
)"#;

fn task() -> Task {
    ron::from_str(TASK).unwrap()
}

#[test]
fn a_task_reads_with_its_defaults_and_refuses_an_unknown_field() {
    let task = task();
    assert_eq!(task.id, "t");
    assert_eq!(task.targets, ["desktop", "web"]);
    assert_eq!(
        task.reference,
        Reference::Overlay("evals/references/t".to_owned())
    );
    assert_eq!(task.waits_for, Vec::<String>::new());
    assert_eq!(
        task.unmeasured,
        [("a Playdate build".to_owned(), vec!["9999".to_owned()])]
    );
    assert_eq!(
        task.checks[5],
        Kind::Responds {
            steps: vec![Input::Act("Right".to_owned()), Input::Wait(0.5)]
        }
    );
    let unknown = TASK.replace(
        "time_box_minutes: 60,",
        "time_box_minutes: 60, model: \"x\",",
    );
    let error = ron::from_str::<Task>(&unknown).unwrap_err().to_string();
    assert!(error.contains("model"), "{error}");
}

#[test]
fn every_check_plans_its_steps_through_the_contract_with_actions_never_keys() {
    let scratch = Path::new("/scratch");
    let planned = task().plan(scratch);
    let names: Vec<&str> = planned.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(
        names,
        [
            "Gates 1: cargo fmt --all --check",
            "Gates 2: cargo clippy --workspace --all-targets -- -D warnings",
            "Gates 3: cargo test --workspace",
            "Gates 4: cargo shuyi check-keys",
            "Gates 5: cargo shuyi lint",
            "Check",
            "Targets",
            "Web",
            "Export linux",
            "Responds",
            "Replay",
            "Scene 1",
            "Scene 2",
        ]
    );
    assert_eq!(GATES.len(), 5);
    let whats = |i: usize| -> Vec<String> { planned[i].steps.iter().map(script::what).collect() };
    assert_eq!(
        whats(6),
        [
            "cargo shuyi check --target desktop --json",
            "cargo shuyi check --target web --json"
        ]
    );
    assert_eq!(
        whats(9),
        ["cargo shuyi run --act Right --wait 0.5 --json responds"]
    );
    let saved = |file: &str| scratch.join(file).to_string_lossy().into_owned();
    assert_eq!(planned[7].after, After::WebFolder(saved("export-web.json")));
    assert_eq!(planned[8].after, After::Package(saved("export-linux.json")));
    // Replay keeps the try's record outside the game, by its absolute path.
    let record = saved("record.ron");
    let contract = |i: usize, step: usize| match &planned[i].steps[step] {
        Step::Shuyi {
            args, expect, save, ..
        } => (args.clone(), expect.clone(), save.clone()),
        other => panic!("{other:?}"),
    };
    let pointer = "/result/0/entrants/0/tries/0/record".to_owned();
    assert_eq!(contract(10, 0).2, Some((pointer, record.clone())));
    assert_eq!(contract(10, 1).0, ["replay".to_owned(), record]);
    let (args, expect, _) = contract(11, 0);
    assert_eq!(args, ["scene", "--act", "Confirm"]);
    assert_eq!(
        expect,
        [
            Expect::Has("/result/scene".to_owned()),
            Expect::Is("/result/scene/data/turn".to_owned(), "o".to_owned()),
            Expect::Exists("/result/scene/data/winner".to_owned()),
        ]
    );
    // No step of any check names a key.
    for p in &planned {
        for step in &p.steps {
            let what = script::what(step);
            assert!(
                !what.contains("--keys") && !what.contains("--pad"),
                "{what}"
            );
        }
    }
    assert_eq!(input_args(&[]), Vec::<String>::new());
}

#[test]
fn has_matches_present_equal_a_string_without_quotes_and_missing() {
    let answer = json!({ "ok": true, "result": { "scene": { "data": {
        "turn": "o", "winner": "", "moves": 0, "won": false, "outcome": null } } } });
    let check = |pointer: &str, equals: Option<&str>| {
        let task = Task {
            checks: vec![Kind::Scene {
                steps: Vec::new(),
                has: vec![(pointer.to_owned(), equals.map(str::to_owned))],
            }],
            ..task()
        };
        let Step::Shuyi { expect, .. } = &task.plan(Path::new("/s"))[0].steps[0] else {
            panic!("no contract step");
        };
        expect.iter().try_for_each(|e| script::holds(&answer, e))
    };
    assert_eq!(check("/turn", Some("o")), Ok(()));
    assert_eq!(check("/winner", Some("")), Ok(()));
    assert_eq!(check("/moves", Some("0")), Ok(()));
    assert_eq!(check("/won", Some("false")), Ok(()));
    assert_eq!(check("/outcome", None), Ok(()));
    assert_eq!(
        check("/turn", Some("\"o\"")),
        Err("/result/scene/data/turn is \"o\", not \"o\"".to_owned())
    );
    assert_eq!(
        check("/score", None),
        Err("no value at /result/scene/data/score".to_owned())
    );
}

#[test]
fn numstat_counts_lines_files_and_binaries_and_leaves_out_builds_and_locks() {
    let files = [
        ("3\t1\t", "base/README.md", "game/README.md"),
        ("10\t0\t", "/dev/null", "game/crates/game/build.rs"),
        ("0\t4\t", "base/CLAUDE.md", "/dev/null"),
        ("-\t-\t", "/dev/null", "game/files/board.png"),
        ("900\t0\t", "/dev/null", "game/Cargo.lock"),
        (
            "5\t5\t",
            "base/crates/rules/Cargo.lock",
            "game/crates/rules/Cargo.lock",
        ),
        ("7\t0\t", "/dev/null", "game/target/debug/x"),
        ("7\t0\t", "/dev/null", "game/dist/web/index.html"),
        ("7\t0\t", "/dev/null", "game/web-out/game.wasm"),
        ("7\t0\t", "/dev/null", "game/.git/HEAD"),
    ];
    let fields: Vec<&str> = files.iter().flat_map(|&(c, b, a)| [c, b, a]).collect();
    let out = fields.join("\0") + "\0";
    assert_eq!(
        numstat(&out, ("base", Over::Everything), "game", Some("web-out")),
        Lines {
            added: 13,
            removed: 5,
            files: 4
        }
    );
    // A whole project: the base's CLAUDE.md, which it never had, is left
    // out.
    assert_eq!(
        numstat(&out, ("base", Over::GamesFiles), "game", Some("web-out")),
        Lines {
            added: 13,
            removed: 1,
            files: 3
        }
    );
    // Without a web folder of its own, `web-out/` counts.
    let everything = ("base", Over::Everything);
    assert_eq!(numstat(&out, everything, "game/", None).added, 20);
    assert!(left_out("target/x", None));
    assert!(!left_out("targets/x", None));
    assert!(!left_out("crates/target.rs", None));
    assert!(left_out("site/dist/web/a", Some("site/dist/web/")));
    assert_eq!(numstat("", everything, "game", None), Lines::default());
}

#[test]
fn the_web_folder_comes_from_shuyi_ron_and_its_listing_is_checked() {
    let shuyi = r#"// a comment
(
    command: "cargo shuyi",
    targets: { "web": (frame: (384, 384), scale: integer, text_px: 16) },
    web: (package: "g", bin: "g", title: "G", dist: "dist/web"),
)"#;
    assert_eq!(web_dist(shuyi).as_deref(), Some("dist/web"));
    // A whole project file, its other parts skipped.
    let tutorial = fs::read_to_string(root().join("site/tutorial/10-ship/shuyi.ron")).unwrap();
    assert_eq!(web_dist(&tutorial).as_deref(), Some("dist/web"));
    assert_eq!(web_dist("(command: \"cargo shuyi\")"), None);
    assert_eq!(web_dist("not ron"), None);
    let names = |n: &[&str]| -> Vec<String> { n.iter().map(|s| (*s).to_owned()).collect() };
    assert_eq!(
        web_folder_holds(&names(&["index.html", "g.wasm", "a.js"])),
        Ok(())
    );
    assert_eq!(
        web_folder_holds(&names(&["g.wasm"])),
        Err("no index.html".to_owned())
    );
    assert_eq!(
        web_folder_holds(&names(&["index.html", "a.wasm", "b.wasm"])),
        Err("2 .wasm files, not one".to_owned())
    );
}

/// A fake runner: each command's answer from `answer` by its program and
/// arguments joined with spaces.
struct Fake<F: Fn(&str) -> Ran>(F);

impl<F: Fn(&str) -> Ran> Runner for Fake<F> {
    fn run(&self, _: &Path, program: &str, args: &[String]) -> Result<Ran, String> {
        Ok((self.0)(&format!("{program} {}", args.join(" "))))
    }
}

fn ok(stdout: &str) -> Ran {
    Ran {
        code: Some(0),
        stdout: stdout.to_owned(),
        stderr: String::new(),
    }
}

#[test]
fn a_task_that_waits_or_a_missing_game_is_not_graded_and_the_result_has_every_field() {
    let game = temp("grade");
    let out = game.join("out/result.json");
    let fake = Fake(|command: &str| {
        if command.starts_with("git rev-parse") {
            ok("abc123\n")
        } else {
            ok(r#"{"ok": true, "result": {}}"#)
        }
    });
    let waits = Task {
        waits_for: vec!["2739".to_owned()],
        ..task()
    };
    let error = grade::to_file(&waits, &game, None, &root(), &out, &fake).unwrap_err();
    assert_eq!(error, "t waits for 2739: it isn't graded");
    let error = grade::to_file(&task(), &game, None, &root(), &out, &fake).unwrap_err();
    assert!(error.ends_with("no game (no shuyi.ron)"), "{error}");
    fs::write(game.join("shuyi.ron"), "()").unwrap();
    let only_check = Task {
        checks: vec![Kind::Check],
        ..task()
    };
    let outcome = grade::to_file(&only_check, &game, None, &root(), &out, &fake).unwrap();
    assert!(outcome.passed);
    let written: Value = serde_json::from_str(&fs::read_to_string(&out).unwrap()).unwrap();
    assert_eq!(written, outcome.result);
    let fields = "task engine checks passed total lines_added lines_removed files_changed";
    for field in fields.split(' ').chain(["seconds", "unmeasured", "limits"]) {
        assert!(written.get(field).is_some(), "no {field} in {written}");
    }
    assert_eq!(written["engine"], "abc123");
    assert_eq!(written["checks"][0]["check"], "Check");
    assert_eq!(written["lines_added"], Value::Null);
    assert_eq!(
        written["limits"][0],
        "no --base: the lines over the template weren't counted"
    );
    assert_eq!(written["unmeasured"][0]["waits_for"][0], "9999");
    // Nothing in the game but its shuyi.ron and the result.
    assert_eq!(fs::read_dir(&game).unwrap().count(), 2);
    let _ = fs::remove_dir_all(&game);
}

#[test]
fn a_failed_check_says_why_and_the_grader_goes_on() {
    let game = temp("failed");
    fs::write(game.join("shuyi.ron"), "()").unwrap();
    let fake = Fake(|command: &str| match command {
        "cargo shuyi check --json" => Ran {
            code: Some(1),
            stdout: r#"{"ok": false, "error": "en.ron: a missing key"}"#.to_owned(),
            stderr: String::new(),
        },
        "git rev-parse HEAD" => ok("abc"),
        _ => ok(r#"{"ok": true, "result": {"scene": {"data": {"turn": "x"}}}}"#),
    });
    let task = Task {
        checks: vec![
            Kind::Check,
            Kind::Scene {
                steps: vec![Input::Act("Confirm".to_owned())],
                has: vec![("/turn".to_owned(), Some("o".to_owned()))],
            },
            Kind::Targets,
        ],
        ..task()
    };
    let out = temp("failed-out").join("r.json");
    let outcome = grade::to_file(&task, &game, None, &root(), &out, &fake).unwrap();
    assert!(!outcome.passed);
    let checks = &outcome.result["checks"];
    assert_eq!(
        checks[0]["detail"],
        "cargo shuyi check --json: the answer's ok is false, not true; error: en.ron: a missing key"
    );
    assert_eq!(
        checks[1]["detail"],
        "cargo shuyi scene --act Confirm --json: /result/scene/data/turn is \"x\", not o"
    );
    assert_eq!(checks[2]["passed"], true);
    assert_eq!(outcome.result["passed"], 1);
    assert_eq!(outcome.result["total"], 3);
    assert!(
        outcome.report.contains("FAILED Scene 1 ("),
        "{}",
        outcome.report
    );
    let _ = fs::remove_dir_all(&game);
    let _ = fs::remove_dir_all(out.parent().unwrap());
}

#[test]
fn the_options_parse_and_refuse() {
    let args = |a: &[&str]| -> Vec<String> { a.iter().map(|s| (*s).to_owned()).collect() };
    let grade = grade::Options::parse(&args(&["t.ron", "game", "--base", "b"])).unwrap();
    assert_eq!(grade.base, Some(PathBuf::from("b")));
    assert_eq!(grade.out, PathBuf::from(grade::OUT));
    assert!(grade::Options::parse(&args(&["t.ron"])).is_err());
    assert!(grade::Options::parse(&args(&["t.ron", "g", "--bogus"])).is_err());
    let all = reference::Options::parse(&args(&["--all", "--out", "r"])).unwrap();
    assert_eq!(
        (all.id, all.out, all.template_only),
        (None, PathBuf::from("r"), false)
    );
    let one = reference::Options::parse(&args(&["box-pushing", "--template-only"])).unwrap();
    assert_eq!(
        (one.id.as_deref(), one.out, one.template_only),
        (Some("box-pushing"), PathBuf::from(reference::OUT), true)
    );
    for wrong in [
        &[][..],
        &["a", "--all"],
        &["a", "b"],
        &["--all", "--out"],
        &["--all", "--bogus"],
    ] {
        assert!(
            reference::Options::parse(&args(wrong)).is_err(),
            "{wrong:?}"
        );
    }
}

#[test]
fn the_waiting_warning_names_a_waits_for_ticket_that_is_done() {
    let root = temp("waits");
    fs::create_dir_all(root.join("tickets/done")).unwrap();
    fs::write(root.join("tickets/done/1127-a-platformer.md"), "").unwrap();
    fs::write(root.join("tickets/done/27390-another.md"), "").unwrap();
    let task = Task {
        id: "platformer".to_owned(),
        reference: Reference::None,
        waits_for: vec!["1127".to_owned(), "2739".to_owned()],
        ..task()
    };
    assert_eq!(
        done_waits(&root, &task),
        ["1127 is done: give `platformer` a reference and empty its `waits_for`"]
    );
    assert_eq!(done_waits(&root, &self::task()), Vec::<String>::new());
    let all = [task, self::task()];
    assert_eq!(
        reference::waiting(&all),
        ["platformer (waits for 1127, 2739)"]
    );
    assert_eq!(reference::pick(&all, None).unwrap(), [&all[1]]);
    assert_eq!(
        reference::pick(&all, Some("platformer")).unwrap(),
        [&all[0]]
    );
    assert_eq!(
        reference::pick(&all, Some("pong")).unwrap_err(),
        "no task pong in evals/tasks/"
    );
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn an_overlay_that_places_the_engines_crates_is_refused_with_exit_code_2() {
    for placed in [
        "shuyi = { path = \"../engine/crates/shuyi\" }",
        "  \"shuyi-cli\" = { git = \"https://x\" }",
        "shuyi-core = \"0.1\"",
        "shuyi.path = \"..\"",
        "[dependencies.shuyi-core]\npath = \"..\"",
        "[workspace.dependencies.shuyi]\nworkspace = true",
    ] {
        assert!(places_engine_crate(placed), "{placed}");
    }
    for allowed in [
        "[dependencies]\nshuyi.workspace = true\n",
        "shuyi-core = { workspace = true, features = [\"schema\"] }",
        "name = \"tic-tac-toe\"\nserde.workspace = true",
        "shuyiness = { path = \"x\" }",
    ] {
        assert!(!places_engine_crate(allowed), "{allowed}");
    }
    let root = temp("overlay");
    fs::create_dir_all(root.join("evals/tasks")).unwrap();
    fs::write(root.join("evals/tasks/t.ron"), TASK).unwrap();
    let overlay = root.join("evals/references/t");
    fs::create_dir_all(overlay.join("crates/game")).unwrap();
    fs::write(overlay.join("crates/game/build.rs"), "fn main() {}").unwrap();
    let member = "[dependencies]\nshuyi.workspace = true\n";
    fs::write(overlay.join("crates/game/Cargo.toml"), member).unwrap();
    assert_eq!(refuse_overlay(&overlay), None);
    let workspace = "[workspace.dependencies]\nshuyi = { path = \"../shuyi\" }\n";
    fs::write(overlay.join("Cargo.toml"), workspace).unwrap();
    let why = refuse_overlay(&overlay).unwrap();
    assert!(
        why.ends_with(
            "Cargo.toml: an overlay's Cargo.toml may not say where the engine's crates are \
             (`shuyi new --engine-path` does)"
        ),
        "{why}"
    );
    // Refused before any game is made.
    let never = Fake(|command: &str| panic!("ran {command}"));
    let args = ["t".to_owned()];
    assert_eq!(reference::command(&root, &args, &never, &never), 2);
    assert!(!root.join("target").exists());
    // An unknown id isn't graded.
    let pong = ["pong".to_owned()];
    assert_eq!(reference::command(&root, &pong, &never, &never), 2);
    // Nor a task file whose id isn't its name.
    fs::write(root.join("evals/tasks/u.ron"), TASK).unwrap();
    let errors = reference::tasks(&root).unwrap_err();
    assert!(
        errors[0].ends_with("u.ron: its id is t, not its file's name"),
        "{errors:?}"
    );
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn the_tasks_here_read_and_all_grades_box_pushing_and_tic_tac_toe_and_lists_the_waiting() {
    let tasks = reference::tasks(&root()).unwrap_or_else(|e| panic!("{e:?}"));
    for task in &tasks {
        assert_eq!(task.time_box_minutes, 60, "{}", task.id);
        // A task with no reference waits for the ticket that gives it one.
        assert_eq!(
            task.waits_for.is_empty(),
            task.reference != Reference::None,
            "{}",
            task.id
        );
        match &task.reference {
            Reference::Overlay(dir) => assert_eq!(refuse_overlay(&root().join(dir)), None),
            Reference::Project(dir) => {
                assert!(root().join(dir).join("shuyi.ron").is_file(), "{dir}");
            }
            Reference::None => {}
        }
    }
    let ids: Vec<&str> = tasks.iter().map(|t| t.id.as_str()).collect();
    assert_eq!(ids, ["box-pushing", "platformer", "snake", "tic-tac-toe"]);
    let graded: Vec<&str> = (reference::pick(&tasks, None).unwrap().iter())
        .map(|t| t.id.as_str())
        .collect();
    assert_eq!(graded, ["box-pushing", "tic-tac-toe"]);
    let waiting = reference::waiting(&tasks);
    for id in ["platformer", "snake"] {
        assert!(waiting.iter().any(|w| w.starts_with(id)), "{waiting:?}");
    }
}

/// The schema in `crates/cli/schemas/` of `op`'s answer.
fn schema(op: &str) -> Value {
    let file = root().join(format!("crates/cli/schemas/{op}.output.json"));
    serde_json::from_str(&fs::read_to_string(&file).unwrap()).unwrap()
}

/// The schema `pointer` (a path of property names; `0` an array's items)
/// reaches in `schema`, following `$ref`s and `anyOf`s; `None` when it
/// names no property there.
fn walk<'a>(schema: &'a Value, root: &'a Value, path: &[&str]) -> Option<&'a Value> {
    let resolved = |mut node: &'a Value| {
        loop {
            if let Some(r) = node.get("$ref").and_then(Value::as_str) {
                node = root.pointer(r.trim_start_matches('#'))?;
            } else if let Some(any) = node.get("anyOf").and_then(Value::as_array) {
                node = any.iter().find(|n| n.get("type") != Some(&json!("null")))?;
            } else {
                return Some(node);
            }
        }
    };
    let mut node = resolved(schema)?;
    for name in path {
        node = if *name == "0" {
            resolved(node.get("items")?)?
        } else {
            resolved(node.get("properties")?.get(*name)?)?
        };
    }
    Some(node)
}

#[test]
fn the_answers_paths_the_grader_reads_are_in_the_contracts_schemas() {
    for (op, path) in [
        ("run", &["ok"][..]),
        ("run", &["error"]),
        ("run", &["result", "frames"]),
        ("run", &["result", "frame"]),
        ("run", &["result", "scene", "data"]),
        ("scene", &["result", "scene", "data"]),
        ("check", &["ok"]),
        ("replay", &["result", "first_difference"]),
        (
            "playtest",
            &["result", "0", "entrants", "0", "tries", "0", "record"],
        ),
        ("export", &["result", "out"]),
        ("export", &["result", "folder"]),
        ("export", &["result", "package"]),
    ] {
        let schema = schema(op);
        let found = if op == "export" {
            // `result` is one of three shapes: `export` made a package.
            let made = &schema["$defs"]["ExportOutput"];
            walk(made, &schema, &path[1..])
        } else {
            walk(&schema, &schema, path)
        };
        assert!(
            found.is_some(),
            "{op}: no /{} in its schema",
            path.join("/")
        );
    }
}
