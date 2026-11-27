use std::path::PathBuf;

use super::*;

/// The repository's root, whose rules and tracked files the tests read.
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The repository's own rules.
fn real_rules() -> Vec<Rule> {
    let text = fs::read_to_string(root().join(RULES)).unwrap();
    parse(&text).unwrap()
}

fn rule(name: &str) -> Rule {
    real_rules()
        .into_iter()
        .find(|rule| rule.name == name)
        .unwrap_or_else(|| panic!("no rule `{name}`"))
}

fn files(paths: &[&str]) -> Vec<String> {
    paths.iter().map(|path| (*path).to_owned()).collect()
}

fn work() -> PrKind {
    PrKind::Work("2100".to_owned())
}

/// Ticket 2100, done, with `notes` in its completion notes.
fn done_with(notes: &'static str) -> impl Fn(&str) -> Option<String> {
    move |id| (id == "2100").then(|| format!("# 2100 — A\n\n## Completion notes\n\n{notes}\n"))
}

fn no_ticket(_: &str) -> Option<String> {
    None
}

// -- globs --

#[test]
fn a_double_star_spans_any_number_of_folders() {
    assert!(matches("skills/**", "skills/shuyi-new-game/SKILL.md"));
    assert!(matches("skills/**", "skills/README.md"));
    assert!(!matches("skills/**", ".claude/skills/work-ticket/SKILL.md"));
    let formats = "crates/assets/src/**/formats.rs";
    assert!(matches(formats, "crates/assets/src/audio/formats.rs"));
    assert!(matches(formats, "crates/assets/src/formats.rs"));
    assert!(!matches(formats, "crates/assets/src/format.rs"));
    assert!(!matches(formats, "crates/assets/src/audio/formats.rs.bak"));
}

#[test]
fn a_star_stays_in_its_folder() {
    let glob = "crates/cli/src/template/*.rs";
    assert!(matches(glob, "crates/cli/src/template/compose.rs"));
    assert!(!matches(glob, "crates/cli/src/template/ron_merge/tests.rs"));
    assert!(!matches(glob, "crates/cli/src/template/plans.golden"));
    assert!(matches(
        "templates/*/template.ron",
        "templates/shuyi-default/template.ron"
    ));
    assert!(matches("a/*x*y/b", "a/1x2y/b"));
    assert!(!matches("a/*x*y/b", "a/1y2x/b"));
    assert!(matches("docs/contract.md", "docs/contract.md"));
    assert!(!matches("docs/contract.md", "docs/contract.mdx"));
}

// -- the rules' file --

#[test]
fn the_rules_parse_and_every_glob_matches_a_tracked_file() {
    let rules = rules_of(&root()).unwrap_or_else(|errors| panic!("{errors:#?}"));
    assert!(rules.len() >= 4, "{rules:?}");
}

#[test]
fn the_tickets_first_rules_are_there() {
    for (name, trigger, home, mark) in [
        (
            "contract-skills",
            "crates/cli/schemas/**",
            "skills/**",
            "Skills:",
        ),
        (
            "contract-book",
            "crates/cli/schemas/**",
            "site/content/learn/**",
            "Book:",
        ),
        ("template-skills", "templates/**", "skills/**", "Skills:"),
        ("template-book", "templates/**", "site/tutorial/**", "Book:"),
        ("format-skills", "docs/formats.md", "skills/**", "Skills:"),
        (
            "problem-code-skills",
            "crates/cli/errors/**",
            "skills/**",
            "Skills:",
        ),
    ] {
        let rule = rule(name);
        assert!(rule.triggers.iter().any(|t| t == trigger), "{rule:?}");
        assert!(rule.homes.iter().any(|h| h == home), "{rule:?}");
        assert_eq!(rule.mark, mark, "{rule:?}");
    }
}

#[test]
fn a_format_registered_in_any_of_the_three_ways_asks_for_the_skill_pack() {
    let rule = rule("format-skills");
    for path in [
        "docs/formats.md",
        "crates/assets/src/format.rs",
        "crates/assets/src/audio/formats.rs",
        "crates/assets/src/image/formats.rs",
    ] {
        assert!(
            rule.triggers.iter().any(|glob| matches(glob, path)),
            "{path}"
        );
    }
}

#[test]
fn the_books_rules_name_the_folders_of_the_apis_book_rule() {
    for rule in real_rules().iter().filter(|rule| rule.mark == "Book:") {
        for dir in crate::api::BOOK_DIRS {
            assert!(
                rule.homes.iter().any(|glob| shown(glob) == *dir),
                "rule `{}` lacks {dir}",
                rule.name
            );
        }
    }
}

#[test]
fn a_glob_that_matches_no_tracked_file_fails() {
    let mut rules = vec![rule("contract-skills")];
    rules[0].homes.push("skils/**".to_owned());
    let tracked = files(&["crates/cli/schemas/new.output.json", "skills/README.md"]);
    let errors = unmatched_globs(&rules, &tracked);
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(
        errors[0].contains("rule `contract-skills`'s home `skils/**` matches no tracked file"),
        "{errors:?}"
    );
    assert_eq!(unmatched_globs(&rules[..0], &tracked), Vec::<String>::new());
}

#[test]
fn a_rules_file_that_is_wrong_fails_to_parse() {
    let one = |fields: &str| format!("(rules: [({fields})])");
    let good = r#"name: "a", changes: "A", triggers: ["x/**"], homes: ["y/**"], home: "y", part: "y", mark: "Y:""#;
    assert!(parse(&one(good)).is_ok());
    for (text, expected) in [
        (one(&format!("{good}, extra: 1")), "extra"),
        (
            one(&good.replace(r#"triggers: ["x/**"]"#, "triggers: []")),
            "needs a trigger and a home",
        ),
        (
            one(&good.replace(r#""Y:""#, r#""Y""#)),
            "is not a word and a colon",
        ),
        (
            one(&good.replace(r#""Y:""#, r#""Y: none:""#)),
            "is not a word and a colon",
        ),
        (
            format!("(rules: [({good}), ({good})])"),
            "two rules are named `a`",
        ),
    ] {
        let error = parse(&text).unwrap_err();
        assert!(error.contains(expected), "{error}");
    }
}

// -- a rule on a PR --

#[test]
fn a_schema_change_with_no_skill_and_no_line_fails_naming_the_line() {
    let changed = files(&[
        "crates/cli/schemas/new.output.json",
        "crates/cli/src/lib.rs",
    ]);
    let Verdict::Missing(why) = judge(
        &rule("contract-skills"),
        &work(),
        &changed,
        &done_with("Done."),
    ) else {
        panic!("not missing");
    };
    assert!(
        why.starts_with("ticket 2100's `## Completion notes` have no `Skills:` line.\n"),
        "{why}"
    );
    assert!(
        why.contains(
            "An operation or a field of the contract (crates/cli/schemas/new.output.json) \
             changed, and nothing in skills/ did."
        ),
        "{why}"
    );
    assert!(why.contains("the docs map (docs/README.md)"), "{why}");
    assert!(why.ends_with("\n\n    Skills: none, <why>"), "{why}");

    let Verdict::Missing(why) = judge(
        &rule("contract-book"),
        &work(),
        &changed,
        &done_with("Done."),
    ) else {
        panic!("not missing");
    };
    assert!(
        why.contains("nothing in site/content/learn/ or site/snippets/ did"),
        "{why}"
    );
    assert!(why.ends_with("\n\n    Book: none, <why>"), "{why}");
}

#[test]
fn every_rule_fails_passes_with_a_home_and_passes_with_its_line() {
    let tracked = paths(&git(&root(), &["ls-files"]).unwrap());
    for rule in real_rules() {
        let in_home = |path: &&String| rule.homes.iter().any(|glob| matches(glob, path));
        let trigger = tracked
            .iter()
            .filter(|path| !in_home(path))
            .find(|path| rule.triggers.iter().any(|glob| matches(glob, path)))
            .unwrap_or_else(|| panic!("rule `{}` has no trigger outside its homes", rule.name));
        let home = tracked.iter().find(in_home).unwrap();
        let name = &rule.name;

        let alone = vec![trigger.clone()];
        let verdict = judge(&rule, &work(), &alone, &done_with("Done."));
        assert!(
            matches!(&verdict, Verdict::Missing(why) if why.contains(&format!("{} none, <why>", rule.mark))),
            "{name}: {verdict:?}"
        );

        let with_home = vec![trigger.clone(), home.clone()];
        assert_eq!(
            judge(&rule, &work(), &with_home, &done_with("Done.")),
            Verdict::HomeChanged(vec![home.clone()]),
            "{name}"
        );

        let line = format!("- {} none, the test says so.", rule.mark);
        let body = format!("## Completion notes\n\n{line}\n");
        let verdict = judge(&rule, &work(), &alone, &move |_| Some(body.clone()));
        assert_eq!(
            verdict,
            Verdict::Waived(format!(
                "ticket 2100's notes say: {} none, the test says so.",
                rule.mark
            )),
            "{name}"
        );

        assert_eq!(
            judge(&rule, &work(), &files(&["README.md"]), &no_ticket),
            Verdict::Quiet,
            "{name}"
        );
    }
}

#[test]
fn a_line_outside_the_notes_does_not_count() {
    let changed = files(&["templates/shuyi-default/template.ron"]);
    let ticket = |_: &str| Some("## Scope\n\nSkills: none, said too early.\n".to_owned());
    assert!(matches!(
        judge(&rule("template-skills"), &work(), &changed, &ticket),
        Verdict::Missing(_)
    ));
}

#[test]
fn a_ticket_not_done_fails_saying_so() {
    let changed = files(&["crates/cli/errors/SH0001.md"]);
    let Verdict::Missing(why) = judge(&rule("problem-code-skills"), &work(), &changed, &no_ticket)
    else {
        panic!("not missing");
    };
    assert!(
        why.starts_with("ticket 2100 is not in tickets/done/, so it has no `Skills:` line."),
        "{why}"
    );
}

#[test]
fn a_pr_for_no_tickets_work_is_reported_not_failed() {
    let changed = files(&["templates/shuyi-default/template.ron"]);
    for (kind, said) in [
        (
            PrKind::TicketsOnly(vec!["2100".to_owned()]),
            "a ticket-only PR",
        ),
        (PrKind::Bot, "a bot's PR"),
        (PrKind::Sonar, "a Sonar batch"),
        (PrKind::Post, "a post for News"),
        (PrKind::GameSmall("1203".to_owned()), "game ticket 1203"),
        (PrKind::None, "no ticket is named"),
    ] {
        let outcome = check(&real_rules(), &kind, &changed, &no_ticket, true);
        assert_eq!(outcome.errors, Vec::<String>::new(), "{kind:?}");
        assert!(!outcome.warnings.is_empty(), "{kind:?}");
        assert!(
            outcome
                .warnings
                .iter()
                .all(|w| w.contains("reported, not failed") && w.contains(said)),
            "{kind:?}: {:?}",
            outcome.warnings
        );
    }
}

#[test]
fn ci_fails_a_missing_line_and_a_local_run_warns() {
    let changed = files(&["templates/shuyi-default/base/AGENTS.md.tmpl"]);
    let strict = check(&real_rules(), &work(), &changed, &done_with("Done."), true);
    assert_eq!(strict.errors.len(), 2, "{:?}", strict.errors);
    for (error, name) in strict
        .errors
        .iter()
        .zip(["template-skills", "template-book"])
    {
        assert!(
            error.starts_with(&format!(
                "rule `{name}` (tools/docs-rules.ron): ticket 2100's"
            )),
            "{error}"
        );
    }
    assert_eq!(strict.warnings, Vec::<String>::new());
    assert!(
        strict
            .report
            .contains("template-skills: no home changed and no `Skills:` line\n"),
        "{}",
        strict.report
    );

    let local = check(&real_rules(), &work(), &changed, &done_with("Done."), false);
    assert_eq!(local.errors, Vec::<String>::new());
    assert_eq!(local.warnings, strict.errors);
}

#[test]
fn one_line_waives_every_rule_of_its_mark() {
    let changed = files(&["templates/shuyi-default/base/AGENTS.md.tmpl"]);
    let ticket = done_with("Skills: none, a comment.\nBook: none, a comment.");
    let outcome = check(&real_rules(), &work(), &changed, &ticket, true);
    assert_eq!(outcome.errors, Vec::<String>::new());
    assert!(
        outcome
            .report
            .contains("template-skills: ticket 2100's notes say: Skills: none, a comment.\n"),
        "{}",
        outcome.report
    );
}

#[test]
fn a_pr_no_rule_cares_about_says_so() {
    let outcome = check(
        &real_rules(),
        &work(),
        &files(&["README.md"]),
        &no_ticket,
        true,
    );
    assert_eq!(outcome.report, "No rule's paths changed.\n");
    assert!(outcome.errors.is_empty() && outcome.warnings.is_empty());
}

#[test]
fn the_options_read_each_flag() {
    let args = files(&[
        "--base",
        "HEAD^1",
        "--head",
        "abc",
        "--pr-branch",
        "t2100-a",
        "--pr-title",
        "[2100] A",
    ]);
    assert_eq!(
        Options::parse(&args),
        Some(Options {
            base: Some("HEAD^1".to_owned()),
            head: Some("abc".to_owned()),
            pr_branch: Some("t2100-a".to_owned()),
            pr_title: Some("[2100] A".to_owned()),
        })
    );
    assert_eq!(Options::parse(&files(&["--base"])), None);
    assert_eq!(Options::parse(&files(&["--what", "x"])), None);
}
