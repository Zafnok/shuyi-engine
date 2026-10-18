//! A toy cinematic of two toy shot kinds, read from its file and played
//! (ticket 1113): a big letter in the middle of the screen, and a bar
//! sweeping across it. What each frame shows is read from the frame's
//! plain data, and the drawing from the cells.

use serde::Deserialize;
use shuyi_assets::cinematic::{self, CinematicDef, ShotCheck};
use shuyi_ui::cinematic::{CinematicPlayer, Moment, ShotPainter, ease_in_out};
use shuyi_ui::frame::{Cell, EngineColor, GlyphBuffer, Rect, Rgb};
use shuyi_ui::toy::{self, Toy, ToyCtx};

/// A shot as the toy's file writes it.
#[derive(Debug, Deserialize)]
enum Written {
    /// A capital letter in the middle of the screen.
    Letter(char),
    /// A bar sweeping from the left edge to the right.
    Sweep,
}

/// The toy's checks: a letter is a capital.
struct Capitals;

impl ShotCheck<Written> for Capitals {
    type Shot = Shot;

    fn check(&mut self, n: usize, shot: &Written, problems: &mut Vec<String>) -> Shot {
        match shot {
            Written::Letter(c) => {
                if !c.is_ascii_uppercase() {
                    problems.push(format!("shot {n}: '{c}' is not a capital"));
                }
                Shot::Letter(*c)
            }
            Written::Sweep => Shot::Sweep,
        }
    }
}

/// A shot as the toy keeps and plays it.
#[derive(Debug, Clone, PartialEq)]
enum Shot {
    Letter(char),
    Sweep,
}

/// What a frame of a toy shot shows.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Shown {
    /// The letter.
    Letter(char),
    /// The bar, at this column.
    Sweep { x: i32 },
}

impl ShotPainter for Shot {
    type Game = Toy;
    /// The bar's glyph: the same for every shot.
    type Shared = char;
    type Frame = Shown;

    fn frame(&self, _: &ToyCtx, _: &char, moment: Moment, screen: Rect) -> Shown {
        match self {
            Shot::Letter(c) => Shown::Letter(*c),
            Shot::Sweep => {
                #[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
                let x = (ease_in_out(moment.progress) * (screen.w - 1) as f32).round() as i32;
                Shown::Sweep { x }
            }
        }
    }

    fn paint(&self, ctx: &ToyCtx, bar: &char, _: Moment, frame: Shown, buf: &mut GlyphBuffer) {
        let fg = ctx.palette.engine(EngineColor::Highlight);
        let bg = ctx.palette.engine(EngineColor::Background);
        let b = buf.bounds();
        match frame {
            Shown::Letter(c) => buf.set(b.w / 2, b.h / 2, Cell::new(c, fg, bg)),
            Shown::Sweep { x } => {
                for y in 0..b.h {
                    buf.set(x, y, Cell::new(*bar, fg, bg));
                }
            }
        }
    }
}

/// On the toy's 10-second track `theme`: an A for 4 s, a sweep for 3 s,
/// then a Z.
const FILE: &str = "(
    id: \"toy\",
    length: Music(\"theme\"),
    shots: [
        (at: 0.0, shot: Letter('A')),
        (at: 4.0, shot: Sweep),
        (at: 7.0, shot: Letter('Z')),
    ],
)";

fn load(ctx: &ToyCtx, source: &str) -> Result<CinematicDef<Shot>, Vec<String>> {
    let audio = &ctx.game.audio;
    let loaded = cinematic::from_source::<Written, _>("toy.ron", "toy", source, audio, Capitals);
    loaded.map_err(|errors| errors.into_iter().map(|e| e.message).collect())
}

fn player(ctx: &ToyCtx) -> CinematicPlayer<Shot> {
    let def = load(ctx, FILE).unwrap_or_else(|e| panic!("{e:?}"));
    CinematicPlayer::of(&def, Shot::clone)
}

/// The frame at `t` and `brightness`, drawn over a stale buffer.
fn draw(ctx: &ToyCtx, t: f32, brightness: f32) -> GlyphBuffer {
    let stale = Cell::new('x', Rgb::new(1, 2, 3), Rgb::new(4, 5, 6));
    let mut buf = GlyphBuffer::with_spec(ctx.console, stale);
    player(ctx).draw(ctx, &'|', t, brightness, &mut buf);
    buf
}

/// The screen's glyphs, a line per row.
fn glyphs(buf: &GlyphBuffer) -> Vec<String> {
    let b = buf.bounds();
    let row = |y| {
        (0..b.w)
            .filter_map(|x| buf.get(x, y))
            .map(|c| c.glyph)
            .collect()
    };
    (0..b.h).map(row).collect()
}

#[test]
fn a_toy_cinematic_is_read_with_the_games_own_shots() {
    let ctx = toy::ctx();
    let def = load(&ctx, FILE).unwrap();
    assert_eq!(
        (def.music.as_deref(), def.length_ms),
        (Some("theme"), 10_000)
    );
    let shots: Vec<(u32, Shot)> = def.shots.into_iter().map(|s| (s.at_ms, s.shot)).collect();
    assert_eq!(
        shots,
        [
            (0, Shot::Letter('A')),
            (4000, Shot::Sweep),
            (7000, Shot::Letter('Z'))
        ]
    );
    // The game's problems come after the frame's, shot by shot.
    let wrong = FILE.replace("'Z'", "'z'").replace("7.0", "3.0");
    assert_eq!(
        load(&ctx, &wrong).unwrap_err(),
        [
            "shot 3: at 3 must be after the shot before it; shots are in order of time",
            "shot 3: 'z' is not a capital",
        ]
    );
}

#[test]
fn each_moment_shows_the_shot_it_falls_in() {
    let ctx = toy::ctx();
    let player = player(&ctx);
    // Each shot is built once, in the file's order.
    assert_eq!(player.shot(0), Some(&Shot::Letter('A')));
    assert_eq!(player.shot(1), Some(&Shot::Sweep));
    assert_eq!(player.shot(2), Some(&Shot::Letter('Z')));
    assert_eq!(player.shot(3), None);
    let screen = Rect::new(0, 0, 40, 20);
    let frame = |t: f32| player.frame(&ctx, &'|', t, screen).unwrap();
    assert_eq!(frame(0.0), Shown::Letter('A'));
    assert_eq!(frame(3.999), Shown::Letter('A'));
    // The sweep crosses the screen in its 3 s, slow at both ends.
    assert_eq!(frame(4.0), Shown::Sweep { x: 0 });
    assert_eq!(frame(4.75), Shown::Sweep { x: 6 });
    assert_eq!(frame(5.5), Shown::Sweep { x: 20 });
    assert_eq!(frame(6.999), Shown::Sweep { x: 39 });
    assert_eq!(frame(7.0), Shown::Letter('Z'));
    // It loops with its track, and a time is always the same frame.
    assert_eq!(frame(10.0), Shown::Letter('A'));
    assert_eq!(frame(25.5), frame(5.5));
    assert_eq!(frame(-1.0), Shown::Letter('A'));
    // A player of no shots shows nothing.
    let none: CinematicPlayer<Shot> = CinematicPlayer::new(player.timeline().clone(), vec![]);
    assert_eq!(none.frame(&ctx, &'|', 5.5, screen), None);
}

#[test]
fn a_frame_is_drawn_over_a_cleared_screen_and_faded_whole() {
    let ctx = toy::ctx();
    let a = draw(&ctx, 1.0, 1.0);
    let rows = glyphs(&a);
    assert_eq!(rows[10], format!("{}A{}", " ".repeat(20), " ".repeat(19)));
    assert!(
        rows.iter()
            .enumerate()
            .all(|(y, r)| y == 10 || r.trim().is_empty())
    );
    let background = ctx.palette.engine(EngineColor::Background);
    assert_eq!(a.get(0, 0).unwrap().bg, background);
    assert_eq!(
        a.get(0, 0).unwrap().fg,
        ctx.palette.engine(EngineColor::Text)
    );
    // The sweep's bar, in the glyph the player was given.
    let sweep = glyphs(&draw(&ctx, 5.5, 1.0));
    assert!(
        sweep
            .iter()
            .all(|r| r.trim() == "|" && r.find('|') == Some(20))
    );
    // The same time draws the same frame; brightness fades all of it.
    assert_eq!(draw(&ctx, 1.0, 1.0), a);
    let black = draw(&ctx, 1.0, 0.0);
    let cell = black.get(20, 10).unwrap();
    assert_eq!(
        (cell.glyph, cell.fg, cell.bg),
        ('A', Rgb::new(0, 0, 0), Rgb::new(0, 0, 0))
    );
    let half = draw(&ctx, 1.0, 0.5).get(20, 10).unwrap().fg;
    assert_eq!(half, a.get(20, 10).unwrap().fg.scale(0.5));
}
