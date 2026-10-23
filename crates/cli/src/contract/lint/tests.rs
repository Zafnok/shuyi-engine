//! `lint`'s readers of clippy and `cargo tree`, and the answer, on a fake
//! cargo.

use std::cell::RefCell;
use std::fs;
use std::path::PathBuf;

use serde_json::json;

use super::*;
use crate::project::{CheckKeys, CheckText, Config};

/// A compiler message of lint `code` (none for a compiler error with no
/// code) saying `text` at `file`:`line`:`column`, with `notes`, as cargo's
/// `--message-format=json` writes a line.
fn message(code: Option<&str>, text: &str, at: Option<(&str, u32, u32)>, notes: &[&str]) -> String {
    let spans: Vec<Value> = at
        .map(|(file, line, column)| {
            json!({"file_name": file, "line_start": line, "column_start": column, "is_primary": true})
        })
        .into_iter()
        .collect();
    let children: Vec<Value> = notes
        .iter()
        .map(|note| json!({"level": "note", "message": note, "spans": []}))
        .chain([
            json!({"level": "help", "message": "for further information visit …", "spans": []}),
        ])
        .collect();
    json!({
        "reason": "compiler-message",
        "message": {
            "level": "error",
            "message": text,
            "code": code.map(|code| json!({"code": code})),
            "spans": spans,
            "children": children,
        }
    })
    .to_string()
}

/// A broken rules crate's clippy output: the fence's three lints, another
/// lint at the same place, and the build's last line.
fn broken() -> String {
    let file = "crates\\rules\\src\\lib.rs";
    [
        message(
            Some("clippy::implicit_hasher"),
            "parameter of type `HashMap` should be generalized over different hashers",
            Some((file, 403, 34)),
            &[],
        ),
        message(
            Some("clippy::disallowed_types"),
            "use of a disallowed type `std::collections::HashMap`",
            Some((file, 403, 34)),
            &[
                "its order changes from run to run, so replays differ: use BTreeMap",
                "`-D clippy::disallowed-types` implied by `-D warnings`",
            ],
        ),
        message(
            Some("clippy::disallowed_methods"),
            "use of a disallowed method `std::time::Instant::now`",
            Some((file, 404, 13)),
            &["the rules have no clock: count turns, or put the time in a Command"],
        ),
        message(
            Some("clippy::float_arithmetic"),
            "floating-point arithmetic detected",
            Some((file, 405, 5)),
            &[],
        ),
        r#"{"reason":"build-finished","success":false}"#.to_owned(),
    ]
    .join("\n")
}

#[test]
fn clippy_is_read_for_the_fences_lints_each_once_with_its_reason() {
    let root = Path::new("/game");
    let mut stdout = broken();
    // Cargo may say one twice; it is one finding.
    stdout.push('\n');
    stdout.push_str(&broken());
    let found = clippy_findings(root, &stdout);
    let shown: Vec<(&str, Option<u32>, Option<u32>, &str)> = found
        .iter()
        .map(|f| (f.file.as_str(), f.line, f.column, f.message.as_str()))
        .collect();
    assert_eq!(
        shown,
        [
            (
                "crates/rules/src/lib.rs",
                Some(403),
                Some(34),
                "use of a disallowed type `std::collections::HashMap`: its order changes from run \
                 to run, so replays differ: use BTreeMap"
            ),
            (
                "crates/rules/src/lib.rs",
                Some(404),
                Some(13),
                "use of a disallowed method `std::time::Instant::now`: the rules have no clock: \
                 count turns, or put the time in a Command"
            ),
            (
                "crates/rules/src/lib.rs",
                Some(405),
                Some(5),
                "floating-point arithmetic detected"
            ),
        ]
    );
    assert!(found.iter().all(|f| f.check == LintCheck::RulesFence));
    assert!(found.iter().all(|f| f.code == "SH2103"));
    assert_eq!(found[0].help.as_deref(), codes::help_of("SH2103"));
    let seen = clippy_findings(root, "not json\n");
    assert!(seen.is_empty(), "{seen:?}");
    assert!(compile_error(root, &stdout).is_none(), "lints aren't");
}

#[test]
fn a_message_with_no_span_is_a_finding_with_no_place() {
    let stdout = message(
        Some("clippy::disallowed_types"),
        "use of a disallowed type `f32`",
        None,
        &[],
    );
    let found = clippy_findings(Path::new("."), &stdout);
    assert_eq!(found.len(), 1);
    assert_eq!(
        (found[0].file.as_str(), found[0].line, found[0].column),
        ("", None, None)
    );
}

#[test]
fn a_compile_error_is_told_apart_from_a_lint() {
    let root = Path::new(".");
    let rustc = message(
        Some("E0425"),
        "cannot find value `hero` in this scope",
        Some(("crates/rules/src/lib.rs", 9, 1)),
        &[],
    );
    assert_eq!(
        compile_error(root, &rustc).as_deref(),
        Some("crates/rules/src/lib.rs:9: cannot find value `hero` in this scope")
    );
    let bare = message(None, "expected item", None, &[]);
    assert_eq!(compile_error(root, &bare).as_deref(), Some("expected item"));
    let lint = message(Some("unused_variables"), "unused variable", None, &[]);
    assert!(compile_error(root, &lint).is_none());
}

#[test]
fn cargo_tree_is_read_for_the_crates_the_rules_never_reach() {
    let root = Path::new("/game");
    let stdout = "demo-rules v0.1.0 (/game/crates/rules)\n\
                  shuyi-core v0.1.0 (/engine/crates/core)\n\
                  shuyi-ui v0.1.0 (/engine/crates/ui)\n\
                  quad-snd v0.2.8\n\
                  shuyi-ui v0.1.0 (/engine/crates/ui)\n\
                  serde v1.0.0\n";
    let found = tree_findings(root, "demo-rules", stdout);
    let names: Vec<(&str, Option<&str>)> = found
        .iter()
        .map(|f| (f.file.as_str(), f.command.as_deref()))
        .collect();
    assert_eq!(
        names,
        [
            (
                "crates/rules/Cargo.toml",
                Some("cargo tree -p demo-rules -i quad-snd")
            ),
            (
                "crates/rules/Cargo.toml",
                Some("cargo tree -p demo-rules -i shuyi-ui")
            ),
        ]
    );
    assert!(
        found[1]
            .message
            .starts_with("demo-rules depends on shuyi-ui: ")
    );
    assert!(found.iter().all(|f| f.code == "SH2104" && f.line.is_none()));
    let seen = tree_findings(
        root,
        "demo-rules",
        "demo-rules v0.1.0 (/game/crates/rules)\n",
    );
    assert!(seen.is_empty(), "{seen:?}");
    // A tree with no path says the root's manifest.
    let found = tree_findings(root, "r", "r v0.1.0\nrand v0.9.0\n");
    assert_eq!(found[0].file, "Cargo.toml");
}

#[test]
fn the_crates_never_under_the_rules_are_the_templates() {
    let test = include_str!("../../../../../templates/grid/crates/rules/tests/layers.rs");
    let listed = test
        .split_once("const NEVER: &[&str] = &[")
        .and_then(|(_, rest)| rest.split_once("];"))
        .map(|(list, _)| list)
        .unwrap();
    let names: Vec<&str> = listed
        .split(',')
        .map(|name| name.trim().trim_matches('"'))
        .filter(|name| !name.is_empty())
        .collect();
    assert_eq!(names, NEVER);
}

#[test]
fn a_checks_line_is_a_finding_with_its_place() {
    let found = reported(
        LintCheck::CheckKeys,
        codes::HARD_CODED_KEY,
        "crates/game/src/a.rs:12: names a key or button with `Key::` — use an action",
    );
    assert_eq!(
        (found.file.as_str(), found.line, found.message.as_str()),
        (
            "crates/game/src/a.rs",
            Some(12),
            "names a key or button with `Key::` — use an action"
        )
    );
    assert_eq!(found.code, "SH2101");
    let unread = reported(
        LintCheck::CheckKeys,
        codes::HARD_CODED_KEY,
        "D:\\game\\crates\\src: cannot read directory: gone",
    );
    assert_eq!(
        (unread.file.as_str(), unread.line, unread.code.as_str()),
        ("D:\\game\\crates\\src", None, "SH3004")
    );
}

/// A cargo that answers clippy and `cargo tree` with what it is given,
/// and keeps the commands run.
#[derive(Default)]
struct FakeCargo {
    clippy: Ran,
    tree: Ran,
    ran: RefCell<Vec<String>>,
    broken: bool,
}

impl Cargo for FakeCargo {
    fn run(&self, _root: &Path, args: &[&str]) -> Result<Ran, String> {
        if self.broken {
            return Err("cannot run cargo: not found".to_owned());
        }
        self.ran.borrow_mut().push(args.join(" "));
        Ok(if args[0] == "clippy" {
            self.clippy.clone()
        } else {
            self.tree.clone()
        })
    }
}

/// A game in a folder of its own named `name`: a screen source with a key,
/// a literal and a data name read.
fn game(name: &str) -> Project {
    let root = std::env::temp_dir().join(format!("shuyi-lint-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(
        root.join("src/screen.rs"),
        "fn draw(ctx: &mut Ctx, unit: &Unit) {\n    \
         let _ = Key::Space;\n    \
         ctx.print(0, 0, \"Your turn\");\n    \
         ctx.print(0, 1, unit.name);\n}\n",
    )
    .unwrap();
    Project {
        root,
        config: Config::default(),
    }
}

fn rules(dir: &str) -> (CheckKeys, CheckText) {
    (
        CheckKeys {
            rust: vec![dir.to_owned()],
            hint: "use an action".to_owned(),
            ..CheckKeys::default()
        },
        CheckText {
            dir: dir.to_owned(),
            hint: "use the language file".to_owned(),
            name_hint: "use the words helpers".to_owned(),
            ..CheckText::default()
        },
    )
}

#[test]
fn without_its_parts_lint_says_what_it_did_not_check() {
    let project = game("bare");
    let cargo = FakeCargo::default();
    let answer = lint(&cargo, &project, &LintInput::default());
    assert!(answer.ok);
    assert_eq!(answer.op, Op::Lint);
    assert_eq!(
        answer.limits,
        [
            "check-keys wasn't run: shuyi.ron has no `check_keys`",
            "check-text wasn't run: shuyi.ron has no `check_text`",
            "the rules crate's fence wasn't checked: shuyi.ron names no rules package (`rules`)",
        ]
    );
    let got = cargo.ran.borrow();
    assert!(got.is_empty(), "{got:?}");
    assert_eq!(text(answer.result.as_ref().unwrap()), "lint: OK\n");
    fs::remove_dir_all(&project.root).unwrap();
}

#[test]
fn lint_answers_every_checks_findings_in_one_list() {
    let mut project = game("all");
    let (keys, text_rules) = rules("src");
    project.config.check_keys = Some(keys);
    project.config.check_text = Some(text_rules);
    project.config.rules = Some("demo-rules".to_owned());
    let cargo = FakeCargo {
        clippy: Ran {
            success: false,
            stdout: broken(),
            stderr: "error: could not compile `demo-rules`".to_owned(),
        },
        tree: Ran {
            success: true,
            stdout: "demo-rules v0.1.0\nshuyi-ui v0.1.0\n".to_owned(),
            stderr: String::new(),
        },
        ..FakeCargo::default()
    };
    let answer = lint(&cargo, &project, &LintInput::default());
    assert!(!answer.ok);
    assert_eq!(answer.exit_code(), 1);
    assert!(answer.limits.is_empty(), "{:?}", answer.limits);
    let out = answer.result.as_ref().unwrap();
    let checks: Vec<(LintCheck, &str, Option<u32>)> = out
        .findings
        .iter()
        .map(|f| (f.check, f.code.as_str(), f.line))
        .collect();
    assert_eq!(
        checks,
        [
            (LintCheck::CheckKeys, "SH2101", Some(2)),
            (LintCheck::CheckText, "SH2102", Some(3)),
            (LintCheck::CheckText, "SH2102", Some(4)),
            (LintCheck::RulesFence, "SH2103", Some(403)),
            (LintCheck::RulesFence, "SH2103", Some(404)),
            (LintCheck::RulesFence, "SH2103", Some(405)),
            (LintCheck::RulesLayers, "SH2104", None),
        ]
    );
    assert_eq!(
        out.findings[1].message,
        "screen text written as a literal, \"Your turn\" (1 in src, at most 0 allowed) — use the \
         language file"
    );
    assert_eq!(
        *cargo.ran.borrow(),
        [
            "clippy --quiet -p demo-rules --message-format=json -- -D warnings",
            "tree -p demo-rules --edges normal,build --target all --prefix none --format {p}",
        ]
    );
    let said = text(out);
    assert!(
        said.starts_with("lint: 7 finding(s)\n  src/screen.rs:2: error[SH2101]: "),
        "{said}"
    );
    assert!(said.contains("  crates/rules/src/lib.rs:404:13: error[SH2103]: use of a disallowed"));
    assert!(said.contains("    see the way in: cargo tree -p demo-rules -i shuyi-ui\n"));
    assert_eq!(said.matches("help[SH2103]").count(), 1);
    // Literals within the allowance aren't findings.
    project.config.check_text.as_mut().unwrap().max_literals = 1;
    let answer = lint(&cargo, &project, &LintInput::default());
    assert_eq!(answer.result.unwrap().findings.len(), 6);
    fs::remove_dir_all(&project.root).unwrap();
}

#[test]
fn a_cargo_that_fails_is_a_limit_and_one_that_doesnt_start_an_error() {
    let mut project = game("cargo");
    project.config.rules = Some("demo-rules".to_owned());
    let failed = Ran {
        success: false,
        stdout: String::new(),
        stderr: "error: package ID specification `demo-rules` did not match any packages\n"
            .to_owned(),
    };
    let cargo = FakeCargo {
        clippy: failed.clone(),
        tree: failed,
        ..FakeCargo::default()
    };
    let answer = lint(&cargo, &project, &LintInput::default());
    assert!(answer.ok);
    assert_eq!(
        &answer.limits[2..],
        [
            "the rules crate's fence wasn't checked: cargo clippy -p demo-rules failed: error: \
             package ID specification `demo-rules` did not match any packages",
            "the rules crate's layers weren't checked: cargo tree -p demo-rules failed: error: \
             package ID specification `demo-rules` did not match any packages",
        ]
    );
    // Rules that don't compile are checked only in part.
    let cargo = FakeCargo {
        clippy: Ran {
            success: false,
            stdout: message(Some("E0425"), "cannot find value `hero`", None, &[]),
            stderr: String::new(),
        },
        tree: Ran {
            success: true,
            ..Ran::default()
        },
        ..FakeCargo::default()
    };
    let answer = lint(&cargo, &project, &LintInput::default());
    assert_eq!(
        answer.limits[2],
        "the rules crate's fence was checked only in part: demo-rules doesn't compile (cannot \
         find value `hero`)"
    );
    let broken = FakeCargo {
        broken: true,
        ..FakeCargo::default()
    };
    let answer = lint(&broken, &project, &LintInput::default());
    assert_eq!(answer.code.as_deref(), Some("SH1003"));
    assert_eq!(answer.exit_code(), 3);
    assert_eq!(usage("cargo shuyi"), "usage: cargo shuyi lint [--json]");
    fs::remove_dir_all(&project.root).unwrap();
}

#[test]
fn this_machines_cargo_runs_or_says_why_not() {
    let here = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let ran = ThisMachine.run(&here, &["--version"]).unwrap();
    assert!(ran.success);
    assert!(ran.stdout.starts_with("cargo "));
    assert_eq!(last_line(""), "no output");
}

#[test]
fn a_findings_place_is_its_primary_span_not_its_first() {
    let stdout = json!({
        "reason": "compiler-message",
        "message": {
            "level": "error",
            "message": "use of a disallowed type `f32`",
            "code": {"code": "clippy::disallowed_types"},
            "spans": [
                {"file_name": "src/a.rs", "line_start": 1, "column_start": 1, "is_primary": false},
                {"file_name": "src/b.rs", "line_start": 7, "column_start": 3, "is_primary": true},
            ],
            "children": [],
        }
    })
    .to_string();
    let found = clippy_findings(Path::new("."), &stdout);
    assert_eq!(
        (found[0].file.as_str(), found[0].line, found[0].column),
        ("src/b.rs", Some(7), Some(3))
    );
}

#[test]
fn rules_that_fail_only_on_other_lints_are_checked() {
    let mut project = game("other");
    project.config.rules = Some("demo-rules".to_owned());
    let cargo = FakeCargo {
        clippy: Ran {
            success: false,
            stdout: message(
                Some("clippy::implicit_hasher"),
                "parameter of type `HashMap` should be generalized",
                None,
                &[],
            ),
            stderr: "error: could not compile `demo-rules`".to_owned(),
        },
        tree: Ran {
            success: true,
            ..Ran::default()
        },
        ..FakeCargo::default()
    };
    let answer = lint(&cargo, &project, &LintInput::default());
    assert!(answer.ok, "the other lint is clippy's own gate's");
    assert_eq!(answer.limits.len(), 2, "{:?}", answer.limits);
    fs::remove_dir_all(&project.root).unwrap();
}
