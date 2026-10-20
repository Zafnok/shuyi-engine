//! What every screen here keeps to: its logic says what it shows, never how
//! it looks.

/// The logic modules of the screens say what they show, never how it
/// looks: none of them names a color, a rectangle, a box style or a cell,
/// and each `draw` is one call to its skin with its view (the glyph skin,
/// or the painter the game gave, ticket 1045).
#[test]
fn the_logic_modules_name_no_cells_or_colours() {
    let sources = [
        ("key_bindings.rs", include_str!("key_bindings.rs")),
        ("layout_picker.rs", include_str!("layout_picker.rs")),
        ("options.rs", include_str!("options.rs")),
    ];
    for (file, source) in sources {
        for name in ["EngineColor", "Rect", "BoxStyle", "Cell", "Rgb"] {
            assert!(!names_word(source, name), "{file} names `{name}`");
        }
        let draws = source.matches("fn draw(").count();
        let paints = source
            .lines()
            .filter(|l| {
                let skin = l.contains("glyph::paint") || l.contains("(self.paint)");
                skin && l.contains("(ctx, &self.view(ctx), buf);")
            })
            .count();
        assert!(
            draws > 0 && paints == draws,
            "{file}'s draw doesn't paint its view"
        );
    }
}

/// Whether `word` appears in `source` as a whole identifier.
fn names_word(source: &str, word: &str) -> bool {
    let ident = |c: char| c.is_alphanumeric() || c == '_';
    source.match_indices(word).any(|(at, _)| {
        let before = source[..at].chars().next_back();
        let after = source[at + word.len()..].chars().next();
        !before.is_some_and(ident) && !after.is_some_and(ident)
    })
}

#[test]
fn names_word_finds_whole_identifiers_only() {
    assert!(names_word("use crate::frame::color::Rgb;", "Rgb"));
    assert!(names_word("let r: Rect = x;", "Rect"));
    assert!(!names_word("let r = PxRect::new();", "Rect"));
    assert!(!names_word("let cells = 3;", "Cell"));
    assert!(!names_word("BoxStyles", "BoxStyle"));
}

#[test]
fn text_is_centred_rounding_left() {
    use crate::frame::color::{EngineColor, tests::engine_palette};
    use crate::frame::glyph_buffer::{Cell, GlyphBuffer};
    let p = engine_palette();
    let (fg, bg) = (p.engine(EngineColor::Text), p.engine(EngineColor::Panel));
    let mut buf = GlyphBuffer::new(10, 2, Cell::new(' ', fg, bg));
    assert_eq!(super::centre_x(&buf, 4), 3);
    assert_eq!(super::centre_x(&buf, 3), 3);
    assert_eq!(super::centre_x(&buf, 10), 0);
    assert_eq!(super::centre_x(&buf, 12), -1);
    super::print_centred(&mut buf, 1, "abcd", fg, bg);
    let row: alloc::string::String = (0..10).map(|x| buf.get(x, 1).unwrap().glyph).collect();
    assert_eq!(row, "   abcd   ");
}

#[test]
fn a_text_wider_than_the_buffer_starts_at_column_0_cut_with_an_ellipsis() {
    use crate::frame::color::{EngineColor, tests::engine_palette};
    use crate::frame::glyph_buffer::{Cell, GlyphBuffer};
    let p = engine_palette();
    let (fg, bg) = (p.engine(EngineColor::Text), p.engine(EngineColor::Panel));
    let mut buf = GlyphBuffer::new(10, 1, Cell::new(' ', fg, bg));
    super::print_centred(&mut buf, 0, "abcdefghijklmn", fg, bg);
    let row: alloc::string::String = (0..10).map(|x| buf.get(x, 0).unwrap().glyph).collect();
    assert_eq!(row, "abcdefghi…");
}

#[test]
fn cut_keeps_a_text_that_fits_and_ends_a_longer_one_with_an_ellipsis() {
    assert_eq!(super::cut("abc", 3), "abc");
    assert_eq!(super::cut("abcd", 3), "ab…");
    assert_eq!(super::cut("abcd", 1), "…");
    assert_eq!(super::cut("abcd", 0), "");
}

/// What each engine screen declares (ADR-0025), with the toy's actions:
/// the layout picker its menu's, the others the cursor, Confirm and
/// Cancel.
#[test]
fn every_screen_declares_its_actions() {
    use crate::prelude::*;
    use crate::screen::Screen;
    use crate::toy::{self, Action, Toy};
    use Action::{Cancel, Confirm, CursorDown, CursorLeft, CursorRight, CursorUp};
    let c = toy::ctx();
    let navigation = Some(vec![
        CursorLeft,
        CursorDown,
        CursorUp,
        CursorRight,
        Confirm,
        Cancel,
    ]);
    let screens: [(Box<dyn Screen<Toy>>, _); 4] = [
        (
            Box::new(super::options::OptionsScreen::new(toy::option_rows)),
            navigation.clone(),
        ),
        (
            Box::new(super::key_bindings::KeyBindingsScreen::new(
                &c,
                &toy::KEY_ROWS,
            )),
            navigation.clone(),
        ),
        (
            Box::new(super::profiles::ProfilesScreen::new(&c)),
            navigation,
        ),
        (
            Box::new(super::layout_picker::LayoutPickerScreen::new(
                &c,
                &toy::LEGEND,
            )),
            Some(vec![CursorDown, CursorUp, Confirm, Cancel]),
        ),
    ];
    for (screen, declared) in screens {
        assert_eq!(screen.actions(), declared, "{}", screen.name());
    }
}
