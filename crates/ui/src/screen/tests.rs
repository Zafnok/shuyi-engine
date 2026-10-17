//! The stack's rules, with screens that log what they are asked to do.

use super::*;
use alloc::rc::Rc;
use core::cell::RefCell;

use crate::frame::color::Rgb;
use crate::frame::glyph_buffer::Cell;
use crate::toy::{Action, Home, Toy, ToyCtx, ctx};

type Log = Rc<RefCell<Vec<String>>>;

/// A test screen that logs calls, answers updates from a script and
/// draws its tag in the top-left cell.
struct Probe {
    name: &'static str,
    overlay: bool,
    log: Log,
    next: RefCell<Vec<Transition<Toy>>>,
}

impl Probe {
    fn boxed(name: &'static str, overlay: bool, log: &Log) -> Box<dyn Screen<Toy>> {
        Box::new(Self::with(name, overlay, log, vec![]))
    }

    fn with(name: &'static str, overlay: bool, log: &Log, next: Vec<Transition<Toy>>) -> Self {
        Self {
            name,
            overlay,
            log: Rc::clone(log),
            next: RefCell::new(next),
        }
    }
}

impl Screen<Toy> for Probe {
    fn name(&self) -> &'static str {
        self.name
    }

    fn update(&mut self, _: &mut ToyCtx, input: &FrameInput<Action>) -> Transition<Toy> {
        self.log
            .borrow_mut()
            .push(format!("update {} {:?}", self.name, input.actions));
        let mut next = self.next.borrow_mut();
        if next.is_empty() {
            Transition::None
        } else {
            next.remove(0)
        }
    }

    fn draw(&self, _: &ToyCtx, buf: &mut GlyphBuffer) {
        self.log.borrow_mut().push(format!("draw {}", self.name));
        let black = Rgb::new(0, 0, 0);
        let glyph = self.name.chars().next().unwrap_or('?');
        buf.set(0, 0, Cell::new(glyph, black, black));
    }

    fn is_overlay(&self) -> bool {
        self.overlay
    }
}

fn input(actions: &[Action]) -> FrameInput<Action> {
    FrameInput::new(actions.to_vec(), 0.0, vec![])
}

fn draws(stack: &ScreenStack<Toy>, log: &Log) -> Vec<String> {
    log.borrow_mut().clear();
    let ctx = ctx();
    let mut buf = GlyphBuffer::new(1, 1, Cell::new(' ', Rgb::new(0, 0, 0), Rgb::new(0, 0, 0)));
    stack.draw(&ctx, &mut buf);
    log.borrow().clone()
}

#[test]
fn only_the_top_screen_updates() {
    let log = Log::default();
    let mut stack = ScreenStack::new(Probe::boxed("a", false, &log));
    stack.push(Probe::boxed("b", false, &log));
    assert!(!stack.update(&mut ctx(), &input(&[Action::Confirm])));
    assert_eq!(*log.borrow(), ["update b [Confirm]"]);
    assert_eq!(stack.names(), ["a", "b"]);
    assert_eq!(stack.top_name(), Some("b"));
}

#[test]
fn transitions_change_the_stack() {
    let log = Log::default();
    let mut stack = ScreenStack::new(Probe::boxed("a", false, &log));
    assert!(!stack.apply(Transition::None));
    assert_eq!(stack.names(), ["a"]);
    assert!(!stack.apply(Transition::Push(Probe::boxed("b", false, &log))));
    assert_eq!(stack.names(), ["a", "b"]);
    assert!(!stack.apply(Transition::Replace(Probe::boxed("c", false, &log))));
    assert_eq!(stack.names(), ["a", "c"]);
    assert!(!stack.apply(Transition::Pop));
    assert_eq!(stack.names(), ["a"]);
    assert!(!stack.is_empty());
    assert!(
        stack.apply(Transition::Pop),
        "popping the last screen quits"
    );
    assert!(stack.is_empty());
    assert_eq!(stack.top_name(), None);
}

#[test]
fn quit_keeps_the_stack() {
    let log = Log::default();
    let mut stack = ScreenStack::new(Probe::boxed("a", false, &log));
    assert!(stack.apply(Transition::Quit));
    assert_eq!(stack.names(), ["a"]);
}

#[test]
fn update_applies_the_returned_transition() {
    let log = Log::default();
    let root = Probe::with(
        "a",
        false,
        &log,
        vec![Transition::Push(Probe::boxed("b", false, &log))],
    );
    let mut stack = ScreenStack::new(Box::new(root));
    assert!(!stack.update(&mut ctx(), &input(&[])));
    assert_eq!(stack.names(), ["a", "b"]);
    let quitter = Probe::with("q", false, &log, vec![Transition::Quit]);
    stack.push(Box::new(quitter));
    assert!(stack.update(&mut ctx(), &input(&[])));
}

#[test]
fn empty_stack_quits_and_draws_nothing() {
    let log = Log::default();
    let mut stack = ScreenStack::default();
    assert!(stack.is_empty());
    assert!(stack.update(&mut ctx(), &input(&[])));
    assert!(draws(&stack, &log).is_empty());
}

#[test]
fn draw_starts_at_the_top_most_opaque_screen() {
    let log = Log::default();
    let mut stack = ScreenStack::new(Probe::boxed("a", false, &log));
    stack.push(Probe::boxed("b", false, &log));
    assert_eq!(draws(&stack, &log), ["draw b"]);
    stack.push(Probe::boxed("o", true, &log));
    stack.push(Probe::boxed("p", true, &log));
    assert_eq!(draws(&stack, &log), ["draw b", "draw o", "draw p"]);
}

/// A screen is opaque unless it says otherwise: one that keeps the default
/// hides the screens below it.
#[test]
fn a_screen_is_opaque_by_default() {
    let log = Log::default();
    assert!(!Home::default().is_overlay());
    let mut stack = ScreenStack::new(Probe::boxed("a", false, &log));
    stack.push(Box::new(Home::default()));
    assert!(draws(&stack, &log).is_empty(), "only the home screen draws");
    stack.push(Probe::boxed("o", true, &log));
    assert_eq!(draws(&stack, &log), ["draw o"]);
}

#[test]
fn all_overlays_draw_everything() {
    let log = Log::default();
    let mut stack = ScreenStack::new(Probe::boxed("o", true, &log));
    stack.push(Probe::boxed("p", true, &log));
    assert_eq!(draws(&stack, &log), ["draw o", "draw p"]);
}

#[test]
fn transition_debug_names_screens() {
    let log = Log::default();
    let dbg = |t: Transition<Toy>| format!("{t:?}");
    assert_eq!(dbg(Transition::None), "None");
    assert_eq!(
        dbg(Transition::Push(Probe::boxed("a", false, &log))),
        "Push(a)"
    );
    assert_eq!(dbg(Transition::Pop), "Pop");
    assert_eq!(
        dbg(Transition::Replace(Probe::boxed("b", false, &log))),
        "Replace(b)"
    );
    assert_eq!(dbg(Transition::Quit), "Quit");
}

/// A screen that opts in is found by its type, the top-most first; one
/// that doesn't never is.
#[test]
fn screens_are_found_by_type_if_they_opt_in() {
    let log = Log::default();
    let mut stack = ScreenStack::new(Box::new(Home { frames: 1 }));
    stack.push(Box::new(Home { frames: 2 }));
    stack.push(Probe::boxed("p", false, &log));
    assert_eq!(stack.find::<Home>().map(|h| h.frames), Some(2));
    if let Some(home) = stack.find_mut::<Home>() {
        home.frames = 5;
    }
    assert_eq!(stack.find::<Home>().map(|h| h.frames), Some(5));
    assert!(stack.find::<Probe>().is_none());
    assert!(stack.find_mut::<Probe>().is_none());
    assert!(ScreenStack::<Toy>::default().find::<Home>().is_none());
}
