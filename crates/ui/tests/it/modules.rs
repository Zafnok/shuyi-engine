//! Modules joining the toy game (ADR-0040): their hooks run in install
//! order, before the screens' update and over their drawing, only when
//! declared; screens read their state by type; their debug entries come
//! last on the debug menu; and a module that only draws changes nothing
//! that happens.

use std::cell::{Cell as Shared, RefCell};
use std::rc::Rc;

use insta::assert_snapshot;
use shuyi_platform::harness::{FRAME_DT, Harness};
use shuyi_ui::debug::{DebugEntry, DebugMenuScreen};
use shuyi_ui::frame::{Cell, EngineColor, GlyphBuffer, Layer, Overlay, Rect};
use shuyi_ui::module::{Hook, Install, Module};
use shuyi_ui::toy::{self, Action, Gallery, Toy, ToyCtx};
use shuyi_ui::{FrameInput, Game, GameHooks, Screen, ScreenHook, Transition};

/// Who ran, in order: module A's state on the context.
#[derive(Debug, Default)]
struct Order(Vec<&'static str>);

/// A module that writes its name in the [`Order`] before each update;
/// `A` puts the `Order` on the context.
struct Writer(&'static str);

impl Module<Toy> for Writer {
    fn name(&self) -> &'static str {
        self.0
    }

    fn install(&mut self, install: &mut Install<'_, Toy>) {
        install.hook(Hook::BeforeUpdate);
        if self.0 == "a" {
            install.state(Order::default());
        }
    }

    fn before_update(&mut self, ctx: &mut ToyCtx, _: f32) {
        if let Some(order) = ctx.module_state_mut::<Order>() {
            order.0.push(self.0);
        }
    }
}

/// A screen that writes `screen` in the [`Order`] on each update.
struct Ordered;

impl Screen<Toy> for Ordered {
    fn name(&self) -> &'static str {
        "ordered"
    }

    fn update(&mut self, ctx: &mut ToyCtx, _: &FrameInput<Action>) -> Transition<Toy> {
        if let Some(order) = ctx.module_state_mut::<Order>() {
            order.0.push("screen");
        }
        Transition::None
    }

    fn draw(&self, _: &ToyCtx, _: &mut GlyphBuffer) {}
}

fn on(root: impl Screen<Toy> + 'static) -> Game<Toy> {
    Game::new(toy::ctx(), Box::new(root), GameHooks::default())
}

#[test]
fn each_frame_runs_the_modules_in_install_order_then_the_screen() {
    let game = on(Ordered).with_module(Writer("a")).unwrap();
    let mut h = Harness::new(game.with_module(Writer("b")).unwrap());
    h.wait(FRAME_DT).wait(FRAME_DT);
    let order = |h: &Harness<Toy>| h.ctx().module_state::<Order>().unwrap().0.clone();
    assert_eq!(order(&h), ["a", "b", "screen", "a", "b", "screen"]);
    // An action from outside the keys passes no time: the modules don't
    // run for it, and do in the frame after.
    h.act(Action::Info);
    assert_eq!(order(&h)[6..], ["screen", "a", "b", "screen"]);
}

/// A module that shades the eight pixels from `x` and writes `glyph` in
/// the sixth cell over the screens.
struct Painter {
    name: &'static str,
    x: i32,
    glyph: char,
}

const A: Painter = Painter {
    name: "a",
    x: 0,
    glyph: 'A',
};
const B: Painter = Painter {
    name: "b",
    x: 8,
    glyph: 'B',
};

impl Painter {
    fn overlay(&self, ctx: &ToyCtx) -> Overlay {
        let color = ctx.palette.engine(EngineColor::Highlight);
        Overlay::new(Rect::new(self.x, 0, 8, 8), color, Layer::Over)
    }
}

impl Module<Toy> for Painter {
    fn name(&self) -> &'static str {
        self.name
    }

    fn install(&mut self, install: &mut Install<'_, Toy>) {
        install.hook(Hook::DrawOver);
    }

    fn draw_over(&self, ctx: &ToyCtx, buf: &mut GlyphBuffer) {
        buf.add_overlay(self.overlay(ctx));
        let text = ctx.palette.engine(EngineColor::Text);
        buf.set(
            5,
            0,
            Cell::new(self.glyph, text, ctx.palette.engine(EngineColor::Panel)),
        );
    }
}

/// A screen with an overlay of its own, under the modules'.
struct Shaded;

impl Screen<Toy> for Shaded {
    fn name(&self) -> &'static str {
        "shaded"
    }

    fn update(&mut self, _: &mut ToyCtx, _: &FrameInput<Action>) -> Transition<Toy> {
        Transition::None
    }

    fn draw(&self, ctx: &ToyCtx, buf: &mut GlyphBuffer) {
        let color = ctx.palette.engine(EngineColor::Text);
        buf.add_overlay(Overlay::new(Rect::new(0, 8, 8, 8), color, Layer::Under));
    }
}

fn painted(game: Game<Toy>) -> Harness<Toy> {
    let modules: [Box<dyn Module<Toy>>; 2] = [Box::new(A), Box::new(B)];
    let game = game.with_modules(modules).unwrap_or_else(|e| panic!("{e}"));
    let mut h = Harness::new(game);
    h.wait(FRAME_DT);
    h
}

#[test]
fn the_modules_draw_over_the_screens_in_install_order() {
    let h = painted(on(Shaded));
    let buffer = h.game().buffer();
    let ctx = h.ctx();
    let screens = Overlay::new(
        Rect::new(0, 8, 8, 8),
        ctx.palette.engine(EngineColor::Text),
        Layer::Under,
    );
    assert_eq!(buffer.overlays(), [screens, A.overlay(ctx), B.overlay(ctx)]);
    assert_eq!(buffer.get(5, 0).map(|c| c.glyph), Some('B'));
}

#[test]
fn the_toys_home_with_two_drawing_modules() {
    assert_snapshot!(painted(toy::game()).snapshot());
}

/// A module that counts its hooks' calls but declares none.
struct Undeclared(Rc<Shared<u32>>);

impl Module<Toy> for Undeclared {
    fn name(&self) -> &'static str {
        "undeclared"
    }

    fn before_update(&mut self, _: &mut ToyCtx, _: f32) {
        self.0.set(self.0.get() + 1);
    }

    fn draw_over(&self, _: &ToyCtx, _: &mut GlyphBuffer) {
        self.0.set(self.0.get() + 1);
    }
}

#[test]
fn a_hook_not_declared_is_never_called() {
    let calls = Rc::new(Shared::new(0));
    let game = toy::game().with_module(Undeclared(Rc::clone(&calls)));
    let mut h = Harness::new(game.unwrap());
    h.keys("f d").wait(0.5);
    assert_eq!(calls.get(), 0);
    assert_eq!(h.game().modules()[0].hooks, []);
}

/// How many frames the [`Clock`] module has seen.
#[derive(Debug, Default)]
struct Ticks(u32);

/// A module that counts frames in its state on the context.
struct Clock;

impl Module<Toy> for Clock {
    fn name(&self) -> &'static str {
        "clock"
    }

    fn install(&mut self, install: &mut Install<'_, Toy>) {
        install.hook(Hook::BeforeUpdate);
        install.state(Ticks::default());
    }

    fn before_update(&mut self, ctx: &mut ToyCtx, _: f32) {
        if let Some(ticks) = ctx.module_state_mut::<Ticks>() {
            ticks.0 += 1;
        }
    }
}

/// A screen that keeps what it reads of the [`Clock`]'s state each frame.
struct Reader(Rc<RefCell<Vec<Option<u32>>>>);

impl Screen<Toy> for Reader {
    fn name(&self) -> &'static str {
        "reader"
    }

    fn update(&mut self, ctx: &mut ToyCtx, _: &FrameInput<Action>) -> Transition<Toy> {
        let ticks = ctx.module_state::<Ticks>().map(|t| t.0);
        self.0.borrow_mut().push(ticks);
        Transition::None
    }

    fn draw(&self, _: &ToyCtx, _: &mut GlyphBuffer) {}
}

#[test]
fn a_screen_reads_the_state_a_module_wrote_and_none_without_it() {
    let read = |with_clock: bool| {
        let seen = Rc::default();
        let mut game = on(Reader(Rc::clone(&seen)));
        if with_clock {
            game = game.with_module(Clock).unwrap();
        }
        Harness::new(game).wait(FRAME_DT).wait(FRAME_DT);
        seen.take()
    };
    assert_eq!(read(true), [Some(1), Some(2)]);
    assert_eq!(read(false), [None, None]);
}

#[test]
fn a_module_that_only_draws_changes_nothing_that_happens() {
    let play = |mut h: Harness<Toy>| {
        h.keys("f f d").wait(0.2).keys("d");
        let audio: Vec<String> = h.audio_log().iter().map(|r| format!("{r:?}")).collect();
        (h.screen_log().to_vec(), audio, h.quit_frame())
    };
    let plain = play(Harness::new(toy::game()));
    let painted = play(Harness::new(toy::game().with_module(A).unwrap()));
    assert!(plain.2.is_some() && !plain.1.is_empty(), "{plain:?}");
    assert_eq!(painted, plain);
}

/// A module with one tool on the debug menu.
struct Bodies;

impl Module<Toy> for Bodies {
    fn name(&self) -> &'static str {
        "bodies"
    }

    fn install(&mut self, install: &mut Install<'_, Toy>) {
        install.debug_entry(|| DebugEntry::new("Show bodies", |_| Transition::None));
    }
}

#[test]
fn a_modules_debug_entry_shows_last_on_the_debug_menu() {
    let hooks = GameHooks {
        debug: Some(ScreenHook {
            name: DebugMenuScreen::<Toy>::NAME,
            open: |ctx| {
                let gallery = DebugEntry::new("Gallery", |_| Transition::Push(Box::new(Gallery)));
                Box::new(DebugMenuScreen::new(ctx, vec![gallery]))
            },
        }),
        debug_screens: &[DebugMenuScreen::<Toy>::NAME],
        ..GameHooks::default()
    };
    let game = Game::new(toy::ctx(), Box::new(Ordered), hooks);
    let mut h = Harness::new(game.with_module(Bodies).unwrap());
    h.keys("F2");
    assert_eq!(h.top_screen(), "debug_menu");
    // The menu is drawn one tool a row, in its order.
    let shown = h.snapshot();
    let row = |label: &str| shown.lines().position(|l| l.contains(label));
    let (gallery, bodies) = (row("Gallery"), row("Show bodies"));
    assert!(gallery.is_some() && bodies > gallery, "{shown}");
    assert_eq!(h.game().modules()[0].debug_entries, ["Show bodies"]);
}

/// A module that notes what changed; it says it hears of it if `declared`.
struct Listener {
    name: &'static str,
    declared: bool,
    heard: Rc<RefCell<Vec<String>>>,
}

impl Module<Toy> for Listener {
    fn name(&self) -> &'static str {
        self.name
    }

    fn install(&mut self, install: &mut Install<'_, Toy>) {
        if self.declared {
            install.hook(Hook::Reloaded);
        }
    }

    fn reloaded(&mut self, _: &mut ToyCtx, changed: &[String]) {
        for path in changed {
            self.heard
                .borrow_mut()
                .push(format!("{}: {path}", self.name));
        }
    }
}

#[test]
fn a_changed_file_is_heard_in_install_order_by_those_that_declared_it() {
    let heard = Rc::new(RefCell::new(Vec::new()));
    let listener = |name, declared| Listener {
        name,
        declared,
        heard: Rc::clone(&heard),
    };
    let game = on(Ordered)
        .with_module(listener("first", true))
        .and_then(|game| game.with_module(listener("deaf", false)))
        .and_then(|game| game.with_module(listener("second", true)))
        .unwrap();
    let mut game = game;
    game.reloaded(&["a.ron".to_owned()]);
    assert_eq!(*heard.borrow(), ["first: a.ron", "second: a.ron"]);
    assert_eq!(game.modules()[0].hooks, [Hook::Reloaded]);
}

#[test]
fn a_reload_error_stays_until_that_values_next_good_reload() {
    use shuyi_assets::ContentError;
    use shuyi_assets::hot::Reload;

    let mut ctx = toy::ctx();
    let failed = |message: &str| Reload::Failed(vec![ContentError::new("a.ron", message)]);
    assert_eq!(ctx.reload_errors(), [] as [String; 0]);
    ctx.note_reload("a", &failed("first"));
    ctx.note_reload("b", &failed("second"));
    assert_eq!(ctx.reload_errors(), ["a.ron: first", "a.ron: second"]);
    ctx.note_reload("a", &Reload::Unchanged);
    assert_eq!(ctx.reload_errors().len(), 2);
    ctx.note_reload("b", &Reload::Reloaded);
    assert_eq!(ctx.reload_errors(), ["a.ron: first"]);
    ctx.note_reload("a", &failed("again"));
    assert_eq!(ctx.reload_errors(), ["a.ron: again"]);
    ctx.note_reload("a", &Reload::Reloaded);
    assert_eq!(ctx.reload_errors(), [] as [String; 0]);
}
