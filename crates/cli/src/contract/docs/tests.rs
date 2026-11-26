use proptest::prelude::*;

use super::corpus::Chunk;
use super::*;

fn asked(query: &str) -> DocsOutput {
    let answer = docs(&DocsInput::new(query));
    assert!(answer.ok, "{answer:?}");
    answer.result.unwrap()
}

fn with_limit(query: &str, limit: u32) -> usize {
    let mut input = DocsInput::new(query);
    input.limit = Some(limit);
    docs(&input).result.unwrap().hits.len()
}

#[test]
fn play_a_sound_names_the_page_that_says_how() {
    let output = asked("play a sound");
    let row = output
        .how
        .iter()
        .find(|row| row.ask == "play a sound")
        .unwrap_or_else(|| panic!("{output:?}"));
    assert_eq!(row.path, "site/content/learn/book/games-and-screens.md");
    assert_eq!(row.anchor.as_deref(), Some("the-context"));
    assert!(
        row.url
            .as_deref()
            .unwrap()
            .ends_with("/learn/book/games-and-screens/index.md")
    );
    assert_ne!(output.hits.len(), 0);
}

#[test]
fn a_name_is_found_in_the_code_it_is_in() {
    let output = asked("add_sprite");
    assert!(
        output
            .hits
            .iter()
            .any(|hit| hit.excerpt.contains("add_sprite")),
        "{:#?}",
        output.hits
    );
}

#[test]
fn a_problem_code_finds_its_page() {
    let output = asked("SH2003");
    assert!(
        output
            .hits
            .iter()
            .any(|hit| hit.kind == HitKind::Code && hit.path == "crates/cli/errors/SH2003.md"),
        "{:#?}",
        output.hits
    );
}

#[test]
fn an_empty_query_or_one_of_common_words_is_a_wrong_argument() {
    for query in ["", "   ", "the a of"] {
        let answer = docs(&DocsInput::new(query));
        assert!(!answer.ok);
        assert_eq!(answer.code.as_deref(), Some("SH0001"), "{query:?}");
        assert_eq!(answer.exit_code(), 2);
    }
    let said = docs(&DocsInput::new("the")).error.unwrap();
    assert!(said.contains("only common words"), "{said}");
}

#[test]
fn nothing_matched_passes_with_a_limit_line() {
    let answer = docs(&DocsInput::new("zzzqqq"));
    assert!(answer.ok);
    let output = answer.result.as_ref().unwrap();
    assert!(output.hits.is_empty() && output.how.is_empty());
    assert_eq!(answer.limits, [NOTHING]);
}

#[test]
fn the_limit_is_clamped_from_1_to_50() {
    assert_eq!(with_limit("screen", 0), 1);
    assert_eq!(with_limit("screen", 1), 1);
    assert_eq!(with_limit("screen", 50), 50);
    assert_eq!(with_limit("screen", 51), 50);
    assert_eq!(with_limit("screen", 500), 50);
    assert_eq!(asked("screen").hits.len(), DEFAULT_LIMIT as usize);
}

#[test]
fn kinds_keep_only_their_hits() {
    let mut input = DocsInput::new("screen key");
    input.kinds = Some(vec![HitKind::Skill]);
    let output = docs(&input).result.unwrap();
    assert_ne!(output.hits.len(), 0);
    assert!(output.hits.iter().all(|hit| hit.kind == HitKind::Skill));
    assert!(output.how.iter().all(|row| row.kind == HitKind::Skill));
}

#[test]
fn the_same_query_answers_the_same_bytes() {
    let once = docs(&DocsInput::new("save the game")).to_json();
    let again = docs(&DocsInput::new("save the game")).to_json();
    assert_eq!(once, again);
    let fresh = search(
        &DocsInput::new("save the game"),
        &Index::new(corpus::chunks()),
        &rows(),
    );
    assert_eq!(fresh.to_json(), once);
}

#[test]
fn every_how_do_i_row_goes_to_a_chunk_of_the_corpus() {
    let rows = rows_of(HOW_DO_I).unwrap();
    assert!(rows.len() >= 30, "{} rows", rows.len());
    let chunks = &Index::embedded().chunks;
    for row in &rows {
        assert!(
            target(&row.go, chunks).is_some(),
            "site/data/how-do-i.toml: `{}` names no section the docs hold",
            row.go
        );
        assert!(!row.ask.is_empty(), "{}: no task", row.go);
        for phrase in &row.ask {
            assert!(
                !tokens(phrase).is_empty(),
                "{}: `{phrase}` is only common words",
                row.go
            );
        }
    }
    assert!(rows_of("[[row]]\nask = 1\n").is_err());
    assert!(target("nowhere/x", chunks).is_none());
    assert!(target("learn/book/actions.md#no-such-heading", chunks).is_none());
    assert!(target("learn/book/actions.md", chunks).is_some());
}

#[test]
fn an_excerpt_is_cut_on_a_characters_boundary() {
    let text = format!("{}é and more", "a".repeat(EXCERPT - 1));
    let cut = excerpt(&text, 0);
    assert_eq!(cut, format!("{}…", "a".repeat(EXCERPT - 1)));
    assert_eq!(excerpt("short", 0), "short");
    assert_eq!(excerpt("ab", 1), "…b");
    assert_eq!(excerpt("é", 1), "é");
}

#[test]
fn an_excerpt_starts_where_the_rarest_term_is() {
    let early = "the word here\n";
    let late = format!("{}\nlater the name_x is here\n", "filler ".repeat(80));
    let idf = |t: &str| if t == "name_x" { 2.0 } else { 1.0 };
    let terms = tokens("name_x word");
    assert_eq!(excerpt_start(early, &terms, idf), 0);
    let from = excerpt_start(&late, &terms, idf);
    assert!(
        late[from..].starts_with("later the name_x"),
        "{}",
        &late[from..]
    );
    assert_eq!(excerpt_start("nothing", &terms, idf), 0);
}

#[test]
fn its_arguments_are_words_then_flags_anywhere() {
    let args = |list: &[&str]| list.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>();
    let input = parse_args(&args(&[
        "play",
        "--limit",
        "3",
        "a",
        "sound",
        "--kind",
        "learn,skill",
    ]))
    .unwrap();
    assert_eq!(input.query, "play a sound");
    assert_eq!(input.limit, Some(3));
    assert_eq!(input.kinds, Some(vec![HitKind::Learn, HitKind::Skill]));
    let again = parse_args(&args(&["x", "--kind", "code", "--kind", "example"])).unwrap();
    assert_eq!(again.kinds, Some(vec![HitKind::Code, HitKind::Example]));
    assert_eq!(parse_args(&args(&[""])).unwrap().query, "");
    for wrong in [
        &[][..],
        &["x", "--limit"],
        &["x", "--limit", "-1"],
        &["x", "--kind"],
        &["x", "--kind", "api"],
        &["x", "--bogus"],
    ] {
        assert!(parse_args(&args(wrong)).is_err(), "{wrong:?}");
    }
    assert!(usage("shuyi").starts_with("usage: shuyi docs"));
}

#[test]
fn for_people_it_lists_the_rows_then_the_hits() {
    let output = asked("play a sound");
    let said = text(&output);
    assert!(
        said.starts_with("how do I play a sound: learn/book/games-and-screens.md#the-context\n"),
        "{said}"
    );
    assert!(said.contains("\n  site/content/learn/"), "{said}");
    assert_eq!(text(&DocsOutput::default()), "");
}

/// A small corpus for the property: chunks of a few words each.
fn small_index() -> Index {
    let words = [
        "sound", "screen", "key", "save", "map", "tile", "test", "rule",
    ];
    let chunks = (0..40u32)
        .map(|n| Chunk {
            kind: HitKind::Learn,
            title: words[(n % 8) as usize].to_owned(),
            path: format!("p{}.md", n % 7),
            anchor: None,
            url: None,
            position: n,
            text: (0..=n % 5)
                .map(|k| words[((n + k * 3) % 8) as usize])
                .collect::<Vec<_>>()
                .join(" "),
        })
        .collect();
    Index::new(chunks)
}

proptest! {
    #[test]
    fn hits_are_sorted_and_at_most_the_limit(
        query in proptest::collection::vec("(sound|screen|key|save|map|tile|test|rule|zz)", 1..5),
        limit in 0u32..60,
    ) {
        let index = small_index();
        let mut input = DocsInput::new(query.join(" "));
        input.limit = Some(limit);
        let output = search(&input, &index, &[]).result.unwrap();
        prop_assert!(output.hits.len() <= limit.clamp(1, MAX_LIMIT) as usize);
        for pair in output.hits.windows(2) {
            let (a, b) = (&pair[0], &pair[1]);
            prop_assert!(a.score > b.score || (a.score == b.score && a.path <= b.path));
        }
    }
}

#[test]
fn the_scores_are_the_same_numbers_on_every_machine() {
    // A fixed corpus and query: CI runs this on Linux and Windows, so a
    // float that rounds otherwise on one of them fails here.
    let index = small_index();
    let output = search(&DocsInput::new("sound tile rule"), &index, &[])
        .result
        .unwrap();
    let scores: Vec<(&str, u64)> = output
        .hits
        .iter()
        .map(|hit| (hit.path.as_str(), hit.score))
        .collect();
    assert_eq!(
        scores,
        [
            ("p2.md", 3264),
            ("p4.md", 3080),
            ("p0.md", 2656),
            ("p4.md", 2508),
            ("p0.md", 2501),
            ("p2.md", 2501),
            ("p6.md", 2365),
            ("p1.md", 2244),
        ]
    );
}
