//! The frame's and the widgets' types that open tickets grow (ticket
//! 2082), built from outside the crate as a game builds them: through
//! their constructors, with a `_` arm on each enum (ADR-0039).

use shuyi_assets::image::{ImageInfo, ImageTable};
use shuyi_ui::dialogue::SkipView;
use shuyi_ui::frame::draw::PanelLook;
use shuyi_ui::frame::{BoxStyle, Layer, Overlay, Paint, Rect, Rgb, Sprite};
use shuyi_ui::grid::glyph::{CursorLook, FlashLook, RangeLook, ThreatLook};
use shuyi_ui::grid::{GlyphSkin, GlyphStyle, Grid, Projection};
use shuyi_ui::input::fit::{FitRule, ScreenActions};
use shuyi_ui::toy::Action;
use shuyi_ui::toy::grid::{COLORS, GLYPH};
use shuyi_ui::widgets::name_entry::{GridCell, GridView};
use shuyi_ui::widgets::{AnswerView, AskView, MenuView};

const RED: Rgb = Rgb::new(200, 0, 0);
const WHITE: Rgb = Rgb::new(255, 255, 255);
const BLUE: Rgb = Rgb::new(0, 0, 160);

#[test]
fn the_frames_types_are_built_through_their_constructors() {
    let table = ImageTable::new([("a.png", ImageInfo::new(16, 16))].into());
    let image = table.id("a.png").expect("a.png is in the table");
    let src = Rect::new(0, 0, 16, 16);
    let dest = Rect::new(8, 8, 32, 32);
    let mut sprite = Sprite::new(image, src, dest, Layer::Over).painted(Paint::Dimmed);
    sprite.opacity = 128;
    assert_eq!(
        (sprite.clip, sprite.paint, sprite.opacity),
        (dest, Paint::Dimmed, 128)
    );

    let overlay = Overlay::new(dest, RED, Layer::Under);
    assert_eq!((overlay.rect, overlay.color), (dest, RED));

    let name = match overlay.layer {
        Layer::Under => "under",
        Layer::Over => "over",
        _ => "newer",
    };
    assert_eq!(name, "under");
    let solid = match Paint::Solid(RED) {
        Paint::Solid(color) => Some(color),
        _ => None,
    };
    assert_eq!(solid, Some(RED));
}

#[test]
fn a_panel_look_is_built_with_new_and_its_with_methods() {
    const LOOK: PanelLook = PanelLook::new(BoxStyle::Double, WHITE, BLUE)
        .with_border(RED)
        .with_title(RED);
    assert_eq!(
        (LOOK.style, LOOK.border, LOOK.title, LOOK.text, LOOK.bg),
        (BoxStyle::Double, RED, RED, WHITE, BLUE)
    );
}

#[test]
fn a_glyph_style_and_skin_are_built_in_a_const() {
    const STYLE: GlyphStyle = GlyphStyle::PLAIN
        .with_range(RangeLook::Tint)
        .with_flash(FlashLook::Invert)
        .with_cursor(CursorLook::Flat)
        .with_threat(ThreatLook::Plain { edge: "text" })
        .with_side_cells(1);
    const SKIN: GlyphSkin = GLYPH.with_style(STYLE);
    assert_eq!(STYLE.flash, FlashLook::Invert);
    assert_eq!(STYLE.side_cells, 1);
    assert_eq!(SKIN.style, STYLE);
    assert_eq!(SKIN.colors, COLORS);
}

#[test]
fn a_grid_is_laid_top_down() {
    let grid = Grid::top_down(shuyi_core::Pos::new(0, 0), (0, 0), (16, 8), (4, 3), 1);
    let corner = match grid.projection {
        Projection::TopDown => grid.tile_px(shuyi_core::Pos::new(1, 1)),
        _ => (0, 0),
    };
    assert_eq!(corner, (16, 8));
}

#[test]
fn the_widgets_views_are_built_through_their_constructors() {
    let menu = MenuView::new(Vec::new());
    assert_eq!((menu.first_shown, menu.shown), (0, 0));

    let mut ask: AskView<bool> = AskView::new("Quit?", Vec::<AnswerView<bool>>::new());
    ask.keys = Some("f yes / d no".to_owned());
    assert_eq!((ask.text.as_str(), ask.focused()), ("Quit?", None));

    let grid = GridView::new("Name", "Ann", vec![vec!["A".to_owned()]], (0, 0));
    assert_eq!(
        (grid.title.as_str(), grid.spelled.as_str()),
        ("Name", "Ann")
    );
    let typed = match GridCell::Char('A') {
        GridCell::Char(c) => Some(c),
        _ => None,
    };
    assert_eq!(typed, Some('A'));

    let skip = SkipView::new("Skip the scene?", "f yes / d no");
    assert_eq!(skip.question, "Skip the scene?");
}

#[test]
fn screen_actions_are_built_through_their_constructors() {
    let declared = ScreenActions::declared("title", vec![Action::Confirm]);
    let open = ScreenActions::<Action>::undeclared("title");
    assert_eq!(declared.actions, Some(vec![Action::Confirm]));
    assert_eq!(open.actions, None);
    let error = match FitRule::Undeclared {
        FitRule::Undeclared => false,
        _ => true,
    };
    assert!(!error);
}
