//! The debug view (ADR-0085) through the Harness: screens hand their
//! gizmos in only with debug tools on, bottom up; the game paints them
//! over everything, the modules' drawing too; the toy's grid field offers
//! the grid kit's `grid` channel, which the debug menu's "Debug view"
//! turns on and a later launch remembers; and looking changes nothing.

use std::cell::Cell as Shared;
use std::rc::Rc;

use insta::assert_snapshot;
use shuyi_platform::harness::Harness;
use shuyi_ui::debug::gizmo::{Channel, Gizmo, GizmoRole, Gizmos};
use shuyi_ui::debug::{DebugMenuScreen, DebugViewScreen};
use shuyi_ui::frame::{EngineColor, GizmoItem, GizmoShape, GlyphBuffer, Layer, Overlay, Rect};
use shuyi_ui::grid::debug::GRID;
use shuyi_ui::module::{Hook, Install, Module};
use shuyi_ui::storage::{MemoryStorage, Storage};
use shuyi_ui::toy::grid::Field;
use shuyi_ui::toy::{self, Action, Toy, ToyCtx};
use shuyi_ui::{FrameInput, Game, GameHooks, Screen, ScreenHook, Transition};

/// A channel of the tests' own.
const MARKS: Channel = Channel::new("marks", "Marks");

/// A screen that counts its `debug_draw` calls and hands in one rectangle
/// at `x` on [`MARKS`].
struct Marked {
    name: &'static str,
    x: i32,
    overlay: bool,
    calls: Rc<Shared<u32>>,
}

impl Marked {
    fn new(name: &'static str, x: i32, overlay: bool) -> (Self, Rc<Shared<u32>>) {
        let calls = Rc::new(Shared::new(0));
        let screen = Self {
            name,
            x,
            overlay,
            calls: Rc::clone(&calls),
        };
        (screen, calls)
    }
}

impl Screen<Toy> for Marked {
    fn name(&self) -> &'static str {
        self.name
    }

    fn update(&mut self, _: &mut ToyCtx, _: &FrameInput<Action>) -> Transition<Toy> {
        Transition::None
    }

    fn draw(&self, _: &ToyCtx, _: &mut GlyphBuffer) {}

    fn is_overlay(&self) -> bool {
        self.overlay
    }

    fn debug_draw(&self, _: &ToyCtx, out: &mut Gizmos) {
        self.calls.set(self.calls.get() + 1);
        let rect = Rect::new(self.x, 0, 8, 8);
        out.push(MARKS, GizmoRole::Solid, Gizmo::Rect { rect });
    }
}

/// The gizmos' shapes in the frame, in drawing order.
fn shapes(h: &Harness<Toy>) -> Vec<GizmoShape> {
    h.game().buffer().gizmos().iter().map(|g| g.shape).collect()
}

/// No gizmo.
const NONE: [GizmoShape; 0] = [];

fn rect_at(x: i32) -> GizmoShape {
    GizmoShape::Rect {
        rect: Rect::new(x, 0, 8, 8),
    }
}

#[test]
fn with_debug_tools_off_no_screen_is_asked() {
    let mut ctx = toy::ctx();
    ctx.debug_tools = false;
    ctx.debug_view.set("marks", true);
    let (screen, calls) = Marked::new("marked", 0, false);
    let mut h = Harness::new(Game::new(ctx, Box::new(screen), GameHooks::default()));
    h.keys("f").wait(0.5);
    assert_eq!(calls.get(), 0);
    assert_eq!(shapes(&h), NONE);
    assert_eq!(h.ctx().debug_view.offered(), []);
}

#[test]
fn with_debug_tools_on_every_screen_drawn_is_asked_bottom_up() {
    let (base, base_calls) = Marked::new("base", 0, false);
    let (over, _) = Marked::new("over", 16, true);
    // Asked, and offering its channel, with no channel on.
    let game = Game::new(toy::ctx(), Box::new(base), GameHooks::default());
    let mut h = Harness::new(game);
    h.wait(0.1);
    assert!(base_calls.get() > 0);
    assert_eq!(h.ctx().debug_view.offered(), [MARKS]);
    assert_eq!(shapes(&h), NONE);
    // On: drawn from the next frame.
    h.ctx_mut().debug_view.set("marks", true);
    h.wait(0.1);
    assert_eq!(shapes(&h), [rect_at(0)]);
    // An overlay's gizmos come after the screen's under it.
    let mut stacked = toy::ctx();
    stacked.debug_view.set("marks", true);
    let (base, _) = Marked::new("base", 0, false);
    let mut stack = shuyi_ui::screen::ScreenStack::new(Box::new(base));
    stack.push(Box::new(over));
    let mut h = Harness::new(Game::with_stack(stacked, stack, GameHooks::default()));
    h.wait(0.1);
    assert_eq!(shapes(&h), [rect_at(0), rect_at(16)]);
    // In the debug skin's colour for the role, never the game's.
    let solid = h.ctx().palette.engine(EngineColor::Warning);
    assert!(h.game().buffer().gizmos().iter().all(|g| g.color == solid));
}

/// A module that draws a rectangle over the screens.
struct Painter;

impl Module<Toy> for Painter {
    fn name(&self) -> &'static str {
        "painter"
    }

    fn install(&mut self, install: &mut Install<'_, Toy>) {
        install.hook(Hook::DrawOver);
    }

    fn draw_over(&self, ctx: &ToyCtx, buf: &mut GlyphBuffer) {
        let color = ctx.palette.engine(EngineColor::Accent);
        buf.add_overlay(Overlay::new(Rect::new(0, 0, 16, 16), color, Layer::Over));
    }
}

#[test]
fn the_gizmos_are_drawn_over_what_the_modules_draw() {
    let mut ctx = toy::ctx();
    ctx.debug_view.set("marks", true);
    let (screen, _) = Marked::new("marked", 4, false);
    let game = Game::new(ctx, Box::new(screen), GameHooks::default())
        .with_module(Painter)
        .unwrap_or_else(|e| panic!("{e}"));
    let mut h = Harness::new(game);
    h.wait(0.1);
    let items = h.game().buffer().items();
    assert_eq!(items.len(), 2);
    assert!(matches!(items[0], shuyi_ui::frame::Item::Rect(_)));
    assert!(matches!(items[1], shuyi_ui::frame::Item::Gizmo(_)));
    assert_snapshot!(h.snapshot());
}

/// The toy's grid field under the glyph skin, or under the sprite skin,
/// with the `grid` channel on.
fn field(pictures: bool) -> Harness<Toy> {
    let mut ctx = toy::ctx();
    ctx.debug_view.set("grid", true);
    let field = if pictures {
        Field::pictures(&ctx)
    } else {
        Field::glyphs()
    };
    Harness::new(Game::new(ctx, Box::new(field), GameHooks::default()))
}

/// Each gizmo's role and shape's kind, in drawing order, runs of the same
/// counted: `bounds line ×42`.
fn kinds(gizmos: &[GizmoItem]) -> Vec<String> {
    let kind = |g: &GizmoItem| {
        let shape = match g.shape {
            GizmoShape::Rect { .. } => "rect",
            GizmoShape::Line { .. } => "line",
            GizmoShape::Label { .. } => "label",
            _ => "other",
        };
        format!("{} {shape}", g.role.name())
    };
    let mut runs: Vec<(String, usize)> = Vec::new();
    for g in gizmos {
        let k = kind(g);
        match runs.last_mut() {
            Some((last, n)) if *last == k => *n += 1,
            _ => runs.push((k, 1)),
        }
    }
    runs.into_iter().map(|(k, n)| format!("{k} ×{n}")).collect()
}

#[test]
fn the_grid_channel_shows_the_cell_lines_the_bounds_the_cursor_and_the_units() {
    for pictures in [false, true] {
        let h = field(pictures);
        let gizmos = h.game().buffer().gizmos();
        assert!(gizmos.iter().all(|g| g.channel == "grid"));
        let field = h.screen::<Field>().unwrap();
        let (across, down) = field.scene().size;
        // The field fits the view: no edge, and its cursor is on your
        // piece, so the cursor's cell and that piece's are the same.
        let lines = usize::try_from(across + 1 + down + 1).unwrap();
        assert_eq!(
            kinds(&gizmos),
            [
                format!("bounds line ×{lines}"),
                "bounds rect ×1".to_owned(),
                "note rect ×2".to_owned(),
                "note label ×1".to_owned(),
                "note rect ×1".to_owned(),
                "note label ×1".to_owned(),
            ],
            "pictures: {pictures}"
        );
        assert!(h.ctx().debug_view.offered().contains(&GRID));
    }
}

#[test]
fn glyph_field_with_the_grid_channel_snapshot() {
    assert_snapshot!(field(false).snapshot());
}

#[test]
fn sprite_field_with_the_grid_channel_snapshot() {
    assert_snapshot!(field(true).snapshot());
}

/// The toy's grid field on the context a launch finds in `storage`, with
/// the right-handed keys, whose Debug key opens the debug menu of the
/// engine's own tools.
fn with_debug_menu(storage: Box<dyn Storage>) -> Harness<Toy> {
    let mut ctx = toy::launch(storage);
    ctx.use_layout(toy::RIGHT_HANDED);
    let hooks = GameHooks {
        debug: Some(ScreenHook {
            name: DebugMenuScreen::<Toy>::NAME,
            open: |ctx| Box::new(DebugMenuScreen::new(ctx, Vec::new())),
        }),
        debug_screens: &[DebugMenuScreen::<Toy>::NAME, DebugViewScreen::NAME],
        ..GameHooks::default()
    };
    Harness::new(Game::new(ctx, Box::new(Field::glyphs()), hooks))
}

#[test]
fn the_debug_view_turns_the_grid_on_from_the_debug_menu_and_a_later_launch_keeps_it() {
    let mut h = with_debug_menu(Box::new(MemoryStorage::new()));
    assert_eq!(shapes(&h), NONE);
    // The menu hides the field, but its channel was offered before.
    h.keys("F2");
    assert_eq!(h.top_screen(), DebugMenuScreen::<Toy>::NAME);
    h.act(Action::Confirm);
    assert_eq!(h.top_screen(), DebugViewScreen::NAME);
    assert_snapshot!("debug_view_screen", h.snapshot());
    h.act(Action::Confirm);
    assert!(h.ctx().debug_view.is_on("grid"));
    assert_snapshot!("debug_view_screen_grid_on", h.snapshot());
    h.act(Action::Cancel).act(Action::Cancel);
    assert_eq!(h.top_screen(), "field");
    assert_ne!(shapes(&h), NONE);

    // Saved in the computer's storage: the next launch has it on.
    let storage = h.into_storage();
    assert!(storage.read("debug_view").unwrap().is_some());
    let mut again = with_debug_menu(storage);
    again.wait(0.1);
    assert!(again.ctx().debug_view.is_on("grid"));
    assert_ne!(shapes(&again), NONE);
    // And off again.
    again.keys("F2").act(Action::Confirm).act(Action::Confirm);
    assert!(!again.ctx().debug_view.is_on("grid"));
    let later = with_debug_menu(again.into_storage());
    assert!(!later.ctx().debug_view.is_on("grid"));
}

#[test]
fn a_launch_without_debug_tools_never_reads_the_debug_view() {
    let mut storage = MemoryStorage::new();
    storage.write("debug_view", r#"(on: ["grid"])"#).unwrap();
    let mut ctx = toy::first_launch();
    ctx.debug_tools = false;
    let ctx = ctx.with_storage(Box::new(storage));
    assert!(!ctx.debug_view.any_on());
}

#[test]
fn looking_changes_nothing_a_replay_has_the_same_hashes_with_every_channel_on() {
    let play = |all: bool| {
        let mut ctx = toy::ctx();
        if all {
            ctx.debug_view.show_all();
        }
        let mut h = Harness::new(Game::new(
            ctx,
            Box::new(Field::glyphs()),
            GameHooks::default(),
        ));
        let mut hashes = Vec::new();
        for keys in ["f", "Down Down", "Right Right", "f", "Up"] {
            h.keys(keys).wait(0.5);
            hashes.push(h.game().state_hash());
        }
        (hashes, h.game().buffer().gizmos().len())
    };
    let (none, none_drawn) = play(false);
    let (all, all_drawn) = play(true);
    assert!(none.iter().all(Option::is_some));
    assert_eq!(none, all);
    assert_eq!(none_drawn, 0);
    assert!(all_drawn > 0);
}
