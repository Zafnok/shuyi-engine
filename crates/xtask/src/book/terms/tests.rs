use super::*;

const GLOSSARY_TEXT: &str = "Intro.\n\n| Word | New to | What it means | Where |\n| ---- | ------ | ------------- | ----- |\n| Update, draw | Making games | The loop's jobs | [Step 1](a.md) |\n| `History` | Shuyi | Every command | [Step 5](b.md) |\n| Layers, mod | Shuyi | Bundles in order | [Chapter 3](c.md) |\n| Golden table | Programming | Known answers | [Chapter 8](d.md) |\n";

fn words() -> Vec<String> {
    glossary_words(GLOSSARY_TEXT)
}

#[test]
fn the_rows_give_their_words() {
    assert_eq!(
        words(),
        ["update", "draw", "history", "layer", "mod", "golden table"]
    );
}

#[test]
fn a_term_with_a_row_passes() {
    let body = "The **update** reads the input, and **`History`** keeps it.\n";
    assert_eq!(check("a.md", body, 4, &words()), Vec::<String>::new());
}

#[test]
fn a_term_without_a_row_fails_naming_its_page_line_and_word() {
    let body = "First.\n\nA **stream** of numbers.\n";
    let errors = check("site/a.md", body, 4, &words());
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(errors[0].starts_with("site/a.md:6: the term **stream** has no row"));
}

#[test]
fn case_and_plural_are_aside() {
    let body = "It **draws**; a **Layer**; two **Golden Tables**; the **mods**.\n";
    assert_eq!(check("a.md", body, 1, &words()), Vec::<String>::new());
}

#[test]
fn a_run_in_heading_is_not_a_term() {
    let body = "**A bad key.** It fails.\n\n**Windows:** run it.\n\n**New to Rust?** Read on.\n";
    assert_eq!(check("a.md", body, 1, &words()), Vec::<String>::new());
}

#[test]
fn emphasis_in_italics_is_not_a_term() {
    let body = "State changes *only through commands*, and _twice_.\n";
    assert_eq!(check("a.md", body, 1, &words()), Vec::<String>::new());
}

#[test]
fn a_bold_may_span_lines_and_code_is_skipped() {
    let body = "A **golden\ntable** here.\n\n```rust\nlet x = **y**;\n```\n\nAnd `**not**` bold.\n";
    let found = bolds(body, 1);
    assert_eq!(
        found,
        [Bold {
            text: "golden table".to_string(),
            line: 1
        }]
    );
    assert_eq!(check("a.md", body, 1, &words()), Vec::<String>::new());
}

#[test]
fn an_unclosed_bold_ends_with_its_paragraph() {
    let body = "A ** star.\n\nThen **draw**.\n";
    assert_eq!(
        bolds(body, 1),
        [Bold {
            text: "draw".to_string(),
            line: 3
        }]
    );
}

#[test]
fn only_the_book_s_pages_are_checked() {
    assert!(is_book_page("site/content/learn/book/actions.md"));
    assert!(is_book_page("site/content/learn/book/box-pusher/01-new.md"));
    assert!(!is_book_page("site/content/learn/book/glossary.md"));
    assert!(!is_book_page("site/content/learn/book/_index.md"));
    assert!(!is_book_page(
        "site/content/learn/book/box-pusher/_index.md"
    ));
    assert!(!is_book_page("site/content/learn/playground.md"));
}

#[test]
fn plurals_reduce_to_one_key() {
    assert_eq!(key("Entries"), "entry");
    assert_eq!(key("boxes"), "box");
    assert_eq!(key("class"), "class");
    assert_eq!(key("bus"), "bus");
    assert_eq!(key("`Vec`"), "vec");
}
