//! The debug view (ADR-0085) on Hello, Grid: its stage offers the grid
//! kit's `grid` channel, and with it on draws the map's layout as the
//! player's skin lays it out, the cursor's cell among it; with it off,
//! nothing.

use hello_grid::screens::StageScreen;
use hello_grid::screens::stage::area;
use hello_grid::{Hello, MapLook};
use shuyi::platform::harness::Harness;
use shuyi::ui::frame::{GizmoItem, GizmoShape, PxRect};
use shuyi::ui::grid::GridSkin;
use shuyi::ui::grid::debug::GRID;
use shuyi::ui::{Game, GameHooks};

/// The stage in `look`, its scene seen, with the `grid` channel on or off
/// from the start.
fn stage(look: MapLook, grid: bool) -> Harness<Hello> {
    let mut ctx = hello_grid::ctx(1);
    ctx.debug_tools = true;
    ctx.preview_settings(|s| s.game.look = look);
    ctx.debug_view.set(GRID.id(), grid);
    let stage = StageScreen::new(&mut ctx).briefed();
    let mut h = Harness::new(Game::new(ctx, Box::new(stage), GameHooks::default()));
    h.wait(0.1);
    h
}

/// The cursor's cell on screen, as the skin of `look` lays the stage out.
fn cursor_cell(h: &Harness<Hello>, look: MapLook) -> PxRect {
    let ctx = h.ctx();
    let area = area(ctx.console);
    let screen = crate::stage(h);
    let cell = |skin: &dyn GridSkin<_>| {
        let scene = screen.scene(skin.view_tiles(area));
        scene
            .cursor_tile()
            .and_then(|pos| skin.grid(&scene, area).rect(pos))
            .unwrap_or_else(|| panic!("no cursor in view"))
    };
    match look {
        MapLook::Glyphs => cell(&hello_grid::map::GLYPHS),
        MapLook::Tiles => cell(&ctx.game.sprite_skin()),
    }
}

fn gizmos(h: &Harness<Hello>) -> Vec<GizmoItem> {
    h.game().buffer().gizmos()
}

#[test]
fn the_stage_offers_the_grid_channel_and_draws_its_layout_with_it_on() {
    for look in [MapLook::Glyphs, MapLook::Tiles] {
        let h = stage(look, true);
        let drawn = gizmos(&h);
        assert!(drawn.iter().all(|g| g.channel == GRID.id()), "{look:?}");
        let cursor = GizmoShape::Rect {
            rect: cursor_cell(&h, look),
        };
        assert!(drawn.iter().any(|g| g.shape == cursor), "{look:?}");
        assert!(h.ctx().debug_view.offered().contains(&GRID));
    }
}

#[test]
fn with_the_grid_channel_off_the_stage_draws_none() {
    for look in [MapLook::Glyphs, MapLook::Tiles] {
        let h = stage(look, false);
        assert_eq!(gizmos(&h), [], "{look:?}");
        assert!(h.ctx().debug_view.offered().contains(&GRID));
    }
}
