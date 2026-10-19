//! The tactics kit's grid map through the Harness (ticket 1115): the toy's
//! 10 × 8 field with two pieces, under the glyph skin and the sprite skin.
//! Pick your piece up, see where it can go, steer its path round the foe
//! and walk it there at the skin's pace. What happened is read from the
//! scene; how it looks, from a snapshot of each skin.

use insta::assert_snapshot;
use shuyi_core::Pos;
use shuyi_ui::grid::{GridSkin, MapScene, MapUnit};
use shuyi_ui::harness::Harness;
use shuyi_ui::toy::grid::{AREA, Field, GLYPH, MOVE, Piece, SPRITE_WALK};
use shuyi_ui::toy::{self, Toy};
use shuyi_ui::{Game, GameHooks};

fn p(x: i32, y: i32) -> Pos {
    Pos::new(x, y)
}

/// The toy game showing the field under the glyph skin.
fn glyphs() -> Harness<Toy> {
    Harness::new(Game::new(
        toy::ctx(),
        Box::new(Field::glyphs()),
        GameHooks::default(),
    ))
}

/// The toy game showing the field under the sprite skin.
fn pictures() -> Harness<Toy> {
    let ctx = toy::ctx();
    let field = Field::pictures(&ctx);
    Harness::new(Game::new(ctx, Box::new(field), GameHooks::default()))
}

fn field(h: &Harness<Toy>) -> &Field {
    h.screen::<Field>()
        .unwrap_or_else(|| panic!("no field: {:?}", h.screens()))
}

fn scene(h: &Harness<Toy>) -> MapScene<Piece> {
    field(h).scene()
}

/// Your piece picked up, its path steered down, right and right again,
/// round the foe below it: the move range and the path as they are then.
fn steered(h: &mut Harness<Toy>) {
    // The cursor starts on your piece at (2, 3).
    assert_eq!(scene(h).cursor_tile(), Some(p(2, 3)));
    h.keys("f");
    h.keys("Down Down Right Right");
}

#[test]
fn the_field_shows_the_map_its_pieces_and_the_cursor_in_both_skins() {
    for h in [glyphs(), pictures()] {
        let s = scene(&h);
        let name = field(&h).skin.name();
        // The 10 × 8 field is smaller than the view: centred in it, every
        // tile of it shown.
        assert_eq!(s.size, field(&h).skin.view_tiles(AREA), "{name}");
        for pos in (0..8).flat_map(|y| (0..10).map(move |x| p(x, y))) {
            assert!(s.terrain_at(pos).is_some(), "{name}: {pos:?}");
        }
        assert_eq!(s.units.len(), 2, "{name}");
        assert_eq!(
            s.unit_at(p(2, 3)).map(MapUnit::text).as_deref(),
            Some("#1 y (2,3)")
        );
        assert_eq!(s.unit_at(p(3, 4)).map(|u| u.letter), Some('f'));
        assert!(s.ranges.is_empty(), "{name}");
        assert!(s.path.is_empty(), "{name}");
    }
    // The skins fit their own number of tiles: 20 × 20 two-cell glyph
    // tiles, 13 × 13 tiles of 24 px.
    assert_eq!(scene(&glyphs()).size, (20, 20));
    assert_eq!(scene(&pictures()).size, (13, 13));
}

#[test]
fn a_picked_up_piece_shows_where_it_can_go_and_its_path_follows_the_cursor() {
    for mut h in [glyphs(), pictures()] {
        steered(&mut h);
        let s = scene(&h);
        // Four steps, never onto the water or the foe: the range is the
        // field's, whatever the skin.
        let range = s.tinted(MOVE);
        assert_eq!(range.len(), 32, "{}", field(&h).skin.name());
        assert!(range.contains(&p(2, 7)) && range.contains(&p(4, 2)));
        assert!(!range.contains(&p(3, 4)), "the foe's tile");
        assert!(!range.contains(&p(5, 3)), "the water");
        assert!(range.contains(&p(5, 2)), "the bridge");
        assert!(!range.contains(&p(6, 2)), "one step too far");
        // The path went round the foe, as steered.
        assert_eq!(s.path, [p(2, 3), p(2, 4), p(2, 5), p(3, 5), p(4, 5)]);
        assert_eq!(s.cursor_tile(), Some(p(4, 5)));
        // Back onto the path: cut back to it.
        h.keys("Left");
        assert_eq!(scene(&h).path, [p(2, 3), p(2, 4), p(2, 5), p(3, 5)]);
        // Cancel puts it down.
        h.keys("d");
        assert!(scene(&h).ranges.is_empty());
        assert!(scene(&h).path.is_empty());
    }
}

#[test]
fn a_piece_walks_its_path_at_the_skins_pace() {
    let (glyph, sprite) = (GridSkin::<Piece>::walk(&GLYPH), SPRITE_WALK);
    assert!(sprite.normal < glyph.normal);
    for (mut h, pace) in [(glyphs(), glyph.normal), (pictures(), sprite.normal)] {
        steered(&mut h);
        h.keys("f");
        // Two and a half tiles along its four steps, by its skin's pace.
        h.wait(2.5 / pace);
        let mid = scene(&h);
        let name = field(&h).skin.name();
        assert_eq!(mid.unit(1).map(|u| u.pos), Some(p(2, 5)), "{name}");
        // No cursor while it walks; the range and path are gone.
        assert_eq!(mid.cursor, None, "{name}");
        assert!(mid.ranges.is_empty() && mid.path.is_empty(), "{name}");
        h.wait(2.0 / pace);
        let done = scene(&h);
        assert_eq!(done.unit(1).map(|u| u.pos), Some(p(4, 5)), "{name}");
        assert_eq!(field(&h).pieces[0].pos, p(4, 5), "{name}");
        assert_eq!(done.cursor_tile(), Some(p(4, 5)), "{name}");
    }
}

#[test]
fn a_tile_out_of_reach_is_not_walked_to() {
    let mut h = glyphs();
    h.keys("f Right Right Right Right Right Right");
    // Past the river: no path there, nothing walks.
    let s = scene(&h);
    assert_eq!(s.cursor_tile(), Some(p(8, 3)));
    assert_eq!(s.path, [p(2, 3), p(3, 3), p(4, 3)]);
    h.keys("f");
    assert!(field(&h).walking.is_none());
    assert_eq!(field(&h).pieces[0].pos, p(2, 3));
    // Confirm off your piece picks nothing up.
    h.keys("d Left f");
    assert!(field(&h).selected.is_none());
}

/// The field with your piece's range and path, under the glyph skin.
#[test]
fn glyph_field_snapshot() {
    let mut h = glyphs();
    steered(&mut h);
    assert_snapshot!(h.snapshot());
}

/// The same under the sprite skin of the toy's tiles.
#[test]
fn sprite_field_snapshot() {
    let mut h = pictures();
    steered(&mut h);
    assert_snapshot!(h.snapshot());
}
