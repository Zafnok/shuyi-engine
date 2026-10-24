//! The intro: the cinematic of `files/intro.ron` played over the title,
//! each frame drawn from the time alone. It keeps its own time, or the
//! music's when its cinematic follows a cue that is sounding. Confirm or
//! Cancel skips it, and it closes at its end.
//!
//! Its part of the game's data is [`Own`]: the cinematic, read and checked
//! with the rest of the files ([`own`]). Its shots are the game's own kind,
//! [`Shot`]: what a frame of one shows is plain data ([`ShotPainter::frame`],
//! the text on screen) that its painter draws.

use std::any::Any;

use serde::Deserialize;
use shuyi::assets::Formats;
use shuyi::assets::audio::AudioManifest;
use shuyi::assets::cinematic::{self, CinematicDef, CinematicFile, CinematicRon, ShotCheck};
use shuyi::assets::lang::English;
use shuyi::prelude::*;
use shuyi::ui::cinematic::{CinematicPlayer, Moment, ShotPainter, millis};

use crate::Data;

/// The cinematic, in the game's files.
// check-text: not player text, a file name
pub const INTRO: &str = "intro.ron";

/// The cinematic's id: its file's name.
// check-text: not player text, an id
pub const ID: &str = "intro";

/// The game's English text, which the shots name by key.
// check-text: not player text, a file name
const ENGLISH: &str = "en.ron";

/// A shot of the game's own: a line of its text, by key.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub enum Shot {
    /// The line, whole, in the middle of the screen.
    Line(String),
    /// The line typed out over the shot, a letter at a time.
    Typed(String),
}

impl Shot {
    /// The text key it shows.
    pub fn key(&self) -> &str {
        match self {
            Shot::Line(key) | Shot::Typed(key) => key,
        }
    }
}

/// The game's check of its shots: each names a text key its English has.
struct Keys<'a>(&'a English);

impl ShotCheck<Shot> for Keys<'_> {
    type Shot = Shot;

    fn check(&mut self, n: usize, shot: &Shot, problems: &mut Vec<String>) -> Shot {
        let key = shot.key();
        if !self.0.0.contains_key(key) {
            // check-text: not player text, a content error `cargo shuyi check` shows
            problems.push(format!("shot {n}: no text \"{key}\" in {ENGLISH}"));
        }
        shot.clone()
    }
}

impl ShotPainter for Shot {
    type Game = Data;
    type Shared = ();
    /// The text on screen.
    type Frame = String;

    fn frame(&self, ctx: &Ctx<Data>, (): &(), moment: Moment, _: Rect) -> String {
        let text = ctx.text(self.key());
        match self {
            Shot::Line(_) => text.to_owned(),
            Shot::Typed(_) => {
                // As many letters as the shot is through, in whole
                // milliseconds: the same moment always shows the same.
                let letters = u64::try_from(text.chars().count()).unwrap_or(0);
                let shown = letters * u64::from(moment.into_ms) / u64::from(moment.shot_ms.max(1));
                text.chars()
                    .take(usize::try_from(shown).unwrap_or(0))
                    .collect()
            }
        }
    }

    fn paint(&self, ctx: &Ctx<Data>, (): &(), _: Moment, text: String, buf: &mut GlyphBuffer) {
        let c = |e| ctx.palette.engine(e);
        let row = i32::from(buf.height()) / 2;
        print_centred(
            buf,
            row,
            &text,
            c(EngineColor::Highlight),
            c(EngineColor::Background),
        );
    }
}

/// The stage's part of the game's data: the cinematic, checked.
#[derive(Debug)]
pub struct Own {
    /// The intro.
    pub intro: CinematicDef<Shot>,
}

/// Reads the cinematic from the game's files through the engine's
/// cinematic format, then checks its times and its shots: every problem,
/// with its file.
pub fn own(bundle: &Bundle) -> Result<Own, Vec<ContentError>> {
    let english = BasicData::<UiAction>::formats().load::<English>(bundle, ENGLISH)?;
    let mut formats = Formats::new();
    // The registry is empty, so the format is never refused.
    let _ = formats.register(CinematicRon::<Shot>::new());
    let raw = formats.load::<CinematicFile<Shot>>(bundle, INTRO)?;
    let file = bundle.display_path(INTRO);
    let music = AudioManifest::default();
    let intro = cinematic::check(&file, ID, &raw, &music, Keys(&english))?;
    Ok(Own { intro })
}

/// The intro, as the title starts it.
pub fn first(ctx: &mut Ctx<Data>) -> Box<dyn Screen<Data>> {
    Box::new(Intro::new(ctx))
}

/// Every screen the stage can show.
pub fn screens(ctx: &mut Ctx<Data>) -> Vec<Box<dyn Screen<Data>>> {
    vec![first(ctx)]
}

/// The intro screen.
#[derive(Debug)]
pub struct Intro {
    /// The cinematic, ready to draw at any time.
    pub player: CinematicPlayer<Shot>,
    /// The music cue it follows, if it follows one.
    pub music: Option<String>,
    /// Seconds since it opened.
    pub t: f32,
}

impl Intro {
    /// The game's intro, at its start.
    pub fn new(ctx: &Ctx<Data>) -> Self {
        let def = &ctx.game.own.intro;
        Self {
            player: CinematicPlayer::of(def, Shot::clone),
            music: def.music.clone(),
            t: 0.0,
        }
    }

    /// The time it is drawn at: the music's, while the cue it follows
    /// sounds, else its own.
    pub fn time(&self, ctx: &Ctx<Data>) -> f32 {
        match &ctx.music_clock {
            Some(clock) if self.music.as_deref() == Some(clock.cue.as_str()) => clock.position,
            _ => self.t,
        }
    }

    /// What it shows now on a screen of `screen` cells: the text, as
    /// plain data, for a test of what plays.
    pub fn shown(&self, ctx: &Ctx<Data>, screen: Rect) -> Option<String> {
        self.player.frame(ctx, &(), self.time(ctx), screen)
    }
}

impl Screen<Data> for Intro {
    // check-text: not player text, the screen's id
    fn name(&self) -> &'static str {
        "intro"
    }

    /// Confirm or Cancel skips it.
    fn actions(&self) -> Option<Vec<UiAction>> {
        Some(vec![UiAction::Confirm, UiAction::Cancel])
    }

    fn update(&mut self, ctx: &mut Ctx<Data>, input: &FrameInput<UiAction>) -> Transition<Data> {
        if input
            .actions
            .iter()
            .any(|a| matches!(a, UiAction::Confirm | UiAction::Cancel))
        {
            return Transition::Pop;
        }
        self.t += input.dt();
        let length = u64::from(self.player.timeline().length_ms());
        if millis(self.time(ctx)) >= length {
            return Transition::Pop;
        }
        Transition::None
    }

    fn draw(&self, ctx: &Ctx<Data>, buf: &mut GlyphBuffer) {
        self.player.draw(ctx, &(), self.time(ctx), 1.0, buf);
    }

    fn as_any(&self) -> Option<&dyn Any> {
        Some(self)
    }
}
