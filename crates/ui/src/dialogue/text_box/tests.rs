//! The text box's pages and typing.

use super::*;

/// Ten words of five letters: two lines of 17 and one of 23 at width 23.
const TEXT: &str = "aaaaa bbbbb ccccc ddddd eeeee fffff ggggg hhhhh iiiii jjjjj";

#[test]
fn text_is_wrapped_into_pages_of_so_many_lines() {
    let mut b = TextBox::new(TEXT, 11, 2);
    assert_eq!(b.page_lines(), ["aaaaa bbbbb", "ccccc ddddd"]);
    assert!(b.next_page());
    assert_eq!(b.page_lines(), ["eeeee fffff", "ggggg hhhhh"]);
    assert!(b.next_page());
    assert_eq!(b.page_lines(), ["iiiii jjjjj"]);
    assert!(!b.next_page(), "the last page");
    assert_eq!(b.page_lines(), ["iiiii jjjjj"]);
    // No text is one empty page; a page of no lines is a page of one.
    let mut empty = TextBox::new("", 11, 2);
    let found = empty.page_lines();
    assert!(found.is_empty(), "{found:?}");
    assert!(empty.is_revealed());
    assert!(!empty.next_page());
    let mut one = TextBox::new("ab cd", 2, 0);
    assert_eq!(one.page_lines(), ["ab"]);
    assert!(one.next_page());
    assert_eq!(one.page_lines(), ["cd"]);
}

#[test]
fn a_page_types_out_at_the_speed_and_waits_once_revealed() {
    let mut b = TextBox::new("hello there", 40, 3);
    assert_eq!((b.shown(), b.progress()), (0, 0.0));
    assert!(!b.is_revealed());
    b.type_on(0.25, Some(10.0));
    assert_eq!(b.shown(), 2);
    assert!((b.progress() - 2.5).abs() < 1e-6);
    b.type_on(10.0, Some(10.0));
    assert_eq!(b.shown(), 11, "never past the page");
    assert!(b.is_revealed());
    // The marker blinks while it waits.
    assert!(b.marker_lit());
    b.type_on(0.6, Some(10.0));
    assert!(!b.marker_lit());
    b.type_on(0.5, Some(10.0));
    assert!(b.marker_lit());
    // Instant: the whole page at once.
    let mut b = TextBox::new("hello", 40, 3);
    b.type_on(0.0, None);
    assert!(b.is_revealed());
    // Revealed at once by a key.
    let mut b = TextBox::new("hello", 40, 3);
    b.reveal();
    assert_eq!(b.shown(), 5);
}

#[test]
fn holding_the_key_types_faster_after_a_moment() {
    let typed = |held_for: f32| {
        let mut b = TextBox::new(&"x".repeat(200), 300, 3);
        b.hold(true, held_for);
        b.type_on(0.1, Some(10.0));
        b.progress()
    };
    // Under the delay: the plain speed.
    assert!((typed(0.1) - 1.0).abs() < 1e-4);
    // Long past it: the whole frame fast.
    assert!((typed(1.0) - 1.0 * FAST_FORWARD).abs() < 1e-4);
    // Part of the frame past the delay: that part fast.
    let part = typed(FAST_FORWARD_DELAY + 0.05);
    assert!(
        (part - (0.1 + 0.05 * (FAST_FORWARD - 1.0)) * 10.0).abs() < 1e-4,
        "{part}"
    );
    // Let go, or released, the count starts over.
    let mut b = TextBox::new(&"x".repeat(200), 300, 3);
    b.hold(true, 1.0);
    b.hold(false, 0.1);
    b.type_on(0.1, Some(10.0));
    assert!((b.progress() - 1.0).abs() < 1e-4);
    b.hold(true, 1.0);
    b.release();
    b.type_on(0.1, Some(10.0));
    assert!((b.progress() - 2.0).abs() < 1e-4);
    // A page turned starts over too.
    let mut b = TextBox::new("ab cd", 2, 1);
    b.hold(true, 1.0);
    b.reveal();
    b.type_on(0.75, Some(10.0));
    assert!(b.next_page());
    assert_eq!(b.shown(), 0);
    assert!(b.marker_lit(), "not waiting yet");
    b.type_on(0.1, Some(10.0));
    assert!((b.progress() - 1.0).abs() < 1e-4);
}

#[test]
fn the_last_page_shows_revealed_for_a_choice() {
    let mut b = TextBox::new(TEXT, 11, 2);
    b.to_last_page();
    assert_eq!(b.page_lines(), ["iiiii jjjjj"]);
    assert!(b.is_revealed());
    let mut b = TextBox::new("", 11, 2);
    b.to_last_page();
    let found = b.page_lines();
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn a_view_reveals_its_lines_from_the_start() {
    let view = TextView {
        narration: false,
        lines: vec!["abc".into(), "defg".into()],
        shown: 5,
        choices: None,
        skip: None,
        waiting: None,
    };
    assert_eq!(view.revealed(), ["abc", "de"]);
    let none = TextView {
        shown: 0,
        ..view.clone()
    };
    assert_eq!(none.revealed(), ["", ""]);
    let all = TextView { shown: 99, ..view };
    assert_eq!(all.revealed(), ["abc", "defg"]);
}

#[test]
fn the_marker_is_out_from_half_a_second_on() {
    let mut b = TextBox::new("a", 40, 3);
    b.reveal();
    b.type_on(BLINK_S, Some(10.0));
    assert!(!b.marker_lit(), "out at exactly half a second");
    b.type_on(BLINK_S, Some(10.0));
    assert!(b.marker_lit(), "lit again at a second");
}

#[test]
fn confirm_reveals_the_page_then_turns_it_then_is_done() {
    let mut b = TextBox::new(TEXT, 11, 2);
    b.type_on(0.1, Some(10.0));
    assert_eq!(b.confirm(), Step::Revealed, "a page being typed");
    assert!(b.is_revealed());
    assert_eq!(
        b.confirm(),
        Step::NextPage,
        "a revealed page, another after"
    );
    assert_eq!(b.page_lines(), ["eeeee fffff", "ggggg hhhhh"]);
    assert_eq!(b.shown(), 0, "the next page types from its start");
    assert_eq!(b.confirm(), Step::Revealed);
    assert_eq!(b.confirm(), Step::NextPage);
    assert_eq!(b.confirm(), Step::Revealed);
    assert_eq!(b.confirm(), Step::Done, "the last page");
    assert_eq!(b.confirm(), Step::Done, "and stays so");
    assert_eq!(b.page_lines(), ["iiiii jjjjj"]);
    assert!(b.is_revealed());
    // An empty box is revealed at once: done.
    assert_eq!(TextBox::new("", 11, 2).confirm(), Step::Done);
}

#[test]
fn the_views_constructors_give_the_literals() {
    assert_eq!(
        TextView::new(vec!["Hello".to_owned()], 3),
        TextView {
            narration: false,
            lines: vec!["Hello".to_owned()],
            shown: 3,
            choices: None,
            skip: None,
            waiting: None,
        }
    );
    assert_eq!(
        WaitView::new("Enter", true),
        WaitView {
            key: "Enter".to_owned(),
            log_hint: None,
            marker: true,
        }
    );
}
