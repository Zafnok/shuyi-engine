//! Screens and the screen stack: the shape every screen of a game has.
//!
//! A [`Screen`] reads the frame's actions ([`FrameInput`]) in
//! [`update`](Screen::update) and answers with a [`Transition`]; it paints
//! itself into a [`GlyphBuffer`] in [`draw`](Screen::draw). The
//! [`ScreenStack`] routes input to the top screen only and draws from the
//! top-most opaque screen up, so overlays (menus, dialogs) show the screen
//! below them. Screens are generic over the game's part of the context
//! ([`GameData`]), which also gives their actions.
//!
//! A screen can ask something on a screen of its own and get the answer
//! back: it pushes the question, which pops with a value
//! ([`Transition::pop_with`]), and the stack hands that value to the
//! screen below in [`uncovered`](Screen::uncovered). A screen also hears
//! when one is pushed over it ([`covered`](Screen::covered)) and when that
//! one goes, with or without a value.

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

    /// Called when a screen is pushed on top of this one
    /// ([`Transition::Push`]): it is no longer updated until it is
    /// [`uncovered`](Self::uncovered). Not called when the screen above
    /// swaps itself for another ([`Transition::Replace`]). Does nothing
    /// unless the screen needs it.
    fn covered(&mut self, _ctx: &mut Ctx<G>) {}

    /// Called when the screen above pops and this one is on top again:
    /// `result` is what it popped with ([`Transition::PopWith`]), `None`
    /// after a plain [`Transition::Pop`]. The transition returned is
    /// applied at once, as if from [`update`](Self::update), so a screen
    /// can act on the answer in the same frame: push the next screen, or
    /// pop with an answer of its own. By default, nothing happens.
    ///
    /// ```
    /// # use shuyi_ui::screen::ScreenResult;
    /// // A question screen answers yes…
    /// let result = Some(ScreenResult::new(true));
    /// // …and the screen below reads it as the type the two agree on.
    /// let yes = result.and_then(ScreenResult::take::<bool>) == Some(true);
    /// assert!(yes);
    /// ```
    fn uncovered(&mut self, ctx: &mut Ctx<G>, result: Option<ScreenResult>) -> Transition<G> {
        let _ = (ctx, result);
        Transition::None
    }
}

/// A value a screen pops with ([`Transition::PopWith`]) for the screen
/// below, which reads it back as its type in [`Screen::uncovered`]. Any
/// `'static` type will do; the two screens agree on which.
pub struct ScreenResult {
    value: Box<dyn Any>,
    type_name: &'static str,
}

impl ScreenResult {
    /// `value`, to hand to the screen below.
    pub fn new<T: Any>(value: T) -> Self {
        Self {
            value: Box::new(value),
            type_name: core::any::type_name::<T>(),
        }
    }

    /// Whether the value is a `T`.
    pub fn is<T: Any>(&self) -> bool {
        self.value.is::<T>()
    }

    /// The value, if it is a `T`.
    pub fn get<T: Any>(&self) -> Option<&T> {
        self.value.downcast_ref()
    }

    /// The value, if it is a `T`; dropped if it isn't.
    pub fn take<T: Any>(self) -> Option<T> {
        self.downcast().ok()
    }

    /// The value, if it is a `T`.
    ///
    /// # Errors
    ///
    /// The result itself, unchanged, when its value is not a `T`.
    pub fn downcast<T: Any>(self) -> Result<T, Self> {
        let type_name = self.type_name;
        self.value
            .downcast()
            .map(|value| *value)
            .map_err(|value| Self { value, type_name })
    }
}

// check-text: not player text (debug output)
impl fmt::Debug for ScreenResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ScreenResult({})", self.type_name)
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
    /// [`Pop`](Self::Pop), handing a value to the screen below
    /// ([`Screen::uncovered`]). [`Transition::pop_with`] makes one.
    PopWith(ScreenResult),
    /// Swap this screen for another.
    Replace(Box<dyn Screen<G>>),
    /// Quit the game.
    Quit,
}

impl<G: GameData> Transition<G> {
    /// [`Transition::PopWith`] `value`.
    pub fn pop_with<T: Any>(value: T) -> Self {
        Self::PopWith(ScreenResult::new(value))
    }
}

// check-text: not player text (debug output)
impl<G: GameData> fmt::Debug for Transition<G> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::None => f.write_str("None"),
            Self::Push(s) => write!(f, "Push({})", s.name()),
            Self::Pop => f.write_str("Pop"),
            Self::PopWith(result) => write!(f, "PopWith({})", result.type_name),
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

    /// Puts `screen` on top, to build a stack: the screen below is not
    /// told ([`Screen::covered`]); [`apply_in`](Self::apply_in) with
    /// [`Transition::Push`] tells it.
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

    /// Updates the top screen and applies its transition
    /// ([`apply_in`](Self::apply_in)). Returns `true` if the game should
    /// quit ([`Transition::Quit`], or the last screen popped). Does nothing
    /// and returns `true` on an empty stack.
    pub fn update(&mut self, ctx: &mut Ctx<G>, input: &FrameInput<G::Action>) -> bool {
        let Some(top) = self.screens.last_mut() else {
            return true;
        };
        let transition = top.update(ctx, input);
        self.apply_in(ctx, transition)
    }

    /// Applies a transition as if the top screen had returned it, telling
    /// the screens it covers and uncovers: a push calls the old top's
    /// [`Screen::covered`]; a pop calls the new top's
    /// [`Screen::uncovered`] with what the screen popped with, then applies
    /// the transition that returns, and so on. A replace tells nobody.
    /// Returns `true` if the game should quit.
    pub fn apply_in(&mut self, ctx: &mut Ctx<G>, transition: Transition<G>) -> bool {
        let mut next = transition;
        loop {
            let result = match next {
                Transition::Pop => None,
                Transition::PopWith(result) => Some(result),
                Transition::Push(screen) => {
                    if let Some(top) = self.screens.last_mut() {
                        top.covered(ctx);
                    }
                    self.screens.push(screen);
                    break;
                }
                other => return self.apply(other),
            };
            self.screens.pop();
            let Some(top) = self.screens.last_mut() else {
                break;
            };
            next = top.uncovered(ctx, result);
        }
        self.screens.is_empty()
    }

    /// Applies a transition as if the top screen had returned it, to the
    /// stack alone: no screen is told it is covered or uncovered, and a
    /// value popped with is dropped ([`apply_in`](Self::apply_in) tells
    /// them). Returns `true` if the game should quit.
    pub fn apply(&mut self, transition: Transition<G>) -> bool {
        match transition {
            Transition::None => {}
            Transition::Push(screen) => self.screens.push(screen),
            Transition::Pop | Transition::PopWith(_) => {
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
