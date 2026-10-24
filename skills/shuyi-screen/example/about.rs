//! The about screen: what the game is, over the whole screen. The title's
//! Confirm opens it while the game has no stage; Confirm or Cancel goes
//! back.
//!
//! It builds an [`AboutView`] (plain data) and paints it with glyphs:
//! tests of what it shows read the view.

use std::any::Any;

use shuyi::prelude::*;

use crate::Data;

/// What the about screen shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AboutView {
    /// Its heading.
    pub heading: String,
    /// The line under it.
    pub line: String,
    /// The help line, naming the player's own keys.
    pub help: String,
}

/// The about screen.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct About;

impl About {
    /// What it shows: its text, asked for by key from `files/en.ron`.
    pub fn view(ctx: &Ctx<Data>) -> AboutView {
        AboutView {
            heading: ctx.text("about.heading").to_owned(),
            line: ctx.text("about.line").to_owned(),
            help: ctx.text_with("about.help", &[]),
        }
    }
}

impl Screen<Data> for About {
    // check-text: not player text, the screen's id
    fn name(&self) -> &'static str {
        "about"
    }

    /// Confirm and Cancel, which both go back.
    fn actions(&self) -> Option<Vec<UiAction>> {
        Some(vec![UiAction::Confirm, UiAction::Cancel])
    }

    fn update(&mut self, _: &mut Ctx<Data>, input: &FrameInput<UiAction>) -> Transition<Data> {
        let back = input
            .actions
            .iter()
            .any(|a| matches!(a, UiAction::Confirm | UiAction::Cancel));
        if back {
            Transition::Pop
        } else {
            Transition::None
        }
    }

    fn draw(&self, ctx: &Ctx<Data>, buf: &mut GlyphBuffer) {
        let c = |e| ctx.palette.engine(e);
        let bg = c(EngineColor::Background);
        buf.fill_rect(buf.bounds(), Cell::new(' ', bg, bg));
        let view = Self::view(ctx);
        print_centred(buf, 8, &view.heading, c(EngineColor::Highlight), bg);
        print_centred(buf, 10, &view.line, c(EngineColor::Text), bg);
        let last = i32::from(buf.height()) - 1;
        print_centred(buf, last, &view.help, c(EngineColor::TextDim), bg);
    }

    fn as_any(&self) -> Option<&dyn Any> {
        Some(self)
    }
}
