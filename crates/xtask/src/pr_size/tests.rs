use super::*;

/// A diff with a file of every kind, as `git diff --numstat -z` prints it.
const FIXTURE_DIFF: &str = "\
    120\t30\tcrates/ui/src/frame.rs\0\
    40\t0\tcrates/ui/tests/frame.rs\0\
    10\t0\tcrates/assets/src/golden/tests.rs\0\
    25\t5\tdocs/adr/0120-x.md\0\
    8\t0\tsite/snippets/hello.rs\0\
    6\t2\texamples/hello-grid/files/levels/one.ron\0\
    300\t0\tcrates/ui/src/frame/snapshots/frame__a.snap\0\
    50\t50\tcrates/ui/tests/golden/onebit.ron\0\
    70\t0\tcrates/cli/schemas/run.output.json\0\
    5\t0\ttools/x.schema.json\0\
    400\t200\tCargo.lock\0\
    9\t9\tsite/tutorial/01-new/Cargo.lock\0\
    12\t0\t.agents/skills/run-gates/SKILL.md\0\
    700\t0\texamples/hello-grid/files/font/atlas.ron\0\
    20\t0\texamples/hello-grid/src/app_info.rs\0\
    60\t1\ttickets/open/2999-x.md\0\
    -\t-\texamples/hello-grid/preview.png\0\
    3\t1\t\0crates/ui/src/old.rs\0crates/ui/src/new.rs\0";

fn fixture() -> Size {
    Size::of(numstat(FIXTURE_DIFF))
}

#[test]
fn numstat_reads_plain_renamed_and_binary_files() {
    let files = numstat(FIXTURE_DIFF);
    assert_eq!(files.len(), 17, "{files:?}");
    assert_eq!(files[0], ("crates/ui/src/frame.rs".to_owned(), 150));
    assert_eq!(files[16], ("crates/ui/src/new.rs".to_owned(), 4));
    assert!(
        !files
            .iter()
            .any(|(p, _)| p == "examples/hello-grid/preview.png")
    );
    assert_eq!(numstat(""), []);
}

#[test]
fn each_file_is_its_kind() {
    let size = fixture();
    let kinds: Vec<(&str, Kind)> = size
        .files
        .iter()
        .map(|(p, _, k)| (p.as_str(), *k))
        .collect();
    let left = Kind::LeftOut;
    assert_eq!(
        kinds,
        [
            ("crates/ui/src/frame.rs", Kind::Code),
            ("crates/ui/tests/frame.rs", Kind::Test),
            ("crates/assets/src/golden/tests.rs", Kind::Test),
            ("docs/adr/0120-x.md", Kind::Docs),
            ("site/snippets/hello.rs", Kind::Docs),
            ("examples/hello-grid/files/levels/one.ron", Kind::Data),
            (
                "crates/ui/src/frame/snapshots/frame__a.snap",
                left(LeftOut::Snapshot)
            ),
            ("crates/ui/tests/golden/onebit.ron", left(LeftOut::Golden)),
            ("crates/cli/schemas/run.output.json", left(LeftOut::Schema)),
            ("tools/x.schema.json", left(LeftOut::Schema)),
            ("Cargo.lock", left(LeftOut::Lock)),
            ("site/tutorial/01-new/Cargo.lock", left(LeftOut::Lock)),
            (
                ".agents/skills/run-gates/SKILL.md",
                left(LeftOut::Generated)
            ),
            (
                "examples/hello-grid/files/font/atlas.ron",
                left(LeftOut::Generated)
            ),
            (
                "examples/hello-grid/src/app_info.rs",
                left(LeftOut::Generated)
            ),
            ("tickets/open/2999-x.md", Kind::Tickets),
            ("crates/ui/src/new.rs", Kind::Code),
        ]
    );
    // The engine's own `app_info` modules are written by hand.
    assert_eq!(classify("crates/cli/src/app_info.rs"), Kind::Code);
    assert_eq!(classify("crates/xtask/src/golden.rs"), Kind::Code);
}

#[test]
fn the_totals_count_apart_what_is_left_out_and_the_tickets() {
    let size = fixture();
    // 150 + 4 code, 40 + 10 tests, 30 + 8 docs, 8 data.
    assert_eq!(size.counted(), 250);
    // 300 + 100 + 70 + 5 + 600 + 18 + 12 + 700 + 20.
    assert_eq!(size.left_out(), 1_825);
    let report = size.report("abc");
    assert_eq!(
        report,
        "pr-size: 250 counted lines (added + removed), abc to the working tree\n\
         \x20 counted: code 154, tests 50, docs 38, data 8\n\
         \x20 left out: 1,825 lines (snapshots 300, golden tables 100, schemas 75, \
         lock files 618, generated 732), and binary files\n\
         \x20 tickets: 61 lines, counted apart\n\
         \x20   150  crates/ui/src/frame.rs (code)\n\
         \x20    40  crates/ui/tests/frame.rs (tests)\n\
         \x20    30  docs/adr/0120-x.md (docs)\n\
         \x20    10  crates/assets/src/golden/tests.rs (tests)\n\
         \x20     8  examples/hello-grid/files/levels/one.ron (data)\n"
    );
}

#[test]
fn untracked_lines_count_a_last_line_without_a_newline() {
    assert_eq!(untracked_lines(b"a\nb\n"), Some(2));
    assert_eq!(untracked_lines(b"a\nb"), Some(2));
    assert_eq!(untracked_lines(b""), Some(0));
    assert_eq!(untracked_lines(b"PNG\0\x01"), None);
}

#[test]
fn thousands_puts_a_comma_every_three_digits() {
    assert_eq!(thousands(0), "0");
    assert_eq!(thousands(999), "999");
    assert_eq!(thousands(1_000), "1,000");
    assert_eq!(thousands(1_234_567), "1,234,567");
}

#[test]
fn verdicts_follow_the_warning_the_limit_and_the_kind() {
    let work = PrKind::Work("2053".to_owned());
    assert_eq!(Verdict::of(1_000, Some(&work)), Verdict::Fine);
    assert_eq!(Verdict::of(1_001, Some(&work)), Verdict::Warn);
    assert_eq!(Verdict::of(1_500, None), Verdict::Warn);
    assert_eq!(Verdict::of(1_501, None), Verdict::OverTheLimit);
    for exempt in [
        PrKind::Bot,
        PrKind::Sonar,
        PrKind::TicketsOnly(vec!["2053".to_owned()]),
    ] {
        assert_eq!(Verdict::of(9_000, Some(&exempt)), Verdict::Exempt);
    }
    assert_eq!(
        Verdict::of(9_000, Some(&PrKind::GameSmall("1200".to_owned()))),
        Verdict::OverTheLimit
    );
}

fn ticket_with(notes: &str) -> TicketSource {
    TicketSource::File(
        "tickets/done/2053-x.md".to_owned(),
        format!("# 2053\n\n## Completion notes\n\n{notes}\n"),
    )
}

#[test]
fn over_the_limit_a_ticket_needs_a_size_line_with_the_number() {
    assert_eq!(size_problem(1_500, &ticket_with("")), None);
    let missing = size_problem(1_820, &ticket_with("Done.")).unwrap();
    assert!(missing.contains("1,820 counted lines"), "{missing}");
    assert!(missing.contains("has no `Size:` line"), "{missing}");

    let line = "Size: 1,820 counted lines, the parser and its fixtures are one change.";
    assert_eq!(size_problem(1_820, &ticket_with(line)), None);
    // Within a tenth: a merge or the line itself moves the count.
    assert_eq!(size_problem(2_000, &ticket_with(line)), None);
    let stale = size_problem(2_100, &ticket_with(line)).unwrap();
    assert!(stale.contains("says 1,820"), "{stale}");

    let no_reason = size_problem(1_820, &ticket_with("Size: 1,820 lines.")).unwrap();
    assert!(no_reason.contains("says why"), "{no_reason}");
    assert_eq!(size_problem(9_000, &TicketSource::Exempt), None);
    let none = size_problem(1_820, &TicketSource::Missing("no ticket".to_owned())).unwrap();
    assert!(none.contains("no ticket"), "{none}");
}

#[test]
fn ci_fails_what_a_local_run_only_warns_about() {
    let size = |lines| Size::of(vec![("crates/a/src/lib.rs".to_owned(), lines)]);
    let work = PrKind::Work("2053".to_owned());
    let ticket = ticket_with("Done.");
    let (errors, warnings) = judge(&size(1_600), Some(&work), &ticket, true);
    assert_eq!((errors.len(), warnings.len()), (1, 0));
    let (errors, warnings) = judge(&size(1_600), Some(&work), &ticket, false);
    assert_eq!((errors.len(), warnings.len()), (0, 1));
    let (errors, warnings) = judge(&size(1_200), Some(&work), &ticket, true);
    assert_eq!(errors, Vec::<String>::new());
    assert!(warnings[0].contains("1,200 counted lines"), "{warnings:?}");
    assert_eq!(
        judge(&size(900), Some(&work), &ticket, true),
        (vec![], vec![])
    );
    // Lines left out never make the size.
    let snapshots = Size::of(vec![("a/snapshots/x.snap".to_owned(), 9_000)]);
    assert_eq!(
        judge(&snapshots, Some(&work), &ticket, true),
        (vec![], vec![])
    );
}

#[test]
fn the_options_read_every_flag() {
    let args = |list: &[&str]| list.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>();
    assert_eq!(Options::parse(&[]), Some(Options::default()));
    assert_eq!(
        Options::parse(&args(&[
            "--base",
            "HEAD^1",
            "--hook",
            "--ticket",
            "t.md",
            "--pr-branch",
            "b",
            "--pr-title",
            "[2053] x",
        ])),
        Some(Options {
            base: Some("HEAD^1".to_owned()),
            hook: true,
            ticket: Some(PathBuf::from("t.md")),
            pr_branch: Some("b".to_owned()),
            pr_title: Some("[2053] x".to_owned()),
        })
    );
    assert_eq!(Options::parse(&args(&["--base"])), None);
    assert_eq!(Options::parse(&args(&["--size"])), None);
}

#[test]
fn the_hook_answer_is_context_for_the_session() {
    let json: serde_json::Value = serde_json::from_str(&hook_json("a \"b\"\nc")).unwrap();
    assert_eq!(json["hookSpecificOutput"]["hookEventName"], "PreToolUse");
    assert_eq!(
        json["hookSpecificOutput"]["additionalContext"],
        "pr-size: a \"b\"\nc"
    );
}

#[test]
fn the_hook_input_names_the_directory_and_the_command() {
    let json = r#"{"cwd":"D:\\w\\t1","tool_input":{"command":"git push -u origin HEAD"}}"#;
    assert_eq!(
        hook_input(json),
        HookInput {
            cwd: Some(PathBuf::from(r"D:\w\t1")),
            command: "git push -u origin HEAD".to_owned(),
        }
    );
    assert_eq!(hook_input(r#"{"cwd":""}"#), HookInput::default());
    assert_eq!(hook_input("not json"), HookInput::default());
}

#[test]
fn only_a_commit_a_push_or_a_new_pr_is_a_pr_step() {
    for command in [
        "git commit -m x",
        "cd /d/w && git commit -F msg.txt",
        "git add . ; git push",
        "gh pr create --title t",
    ] {
        assert!(runs_a_pr_step(command), "{command}");
    }
    for command in ["", "git status", "git commit-tree abc", "gh pr view 274"] {
        assert!(!runs_a_pr_step(command), "{command}");
    }
}

/// A throwaway repository on a `t2999-x` branch: an edit, a rename, a
/// snapshot, a binary file and untracked files, against the first commit;
/// then the same PR over the limit, before and after its ticket's `Size:`
/// line, through the hook and a CI run.
#[test]
fn a_real_diff_is_measured_and_judged() {
    let root = std::env::temp_dir().join(format!("shuyi-pr-size-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("tickets/done")).unwrap();
    let write = |path: &str, text: &str| std::fs::write(root.join(path), text).unwrap();
    let run_git = |args: &[&str]| git(&root, args).unwrap();
    run_git(&["init", "-q", "-b", "trunk"]);
    run_git(&["config", "user.email", "test@example.com"]);
    run_git(&["config", "user.name", "Test"]);
    write("edit.rs", "a\nb\n");
    write("moved.md", "one\ntwo\nthree\nfour\nfive\nsix\n");
    run_git(&["add", "."]);
    run_git(&["commit", "-q", "-m", "first"]);
    run_git(&["switch", "-q", "-c", "t2999-x"]);

    write("edit.rs", "a\nc\nd\n");
    run_git(&["mv", "moved.md", "renamed.md"]);
    write("a.snap", "generated\n");
    write("picture.bin", "\0\0");
    write("new.md", "x\ny");
    run_git(&["add", "a.snap", "edit.rs", "picture.bin"]);
    run_git(&["commit", "-q", "-m", "second"]);

    let (files, from) = changed_files(&root, "trunk").unwrap();
    let size = Size::of(files);
    assert_eq!(size.counted(), 5, "{:?}", size.files);
    assert_eq!(size.left_out(), 1);
    assert!(size.report(&from).starts_with("pr-size: 5 counted lines"));
    assert!(changed_files(&root, "no-such-ref-2053").is_err());

    // Over the limit: a local run and the hook warn, CI fails.
    write("big.rs", &"line\n".repeat(LIMIT));
    let local = run(
        &root,
        &Options {
            base: Some("trunk".to_owned()),
            ..Options::default()
        },
    );
    assert_eq!(local.errors, Vec::<String>::new());
    assert!(
        local.warnings[0].contains("1,505 counted lines"),
        "{local:?}"
    );
    let hook_in = |command: &str| {
        serde_json::json!({ "cwd": root, "tool_input": { "command": command } }).to_string()
    };
    let hook = hook_answer(&hook_in("git push"), Path::new("."), Some("trunk"));
    assert!(hook.contains("over the limit of 1,500"), "{hook}");
    assert_eq!(
        hook_answer(&hook_in("git status"), Path::new("."), Some("trunk")),
        ""
    );
    let ci = |title: &str| {
        run(
            &root,
            &Options {
                base: Some("trunk".to_owned()),
                pr_branch: Some("t2999-x".to_owned()),
                pr_title: Some(title.to_owned()),
                ..Options::default()
            },
        )
    };
    let failed = ci("[2999] X");
    assert!(
        failed.errors[0].contains("ticket 2999 is not in tickets/done/"),
        "{failed:?}"
    );
    write(
        "tickets/done/2999-x.md",
        "# 2999\n\n## Completion notes\n\nSize: 1,560 counted lines, one parser and \
         its cases.\n",
    );
    let passed = ci("[2999] X");
    assert_eq!(passed.errors, Vec::<String>::new(), "{passed:?}");
    assert_eq!(ci("Ticket 2999: x").errors, Vec::<String>::new());
    assert_eq!(
        hook_answer(&hook_in("git push"), Path::new("."), Some("trunk")),
        ""
    );
    std::fs::remove_dir_all(&root).unwrap();
}
