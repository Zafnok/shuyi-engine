use super::*;
use crate::contract::docs::HitKind;

fn chunk(path: &str, position: u32, title: &str, text: &str) -> Chunk {
    Chunk {
        kind: HitKind::Learn,
        title: title.to_owned(),
        path: path.to_owned(),
        anchor: None,
        url: None,
        position,
        text: text.to_owned(),
    }
}

fn terms(text: &str) -> Vec<String> {
    tokens(text)
}

#[test]
fn a_snake_or_camel_word_is_kept_whole_and_split() {
    assert_eq!(terms("add_sprite"), ["add_sprite", "add", "sprite"]);
    assert_eq!(terms("AddSprite"), ["addsprite", "add", "sprite"]);
    assert_eq!(terms("HTTPServer"), ["httpserver", "http", "server"]);
    assert_eq!(terms("__init__"), ["init"]);
    assert_eq!(
        terms("buf.add_sprite(x)"),
        ["buf", "add_sprite", "add", "sprite", "x"]
    );
    assert_eq!(terms("SH2003"), ["sh2003"]);
}

#[test]
fn stop_words_are_left_out_and_others_lowercased() {
    assert_eq!(terms("Play a sound in the Game"), ["play", "sound", "game"]);
    assert_eq!(terms("the a of to is and in it"), Vec::<String>::new());
    assert_eq!(terms("  , . ; "), Vec::<String>::new());
    // A stop word inside a snake word is left out too.
    assert_eq!(terms("a_b"), ["a_b", "b"]);
}

#[test]
fn a_non_ascii_word_is_one_token() {
    assert_eq!(terms("café 書以"), ["café", "書以"]);
    // Only ASCII is lowercased.
    assert_eq!(terms("ÉCOLE"), ["École"]);
}

#[test]
fn a_title_term_ranks_above_one_in_the_body_once() {
    let index = Index::new(vec![
        chunk(
            "a.md",
            0,
            "Notes",
            "A sound plays here, among many other words of text.",
        ),
        chunk(
            "b.md",
            0,
            "Sound",
            "Many other words of text, about nothing much at all.",
        ),
        chunk("c.md", 0, "Other", "Nothing to see."),
    ]);
    let found = index.search(&terms("sound"));
    assert_eq!(found.len(), 2);
    assert_eq!(index.chunks[found[0].chunk].path, "b.md");
    assert!(found[0].score > found[1].score, "{found:?}");
}

#[test]
fn ties_go_by_path_then_position() {
    let index = Index::new(vec![
        chunk("b.md", 0, "t", "word"),
        chunk("a.md", 1, "t", "word"),
        chunk("a.md", 0, "t", "word"),
    ]);
    let found = index.search(&terms("word"));
    let order: Vec<(&str, u32)> = found
        .iter()
        .map(|f| {
            (
                index.chunks[f.chunk].path.as_str(),
                index.chunks[f.chunk].position,
            )
        })
        .collect();
    assert_eq!(order, [("a.md", 0), ("a.md", 1), ("b.md", 0)]);
    assert!(found.windows(2).all(|w| w[0].score == w[1].score));
}

#[test]
fn a_term_no_chunk_has_finds_nothing() {
    let index = Index::new(vec![chunk("a.md", 0, "t", "word")]);
    assert_eq!(index.search(&terms("zzzqqq")), []);
    assert!(index.idf("zzzqqq").abs() < f64::EPSILON);
    assert!(index.idf("word") > 0.0);
    // An empty index answers nothing, and divides by nothing.
    assert_eq!(Index::new(Vec::new()).search(&terms("word")), []);
}

#[test]
fn a_score_is_thousandths_rounded() {
    assert_eq!(thousandths(1.2345), 1235);
    assert_eq!(thousandths(0.0), 0);
    assert_eq!(thousandths(-1.0), 0);
}
