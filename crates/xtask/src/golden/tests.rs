use std::cell::RefCell;

use super::*;

/// A report's tail as a failed test prints it: the table at `path`.
fn printed(path: &str, rows: &str) -> String {
    format!(
        "golden table {path}: 0 changed, 1 added, 0 removed\n  added \"a\": 1\n\n\
         {START}{path}{START_END}\n{rows}{END}\n"
    )
}

#[test]
fn tables_are_read_between_the_markers_the_last_per_path_winning() {
    let output = [
        "running 2 tests\n".to_string(),
        printed("crates/a/tests/golden/a.ron", "{\n    \"a\": \"1\",\n}\n"),
        "---- other stdout ----\n".to_string(),
        printed("crates/b/tests/golden/b.ron", "{}\n"),
        printed("crates/a/tests/golden/a.ron", "{\n    \"a\": \"2\",\n}\n"),
        // An unclosed table is no table.
        format!("{START}crates/c/tests/golden/c.ron{START_END}\n{{\n"),
    ]
    .concat();
    let found = tables(&output);
    assert_eq!(
        found,
        BTreeMap::from([
            (
                "crates/a/tests/golden/a.ron".to_string(),
                "{\n    \"a\": \"2\",\n}\n".to_string()
            ),
            (
                "crates/b/tests/golden/b.ron".to_string(),
                "{}\n".to_string()
            ),
        ])
    );
    // Windows line ends read the same.
    let crlf = printed("t.ron", "{}\n").replace('\n', "\r\n");
    assert_eq!(tables(&crlf)["t.ron"], "{}\n");
}

#[test]
fn a_path_outside_the_repository_is_refused() {
    let root = std::env::temp_dir().join("repo").join("root");
    let ok = root.join("crates/a/tests/golden/a.ron");
    assert_eq!(inside(&root, &ok.display().to_string()), Ok(ok.clone()));
    assert_eq!(inside(&root, "crates/a/tests/golden/a.ron"), Ok(ok));
    assert!(inside(&root, "../elsewhere/a.ron").is_err());
    let beside = std::env::temp_dir()
        .join("repo")
        .join("other/a.ron")
        .display()
        .to_string();
    assert!(inside(&root, &beside).is_err());
    let climbing = root.join("crates/../../other/a.ron").display().to_string();
    assert!(inside(&root, &climbing).is_err());
}

/// Fake runs: `cargo test`'s answers in order, and `git`'s one answer.
struct Fake {
    tests: RefCell<Vec<Ran>>,
    ran: RefCell<Vec<String>>,
}

impl Fake {
    fn new(tests: Vec<Ran>) -> Self {
        Self {
            tests: RefCell::new(tests),
            ran: RefCell::new(Vec::new()),
        }
    }
}

impl Runner for Fake {
    fn run(&self, dir: &Path, program: &str, args: &[&str]) -> Result<Ran, String> {
        self.run_with_env(dir, program, args, &[])
    }

    fn run_with_env(
        &self,
        _dir: &Path,
        program: &str,
        args: &[&str],
        env: &[(&str, &str)],
    ) -> Result<Ran, String> {
        let env: Vec<String> = env.iter().map(|(k, v)| format!("{k}={v} ")).collect();
        let command = format!("{}{program} {}", env.concat(), args.join(" "));
        self.ran.borrow_mut().push(command);
        if program == "git" {
            return Ok(ran(true, " a.ron | 2 +-\n"));
        }
        let mut tests = self.tests.borrow_mut();
        if tests.is_empty() {
            return Err("no more fake runs".to_string());
        }
        Ok(tests.remove(0))
    }
}

fn ran(ok: bool, stdout: &str) -> Ran {
    Ran {
        ok,
        stdout: stdout.to_string(),
        stderr: String::new(),
    }
}

/// A repository root of its own under the temp folder.
fn temp_root(name: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!("xtask-golden-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("crates/a/tests/golden")).unwrap();
    root
}

#[test]
fn bless_writes_the_tables_that_failed_and_runs_the_tests_again() {
    let root = temp_root("writes");
    let file = root.join("crates/a/tests/golden/a.ron");
    fs::write(&file, "{}\n").unwrap();
    let path = file.display().to_string();
    let table = "{\n    \"a\": \"1\",\n}\n";
    let fake = Fake::new(vec![ran(false, &printed(&path, table)), ran(true, "")]);
    let outcome = bless(&root, &fake);
    assert_eq!(outcome.errors, Vec::<String>::new());
    assert_eq!(fs::read_to_string(&file).unwrap(), table);
    assert_eq!(outcome.lines[0], format!("wrote {path}"));
    assert!(outcome.lines.contains(&" a.ron | 2 +-".to_string()));
    let ran = fake.ran.borrow();
    assert_eq!(
        *ran,
        [
            "SHUYI_BLESS=1 cargo test --workspace --locked --no-fail-fast -- golden".to_string(),
            format!("git diff --stat -- {path}"),
            "cargo test --workspace --locked --no-fail-fast -- golden".to_string(),
        ]
    );
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn bless_fails_when_the_tests_still_fail_or_failed_without_a_table() {
    let root = temp_root("fails");
    let path = root
        .join("crates/a/tests/golden/a.ron")
        .display()
        .to_string();
    let fake = Fake::new(vec![
        ran(false, &printed(&path, "{}\n")),
        ran(false, "a test that isn't a table's failed"),
    ]);
    let outcome = bless(&root, &fake);
    assert_eq!(outcome.errors.len(), 1);
    assert!(outcome.errors[0].contains("still fail after blessing"));
    assert!(outcome.errors[0].contains("isn't a table's failed"));
    // A failure that printed no table writes nothing.
    let outcome = bless(&root, &Fake::new(vec![ran(false, "a panic")]));
    assert!(
        outcome.errors[0].contains("without printing a table"),
        "{outcome:?}"
    );
    // Nothing failed: nothing to bless.
    let outcome = bless(&root, &Fake::new(vec![ran(true, "")]));
    assert_eq!(outcome.errors, Vec::<String>::new());
    assert_eq!(
        outcome.lines,
        ["every golden table matches: nothing to bless."]
    );
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn bless_refuses_a_table_outside_the_repository_and_writes_none() {
    let root = temp_root("outside");
    let outside = root.parent().unwrap().join("stray.ron");
    let fake = Fake::new(vec![ran(
        false,
        &printed(&outside.display().to_string(), "{}\n"),
    )]);
    let outcome = bless(&root, &fake);
    assert!(
        outcome.errors[0].contains("outside the repository"),
        "{outcome:?}"
    );
    assert!(!outside.exists());
    // Neither `git diff` nor the second run ran.
    assert_eq!(fake.ran.borrow().len(), 1);
    fs::remove_dir_all(&root).unwrap();
}
