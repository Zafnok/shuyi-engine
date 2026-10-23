//! The stack's rules, with screens that log what they are asked to do.

use super::*;
use alloc::rc::Rc;
use core::cell::RefCell;

use crate::frame::color::Rgb;
use crate::frame::glyph_buffer::Cell;
use crate::toy::{Action, Home, Toy, ToyCtx, ctx};

type Log = Rc<RefCell<Vec<String>>>;

/// A test screen that logs calls, answers updates (and being uncovered)
/// from a script and draws its tag in the top-left cell.
struct Probe {
    name: &'static str,
    overlay: bool,
    log: Log,
    next: RefCell<Vec<Transition<Toy>>>,
    on_uncover: Vec<Transition<Toy>>,
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
            on_uncover: vec![],
        }
    }

    /// The same probe, answering being uncovered with `answers`, in turn.
    fn uncovering(mut self, answers: Vec<Transition<Toy>>) -> Box<dyn Screen<Toy>> {
        self.on_uncover = answers;
        Box::new(self)
    }
}

/// How a probe logs what it was uncovered with: a `u8` by its value.
fn shown(result: Option<&ScreenResult>) -> String {
    match result {
        None => "None".to_owned(),
        Some(r) => r
            .get::<u8>()
            .map_or_else(|| format!("{r:?}"), ToString::to_string),
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

    fn covered(&mut self, _: &mut ToyCtx) {
        self.log.borrow_mut().push(format!("covered {}", self.name));
    }

    fn uncovered(&mut self, _: &mut ToyCtx, result: Option<ScreenResult>) -> Transition<Toy> {
        let line = format!("uncovered {} {}", self.name, shown(result.as_ref()));
        self.log.borrow_mut().push(line);
        if self.on_uncover.is_empty() {
            Transition::None
        } else {
            self.on_uncover.remove(0)
        }
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
    let got = draws(&stack, &log);
    assert!(got.is_empty(), "{got:?}");
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

/// The log since it was last read, emptied.
fn drain(log: &Log) -> Vec<String> {
    log.borrow_mut().drain(..).collect()
}

/// Ticket 2306: a screen pushed over another pops with `7u8`; the one
/// below gets it in `uncovered`, in the same update.
#[test]
fn a_screen_hands_its_result_to_the_one_below() {
    let log = Log::default();
    let mut ctx = ctx();
    let mut stack = ScreenStack::new(Probe::boxed("a", false, &log));
    let asker = Probe::with("b", true, &log, vec![Transition::pop_with(7u8)]);
    assert!(!stack.apply_in(&mut ctx, Transition::Push(Box::new(asker))));
    assert!(!stack.update(&mut ctx, &input(&[Action::Confirm])));
    assert_eq!(
        drain(&log),
        ["covered a", "update b [Confirm]", "uncovered a 7"]
    );
    assert_eq!(stack.names(), ["a"]);
}

/// `covered` once on a push, `uncovered` once on a pop (with nothing for
/// a plain one), and neither when the top screen replaces itself.
#[test]
fn covered_on_push_uncovered_on_pop_neither_on_replace() {
    let log = Log::default();
    let mut ctx = ctx();
    let mut stack = ScreenStack::new(Probe::boxed("a", false, &log));
    assert!(!stack.apply_in(&mut ctx, Transition::Push(Probe::boxed("b", false, &log))));
    assert_eq!(drain(&log), ["covered a"]);
    assert!(!stack.apply_in(
        &mut ctx,
        Transition::Replace(Probe::boxed("c", false, &log))
    ));
    assert!(drain(&log).is_empty(), "a replace tells nobody");
    assert_eq!(stack.names(), ["a", "c"]);
    assert!(!stack.apply_in(&mut ctx, Transition::None));
    let got = drain(&log);
    assert!(got.is_empty(), "{got:?}");
    assert!(!stack.apply_in(&mut ctx, Transition::Pop));
    assert_eq!(drain(&log), ["uncovered a None"]);
    assert_eq!(stack.names(), ["a"]);
}

/// The root screen replacing itself, and pushing onto an empty stack,
/// tell nobody either.
#[test]
fn a_lone_screen_has_nobody_to_tell() {
    let log = Log::default();
    let mut ctx = ctx();
    let mut stack = ScreenStack::new(Probe::boxed("a", false, &log));
    assert!(!stack.apply_in(
        &mut ctx,
        Transition::Replace(Probe::boxed("b", false, &log))
    ));
    assert_eq!(stack.names(), ["b"]);
    let mut empty = ScreenStack::default();
    assert!(!empty.apply_in(&mut ctx, Transition::Push(Probe::boxed("c", false, &log))));
    assert_eq!(empty.names(), ["c"]);
    let got = drain(&log);
    assert!(got.is_empty(), "{got:?}");
}

/// Popping the last screen quits, with or without a value, and nobody
/// hears of it; Quit keeps the stack and tells nobody.
#[test]
fn quitting_tells_nobody() {
    let log = Log::default();
    let mut ctx = ctx();
    let mut stack = ScreenStack::new(Probe::boxed("a", false, &log));
    assert!(stack.apply_in(&mut ctx, Transition::Quit));
    assert_eq!(stack.names(), ["a"]);
    assert!(stack.apply_in(&mut ctx, Transition::pop_with(1u8)));
    assert!(stack.is_empty());
    let mut stack = ScreenStack::new(Probe::boxed("a", false, &log));
    assert!(stack.apply_in(&mut ctx, Transition::Pop));
    assert!(stack.is_empty());
    let got = drain(&log);
    assert!(got.is_empty(), "{got:?}");
}

/// What `uncovered` returns applies at once: a screen can push the next
/// one, or pop with an answer of its own, which reaches the screen below
/// it in turn.
#[test]
fn the_transition_from_uncovered_applies_at_once() {
    let log = Log::default();
    let mut ctx = ctx();
    let next = Probe::boxed("c", false, &log);
    let flow = Probe::with("a", false, &log, vec![]).uncovering(vec![Transition::Push(next)]);
    let mut stack = ScreenStack::new(flow);
    stack.push(Probe::boxed("b", false, &log));
    assert!(!stack.apply_in(&mut ctx, Transition::pop_with(1u8)));
    assert_eq!(stack.names(), ["a", "c"]);
    assert_eq!(drain(&log), ["uncovered a 1", "covered a"]);
    // Two answers passed down: c pops with 2, b passes 3 on to the root.
    let root = Probe::boxed("r", false, &log);
    let middle = Probe::with("m", false, &log, vec![]).uncovering(vec![Transition::pop_with(3u8)]);
    let mut stack = ScreenStack::new(root);
    stack.push(middle);
    stack.push(Probe::boxed("t", false, &log));
    assert!(!stack.apply_in(&mut ctx, Transition::pop_with(2u8)));
    assert_eq!(drain(&log), ["uncovered m 2", "uncovered r 3"]);
    assert_eq!(stack.names(), ["r"]);
    // A screen that pops itself on being uncovered, the last one: quit.
    let last = Probe::with("l", false, &log, vec![]).uncovering(vec![Transition::Pop]);
    let mut stack = ScreenStack::new(last);
    stack.push(Probe::boxed("t", false, &log));
    assert!(stack.apply_in(&mut ctx, Transition::Pop));
    assert!(stack.is_empty());
}

/// `apply`, without the context, changes the stack alone: nobody hears,
/// and a value popped with is dropped.
#[test]
fn apply_changes_the_stack_alone() {
    let log = Log::default();
    let mut stack = ScreenStack::new(Probe::boxed("a", false, &log));
    assert!(!stack.apply(Transition::Push(Probe::boxed("b", false, &log))));
    assert!(!stack.apply(Transition::pop_with(7u8)));
    assert_eq!(stack.names(), ["a"]);
    let got = drain(&log);
    assert!(got.is_empty(), "{got:?}");
    assert!(stack.apply(Transition::pop_with(7u8)), "the last one quits");
}

/// A result reads back as its own type only, and says which it is.
#[test]
fn a_result_is_read_as_its_type() {
    let result = ScreenResult::new(7u8);
    assert!(result.is::<u8>());
    assert!(!result.is::<u16>());
    assert_eq!(result.get::<u8>(), Some(&7));
    assert_eq!(result.get::<bool>(), None);
    assert_eq!(format!("{result:?}"), "ScreenResult(u8)");
    let result = result.downcast::<bool>().unwrap_err();
    assert_eq!(result.downcast::<u8>().ok(), Some(7));
    assert_eq!(ScreenResult::new("yes").take::<&str>(), Some("yes"));
    assert_eq!(ScreenResult::new(1u8).take::<i8>(), None);
    let dbg = |t: Transition<Toy>| format!("{t:?}");
    assert_eq!(dbg(Transition::pop_with(true)), "PopWith(bool)");
}

/// A screen that declares Confirm alone and logs the actions it gets.
struct Picky {
    log: Log,
}

impl Screen<Toy> for Picky {
    fn name(&self) -> &'static str {
        "picky"
    }

    fn actions(&self) -> Option<Vec<Action>> {
        Some(vec![Action::Confirm])
    }

    fn update(&mut self, _: &mut ToyCtx, input: &FrameInput<Action>) -> Transition<Toy> {
        let held = input.is_held(Action::Confirm);
        self.log
            .borrow_mut()
            .push(format!("{:?} held {held}", input.actions));
        Transition::None
    }

    fn draw(&self, _: &ToyCtx, _: &mut GlyphBuffer) {}
}

/// With the guard on, a screen that declares its actions gets only
/// those, and the rest are logged; off (a game's own stack), it gets
/// everything. A screen that declares nothing gets everything either way.
#[test]
fn the_guard_hands_a_screen_only_the_actions_it_declares() {
    let mut ctx = ctx();
    let log = Log::default();
    let input = FrameInput::new(
        vec![Action::Cancel, Action::Confirm, Action::Info],
        0.25,
        vec![Action::Confirm, Action::Cancel],
    );
    let mut stack = ScreenStack::new(Box::new(Picky {
        log: Rc::clone(&log),
    }));
    stack.update(&mut ctx, &input);
    assert_eq!(drain(&log), ["[Cancel, Confirm, Info] held true"]);
    assert_eq!(stack.undeclared(), []);
    stack.guard_actions(true);
    stack.update(&mut ctx, &input);
    assert_eq!(drain(&log), ["[Confirm] held true"]);
    assert_eq!(
        stack.undeclared(),
        [("picky", Action::Cancel), ("picky", Action::Info)]
    );
    stack.push(Probe::boxed("open", false, &log));
    stack.update(&mut ctx, &input);
    assert_eq!(drain(&log), ["update open [Cancel, Confirm, Info]"]);
    assert_eq!(stack.undeclared().len(), 2);
}

/// What a screen declares, its input keeps: the time, the keys, and how
/// often each action came.
#[test]
fn declared_input_keeps_all_but_the_other_actions() {
    let input = FrameInput::new(
        vec![Action::Confirm, Action::Cancel, Action::Confirm],
        0.5,
        vec![Action::Cancel],
    )
    .with_ticks(2)
    .with_typing(vec![], vec!['a']);
    let (seen, missed) = input.declared("s", &[Action::Confirm]);
    assert_eq!(seen.actions, [Action::Confirm, Action::Confirm]);
    assert_eq!(missed, [Action::Cancel]);
    assert!(!seen.is_held(Action::Confirm));
    assert!((seen.dt() - 0.5).abs() < f32::EPSILON);
    assert_eq!((seen.ticks(), seen.text()), (2, &['a'][..]));
    let (all, none) = input.declared("s", &[Action::Confirm, Action::Cancel]);
    assert!(all.is_held(Action::Cancel));
    assert_eq!(all.actions, input.actions);
    assert!(none.is_empty(), "{none:?}");
}

#[test]
#[should_panic(expected = "screen \"s\" asks whether Cancel is held, but doesn't declare it")]
fn asking_about_an_undeclared_action_panics() {
    let input = FrameInput::<Action>::new(vec![], 0.0, vec![Action::Cancel]);
    let (seen, _) = input.declared("s", &[Action::Confirm]);
    let _ = seen.is_held(Action::Cancel);
}
