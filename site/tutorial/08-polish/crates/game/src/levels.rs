//! The level menu: every level of the list, chosen with the engine's
//! [`Menu`]. A board that is won pops with a [`LevelWon`], and the menu
//! moves on to the next level.

use std::any::Any;

use shuyi::prelude::*;
use shuyi::ui::screens::options::OptionsScreen;
use shuyi::ui::widgets::menu::glyph;
use shuyi::ui::widgets::{Menu, MenuEvent, MenuItem};

use crate::Data;
use crate::action::Action;
use crate::board::Board;
use crate::options::option_rows;
use crate::progress;

// ANCHOR: result
/// What a won board hands the menu: the level it was on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LevelWon(pub usize);
// ANCHOR_END: result

/// What an item of the menu stands for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Choice {
    /// Plays this level, counted from 0.
    Level(usize),
    /// Opens the options.
    Options,
}

/// The level menu.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LevelMenu {
    /// Which item has the focus.
    pub focus: usize,
    /// How many levels are open: the one after the last won, and those
    /// before it.
    pub open: usize,
}

impl LevelMenu {
    // ANCHOR: new
    /// The menu, on the furthest level the player has reached.
    pub fn new(ctx: &Ctx<Data>) -> Self {
        let last = ctx.game.own.levels.len().saturating_sub(1);
        let furthest = progress::furthest(&ctx.storage).min(last);
        Self {
            focus: furthest,
            open: furthest + 1,
        }
    }
    // ANCHOR_END: new

    // ANCHOR: menu
    /// The menu's items: a text key and a value each.
    pub fn menu(&self, ctx: &Ctx<Data>) -> Menu<Choice> {
        let levels = ctx.game.own.levels.len();
        let items = (0..levels).map(|n| {
            MenuItem::keyed("levels.level")
                .with_arg("n", n + 1)
                .with_enabled(n < self.open)
                .with_value(Choice::Level(n))
        });
        let options = MenuItem::keyed("levels.options").with_value(Choice::Options);
        Menu::new(items.chain([options]).collect()).focused(self.focus)
    }
    // ANCHOR_END: menu
}

impl Screen<Data> for LevelMenu {
    // check-text: not player text, the screen's id
    fn name(&self) -> &'static str {
        "levels"
    }

    /// Its menu's.
    fn actions(&self) -> Option<Vec<Action>> {
        let actions = Menu::<()>::ACTIONS.iter();
        Some(actions.filter_map(|&ui| Action::for_ui(ui)).collect())
    }

    // ANCHOR: update
    fn update(&mut self, ctx: &mut Ctx<Data>, input: &FrameInput<Action>) -> Transition<Data> {
        let mut menu = self.menu(ctx);
        for &action in &input.actions {
            let event = menu.handle_with_sound(action, &mut ctx.audio);
            self.focus = menu.focus();
            match event {
                Some(MenuEvent::Chosen(Choice::Level(n))) => {
                    return Transition::Push(Box::new(Board::new(ctx, n)));
                }
                // ANCHOR: chosen
                Some(MenuEvent::Chosen(Choice::Options)) => {
                    return Transition::Push(Box::new(OptionsScreen::new(option_rows)));
                }
                // ANCHOR_END: chosen
                Some(MenuEvent::Cancelled) => return Transition::Pop,
                None => {}
            }
        }
        Transition::None
    }
    // ANCHOR_END: update

    // ANCHOR: uncovered
    /// A won board hands back its level: the menu moves on to the next.
    fn uncovered(&mut self, ctx: &mut Ctx<Data>, result: Option<ScreenResult>) -> Transition<Data> {
        if let Some(LevelWon(level)) = result.and_then(ScreenResult::take::<LevelWon>) {
            let last = ctx.game.own.levels.len().saturating_sub(1);
            self.focus = (level + 1).min(last);
            // ANCHOR: keep
            self.open = self.open.max(self.focus + 1);
            // A storage that fails loses only the player's progress.
            let _ = progress::reached(&mut ctx.storage, self.focus);
            // ANCHOR_END: keep
        }
        Transition::None
    }
    // ANCHOR_END: uncovered

    fn draw(&self, ctx: &Ctx<Data>, buf: &mut GlyphBuffer) {
        let c = |e| ctx.palette.engine(e);
        let bg = c(EngineColor::Background);
        buf.fill_rect(buf.bounds(), Cell::new(' ', bg, bg));
        print_centred(
            buf,
            3,
            ctx.text("levels.title"),
            c(EngineColor::Highlight),
            bg,
        );
        let view = self.menu(ctx).view();
        let (width, _) = glyph::size(ctx, &view);
        let x = (i32::from(buf.width()) - width).max(0) / 2;
        glyph::paint(ctx, &view, buf, x, 6);
        let help = ctx.text_with("levels.help", &[]);
        let last = i32::from(buf.height()) - 1;
        print_centred(buf, last, &help, c(EngineColor::TextDim), bg);
    }

    fn as_any(&self) -> Option<&dyn Any> {
        Some(self)
    }
}
