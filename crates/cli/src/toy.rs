//! The toy game's tools ([`shuyi_ui::toy`]): `frame-png` and
//! `lang-status` on the toy, for this crate's tests and its `toy` example
//! (the toy's tool program, which `toy/shuyi.ron` names).
//!
//! The toy has no font, so its pictures draw every letter as a box, and
//! its images (sizes only) as the missing colour. Its session (`run`)
//! measures every frame with a fake clock ([`PerfClock::fake`]) and reads a
//! fake heap ([`HeadlessPlatform::with_fake_memory`]), so its perf records
//! are the same on every run.
//!
//! [`link`] runs the toy behind the live link's server, a test host for
//! what attaches to a running build.

pub mod link;

use std::collections::BTreeMap;

use shuyi_platform::harness::Harness;
use shuyi_platform::headless::HeadlessPlatform;
use shuyi_ui::GameData;
use shuyi_ui::perf::PerfClock;
use shuyi_ui::toy::{self, Toy};

use crate::codec::encode_png;
use crate::contract::session::{Headless, Session};
use crate::font_atlas::{BitmapFont, build_atlas};
use crate::frame_png::{Painter, Script, Shot};
use crate::game::{GameArg, GameLang, GameOption, GameTools};

/// The toy's own `frame-png` option.
const OPTIONS: [GameOption; 1] = [GameOption {
    flag: "--lobby",
    value: None,
    help: "start in the lobby of the engine's screens, not on the home screen",
}];

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
        let lobby = options.iter().any(|option| option.flag == "--lobby");
        let mut h = Harness::new(if lobby { toy::lobby() } else { toy::game() });
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

    /// The toy on a headless platform with a fake clock and a fake heap.
    fn session(&self, options: &[GameArg]) -> Result<Box<dyn Session>, String> {
        let lobby = options.iter().any(|option| option.flag == "--lobby");
        let game = if lobby { toy::lobby() } else { toy::game() };
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

/// A font atlas of 8 × 16 cells in which every printable ASCII letter but
/// the space, and every box-drawing, block and shape character, is a box;
/// and its picture.
fn box_atlas() -> Result<(shuyi_assets::font::FontAtlasDef, Vec<u8>), String> {
    let (cell_w, cell_h) = (8, 16);
    let edge = |x: u32, y: u32| {
        let inside = (1..6).contains(&x) && (3..13).contains(&y);
        inside && (x == 1 || x == 5 || y == 3 || y == 12)
    };
    let boxed: Vec<bool> = (0..cell_h)
        .flat_map(|y| (0..cell_w).map(move |x| edge(x, y)))
        .collect();
    let blank = vec![false; boxed.len()];
    let pixels = |c: char| if c == ' ' { &blank } else { &boxed };
    let glyphs: BTreeMap<char, Vec<bool>> = (' '..='~')
        .chain('\u{2500}'..='\u{25ff}')
        .map(|c| (c, pixels(c).clone()))
        .collect();
    let atlas = build_atlas(&BitmapFont {
        cell_w,
        cell_h,
        glyphs,
    });
    let png = encode_png(atlas.width, atlas.height, &atlas.rgba)?;
    Ok((atlas.def, png))
}
