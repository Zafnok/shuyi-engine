//! The toy game's tools ([`shuyi_ui::toy`]): `frame-png` and
//! `lang-status` on the toy, for this crate's tests and its `toy` example
//! (the toy's tool program, which `toy/shuyi.ron` names).
//!
//! The toy has no font, so its pictures draw every letter as a box, and
//! its images (sizes only) as the missing colour. `--plasma` starts it on
//! its plasma stage, whose skin paints pictures at run time (ADR-0078);
//! `--field` on its grid field, whose layout the debug view's `grid`
//! channel shows (`--debug grid`, ADR-0085);
//! `--target playdate` draws the frame on the Playdate's console, black
//! and white. Its session (`run`)
//! measures every frame with a fake clock ([`PerfClock::fake`]) and reads a
//! fake heap ([`HeadlessPlatform::with_fake_memory`]), so its perf records
//! are the same on every run.
//!
//! `bench`'s fixtures (`toy/*.bench.ron`) start it with `--burn-frame N`,
//! which spends some millions of instructions in frame N, or `--hold-mb N`,
//! which holds N MB from the first frame on. Either gives it the real clock
//! and heap a measured session reads; its session on the Playdate
//! converts each frame as the device does ([`GameTools::session_on`]).
//!
//! `bench --long`'s fixtures (`toy/long/*.long.ron`) start it with
//! `--leak-bytes N`, which keeps N bytes more each frame under the memory
//! tag `toy-leak` (and gives it the real heap, as the bench fixtures do,
//! even with N 0); `--hum-every N`, which starts one more loop of `hum`
//! every N frames and never stops one; or `--pid-screen`, which opens a
//! screen named after the process's id, so two runs hash differently.
//!
//! [`link`] runs the toy behind the live link's server, a test host for
//! what attaches to a running build.

pub mod link;

use std::collections::BTreeMap;

use shuyi_assets::font::FontAtlasDef;
use shuyi_platform::harness::Harness;
use shuyi_platform::headless::HeadlessPlatform;
use shuyi_platform::script::FRAME_DT;
use shuyi_platform::{Frame, Show};
use shuyi_ui::frame::ConsoleSpec;
use shuyi_ui::frame::glyph_buffer::GlyphBuffer;
use shuyi_ui::frame::onebit::{self, FRAME_BYTES, Font, PatternMap};
use shuyi_ui::perf::{PerfClock, TagClass};
use shuyi_ui::screen::{FrameInput, Screen, ScreenStack, Transition};
use shuyi_ui::toy::{self, Toy, ToyCtx};
use shuyi_ui::{Game, GameData};

use crate::codec::encode_png;
use crate::contract::session::{Headless, Session};
use crate::font_atlas::{AtlasChars, BitmapFont, build_atlas};
use crate::frame_png::{Capture, Painter, Script, Shot, one_bit_picture};
use crate::game::{GameArg, GameLang, GameOption, GameTools};

/// The toy's own `frame-png` options.
const OPTIONS: [GameOption; 8] = [
    GameOption {
        flag: "--lobby",
        value: None,
        help: "start in the lobby of the engine's screens, not on the home screen",
    },
    GameOption {
        flag: "--field",
        value: None,
        help: "start on the grid field, under the glyph skin",
    },
    GameOption {
        flag: "--plasma",
        value: None,
        help: "start on the plasma stage, whose skin paints pictures at run time",
    },
    GameOption {
        flag: "--burn-frame",
        value: Some("N"),
        help: "spend some millions of instructions in frame N (a bench fixture)",
    },
    GameOption {
        flag: "--hold-mb",
        value: Some("N"),
        help: "hold N MB of heap from the first frame on (a bench fixture)",
    },
    GameOption {
        flag: "--leak-bytes",
        value: Some("N"),
        help: "keep N bytes more every frame, tagged toy-leak (a long-run fixture)",
    },
    GameOption {
        flag: "--hum-every",
        value: Some("N"),
        help: "start one more loop every N frames, never stopped (a long-run fixture)",
    },
    GameOption {
        flag: "--pid-screen",
        value: None,
        help: "open a screen named after the process's id (a long-run fixture)",
    },
];

/// The memory tag `--leak-bytes` keeps its bytes under.
const LEAK_TAG: &str = "toy-leak";

/// The loops `--burn-frame` runs: at least two instructions each, more
/// than the Playdate's budget of a frame and less than a desktop's.
const BURN: u64 = 1_500_000;

/// The value of start option `flag` among `options`, as a number.
fn number(options: &[GameArg], flag: &str) -> Result<Option<u64>, String> {
    let Some(arg) = options.iter().find(|option| option.flag == flag) else {
        return Ok(None);
    };
    let value = arg.value.as_deref().unwrap_or_default();
    value
        .parse()
        .map(Some)
        .map_err(|_| format!("{flag}: not a whole number: {value}"))
}

/// What `--burn-frame`, `--hold-mb` and `--leak-bytes` ask of each frame
/// presented, if one is given: a burn in that frame, that heap held from
/// the first, and those bytes kept more each frame.
fn fixture(options: &[GameArg]) -> Result<Option<impl FnMut() + 'static>, String> {
    let (burn, hold_mb, leak) = (
        number(options, "--burn-frame")?,
        number(options, "--hold-mb")?,
        number(options, "--leak-bytes")?,
    );
    if burn.is_none() && hold_mb.is_none() && leak.is_none() {
        return Ok(None);
    }
    let (mut frame, mut kept) = (0_u64, None::<Vec<u8>>);
    let mut leaked: Vec<Box<[u8]>> = Vec::new();
    let leak = leak.filter(|&bytes| bytes > 0).map(|bytes| {
        let tag = shuyi_alloc_count::tags::tag(TagClass::Span as u8, LEAK_TAG);
        (usize::try_from(bytes).unwrap_or(usize::MAX), tag)
    });
    Ok(Some(move || {
        frame += 1;
        if let Some((bytes, tag)) = leak {
            let before = shuyi_alloc_count::tags::enter(tag);
            leaked.push(std::hint::black_box(vec![1; bytes].into_boxed_slice()));
            shuyi_alloc_count::tags::leave(before);
        }
        if let Some(mb) = hold_mb
            && kept.is_none()
        {
            let bytes = usize::try_from(mb << 20).unwrap_or(usize::MAX);
            kept = Some(std::hint::black_box(vec![0; bytes]));
        }
        if burn == Some(frame) {
            let mut sum = 0_u64;
            for i in 0..BURN {
                sum = std::hint::black_box(sum.wrapping_add(i));
            }
            std::hint::black_box(sum);
        }
    }))
}

/// A long run's fixture screen over the toy's home: it starts a loop of
/// `hum` every `every` frames, if given, and never stops one; its name is
/// `name`.
struct Humming {
    name: &'static str,
    every: Option<u64>,
    frames: u64,
}

impl Screen<Toy> for Humming {
    fn name(&self) -> &'static str {
        self.name
    }

    fn actions(&self) -> Option<Vec<toy::Action>> {
        Some(Vec::new())
    }

    fn update(&mut self, ctx: &mut ToyCtx, _: &FrameInput<toy::Action>) -> Transition<Toy> {
        self.frames += 1;
        if self
            .every
            .is_some_and(|every| self.frames.is_multiple_of(every))
        {
            ctx.audio.start_sound("hum");
        }
        Transition::None
    }

    fn draw(&self, _: &ToyCtx, _: &mut GlyphBuffer) {}
}

/// The long run's fixture screen `options` ask for, if any.
fn humming(options: &[GameArg]) -> Option<Humming> {
    let every = number(options, "--hum-every")
        .ok()
        .flatten()
        .filter(|&n| n > 0);
    let pid = options.iter().any(|option| option.flag == "--pid-screen");
    let name: &'static str = if pid {
        // One name for the program's life: a long run opens it once.
        Box::leak(format!("pid-{}", std::process::id()).into_boxed_str())
    } else {
        "humming"
    };
    (every.is_some() || pid).then_some(Humming {
        name,
        every,
        frames: 0,
    })
}

/// The toy on `ctx`, started where `options` say.
fn game_on(ctx: ToyCtx, options: &[GameArg]) -> Game<Toy> {
    let has = |flag: &str| options.iter().any(|option| option.flag == flag);
    if let Some(humming) = humming(options) {
        let mut stack = ScreenStack::new(Box::new(toy::Home::default()));
        stack.push(Box::new(humming));
        return Game::with_stack(ctx, stack, toy::hooks());
    }
    if has("--plasma") {
        toy::plasma::game_on(ctx)
    } else if has("--field") {
        Game::new(ctx, Box::new(toy::grid::Field::glyphs()), toy::hooks())
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
        let painter = painter(&h)?;
        Ok(Shot::new(
            painter.render(h.game().buffer(), scale.unwrap_or(2)),
            h.top_screen(),
        ))
    }

    /// Plays the toy once: the painter built once, a frame painted after
    /// the script, then after each `every` frames.
    fn frames(
        &self,
        options: &[GameArg],
        script: &Script,
        scale: Option<u32>,
        capture: Capture,
    ) -> Result<Vec<Shot>, String> {
        let mut h = Harness::new(game_on(toy::ctx(), options));
        script.play(&mut h);
        let painter = painter(&h)?;
        let mut shots = Vec::new();
        for i in 0..capture.count {
            if i > 0 {
                for _ in 0..capture.every {
                    h.frame(&[], FRAME_DT);
                }
            }
            shots.push(Shot::new(
                painter.render(h.game().buffer(), scale.unwrap_or(2)),
                h.top_screen(),
            ));
        }
        Ok(shots)
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
        // The toy has no pattern map: every colour is of no role, dithered
        // by its luminance.
        onebit::render(
            h.game().buffer(),
            &font,
            &(),
            &PatternMap::default(),
            &mut frame,
        );
        Ok(Shot {
            image: one_bit_picture(&frame, scale.unwrap_or(1)),
            shows: format!("{} on {target}", h.top_screen()),
            warning: None,
        })
    }

    /// The toy on a headless platform with a fake clock and a fake heap;
    /// with a bench fixture's option, with neither, and the fixture run on
    /// each frame presented.
    fn session(&self, options: &[GameArg]) -> Result<Box<dyn Session>, String> {
        let game = game_on(toy::ctx(), options);
        let Some(mut fixture) = fixture(options)? else {
            let platform = HeadlessPlatform::new()
                .with_clock(PerfClock::fake())
                .with_fake_memory();
            return Ok(painted(Harness::on(game, platform)));
        };
        let mut h = Harness::on(game, HeadlessPlatform::new());
        h.set_present_hook(Some(Box::new(move |_: &Frame<'_>| fixture())));
        Ok(painted(h))
    }

    /// The toy on the Playdate's console, as [`frame_one_bit`] draws it,
    /// each frame presented converted to one bit as the device converts it
    /// (after a bench fixture's work, if one is asked for).
    ///
    /// [`frame_one_bit`]: GameTools::frame_one_bit
    fn session_on(&self, _target: &str, options: &[GameArg]) -> Result<Box<dyn Session>, String> {
        let mut ctx = toy::ctx();
        ctx.console = ConsoleSpec::text(50, 15);
        let mut fixture = fixture(options)?;
        let (atlas, _) = box_atlas()?;
        let bits = one_bit_boxes(&atlas);
        // Built once for the session, as the device's look is: a session
        // runs once in a tool program.
        let atlas: &'static FontAtlasDef = Box::leak(Box::new(atlas));
        let bits: &'static [u8] = Box::leak(bits.into_boxed_slice());
        let font = Font::new(atlas, bits)?;
        let mut frame = Box::new([0; FRAME_BYTES]);
        let map = PatternMap::default();
        let mut h = Harness::on(game_on(ctx, options), HeadlessPlatform::new());
        h.set_present_hook(Some(Box::new(move |presented: &Frame<'_>| {
            if let Some(fixture) = &mut fixture {
                fixture();
            }
            if let Show::Game { buffer, .. } = presented.show {
                onebit::render(buffer, &font, &(), &map, &mut frame);
            }
        })));
        Ok(Box::new(Headless::new(h)))
    }

    fn lang(&self) -> Result<GameLang, String> {
        Ok(GameLang {
            lang: Toy::new().lang().clone(),
            variants: Vec::new(),
        })
    }
}

/// The painter of the toy in `h`: its box font, its images as the
/// missing colour, its blank cell's background as the clear colour.
/// A session of `h`, its frame painted as `frame-png` paints it.
fn painted(h: Harness<Toy>) -> Box<dyn Session> {
    let picture = |h: &Harness<Toy>, scale: Option<u32>| {
        Ok(painter(h)?.render(h.game().buffer(), scale.unwrap_or(2)))
    };
    Box::new(Headless::new(h).with_picture(picture))
}

fn painter(h: &Harness<Toy>) -> Result<Painter, String> {
    let ctx = h.ctx();
    let (atlas, png) = box_atlas()?;
    let clear = ctx.game.blank_cell(&ctx.palette).bg;
    Painter::new(atlas, &png, &ctx.game.images, |_| None::<&[u8]>, clear)
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
    let png = encode_png(atlas.image.width, atlas.image.height, &atlas.image.rgba)?;
    Ok((atlas.def, png))
}
