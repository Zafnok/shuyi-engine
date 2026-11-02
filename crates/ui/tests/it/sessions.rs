//! What a screen and a debug tool have to do with a session record: a
//! screen's state hash, and "Save session" on the debug menu.

use shuyi_core::StateHash;
use shuyi_platform::harness::Harness;
use shuyi_ui::debug::DebugMenuScreen;
use shuyi_ui::frame::GlyphBuffer;
use shuyi_ui::toy::{self, Action, Toy, ToyCtx};
use shuyi_ui::{FrameInput, Game, GameHooks, Screen, Transition};

/// A screen that holds a state, which it hashes.
struct Holds(u64);

impl Screen<Toy> for Holds {
    fn name(&self) -> &'static str {
        "holds"
    }

    fn update(&mut self, _: &mut ToyCtx, _: &FrameInput<Action>) -> Transition<Toy> {
        Transition::None
    }

    fn draw(&self, _: &ToyCtx, _: &mut GlyphBuffer) {}

    fn state_hash(&self) -> Option<StateHash> {
        Some(StateHash(self.0))
    }
}

#[test]
fn the_game_hashes_the_state_of_a_screen_that_holds_one() {
    let game = Game::new(toy::ctx(), Box::new(Holds(7)), GameHooks::default());
    assert_eq!(game.state_hash(), Some(StateHash(7)));
    // A screen says nothing unless it opts in.
    assert_eq!(toy::game().state_hash(), None);
}

#[test]
fn the_debug_menu_saves_a_session_only_while_the_platform_records_one() {
    let ctx = toy::ctx();
    assert!(!ctx.session_recording());
    let shown = |h: &Harness<Toy>| h.snapshot().contains("Save session");

    let menu = DebugMenuScreen::new(&ctx, Vec::new());
    let plain = Harness::new(Game::new(ctx, Box::new(menu), GameHooks::default()));
    assert!(!shown(&plain));

    let mut ctx = toy::ctx();
    ctx.set_session_recording(true);
    assert!(ctx.session_recording());
    let menu = DebugMenuScreen::new(&ctx, Vec::new());
    let mut h = Harness::new(Game::new(ctx, Box::new(menu), GameHooks::default()));
    assert!(shown(&h));
    assert!(!h.ctx_mut().take_session_save());
    // Choosing it asks once, and the menu stays.
    h.keys("f");
    assert!(shown(&h));
    assert!(h.ctx_mut().take_session_save());
    assert!(!h.ctx_mut().take_session_save());
}
