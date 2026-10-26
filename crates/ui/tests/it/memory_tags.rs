//! Memory tags (ADR-0046 §6) on the Harness, with fake scopes that note
//! what is entered: the frame's phases and a screen's spans, and the same
//! game with the scopes on and off.

use std::cell::{Cell, RefCell};

use proptest::prelude::*;
use shuyi_platform::harness::{FRAME_DT, Harness};
use shuyi_ui::frame::GlyphBuffer;
use shuyi_ui::perf::{MemoryScopes, TagClass};
use shuyi_ui::toy::{self, Action, Toy, ToyCtx};
use shuyi_ui::{FrameInput, Game, GameHooks, Screen, Transition};

std::thread_local! {
    /// The names the fake scopes gave tags, the tag being the place + 1.
    static NAMES: RefCell<Vec<(u8, &'static str)>> = const { RefCell::new(Vec::new()) };
    /// The current tag: 0, `other`, at the start.
    static CURRENT: Cell<u16> = const { Cell::new(0) };
    /// Each tag entered: its class, its name, and the name of the tag it
    /// was entered inside.
    static ENTERED: RefCell<Vec<(u8, &'static str, &'static str)>> =
        const { RefCell::new(Vec::new()) };
}

fn fake_tag(class: u8, name: &'static str) -> u16 {
    NAMES.with_borrow_mut(|names| {
        let at = names.iter().position(|&n| n == (class, name));
        let at = at.unwrap_or_else(|| {
            names.push((class, name));
            names.len() - 1
        });
        u16::try_from(at + 1).unwrap_or(u16::MAX)
    })
}

/// The class and name of `tag`.
fn named(tag: u16) -> (u8, &'static str) {
    match tag {
        0 => (0, "other"),
        n => NAMES.with_borrow(|names| names[usize::from(n) - 1]),
    }
}

fn fake_enter(tag: u16) -> u16 {
    let ((class, name), (_, inside)) = (named(tag), named(CURRENT.get()));
    ENTERED.with_borrow_mut(|entered| entered.push((class, name, inside)));
    CURRENT.replace(tag)
}

fn fake_leave(before: u16) {
    CURRENT.set(before);
}

/// Scopes that note what is entered, on this thread.
fn fake_scopes() -> MemoryScopes {
    MemoryScopes::new(fake_tag, fake_enter, fake_leave)
}

/// What was entered since the last call, and the tag current now.
fn take_entered() -> (Vec<(u8, &'static str, &'static str)>, u16) {
    (ENTERED.take(), CURRENT.get())
}

/// A screen that marks a span inside another in every update.
struct Marks;

impl Screen<Toy> for Marks {
    fn name(&self) -> &'static str {
        "marks"
    }

    fn update(&mut self, ctx: &mut ToyCtx, _: &FrameInput<Action>) -> Transition<Toy> {
        ctx.span("plan", |ctx| ctx.span("path", |_| ()));
        Transition::None
    }

    fn draw(&self, _: &ToyCtx, _: &mut GlyphBuffer) {}
}

#[test]
fn a_frame_enters_its_phases_and_a_span_inside_update() {
    let game = Game::new(toy::ctx(), Box::new(Marks), GameHooks::default());
    let mut h = Harness::new(game);
    h.set_scopes(Some(fake_scopes()));
    let _ = take_entered();
    h.frame(&[], FRAME_DT);
    let (phase, span) = (TagClass::Phase as u8, TagClass::Span as u8);
    let (entered, current) = take_entered();
    assert_eq!(
        entered,
        [
            (phase, "app", "other"),
            (phase, "update", "app"),
            (span, "plan", "update"),
            (span, "path", "plan"),
            (phase, "draw", "app"),
            (phase, "paint", "app"),
            (phase, "perf", "app"),
        ]
    );
    assert_eq!(current, 0, "every tag entered was left");
    h.frame(&[], FRAME_DT);
    assert_eq!(take_entered().0, entered, "the same each frame");
}

#[test]
fn a_span_outside_a_frame_enters_its_tag_too() {
    let mut ctx = toy::ctx();
    ctx.set_memory_scopes(Some(fake_scopes()));
    let _ = take_entered();
    assert_eq!(ctx.span("setup", |_| 4), 4);
    assert_eq!(take_entered(), (vec![(2, "setup", "other")], 0));
    // Scopes taken away inside a span: its tag is still left.
    ctx.span("setup", |ctx| ctx.set_memory_scopes(None));
    assert_eq!(take_entered(), (vec![(2, "setup", "other")], 0));
    ctx.span("setup", |_| ());
    assert_eq!(take_entered(), (vec![], 0));
}

/// Keys the toy reacts to, and some it doesn't.
const KEYS: [&str; 12] = [
    "f", "d", "g", "z", "j", "k", "Up", "Down", "Left", "Right", "Escape", "F2",
];

/// The toy after `script` (indices into [`KEYS`]), with the fake scopes on
/// or off: its screens, its frame's text and the tags entered.
fn played(script: &[usize], scopes: bool) -> (Vec<&'static str>, String, usize) {
    let _ = take_entered();
    let mut h = Harness::new(toy::game());
    if scopes {
        h.set_scopes(Some(fake_scopes()));
    }
    for &key in script {
        h.keys(KEYS[key]);
        h.wait(FRAME_DT * 2.0);
    }
    let (entered, current) = take_entered();
    assert_eq!(current, 0, "every tag entered was left");
    (h.screens(), h.snapshot(), entered.len())
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(32))]

    #[test]
    fn the_toy_plays_the_same_with_the_scopes_on_and_off(
        script in prop::collection::vec(0..KEYS.len(), 0..12),
    ) {
        let (screens, frame, entered) = played(&script, true);
        let (screens_off, frame_off, entered_off) = played(&script, false);
        prop_assert_eq!(screens, screens_off);
        prop_assert_eq!(frame, frame_off);
        prop_assert_eq!(entered_off, 0, "nothing entered with the scopes off");
        prop_assert!(script.is_empty() || entered > 0);
    }
}
