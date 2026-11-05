//! The toy game's tools ([`shuyi_ui::toy`]): `frame-png` and
//! `lang-status` on the toy, for this crate's tests and its `toy` example
//! (the toy's tool program, which `toy/shuyi.ron` names).
//!
//! The toy has no font, so its pictures draw every letter as a box, and
//! its images (sizes only) as the missing colour. `--plasma` starts it on
//! its plasma stage, whose skin paints pictures at run time (ADR-0078);
//! `--target playdate` draws the frame on the Playdate's console, black
//! and white. Its session (`run`)
//! measures every frame with a fake clock ([`PerfClock::fake`]) and reads a
//! fake heap ([`HeadlessPlatform::with_fake_memory`]), so its perf records
//! are the same on every run.
//!
//! [`link`] runs the toy behind the live link's server, a test host for
//! what attaches to a running build.

pub mod link;

use std::collections::BTreeMap;

use shuyi_assets::font::FontAtlasDef;
use shuyi_platform::harness::Harness;
use shuyi_platform::headless::HeadlessPlatform;
use shuyi_ui::frame::ConsoleSpec;
use shuyi_ui::frame::onebit::{self, FRAME_BYTES, Font};
use shuyi_ui::perf::PerfClock;
use shuyi_ui::toy::{self, Toy, ToyCtx};
use shuyi_ui::{Game, GameData};

use crate::codec::encode_png;
use crate::contract::session::{Headless, Session};
use crate::font_atlas::{AtlasChars, BitmapFont, build_atlas};
use crate::frame_png::{Painter, Script, Shot, one_bit_picture};
use crate::game::{GameArg, GameLang, GameOption, GameTools};

/// The toy's own `frame-png` options.
const OPTIONS: [GameOption; 2] = [
    GameOption {
        flag: "--lobby",
        value: None,
        help: "start in the lobby of the engine's screens, not on the home screen",
    },
    GameOption {
        flag: "--plasma",
        value: None,
        help: "start on the plasma stage, whose skin paints pictures at run time",
    },
];

/// The toy on `ctx`, started where `options` say.
fn game_on(ctx: ToyCtx, options: &[GameArg]) -> Game<Toy> {
    let has = |flag: &str| options.iter().any(|option| option.flag == flag);
    if has("--plasma") {
        toy::plasma::game_on(ctx)
    } else if has("--lobby") {
        Game::new(ctx, Box::new(toy::Lobby::new()), toy::hooks())
    } else {
        toy::game_on(ctx)
    }
}

/// The toy game's tools.
#[derive(Debug, Clone, Copy, Default)]
pub struct ToyTools;

impl GameTools for ToyTools {
    fn frame_options(&self) -> &[GameOption] {
        &OPTIONS
    }

    fn frame_notes(&self) -> &'static str {
        "The toy has no font: its letters are boxes."
    }

    fn frame(
        &self,
        options: &[GameArg],
        script: &Script,
        scale: Option<u32>,
    ) -> Result<Shot, String> {
        let mut h = Harness::new(game_on(toy::ctx(), options));
        script.play(&mut h);
        let ctx = h.ctx();
        let (atlas, png) = box_atlas()?;
        let clear = ctx.game.blank_cell(&ctx.palette).bg;
        let painter = Painter::new(atlas, &png, &ctx.game.images, |_| None, clear)?;
        Ok(Shot {
            image: painter.render(h.game().buffer(), scale.unwrap_or(2)),
            shows: h.top_screen().to_owned(),
            warning: None,
        })
    }

    /// The toy on the Playdate's console (50 × 15 cells of 8 × 16
    /// pixels), drawn by the one-bit renderer in the game's inks with its
    /// box font made one-bit.
    fn frame_one_bit(
        &self,
        target: &str,
        options: &[GameArg],
        script: &Script,
        scale: Option<u32>,
    ) -> Result<Shot, String> {
        let mut ctx = toy::ctx();
        ctx.console = ConsoleSpec::text(50, 15);
        let mut h = Harness::new(game_on(ctx, options));
        script.play(&mut h);
        let (atlas, _) = box_atlas()?;
        let bits = one_bit_boxes(&atlas);
        let font = Font::new(&atlas, &bits)?;
        let mut frame = Box::new([0; FRAME_BYTES]);
        onebit::render(h.game().buffer(), &font, &(), &mut frame);
        Ok(Shot {
            image: one_bit_picture(&frame, scale.unwrap_or(1)),
            shows: format!("{} on {target}", h.top_screen()),
            warning: None,
        })
    }

    /// The toy on a headless platform with a fake clock and a fake heap.
    fn session(&self, options: &[GameArg]) -> Result<Box<dyn Session>, String> {
        let game = game_on(toy::ctx(), options);
        let platform = HeadlessPlatform::new()
            .with_clock(PerfClock::fake())
            .with_fake_memory();
        Ok(Box::new(Headless::new(Harness::on(game, platform))))
    }

    fn lang(&self) -> Result<GameLang, String> {
        Ok(GameLang {
            lang: Toy::new().lang().clone(),
            variants: Vec::new(),
        })
    }
}

/// Whether pixel `(x, y)` of an 8 × 16 cell is on the box every letter of
/// the toy's font is.
fn on_box(x: u32, y: u32) -> bool {
    let inside = (1..6).contains(&x) && (3..13).contains(&y);
    inside && (x == 1 || x == 5 || y == 3 || y == 12)
}

/// The one-bit atlas of `atlas`, the box font: each glyph's 16 rows, a
/// byte each, its highest bit the left pixel; the space blank.
fn one_bit_boxes(atlas: &FontAtlasDef) -> Vec<u8> {
    let rows: Vec<u8> = (0..16)
        .map(|y| {
            (0..8)
                .filter(|&x| on_box(x, y))
                .fold(0, |row, x| row | 0x80 >> x)
        })
        .collect();
    let glyphs = atlas
        .glyphs
        .values()
        .max()
        .map_or(0, |&last| last as usize + 1);
    let mut bits = vec![0; glyphs * rows.len()];
    for (&glyph, &index) in &atlas.glyphs {
        if glyph != ' ' {
            bits[index as usize * rows.len()..][..rows.len()].copy_from_slice(&rows);
        }
    }
    bits
}

/// A font atlas of 8 × 16 cells in which every printable ASCII letter but
/// the space, and every box-drawing, block and shape character, is a box;
/// and its picture.
fn box_atlas() -> Result<(FontAtlasDef, Vec<u8>), String> {
    let (cell_w, cell_h) = (8, 16);
    let edge = on_box;
    let boxed: Vec<bool> = (0..cell_h)
        .flat_map(|y| (0..cell_w).map(move |x| edge(x, y)))
        .collect();
    let blank = vec![false; boxed.len()];
    let pixels = |c: char| if c == ' ' { &blank } else { &boxed };
    let glyphs: BTreeMap<char, Vec<bool>> = (' '..='~')
        .chain('\u{2500}'..='\u{25ff}')
        .map(|c| (c, pixels(c).clone()))
        .collect();
    let atlas = build_atlas(
        &BitmapFont {
            cell_w,
            cell_h,
            glyphs,
        },
        &AtlasChars::default(),
    );
    let png = encode_png(atlas.width, atlas.height, &atlas.rgba)?;
    Ok((atlas.def, png))
}
