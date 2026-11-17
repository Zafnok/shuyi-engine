//! A game that turns key caps on (ticket 3215): its frame draws them, and
//! a one-bit screen doesn't unless the style says.

use shuyi_assets::audio::AudioManifest;
use shuyi_assets::keymap::KeymapDef;
use shuyi_assets::lang::Lang;
use shuyi_assets::palette::PaletteDef;

use super::*;
use crate::frame::GlyphStyle;
use crate::frame::color::{KEY_BG, Palette};
use crate::frame::console::ConsoleSpec;
use crate::frame::key_cap::KeyCapStyle;
use crate::screen::Transition;
use crate::screens::print_centred_styled;
use crate::text::StyledText;
use crate::toy::{Action, Prefs, Theme, Toy, Voice};

/// The toy, with key caps in `style`.
#[derive(Debug)]
struct Capped(Toy, Option<KeyCapStyle>);

impl GameData for Capped {
    type Action = Action;
    type Colors = EngineColor;
    type Voice = Voice;
    type Settings = Prefs;
    fn lang(&self) -> &Lang {
        self.0.lang()
    }
    fn keymap_def(&self) -> &KeymapDef<Action> {
        self.0.keymap_def()
    }
    fn audio_manifest(&self) -> &AudioManifest {
        self.0.audio_manifest()
    }
    fn palette_def(&self) -> &PaletteDef {
        self.0.palette_def()
    }
    fn console(&self) -> ConsoleSpec {
        self.0.console()
    }
    fn theme_def(&self, theme: Theme) -> Option<&PaletteDef> {
        self.0.theme_def(theme)
    }
    fn voice_variant(&self) -> Voice {
        Voice::Only
    }
    fn key_caps(&self) -> Option<KeyCapStyle> {
        self.1
    }
}

/// A help line at the bottom, centred as the engine's screens print one.
struct Help;

impl Screen<Capped> for Help {
    fn name(&self) -> &'static str {
        "help"
    }
    fn update(&mut self, _: &mut Ctx<Capped>, _: &FrameInput<Action>) -> Transition<Capped> {
        Transition::None
    }
    fn draw(&self, ctx: &Ctx<Capped>, buf: &mut GlyphBuffer) {
        let mut help = StyledText::new();
        help.push_key("f").push_plain(" select");
        let text = ctx.palette.engine(EngineColor::Text);
        let panel = ctx.palette.engine(EngineColor::Panel);
        print_centred_styled(buf, 0, &help, ctx.palette.text_colors_on(text, panel));
    }
}

fn game(style: Option<KeyCapStyle>, one_bit: bool) -> Game<Capped> {
    let mut ctx = Ctx::new(Capped(Toy::new(), style)).unwrap();
    let mut boxed = ctx.game.palette_def().clone();
    boxed.colors.insert(KEY_BG.to_owned(), [74, 64, 46]);
    ctx.palette = Palette::new::<EngineColor>(&boxed).unwrap();
    ctx.platform.one_bit = one_bit;
    Game::new(ctx, Box::new(Help), GameHooks::default())
}

#[test]
fn a_game_with_no_style_draws_keys_as_text() {
    let game = game(None, false);
    assert!(game.ctx().key_caps().is_none());
    assert!(game.buffer().key_caps().is_none());
    assert_eq!(game.buffer().widths().key_cap_cells(), None);
    assert!(game.buffer().glyph_styles().next().is_none());
}

#[test]
fn a_game_with_caps_draws_its_keys_as_caps_in_every_frame() {
    let mut game = game(Some(KeyCapStyle::new()), false);
    let caps = game.ctx().key_caps().unwrap();
    assert_eq!(game.buffer().key_caps(), Some(&caps));
    assert_eq!(game.buffer().widths().key_cap_cells(), Some(caps.extra_cells()));
    // "F select" and the cap's cell, centred: from column 15 of 40.
    let width = i32::from(game.buffer().width());
    let start = (width - 9) / 2;
    let glyph = |x: i32| game.buffer().get(x, 0).map(|c| c.glyph);
    assert_eq!(glyph(start), Some('F'));
    assert!(game.buffer().glyph_style(start, 0).bold);
    assert_ne!(game.buffer().glyph_style(start, 0), GlyphStyle::PLAIN);
    assert_eq!(game.buffer().overlays().len(), 5);
    // Drawn again, the same: the widths aren't set anew each frame.
    let before = game.buffer().clone();
    game.redraw();
    assert_eq!(*game.buffer(), before);
}

#[test]
fn a_one_bit_screen_draws_keys_as_text_unless_the_style_says() {
    let plain = game(Some(KeyCapStyle::new()), true);
    assert!(plain.ctx().key_caps().is_none());
    assert!(plain.buffer().glyph_styles().next().is_none());
    assert!(plain.buffer().overlays().is_empty());
    let capped = game(Some(KeyCapStyle::new().with_one_bit(true)), true);
    assert!(capped.ctx().key_caps().is_some());
    assert!(capped.buffer().glyph_styles().next().is_some());
}
