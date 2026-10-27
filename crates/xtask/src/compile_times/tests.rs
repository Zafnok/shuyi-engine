use std::cell::RefCell;

use super::*;

fn times(rebuilds: &[f64], dependencies: usize) -> Times {
    Times {
        os: "linux".to_owned(),
        clean: 60.0,
        rebuilds: rebuilds.to_vec(),
        check_clean: 20.0,
        check_rebuilds: vec![0.5, 0.4, 0.6],
        dependencies,
    }
}

fn budget(linux: f64, dependencies: usize) -> Budget {
    Budget {
        rebuild: BTreeMap::from([("linux".to_owned(), linux)]),
        dependencies,
    }
}

#[test]
fn cargo_says_how_long_it_took() {
    let stderr = "   Compiling timed v0.1.0 (/tmp/timed/crates/game)\n    \
                  Finished `dev` profile [unoptimized + debuginfo] target(s) in 2.34s\n";
    assert_eq!(finished_in(stderr), Some(2.34));
    let long = "    Finished `dev` profile [unoptimized] target(s) in 1m 05s\n";
    assert_eq!(finished_in(long), Some(65.0));
    let two = "    Finished `dev` profile target(s) in 9.00s\n    \
               Finished `dev` profile target(s) in 0.25s\n";
    assert_eq!(finished_in(two), Some(0.25));
    assert_eq!(finished_in("error[E0425]: cannot find value `x`\n"), None);
    assert_eq!(
        finished_in("    Finished `dev` profile target(s) in soon\n"),
        None
    );
}

#[test]
fn the_dependencies_are_the_resolve_without_the_workspace() {
    let metadata = r#"{
        "workspace_members": ["path+file:///g/crates/game#blank@0.1.0"],
        "resolve": {"nodes": [
            {"id": "path+file:///g/crates/game#blank@0.1.0"},
            {"id": "path+file:///engine/crates/shuyi#0.1.0"},
            {"id": "registry+https://github.com/rust-lang/crates.io-index#macroquad@0.4.14"}
        ]}
    }"#;
    assert_eq!(dependencies(metadata), Ok(2));
    assert_eq!(
        dependencies(r#"{"workspace_members": []}"#),
        Err("cargo metadata: no resolve".to_owned())
    );
    assert!(dependencies("not json").is_err());
}

#[test]
fn the_median_is_the_middle_time() {
    assert!((median(&[3.0, 1.0, 2.0]) - 2.0).abs() < f64::EPSILON);
    assert!((median(&[4.0, 1.0, 2.0, 3.0]) - 2.5).abs() < f64::EPSILON);
    assert!(median(&[]).abs() < f64::EPSILON);
}

#[test]
fn under_the_budget_passes() {
    assert_eq!(
        check(&times(&[1.0, 2.0, 1.5], 120), &budget(3.0, 120), "b.ron"),
        Verdict::default()
    );
}

#[test]
fn a_slow_rebuild_fails() {
    let verdict = check(&times(&[1.0, 4.0, 3.5], 120), &budget(3.0, 120), "b.ron");
    assert_eq!(verdict.errors.len(), 1, "{verdict:?}");
    assert!(
        verdict.errors[0]
            .starts_with("the median rebuild took 3.50 s on linux, over the 3.00 s of b.ron"),
        "{verdict:?}"
    );
    // A budget of 0 s fails every run.
    assert_eq!(
        check(&times(&[0.1, 0.1, 0.1], 120), &budget(0.0, 120), "b.ron")
            .errors
            .len(),
        1
    );
}

#[test]
fn a_grown_dependency_tree_fails_and_a_smaller_one_warns() {
    let grown = check(&times(&[1.0], 121), &budget(3.0, 120), "b.ron");
    assert_eq!(grown.errors.len(), 1, "{grown:?}");
    assert!(
        grown.errors[0].starts_with("a new game depends on 121 crates, more than the 120 of b.ron"),
        "{grown:?}"
    );
    let smaller = check(&times(&[1.0], 110), &budget(3.0, 120), "b.ron");
    assert!(smaller.errors.is_empty(), "{smaller:?}");
    assert_eq!(
        smaller.warnings,
        vec!["a new game depends on 110 crates, fewer than the 120 of b.ron: lower it".to_owned()]
    );
}

#[test]
fn an_os_without_a_budget_fails() {
    let mut on_mac = times(&[1.0], 120);
    on_mac.os = "macos".to_owned();
    assert_eq!(
        check(&on_mac, &budget(3.0, 120), "b.ron").errors,
        vec!["b.ron has no rebuild budget for macos".to_owned()]
    );
}

#[test]
fn the_budget_in_tools_reads() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let text = fs::read_to_string(root.join("tools/compile-budget.ron")).unwrap();
    let budget = parse_budget(&text).unwrap();
    for os in ["linux", "windows"] {
        assert!(budget.rebuild.contains_key(os), "no budget for {os}");
    }
    assert!(parse_budget("Budget(rebuild: {}, dependencies: 1, more: 2)").is_err());
}

#[test]
fn options_read_their_values() {
    let args = |s: &str| -> Vec<String> { s.split_whitespace().map(str::to_owned).collect() };
    assert_eq!(
        Options::parse(&args("--json --budget b.ron --summary s.md --dir d")),
        Some(Options {
            json: true,
            budget: Some(PathBuf::from("b.ron")),
            summary: Some(PathBuf::from("s.md")),
            dir: Some(PathBuf::from("d")),
        })
    );
    assert_eq!(Options::parse(&[]), Some(Options::default()));
    assert_eq!(Options::parse(&args("--budget")), None);
    assert_eq!(Options::parse(&args("--fast")), None);
}

/// A fake cargo: `shuyi new` makes the game's `main.rs`, `metadata` answers
/// two crates, and each build or check takes one more second than the last.
#[derive(Default)]
struct Fake {
    calls: RefCell<Vec<String>>,
}

impl Runner for Fake {
    fn run(&self, dir: &Path, _: &str, args: &[String]) -> Result<Ran, String> {
        let joined = args.join(" ");
        self.calls.borrow_mut().push(joined.clone());
        let n = self.calls.borrow().len();
        let mut ran = Ran {
            code: Some(0),
            stdout: String::new(),
            stderr: String::new(),
        };
        if let Some(at) = args.iter().position(|a| a == "new") {
            let game = Path::new(&args[at + 1]);
            fs::create_dir_all(game.join("crates/game/src")).unwrap();
            fs::write(game.join(MAIN), "fn main() {}\n").unwrap();
            ran.stdout = r#"{"ok": true}"#.to_owned();
        } else if joined.starts_with("metadata") {
            ran.stdout = r#"{"workspace_members": ["g"], "resolve": {"nodes": [{"id": "g"}, {"id": "a"}, {"id": "b"}]}}"#.to_owned();
        } else {
            assert!(dir.ends_with("timed"), "{joined} in {}", dir.display());
            ran.stderr = format!("    Finished `dev` profile target(s) in {n}.00s\n");
        }
        Ok(ran)
    }
}

#[test]
fn a_run_times_builds_and_checks_after_each_change() {
    let root = std::env::temp_dir().join(format!("xtask-compile-times-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("Cargo.lock"), "# the engine's lock\n").unwrap();
    let games = root.join("games");
    let fake = Fake::default();
    let times = run(&root, &games, &fake, &fake).unwrap();
    // Two `shuyi new`, `metadata`, then build ×4 and check ×4.
    assert_eq!(times.dependencies, 2);
    assert!((times.clean - 4.0).abs() < f64::EPSILON);
    assert_eq!(times.rebuilds, vec![5.0, 6.0, 7.0]);
    assert!((times.check_clean - 8.0).abs() < f64::EPSILON);
    assert_eq!(times.check_rebuilds, vec![9.0, 10.0, 11.0]);
    assert_eq!(
        fs::read_to_string(games.join("blank/Cargo.lock")).unwrap(),
        "# the engine's lock\n"
    );
    let main = fs::read_to_string(games.join("timed").join(MAIN)).unwrap();
    assert!(main.ends_with("// compile-times: build 3\n// compile-times: check 1\n// compile-times: check 2\n// compile-times: check 3\n"), "{main}");
    let calls = fake.calls.borrow();
    assert!(calls[0].contains("new") && calls[0].contains("--template grid"));
    assert!(calls[1].contains("--template blank"));
    assert_eq!(calls[3], "build --color never");
    assert_eq!(calls[10], "check --color never");
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn a_failed_build_says_why() {
    struct Fails;
    impl Runner for Fails {
        fn run(&self, _: &Path, _: &str, args: &[String]) -> Result<Ran, String> {
            Ok(Ran {
                code: Some(101),
                stdout: String::new(),
                stderr: format!("error: {} broke\n", args[0]),
            })
        }
    }
    let error = cargo(&Fails, Path::new("g"), &["build"]).unwrap_err();
    assert!(
        error.contains("`cargo build` in g failed:\nerror: build broke"),
        "{error}"
    );
}

#[test]
fn the_table_names_every_number() {
    let table = times(&[1.0, 2.0, 1.5], 120).table();
    for needle in [
        "| Clean build | 60.00 |",
        "(median of 1.00, 2.00, 1.50) | 1.50 |",
        "| Clean `check` | 20.00 |",
        "(median of 0.50, 0.40, 0.60) | 0.50 |",
        "depends on 120 crates",
    ] {
        assert!(table.contains(needle), "{needle} not in {table}");
    }
    let json = times(&[1.0, 2.0, 1.5], 120).json();
    assert_eq!(json["rebuild"], 1.5);
    assert_eq!(json["dependencies"], 120);
}
