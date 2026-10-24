//! The title, the game's one screen: the game's name in a box, a line
//! under it, and the help. Cancel leaves the game.
//!
//! It builds a [`TitleView`] (plain data) and paints it with glyphs:
//! tests of what it shows read the view.

use shuyi::prelude::*;

use crate::Data;

/// What the title shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TitleView {
    /// The game's name.
    pub name: String,
    /// The line under it.
    pub subtitle: String,
    /// The help line.
    pub help: String,
}

/// The title screen.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Title;

impl Title {
    /// What it shows.
    pub fn view(ctx: &Ctx<Data>) -> TitleView {
        TitleView {
            name: ctx.text("title.name").to_owned(),
            subtitle: ctx.text("title.subtitle").to_owned(),
            help: ctx.text_with("title.help", &[]),
        }
    }
}

impl Screen<Data> for Title {
    fn name(&self) -> &'static str {
        "title"
    }

    /// Cancel leaves.
    fn actions(&self) -> Option<Vec<UiAction>> {
        Some(vec![UiAction::Cancel])
    }

    fn update(&mut self, _: &mut Ctx<Data>, input: &FrameInput<UiAction>) -> Transition<Data> {
        if input.actions.contains(&UiAction::Cancel) {
            return Transition::Quit;
        }
        Transition::None
    }

    fn draw(&self, ctx: &Ctx<Data>, buf: &mut GlyphBuffer) {
        let c = |e| ctx.palette.engine(e);
        let bg = c(EngineColor::Background);
        buf.fill_rect(buf.bounds(), Cell::new(' ', bg, bg));
        let view = Self::view(ctx);
        let width = i32::try_from(view.name.chars().count()).unwrap_or(0) + 6;
        let x = (i32::from(buf.width()) - width).max(0) / 2;
        buf.draw_box(
            Rect::new(x, 8, width, 3),
            BoxStyle::Double,
            c(EngineColor::Border),
            bg,
        );
        print_centred(buf, 9, &view.name, c(EngineColor::Highlight), bg);
        print_centred(buf, 12, &view.subtitle, c(EngineColor::TextDim), bg);
        let last = i32::from(buf.height()) - 1;
        print_centred(buf, last, &view.help, c(EngineColor::TextDim), bg);
    }
}
