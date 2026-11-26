use super::*;

/// The snippet files of the tests: `src/a.rs` with an anchor, and the
/// tutorial step `01-new`'s `shuyi.ron`.
fn files(step: Option<&str>, file: &str) -> Option<String> {
    match (step, file) {
        (None, "src/a.rs") => Some(
            "use x;\n// ANCHOR: two\nfn two() {}\n// ANCHOR: inner\nlet y = 2;\n// ANCHOR_END: inner\n// ANCHOR_END: two\n"
                .to_owned(),
        ),
        (Some("01-new"), "shuyi.ron") => Some("(game: \"g\")".to_owned()),
        _ => None,
    }
}

#[test]
fn each_of_the_five_shortcodes_is_read_as_the_twin_reads_it() {
    let body = "\
Intro.

{{ snippet(file=\"src/a.rs\", anchor=\"two\") }}

{{ snippet(file=\"shuyi.ron\", step=\"01-new\", language=\"ron\") }}

{% newcomer(to=\"rust\", topic=\"a trait\") %}
A trait is a promise.
{% end %}

{% check() %}
What is it?

- [x] A promise. — Right.
{% end %}

{% exercise(check=\"k\", file=\"keymap.ron\") %}
Add a key.
{% end %}

{{ try_it(name=\"title-blue\") }}

Write `{{/* snippet(file=\"x\") */}}` to show a file.
";
    let (text, unknown) = clean(body, &files);
    assert!(unknown.is_empty(), "{unknown:?}");
    assert!(
        text.contains("```rust\nfn two() {}\nlet y = 2;\n```"),
        "{text}"
    );
    assert!(text.contains("```ron\n(game: \"g\")\n```"), "{text}");
    assert!(text.contains("A trait is a promise."), "{text}");
    assert!(text.contains("- [x] A promise. — Right."), "{text}");
    assert!(text.contains("Add a key."), "{text}");
    assert!(
        text.contains("Write `{{ snippet(file=\"x\") }}` to show"),
        "{text}"
    );
    for gone in ["{%", "newcomer(", "try_it", "title-blue", "ANCHOR"] {
        assert!(!text.contains(gone), "{gone} in {text}");
    }
}

#[test]
fn a_shortcode_it_cant_read_is_named() {
    let (text, unknown) = clean(
        "a {{ snippet(file=\"nope.rs\") }} b {{ chart(x=1) }} c {{ snippet(file=\"src/a.rs\", anchor=\"z\") }} {{ not a call }}",
        &files,
    );
    assert_eq!(unknown.len(), 3, "{unknown:?}");
    assert!(unknown[0].contains("no file nope.rs"), "{unknown:?}");
    assert!(unknown[1].contains("`chart`"), "{unknown:?}");
    assert!(unknown[2].contains("no anchor z"), "{unknown:?}");
    assert!(text.ends_with("{{ not a call }}"), "{text}");
    let (_, unknown) = clean("{{ snippet(anchor=\"two\") }}", &files);
    assert_eq!(unknown, ["snippet: no `file`"]);
    let (_, unknown) = clean("{{ snippet(file=\"src/a.rs\", color=\"red\") }}", &files);
    assert_eq!(unknown, ["snippet: no argument `color`"]);
}

#[test]
fn a_page_is_cut_at_its_headings_outside_code() {
    let text = "Start.\n\n## One thing\n\nBody one.\n\n```sh\n## not a heading\n```\n\n### A `Sprite`, [linked](x.md)\n\nBody two.\n\n## One thing\n\nAgain.\n";
    let parts = sections(text);
    let anchors: Vec<Option<&str>> = parts.iter().map(|s| s.anchor.as_deref()).collect();
    assert_eq!(
        anchors,
        [
            None,
            Some("one-thing"),
            Some("a-sprite-linked"),
            Some("one-thing-1")
        ]
    );
    assert_eq!(parts[0].text, "Start.");
    assert!(parts[1].text.contains("## not a heading"), "{:?}", parts[1]);
    assert_eq!(
        parts[2].heading.as_deref(),
        Some("A `Sprite`, [linked](x.md)")
    );
    assert_eq!(
        headings(text),
        ["One thing", "A `Sprite`, [linked](x.md)", "One thing"]
    );
}

#[test]
fn a_slug_is_zolas() {
    assert_eq!(slug("Naming a key in text"), "naming-a-key-in-text");
    assert_eq!(slug("Actions of a game's own"), "actions-of-a-game-s-own");
    assert_eq!(
        slug("Lending with `&`, and a tuple"),
        "lending-with-and-a-tuple"
    );
    assert_eq!(slug("**Bold** _x_"), "bold-x");
    assert_eq!(slug("TurnRules"), "turnrules");
}

#[test]
fn a_twins_url_is_its_pages_with_index_md() {
    let site = crate::contract::new::ENGINE_SITE;
    assert_eq!(
        twin_url("book/actions.md"),
        format!("{site}/learn/book/actions/index.md")
    );
    assert_eq!(
        twin_url("book/_index.md"),
        format!("{site}/learn/book/index.md")
    );
    assert_eq!(twin_url("_index.md"), format!("{site}/learn/index.md"));
}

#[test]
fn an_examples_rust_file_is_its_doc_and_its_names() {
    let source = "//! Hello, Grid.\n//!\n//! A grid game.\nuse x;\n\n/// Not this.\npub fn add_sprite(a: u8) {}\nfn private() {}\npub struct Board {\n}\npub enum Cell { A }\nimpl Screen for Title {\n}\nimpl<'a> View<'a> where 'a: 'static {\n";
    assert_eq!(
        example_text(source),
        "Hello, Grid.\n\nA grid game.\n\nadd_sprite\nBoard\nCell\nimpl Screen for Title\nimpl<'a> View<'a>"
    );
    assert_eq!(example_text("fn main() {}\n"), "");
}

#[test]
fn front_matter_gives_the_title_and_is_dropped() {
    let (title, body) =
        split_front("+++\ntitle = \"Actions\"\nweight = 4\n\n[extra]\nx = 1\n+++\n\nBody.\n");
    assert_eq!(title.as_deref(), Some("Actions"));
    assert_eq!(body, "Body.\n");
    assert_eq!(split_front("No front.\n"), (None, "No front.\n"));
    assert_eq!(
        split_front("+++\nnever ends\n"),
        (None, "+++\nnever ends\n")
    );
}

#[test]
fn every_learn_page_reads_without_an_unknown_shortcode() {
    let pages = learn_pages();
    assert!(pages.len() >= 25, "{}", pages.len());
    for page in &pages {
        assert!(page.unknown.is_empty(), "{}: {:?}", page.path, page.unknown);
        assert!(!page.text.contains("{%"), "{}", page.path);
        assert_ne!(page.title, page.path, "{}: no title", page.path);
    }
    assert!(learn_page("book/actions.md").is_some());
    assert!(learn_page("book/nope.md").is_none());
    assert!(learn_page("book").is_none());
}

#[test]
fn the_corpus_holds_every_kind_in_path_order() {
    let chunks = chunks();
    for kind in super::super::HitKind::ALL {
        assert!(chunks.iter().any(|c| c.kind == *kind), "{kind:?}");
    }
    assert!(
        chunks
            .windows(2)
            .all(|w| (&w[0].path, w[0].position) < (&w[1].path, w[1].position))
    );
    let skill = chunks
        .iter()
        .find(|c| c.path == "skills/shuyi-input/SKILL.md")
        .unwrap();
    assert_eq!(skill.title, "shuyi-input: Input: actions, never keys");
    assert!(
        skill.text.starts_with("Input on Shuyi Engine"),
        "{}",
        skill.text
    );
    let code = chunks
        .iter()
        .find(|c| c.path == "crates/cli/errors/SH2003.md")
        .unwrap();
    assert!(code.title.starts_with("SH2003: "), "{}", code.title);
    let learn = chunks
        .iter()
        .find(|c| c.anchor.as_deref() == Some("naming-a-key-in-text"))
        .unwrap();
    assert_eq!(learn.path, "site/content/learn/book/actions.md");
    assert!(
        learn
            .url
            .as_deref()
            .unwrap()
            .ends_with("/learn/book/actions/index.md")
    );
    assert!(
        chunks
            .iter()
            .filter(|c| c.kind == super::super::HitKind::Example)
            .all(|c| has_extension(&c.path, "rs") || has_extension(&c.path, "md"))
    );
    // A game's tools' output (`dist/bench/…/bench-report.md`) is no part
    // of it: a `bench` run would otherwise rebuild `shuyi` for the next.
    assert!(
        chunks
            .iter()
            .all(|c| !c.path.contains("/dist/") && !c.path.contains("/target/")),
        "{:?}",
        chunks
            .iter()
            .find(|c| c.path.contains("/dist/") || c.path.contains("/target/"))
    );
}

#[test]
fn the_embedded_corpus_stays_small() {
    let sizes = embedded_bytes();
    let total: usize = sizes.iter().map(|(_, bytes)| bytes).sum();
    // The Learn pages, the snippets and the examples, as the ticket
    // planned them, under 1 MB; the tutorial's files the snippets show,
    // under 600 KB more.
    let planned: usize = sizes
        .iter()
        .filter(|(name, _)| *name != "tutorial")
        .map(|(_, bytes)| bytes)
        .sum();
    assert!(planned < 1_000_000, "{sizes:?}");
    assert!(total - planned < 600_000, "{sizes:?}");
}
