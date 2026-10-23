//! The engine's integration tests: the test Harness driving the toy game
//! (`shuyi_ui::toy`) as a platform would, with no window. One test program;
//! each file is a module.

mod audio;
mod cinematic;
mod controller;
mod grid;
mod key_bindings;
mod layout_picker;
mod modules;
mod scene_camera;
mod screens;
mod scripts;
mod sprites;

use std::cell::RefCell;
use std::rc::Rc;

use shuyi_platform::harness::Harness;
use shuyi_ui::frame::GlyphBuffer;
use shuyi_ui::input::Button;
use shuyi_ui::toy::{self, Action, Toy, ToyCtx};
use shuyi_ui::{FrameInput, Game, GameHooks, Screen, Transition};

/// What a [`Recorder`] saw in each frame: the actions, the frame time, the
/// buttons and the chords that went down, and the text typed.
#[derive(Debug, Clone, PartialEq)]
pub struct Seen {
    actions: Vec<Action>,
    dt: f32,
    buttons: Vec<Button>,
    chords: Vec<String>,
    text: String,
}

/// Every frame a [`Recorder`] saw, oldest first.
pub type Log = Rc<RefCell<Vec<Seen>>>;

/// A screen that records every frame it sees.
pub struct Recorder(Log);

impl Screen<Toy> for Recorder {
    fn name(&self) -> &'static str {
        "recorder"
    }

    fn update(&mut self, _: &mut ToyCtx, input: &FrameInput<Action>) -> Transition<Toy> {
        self.0.borrow_mut().push(Seen {
            actions: input.actions.clone(),
            dt: input.dt(),
            buttons: input.pressed_buttons().to_vec(),
            chords: input
                .pressed_chords()
                .iter()
                .map(ToString::to_string)
                .collect(),
            text: input.text().iter().collect(),
        });
        Transition::None
    }

    fn draw(&self, _: &ToyCtx, _: &mut GlyphBuffer) {}
}

/// The toy's context showing only a [`Recorder`], and its log.
pub fn recorder() -> (Harness<Toy>, Log) {
    let log = Log::default();
    let screen = Box::new(Recorder(Rc::clone(&log)));
    let game = Game::new(toy::ctx(), screen, GameHooks::default());
    (Harness::new(game), log)
}

/// The toy game with the right-handed layout chosen, at its home screen.
pub fn toy() -> Harness<Toy> {
    Harness::new(toy::game())
}

/// The toy game as on a first launch: no layout chosen.
pub fn first_launch() -> Harness<Toy> {
    Harness::new(toy::game_on(toy::first_launch()))
}
