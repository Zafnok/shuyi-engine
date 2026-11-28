//! A restart a module asks for (`Ctx::restart`): honoured once every
//! module's reload hook has run, once; and asked anywhere else, at the
//! start of the next frame.

use super::*;
use alloc::rc::Rc;
use core::cell::RefCell;
use core::sync::atomic::{AtomicU32, Ordering};

use crate::module::{Hook, Install, Module};
use crate::toy::{Lobby, Toy, ToyCtx, game};

/// What the modules saw, in order.
type Log = Rc<RefCell<Vec<String>>>;

/// How many lobbies `lobby` made: only the test of the reload makes them.
static LOBBIES: AtomicU32 = AtomicU32::new(0);

/// The screen a restart starts over at: a lobby, counted.
fn lobby(_: &mut ToyCtx) -> Box<dyn Screen<Toy>> {
    LOBBIES.fetch_add(1, Ordering::SeqCst);
    Box::new(Lobby::new())
}

/// A module that asks for a restart at the lobby when a file changes, if
/// `asks`, and notes how many lobbies were made when it heard.
struct Restarter {
    name: &'static str,
    asks: bool,
    log: Log,
}

impl Module<Toy> for Restarter {
    fn name(&self) -> &'static str {
        self.name
    }

    fn install(&mut self, install: &mut Install<'_, Toy>) {
        install.hook(Hook::Reloaded);
    }

    fn reloaded(&mut self, ctx: &mut ToyCtx, _: &[String]) {
        let made = LOBBIES.load(Ordering::SeqCst);
        self.log.borrow_mut().push(format!("{}: {made}", self.name));
        if self.asks {
            ctx.restart(lobby);
        }
    }
}

#[test]
fn a_restart_asked_in_a_reload_is_honoured_after_every_hook_once() {
    let log = Log::default();
    let module = |name, asks| Restarter {
        name,
        asks,
        log: Rc::clone(&log),
    };
    let mut game = game()
        .with_module(module("first", true))
        .and_then(|game| game.with_module(module("second", true)))
        .and_then(|game| game.with_module(module("third", false)))
        .unwrap();
    game.act(crate::toy::Action::Confirm);
    assert_eq!(game.screens(), ["home", "menu"]);
    game.reloaded(&["stage.map".to_owned()]);
    // Every hook ran on the old screens, and the two asks made one lobby.
    assert_eq!(*log.borrow(), ["first: 0", "second: 0", "third: 0"]);
    assert_eq!(LOBBIES.load(Ordering::SeqCst), 1);
    assert_eq!(game.screens(), ["lobby"]);
    // Asked no more: the next frame keeps the lobby it made.
    game.frame(&[], 0.016);
    assert_eq!(LOBBIES.load(Ordering::SeqCst), 1);
    assert_eq!(game.screens(), ["lobby"]);
}

#[test]
fn a_restart_asked_outside_a_reload_is_honoured_at_the_next_frame() {
    let mut game = game();
    game.act(crate::toy::Action::Confirm);
    game.ctx_mut().restart(|_| Box::new(Lobby::new()));
    assert_eq!(game.screens(), ["home", "menu"], "not before the frame");
    game.frame(&[], 0.016);
    assert_eq!(game.screens(), ["lobby"]);
}

#[test]
fn a_reload_with_no_restart_asked_keeps_the_screens() {
    let log = Log::default();
    let mut game = game()
        .with_module(Restarter {
            name: "quiet",
            asks: false,
            log,
        })
        .unwrap();
    game.act(crate::toy::Action::Confirm);
    game.reloaded(&["en.ron".to_owned()]);
    assert_eq!(game.screens(), ["home", "menu"]);
}
