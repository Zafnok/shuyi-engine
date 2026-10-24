//! Two RON texts merged: what is added, where, with which comments, and
//! what fails.

use super::*;

#[test]
fn a_struct_gains_the_fields_it_lacks_with_their_comments() {
    let base = "// The base.\n(\n    a: 1,\n\n    // B.\n    b: [\"x\"],\n)\n";
    let other = "// The part.\n(\n    // C, on\n    // two lines.\n    c: (\n        d: 2,\n    ),\n    b: [\"y\"],\n)\n";
    let merged = merge(base, other).unwrap();
    assert_eq!(
        merged,
        "// The base.\n(\n    a: 1,\n\n    // B.\n    b: [\"x\", \"y\"],\n\n    // C, on\n    \
         // two lines.\n    c: (\n        d: 2,\n    ),\n)\n"
    );
    // What comes out reads as RON.
    let _: ron::Value = ron::from_str(&merged).unwrap();
}

#[test]
fn a_map_gains_its_entries_and_a_list_only_the_items_it_lacks() {
    let base = "{\n    \"a\": \"1\",\n    \"b\": \"2\"\n}";
    let other = "{\n    \"b\": \"2\",\n    // C.\n    \"c\": \"3\",\n}";
    assert_eq!(
        merge(base, other).unwrap(),
        "{\n    \"a\": \"1\",\n    \"b\": \"2\",\n    // C.\n    \"c\": \"3\",\n}"
    );
    let base = "(list: [\"a\", \"b\"])";
    let other = "(list: [\"b\", \"c\", \"c\"])";
    assert_eq!(merge(base, other).unwrap(), "(list: [\"a\", \"b\", \"c\"])");
    assert_eq!(merge("[]", "[1, 2]").unwrap(), "[1, 2]");
    assert_eq!(merge("{}", "{\"k\": 1}").unwrap(), "{\"k\": 1}");
    assert_eq!(merge("(a: 1)", "()").unwrap(), "(a: 1)");
}

#[test]
fn deep_structs_merge_and_the_same_value_twice_is_fine() {
    let base = "Config(\n    keys: (\n        rust: [\"crates/game/src\"],\n        hint: \"h\",\n    ),\n)\n";
    let other =
        "(\n    keys: (\n        rust: [\"crates/rules/src\"],\n        hint: \"h\",\n    ),\n)\n";
    let merged = merge(base, other).unwrap();
    assert!(
        merged.contains("rust: [\"crates/game/src\", \"crates/rules/src\"],"),
        "{merged}"
    );
    assert_eq!(merge(base, base).unwrap(), base);
}

#[test]
fn a_value_that_differs_fails_with_its_path() {
    let (at, message) = merge("(a: (b: 1))", "(a: (b: 2))").unwrap_err();
    assert_eq!(at, "a.b");
    assert_eq!(message, "`1` in one, `2` in another");
    let (at, _) = merge("{\"title.help\": \"x\"}", "{\"title.help\": \"y\"}").unwrap_err();
    assert_eq!(at, "[\"title.help\"]");
    // Tuples, and structs of other names, must be the same.
    assert!(merge("(f: (1, 2))", "(f: (1, 3))").is_err());
    assert!(merge("A(x: 1)", "B(y: 1)").is_err());
    assert!(merge("[1]", "{}").is_err());
}

#[test]
fn strings_and_comments_with_brackets_in_them_are_read_whole() {
    let base = "(\n    // a ) comment ] with { brackets\n    s: \"a ) string, \\\" with: (\",\n    /* a\n    /* nested */ block */\n    r: r##\"raw \"# here\"##,\n    c: ')',\n)";
    let other = "(t: Some(\"x\"), s: \"a ) string, \\\" with: (\")";
    let merged = merge(base, other).unwrap();
    assert!(merged.ends_with("    t: Some(\"x\"),\n)"), "{merged}");
}

#[test]
fn text_that_isnt_ron_fails_saying_where() {
    for bad in [
        "(a: 1",
        "(a 1)",
        "[1 2]",
        "{\"a\" 1}",
        "\"open",
        "(a: 1) x",
        "",
        "/* x",
    ] {
        let (at, message) = merge(bad, "()").unwrap_err();
        assert_eq!(at, "", "{bad:?}");
        assert!(message.starts_with("not RON: "), "{bad:?}: {message}");
    }
    assert!(merge("()", "(").unwrap_err().1.contains("at 1:2"));
}

#[test]
fn an_attribute_before_the_value_is_skipped() {
    let merged = merge("#![enable(implicit_some)]\n(a: 1)", "(b: 2)").unwrap();
    assert_eq!(merged, "#![enable(implicit_some)]\n(a: 1, b: 2)");
}

#[test]
fn a_merge_never_panics_on_any_text() {
    use proptest::prelude::*;
    let pieces = prop::collection::vec(
        prop::sample::select(vec![
            "(", ")", "[", "]", "{", "}", ",", ":", "\"", "'", "a", "1", " ", "\n", "//", "/*",
            "*/", "r#", "#!", "é",
        ]),
        0..40,
    );
    proptest!(|(a in pieces.clone(), b in pieces)| {
        let _ = merge(&a.concat(), &b.concat());
    });
}
