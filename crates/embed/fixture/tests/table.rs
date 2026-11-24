//! The table a build script wrote compiles, and holds the folder's files.

use embed_fixture::{FILES, LEFT_OUT};

#[test]
fn the_table_holds_every_file_sorted_with_its_bytes() {
    let paths: Vec<&str> = FILES.iter().map(|(path, _)| *path).collect();
    assert_eq!(paths, ["data/level-two.ron", "one.txt"]);
    assert_eq!(FILES[0].1, b"level two\n");
    assert_eq!(FILES[1].1, b"first\n");
}

#[test]
fn the_skipped_notes_are_listed_and_not_embedded() {
    assert_eq!(LEFT_OUT, ["data/notes.txt", "notes.txt"]);
}
