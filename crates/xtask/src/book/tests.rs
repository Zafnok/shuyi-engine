use super::*;

/// A page with `body` after a front matter of two lines, so its first line
/// is line 4.
fn page(body: &str) -> String {
    format!("+++\ntitle = \"A\"\n+++\n{body}")
}

fn has(errors: &[String], part: &str) -> bool {
    errors.iter().any(|e| e.contains(part))
}

#[test]
fn each_audience_has_its_label() {
    assert_eq!(label("programming"), Some("New to programming?"));
    assert_eq!(label("games"), Some("New to making games?"));
    assert_eq!(label("rust"), Some("New to Rust?"));
    assert_eq!(label("shuyi"), Some("New to Shuyi?"));
    assert_eq!(label("godot"), None);
}

#[test]
fn a_page_with_marks_passes_and_splits_at_them() {
    let body = "Before.\n\n{% newcomer(to=\"rust\", topic=\"`match`\") %}\nIt compares.\n\n{{ snippet(file=\"a.rs\") }}\n{% end %}\n\nAfter.\n";
    assert_eq!(check("a.md", &page(body)), Vec::<String>::new());
    let (parts, errors) = parts("a.md", body, 4);
    assert!(errors.is_empty(), "{errors:?}");
    assert_eq!(
        parts,
        [
            Part {
                mark: None,
                check: None,
                exercise: None,
                text: "Before.\n\n",
                line: 4
            },
            Part {
                mark: Some(Mark {
                    to: "rust".into(),
                    topic: "`match`".into()
                }),
                check: None,
                exercise: None,
                text: "It compares.\n\n{{ snippet(file=\"a.rs\") }}\n",
                line: 7
            },
            Part {
                mark: None,
                check: None,
                exercise: None,
                text: "\nAfter.\n",
                line: 11
            },
        ]
    );
}

#[test]
fn a_page_with_no_mark_is_one_plain_part() {
    let (parts, errors) = parts("a.md", "Plain.\n", 4);
    assert_eq!(errors, Vec::<String>::new());
    assert_eq!(parts.len(), 1);
    assert_eq!(parts[0].mark, None);
}

#[test]
fn an_unknown_audience_fails_on_its_line() {
    let errors = check(
        "a.md",
        &page("\n{% newcomer(to=\"godot\", topic=\"nodes\") %}\nText.\n{% end %}\n"),
    );
    assert!(
        has(&errors, "a.md:5: to=\"godot\" isn't an audience"),
        "{errors:?}"
    );
}

#[test]
fn a_missing_or_empty_topic_or_to_fails() {
    for (args, wanted) in [
        ("to=\"rust\"", "no `topic`"),
        ("to=\"rust\", topic=\" \"", "no `topic`"),
        ("topic=\"x\"", "no `to`"),
        ("to=\"rust\", topic=\"x\", look=\"y\"", "no argument `look`"),
    ] {
        let errors = check(
            "a.md",
            &page(&format!("\n{{% newcomer({args}) %}}\nText.\n{{% end %}}\n")),
        );
        assert!(has(&errors, wanted), "{args}: {errors:?}");
    }
}

#[test]
fn an_unclosed_mark_fails_on_its_opener() {
    let errors = check(
        "a.md",
        &page("\n{% newcomer(to=\"rust\", topic=\"x\") %}\nText.\n"),
    );
    assert!(
        has(&errors, "a.md:5: a mark with no `{% end %}`"),
        "{errors:?}"
    );
}

#[test]
fn a_mark_inside_a_mark_fails() {
    let errors = check(
        "a.md",
        &page(
            "\n{% newcomer(to=\"rust\", topic=\"x\") %}\n\n{% newcomer(to=\"programming\", topic=\"y\") %}\nText.\n{% end %}\n\n{% end %}\n",
        ),
    );
    assert!(
        has(
            &errors,
            "a.md:7: a mark inside the mark of line 5: marks don't nest"
        ),
        "{errors:?}"
    );
}

#[test]
fn an_end_with_no_mark_fails() {
    let errors = check("a.md", &page("\nText.\n\n{% end %}\n"));
    assert!(
        has(&errors, "a.md:7: a `{% end %}` with no mark open"),
        "{errors:?}"
    );
}

#[test]
fn a_mark_not_on_its_own_lines_fails() {
    for (body, wanted) in [
        (
            "\nSome {% newcomer(to=\"rust\", topic=\"x\") %} text.\n",
            "a mark stands on a line of its own",
        ),
        (
            "\n{% newcomer(to=\"rust\", topic=\"x\") %} Text.\n{% end %}\n",
            "a mark stands on a line of its own",
        ),
        (
            "\n  {% newcomer(to=\"rust\", topic=\"x\") %}\n",
            "with no indent",
        ),
        (
            "\nText. {% end %}\n",
            "a `{% end %}` stands on a line of its own",
        ),
    ] {
        let errors = check("a.md", &page(body));
        assert!(has(&errors, wanted), "{body}: {errors:?}");
    }
}

#[test]
fn a_mark_needs_a_blank_line_before_and_after() {
    let errors = check(
        "a.md",
        &page("Text.\n{% newcomer(to=\"rust\", topic=\"x\") %}\nIn.\n{% end %}\nText.\n"),
    );
    assert!(
        has(&errors, "a.md:5: a mark has no blank line before it"),
        "{errors:?}"
    );
    assert!(
        has(&errors, "a.md:7: a mark's line has no blank line after it"),
        "{errors:?}"
    );
}

#[test]
fn an_escaped_mark_is_text() {
    let body = "\n```text\n{%/* newcomer(to=\"x\") */%}\n{%/* end */%}\n```\n";
    assert_eq!(check("a.md", &page(body)), Vec::<String>::new());
}

#[test]
fn a_callout_of_the_old_form_fails() {
    for old in [
        "> **Rust:** A crate.",
        "> **Shuyi:** A screen.",
        "  > **Game idea:** A loop.",
    ] {
        let errors = check("a.md", &page(&format!("\n{old}\n")));
        assert!(
            has(&errors, "a.md:5: a callout of the old form"),
            "{old}: {errors:?}"
        );
    }
    assert_eq!(
        check("a.md", &page("\n> **Note:** Fine.\n")),
        Vec::<String>::new()
    );
}

#[test]
fn a_page_marked_whole_names_an_audience() {
    let ok = "+++\ntitle = \"A\"\n\n[extra]\nnewcomer = \"programming\"\n+++\n\nText.\n";
    assert_eq!(check("a.md", ok), Vec::<String>::new());
    let errors = check("a.md", &ok.replace("programming", "beginners"));
    assert!(
        has(
            &errors,
            "a.md:5: newcomer = \"beginners\" isn't an audience"
        ),
        "{errors:?}"
    );
}

#[test]
fn a_page_with_no_front_matter_fails() {
    assert!(has(&check("a.md", "Text.\n"), "a.md:1: no `+++`"));
    assert!(has(
        &check("a.md", "+++\ntitle = \"A\"\n"),
        "no closing `+++`"
    ));
}

// -- the quick checks (ticket 2827) --

/// A check with `to` and `answers` (each a whole line).
fn a_check(to: &str, answers: &str) -> String {
    format!("\n{{% check({to}) %}}\nWhat does `?` do?\n\n{answers}{{% end %}}\n")
}

const GOOD: &str =
    "- [x] It returns early. — `?` hands the `Err` back.\n- [ ] It panics. — Nothing stops.\n";

#[test]
fn a_check_reads_into_its_question_and_answers() {
    let body = a_check("to=\"rust\"", GOOD);
    let (parts, errors) = parts("a.md", &body, 4);
    assert!(errors.is_empty(), "{errors:?}");
    let part = parts.iter().find(|part| part.check.is_some()).unwrap();
    assert_eq!(
        part.mark,
        Some(Mark {
            to: "rust".into(),
            topic: CHECK_TOPIC.into()
        })
    );
    assert_eq!(
        part.check,
        Some(Check {
            question: "What does `?` do?".into(),
            answers: vec![
                Answer {
                    right: true,
                    text: "It returns early.".into(),
                    why: "`?` hands the `Err` back.".into()
                },
                Answer {
                    right: false,
                    text: "It panics.".into(),
                    why: "Nothing stops.".into()
                },
            ]
        })
    );
    assert_eq!(part.line, 6);
}

#[test]
fn a_check_with_no_to_is_for_everyone() {
    let body = a_check("", GOOD);
    let (parts, errors) = parts("a.md", &body, 4);
    assert!(errors.is_empty(), "{errors:?}");
    let part = parts.iter().find(|part| part.check.is_some()).unwrap();
    assert_eq!(part.mark, None);
}

#[test]
fn a_malformed_check_fails() {
    for (to, answers, wanted) in [
        (
            "",
            "- [ ] It returns early. — No.\n- [ ] It panics. — No.\n",
            "a.md:5: a check with no right answer",
        ),
        (
            "",
            "- [x] It returns early.\n- [ ] It panics. — Nothing stops.\n",
            "a.md:8: an answer with no reason",
        ),
        (
            "",
            "- [x] It returns early. — \n- [ ] It panics. — Nothing stops.\n",
            "a.md:8: an answer with no reason",
        ),
        (
            "to=\"godot\"",
            GOOD,
            "a.md:5: to=\"godot\" isn't an audience",
        ),
        ("look=\"x\"", GOOD, "no argument `look`"),
        (
            "",
            "- [x] It returns early. — Yes.\n",
            "a.md:5: a check with 1 answers: it has two to four",
        ),
        (
            "",
            "- [x] A. — a.\n- [ ] B. — b.\n- [ ] C. — c.\n- [ ] D. — d.\n- [ ] E. — e.\n",
            "a check with 5 answers",
        ),
        (
            "",
            "- [X] It returns early. — Yes.\n- [ ] It panics. — No.\n",
            "a.md:8: an answer starts `- [x] `",
        ),
        (
            "",
            "- [x] It returns early. — Yes.\n- [ ] It panics. — No.\nMore text.\n",
            "a.md:10: a check's answers are one line each",
        ),
        (
            "",
            "- [x] See [the book](@/a.md). — Yes.\n- [ ] It panics. — No.\n",
            "an answer holds no link",
        ),
    ] {
        let errors = check("a.md", &page(&a_check(to, answers)));
        assert!(has(&errors, wanted), "{to} {answers}: {errors:?}");
    }
}

#[test]
fn a_check_needs_its_question() {
    let errors = check(
        "a.md",
        &page(&format!("\n{{% check() %}}\n{GOOD}{{% end %}}\n")),
    );
    assert!(
        has(&errors, "a.md:6: a check with no question"),
        "{errors:?}"
    );
}

#[test]
fn a_mark_inside_a_check_fails() {
    let body = "\n{% check() %}\nWhat?\n\n{% newcomer(to=\"rust\", topic=\"x\") %}\nText.\n{% end %}\n\n{% end %}\n";
    let errors = check("a.md", &page(body));
    assert!(
        has(
            &errors,
            "a.md:8: a mark inside the check of line 5: marks don't nest"
        ),
        "{errors:?}"
    );
}

/// A page of `kind` (`chapter = 1`, `step = 1` or nothing) with `body`,
/// whose first line is line 7.
fn kind_page(kind: &str, body: &str) -> String {
    format!("+++\ntitle = \"A\"\n\n[extra]\n{kind}\n+++\n{body}")
}

#[test]
fn a_chapter_or_a_step_ends_with_two_or_three_checks() {
    let two = format!(
        "\n## Check yourself\n{}{}",
        a_check("", GOOD),
        a_check("", GOOD)
    );
    for kind in ["chapter = 1", "step = 1"] {
        assert_eq!(
            check("a.md", &kind_page(kind, &two)),
            Vec::<String>::new(),
            "{kind}"
        );
        let none = check("a.md", &kind_page(kind, "\nText.\n"));
        assert!(has(&none, "ends with `## Check yourself`"), "{none:?}");
        let one = format!("\n## Check yourself\n{}", a_check("", GOOD));
        let errors = check("a.md", &kind_page(kind, &one));
        assert!(
            has(&errors, "a.md:8: `## Check yourself` holds 1 checks"),
            "{errors:?}"
        );
        let after = format!("{two}\n## More\n");
        let errors = check("a.md", &kind_page(kind, &after));
        assert!(
            has(&errors, "a heading after `## Check yourself`"),
            "{errors:?}"
        );
        let before = format!("{}{two}", a_check("", GOOD));
        let errors = check("a.md", &kind_page(kind, &before));
        assert!(
            has(&errors, "a.md:8: a check before `## Check yourself`"),
            "{errors:?}"
        );
    }
}

#[test]
fn another_page_has_no_checks() {
    let two = format!(
        "\n## Check yourself\n{}{}",
        a_check("", GOOD),
        a_check("", GOOD)
    );
    let errors = check("a.md", &kind_page("appendix = \"A\"", &two));
    assert!(
        has(
            &errors,
            "a.md:8: `## Check yourself` is for a concept chapter"
        ),
        "{errors:?}"
    );
    assert!(
        has(&errors, "a.md:10: a check is for a concept chapter"),
        "{errors:?}"
    );
}

/// A check whose one right answer is second.
const SECOND: &str =
    "- [ ] It panics. — Nothing stops.\n- [x] It returns early. — `?` hands the `Err` back.\n";

/// A check with two right answers, which the spread doesn't count.
const BOTH: &str =
    "- [x] It returns early. — `?` hands the `Err` back.\n- [x] It converts. — `From` runs.\n";

/// Two chapters, a.md and b.md, holding `checks` two to a page.
fn chapters(checks: &[&str]) -> Vec<(String, String)> {
    checks
        .chunks(2)
        .zip(["a.md", "b.md"])
        .map(|(two, shown)| {
            let body = format!(
                "\n## Check yourself\n{}{}",
                a_check("", two[0]),
                a_check("", two[1])
            );
            (shown.to_owned(), kind_page("chapter = 1", &body))
        })
        .collect()
}

#[test]
fn the_right_answers_are_spread_across_the_book() {
    // 2 of 4 first: half is spread enough.
    let half = chapters(&[GOOD, SECOND, GOOD, SECOND]);
    assert_eq!(check_pages(&half), Vec::<String>::new());
    // 3 of 4 first, across two pages.
    let most = chapters(&[GOOD, GOOD, SECOND, GOOD]);
    assert_eq!(
        check_pages(&most),
        [
            "site/content/learn: the right answer is first in 3 of 4 quick checks, \
             second in 1: spread them (2833)"
        ]
    );
    // A check with two right answers counts for neither side.
    let two_right = chapters(&[GOOD, BOTH, SECOND, BOTH]);
    assert_eq!(check_pages(&two_right), Vec::<String>::new());
}

#[test]
fn the_book_s_pages_pass() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    assert_eq!(run(&root), Vec::<String>::new());
}

// -- the exercises (ticket 2828) --

/// An exercise with the opener's arguments `args` and `body`.
fn an_exercise(args: &str, body: &str) -> String {
    format!("\n{{% exercise({args}) %}}\n{body}{{% end %}}\n")
}

/// An exercise's body: its task and its hint.
const TASK: &str = "Add a box.\n\nHint: a box is a feature.\nIt needs a target.\n";

/// A step whose playground holds `levels/1.map`, with two checks and then
/// `after`, whose section starts at line 9.
fn step_with(after: &str) -> String {
    let body = format!(
        "\n## Check yourself\n{}{}{after}",
        a_check("", GOOD),
        a_check("", GOOD)
    );
    format!(
        "+++\ntitle = \"A\"\n\n[extra]\nstep = 2\nfiles = [\"levels/1.map\", \"palette.ron\"]\n+++\n{body}"
    )
}

#[test]
fn the_files_line_is_read_as_copy_playground_reads_it() {
    assert_eq!(
        listed(r#"files = ["levels/1.map", "en.ron"]"#),
        ["levels/1.map", "en.ron"]
    );
    assert_eq!(listed("files = []"), Vec::<String>::new());
    assert_eq!(listed("files"), Vec::<String>::new());
    assert_eq!(page_files(&step_with("")), ["levels/1.map", "palette.ron"]);
}

#[test]
fn an_exercise_reads_into_its_check_file_task_and_hint() {
    let text = step_with(&an_exercise(
        r#"check="second-box", file="levels/1.map", to="games""#,
        TASK,
    ));
    assert_eq!(check("a.md", &text), Vec::<String>::new());
    let exercises = exercises("a.md", &text);
    assert_eq!(
        exercises,
        [Exercise {
            check: "second-box".to_owned(),
            file: "levels/1.map".to_owned(),
            task: "Add a box.".to_owned(),
            hint: "a box is a feature.\nIt needs a target.".to_owned(),
        }]
    );
    let front = front_matter("a.md", &text).unwrap();
    let (found, _) = parts("a.md", front.body, front.first_line);
    let part = found.iter().find(|part| part.exercise.is_some()).unwrap();
    assert_eq!(
        part.mark,
        Some(Mark {
            to: "games".to_owned(),
            topic: EXERCISE_TOPIC.to_owned()
        })
    );
}

#[test]
fn a_malformed_exercise_fails() {
    let cases = [
        (
            r#"file="levels/1.map""#,
            TASK,
            "an exercise is `{% exercise(check=",
        ),
        (
            r#"check="Second box", file="levels/1.map""#,
            TASK,
            "lowercase letters, digits and `-`",
        ),
        (
            r#"check="a", file="levels/1.map", to="cats""#,
            TASK,
            "to=\"cats\" isn't an audience",
        ),
        (
            r#"check="a", file="levels/1.map", size="2""#,
            TASK,
            "no argument `size`",
        ),
        (
            r#"check="a", file="levels/1.map""#,
            "Add a box.\n",
            "an exercise with no hint",
        ),
        (
            r#"check="a", file="levels/1.map""#,
            "Hint: just this.\n",
            "no task before its hint",
        ),
        (
            r#"check="a", file="levels/1.map""#,
            "Add a box.\nHint: here.\n",
            "a blank line before `Hint: `",
        ),
        (
            r#"check="a", file="levels/1.map""#,
            "Add a box.\n\nHint: one.\n\nHint: two.\n",
            "one hint",
        ),
        (
            r#"check="a", file="levels/1.map""#,
            "Add {{ snippet() }}.\n\nHint: here.\n",
            "holds no shortcode",
        ),
        (
            r#"check="a", file="en.ron""#,
            TASK,
            "which the page's playground doesn't hold",
        ),
    ];
    for (args, body, error) in cases {
        let errors = check("a.md", &step_with(&an_exercise(args, body)));
        assert!(has(&errors, error), "{args} {body:?}: {errors:#?}");
    }
    let twice = an_exercise(r#"check="a", file="levels/1.map""#, TASK).repeat(2);
    let errors = check("a.md", &step_with(&twice));
    assert!(
        has(&errors, "a second exercise with the check `a`"),
        "{errors:#?}"
    );
}

#[test]
fn a_step_with_a_playground_has_one_or_two_exercises_after_its_checks() {
    let one = an_exercise(r#"check="a", file="levels/1.map""#, TASK);
    let two = one.clone() + &an_exercise(r#"check="b", file="palette.ron""#, TASK);
    assert_eq!(check("a.md", &step_with(&two)), Vec::<String>::new());
    let errors = check("a.md", &step_with(""));
    assert!(
        has(&errors, "a.md:9: the step has 0 exercises"),
        "{errors:#?}"
    );
    let three = two + &an_exercise(r#"check="c", file="palette.ron""#, TASK);
    let errors = check("a.md", &step_with(&three));
    assert!(has(&errors, "the step has 3 exercises"), "{errors:#?}");
    let first = format!(
        "+++\ntitle = \"A\"\n\n[extra]\nstep = 2\nfiles = [\"levels/1.map\"]\n+++\n\n## Check yourself\n{one}{}{}",
        a_check("", GOOD),
        a_check("", GOOD)
    );
    let errors = check("a.md", &first);
    assert!(has(&errors, "an exercise before a check"), "{errors:#?}");
}

#[test]
fn only_a_step_with_a_playground_has_exercises() {
    let one = an_exercise(r#"check="a", file="levels/1.map""#, TASK);
    let two = format!(
        "\n## Check yourself\n{}{}{one}",
        a_check("", GOOD),
        a_check("", GOOD)
    );
    for kind in ["chapter = 1", "step = 1", ""] {
        let errors = check("a.md", &kind_page(kind, &two));
        assert!(
            has(
                &errors,
                "an exercise is for a project step with a playground"
            ),
            "{kind}: {errors:#?}"
        );
    }
}
