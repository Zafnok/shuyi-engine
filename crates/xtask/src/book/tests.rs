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
                text: "Before.\n\n",
                line: 4
            },
            Part {
                mark: Some(Mark {
                    to: "rust".into(),
                    topic: "`match`".into()
                }),
                text: "It compares.\n\n{{ snippet(file=\"a.rs\") }}\n",
                line: 7
            },
            Part {
                mark: None,
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

#[test]
fn the_book_s_pages_pass() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    assert_eq!(run(&root), Vec::<String>::new());
}
