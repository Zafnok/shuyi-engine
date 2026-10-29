use super::*;

const TABLE: &str = "{\n    \"a\": \"1\",\n    \"b\": \"2\",\n}\n";

/// The report of `golden`, which must fail.
fn report(golden: Golden) -> GoldenReport {
    golden
        .finish()
        .err()
        .unwrap_or_else(|| panic!("expected a report"))
}

/// Rows from `pairs`.
fn rows(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs
        .iter()
        .map(|&(name, value)| (name.to_owned(), value.to_owned()))
        .collect()
}

#[test]
fn a_table_that_matches_passes() {
    let mut golden = Golden::new("t.ron", TABLE);
    golden.check("b", 2);
    golden.check(String::from("a"), "1");
    assert_eq!(golden.finish(), Ok(()));
    // An empty table, checked by nothing.
    assert_eq!(Golden::new("t.ron", "{}").finish(), Ok(()));
}

#[test]
fn a_changed_row_is_reported_as_a_break_with_the_new_table() {
    let mut golden = Golden::new("t.ron", TABLE);
    golden.check("a", 1);
    golden.check("b", 3);
    let report = report(golden);
    assert_eq!(
        report.changed,
        BTreeMap::from([("b".into(), ("2".into(), "3".into()))])
    );
    assert!(report.added.is_empty() && report.removed.is_empty());
    assert_eq!(report.table, table_text(&rows(&[("a", "1"), ("b", "3")])));
    let shown = report.to_string();
    assert!(shown.starts_with("golden table t.ron: 1 changed, 0 added, 0 removed\n"));
    assert!(shown.contains("  changed \"b\": 2 -> 3\n"), "{shown}");
    assert!(
        shown.contains("add a `Breaking:` line naming it"),
        "{shown}"
    );
    assert!(shown.ends_with(
        "--- new golden table: t.ron ---\n{\n    \"a\": \"1\",\n    \"b\": \"3\",\n}\n--- end of golden table ---"
    ));
}

#[test]
fn an_added_row_is_not_a_break() {
    let mut golden = Golden::new("t.ron", TABLE);
    golden.check("a", 1);
    golden.check("b", 2);
    golden.check("c", 3);
    let report = report(golden);
    assert_eq!(report.added, rows(&[("c", "3")]));
    assert!(report.changed.is_empty() && report.removed.is_empty());
    let shown = report.to_string();
    assert!(shown.contains("  added \"c\": 3\n"), "{shown}");
    assert!(shown.contains("new rows are not a break"), "{shown}");
    assert!(!shown.contains("Breaking:"), "{shown}");
}

#[test]
fn a_removed_row_is_a_break() {
    let mut golden = Golden::new("t.ron", TABLE);
    golden.check("a", 1);
    let report = report(golden);
    assert_eq!(report.removed, rows(&[("b", "2")]));
    assert_eq!(report.table, "{\n    \"a\": \"1\",\n}\n");
    let shown = report.to_string();
    assert!(shown.contains("  removed \"b\": 2\n"), "{shown}");
    assert!(
        shown.contains("add a `Breaking:` line naming it"),
        "{shown}"
    );
}

#[test]
fn a_name_checked_twice_fails_and_the_first_value_stands() {
    let mut golden = Golden::new("t.ron", TABLE);
    golden.check("a", 1);
    golden.check("b", 2);
    golden.check("a", 9);
    let report = report(golden);
    assert_eq!(report.twice, ["a"]);
    assert!(report.changed.is_empty());
    let shown = report.to_string();
    assert!(shown.contains("  checked twice \"a\"\n"), "{shown}");
    assert!(shown.contains("a name of its own"), "{shown}");
}

#[test]
fn a_text_that_does_not_parse_is_reported_with_its_line() {
    let mut golden = Golden::new("t.ron", "{\n    \"a\": 1,\n}\n");
    golden.check("a", 1);
    let report = report(golden);
    let why = report.unreadable.clone().unwrap_or_default();
    assert!(why.starts_with("t.ron:2:"), "{why}");
    // Every row counts as added, and the table is written again.
    assert_eq!(report.added, rows(&[("a", "1")]));
    assert!(report.to_string().contains("Write the table again"));
}

#[test]
fn table_text_writes_rows_in_name_order_escaped_and_reads_back() {
    let rows = rows(&[
        ("step 010", "c"),
        ("step 002", "b"),
        ("a \"quoted\" name", "back\\slash"),
    ]);
    let text = table_text(&rows);
    assert_eq!(
        text,
        "{\n    \"a \\\"quoted\\\" name\": \"back\\\\slash\",\n    \"step 002\": \"b\",\n    \"step 010\": \"c\",\n}\n"
    );
    let read: BTreeMap<String, String> = ::ron::from_str(&text).unwrap_or_default();
    assert_eq!(read, rows);
    assert_eq!(table_text(&BTreeMap::new()), "{}\n");
    // What a run checks reads back as the same table.
    let mut golden = Golden::new("t.ron", &text);
    for (name, value) in &rows {
        golden.check(name.clone(), value);
    }
    assert_eq!(golden.finish(), Ok(()));
}

#[test]
#[should_panic(expected = "golden table t.ron: 0 changed, 1 added, 0 removed")]
fn assert_panics_with_the_report() {
    let mut golden = Golden::new("t.ron", "{}");
    golden.check("new", 1);
    golden.assert();
}
