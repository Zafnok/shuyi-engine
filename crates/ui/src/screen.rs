//! Screens and the screen stack: the shape every screen of a game has.
//!
//! A [`Screen`] reads the frame's actions ([`FrameInput`]) in
//! [`update`](Screen::update) and answers with a [`Transition`]; it paints
//! itself into a [`GlyphBuffer`] in [`draw`](Screen::draw). The
//! [`ScreenStack`] routes input to the top screen only and draws from the
//! top-most opaque screen up, so overlays (menus, dialogs) show the screen
//! below them. Screens are generic over the game's part of the context
//! ([`GameData`]), which also gives their actions.

use crate::prelude::*;
use core::any::Any;
use core::fmt;

use crate::ctx::{Ctx, GameData};
use crate::frame::glyph_buffer::GlyphBuffer;

pub use crate::input::frame::FrameInput;

/// One screen of a game: a title, a map, a menu overlay, …
pub trait Screen<G: GameData> {
    /// A stable, unique `snake_case` name, for tests and debugging.
    fn name(&self) -> &'static str;

    /// Handles one frame: `input` holds this frame's actions (often none)
    /// and elapsed time. Called only while this screen is on top.
    fn update(&mut self, ctx: &mut Ctx<G>, input: &FrameInput<G::Action>) -> Transition<G>;

    /// Paints the screen. An opaque screen must cover the whole buffer.
    fn draw(&self, ctx: &Ctx<G>, buf: &mut GlyphBuffer);

    /// Whether the screen below shows through (drawn first, then this one).
    fn is_overlay(&self) -> bool {
        false
    }

    /// This screen as [`Any`], for tests that look inside a screen on the
    /// stack ([`ScreenStack::find`]). `None` unless the screen opts in.
    fn as_any(&self) -> Option<&dyn Any> {
        None
    }

    /// [`as_any`](Self::as_any), mutable ([`ScreenStack::find_mut`]).
    fn as_any_mut(&mut self) -> Option<&mut dyn Any> {
        None
    }
}

/// What a screen asks the stack to do after an update.
pub enum Transition<G: GameData> {
    /// Stay on this screen.
    None,
    /// Put a new screen on top of this one.
    Push(Box<dyn Screen<G>>),
    /// Remove this screen, returning to the one below (or quitting if it was
    /// the last).
    Pop,
    /// Swap this screen for another.
    Replace(Box<dyn Screen<G>>),
    /// Quit the game.
    Quit,
}

// check-text: not player text (debug output)
impl<G: GameData> fmt::Debug for Transition<G> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::None => f.write_str("None"),
            Self::Push(s) => write!(f, "Push({})", s.name()),
            Self::Pop => f.write_str("Pop"),
            Self::Replace(s) => write!(f, "Replace({})", s.name()),
            Self::Quit => f.write_str("Quit"),
        }
    }
}

/// The stack of open screens. Only the top screen is updated; drawing starts
/// at the top-most opaque screen and goes up through the overlays above it.
pub struct ScreenStack<G: GameData> {
    screens: Vec<Box<dyn Screen<G>>>,
}

impl<G: GameData> Default for ScreenStack<G> {
    fn default() -> Self {
        Self {
            screens: Vec::new(),
        }
    }
}

impl<G: GameData> ScreenStack<G> {
    /// A stack holding just `root`.
    pub fn new(root: Box<dyn Screen<G>>) -> Self {
        Self {
            screens: vec![root],
        }
    }

    /// Whether no screen is open (the game should quit).
    pub fn is_empty(&self) -> bool {
        self.screens.is_empty()
    }

    /// Names of the open screens, bottom first.
    pub fn names(&self) -> Vec<&'static str> {
        self.screens.iter().map(|s| s.name()).collect()
    }

    /// Name of the top screen.
    pub fn top_name(&self) -> Option<&'static str> {
        self.screens.last().map(|s| s.name())
    }

    /// Puts `screen` on top.
    pub fn push(&mut self, screen: Box<dyn Screen<G>>) {
        self.screens.push(screen);
    }

    /// The top-most screen of type `T` (one that opts in with
    /// [`Screen::as_any`]).
    pub fn find<T: Any>(&self) -> Option<&T> {
        self.screens
            .iter()
            .rev()
            .find_map(|s| s.as_any()?.downcast_ref())
    }

    /// The top-most screen of type `T`, mutable ([`Screen::as_any_mut`]).
    pub fn find_mut<T: Any>(&mut self) -> Option<&mut T> {
        self.screens
            .iter_mut()
            .rev()
            .find_map(|s| s.as_any_mut()?.downcast_mut())
    }

    /// Updates the top screen and applies its transition. Returns `true`
    /// if the game should quit ([`Transition::Quit`], or the last screen
    /// popped). Does nothing and returns `true` on an empty stack.
    pub fn update(&mut self, ctx: &mut Ctx<G>, input: &FrameInput<G::Action>) -> bool {
        let Some(top) = self.screens.last_mut() else {
            return true;
        };
        let transition = top.update(ctx, input);
        self.apply(transition)
    }

    /// Applies a transition as if the top screen had returned it. Returns
    /// `true` if the game should quit.
    pub fn apply(&mut self, transition: Transition<G>) -> bool {
        match transition {
            Transition::None => {}
            Transition::Push(screen) => self.screens.push(screen),
            Transition::Pop => {
                self.screens.pop();
            }
            Transition::Replace(screen) => {
                self.screens.pop();
                self.screens.push(screen);
            }
            Transition::Quit => return true,
        }
        self.screens.is_empty()
    }

    /// Draws the top-most opaque screen and every overlay above it, bottom
    /// up. If every screen is an overlay, all are drawn.
    pub fn draw(&self, ctx: &Ctx<G>, buf: &mut GlyphBuffer) {
        let base = self
            .screens
            .iter()
            .rposition(|s| !s.is_overlay())
            .unwrap_or(0);
        for screen in &self.screens[base..] {
            screen.draw(ctx, buf);
        }
    }
}

#[cfg(test)]
mod tests;
