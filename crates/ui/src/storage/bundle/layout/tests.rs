use super::*;
use proptest::prelude::*;

#[test]
fn a_struct_is_laid_out_over_lines_four_spaces_to_a_level() {
    assert_eq!(
        layout(
            "(version:2,lead:(name:\"Peter\"),tags:[a,b],empty:(),m:{})",
            0
        ),
        "(\n    version: 2,\n    lead: (\n        name: \"Peter\"\n    ),\n    tags: [\n        a,\n        b\n    ],\n    empty: (),\n    m: {}\n)"
    );
}

#[test]
fn a_value_is_laid_out_at_the_level_it_sits_at() {
    assert_eq!(layout("Some(3)", 2), "Some(\n            3\n        )");
    assert_eq!(layout("[1,]", 1), "[\n        1,\n    ]");
}

#[test]
fn a_map_gets_a_space_after_each_key() {
    assert_eq!(
        layout("{\"a\":1,\"b\":[]}", 0),
        "{\n    \"a\": 1,\n    \"b\": []\n}"
    );
}

#[test]
fn one_token_is_left_as_it_is() {
    for one in ["slot_07", "Vim", "3", "-1.5e3", "\"x\"", "'c'", "true"] {
        assert_eq!(layout(one, 2), one);
        assert_eq!(compact(one), one);
    }
}

#[test]
fn strings_keep_their_brackets_commas_comments_and_escapes() {
    let value = r#"(name:"a, (b) // c\n \" /* d",c:'(',d:'\'',e:b"x,y")"#;
    let laid = layout(value, 0);
    assert_eq!(
        laid,
        "(\n    name: \"a, (b) // c\\n \\\" /* d\",\n    c: '(',\n    d: '\\'',\n    e: b\"x,y\"\n)"
    );
    assert_eq!(compact(&laid), value);
}

#[test]
fn raw_strings_are_kept_whole() {
    let value = r###"(a:r"x,(",b:r#"say "hi", (ok)"#,c:br##"a"#b"##,d:r#ident)"###;
    let laid = layout(value, 0);
    assert!(laid.contains(r#"a: r"x,(","#), "{laid}");
    assert!(laid.contains(r##"b: r#"say "hi", (ok)"#,"##), "{laid}");
    assert!(laid.contains(r###"c: br##"a"#b"##,"###), "{laid}");
    assert!(laid.contains("d: r#ident\n"), "{laid}");
    assert_eq!(compact(&laid), value);
}

#[test]
fn comments_and_whitespace_are_dropped_and_words_stay_apart() {
    assert_eq!(
        compact("( a : 1 , // one\n b: /* two /* nested */ */ 2 )\r\n"),
        "(a:1,b:2)"
    );
    assert_eq!(compact("a /* x */ b//c"), "a b");
    assert_eq!(compact("a/b"), "a/b");
    assert_eq!(compact("  \u{2028} "), "");
}

#[test]
fn unclosed_tokens_run_to_the_end() {
    assert_eq!(compact("(a:\"b, c"), "(a:\"b, c");
    assert_eq!(compact("(a:r#\"b, c"), "(a:r#\"b, c");
    assert_eq!(compact("(a:'b"), "(a:'b");
    assert_eq!(compact("(a /* b"), "(a");
    assert_eq!(compact("(a // b"), "(a");
    assert_eq!(compact("\"\\"), "\"\\");
}

#[test]
fn a_stray_closing_bracket_stays_at_the_left() {
    assert_eq!(layout(")", 0), ")");
    assert_eq!(layout("(]", 0), "(\n]");
}

/// A compact RON value, as `ron::to_string` writes one, with raw strings
/// and chars too.
fn ron_value() -> impl Strategy<Value = String> {
    let leaf = prop_oneof![
        any::<i64>().prop_map(|n| n.to_string()),
        "[A-Za-z_][A-Za-z0-9_]{0,6}",
        any::<String>().prop_map(|s| ron::to_string(&s).unwrap()),
        any::<char>().prop_map(|c| ron::to_string(&c).unwrap()),
        "[^#]{0,10}".prop_map(|s| format!("r#\"{s}\"#")),
    ];
    leaf.prop_recursive(4, 32, 5, |inner| {
        prop_oneof![
            prop::collection::vec(inner.clone(), 0..5).prop_map(|v| format!("({})", v.join(","))),
            prop::collection::vec(inner.clone(), 0..5).prop_map(|v| format!("[{}]", v.join(","))),
            prop::collection::vec(("[a-z_]{1,6}", inner.clone()), 0..5).prop_map(|v| {
                let fields: Vec<String> = v.into_iter().map(|(k, v)| format!("{k}:{v}")).collect();
                format!("Named({})", fields.join(","))
            }),
            prop::collection::vec((inner.clone(), inner), 0..4).prop_map(|v| {
                let pairs: Vec<String> = v.into_iter().map(|(k, v)| format!("{k}:{v}")).collect();
                format!("{{{}}}", pairs.join(","))
            }),
        ]
    })
}

proptest! {
    #[test]
    fn laying_out_is_undone_by_compacting_and_changes_nothing_twice(
        value in ron_value(),
        level in 0_usize..4,
    ) {
        prop_assert_eq!(compact(&value), value.clone());
        let laid = layout(&value, level);
        prop_assert_eq!(compact(&laid), value);
        prop_assert_eq!(layout(&laid, level), laid);
    }

    #[test]
    fn any_text_compacts_and_lays_out_the_same_twice(text in "\\PC{0,60}", level in 0_usize..3) {
        let compacted = compact(&text);
        prop_assert_eq!(compact(&compacted), compacted.clone());
        let laid = layout(&text, level);
        prop_assert_eq!(compact(&laid), compacted);
        prop_assert_eq!(layout(&laid, level), laid);
    }

    #[test]
    fn a_laid_out_value_reads_as_the_same_ron(value in ron_value()) {
        let compact_value: Option<ron::Value> = ron::from_str(&value).ok();
        let laid_value: Option<ron::Value> = ron::from_str(&layout(&value, 2)).ok();
        prop_assert_eq!(compact_value, laid_value);
    }
}
