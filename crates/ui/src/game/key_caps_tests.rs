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
use crate::platform_facts::ScreenColor;
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
    /// `{Move}` is the cursor keys too, as a game may name them.
    fn key_text(
        &self,
        name: &str,
        keys: crate::input::help_keys::HelpKeys<'_, Action>,
    ) -> Option<String> {
        if name == "Move" {
            return Some(crate::input::help_keys::cursor_keys_name(keys));
        }
        crate::text::key_text(name, keys)
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

/// The cursor's keys and Confirm's, as the engine's screens print a help
/// line: `arrows move · f select`.
struct CursorHelp;

impl Screen<Capped> for CursorHelp {
    fn name(&self) -> &'static str {
        "cursor help"
    }
    fn update(&mut self, _: &mut Ctx<Capped>, _: &FrameInput<Action>) -> Transition<Capped> {
        Transition::None
    }
    fn draw(&self, ctx: &Ctx<Capped>, buf: &mut GlyphBuffer) {
        let keys = ctx.help_keys();
        let mut help = crate::input::help_keys::cursor_keys_styled(keys);
        help.push_plain(" move · ")
            .push_key(&crate::input::help_keys::key_name(keys, Action::Confirm))
            .push_plain(" select");
        let text = ctx.palette.engine(EngineColor::Text);
        let panel = ctx.palette.engine(EngineColor::Panel);
        print_centred_styled(buf, 0, &help, ctx.palette.text_colors_on(text, panel));
    }
}

fn game(style: Option<KeyCapStyle>, one_bit: bool) -> Game<Capped> {
    game_on(style, one_bit, Box::new(Help))
}

fn game_on(
    style: Option<KeyCapStyle>,
    one_bit: bool,
    screen: Box<dyn Screen<Capped>>,
) -> Game<Capped> {
    let mut ctx = Ctx::new(Capped(Toy::new(), style)).unwrap();
    ctx.use_layout(crate::toy::RIGHT_HANDED);
    let mut boxed = ctx.game.palette_def().clone();
    boxed.colors.insert(KEY_BG.to_owned(), [74, 64, 46]);
    ctx.palette = Palette::new::<EngineColor>(&boxed).unwrap();
    if one_bit {
        ctx.platform.screen = ScreenColor::OneBit;
    }
    Game::new(ctx, screen, GameHooks::default())
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
    assert_eq!(
        game.buffer().widths().key_cap_cells(),
        Some(caps.extra_cells())
    );
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
    assert_eq!(plain.buffer().overlays().len(), 0);
    let capped = game(Some(KeyCapStyle::new().with_one_bit(true)), true);
    assert!(capped.ctx().key_caps().is_some());
    assert!(capped.buffer().glyph_styles().next().is_some());
}

/// Key caps with a made-up 24 × 16 picture of the arrow cluster.
fn arrows_style() -> KeyCapStyle {
    use shuyi_assets::image::{ImageInfo, ImageTable};
    let table = ImageTable::new([("arrows.png", ImageInfo::new(24, 16))].into());
    let image = table.id("arrows.png").unwrap();
    KeyCapStyle::new().with_arrows(image, crate::frame::Rect::new(0, 0, 24, 16))
}

#[test]
fn the_cursor_keys_in_a_games_text_are_one_arrows_span() {
    use crate::text::TextRole;
    let game = game(Some(arrows_style()), false);
    let ctx = game.ctx();
    let help = ctx.text_styled("home.help", &[]);
    assert_eq!(help.plain(), ctx.text_with("home.help", &[]));
    let arrows: Vec<&str> = help
        .spans()
        .iter()
        .filter(|s| s.role == Some(TextRole::Arrows))
        .map(|s| s.text.as_str())
        .collect();
    assert_eq!(arrows, ["arrows"]);
    // A game's own name for the cursor keys gives the same span; an
    // action's key stays a key.
    let keys = ctx.help_keys();
    let moved = ctx.game.key_text_styled("Move", keys).unwrap();
    assert_eq!(moved.spans()[0].role, Some(TextRole::Arrows));
    let confirm = ctx.game.key_text_styled("Confirm", keys).unwrap();
    assert_eq!(confirm.spans()[0].role, Some(TextRole::Key));
    assert_eq!(ctx.game.key_text_styled("Nope", keys), None);
}

#[test]
fn a_help_line_with_the_arrows_picture_is_as_its_snapshot_says() {
    let game = game_on(Some(arrows_style()), false, Box::new(CursorHelp));
    assert_eq!(game.buffer().sprites().len(), 1);
    insta::assert_snapshot!(game.buffer().to_snapshot(&game.ctx().palette));
}

#[test]
fn a_help_line_with_caps_off_is_as_its_snapshot_says() {
    let game = game_on(None, false, Box::new(CursorHelp));
    assert!(game.buffer().items().is_empty());
    insta::assert_snapshot!(game.buffer().to_snapshot(&game.ctx().palette));
}
