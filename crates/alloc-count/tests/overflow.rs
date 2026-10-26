//! The process's table of tag names filled: a test binary of its own, so
//! no other test asks for a tag here.

use shuyi_alloc_count::tags::{self, OVERFLOW};

#[test]
fn the_255th_name_counts_under_overflow_and_is_counted_once() {
    let names: Vec<&'static str> = (0..255)
        .map(|n| format!("name {n}").leak() as &str)
        .collect();
    for (n, name) in names.iter().enumerate().take(254) {
        assert_eq!(usize::from(tags::tag(2, name)), n + 1, "{name}");
    }
    assert_eq!(tags::overflowed(), 0);
    assert_eq!(tags::tag(2, names[254]), OVERFLOW);
    assert_eq!(tags::overflowed(), 1);
    // Asked again, it is the same name: still one.
    assert_eq!(tags::tag(2, names[254]), OVERFLOW);
    assert_eq!(tags::overflowed(), 1);
    // A name given before keeps its tag.
    assert_eq!(tags::tag(2, names[0]), 1);
}
