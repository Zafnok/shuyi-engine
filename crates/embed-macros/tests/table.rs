//! `table!` on a folder: the table it writes, however the folder is named.

use shuyi_embed_macros::table;

/// What `tests/fixture` holds, sorted by path.
const FIXTURE: &[(&str, &[u8])] = &[("data/deep.txt", b"deep\n"), ("top.txt", b"top\n")];

#[test]
fn a_folder_named_from_the_crates_folder_is_its_table() {
    const TABLE: &[(&str, &[u8])] = table!("$CARGO_MANIFEST_DIR/tests/fixture");
    assert_eq!(TABLE, FIXTURE);
}

#[test]
fn a_relative_folder_is_the_crates() {
    const TABLE: &[(&str, &[u8])] = table!("tests/fixture");
    assert_eq!(TABLE, FIXTURE);
}

#[test]
fn a_raw_string_names_a_folder_too() {
    const TABLE: &[(&str, &[u8])] = table!(r"tests/fixture");
    assert_eq!(TABLE, FIXTURE);
}

/// `table!` behind a `macro_rules!` fragment, as `shuyi_assets::files!` may
/// hand it on: a literal or an expression arrives in a group of its own.
macro_rules! through_literal {
    ($folder:literal) => {
        table!($folder)
    };
}

macro_rules! through_expr {
    ($folder:expr) => {
        table!($folder)
    };
}

#[test]
fn a_folder_handed_on_by_another_macro_is_read() {
    const BY_LITERAL: &[(&str, &[u8])] = through_literal!("tests/fixture");
    const BY_EXPR: &[(&str, &[u8])] = through_expr!("tests/fixture");
    assert_eq!(BY_LITERAL, FIXTURE);
    assert_eq!(BY_EXPR, FIXTURE);
}
