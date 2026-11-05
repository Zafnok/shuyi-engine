//! A font atlas reloaded while the game runs: the App's test that the
//! frame takes its new widths, with `std` for the thread's note of it.
#![cfg(feature = "std")]

use super::*;

std::thread_local! {
    /// Whether the font atlas was reloaded, on this thread: then the toy's
    /// look is [`WIDE_FONT`].
    static ATLAS_RELOADED: core::cell::Cell<bool> = const { core::cell::Cell::new(false) };
}

/// The font after a reload: `あ` is wide.
static WIDE_FONT: std::sync::LazyLock<shuyi_assets::font::FontAtlasDef> =
    std::sync::LazyLock::new(|| shuyi_assets::font::FontAtlasDef {
        wide: [('あ', 0)].into(),
        ..shuyi_assets::font::FontAtlasDef::default()
    });

#[test]
fn a_reloaded_font_atlas_with_a_new_wide_glyph_prints_it_in_two_cells() {
    fn look(_: &Toy) -> Look<'_> {
        if ATLAS_RELOADED.with(core::cell::Cell::get) {
            Look::new(&WIDE_FONT, &IMAGES)
        } else {
            Look::new(&FONT, &IMAGES)
        }
    }
    let start = Start::new(Card::text("A toy"), look, |setup| {
        Ok(toy::game_on(setup.ctx(toy::ctx())))
    });
    let mut app = App::new(start);
    let mut p = HeadlessPlatform::new();
    let _ = app.frame(&mut p, Millis(0));
    let _ = app.frame(&mut p, Millis(16));
    let printed = |app: &App<Toy>| {
        let white = Rgb::new(255, 255, 255);
        running(app)
            .buffer()
            .clone()
            .print(0, 0, "あ", white, white)
    };
    assert_eq!(printed(&app), 1);
    // The game's atlas changes, but no file has yet: the widths stay.
    ATLAS_RELOADED.with(|reloaded| reloaded.set(true));
    let _ = app.frame(&mut p, Millis(32));
    assert_eq!(printed(&app), 1);
    let layer = shuyi_assets::bundle::Layer::owned("disk".to_owned(), BTreeMap::new());
    let layer = layer.unwrap_or_else(|error| panic!("{error:?}"));
    p.change_file(layer, "font/atlas.ron", b"(wide)".to_vec());
    let _ = app.frame(&mut p, Millis(48));
    // Files are told, and the atlas read again, only with debug tools.
    let cells = if shuyi_ui::DEBUG_TOOLS { 2 } else { 1 };
    assert_eq!(printed(&app), cells);
    assert_eq!(
        running(&app).ctx().widths().of_char('あ'),
        usize::from(cells)
    );
}
