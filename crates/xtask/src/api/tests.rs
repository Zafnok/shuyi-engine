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

fn catalog_names() -> Vec<&'static str> {
    API_CRATES.iter().map(|krate| krate.name).collect()
}

/// `cargo metadata`'s JSON of a workspace of `packages`, each with the
/// features its entry in [`API_CRATES`] chooses.
fn metadata_json(packages: &[&str]) -> String {
    metadata_json_without(packages, &[])
}

/// [`metadata_json`], but `missing` (a crate and a feature) not in that
/// crate's manifest.
fn metadata_json_without(packages: &[&str], missing: &[(&str, &str)]) -> String {
    metadata_json_with(packages, missing, &[])
}

/// [`metadata_json_without`], with each crate of `deps` depending on its
/// list: a name `dev:<name>` as a dev-dependency, any other as a normal one.
fn metadata_json_with(
    packages: &[&str],
    missing: &[(&str, &str)],
    deps: &[(&str, &[&str])],
) -> String {
    let list: Vec<String> = packages
        .iter()
        .map(|name| {
            let chosen =
                API_CRATES
                    .iter()
                    .find(|krate| krate.name == *name)
                    .map_or(&[][..], |krate| {
                        let Features::Only(chosen) = krate.features;
                        chosen
                    });
            let features: Vec<String> = chosen
                .iter()
                .filter(|feature| !missing.contains(&(*name, **feature)))
                .map(|feature| format!("\"{feature}\":[]"))
                .collect();
            let dependencies: Vec<String> = deps
                .iter()
                .filter(|(krate, _)| krate == name)
                .flat_map(|(_, list)| list.iter())
                .map(|dep| match dep.strip_prefix("dev:") {
                    Some(dep) => format!("{{\"name\":\"{dep}\",\"kind\":\"dev\"}}"),
                    None => format!("{{\"name\":\"{dep}\",\"kind\":null}}"),
                })
                .collect();
            format!(
                "{{\"name\":\"{name}\",\"features\":{{{}}},\"dependencies\":[{}]}}",
                features.join(","),
                dependencies.join(",")
            )
        })
        .collect();
    format!("{{\"packages\":[{}]}}", list.join(","))
}

/// The head's `cargo metadata` with dependencies.
const RESOLVE: &str = "head: cargo metadata --format-version 1 --locked";

/// `cargo metadata`'s JSON with dependencies, of a workspace of `members`
/// and the packages `deps` names: each package's normal dependencies (a
/// name `dev:<name>` as a dev-dependency). A package's id is its name, and
/// its library's name is its own, `-` as `_`.
fn resolve_json(members: &[&str], deps: &[(&str, &[&str])]) -> String {
    let mut names: Vec<&str> = members.to_vec();
    for (krate, list) in deps {
        names.push(krate);
        names.extend(list.iter().map(|dep| dep.trim_start_matches("dev:")));
    }
    names.sort_unstable();
    names.dedup();
    let packages: Vec<String> = names
        .iter()
        .map(|name| {
            format!(
                "{{\"id\":\"{name}\",\"name\":\"{name}\",\"targets\":[{{\"kind\":[\"lib\"],                 \"name\":\"{}\"}}]}}",
                name.replace('-', "_")
            )
        })
        .collect();
    let nodes: Vec<String> = deps
        .iter()
        .map(|(krate, list)| {
            let deps: Vec<String> = list
                .iter()
                .map(|dep| match dep.strip_prefix("dev:") {
                    Some(dep) => {
                        format!("{{\"pkg\":\"{dep}\",\"dep_kinds\":[{{\"kind\":\"dev\"}}]}}")
                    }
                    None => format!("{{\"pkg\":\"{dep}\",\"dep_kinds\":[{{\"kind\":null}}]}}"),
                })
                .collect();
            format!("{{\"id\":\"{krate}\",\"deps\":[{}]}}", deps.join(","))
        })
        .collect();
    let members: Vec<String> = members.iter().map(|name| format!("\"{name}\"")).collect();
    format!(
        "{{\"packages\":[{}],\"workspace_members\":[{}],\"resolve\":{{\"nodes\":[{}]}}}}",
        packages.join(","),
        members.join(","),
        nodes.join(",")
    )
}

/// A fake [`Runner`]. Each command is written `<where>: <program> <args>`,
/// `<where>` being `head` (the root) or `base` (the base's tree); the last
/// answer given whose key starts the command answers it, and any other
/// command succeeds and prints nothing. It records every command, and the
/// variables each command run with some was given.
struct Fake {
    root: PathBuf,
    answers: Vec<(String, Result<Ran, String>)>,
    ran: RefCell<Vec<String>>,
    envs: RefCell<Vec<(String, Vec<String>)>>,
}

impl Fake {
    fn new(root: &Path) -> Self {
        Self {
            root: root.to_path_buf(),
            answers: Vec::new(),
            ran: RefCell::new(Vec::new()),
            envs: RefCell::new(Vec::new()),
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

    /// The base revision is the commit `commit`, and the head's workspace
    /// has the whole catalog.
    fn with_base(self, commit: &str) -> Self {
        self.answer(
            "head: git rev-parse --verify ",
            Ok(ok(&format!("{commit}\n"))),
        )
        .with_head_packages(&catalog_names())
    }

    /// The base's workspace has `packages`.
    fn with_base_packages(self, packages: &[&str]) -> Self {
        self.answer(
            "base: cargo metadata --no-deps --format-version 1",
            Ok(ok(&metadata_json(packages))),
        )
    }

    /// The head's workspace has `packages`, whose dependencies reach no
    /// crate.
    fn with_head_packages(self, packages: &[&str]) -> Self {
        self.answer(
            "head: cargo metadata --no-deps --format-version 1",
            Ok(ok(&metadata_json(packages))),
        )
        .answer(RESOLVE, Ok(ok("{}")))
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
    fn run_with_env(
        &self,
        dir: &Path,
        program: &str,
        args: &[&str],
        env: &[(&str, &str)],
    ) -> Result<Ran, String> {
        let names: Vec<String> = env.iter().map(|(k, v)| format!("{k}={v}")).collect();
        self.envs
            .borrow_mut()
            .push((format!("{program} {}", args.join(" ")), names));
        self.run(dir, program, args)
    }

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
                           Breaking: shuyi-core: `shuyi_core::state_hash`: hash with `StateHash::of`.\n\
                           Book: none, the hash is not in the book.\n";

// -- the options --

#[test]
fn options_read_each_flag() {
    let args = lines(&[
        "--base",
        "HEAD^1",
        "--pr-branch",
        "t2010-api",
        "--pr-title",
        "[2010] The API",
        "--summary",
        "out.md",
        "--verdict",
        "verdict.txt",
    ]);
    assert_eq!(
        Options::parse(&args),
        Some(Options {
            base: Some("HEAD^1".to_string()),
            pr_branch: Some("t2010-api".to_string()),
            pr_title: Some("[2010] The API".to_string()),
            summary: Some(PathBuf::from("out.md")),
            verdict: Some(PathBuf::from("verdict.txt")),
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
    assert_eq!(api.added, Vec::<String>::new());
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
    assert_eq!(api.removed, Vec::<String>::new());
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
    assert_eq!(api.removed, Vec::<String>::new());
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

/// A PR on `branch`, with no title (a local run).
fn pr(branch: &str) -> Pr<'_> {
    Pr {
        branch,
        title: None,
    }
}

/// A PR on a desktop worktree's branch, titled `title`.
fn titled(title: &str) -> Pr<'_> {
    Pr {
        branch: "claude/x-1a2b3c",
        title: Some(title),
    }
}

#[test]
fn a_break_reads_the_ticket_from_the_title() {
    let verdict = rule(&breaking_core(), &titled("[2100] A"), &|id| {
        (id == "2100").then(|| TICKET_2100.to_string())
    });
    assert!(matches!(verdict, Verdict::Pass(_)), "{verdict:?}");
    let Verdict::Fail(why) = rule(&breaking_core(), &titled("[2100] A"), &no_ticket) else {
        panic!("passed");
    };
    assert!(
        why.starts_with("ticket 2100 is not in tickets/done/"),
        "{why}"
    );
}

#[test]
fn a_book_line_is_read_from_the_titles_ticket() {
    let Verdict::Fail(why) = book_rule(&added_ui(), &titled("[2100] A"), &[], &done_with("Done."))
    else {
        panic!("passed");
    };
    assert!(
        why.starts_with("ticket 2100's `## Completion notes`"),
        "{why}"
    );
    let verdict = book_rule(
        &added_ui(),
        &titled("[2100] A"),
        &[],
        &done_with("Book: none, a test helper."),
    );
    assert!(matches!(verdict, Verdict::Pass(_)), "{verdict:?}");
}

#[test]
fn a_break_on_a_titled_ticket_only_pr_fails() {
    let Verdict::Fail(why) = rule(&breaking_core(), &titled("Ticket 2100: A"), &no_ticket) else {
        panic!("passed");
    };
    assert!(why.contains("names no ticket's work"), "{why}");
    assert!(why.contains("`[NNNN] …`"), "{why}");
}

#[test]
fn a_title_and_branch_that_disagree_fail_a_break() {
    let pr = Pr {
        branch: "t2101-b",
        title: Some("[2100] A"),
    };
    let Verdict::Fail(why) = rule(&breaking_core(), &pr, &no_ticket) else {
        panic!("passed");
    };
    assert!(why.contains("names ticket 2101"), "{why}");
}

#[test]
fn a_game_small_change_adds_no_public_item() {
    let Verdict::Fail(why) = rule(&added_ui(), &titled("Game 0859: a hook"), &no_ticket) else {
        panic!("passed");
    };
    assert!(why.contains("game ticket 0859's small change"), "{why}");
    let listing = lines(&["pub fn a()"]);
    let same = [CrateApi::new(
        "shuyi-ui",
        Some(&listing),
        &listing,
        Vec::new(),
    )];
    assert!(matches!(
        rule(&same, &titled("Game 0859: a fix"), &no_ticket),
        Verdict::Pass(_)
    ));
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
        rule(&crates, &pr("t2100-a"), &no_ticket),
        Verdict::Pass(_)
    ));
}

#[test]
fn a_break_with_a_line_naming_the_crate_passes() {
    let verdict = rule(&breaking_core(), &pr("t2100-a"), &|id| {
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
    let verdict = rule(&breaking_core(), &pr("t2100-a"), &|_| {
        Some(ticket.to_string())
    });
    assert!(matches!(verdict, Verdict::Pass(_)), "{verdict:?}");
}

#[test]
fn a_line_naming_another_crate_fails() {
    let ticket = "## Completion notes\n\nBreaking: shuyi-ui: `x`: y.\n";
    let Verdict::Fail(why) = rule(&breaking_core(), &pr("t2100-a"), &|_| {
        Some(ticket.to_string())
    }) else {
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
    let verdict = rule(&breaking_core(), &pr("t2100-a"), &|_| {
        Some(ticket.to_string())
    });
    assert!(matches!(verdict, Verdict::Fail(_)), "{verdict:?}");
}

#[test]
fn a_ticket_not_in_done_fails() {
    let Verdict::Fail(why) = rule(&breaking_core(), &pr("t2100-a"), &no_ticket) else {
        panic!("passed");
    };
    assert!(
        why.starts_with("ticket 2100 is not in tickets/done/"),
        "{why}"
    );
}

#[test]
fn a_break_on_a_sonar_branch_fails() {
    let Verdict::Fail(why) = rule(&breaking_core(), &pr("sonar/312-lifetimes"), &no_ticket) else {
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
        rule(&crates, &pr("sonar/312-lifetimes"), &no_ticket),
        Verdict::Pass(_)
    ));
}

// -- the book's rule --

/// A change to `shuyi-ui`'s public API with no semver finding.
fn added_ui() -> Vec<CrateApi> {
    vec![
        CrateApi::new(
            "shuyi-ui",
            Some(&[]),
            &lines(&["pub fn shuyi_ui::a()"]),
            Vec::new(),
        ),
        CrateApi::new("shuyi-core", Some(&[]), &[], Vec::new()),
    ]
}

fn done_with(notes: &'static str) -> impl Fn(&str) -> Option<String> {
    move |id| (id == "2100").then(|| format!("## Completion notes\n\n{notes}\n"))
}

#[test]
fn an_api_change_without_the_book_or_a_line_fails() {
    let changed = lines(&["crates/ui/src/lib.rs", "site/content/news/x.md"]);
    let Verdict::Fail(why) = book_rule(&added_ui(), &pr("t2100-a"), &changed, &done_with("Done."))
    else {
        panic!("passed");
    };
    assert!(
        why.starts_with("ticket 2100's `## Completion notes` have no `Book:` line."),
        "{why}"
    );
    assert!(why.contains("The public API of shuyi-ui changed"), "{why}");
    assert!(!why.contains("shuyi-core"), "{why}");
    assert!(why.contains("the docs map (docs/README.md)"), "{why}");
    assert!(why.ends_with("\n\n    Book: none, <why>"), "{why}");
}

#[test]
fn an_api_change_with_a_book_line_passes_and_quotes_it() {
    let verdict = book_rule(
        &added_ui(),
        &pr("t2100-a"),
        &[],
        &done_with("- Book: none, a helper the tests call made public."),
    );
    assert_eq!(
        verdict,
        Verdict::Pass(
            "ticket 2100's notes say: Book: none, a helper the tests call made public.".to_string()
        )
    );
}

#[test]
fn an_api_change_with_the_book_changed_passes() {
    for file in [
        "site/content/learn/book/screens.md",
        "site/snippets/src/rules.rs",
    ] {
        let changed = lines(&["crates/ui/src/lib.rs", file]);
        let verdict = book_rule(&added_ui(), &pr("t2100-a"), &changed, &done_with("Done."));
        assert!(
            matches!(&verdict, Verdict::Pass(why) if why.starts_with("the book changed")),
            "{verdict:?}"
        );
    }
}

#[test]
fn a_branch_with_no_ticket_is_skipped() {
    for branch in [
        "dependabot/cargo/serde-1.0.300",
        "sonar/312-lifetimes",
        "game-1203-x",
    ] {
        let verdict = book_rule(&added_ui(), &pr(branch), &[], &no_ticket);
        assert!(
            matches!(&verdict, Verdict::Pass(why) if why.contains("names no ticket")),
            "{verdict:?}"
        );
    }
}

#[test]
fn no_api_change_asks_nothing_of_the_book() {
    let listing = lines(&["pub fn a()"]);
    let crates = [CrateApi::new(
        "shuyi-ui",
        Some(&listing),
        &listing,
        Vec::new(),
    )];
    assert!(matches!(
        book_rule(&crates, &pr("t2100-a"), &[], &no_ticket),
        Verdict::Pass(_)
    ));
}

#[test]
fn an_api_change_on_a_ticket_not_done_fails() {
    let Verdict::Fail(why) = book_rule(&added_ui(), &pr("t2100-a"), &[], &no_ticket) else {
        panic!("passed");
    };
    assert!(
        why.starts_with("ticket 2100 is not in tickets/done/"),
        "{why}"
    );
}

#[test]
fn the_summary_ends_with_the_books_verdict() {
    let crates = added_ui();
    let pass = Verdict::Pass("fine.".to_string());
    let text = summary("abc", &crates, &[], Some(&pass), Some(&pass));
    assert!(
        text.ends_with("**Pass:** fine.\n\n**Book:** fine.\n"),
        "{text}"
    );
    let fail = Verdict::Fail("no line\n".to_string());
    let text = summary("abc", &crates, &[], Some(&pass), Some(&fail));
    assert!(
        text.ends_with("**Book, fail:**\n\n```text\nno line\n```\n"),
        "{text}"
    );
}

#[test]
fn named_crates_read_the_list_before_the_item() {
    assert_eq!(
        named_crates("Breaking: shuyi-core, `shuyi-ui`: `f`: g: h"),
        vec!["shuyi-core", "shuyi-ui"]
    );
    assert_eq!(named_crates("Breaking: shuyi-core"), vec!["shuyi-core"]);
    assert_eq!(named_crates("Breaking: : x"), Vec::<&str>::new());
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
        summary("abc123", &crates, &warnings, Some(&verdict), None),
        expected
    );
}

#[test]
fn the_summary_cuts_a_diff_at_200_lines() {
    let head: Vec<String> = (0..203).map(|i| format!("pub fn f{i:03}()")).collect();
    let crates = vec![CrateApi::new("shuyi-ui", Some(&[]), &head, Vec::new())];
    let text = summary("abc", &crates, &[], None, None);
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
        None,
    );
    assert!(text.contains("+pub fn f199()\n```\n\n</details>"), "{text}");
    assert!(!text.contains("more."), "{text}");
    assert!(text.ends_with("**Pass:** fine.\n"), "{text}");
}

// -- the tools --

#[test]
fn every_tool_present_is_nothing_missing() {
    let root = PathBuf::from("root");
    assert_eq!(
        missing_tools(&root, &Fake::new(&root).with_tools()),
        Vec::<String>::new()
    );
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
    assert!(missing[0].contains("is not installed"), "{missing:?}");
    assert!(missing[1].contains("is not installed"), "{missing:?}");
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
    assert!(!is_the_nightly(&format!("{NIGHTLY}0-x86_64")));
    assert!(!is_the_nightly(&format!("stable {NIGHTLY}")));
    assert!(!is_the_nightly(""));
}

// -- the verdict --

#[test]
fn no_diff_and_no_finding_is_none() {
    let listing = lines(&["pub fn a()"]);
    let crates = [
        CrateApi::new("shuyi-core", Some(&listing), &listing, Vec::new()),
        CrateApi::new("shuyi-ui", Some(&[]), &[], Vec::new()),
    ];
    assert_eq!(verdict(&crates), Label::None);
    assert_eq!(verdict(&[]), Label::None);
}

#[test]
fn a_diff_and_no_finding_is_added() {
    let listing = lines(&["pub fn a()"]);
    let added = CrateApi::new(
        "shuyi-ui",
        Some(&listing),
        &lines(&["pub fn a()", "pub fn b()"]),
        Vec::new(),
    );
    let same = CrateApi::new("shuyi-core", Some(&listing), &listing, Vec::new());
    assert_eq!(verdict(&[same, added]), Label::Added);
    // A lost line semver doesn't call breaking is still a listing diff.
    let lost = CrateApi::new("shuyi-ui", Some(&listing), &[], Vec::new());
    assert_eq!(verdict(&[lost]), Label::Added);
    let new = CrateApi::new(
        "shuyi-net",
        None,
        &lines(&["pub mod shuyi_net"]),
        Vec::new(),
    );
    assert_eq!(verdict(&[new]), Label::Added);
}

#[test]
fn any_finding_is_breaking() {
    assert_eq!(verdict(&breaking_core()), Label::Breaking);
    // A finding breaks even where the listing didn't change.
    let listing = lines(&["pub trait A"]);
    let finding_only = CrateApi::new(
        "shuyi-core",
        Some(&listing),
        &listing,
        vec![finding("trait_method_added", &["A::b"])],
    );
    assert_eq!(verdict(&[finding_only]), Label::Breaking);
}

#[test]
fn each_label_has_its_word() {
    assert_eq!(
        [Label::None, Label::Added, Label::Breaking].map(Label::word),
        ["none", "added", "breaking"]
    );
}

#[test]
fn a_verdict_is_written_to_its_file() {
    let root = temp_root("verdict");
    let file = root.join("verdict");
    fs::write(&file, "old").unwrap();
    assert_eq!(write_verdict(Some(Label::Added), Some(&file)), None);
    assert_eq!(fs::read_to_string(&file).unwrap(), "added");
    assert_eq!(write_verdict(Some(Label::Added), None), None);
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn no_verdict_writes_no_file() {
    let root = temp_root("no-verdict");
    let file = root.join("verdict");
    assert_eq!(write_verdict(None, Some(&file)), None);
    assert!(!file.exists());
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn a_verdict_file_that_cannot_be_written_is_an_error() {
    let root = temp_root("bad-verdict");
    let error = write_verdict(Some(Label::None), Some(&root)).unwrap();
    assert!(error.starts_with(&root.display().to_string()), "{error}");
    let _ = fs::remove_dir_all(&root);
}

// -- a whole run, on fakes --

/// The base has every crate but `shuyi-cli`; `shuyi-core` lost
/// `state_hash`, and the semver check says so.
fn breaking_run(root: &Path) -> Fake {
    let mut fake = Fake::new(root)
        .with_tools()
        .with_base("abc123")
        .with_base_packages(&catalog_names()[..5])
        .with_listing("head", "shuyi-core", "pub mod shuyi_core\n")
        .with_listing(
            "base",
            "shuyi-core",
            "pub mod shuyi_core\npub fn shuyi_core::state_hash()\n",
        )
        .with_listing("head", "shuyi-cli", "pub mod shuyi_cli\n\n")
        .with_semver("shuyi-core", "one-finding");
    for krate in &catalog_names()[1..5] {
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
    assert_eq!(outcome.label, Some(Label::Breaking));

    let ran = fake.ran();
    assert!(ran.contains(&"head: git worktree add --detach target/api-base abc123".to_string()));
    let base_root = root.join(BASE_DIR).to_string_lossy().into_owned();
    assert!(ran.contains(&format!(
        "head: cargo semver-checks check-release -p shuyi-core --baseline-root {base_root} \
         --release-type minor --color never{CORE_SEMVER}"
    )));
    let target = root.join("target").to_string_lossy().into_owned();
    assert!(ran.contains(&format!(
        "head: cargo +{NIGHTLY} public-api -p shuyi-ui --simplified{UI_LISTING} \
         --target-dir {target}"
    )));
    // The base builds apart, so it never builds on the head's crates.
    let base_target = root.join(BASE_TARGET_DIR).to_string_lossy().into_owned();
    assert!(ran.contains(&format!(
        "base: cargo +{NIGHTLY} public-api -p shuyi-ui --simplified{UI_LISTING} \
         --target-dir {base_target}"
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
        outcome.summary.ends_with(
            "**Pass:** ticket 2100 has a `Breaking:` line for shuyi-core.\n\n\
             **Book:** ticket 2100's notes say: Book: none, the hash is not in the book.\n"
        ),
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
    // Its notes have no `Book:` line either.
    assert_eq!(outcome.errors.len(), 2, "{:?}", outcome.errors);
    assert!(outcome.errors[0].contains("no `Breaking:` line naming shuyi-core"));
    assert!(outcome.errors[1].contains("have no `Book:` line"));
    assert!(outcome.summary.contains("**Fail:**"), "{}", outcome.summary);
    assert!(
        outcome.summary.contains("**Book, fail:**"),
        "{}",
        outcome.summary
    );
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn a_break_of_the_contracts_schemas_needs_a_line_naming_shuyi_cli() {
    let root = temp_root("contract-break");
    let schemas = root.join("crates/cli/schemas");
    fs::create_dir_all(&schemas).unwrap();
    fs::write(
        schemas.join("scene.output.json"),
        r#"{"type":"object","properties":{}}"#,
    )
    .unwrap();
    fs::write(
        root.join("tickets/done/2100-a.md"),
        "## Completion notes\n\nBreaking: shuyi-core: state_hash: gone\n\nBook: none, a test\n",
    )
    .unwrap();
    let options = Options {
        base: Some("abc123".to_string()),
        pr_branch: Some("t2100-a".to_string()),
        ..Options::default()
    };
    let fake = breaking_run(&root)
        .answer(
            "head: git ls-tree --name-only abc123:crates/cli/schemas",
            Ok(ok("scene.output.json\nversions.json\n")),
        )
        .answer(
            "head: git show abc123:crates/cli/schemas/scene.output.json",
            Ok(ok(
                r#"{"type":"object","properties":{"frames":{"type":"integer"}}}"#,
            )),
        );
    let outcome = run(&root, &options, &fake);
    assert_eq!(outcome.errors.len(), 1, "{:?}", outcome.errors);
    let error = &outcome.errors[0];
    assert!(
        error.contains("no `Breaking:` line naming shuyi-cli"),
        "{error}"
    );
    assert!(error.contains("frames: the field is gone"), "{error}");
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn a_run_reads_the_books_change_from_git() {
    let root = temp_root("book-changed");
    fs::write(
        root.join("tickets/done/2100-a.md"),
        "## Completion notes\n\nBreaking: shuyi-core: `f`: g.\n",
    )
    .unwrap();
    let options = Options {
        base: Some("abc123".to_string()),
        pr_branch: Some("t2100-a".to_string()),
        ..Options::default()
    };
    let fake = breaking_run(&root).answer(
        "head: git diff --name-only abc123",
        Ok(ok("crates/core/src/lib.rs\nsite/snippets/src/rules.rs\n")),
    );
    let outcome = run(&root, &options, &fake);
    assert!(outcome.errors.is_empty(), "{:?}", outcome.errors);
    assert!(
        outcome.summary.ends_with(
            "**Book:** the book changed with the public API (1 file(s) under \
             site/content/learn/ or site/snippets/).\n"
        ),
        "{}",
        outcome.summary
    );
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn a_failed_git_diff_fails_the_book_rule() {
    let root = temp_root("book-diff-failed");
    fs::write(root.join("tickets/done/2100-a.md"), TICKET_2100).unwrap();
    let options = Options {
        base: Some("abc123".to_string()),
        pr_branch: Some("t2100-a".to_string()),
        ..Options::default()
    };
    let fake = breaking_run(&root).answer(
        "head: git diff --name-only",
        Ok(failed("fatal: bad revision")),
    );
    let outcome = run(&root, &options, &fake);
    assert_eq!(outcome.errors.len(), 1, "{:?}", outcome.errors);
    assert!(
        outcome.errors[0].contains("`git diff --name-only abc123` failed"),
        "{:?}",
        outcome.errors
    );
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn a_lost_line_without_a_finding_is_a_warning() {
    let root = temp_root("lost");
    let fake = Fake::new(&root)
        .with_tools()
        .with_base("abc123")
        .with_base_packages(&catalog_names())
        .with_listing("base", "shuyi-ui", "pub fn shuyi_ui::a()\n");
    let options = Options {
        base: Some("abc123".to_string()),
        pr_branch: Some("sonar/1-x".to_string()),
        ..Options::default()
    };
    let fake = catalog_names()
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
    assert_eq!(outcome.label, Some(Label::Added));
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
    assert_eq!(outcome.summary, "");
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn the_semver_check_builds_in_a_short_target_folder_of_its_own() {
    let root = PathBuf::from("D:/repo/.claude/worktrees/some-worktree-name");
    let fake = Fake::new(&root).answer(
        "head: cargo semver-checks check-release",
        Ok(fixture("pass")),
    );
    let base_root = root.join(BASE_DIR);
    let findings = semver(&root, "shuyi-core", &[], &base_root, &fake).unwrap();
    assert_eq!(findings, Vec::<Finding>::new());
    let target = root
        .join("target/api-semver")
        .to_string_lossy()
        .into_owned();
    let envs = fake.envs.borrow();
    assert_eq!(envs.len(), 1, "{envs:?}");
    assert!(
        envs[0]
            .0
            .starts_with("cargo semver-checks check-release -p shuyi-core ")
    );
    assert_eq!(envs[0].1, vec![format!("CARGO_TARGET_DIR={target}")]);

    // Ticket 2061: Windows starts no program at a path of 260 characters or
    // more. A build script there is `<target>/debug/build/<dependency>-<16
    // hex>/build-script-build.exe`. With a root of 120 characters (the
    // longest worktree seen was 77) and a dependency's name of 40 characters,
    // it stays under.
    let longest = 120
        + 1
        + SEMVER_TARGET_DIR.len()
        + "/debug/build/".len()
        + 40
        + 1
        + 16
        + "/build-script-build.exe".len();
    assert!(longest < 260, "{longest}");
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
    assert_eq!(outcome.summary, "");
    // A failed run has no verdict, so `--verdict` writes no file.
    assert_eq!(outcome.label, None);
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
        .with_base_packages(&catalog_names())
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

// -- the catalog --

fn members(names: &[&str]) -> HashSet<String> {
    names.iter().map(|name| (*name).to_string()).collect()
}

#[test]
fn the_catalog_is_the_real_workspaces_engine_crates() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let real = packages(&root, &System).unwrap().into_keys().collect();
    assert_eq!(check_catalog(&real, API_CRATES), Ok(()));
    assert_eq!(API_CRATES.len(), 14);
}

#[test]
fn an_engine_member_left_out_is_named() {
    let mut names = catalog_names();
    names.extend(["shuyi-net", "shuyi-gba"]);
    let error = check_catalog(&members(&names), API_CRATES).unwrap_err();
    assert!(error.contains(": shuyi-gba, shuyi-net."), "{error}");
}

#[test]
fn what_is_no_engine_crate_needs_no_entry() {
    let mut names = catalog_names();
    names.extend(["xtask", "hello-grid", "book-snippets", "shuyish"]);
    assert_eq!(check_catalog(&members(&names), API_CRATES), Ok(()));
}

#[test]
fn an_entry_that_is_no_member_is_named() {
    let names = catalog_names();
    let error = check_catalog(&members(&names[1..]), API_CRATES).unwrap_err();
    assert!(
        error.ends_with("no workspace member: shuyi-core"),
        "{error}"
    );
}

#[test]
fn a_run_on_an_engine_member_left_out_fails_before_any_listing() {
    let root = temp_root("left-out");
    let mut names = catalog_names();
    names.push("shuyi-net");
    let fake = Fake::new(&root)
        .with_tools()
        .with_base("abc123")
        .with_head_packages(&names)
        .with_base_packages(&catalog_names());
    let outcome = run(&root, &Options::default(), &fake);
    assert_eq!(outcome.errors.len(), 1, "{:?}", outcome.errors);
    assert!(
        outcome.errors[0].contains("shuyi-net"),
        "{:?}",
        outcome.errors
    );
    assert_eq!(outcome.label, None);
    assert!(!fake.ran().iter().any(|c| c.contains("public-api -p")));
    assert_eq!(
        fake.ran().last().map(String::as_str),
        Some("head: git worktree prune")
    );
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn features_become_each_tools_flags() {
    let global = members(&["global", "std"]);
    let none = HashSet::new();
    assert_eq!(
        Features::Only(&[]).listing_args(&global),
        ["--no-default-features"]
    );
    assert_eq!(
        Features::Only(&[]).semver_args(&global, &global),
        ["--only-explicit-features"]
    );
    assert_eq!(
        Features::Only(&["global"]).listing_args(&global),
        ["--no-default-features", "--features", "global"]
    );
    assert_eq!(
        Features::Only(&["global"]).semver_args(&global, &global),
        ["--only-explicit-features", "--features", "global"]
    );
    // A side without the feature builds without it, and the semver check
    // takes it only when both sides have it.
    assert_eq!(
        Features::Only(&["global"]).listing_args(&none),
        ["--no-default-features"]
    );
    assert_eq!(
        Features::Only(&["global"]).semver_args(&global, &none),
        ["--only-explicit-features"]
    );
    assert_eq!(
        Features::Only(&["global"]).semver_args(&none, &global),
        ["--only-explicit-features"]
    );
}

#[test]
fn a_feature_new_in_the_head_is_listed_there_and_its_items_are_added() {
    let root = temp_root("new-feature");
    let fake = Fake::new(&root)
        .with_tools()
        .with_base("abc123")
        .answer(
            "base: cargo metadata --no-deps --format-version 1",
            Ok(ok(&metadata_json_without(
                &catalog_names(),
                &[("shuyi-core", "toy")],
            ))),
        )
        .with_listing("head", "shuyi-core", "pub fn shuyi_core::toy::run()\n");
    let outcome = run(&root, &Options::default(), &fake);
    assert!(outcome.errors.is_empty(), "{:?}", outcome.errors);
    assert_eq!(outcome.label, Some(Label::Added));
    let ran = fake.ran();
    let target = root.join("target").to_string_lossy().into_owned();
    let base_target = root.join(BASE_TARGET_DIR).to_string_lossy().into_owned();
    let base_root = root.join(BASE_DIR).to_string_lossy().into_owned();
    for command in [
        format!(
            "head: cargo +{NIGHTLY} public-api -p shuyi-core --simplified \
             --no-default-features --features std --features schema --features toy \
             --target-dir {target}"
        ),
        format!(
            "base: cargo +{NIGHTLY} public-api -p shuyi-core --simplified \
             --no-default-features --features std --features schema \
             --target-dir {base_target}"
        ),
        format!(
            "head: cargo semver-checks check-release -p shuyi-core --baseline-root {base_root} \
             --release-type minor --color never --only-explicit-features --features std \
             --features schema"
        ),
    ] {
        assert!(ran.contains(&command), "no `{command}` in {ran:#?}");
    }
    let _ = fs::remove_dir_all(&root);
}

// -- the two crates checked in a configuration of their own --

/// What `cargo semver-checks` prints on the removed function `item`.
fn removed(item: &str) -> Ran {
    Ran {
        ok: false,
        stdout: format!(
            "\n--- failure function_missing: pub fn removed or renamed ---\n\n\
             Failed in:\n  function {item}, previously in file /x.rs:1\n"
        ),
        stderr: "     Summary semver requires new major version: 1 major\n".to_string(),
    }
}

/// `krate` lost `item`, a line of its base's listing; every crate is in the
/// base.
fn break_in(root: &Path, krate: &str, item: &str) -> Fake {
    Fake::new(root)
        .with_tools()
        .with_base("abc123")
        .with_base_packages(&catalog_names())
        .with_listing("base", krate, &format!("pub fn {item}()\n"))
        .answer(
            &format!("head: cargo semver-checks check-release -p {krate} "),
            Ok(removed(item)),
        )
}

const ADDED_CRATES: [(&str, &str, &str, &str); 2] = [
    (
        "shuyi-playdate",
        "shuyi_playdate::stack_high_water",
        " --no-default-features",
        " --only-explicit-features",
    ),
    (
        "shuyi-alloc-count",
        "shuyi_alloc_count::global::rows",
        " --no-default-features --features global",
        " --only-explicit-features --features global",
    ),
];

/// `shuyi-core`'s semver flags, and `shuyi-ui`'s listing flags.
const CORE_SEMVER: &str =
    " --only-explicit-features --features std --features schema --features toy";
const UI_LISTING: &str = " --no-default-features --features std --features debug-tools \
     --features storage-conformance --features onebit-reference --features raster \
     --features toy --features schema";

/// Each crate of the catalog and the features it is checked with (ticket
/// 2041), spelled out here rather than read from [`API_CRATES`].
const CONFIGURATIONS: &[(&str, &[&str])] = &[
    ("shuyi-core", &["std", "schema", "toy"]),
    (
        "shuyi-assets",
        &[
            "std",
            "png",
            "sample-actions",
            "golden",
            "parse-count",
            "jpeg",
            "webp",
            "qoi",
            "jpeg-xl",
            "flac",
            "mp3",
            "sfx",
        ],
    ),
    (
        "shuyi-ui",
        &[
            "std",
            "debug-tools",
            "storage-conformance",
            "onebit-reference",
            "raster",
            "toy",
            "schema",
        ],
    ),
    (
        "shuyi-platform",
        &["std", "harness", "link", "heap", "capture", "schema"],
    ),
    (
        "shuyi-app",
        &["link", "capture", "sound-probe", "system-languages"],
    ),
    ("shuyi-cli", &["toy", "jis"]),
    (
        "shuyi",
        &["std", "app", "schema", "script", "system-languages"],
    ),
    ("shuyi-sound", &["std", "wav", "vorbis"]),
    ("shuyi-sound-out", &[]),
    ("shuyi-playdate", &[]),
    ("shuyi-alloc-count", &["global"]),
    ("shuyi-embed", &[]),
    ("shuyi-embed-macros", &[]),
    ("shuyi-script", &[]),
];

/// The features a crate's manifest has that its check leaves out, each for
/// the reason `API_CRATES` gives.
const LEFT_OUT: [(&str, &str); 1] = [("shuyi-app", "in-house-sound")];

#[test]
fn every_crate_has_one_configuration_on_both_sides_and_in_both_tools() {
    let root = temp_root("configuration");
    let fake = Fake::new(&root)
        .with_tools()
        .with_base("abc123")
        .with_base_packages(&catalog_names());
    let outcome = run(&root, &Options::default(), &fake);
    assert!(outcome.errors.is_empty(), "{:?}", outcome.errors);
    let ran = fake.ran();
    let target = root.join("target").to_string_lossy().into_owned();
    let base_target = root.join(BASE_TARGET_DIR).to_string_lossy().into_owned();
    let base_root = root.join(BASE_DIR).to_string_lossy().into_owned();
    assert_eq!(CONFIGURATIONS.len(), API_CRATES.len());
    for &(krate, features) in CONFIGURATIONS {
        let flags = features.iter().fold(String::new(), |mut flags, f| {
            let _ = write!(flags, " --features {f}");
            flags
        });
        let listing = format!(" --no-default-features{flags}");
        let semver = format!(" --only-explicit-features{flags}");
        for command in [
            format!(
                "head: cargo +{NIGHTLY} public-api -p {krate} --simplified{listing} \
                 --target-dir {target}"
            ),
            format!(
                "base: cargo +{NIGHTLY} public-api -p {krate} --simplified{listing} \
                 --target-dir {base_target}"
            ),
        ] {
            assert!(ran.contains(&command), "no `{command}` in {ran:#?}");
        }
        let semver_command = format!(
            "head: cargo semver-checks check-release -p {krate} --baseline-root {base_root} \
             --release-type minor --color never{semver}"
        );
        // An unchecked crate is listed on both sides and never semver-checked.
        let checked = unchecked(krate).is_none();
        assert_eq!(
            ran.contains(&semver_command),
            checked,
            "`{semver_command}` in {ran:#?}"
        );
        if !checked {
            let prefix = format!("head: cargo semver-checks check-release -p {krate} ");
            assert!(!ran.iter().any(|c| c.starts_with(&prefix)), "{ran:#?}");
        }
    }
    let _ = fs::remove_dir_all(&root);
}

/// Each crate's chosen and left-out features are, together, every feature
/// its manifest has but `default`: a feature added later is chosen or left
/// out on purpose, never skipped by one tool and turned on by the other.
#[test]
fn every_feature_of_each_crates_manifest_is_chosen_or_left_out() {
    let crates = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut manifests = std::collections::HashMap::new();
    for entry in fs::read_dir(&crates).unwrap() {
        let path = entry.unwrap().path().join("Cargo.toml");
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        let manifest: toml::Table = toml::from_str(&text).unwrap();
        let name = manifest["package"]["name"].as_str().unwrap().to_string();
        manifests.insert(name, manifest);
    }
    for krate in API_CRATES {
        let empty = toml::Table::new();
        let table = manifests[krate.name]
            .get("features")
            .and_then(toml::Value::as_table)
            .unwrap_or(&empty);
        let mut declared: Vec<&str> = table
            .keys()
            .map(String::as_str)
            .filter(|name| *name != "default")
            .collect();
        declared.sort_unstable();
        let Features::Only(chosen) = krate.features;
        // A chosen feature, what it turns on of this crate's, and so on.
        let mut covered: Vec<&str> = chosen.to_vec();
        let mut next = 0;
        while let Some(feature) = covered.get(next).copied() {
            next += 1;
            let turned_on = table[feature].as_array().unwrap();
            for name in turned_on.iter().filter_map(toml::Value::as_str) {
                if table.contains_key(name) && !covered.contains(&name) {
                    covered.push(name);
                }
            }
        }
        covered.extend(
            LEFT_OUT
                .iter()
                .filter(|(name, _)| *name == krate.name)
                .map(|(_, feature)| *feature),
        );
        covered.sort_unstable();
        assert_eq!(covered, declared, "{}", krate.name);
    }
}

/// `UNCHECKED` is exactly the workspace's proc-macro crates, each in the
/// catalog with a reason (ticket 3506): `cargo-semver-checks` reads no
/// other kind of target, and refuses a proc-macro.
#[test]
fn the_unchecked_crates_are_the_proc_macro_crates() {
    let crates = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut proc_macros = Vec::new();
    for entry in fs::read_dir(&crates).unwrap() {
        let path = entry.unwrap().path().join("Cargo.toml");
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        let manifest: toml::Table = toml::from_str(&text).unwrap();
        let proc_macro = manifest
            .get("lib")
            .and_then(|lib| lib.get("proc-macro"))
            .and_then(toml::Value::as_bool)
            .unwrap_or(false);
        if proc_macro {
            proc_macros.push(manifest["package"]["name"].as_str().unwrap().to_string());
        }
    }
    proc_macros.sort_unstable();
    let mut listed: Vec<String> = UNCHECKED
        .iter()
        .map(|(name, _)| (*name).to_string())
        .collect();
    listed.sort_unstable();
    assert_eq!(listed, proc_macros);
    for (name, why) in UNCHECKED {
        assert!(catalog_names().contains(name), "{name}");
        assert!(!why.is_empty(), "{name}");
    }
}

/// A run in which the unchecked crate lost a line of its listing and gained
/// one, and every other crate is as in the base.
fn unchecked_run(root: &Path) -> (Fake, &'static str) {
    let (krate, _) = UNCHECKED[0];
    let fake = Fake::new(root)
        .with_tools()
        .with_base("abc123")
        .with_base_packages(&catalog_names())
        .with_listing("base", krate, "pub mod m\npub macro m::a!\n")
        .with_listing("head", krate, "pub mod m\npub macro m::b!\n");
    (fake, krate)
}

#[test]
fn an_unchecked_crate_is_listed_and_its_lost_line_is_a_break() {
    let root = temp_root("unchecked");
    let (fake, krate) = unchecked_run(&root);
    let outcome = run(&root, &Options::default(), &fake);
    assert!(outcome.errors.is_empty(), "{:?}", outcome.errors);
    assert!(outcome.warnings.is_empty(), "{:?}", outcome.warnings);
    assert_eq!(outcome.label, Some(Label::Breaking));
    let why = unchecked(krate).unwrap();
    assert!(
        outcome
            .summary
            .contains(&format!("| `{krate}` | 1 | 1 | 1, not run: {why} |")),
        "{}",
        outcome.summary
    );
    assert!(
        outcome
            .summary
            .contains(&format!("- `{LISTING_LINE_REMOVED}`: pub macro m::a!")),
        "{}",
        outcome.summary
    );
    let ran = fake.ran();
    for place in ["head", "base"] {
        let listed = format!("{place}: cargo +{NIGHTLY} public-api -p {krate} --simplified");
        assert!(ran.iter().any(|c| c.starts_with(&listed)), "{ran:#?}");
    }
    assert!(
        !ran.iter()
            .any(|c| c.contains(&format!("check-release -p {krate} "))),
        "{ran:#?}"
    );
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn an_unchecked_crates_lost_line_needs_its_breaking_line() {
    let root = temp_root("unchecked-rule");
    let (krate, _) = UNCHECKED[0];
    let options = Options {
        base: Some("abc123".to_string()),
        pr_branch: Some("t2100-a".to_string()),
        ..Options::default()
    };
    fs::write(root.join("tickets/done/2100-a.md"), TICKET_2100).unwrap();
    let (fake, _) = unchecked_run(&root);
    let outcome = run(&root, &options, &fake);
    assert!(
        outcome
            .errors
            .iter()
            .any(|e| e.contains(&format!("no `Breaking:` line naming {krate}"))),
        "{:?}",
        outcome.errors
    );
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn an_unchecked_crates_added_line_is_an_addition() {
    let base = lines(&["pub mod m"]);
    let head = lines(&["pub mod m", "pub macro m::b!"]);
    assert_eq!(unchecked_findings(&base, &head), Vec::new());
    assert_eq!(
        unchecked_findings(&head, &base),
        vec![finding(LISTING_LINE_REMOVED, &["pub macro m::b!"])]
    );
    assert_eq!(unchecked("shuyi-core"), None);
}

#[test]
fn an_added_crate_missing_from_the_base_is_added() {
    for (krate, item, listing, _) in ADDED_CRATES {
        let root = temp_root(&format!("new-{krate}"));
        let in_base: Vec<&str> = catalog_names()
            .into_iter()
            .filter(|name| *name != krate)
            .collect();
        let fake = Fake::new(&root)
            .with_tools()
            .with_base("abc123")
            .with_base_packages(&in_base)
            .with_listing("head", krate, &format!("pub fn {item}()\n"));
        let outcome = run(&root, &Options::default(), &fake);
        assert!(outcome.errors.is_empty(), "{:?}", outcome.errors);
        assert_eq!(outcome.label, Some(Label::Added));
        assert!(
            outcome
                .summary
                .contains(&format!("| `{krate}` (new) | 0 | 1 | not checked |")),
            "{}",
            outcome.summary
        );
        let ran = fake.ran();
        assert!(ran.iter().any(|c| {
            c.starts_with("head: cargo +")
                && c.contains(&format!("-p {krate} --simplified{listing} "))
        }));
        assert!(
            !ran.iter()
                .any(|c| c.starts_with("base: cargo +") && c.contains(&format!("-p {krate} ")))
        );
        assert!(
            !ran.iter()
                .any(|c| c.contains(&format!("check-release -p {krate} ")))
        );
        let _ = fs::remove_dir_all(&root);
    }
}

#[test]
fn a_break_in_an_added_crate_needs_its_line_and_the_books() {
    for (krate, item, _, _) in ADDED_CRATES {
        let root = temp_root(&format!("break-{krate}"));
        let options = Options {
            base: Some("abc123".to_string()),
            pr_branch: Some("t2100-a".to_string()),
            ..Options::default()
        };
        fs::write(
            root.join("tickets/done/2100-a.md"),
            "## Completion notes\n\nDone.\n",
        )
        .unwrap();
        let outcome = run(&root, &options, &break_in(&root, krate, item));
        assert_eq!(outcome.label, Some(Label::Breaking));
        assert_eq!(outcome.errors.len(), 2, "{:?}", outcome.errors);
        assert!(
            outcome.errors[0].contains(&format!("no `Breaking:` line naming {krate}")),
            "{:?}",
            outcome.errors
        );
        assert!(outcome.errors[0].contains(item), "{:?}", outcome.errors);
        assert!(
            outcome.errors[1].contains(&format!("The public API of {krate} changed")),
            "{:?}",
            outcome.errors
        );

        fs::write(
            root.join("tickets/done/2100-a.md"),
            format!(
                "## Completion notes\n\nBreaking: {krate}: `{item}`: call another.\n\
                 Book: none, the book doesn't name it.\n"
            ),
        )
        .unwrap();
        let outcome = run(&root, &options, &break_in(&root, krate, item));
        assert!(outcome.errors.is_empty(), "{:?}", outcome.errors);
        assert_eq!(outcome.label, Some(Label::Breaking));
        let _ = fs::remove_dir_all(&root);
    }
}

#[test]
fn an_addition_to_an_added_crate_needs_the_book() {
    for (krate, item, _, _) in ADDED_CRATES {
        let root = temp_root(&format!("addition-{krate}"));
        fs::write(
            root.join("tickets/done/2100-a.md"),
            "## Completion notes\n\nDone.\n",
        )
        .unwrap();
        let options = Options {
            base: Some("abc123".to_string()),
            pr_branch: Some("t2100-a".to_string()),
            ..Options::default()
        };
        let fake = Fake::new(&root)
            .with_tools()
            .with_base("abc123")
            .with_base_packages(&catalog_names())
            .with_listing("head", krate, &format!("pub fn {item}()\n"));
        let outcome = run(&root, &options, &fake);
        assert_eq!(outcome.label, Some(Label::Added));
        assert_eq!(outcome.errors.len(), 1, "{:?}", outcome.errors);
        assert!(
            outcome.errors[0].contains("have no `Book:` line"),
            "{:?}",
            outcome.errors
        );
        let _ = fs::remove_dir_all(&root);
    }
}

#[test]
fn a_failed_tool_on_an_added_crate_is_an_error_never_none() {
    for (krate, _, _, _) in ADDED_CRATES {
        for key in [
            format!("base: cargo +{NIGHTLY} public-api -p {krate} "),
            format!("head: cargo +{NIGHTLY} public-api -p {krate} "),
            format!("head: cargo semver-checks check-release -p {krate} "),
        ] {
            let root = temp_root(&format!("tool-{krate}"));
            let fake = Fake::new(&root)
                .with_tools()
                .with_base("abc123")
                .with_base_packages(&catalog_names())
                .answer(&key, Ok(failed("error: the package has no such feature")));
            let outcome = run(&root, &Options::default(), &fake);
            assert_eq!(outcome.errors.len(), 1, "{key}: {:?}", outcome.errors);
            assert!(
                outcome.errors[0].contains("error: the package has no such feature"),
                "{:?}",
                outcome.errors
            );
            assert_eq!(outcome.label, None);
            assert_eq!(outcome.summary, "");
            let _ = fs::remove_dir_all(&root);
        }
    }
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
    assert_ne!(failed.stderr, "");
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

#[test]
fn a_summary_is_printed_and_appended_to_its_file() {
    let root = temp_root("summary");
    let file = root.join("summary.md");
    fs::write(&file, "before\n").unwrap();
    let mut out = Vec::new();
    assert_eq!(write_summary("## Public API", Some(&file), &mut out), None);
    assert_eq!(write_summary("again", Some(&file), &mut out), None);
    assert_eq!(
        fs::read_to_string(&file).unwrap(),
        "before\n## Public API\nagain\n"
    );
    assert_eq!(String::from_utf8(out).unwrap(), "## Public API\nagain\n");
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn a_summary_without_a_file_is_only_printed() {
    let mut out = Vec::new();
    assert_eq!(write_summary("## Public API", None, &mut out), None);
    assert_eq!(String::from_utf8(out).unwrap(), "## Public API\n");
}

#[test]
fn an_empty_summary_writes_nothing() {
    let root = temp_root("empty-summary");
    let file = root.join("summary.md");
    let mut out = Vec::new();
    assert_eq!(write_summary("", Some(&file), &mut out), None);
    assert_eq!(out, Vec::<u8>::new());
    assert!(!file.exists());
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn a_summary_file_that_cannot_be_written_is_an_error() {
    let root = temp_root("bad-summary");
    // A folder can't be opened as a file to append to.
    let error = write_summary("## Public API", Some(&root), &mut Vec::new()).unwrap();
    assert!(error.starts_with(&root.display().to_string()), "{error}");
    let _ = fs::remove_dir_all(&root);
}

// -- outside crates (ticket 2089, ADR-0119) --

/// No line found.
const NONE: [(String, String); 0] = [];

fn outside_in(krate: &str, line: &str, deps: &[&str]) -> Vec<(String, String)> {
    let deps: Vec<String> = deps.iter().map(|dep| (*dep).to_string()).collect();
    unlisted_outside(krate, &lines(&[line]), &deps)
}

#[test]
fn an_unlisted_outside_crate_is_found() {
    assert_eq!(
        outside_in(
            "shuyi-app",
            "pub fn shuyi_app::x(gilrs::GamepadId)",
            &["gilrs"]
        ),
        vec![(
            "pub fn shuyi_app::x(gilrs::GamepadId)".to_string(),
            "gilrs".to_string()
        )]
    );
}

#[test]
fn serde_core_is_allowed_everywhere() {
    let line = "impl serde_core::de::Deserialize<'de> for shuyi_core::Pos";
    assert_eq!(
        outside_in("shuyi-core", line, &["serde_core", "serde"]),
        NONE
    );
}

#[test]
fn serde_json_is_allowed_in_the_cli_alone() {
    let line = "pub fn shuyi_x::f() -> serde_json::value::Value";
    assert_eq!(outside_in("shuyi-cli", line, &["serde_json"]), NONE);
    assert_eq!(outside_in("shuyi-ui", line, &["serde_json"]).len(), 1);
}

#[test]
fn an_engine_module_named_like_a_crate_is_not_the_crate() {
    let line = "pub struct shuyi_assets::image::ImageTable";
    assert_eq!(outside_in("shuyi-assets", line, &["image"]), NONE);
    assert_eq!(
        outside_in("shuyi-assets", "pub fn my_image::f()", &["image"]),
        NONE
    );
}

#[test]
fn a_crate_at_the_start_of_a_path_is_found_anywhere_on_the_line() {
    for line in [
        "image::RgbaImage",
        "pub fn shuyi_assets::f(&image::RgbaImage)",
        "pub fn shuyi_assets::f() -> Vec<image::RgbaImage>",
        "pub fn shuyi_assets::f(shuyi_assets::image::X, image::Y)",
    ] {
        assert_eq!(
            outside_in("shuyi-assets", line, &["image"]).len(),
            1,
            "{line}"
        );
    }
}

#[test]
fn a_crate_that_is_no_dependency_is_not_looked_for() {
    assert_eq!(
        outside_in("shuyi-app", "pub fn shuyi_app::x(gilrs::GamepadId)", &[]),
        NONE
    );
}

#[test]
fn every_allowed_crate_names_engine_crates_of_the_catalog() {
    let catalog = catalog_names();
    for (path, crates) in OUTSIDE_CRATES {
        for krate in *crates {
            assert!(catalog.contains(krate), "{path}: {krate}");
        }
    }
}

#[test]
fn outside_dependencies_are_the_normal_ones_that_are_no_engine_crate() {
    let root = temp_root("outside-deps");
    let fake = Fake::new(&root).answer(
        "head: cargo metadata --no-deps --format-version 1",
        Ok(ok(&metadata_json_with(
            &["shuyi-app"],
            &[],
            &[(
                "shuyi-app",
                &["quad-snd", "gilrs", "gilrs", "shuyi-ui", "dev:tempfile"],
            )],
        ))),
    );
    let packages = packages(&root, &fake).unwrap();
    assert_eq!(packages["shuyi-app"].outside, lines(&["gilrs", "quad_snd"]));
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn a_run_fails_on_an_unlisted_outside_crate() {
    let root = temp_root("outside-run");
    let options = Options {
        base: Some("HEAD^1".to_string()),
        ..Options::default()
    };
    let fake = breaking_run(&root)
        .answer(
            "head: cargo metadata --no-deps --format-version 1",
            Ok(ok(&metadata_json_with(
                &catalog_names(),
                &[],
                &[("shuyi-app", &["gilrs"]), ("shuyi-ui", &["serde_json"])],
            ))),
        )
        .with_listing(
            "head",
            "shuyi-app",
            "pub mod shuyi_app\npub fn shuyi_app::x(gilrs::GamepadId)\n",
        )
        .with_listing(
            "head",
            "shuyi-ui",
            "pub fn shuyi_ui::f() -> serde_json::value::Value\n",
        )
        // The base's lines don't count.
        .with_listing(
            "base",
            "shuyi-core",
            "pub mod shuyi_core\npub fn shuyi_core::state_hash(gilrs::GamepadId)\n",
        );
    let outcome = run(&root, &options, &fake);
    assert_eq!(outcome.errors.len(), 2, "{:?}", outcome.errors);
    // In the catalog's order: `shuyi-ui` before `shuyi-app`.
    assert!(
        outcome.errors[0].starts_with("shuyi-ui's"),
        "{:?}",
        outcome.errors
    );
    let app = &outcome.errors[1];
    assert!(app.starts_with("shuyi-app's public API"), "{app}");
    assert!(
        app.contains("pub fn shuyi_app::x(gilrs::GamepadId)"),
        "{app}"
    );
    assert!(app.contains("names `gilrs`"), "{app}");
    assert!(
        app.contains("ADR-0119 lists the outside crates the public API may name"),
        "{app}"
    );
    assert!(
        outcome.summary.contains(
            "**Outside crate, fail:** `shuyi-app` names `gilrs`, which ADR-0119 doesn't allow \
             there: `pub fn shuyi_app::x(gilrs::GamepadId)`"
        ),
        "{}",
        outcome.summary
    );
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn a_run_passes_an_allowed_outside_crate() {
    let root = temp_root("outside-allowed");
    let options = Options {
        base: Some("HEAD^1".to_string()),
        ..Options::default()
    };
    let fake = breaking_run(&root)
        .answer(
            "head: cargo metadata --no-deps --format-version 1",
            Ok(ok(&metadata_json_with(
                &catalog_names(),
                &[],
                &[("shuyi-cli", &["serde_json", "semver"])],
            ))),
        )
        .with_listing(
            "head",
            "shuyi-cli",
            "pub mod shuyi_cli\npub use shuyi_cli::serde_json\n\
             pub fn shuyi_cli::f(serde_json::value::Value) -> semver::Version\n",
        );
    let outcome = run(&root, &options, &fake);
    assert!(outcome.errors.is_empty(), "{:?}", outcome.errors);
    assert!(
        !outcome.summary.contains("Outside crate"),
        "{}",
        outcome.summary
    );
    let _ = fs::remove_dir_all(&root);
}

// -- crates the dependencies reach (ticket 2090) --

#[test]
fn a_crate_reaches_its_dependencies_dependencies_but_not_through_an_engine_crate() {
    let metadata: serde_json::Value = serde_json::from_str(&resolve_json(
        &["shuyi-app", "shuyi-ui"],
        &[
            ("shuyi-app", &["macroquad", "shuyi-ui", "dev:tempfile"]),
            ("macroquad", &["miniquad", "quad-snd"]),
            ("miniquad", &["libc"]),
            ("quad-snd", &["libc"]),
            ("shuyi-ui", &["serde_json"]),
            ("tempfile", &["fastrand"]),
        ],
    ))
    .unwrap();
    let reached = reached_outside(&metadata);
    assert_eq!(
        reached["shuyi-app"],
        lines(&["libc", "macroquad", "miniquad", "quad_snd"])
    );
    assert_eq!(reached["shuyi-ui"], lines(&["serde_json"]));
    // Only the workspace's members.
    assert_eq!(reached.len(), 2);
}

#[test]
fn a_crate_reached_also_through_an_outside_crate_counts() {
    let metadata: serde_json::Value = serde_json::from_str(&resolve_json(
        &["shuyi-app", "shuyi-ui"],
        &[
            ("shuyi-app", &["shuyi-ui", "image"]),
            ("shuyi-ui", &["png"]),
            ("image", &["png"]),
        ],
    ))
    .unwrap();
    assert_eq!(
        reached_outside(&metadata)["shuyi-app"],
        lines(&["image", "png"])
    );
}

#[test]
fn a_library_is_named_by_its_targets_name() {
    let metadata = serde_json::json!({
        "packages": [
            {"id": "a", "name": "shuyi-app", "targets": [{"kind": ["lib"], "name": "shuyi_app"}]},
            {"id": "b", "name": "some-crate", "targets": [
                {"kind": ["custom-build"], "name": "build-script-build"},
                {"kind": ["rlib"], "name": "other_name"},
            ]},
            {"id": "c", "name": "no-lib", "targets": []},
        ],
        "workspace_members": ["a"],
        "resolve": {"nodes": [
            {"id": "a", "deps": [
                {"pkg": "b", "dep_kinds": [{"kind": "build"}, {"kind": null}]},
                {"pkg": "c", "dep_kinds": [{"kind": null, "target": "cfg(windows)"}]},
            ]},
        ]},
    });
    assert_eq!(
        reached_outside(&metadata)["shuyi-app"],
        lines(&["no_lib", "other_name"])
    );
}

#[test]
fn a_run_fails_on_a_crate_reached_through_a_dependency() {
    let root = temp_root("outside-reached");
    let options = Options {
        base: Some("HEAD^1".to_string()),
        ..Options::default()
    };
    let fake = breaking_run(&root)
        .answer(
            "head: cargo metadata --no-deps --format-version 1",
            Ok(ok(&metadata_json_with(
                &catalog_names(),
                &[],
                &[("shuyi-app", &["macroquad", "shuyi-ui"])],
            ))),
        )
        .answer(
            RESOLVE,
            Ok(ok(&resolve_json(
                &catalog_names(),
                &[
                    ("shuyi-app", &["macroquad", "shuyi-ui"]),
                    ("macroquad", &["miniquad"]),
                    ("shuyi-ui", &["gilrs"]),
                ],
            ))),
        )
        .with_listing(
            "head",
            "shuyi-app",
            "pub mod shuyi_app
pub fn shuyi_app::f() -> miniquad::conf::Conf
             pub fn shuyi_app::g(gilrs::GamepadId)
",
        );
    let outcome = run(&root, &options, &fake);
    assert_eq!(outcome.errors.len(), 1, "{:?}", outcome.errors);
    let app = &outcome.errors[0];
    assert!(app.starts_with("shuyi-app's public API"), "{app}");
    assert!(
        app.contains(
            "pub fn shuyi_app::f() -> miniquad::conf::Conf
    names `miniquad`"
        ),
        "{app}"
    );
    assert!(app.contains("ADR-0119"), "{app}");
    // `gilrs` is reached only through `shuyi-ui`, an engine crate.
    assert!(!app.contains("gilrs"), "{app}");
    assert!(fake.ran().iter().any(|command| command == RESOLVE));
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn unreadable_metadata_with_dependencies_is_an_error() {
    let root = temp_root("resolve");
    let fake = Fake::new(&root)
        .with_tools()
        .with_base("abc123")
        .answer(RESOLVE, Ok(ok("not json")));
    let options = Options {
        base: Some("abc123".to_string()),
        ..Options::default()
    };
    let outcome = run(&root, &options, &fake);
    assert_eq!(outcome.errors.len(), 1, "{:?}", outcome.errors);
    assert!(
        outcome.errors[0].starts_with("`cargo metadata` with dependencies in "),
        "{:?}",
        outcome.errors
    );
    let _ = fs::remove_dir_all(&root);
}

// -- constructors that take two neighbouring numbers (ticket 3503) --

/// A run in which the head's `shuyi-assets` has `MusicSwitch::new(u32, u32,
/// bool)` on a `#[non_exhaustive]` type, as the base has, and an exhaustive
/// type's constructor of two numbers.
fn constructor_run(root: &Path) -> Fake {
    let listing = "pub mod shuyi_assets\n\
                   #[non_exhaustive] pub struct shuyi_assets::audio::MusicSwitch\n\
                   pub const fn shuyi_assets::audio::MusicSwitch::new(u32, u32, bool) -> Self\n\
                   pub const fn shuyi_assets::audio::MusicSwitch::with_overlap(self, bool) -> Self\n\
                   pub struct shuyi_assets::ImageRect\n\
                   pub const fn shuyi_assets::ImageRect::new(u32, u32, u32, u32) -> Self\n";
    Fake::new(root)
        .with_tools()
        .with_base("abc123")
        .with_base_packages(&catalog_names())
        .with_listing("base", "shuyi-assets", listing)
        .with_listing("head", "shuyi-assets", listing)
}

#[test]
fn a_run_with_a_constructor_of_two_neighbouring_numbers_fails_and_names_it() {
    let root = temp_root("constructors");
    let outcome = run(&root, &Options::default(), &constructor_run(&root));
    assert_eq!(outcome.errors.len(), 1, "{:?}", outcome.errors);
    let error = &outcome.errors[0];
    assert!(
        error.contains("`shuyi_assets::audio::MusicSwitch::new` takes two neighbouring integers"),
        "{error}"
    );
    assert!(!error.contains("ImageRect"), "{error}");
    assert!(
        outcome.summary.contains(
            "**Constructors, fail:**\n\n- shuyi-assets: `shuyi_assets::audio::MusicSwitch::new`"
        ),
        "{}",
        outcome.summary
    );
    // The listing didn't change: this is no break, and the label says so.
    assert_eq!(outcome.label, Some(Label::None));
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn a_run_passes_once_the_file_lists_the_constructor() {
    let root = temp_root("constructors-listed");
    fs::create_dir_all(root.join("tools")).unwrap();
    fs::write(
        root.join(constructors::FILE),
        "(allowed: [(path: \"shuyi_assets::audio::MusicSwitch::new\", why: \"a fade out, \
         then a fade in\")])",
    )
    .unwrap();
    let outcome = run(&root, &Options::default(), &constructor_run(&root));
    assert!(outcome.errors.is_empty(), "{:?}", outcome.errors);
    assert!(
        !outcome.summary.contains("Constructors, fail"),
        "{}",
        outcome.summary
    );
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn a_run_fails_a_stale_entry() {
    let root = temp_root("constructors-stale");
    fs::create_dir_all(root.join("tools")).unwrap();
    fs::write(
        root.join(constructors::FILE),
        "(allowed: [\n\
         (path: \"shuyi_assets::audio::MusicSwitch::new\", why: \"fades\"),\n\
         (path: \"shuyi_assets::ImageRect::new\", why: \"exhaustive: never a constructor here\"),\n\
         ])",
    )
    .unwrap();
    let outcome = run(&root, &Options::default(), &constructor_run(&root));
    assert_eq!(outcome.errors.len(), 1, "{:?}", outcome.errors);
    assert!(
        outcome.errors[0].contains("`shuyi_assets::ImageRect::new` is listed in"),
        "{:?}",
        outcome.errors
    );
    let _ = fs::remove_dir_all(&root);
}
