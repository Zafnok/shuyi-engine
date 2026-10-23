use std::cell::RefCell;

use super::*;

/// What `cargo semver-checks` 0.51.0 printed on `shuyi-core`, captured with
/// `--release-type minor --color never` (ticket 2010, step 2).
fn fixture(name: &str) -> Ran {
    let (stdout, stderr) = match name {
        "pass" => (
            include_str!("../../tests/fixtures/api/pass.stdout"),
            include_str!("../../tests/fixtures/api/pass.stderr"),
        ),
        "one-finding" => (
            include_str!("../../tests/fixtures/api/one-finding.stdout"),
            include_str!("../../tests/fixtures/api/one-finding.stderr"),
        ),
        "two-lints" => (
            include_str!("../../tests/fixtures/api/two-lints.stdout"),
            include_str!("../../tests/fixtures/api/two-lints.stderr"),
        ),
        "build-error" => (
            include_str!("../../tests/fixtures/api/build-error.stdout"),
            include_str!("../../tests/fixtures/api/build-error.stderr"),
        ),
        _ => unreachable!("no fixture {name}"),
    };
    Ran {
        ok: name == "pass",
        stdout: stdout.to_string(),
        stderr: stderr.to_string(),
    }
}

fn ok(stdout: &str) -> Ran {
    Ran {
        ok: true,
        stdout: stdout.to_string(),
        stderr: String::new(),
    }
}

fn failed(stderr: &str) -> Ran {
    Ran {
        ok: false,
        stdout: String::new(),
        stderr: stderr.to_string(),
    }
}

fn lines(list: &[&str]) -> Vec<String> {
    list.iter().map(|line| (*line).to_string()).collect()
}

fn finding(lint: &str, items: &[&str]) -> Finding {
    Finding {
        lint: lint.to_string(),
        items: lines(items),
    }
}

/// A fake [`Runner`]. Each command is written `<where>: <program> <args>`,
/// `<where>` being `head` (the root) or `base` (the base's tree); the last
/// answer given whose key starts the command answers it, and any other
/// command succeeds and prints nothing. It records every command.
struct Fake {
    root: PathBuf,
    answers: Vec<(String, Result<Ran, String>)>,
    ran: RefCell<Vec<String>>,
}

impl Fake {
    fn new(root: &Path) -> Self {
        Self {
            root: root.to_path_buf(),
            answers: Vec::new(),
            ran: RefCell::new(Vec::new()),
        }
    }

    fn answer(mut self, key: &str, answer: Result<Ran, String>) -> Self {
        self.answers.insert(0, (key.to_string(), answer));
        self
    }

    /// The tools at their pinned versions, and the nightly.
    fn with_tools(self) -> Self {
        self.answer(
            "head: cargo semver-checks --version",
            Ok(ok(&format!("cargo-semver-checks {SEMVER_CHECKS}\n"))),
        )
        .answer(
            "head: cargo public-api --version",
            Ok(ok(&format!("cargo-public-api {PUBLIC_API}\n"))),
        )
        .answer(
            "head: rustup toolchain list",
            Ok(ok(&format!(
                "stable-x86_64-unknown-linux-gnu\n{NIGHTLY}-x86_64-unknown-linux-gnu (active)\n"
            ))),
        )
    }

    /// The base revision is the commit `commit`.
    fn with_base(self, commit: &str) -> Self {
        self.answer(
            "head: git rev-parse --verify ",
            Ok(ok(&format!("{commit}\n"))),
        )
    }

    /// The base's workspace has `packages`.
    fn with_base_packages(self, packages: &[&str]) -> Self {
        let list: Vec<String> = packages
            .iter()
            .map(|name| format!("{{\"name\":\"{name}\"}}"))
            .collect();
        let json = format!("{{\"packages\":[{}]}}", list.join(","));
        self.answer(
            "base: cargo metadata --no-deps --format-version 1",
            Ok(ok(&json)),
        )
    }

    /// `krate`'s listing in `place` (`head` or `base`).
    fn with_listing(self, place: &str, krate: &str, listing: &str) -> Self {
        self.answer(
            &format!("{place}: cargo +{NIGHTLY} public-api -p {krate} --simplified"),
            Ok(ok(listing)),
        )
    }

    /// `krate`'s semver run prints the fixture `name`.
    fn with_semver(self, krate: &str, name: &str) -> Self {
        self.answer(
            &format!("head: cargo semver-checks check-release -p {krate} "),
            Ok(fixture(name)),
        )
    }

    fn ran(&self) -> Vec<String> {
        self.ran.borrow().clone()
    }
}

impl Runner for Fake {
    fn run(&self, dir: &Path, program: &str, args: &[&str]) -> Result<Ran, String> {
        let place = if dir == self.root {
            "head"
        } else if dir == self.root.join(BASE_DIR) {
            "base"
        } else {
            "elsewhere"
        };
        let command = format!("{place}: {program} {}", args.join(" "));
        self.ran.borrow_mut().push(command.clone());
        self.answers
            .iter()
            .find(|(key, _)| command.starts_with(key.as_str()))
            .map_or_else(|| Ok(ok("")), |(_, answer)| answer.clone())
    }
}

/// A repository root of its own under the temp folder, with `tickets/done/`.
fn temp_root(name: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!("xtask-api-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("tickets/done")).unwrap();
    root
}

const TICKET_2100: &str = "---\nid: \"2100\"\n---\n\n# 2100 — A\n\n## Completion notes\n\n\
                           Breaking: shuyi-core: `shuyi_core::state_hash`: hash with `StateHash::of`.\n";

// -- the options --

#[test]
fn options_read_each_flag() {
    let args = lines(&[
        "--base",
        "HEAD^1",
        "--pr-branch",
        "t2010-api",
        "--summary",
        "out.md",
    ]);
    assert_eq!(
        Options::parse(&args),
        Some(Options {
            base: Some("HEAD^1".to_string()),
            pr_branch: Some("t2010-api".to_string()),
            summary: Some(PathBuf::from("out.md")),
        })
    );
    assert_eq!(Options::parse(&[]), Some(Options::default()));
}

#[test]
fn options_refuse_anything_else() {
    assert_eq!(Options::parse(&lines(&["--base"])), None);
    assert_eq!(Options::parse(&lines(&["--bless", "x"])), None);
}

// -- the listing diff --

#[test]
fn a_removed_line_only() {
    let api = CrateApi::new(
        "shuyi-core",
        Some(&lines(&["pub fn a()", "pub fn b()"])),
        &lines(&["pub fn a()"]),
        Vec::new(),
    );
    assert_eq!(api.removed, lines(&["pub fn b()"]));
    assert!(api.added.is_empty());
    assert!(!api.new);
}

#[test]
fn an_added_line_only() {
    let api = CrateApi::new(
        "shuyi-core",
        Some(&lines(&["pub fn a()"])),
        &lines(&["pub fn a()", "pub fn b()"]),
        Vec::new(),
    );
    assert!(api.removed.is_empty());
    assert_eq!(api.added, lines(&["pub fn b()"]));
}

#[test]
fn a_changed_line_is_one_removed_and_one_added() {
    let api = CrateApi::new(
        "shuyi-core",
        Some(&lines(&["pub fn a(x: u8)", "pub fn b()"])),
        &lines(&["pub fn a(x: u16)", "pub fn b()"]),
        Vec::new(),
    );
    assert_eq!(api.removed, lines(&["pub fn a(x: u8)"]));
    assert_eq!(api.added, lines(&["pub fn a(x: u16)"]));
}

#[test]
fn no_change() {
    let listing = lines(&["pub fn a()"]);
    let api = CrateApi::new("shuyi-core", Some(&listing), &listing, Vec::new());
    assert!(api.removed.is_empty() && api.added.is_empty());
}

#[test]
fn a_crate_new_in_the_head_adds_every_item() {
    let api = CrateApi::new(
        "shuyi-net",
        None,
        &lines(&["pub mod shuyi_net", "pub fn shuyi_net::a()"]),
        Vec::new(),
    );
    assert!(api.new);
    assert!(api.removed.is_empty());
    assert_eq!(
        api.added,
        lines(&["pub mod shuyi_net", "pub fn shuyi_net::a()"])
    );
}

// -- cargo-semver-checks's output --

#[test]
fn a_pass_has_no_finding() {
    assert_eq!(parse_semver("shuyi-core", &fixture("pass")), Ok(Vec::new()));
}

#[test]
fn one_finding_names_its_lint_and_items() {
    assert_eq!(
        parse_semver("shuyi-core", &fixture("one-finding")),
        Ok(vec![finding(
            "function_missing",
            &[
                "function shuyi_core::hash::state_hash",
                "function shuyi_core::state_hash"
            ]
        )])
    );
}

#[test]
fn findings_in_two_lints() {
    assert_eq!(
        parse_semver("shuyi-core", &fixture("two-lints")),
        Ok(vec![
            finding(
                "function_missing",
                &[
                    "function shuyi_core::hash::state_hash",
                    "function shuyi_core::state_hash",
                    "function shuyi_core::text::split_tokens",
                    "function shuyi_core::split_tokens",
                ]
            ),
            finding(
                "struct_missing",
                &[
                    "struct shuyi_core::rng::ScriptedRng",
                    "struct shuyi_core::ScriptedRng"
                ]
            ),
        ])
    );
}

#[test]
fn a_build_error_is_an_error_never_a_pass() {
    let error = parse_semver("shuyi-core", &fixture("build-error")).unwrap_err();
    assert!(
        error.contains("failed on shuyi-core without naming a break"),
        "{error}"
    );
    assert!(error.contains("could not document `shuyi-core`"), "{error}");
}

#[test]
fn a_success_that_names_a_finding_is_an_error() {
    let mut ran = fixture("one-finding");
    ran.ok = true;
    assert!(parse_semver("shuyi-core", &ran).is_err());
}

#[test]
fn a_failure_with_findings_but_no_summary_is_an_error() {
    let mut ran = fixture("one-finding");
    ran.stderr.clear();
    assert!(parse_semver("shuyi-core", &ran).is_err());
}

#[test]
fn a_warning_is_not_a_finding() {
    let stdout = "\n--- warning trait_method_marked_deprecated: method deprecated ---\n\n\
                  Failed in:\n  method shuyi_core::Foo::bar in file /x.rs:3\n\n\
                  --- failure struct_missing: pub struct removed or renamed ---\n\n\
                  Failed in:\n  struct shuyi_core::Foo, previously in file /x.rs:1\n\
                  \x20    Checked [   0.030s] 2 checks\n  not an item\n";
    assert_eq!(
        failures(stdout),
        vec![finding("struct_missing", &["struct shuyi_core::Foo"])]
    );
}

#[test]
fn an_item_loses_its_file() {
    assert_eq!(
        without_file("struct a::B, previously in file /x.rs:1"),
        "struct a::B"
    );
    assert_eq!(
        without_file("variant a::C::D in file /x.rs:2"),
        "variant a::C::D"
    );
    assert_eq!(without_file("method a::B::c"), "method a::B::c");
}

// -- the rule --

fn breaking_core() -> Vec<CrateApi> {
    vec![
        CrateApi::new(
            "shuyi-core",
            Some(&lines(&["pub fn shuyi_core::state_hash()"])),
            &[],
            vec![finding(
                "function_missing",
                &["function shuyi_core::state_hash"],
            )],
        ),
        CrateApi::new(
            "shuyi-ui",
            Some(&[]),
            &lines(&["pub fn shuyi_ui::a()"]),
            Vec::new(),
        ),
    ]
}

fn no_ticket(_: &str) -> Option<String> {
    None
}

#[test]
fn no_break_on_a_ticket_branch_passes() {
    let crates = vec![CrateApi::new(
        "shuyi-ui",
        Some(&[]),
        &lines(&["pub fn a()"]),
        Vec::new(),
    )];
    assert!(matches!(
        rule(&crates, "t2100-a", &no_ticket),
        Verdict::Pass(_)
    ));
}

#[test]
fn a_break_with_a_line_naming_the_crate_passes() {
    let verdict = rule(&breaking_core(), "t2100-a", &|id| {
        (id == "2100").then(|| TICKET_2100.to_string())
    });
    assert_eq!(
        verdict,
        Verdict::Pass("ticket 2100 has a `Breaking:` line for shuyi-core.".to_string())
    );
}

#[test]
fn a_line_may_name_several_crates_in_backticks() {
    let ticket = "## Completion notes\n\n- Breaking: `shuyi-ui`, `shuyi-core`: `x`: y.\n";
    let verdict = rule(&breaking_core(), "t2100-a", &|_| Some(ticket.to_string()));
    assert!(matches!(verdict, Verdict::Pass(_)), "{verdict:?}");
}

#[test]
fn a_line_naming_another_crate_fails() {
    let ticket = "## Completion notes\n\nBreaking: shuyi-ui: `x`: y.\n";
    let Verdict::Fail(why) = rule(&breaking_core(), "t2100-a", &|_| Some(ticket.to_string()))
    else {
        panic!("passed");
    };
    assert!(
        why.contains("no `Breaking:` line naming shuyi-core"),
        "{why}"
    );
    assert!(
        why.contains("    Breaking: shuyi-core: <item>: <what a game changes>"),
        "{why}"
    );
    assert!(
        why.contains("shuyi-core: function_missing: function shuyi_core::state_hash"),
        "{why}"
    );
}

#[test]
fn a_line_outside_the_completion_notes_fails() {
    let ticket = "## Scope\n\nBreaking: shuyi-core: `x`: y.\n\n## Completion notes\n\nDone.\n";
    let verdict = rule(&breaking_core(), "t2100-a", &|_| Some(ticket.to_string()));
    assert!(matches!(verdict, Verdict::Fail(_)), "{verdict:?}");
}

#[test]
fn a_ticket_not_in_done_fails() {
    let Verdict::Fail(why) = rule(&breaking_core(), "t2100-a", &no_ticket) else {
        panic!("passed");
    };
    assert!(
        why.starts_with("ticket 2100 is not in tickets/done/"),
        "{why}"
    );
}

#[test]
fn a_break_on_a_sonar_branch_fails() {
    let Verdict::Fail(why) = rule(&breaking_core(), "sonar/312-lifetimes", &no_ticket) else {
        panic!("passed");
    };
    assert!(why.contains("names no ticket"), "{why}");
    assert!(why.contains("`write-ticket` skill"), "{why}");
    assert!(why.contains("function shuyi_core::state_hash"), "{why}");
}

#[test]
fn no_break_on_a_sonar_branch_passes() {
    let crates = vec![CrateApi::new(
        "shuyi-ui",
        Some(&[]),
        &lines(&["pub fn a()"]),
        Vec::new(),
    )];
    assert!(matches!(
        rule(&crates, "sonar/312-lifetimes", &no_ticket),
        Verdict::Pass(_)
    ));
}

#[test]
fn named_crates_read_the_list_before_the_item() {
    assert_eq!(
        named_crates("Breaking: shuyi-core, `shuyi-ui`: `f`: g: h"),
        vec!["shuyi-core", "shuyi-ui"]
    );
    assert_eq!(named_crates("Breaking: shuyi-core"), vec!["shuyi-core"]);
    assert!(named_crates("Breaking: : x").is_empty());
}

#[test]
fn lost_lines_warn_only_without_a_finding() {
    let mut crates = breaking_core();
    crates.push(CrateApi::new(
        "shuyi-app",
        Some(&lines(&["pub fn a()", "pub fn b()"])),
        &[],
        Vec::new(),
    ));
    assert_eq!(
        lost_lines(&crates),
        vec![
            "shuyi-app: 2 removed line(s) cargo-semver-checks didn't call breaking: read them"
                .to_string()
        ]
    );
}

// -- the summary --

#[test]
fn the_summary_for_a_fixed_input() {
    let mut crates = breaking_core();
    crates.push(CrateApi::new(
        "shuyi-net",
        None,
        &lines(&["pub mod shuyi_net"]),
        Vec::new(),
    ));
    crates.push(CrateApi::new("shuyi-app", Some(&[]), &[], Vec::new()));
    let verdict = Verdict::Fail("why\nand how\n".to_string());
    let warnings = vec!["shuyi-ui: read them".to_string()];
    let expected = "\
## Public API

Against `abc123`.

| Crate | Items removed | Items added | Semver findings |
| ----- | ------------- | ----------- | --------------- |
| `shuyi-core` | 1 | 0 | 1 |
| `shuyi-ui` | 0 | 1 | 0 |
| `shuyi-net` (new) | 0 | 1 | not checked |
| `shuyi-app` | 0 | 0 | 0 |

> **Warning:** shuyi-ui: read them.

<details><summary><code>shuyi-core</code>: 1 removed, 0 added</summary>

- `function_missing`: function shuyi_core::state_hash

```diff
-pub fn shuyi_core::state_hash()
```

</details>

<details><summary><code>shuyi-ui</code>: 0 removed, 1 added</summary>

```diff
+pub fn shuyi_ui::a()
```

</details>

<details><summary><code>shuyi-net</code>: 0 removed, 1 added</summary>

```diff
+pub mod shuyi_net
```

</details>

**Fail:**

```text
why
and how
```
";
    assert_eq!(
        summary("abc123", &crates, &warnings, Some(&verdict)),
        expected
    );
}

#[test]
fn the_summary_cuts_a_diff_at_200_lines() {
    let head: Vec<String> = (0..203).map(|i| format!("pub fn f{i:03}()")).collect();
    let crates = vec![CrateApi::new("shuyi-ui", Some(&[]), &head, Vec::new())];
    let text = summary("abc", &crates, &[], None);
    assert!(
        text.contains("+pub fn f199()\n```\n\nAnd 3 more.\n"),
        "{text}"
    );
    assert!(!text.contains("f200"), "{text}");
    assert!(
        text.ends_with("No `--pr-branch`: the rule was not checked.\n"),
        "{text}"
    );
}

#[test]
fn the_summary_shows_exactly_200_lines_without_a_cut() {
    let head: Vec<String> = (0..200).map(|i| format!("pub fn f{i:03}()")).collect();
    let crates = vec![CrateApi::new("shuyi-ui", Some(&[]), &head, Vec::new())];
    let text = summary(
        "abc",
        &crates,
        &[],
        Some(&Verdict::Pass("fine.".to_string())),
    );
    assert!(text.contains("+pub fn f199()\n```\n\n</details>"), "{text}");
    assert!(!text.contains("more."), "{text}");
    assert!(text.ends_with("**Pass:** fine.\n"), "{text}");
}

// -- the tools --

#[test]
fn every_tool_present_is_nothing_missing() {
    let root = PathBuf::from("root");
    assert!(missing_tools(&root, &Fake::new(&root).with_tools()).is_empty());
}

#[test]
fn a_missing_tool_prints_its_install_line() {
    let root = PathBuf::from("root");
    let fake = Fake::new(&root)
        .with_tools()
        .answer(
            "head: cargo semver-checks --version",
            Ok(failed("error: no such command: `semver-checks`")),
        )
        .answer(
            "head: cargo public-api --version",
            Err("no cargo".to_string()),
        )
        .answer(
            "head: rustup toolchain list",
            Ok(ok("stable-x86_64\nnightly-x86_64\n")),
        );
    let missing = missing_tools(&root, &fake);
    assert_eq!(missing.len(), 3, "{missing:?}");
    assert!(missing[0].ends_with(&format!(
        "\n    cargo install --locked cargo-semver-checks@{SEMVER_CHECKS}"
    )));
    assert!(missing[1].ends_with(&format!(
        "\n    cargo install --locked cargo-public-api@{PUBLIC_API}"
    )));
    assert!(missing[2].ends_with(&format!(
        "\n    rustup toolchain install {NIGHTLY} --profile minimal"
    )));
}

#[test]
fn another_version_of_a_tool_is_missing_too() {
    let root = PathBuf::from("root");
    let fake = Fake::new(&root).with_tools().answer(
        "head: cargo semver-checks --version",
        Ok(ok("cargo-semver-checks 0.40.0\n")),
    );
    let missing = missing_tools(&root, &fake);
    assert_eq!(missing.len(), 1, "{missing:?}");
    assert!(
        missing[0].contains("not `cargo-semver-checks 0.40.0`"),
        "{missing:?}"
    );
}

#[test]
fn a_failed_toolchain_list_is_a_missing_nightly() {
    let root = PathBuf::from("root");
    let fake = Fake::new(&root).with_tools().answer(
        "head: rustup toolchain list",
        Ok(Ran {
            ok: false,
            stdout: format!("{NIGHTLY}\n"),
            stderr: String::new(),
        }),
    );
    assert_eq!(missing_tools(&root, &fake).len(), 1);
}

#[test]
fn the_nightly_is_known_by_its_name_on_any_host() {
    assert!(is_the_nightly(NIGHTLY));
    assert!(is_the_nightly(&format!(
        "{NIGHTLY}-x86_64-pc-windows-gnu (active)"
    )));
    assert!(!is_the_nightly("nightly-2026-05-230-x86_64"));
    assert!(!is_the_nightly(&format!("stable {NIGHTLY}")));
    assert!(!is_the_nightly(""));
}

// -- a whole run, on fakes --

/// The base has every crate but `shuyi-cli`; `shuyi-core` lost
/// `state_hash`, and the semver check says so.
fn breaking_run(root: &Path) -> Fake {
    let mut fake = Fake::new(root)
        .with_tools()
        .with_base("abc123")
        .with_base_packages(&API_CRATES[..5])
        .with_listing("head", "shuyi-core", "pub mod shuyi_core\n")
        .with_listing(
            "base",
            "shuyi-core",
            "pub mod shuyi_core\npub fn shuyi_core::state_hash()\n",
        )
        .with_listing("head", "shuyi-cli", "pub mod shuyi_cli\n\n")
        .with_semver("shuyi-core", "one-finding");
    for krate in &API_CRATES[1..5] {
        fake = fake.with_semver(krate, "pass");
    }
    fake
}

#[test]
fn a_run_compares_every_crate_and_removes_the_base() {
    let root = temp_root("run");
    let options = Options {
        base: Some("HEAD^1".to_string()),
        ..Options::default()
    };
    let fake = breaking_run(&root);
    let outcome = run(&root, &options, &fake);
    assert!(
        fake.ran()
            .contains(&"head: git rev-parse --verify HEAD^1^{commit}".to_string())
    );

    assert!(outcome.errors.is_empty(), "{:?}", outcome.errors);
    assert!(outcome.warnings.is_empty(), "{:?}", outcome.warnings);
    assert!(
        outcome.summary.contains("Against `abc123`."),
        "{}",
        outcome.summary
    );
    assert!(
        outcome.summary.contains("| `shuyi-core` | 1 | 0 | 2 |"),
        "{}",
        outcome.summary
    );
    assert!(
        outcome
            .summary
            .contains("| `shuyi-cli` (new) | 0 | 1 | not checked |"),
        "{}",
        outcome.summary
    );
    assert!(outcome.summary.contains("the rule was not checked"));

    let ran = fake.ran();
    assert!(ran.contains(&"head: git worktree add --detach target/api-base abc123".to_string()));
    let base_root = root.join(BASE_DIR).to_string_lossy().into_owned();
    assert!(ran.contains(&format!(
        "head: cargo semver-checks check-release -p shuyi-core --baseline-root {base_root} \
         --release-type minor --color never"
    )));
    let target = root.join("target").to_string_lossy().into_owned();
    assert!(ran.contains(&format!(
        "base: cargo +{NIGHTLY} public-api -p shuyi-ui --simplified --target-dir {target}"
    )));
    // `shuyi-cli` is new: listed in the head only, and never checked.
    assert!(
        !ran.iter()
            .any(|c| c.starts_with("base: cargo +") && c.contains("shuyi-cli"))
    );
    assert!(!ran.iter().any(|c| c.contains("check-release -p shuyi-cli")));
    assert_eq!(
        ran[ran.len() - 2..],
        [
            "head: git worktree remove --force target/api-base".to_string(),
            "head: git worktree prune".to_string(),
        ]
    );
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn a_break_with_its_line_in_the_done_ticket_passes() {
    let root = temp_root("recorded");
    fs::write(root.join("tickets/done/2100-a.md"), TICKET_2100).unwrap();
    let options = Options {
        base: Some("abc123".to_string()),
        pr_branch: Some("t2100-a".to_string()),
        ..Options::default()
    };
    let outcome = run(&root, &options, &breaking_run(&root));
    assert!(outcome.errors.is_empty(), "{:?}", outcome.errors);
    assert!(
        outcome
            .summary
            .ends_with("**Pass:** ticket 2100 has a `Breaking:` line for shuyi-core.\n"),
        "{}",
        outcome.summary
    );
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn a_break_without_its_line_fails() {
    let root = temp_root("unrecorded");
    fs::write(
        root.join("tickets/done/2100-a.md"),
        "## Completion notes\n\nDone.\n",
    )
    .unwrap();
    fs::write(root.join("tickets/done/2101-b.md"), TICKET_2100).unwrap();
    let options = Options {
        base: Some("abc123".to_string()),
        pr_branch: Some("t2100-a".to_string()),
        ..Options::default()
    };
    let outcome = run(&root, &options, &breaking_run(&root));
    assert_eq!(outcome.errors.len(), 1, "{:?}", outcome.errors);
    assert!(outcome.errors[0].contains("no `Breaking:` line naming shuyi-core"));
    assert!(outcome.summary.contains("**Fail:**"), "{}", outcome.summary);
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn a_lost_line_without_a_finding_is_a_warning() {
    let root = temp_root("lost");
    let fake = Fake::new(&root)
        .with_tools()
        .with_base("abc123")
        .with_base_packages(API_CRATES)
        .with_listing("base", "shuyi-ui", "pub fn shuyi_ui::a()\n");
    let options = Options {
        base: Some("abc123".to_string()),
        pr_branch: Some("sonar/1-x".to_string()),
        ..Options::default()
    };
    let fake = API_CRATES
        .iter()
        .fold(fake, |fake, krate| fake.with_semver(krate, "pass"));
    let outcome = run(&root, &options, &fake);
    assert!(outcome.errors.is_empty(), "{:?}", outcome.errors);
    assert_eq!(outcome.warnings.len(), 1, "{:?}", outcome.warnings);
    assert!(
        outcome
            .summary
            .contains("> **Warning:** shuyi-ui: 1 removed line(s)")
    );
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn the_base_defaults_to_where_the_head_left_main() {
    let root = temp_root("merge-base");
    let fake = breaking_run(&root)
        .answer("head: git merge-base HEAD main", Ok(ok("f00d\n")))
        .with_base("f00d");
    let outcome = run(&root, &Options::default(), &fake);
    assert!(
        fake.ran()
            .contains(&"head: git rev-parse --verify f00d^{commit}".to_string())
    );
    assert!(
        outcome.summary.contains("Against `f00d`."),
        "{}",
        outcome.summary
    );
    assert!(
        fake.ran()
            .contains(&"head: git worktree add --detach target/api-base f00d".to_string())
    );
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn no_merge_base_is_an_error() {
    let root = temp_root("no-merge-base");
    let fake = breaking_run(&root).answer(
        "head: git merge-base HEAD main",
        Ok(failed("fatal: Not a valid object name main")),
    );
    let outcome = run(&root, &Options::default(), &fake);
    assert_eq!(
        outcome.errors,
        vec!["`git merge-base HEAD main` failed:\nfatal: Not a valid object name main".to_string()]
    );
    assert!(outcome.summary.is_empty());
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn a_semver_build_error_fails_the_run_and_still_removes_the_base() {
    let root = temp_root("build-error");
    let fake = breaking_run(&root).answer(
        "head: cargo semver-checks check-release -p shuyi-core ",
        Ok(fixture("build-error")),
    );
    let options = Options {
        base: Some("abc123".to_string()),
        ..Options::default()
    };
    let outcome = run(&root, &options, &fake);
    assert_eq!(outcome.errors.len(), 1, "{:?}", outcome.errors);
    assert!(outcome.errors[0].contains("without naming a break"));
    assert!(outcome.summary.is_empty());
    assert_eq!(
        fake.ran().last().map(String::as_str),
        Some("head: git worktree prune")
    );
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn a_failed_listing_is_an_error() {
    let root = temp_root("listing-error");
    let fake = Fake::new(&root)
        .with_tools()
        .with_base("abc123")
        .with_base_packages(API_CRATES)
        .answer(
            &format!("head: cargo +{NIGHTLY} public-api -p shuyi-core"),
            Ok(failed("error: unsupported rustdoc JSON format")),
        );
    let options = Options {
        base: Some("abc123".to_string()),
        ..Options::default()
    };
    let outcome = run(&root, &options, &fake);
    assert_eq!(outcome.errors.len(), 1, "{:?}", outcome.errors);
    assert!(outcome.errors[0].ends_with("failed:\nerror: unsupported rustdoc JSON format"));
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn unreadable_metadata_is_an_error() {
    let root = temp_root("metadata");
    let fake = Fake::new(&root)
        .with_tools()
        .with_base("abc123")
        .answer("base: cargo metadata", Ok(ok("not json")));
    let options = Options {
        base: Some("abc123".to_string()),
        ..Options::default()
    };
    let outcome = run(&root, &options, &fake);
    assert_eq!(outcome.errors.len(), 1, "{:?}", outcome.errors);
    assert!(
        outcome.errors[0].starts_with("`cargo metadata` in "),
        "{:?}",
        outcome.errors
    );
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn a_failed_worktree_is_an_error() {
    let root = temp_root("worktree");
    let fake = Fake::new(&root)
        .with_tools()
        .with_base("abc123")
        .answer("head: git worktree add", Ok(failed("fatal: no space left")));
    let options = Options {
        base: Some("HEAD^1".to_string()),
        ..Options::default()
    };
    let outcome = run(&root, &options, &fake);
    assert_eq!(
        outcome.errors,
        vec![
            "`git worktree add --detach target/api-base abc123` failed:\nfatal: no space left"
                .to_string()
        ]
    );
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn an_unknown_base_is_an_error() {
    let root = temp_root("unknown-base");
    let fake = Fake::new(&root).with_tools().answer(
        "head: git rev-parse --verify ",
        Ok(failed("fatal: Needed a single revision")),
    );
    let options = Options {
        base: Some("nope".to_string()),
        ..Options::default()
    };
    let outcome = run(&root, &options, &fake);
    assert_eq!(
        outcome.errors,
        vec![
            "`git rev-parse --verify nope^{commit}` failed:\nfatal: Needed a single revision"
                .to_string()
        ]
    );
    assert!(!fake.ran().iter().any(|c| c.contains("worktree add")));
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn an_old_base_folder_is_removed_first() {
    let root = temp_root("old-base");
    fs::create_dir_all(root.join(BASE_DIR)).unwrap();
    fs::write(
        root.join(BASE_DIR).join("left.txt"),
        "from a run that crashed",
    )
    .unwrap();
    add_worktree(&root, "abc123", &Fake::new(&root)).unwrap();
    assert!(!root.join(BASE_DIR).exists());
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn missing_tools_stop_the_run_before_git() {
    let root = PathBuf::from("root");
    let fake = Fake::new(&root);
    let outcome = run(&root, &Options::default(), &fake);
    assert_eq!(outcome.errors.len(), 3, "{:?}", outcome.errors);
    assert!(!fake.ran().iter().any(|c| c.contains("git")));
}

#[test]
fn a_done_ticket_is_read_by_its_id() {
    let root = temp_root("done");
    fs::write(root.join("tickets/done/2100-a.md"), "first").unwrap();
    fs::write(root.join("tickets/done/2100-a.txt"), "not a ticket").unwrap();
    fs::write(root.join("tickets/done/21000-b.md"), "not this one").unwrap();
    assert_eq!(done_ticket(&root, "2100"), Some("first".to_string()));
    assert_eq!(done_ticket(&root, "2101"), None);
    assert_eq!(done_ticket(&root.join("nowhere"), "2100"), None);
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn the_system_runner_runs_a_command() {
    let ran = System.run(Path::new("."), "git", &["--version"]).unwrap();
    assert!(ran.ok);
    assert!(ran.stdout.starts_with("git version"), "{}", ran.stdout);
    let failed = System
        .run(Path::new("."), "git", &["no-such-command"])
        .unwrap();
    assert!(!failed.ok);
    assert!(!failed.stderr.is_empty());
    assert!(
        System
            .run(Path::new("."), "no-such-program-xtask", &[])
            .is_err()
    );
}

#[test]
fn ci_installs_the_pinned_tools_and_nightly() {
    let ci = include_str!("../../../../.github/workflows/ci.yml");
    for pinned in [
        format!("rustup toolchain install {NIGHTLY} --profile minimal"),
        format!("tool: cargo-semver-checks@{SEMVER_CHECKS}"),
        format!("cargo install --locked cargo-public-api@{PUBLIC_API}"),
    ] {
        assert!(ci.contains(&pinned), "ci.yml lacks `{pinned}`");
    }
}
