use std::cell::RefCell;

use super::*;
use crate::script::Ran;

/// A temporary engine checkout with the skills `skills`, each a name and
/// its `example.ron` (none for `None`).
fn checkout(test: &str, skills: &[(&str, Option<&str>)]) -> PathBuf {
    let root = std::env::temp_dir().join(format!("xtask-skills-{test}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    for (name, example) in skills {
        let dir = root.join(SKILLS).join(name);
        fs::create_dir_all(dir.join("example")).unwrap();
        fs::write(dir.join("SKILL.md"), "---\nname: x\n---\n").unwrap();
        fs::write(dir.join("example/note.txt"), "a note\n").unwrap();
        if let Some(example) = example {
            fs::write(dir.join(EXAMPLE), example).unwrap();
        }
    }
    root
}

/// A fake runner: `shuyi new` makes the game's folder and answers `ok`;
/// `cargo shuyi …` answers `ok` unless it runs `lint`; every call kept.
#[derive(Default)]
struct Fake {
    calls: RefCell<Vec<(PathBuf, String)>>,
}

impl Runner for Fake {
    fn run(&self, dir: &Path, _: &str, args: &[String]) -> Result<Ran, String> {
        let joined = args.join(" ");
        self.calls
            .borrow_mut()
            .push((dir.to_path_buf(), joined.clone()));
        if let Some(at) = args.iter().position(|a| a == "new") {
            fs::create_dir_all(&args[at + 1]).unwrap();
        }
        let ok = !joined.contains("lint");
        Ok(Ran {
            code: Some(i32::from(!ok)),
            stdout: format!(r#"{{"ok": {ok}, "error": "a finding"}}"#),
            stderr: String::new(),
        })
    }
}

const RUNS: &str = r#"Example(template: "grid", steps: [
    Shuyi(args: ["run"]),
    Write(to: "note.txt", from: "example/note.txt"),
    Holds(file: "note.txt", text: "a note"),
])"#;

#[test]
fn options_name_skills_and_a_shard() {
    let args = |s: &str| -> Vec<String> { s.split_whitespace().map(str::to_owned).collect() };
    assert_eq!(
        Options::parse(&args("shuyi-gates shuyi-test --shard 2/3")),
        Ok(Options {
            only: vec!["shuyi-gates".to_owned(), "shuyi-test".to_owned()],
            shard: Some((2, 3)),
        })
    );
    assert_eq!(Options::parse(&[]), Ok(Options::default()));
    assert_eq!(
        Options::parse(&args("--shard 3/3")).map(|o| o.shard),
        Ok(Some((3, 3)))
    );
    for wrong in ["0/3", "4/3", "2", "a/b"] {
        assert_eq!(
            Options::parse(&args(&format!("--shard {wrong}"))),
            Err(format!("--shard {wrong}: not <i>/<n> with 1 <= i <= n"))
        );
    }
    assert_eq!(
        Options::parse(&args("--shard")),
        Err("--shard needs <i>/<n>".to_owned())
    );
    assert_eq!(
        Options::parse(&args("--keep")),
        Err("unknown option --keep".to_owned())
    );
}

#[test]
fn the_examples_are_read_in_name_order_and_a_bad_one_named() {
    let root = checkout(
        "read",
        &[
            ("shuyi-test", Some(RUNS)),
            ("shuyi-net", None),
            ("shuyi-gates", Some(RUNS)),
            ("shuyi-bad", Some("Example(template: \"grid\", nope: [])")),
        ],
    );
    let (found, errors) = examples(&root);
    let names: Vec<&str> = found.iter().map(|(n, _, _)| n.as_str()).collect();
    assert_eq!(names, ["shuyi-gates", "shuyi-test"]);
    assert_eq!(found[0].2.template, "grid");
    assert_eq!(found[0].2.steps.len(), 3);
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(
        errors[0].starts_with("skills/shuyi-bad/example.ron: "),
        "{errors:?}"
    );
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn each_example_runs_on_a_new_game_made_from_this_checkout() {
    let root = checkout("runs", &[("shuyi-gates", Some(RUNS))]);
    let games = root.join("games");
    let fake = Fake::default();
    let outcome = run(&root, &games, &Options::default(), &fake);
    assert_eq!(outcome.errors, Vec::<String>::new());
    let game = games.join("shuyi-gates");
    let calls = fake.calls.borrow();
    assert_eq!(calls.len(), 2, "{calls:?}");
    assert_eq!(calls[0].0, root);
    let new = format!(
        "run -q -p shuyi-cli --locked -- new {} --template grid --name shuyi-gates --engine-path {} --json",
        game.display(),
        root.display()
    );
    assert_eq!(calls[0].1, new);
    assert_eq!(calls[1], (game.clone(), "shuyi run --json".to_owned()));
    assert_eq!(
        fs::read_to_string(game.join("note.txt")).unwrap(),
        "a note\n"
    );
    assert!(
        outcome
            .report
            .starts_with("shuyi-gates: 3 step(s) on a new `grid` game in "),
        "{}",
        outcome.report
    );
    assert!(
        outcome.report.contains("3. ok     note.txt holds its text"),
        "{}",
        outcome.report
    );
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn a_failed_step_names_its_skill_and_a_manual_example_isnt_run() {
    let root = checkout(
        "fails",
        &[
            (
                "shuyi-gates",
                Some(r#"Example(template: "grid", steps: [Shuyi(args: ["lint"])])"#),
            ),
            (
                "shuyi-live",
                Some(r#"Example(template: "grid", manual: Some("needs a window"), steps: [])"#),
            ),
        ],
    );
    let fake = Fake::default();
    let outcome = run(&root, &root.join("games"), &Options::default(), &fake);
    assert_eq!(
        outcome.errors,
        [
            "shuyi-gates: step 1 (cargo shuyi lint --json) failed: the answer's ok is false, not \
             true; error: a finding"
        ]
    );
    assert!(
        outcome
            .report
            .contains("shuyi-live: manual, not run: needs a window"),
        "{}",
        outcome.report
    );
    assert_eq!(fake.calls.borrow().len(), 2);
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn only_the_named_skills_and_the_shard_run() {
    let root = checkout(
        "shard",
        &[("a", Some(RUNS)), ("b", Some(RUNS)), ("c", Some(RUNS))],
    );
    let ran = |options: &Options| {
        let fake = Fake::default();
        let outcome = run(&root, &root.join("games"), options, &fake);
        let games: Vec<String> = fake
            .calls
            .borrow()
            .iter()
            .filter(|(_, args)| args.contains(" new "))
            .map(|(_, args)| args.split(' ').nth(7).unwrap_or_default().to_owned())
            .map(|dir| {
                dir.rsplit(['/', '\\'])
                    .next()
                    .unwrap_or_default()
                    .to_owned()
            })
            .collect();
        (games, outcome.errors)
    };
    let shard = |i, n| Options {
        only: Vec::new(),
        shard: Some((i, n)),
    };
    assert_eq!(ran(&shard(1, 2)).0, ["a", "c"]);
    assert_eq!(ran(&shard(2, 2)).0, ["b"]);
    let only = Options {
        only: vec!["c".to_owned(), "nope".to_owned()],
        shard: None,
    };
    let (games, errors) = ran(&only);
    assert_eq!(games, ["c"]);
    assert_eq!(
        errors,
        ["nope: no skill of skills/ with a worked example (example.ron)"]
    );
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn the_games_are_made_under_the_target_folder() {
    assert_eq!(
        games_dir(Path::new("engine")),
        Path::new("engine").join("target/skill-examples")
    );
}

#[test]
fn a_skills_second_example_is_read_named_after_it_and_picked_with_it() {
    let root = checkout("second", &[("shuyi-test", Some(RUNS)), ("shuyi-other", Some(RUNS))]);
    let manual = r#"Example(template: "grid", manual: Some("needs a window"), steps: [])"#;
    fs::write(root.join(SKILLS).join("shuyi-test/example-attach.ron"), manual).unwrap();
    let (found, errors) = examples(&root);
    assert_eq!(errors, Vec::<String>::new());
    let names: Vec<&str> = found.iter().map(|(name, _, _)| name.as_str()).collect();
    assert_eq!(names, ["shuyi-other", "shuyi-test", "shuyi-test-attach"]);
    assert!(found[2].1.ends_with("shuyi-test"));
    assert_eq!(found[2].2.manual.as_deref(), Some("needs a window"));
    let only = Options {
        only: vec!["shuyi-test".to_owned()],
        shard: None,
    };
    let fake = Fake::default();
    let outcome = run(&root, &root.join("games"), &only, &fake);
    assert_eq!(outcome.errors, Vec::<String>::new());
    assert!(outcome.report.contains("shuyi-test: 3 step(s)"), "{}", outcome.report);
    assert!(
        outcome
            .report
            .contains("shuyi-test-attach: manual, not run: needs a window"),
        "{}",
        outcome.report
    );
    assert!(!outcome.report.contains("shuyi-other"), "{}", outcome.report);
    let _ = fs::remove_dir_all(&root);
}
