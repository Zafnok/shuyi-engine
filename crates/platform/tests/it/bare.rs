//! [`Bare`]: a backend that implements only the seven methods of
//! ADR-0024 §2, as a private console's backend may, and runs a frame of
//! the toy game.
//!
//! A method added to `Platform` without a default fails to compile here:
//! give it a default instead (ADR-0040 §7), so no backend breaks.

use shuyi_platform::{App, Audio, DisplayInfo, Frame, FrameOutcome, Millis, Platform};
use shuyi_platform::{Silent, SystemEvent, TextInput};
use shuyi_ui::RawInputEvent;
use shuyi_ui::storage::{MemoryStorage, Storage};
use shuyi_ui::toy;

/// The fewest methods a backend can have: no sound, no system keyboard,
/// storage in memory.
#[derive(Debug, Default)]
struct Bare {
    audio: Silent,
    /// How many frames it was shown.
    presented: u32,
}

impl Platform for Bare {
    fn display(&self) -> DisplayInfo {
        DisplayInfo::new(400, 240)
    }

    fn present(&mut self, _: &Frame<'_>) {
        self.presented += 1;
    }

    fn poll_input(&mut self, _: &mut Vec<RawInputEvent>) {}

    fn system_events(&mut self, _: &mut Vec<SystemEvent>) {}

    fn storage(&mut self) -> Box<dyn Storage> {
        Box::new(MemoryStorage::new())
    }

    fn audio(&mut self) -> &mut dyn Audio {
        &mut self.audio
    }

    fn text_input(&mut self) -> Option<&mut dyn TextInput> {
        None
    }
}

#[test]
fn a_backend_of_the_seven_methods_runs_a_frame() {
    let mut bare = Bare::default();
    let mut app = App::running(toy::game());
    assert_eq!(app.frame(&mut bare, Millis(0)), FrameOutcome::Continue);
    assert_eq!(bare.presented, 1);
    // The defaults: no clock, so nothing measured, and no memory read.
    assert!(bare.clock().is_none());
    assert_eq!(bare.memory(), None);
    assert_eq!(bare.heap_snapshot(), None);
    assert_eq!(app.heap_snapshot(&mut bare), None);
    assert_eq!(app.perf().records().len(), 0);
}
