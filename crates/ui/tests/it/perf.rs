//! Frame times and spans (ADR-0046) on the Harness, with the fake clock:
//! the same records on every run, and the same game with the clock on and
//! off.

use insta::assert_snapshot;
use proptest::prelude::*;
use ron::ser::PrettyConfig;
use shuyi_platform::harness::{FRAME_DT, Harness};
use shuyi_ui::frame::GlyphBuffer;
use shuyi_ui::perf::{FrameRecord, PerfClock};
use shuyi_ui::toy::{self, Action, Toy, ToyCtx};
use shuyi_ui::{FrameInput, Game, GameHooks, Screen, Transition};

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

/// The first three records of the toy's context showing [`Marks`], with
/// the fake clock.
fn first_three() -> Vec<FrameRecord> {
    let game = Game::new(toy::ctx(), Box::new(Marks), GameHooks::default());
    let mut h = Harness::new(game);
    h.set_clock(Some(PerfClock::fake()));
    for _ in 0..3 {
        h.frame(&[], FRAME_DT);
    }
    h.perf().records().cloned().collect()
}

#[test]
fn the_first_three_records_with_the_fake_clock() {
    let records = first_three();
    assert_eq!(records, first_three(), "the same on every run");
    let text = ron::ser::to_string_pretty(&records, PrettyConfig::default())
        .unwrap_or_else(|e| panic!("records write: {e}"));
    assert_snapshot!(text);
}

/// Keys the toy reacts to, and some it doesn't.
const KEYS: [&str; 12] = [
    "f", "d", "g", "z", "j", "k", "Up", "Down", "Left", "Right", "Escape", "F2",
];

/// The toy after `script` (indices into [`KEYS`]), with the fake clock on
/// or off: its screens, its frame's text and its records.
fn played(script: &[usize], clock: bool) -> (Vec<&'static str>, String, Vec<FrameRecord>) {
    let mut h = Harness::new(toy::game());
    if clock {
        h.set_clock(Some(PerfClock::fake()));
    }
    for &key in script {
        h.keys(KEYS[key]);
        h.wait(FRAME_DT * 2.0);
    }
    let records = h.perf().records().cloned().collect();
    (h.screens(), h.snapshot(), records)
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(32))]

    #[test]
    fn the_toy_plays_the_same_with_the_clock_on_and_off(
        script in prop::collection::vec(0..KEYS.len(), 0..12),
    ) {
        let (screens, frame, records) = played(&script, true);
        let (screens_off, frame_off, records_off) = played(&script, false);
        prop_assert_eq!(screens, screens_off);
        prop_assert_eq!(frame, frame_off);
        prop_assert!(records_off.is_empty());
        for record in &records {
            let t = record.times;
            let parts = t.update_us + t.draw_us + t.paint_us + t.present_us;
            prop_assert!(parts <= t.frame_us, "{:?}", record);
        }
    }
}
