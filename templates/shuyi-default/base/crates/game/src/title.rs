//! The title, the game's first screen: the game's name in a box, a line
//! under it, and the help. Confirm starts the first stage, if the game
//! has one ([`crate::stages`]); Cancel leaves the game, where the game can
//! (`ctx.platform.can_quit()`: not on the web, where the player closes the
//! tab, nor on a console, where the home button leaves).
//!
//! It builds a [`TitleView`] (plain data) and paints it with glyphs:
//! tests of what it shows read the view.

use shuyi::prelude::*;

use crate::{Data, stages};

/// What the title shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TitleView {
    /// The game's name, as its title.
    pub title: String,
    /// The line under it.
    pub subtitle: String,
    /// How to start the first stage, if the game has one.
    pub play: Option<String>,
    /// The help line: empty where the game can't quit, since Cancel is the
    /// only thing it would name.
    pub help: String,
}

/// The title screen.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Title;

impl Title {
    /// What it shows.
    pub fn view(ctx: &Ctx<Data>) -> TitleView {
        TitleView {
            title: ctx.text("title.name").to_owned(),
            subtitle: ctx.text("title.subtitle").to_owned(),
            play: (!stages::names().is_empty()).then(|| ctx.text_with("title.play", &[])),
            help: if ctx.platform.can_quit() {
                ctx.text_with("title.help", &[])
            } else {
                String::new()
            },
        }
    }
}

impl Screen<Data> for Title {
    // check-text: not player text, the screen's id
    fn name(&self) -> &'static str {
        "title"
    }

    /// Confirm, if there is a stage to start, and Cancel to leave. Cancel is
    /// declared on every target, and does nothing where the game can't quit.
    fn actions(&self) -> Option<Vec<UiAction>> {
        let mut actions = vec![UiAction::Cancel];
        if !stages::names().is_empty() {
            actions.insert(0, UiAction::Confirm);
        }
        Some(actions)
    }

    fn update(&mut self, ctx: &mut Ctx<Data>, input: &FrameInput<UiAction>) -> Transition<Data> {
        if ctx.platform.can_quit() && input.actions.contains(&UiAction::Cancel) {
            return Transition::Quit;
        }
        if input.actions.contains(&UiAction::Confirm)
            && let Some(stage) = stages::first(ctx)
        {
            return Transition::Push(stage);
        }
        Transition::None
    }

    fn draw(&self, ctx: &Ctx<Data>, buf: &mut GlyphBuffer) {
        let c = |e| ctx.palette.engine(e);
        let bg = c(EngineColor::Background);
        buf.fill_rect(buf.bounds(), Cell::new(' ', bg, bg));
        let view = Self::view(ctx);
        let width = i32::try_from(view.title.chars().count()).unwrap_or(0) + 6;
        let x = (i32::from(buf.width()) - width).max(0) / 2;
        buf.draw_box(
            Rect::new(x, 8, width, 3),
            BoxStyle::Double,
            c(EngineColor::Border),
            bg,
        );
        print_centred(buf, 9, &view.title, c(EngineColor::Highlight), bg);
        print_centred(buf, 12, &view.subtitle, c(EngineColor::TextDim), bg);
        if let Some(play) = &view.play {
            print_centred(buf, 15, play, c(EngineColor::Text), bg);
        }
        let last = i32::from(buf.height()) - 1;
        print_centred(buf, last, &view.help, c(EngineColor::TextDim), bg);
    }
}
