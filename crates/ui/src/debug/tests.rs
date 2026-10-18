//! The debug screens on the toy game.

use core::cell::Cell as Shared;

use alloc::rc::Rc;

use super::*;
use crate::frame::color::EngineColor;
use crate::frame::glyph_buffer::Cell;
use crate::input::keymap::Keymap;
use crate::toy::{Action, Gallery, Toy, ToyCtx, ctx};

type FrameInput = crate::screen::FrameInput<Action>;

fn frame(actions: &[Action]) -> FrameInput {
    FrameInput::new(actions.to_vec(), 0.0, vec![])
}

fn row(buf: &GlyphBuffer, y: i32) -> String {
    (0..i32::from(buf.width()))
        .map(|x| buf.get(x, y).map_or(' ', |c| c.glyph))
        .collect()
}

#[test]
fn the_hint_names_the_key_only_with_debug_tools() {
    let mut c = ctx();
    assert_eq!(hint(&c).as_deref(), Some("F2 debug"));
    c.debug_tools = false;
    assert_eq!(hint(&c), None);
}

#[test]
fn the_hint_is_hidden_when_debug_is_unbound() {
    let mut c = ctx();
    c.keymap = Keymap::new(
        Action::ALL
            .iter()
            .flat_map(|&a| c.keymap.chords_for(a).into_iter().map(move |ch| (ch, a)))
            .filter(|&(_, a)| a != Action::Debug),
        c.keymap.repeat(),
    );
    assert_eq!(hint(&c), None);
}

#[test]
fn the_hint_is_right_aligned_and_skipped_over_text() {
    let c = ctx();
    let black = c.palette.engine(EngineColor::Background);
    let empty_row = || GlyphBuffer::new(20, 1, Cell::new(' ', black, black));
    let mut buf = empty_row();
    buf.print(0, 0, "0123456789", black, black);
    glyph::paint_hint(&c.palette, "F2 debug", &mut buf, 0);
    assert_eq!(row(&buf, 0), "0123456789 F2 debug ");
    assert_eq!(
        buf.get(11, 0).unwrap().fg,
        c.palette.engine(EngineColor::TextDim)
    );
    // No gap before the hint's first cell, or text under it: no hint.
    for end in ["01234567890", "0123456789 F2 debu"] {
        let mut buf = empty_row();
        buf.print(0, 0, end, black, black);
        glyph::paint_hint(&c.palette, "F2 debug", &mut buf, 0);
        assert_eq!(row(&buf, 0).trim_end(), end);
    }
    // Text after the hint's cells (the right margin) doesn't matter.
    let mut buf = empty_row();
    buf.print(19, 0, "x", black, black);
    glyph::paint_hint(&c.palette, "F2 debug", &mut buf, 0);
    assert_eq!(row(&buf, 0), "           F2 debugx");
}

/// Two tools: one that opens the gallery, one that counts its runs and
/// says so in its name.
fn menu(c: &ToyCtx, runs: &Rc<Shared<u32>>) -> DebugMenuScreen<Toy> {
    let counted = Rc::clone(runs);
    let shown = Rc::clone(runs);
    DebugMenuScreen::new(
        c,
        vec![
            DebugEntry::new("Gallery", |_| Transition::Push(Box::new(Gallery))),
            DebugEntry::named(
                move |_| format!("Count: {}", shown.get()),
                move |ctx| {
                    counted.set(counted.get() + 1);
                    ctx.audio.play_sound("click");
                    Transition::None
                },
            ),
        ],
    )
}

#[test]
fn the_menu_runs_its_tools_and_reads_their_names_again() {
    use Action::{Cancel, Confirm, CursorDown};
    let mut c = ctx();
    let runs = Rc::new(Shared::new(0));
    let mut m = menu(&c, &runs);
    assert_eq!(m.name(), "debug_menu");
    let labels = |m: &DebugMenuScreen<Toy>| {
        let items = m.menu().items();
        items.iter().map(|i| i.label.clone()).collect::<Vec<_>>()
    };
    assert_eq!(labels(&m), ["Gallery", "Count: 0"]);
    let outcome = |m: &mut DebugMenuScreen<Toy>, c: &mut ToyCtx, a: &[Action]| {
        format!("{:?}", m.update(c, &frame(a)))
    };
    assert_eq!(outcome(&mut m, &mut c, &[Confirm]), "Push(gallery)");
    // A tool that opens nothing runs once a press, keeps the focus and is
    // named again; the keys after it go on.
    assert_eq!(
        outcome(&mut m, &mut c, &[CursorDown, Confirm, Confirm]),
        "None"
    );
    assert_eq!(runs.get(), 2);
    assert_eq!(labels(&m), ["Gallery", "Count: 2"]);
    assert_eq!(m.menu().focus(), 1);
    let cues: Vec<_> = c
        .audio
        .take()
        .iter()
        .filter_map(|r| r.cue().map(str::to_owned))
        .collect();
    assert_eq!(cues[cues.len() - 2..], ["ui_select", "click"]);
    assert_eq!(outcome(&mut m, &mut c, &[Cancel, Confirm]), "Pop");
    assert_eq!(runs.get(), 2);
    assert!(format!("{m:?}").contains("entries: 2"));
    assert!(
        format!("{:?}", DebugEntry::<Toy>::new("x", |_| Transition::None)).contains("DebugEntry")
    );
}

#[test]
fn the_view_is_the_title_the_tools_and_the_help() {
    let c = ctx();
    let runs = Rc::new(Shared::new(0));
    let m = menu(&c, &runs).focused(1);
    let view = m.view(&c);
    assert_eq!(view.title, "Debug tools");
    assert_eq!(view.menu, m.menu().view());
    assert_eq!(
        view.menu.focused().map(|i| i.label.as_str()),
        Some("Count: 0")
    );
    assert_eq!(view.help, "arrows move · f choose · d back");
    // Drawing is the skin painting the view: the title, the menu centred
    // under it, the help at the bottom.
    let black = c.palette.engine(EngineColor::Background);
    let spec = crate::frame::console::ConsoleSpec::text(40, 20);
    let mut drawn = GlyphBuffer::with_spec(spec, Cell::new('x', black, black));
    m.draw(&c, &mut drawn);
    assert_eq!(row(&drawn, 9).trim(), "Debug tools");
    assert!(
        row(&drawn, 12).contains("│ Gallery  │"),
        "{}",
        row(&drawn, 12)
    );
    assert_eq!(row(&drawn, 19).trim(), view.help);
    assert!(row(&drawn, 0).trim().is_empty());
}

/// A sample panel: a row of `#`s from `top`.
fn panels(buf: &mut GlyphBuffer, palette: &Palette, top: i32) {
    let c = palette.engine(EngineColor::Accent);
    buf.print(1, top, "####", c, c);
}

#[test]
fn the_sampler_shows_every_glyph_and_colour_and_the_games_panels() {
    let c = ctx();
    let glyphs: Vec<char> = ('!'..='~').collect();
    let spec = crate::frame::console::ConsoleSpec::text(100, 32);
    let b = glyph::sampler(spec, &c.palette, &glyphs, panels);
    assert_eq!((b.width(), b.height()), (100, 32));
    assert_eq!(row(&b, 0).trim(), format!("Glyphs ({})", glyphs.len()));
    let bottom = glyph::palette_top(&glyphs);
    assert_eq!(bottom, 3, "two rows of glyphs");
    let row_end = 2 + 2 * i32::try_from(glyph::GLYPHS_PER_ROW).unwrap();
    let shown: String = (1..bottom)
        .flat_map(|y| (2..row_end).step_by(2).map(move |x| (x, y)))
        .map(|(x, y)| b.get(x, y).unwrap().glyph)
        .collect();
    let expected: String = glyphs.iter().collect();
    assert!(shown.starts_with(&expected), "{shown}");
    assert!(row(&b, bottom).starts_with(" Palette ("));
    for (name, rgb) in c.palette.iter() {
        let found = (bottom..32).any(|y| {
            (0..100).any(|x| {
                let cell = b.get(x, y).unwrap();
                cell.glyph == '█' && cell.fg == rgb
            })
        });
        assert!(found, "no swatch for {name}");
    }
    let top = glyph::panels_top(&c.palette, &glyphs);
    assert_eq!(
        top,
        glyph::panels_top_for(c.palette.iter().count(), &glyphs)
    );
    assert!(row(&b, top - 1).starts_with(" The quick brown fox"));
    assert!(row(&b, top).starts_with(" ####"));
    // Five swatch columns: ten colours take two rows.
    assert_eq!(glyph::panels_top_for(10, &glyphs), bottom + 4);
    assert_eq!(glyph::panels_top_for(11, &glyphs), bottom + 5);
}

#[test]
fn the_sampler_screen_draws_the_sampler_and_closes_on_cancel() {
    let mut c = ctx();
    let glyphs = ['a', 'b'];
    let spec = crate::frame::console::ConsoleSpec::text(100, 32);
    let mut screen = GlyphSamplerScreen::new(&c, spec, &glyphs, panels);
    assert_eq!(Screen::<Toy>::name(&screen), "glyph_sampler");
    let black = c.palette.engine(EngineColor::Background);
    let mut buf = GlyphBuffer::with_spec(spec, Cell::new('x', black, black));
    Screen::<Toy>::draw(&screen, &c, &mut buf);
    assert_eq!(buf, glyph::sampler(spec, &c.palette, &glyphs, panels));
    let stay = screen.update(&mut c, &frame(&[Action::Confirm]));
    assert!(matches!(stay, Transition::None));
    assert!(c.audio.take().is_empty());
    let pop = screen.update(&mut c, &frame(&[Action::Confirm, Action::Cancel]));
    assert!(matches!(pop, Transition::Pop));
    assert_eq!(c.audio.take()[0].cue(), Some("ui_cancel"));
}

#[test]
fn the_swatches_go_five_to_a_row_with_their_names_cut() {
    use crate::frame::color::tests::def;
    let mut names: Vec<(String, [u8; 3])> = EngineColor::ALL
        .iter()
        .zip(0u8..)
        .map(|(c, i)| (c.name().to_owned(), [i, i, i]))
        .collect();
    names.push(("a_very_long_colour_name".to_owned(), [200, 1, 1]));
    let pairs: Vec<(&str, [u8; 3])> = names.iter().map(|(n, c)| (n.as_str(), *c)).collect();
    let palette = Palette::new::<EngineColor>(&def(&pairs)).unwrap();
    let glyphs = ['a'];
    let spec = crate::frame::console::ConsoleSpec::text(100, 32);
    let b = glyph::sampler(spec, &palette, &glyphs, panels);
    let top = glyph::palette_top(&glyphs) + 1;
    let colors: Vec<_> = palette.iter().collect();
    // Swatch `i` is at column 1 + 19 × (i mod 5), row `i / 5` under the
    // heading.
    // Two cells of colour, a space, then the name.
    for (i, &(name, rgb)) in (0..).zip(&colors) {
        let (x, y) = (1 + i % 5 * 19, top + i / 5);
        let glyph = |dx: i32| b.get(x + dx, y).unwrap().glyph;
        assert_ne!(glyph(-1), '█', "swatch {i} starts at {x}");
        for dx in [0, 1] {
            let cell = b.get(x + dx, y).unwrap();
            assert_eq!((cell.glyph, cell.fg), ('█', rgb), "swatch {i}");
        }
        assert_eq!(glyph(2), ' ');
        assert_eq!(Some(glyph(3)), name.chars().next(), "swatch {i}");
    }
    // Its name, after the swatch, cut at 14 characters.
    let at = colors
        .iter()
        .position(|(n, _)| n.starts_with("a_very"))
        .unwrap();
    let at = i32::try_from(at).unwrap();
    let (x, y) = (1 + at % 5 * 19 + 3, top + at / 5);
    let shown: String = row(&b, y)
        .chars()
        .skip(usize::try_from(x).unwrap())
        .take(16)
        .collect();
    assert_eq!(shown, "a_very_long_co  ");
}
