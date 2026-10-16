//! Tests of the names table, name tokens and the literal-name check.

use alloc::borrow::Cow;

use proptest::prelude::*;

use super::*;

/// A table from `(id, name)` pairs.
fn table(entries: &[(&str, &str)]) -> Names {
    Names {
        names: entries
            .iter()
            .map(|&(id, name)| (id.to_owned(), name.to_owned()))
            .collect(),
    }
}

fn story() -> Names {
    table(&[
        ("king", "Osgar"),
        ("place.fen", "the Fenland"),
        ("faction.guild", "the Guild of Lamps"),
        ("term.vow", "a vow"),
        ("outcast", "Tam"),
    ])
}

/// Every error message for names `src`, as `file:line: message`.
fn errors(src: &str) -> Vec<String> {
    from_source("n.ron", src)
        .err()
        .unwrap_or_default()
        .iter()
        .map(ToString::to_string)
        .collect()
}

#[test]
fn parses_name_tokens() {
    assert_eq!(
        name_token("n:place.fen"),
        Some(NameToken {
            id: "place.fen",
            capital: false
        })
    );
    assert_eq!(
        name_token("N:king"),
        Some(NameToken {
            id: "king",
            capital: true
        })
    );
    assert_eq!(
        name_token("n:"),
        Some(NameToken {
            id: "",
            capital: false
        })
    );
    assert_eq!(name_token("hero"), None);
    assert_eq!(name_token("they"), None);
    assert_eq!(name_token("x:king"), None);
    assert!(is_name_id("place.old_mill"));
    assert!(is_name_id("a1"));
    assert!(!is_name_id(""));
    assert!(!is_name_id("King"));
    assert!(!is_name_id("king-2"));
}

#[test]
fn substitutes_every_use() {
    let text = "{n:king} rides. {n:king} never stops.";
    assert_eq!(story().substitute(text), "Osgar rides. Osgar never stops.");
    // A rename is one entry.
    let renamed = table(&[("king", "Brun")]);
    assert_eq!(renamed.substitute(text), "Brun rides. Brun never stops.");
}

#[test]
fn capital_token_capitalises() {
    let names = story();
    assert_eq!(
        names.substitute("{N:place.fen} is cold. We left {n:place.fen}."),
        "The Fenland is cold. We left the Fenland."
    );
    assert_eq!(names.substitute("{N:king}"), "Osgar");
}

#[test]
fn leaves_other_tokens_and_unknown_ids() {
    let names = story();
    assert_eq!(
        names.substitute("{hero} and {n:nobody}, {they} {n:king} {"),
        "{hero} and {n:nobody}, {they} Osgar {"
    );
    assert!(matches!(names.substitute("No tokens."), Cow::Borrowed(_)));
    assert!(matches!(names.substitute("{n:king}"), Cow::Owned(_)));
    assert_eq!(names.substitute(""), "");
}

#[test]
fn longest_is_the_longest_name() {
    assert_eq!(story().longest(), "the Guild of Lamps".len());
    assert_eq!(Names::default().longest(), 0);
    // Counted in cells, not bytes.
    assert_eq!(table(&[("a", "né"), ("b", "x")]).longest(), 2);
}

#[test]
fn literal_form_drops_articles_and_skips_lowercase() {
    assert_eq!(literal_form("the Fenland"), Some("Fenland"));
    assert_eq!(literal_form("an Echo"), Some("Echo"));
    assert_eq!(literal_form("a Vow"), Some("Vow"));
    assert_eq!(literal_form("Osgar"), Some("Osgar"));
    assert_eq!(literal_form("Ila, the Weaver"), Some("Ila, the Weaver"));
    assert_eq!(literal_form("a vow"), None);
    assert_eq!(literal_form("breath"), None);
}

#[test]
fn literal_check_hits() {
    let names = story();
    assert_eq!(
        names.literal_in("Long live Osgar!"),
        Some(("king", "Osgar"))
    );
    assert_eq!(
        names.literal_in("Back to Fenland."),
        Some(("place.fen", "Fenland"))
    );
    assert_eq!(
        names.literal_in("For the Guild of Lamps, {hero}."),
        Some(("faction.guild", "Guild of Lamps"))
    );
    assert_eq!(names.literal_in("\"Tam\"."), Some(("outcast", "Tam")));
}

#[test]
fn literal_check_reports_the_longest_name() {
    let names = table(&[
        ("family.brook", "Brook"),
        ("smith", "Ada Brook"),
        ("smith.first", "Ada"),
        ("rival.first", "Cole"),
        ("sister.first", "Wynn"),
    ]);
    // A full name is itself, not the short forms inside it.
    assert_eq!(
        names.literal_in("Ada Brook rode out."),
        Some(("smith", "Ada Brook"))
    );
    assert_eq!(names.literal_in("Ada, wait."), Some(("smith.first", "Ada")));
    assert_eq!(
        names.literal_in("A Brook never runs."),
        Some(("family.brook", "Brook"))
    );
    // Of equally long names, the first by id.
    assert_eq!(
        names.literal_in("Wynn and Cole."),
        Some(("rival.first", "Cole"))
    );
}

#[test]
fn literal_check_misses() {
    let names = story();
    // Tokens, other case, and ordinary words aren't hits.
    assert_eq!(names.literal_in("{n:king} and {N:place.fen}."), None);
    assert_eq!(names.literal_in("A tam o'shanter. osgar."), None);
    assert_eq!(names.literal_in("I swore a vow."), None);
}

#[test]
fn literal_check_is_whole_word() {
    let names = story();
    assert_eq!(names.literal_in("Osgars and Osgar2 and XOsgar"), None);
    assert_eq!(names.literal_in("Tamr. Tam's"), Some(("outcast", "Tam")));
    assert_eq!(names.literal_in("_Tam"), Some(("outcast", "Tam")));
}

#[test]
fn valid_source_loads() {
    let t = from_source(
        "n.ron",
        "{\n    \"king\": \"Osgar\",\n    \"place.a_b\": \"A\",\n}",
    );
    assert_eq!(t, Ok(table(&[("king", "Osgar"), ("place.a_b", "A")])));
}

#[test]
fn bad_ids_are_reported() {
    assert_eq!(
        errors("{\n    \"King\": \"Osgar\",\n}"),
        ["n.ron:2: \"King\" is not a valid name id; use lowercase letters, digits, _ and ."]
    );
}

#[test]
fn duplicate_ids_are_reported() {
    assert_eq!(
        errors("{\n    \"king\": \"Osgar\",\n    \"king\": \"Brun\",\n}"),
        ["n.ron:2: name id \"king\" is listed more than once"]
    );
}

#[test]
fn bad_values_are_reported() {
    assert_eq!(
        errors(
            "{\n    \"a\": \" \",\n    \"b\": \"{n:a}\",\n    \"c\": \"Zo\u{eb}\",\n    \"d\": \"x}\",\n}"
        ),
        [
            "n.ron:2: name \"a\" is empty",
            "n.ron:3: name \"b\" contains a brace; names can't hold tokens",
            "n.ron:4: name \"c\": unsupported character '\u{eb}'; dialogue text is plain ASCII",
            "n.ron:5: name \"d\" contains a brace; names can't hold tokens",
        ]
    );
    // An error with no line to point at is still reported.
    let no_line = from_source("n.ron", "{\"King\": \"X\"}")
        .err()
        .unwrap_or_default();
    assert_eq!(no_line.len(), 1);
    assert_eq!(no_line[0].line, Some(1));
    let syntax = errors("{");
    assert_eq!(syntax.len(), 1);
}

#[test]
fn duplicate_values_are_reported() {
    assert_eq!(
        errors("{\n    \"a\": \"Tor\",\n    \"b\": \"Tor\",\n}"),
        ["n.ron:3: \"Tor\" is the name of both \"a\" and \"b\"; names must be unique"]
    );
}

/// The token for name `i` of the property tests' tables.
fn marker(i: usize) -> String {
    format!("q{i}q")
}

/// `text` with every table name turned back into its token (the property
/// tables' names are `q<i>q`, capitalised `Q<i>q`, which plain text can't
/// contain).
fn retokenise(text: &str, count: usize) -> String {
    let mut out = text.to_owned();
    for i in 0..count {
        out = out
            .replace(&marker(i), &format!("{{n:id{i}}}"))
            .replace(&format!("Q{i}q"), &format!("{{N:id{i}}}"));
    }
    out
}

proptest! {
    /// Substituting then turning names back into tokens gives the text
    /// back, and leaves no name token.
    #[test]
    fn substitution_round_trips(
        count in 1..6usize,
        pieces in proptest::collection::vec(
            prop_oneof![
                "[a-pr-z ,.!]{0,6}".prop_map(|s| (s, None)),
                (0..6usize, any::<bool>()).prop_map(|(i, cap)| (String::new(), Some((i, cap)))),
                Just(("{hero}".to_owned(), None)),
            ],
            0..10,
        ),
    ) {
        let names = Names {
            names: (0..count).map(|i| (format!("id{i}"), marker(i))).collect(),
        };
        let text: String = pieces
            .iter()
            .map(|(s, token)| match token {
                Some((i, cap)) => format!("{{{}:id{}}}", if *cap { 'N' } else { 'n' }, i % count),
                None => s.clone(),
            })
            .collect();
        let out = names.substitute(&text);
        prop_assert!(!out.contains("{n:") && !out.contains("{N:"), "tokens left in {}", out);
        prop_assert_eq!(retokenise(&out, count), text);
    }
}

/// `{N:…}` capitalises a name that starts with a Latin letter, accented
/// or not, and leaves any other as it is (a pack's names).
#[test]
fn only_a_latin_first_letter_is_capitalised() {
    let names = table(&[
        ("plain", "the King"),
        ("accent", "étoile du nord"),
        ("kana", "おうさま"),
        ("greek", "βασιλιάς"),
        ("digit", "7th legion"),
    ]);
    let shown = |token: &str| names.substitute(token).into_owned();
    assert_eq!(shown("{N:plain}"), "The King");
    assert_eq!(shown("{n:plain}"), "the King");
    assert_eq!(shown("{N:accent}"), "Étoile du nord");
    assert_eq!(shown("{N:kana}"), "おうさま");
    assert_eq!(shown("{N:greek}"), "βασιλιάς");
    assert_eq!(shown("{N:digit}"), "7th legion");
    assert!(is_latin_letter('a') && is_latin_letter('Z') && is_latin_letter('ß'));
    assert!(is_latin_letter('À') && is_latin_letter('ɏ'));
    assert!(!is_latin_letter('×') && !is_latin_letter('÷'));
    assert!(!is_latin_letter('1') && !is_latin_letter('ɐ') && !is_latin_letter('¿'));
    assert_eq!(capitalise(""), "");
}

/// Every error message for adding names `src` (file `t.ron`) to the
/// story's (file `n.ron`).
fn part_errors(src: &str) -> Vec<String> {
    with_part(story(), "n.ron", "t.ron", src)
        .err()
        .unwrap_or_default()
        .iter()
        .map(ToString::to_string)
        .collect()
}

/// A second file's names are added to the story's, by id.
#[test]
fn a_part_adds_its_names_by_id() {
    let src = "{\n\"test_a\": \"Test A\",\n\"test_b\": \"Test B\",\n}";
    let names = with_part(story(), "n.ron", "t.ron", src).unwrap_or_default();
    assert_eq!(names.get("test_a"), Some("Test A"));
    assert_eq!(names.get("test_b"), Some("Test B"));
    assert_eq!(names.names.len(), story().names.len() + 2);
    for (id, name) in &story().names {
        assert_eq!(names.get(id), Some(name.as_str()));
    }
    assert_eq!(with_part(story(), "n.ron", "t.ron", "{}"), Ok(story()));
}

#[test]
fn a_part_with_an_id_or_a_name_of_the_story_is_an_error() {
    let src = "{\n\"test_a\": \"Test A\",\n\"king\": \"Other\",\n\"test_c\": \"Tam\",\n}";
    assert_eq!(
        part_errors(src),
        [
            "t.ron:3: name id \"king\" is already in n.ron",
            "t.ron:4: \"Tam\" is the name of \"outcast\" in n.ron too; names must be unique",
        ]
    );
    // The same id with the same name is still a clash.
    assert_eq!(
        part_errors("{\n\"king\": \"Osgar\",\n}"),
        ["t.ron:2: name id \"king\" is already in n.ron"]
    );
}

#[test]
fn a_part_is_checked_like_a_names_file() {
    assert_eq!(
        part_errors("{\n\"Bad Id\": \"X\",\n}"),
        ["t.ron:2: \"Bad Id\" is not a valid name id; use lowercase letters, digits, _ and ."]
    );
    let syntax = part_errors("{");
    assert_eq!(syntax.len(), 1);
    assert!(syntax[0].starts_with("t.ron:1:"), "{syntax:?}");
}
