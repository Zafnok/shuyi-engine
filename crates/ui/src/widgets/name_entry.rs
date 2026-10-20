//! Typing a name: a player profile's, a hero's, a farm's.
//!
//! On a keyboard the player **types** it ([`NameBox`]). While the box is
//! open the game's keys do nothing (its letters are letters then); the
//! text box's own fixed keys finish, delete and cancel ([`text_key`]).
//! With a controller, or on a device with no keyboard, it is spelled on a
//! letter grid instead ([`NameEntry`]) with the cursor and Confirm, like
//! a console's naming screen.
//!
//! Name rules for both ([`NameRules`]): letters (A-Z, a-z), `-`, `'`,
//! digits where the rules allow them, and single spaces between words, at
//! most so many characters, not blank.
//!
//! Neither decides what Cancel means: they report it ([`NameEvent::Cancel`])
//! and the screen closes the box, or asks first. The name is the screen's
//! to take only on [`NameEvent::Done`].
//!
//! **The look is a skin**: [`TypingView`] and [`GridView`] say what a box
//! shows, and [`glyph`] paints them.

pub mod glyph;

use crate::prelude::*;

use shuyi_assets::keymap::{ActionSet, UiAction};

use crate::audio::MenuSound;
use crate::ctx::{Ctx, GameData};
use crate::input::fixed::{TextKey, text_key};
use crate::screen::FrameInput;

/// The text key of the name grid's cell that types a space.
pub const SPACE: &str = "name_entry.grid.blank";
/// The text key of the cell that deletes the last letter.
pub const DELETE: &str = "name_entry.grid.delete";
/// The text key of the cell that closes the name grid.
pub const DONE: &str = "name_entry.grid.done";
/// The text key of the typing box's hint.
pub const TYPE_HINT: &str = "name_entry.type_hint";
/// The text key of the help line while the name grid is open.
pub const HELP_GRID: &str = "name_entry.help.grid";

/// What a name may hold.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NameRules {
    /// The most characters.
    pub max_chars: usize,
    /// Whether digits are allowed (and the grid has a row of them).
    pub digits: bool,
}

impl NameRules {
    /// Whether `c` may go at the end of `name`.
    fn allows(self, name: &str, c: char) -> bool {
        match c {
            _ if name.chars().count() >= self.max_chars => false,
            ' ' => !name.is_empty() && !name.ends_with(' '),
            c if c.is_ascii_digit() => self.digits,
            c => c.is_ascii_alphabetic() || c == '-' || c == '\'',
        }
    }
}

/// Adds `c` to `name` if `rules` allow it.
fn add_char(name: &mut String, c: char, rules: NameRules) -> bool {
    let ok = rules.allows(name, c);
    if ok {
        name.push(c);
    }
    ok
}

/// `name` finished: trimmed, or `None` if blank.
pub fn finished(name: &str) -> Option<String> {
    let name = name.trim();
    (!name.is_empty()).then(|| name.to_owned())
}

/// What a key did in a name box or the name grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NameEvent {
    /// Still typing.
    Open,
    /// The player kept the name: it is [`finished`], not blank.
    Done,
    /// The player asked to close it without keeping the name.
    Cancel,
}

/// The name being typed on the keyboard.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NameBox {
    /// The name typed so far.
    name: String,
    rules: NameRules,
}

impl NameBox {
    /// The box for `name`, which the player types on from.
    pub fn new(name: &str, rules: NameRules) -> Self {
        Self {
            name: name.to_owned(),
            rules,
        }
    }

    /// The name typed so far.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// One frame: the characters typed, then the text box's keys. Actions
    /// are ignored. A character the rules refuse sounds denied; typing
    /// makes no sound. Cancel makes none either: it is the screen's.
    pub fn handle<G: GameData>(
        &mut self,
        input: &FrameInput<G::Action>,
        ctx: &mut Ctx<G>,
    ) -> NameEvent {
        for &c in input.text() {
            if !add_char(&mut self.name, c, self.rules) {
                ctx.audio.menu(MenuSound::Denied);
            }
        }
        for &chord in input.pressed_chords() {
            match text_key(chord) {
                Some(TextKey::Done) => match finished(&self.name) {
                    Some(name) => {
                        self.name = name;
                        ctx.audio.menu(MenuSound::Select);
                        return NameEvent::Done;
                    }
                    None => ctx.audio.menu(MenuSound::Denied),
                },
                Some(TextKey::Delete) => {
                    if self.name.pop().is_some() {
                        ctx.audio.menu(MenuSound::Cancel);
                    }
                }
                Some(TextKey::Cancel) => return NameEvent::Cancel,
                None => {}
            }
        }
        NameEvent::Open
    }

    /// What the box shows, titled `title`, for a skin to paint.
    pub fn view<G: GameData>(&self, ctx: &Ctx<G>, title: &str) -> TypingView {
        TypingView {
            title: title.to_owned(),
            typed: self.name.clone(),
            hint: ctx.text(TYPE_HINT).to_owned(),
        }
    }
}

/// The letters of the name grid, one row each.
// check-text: not player text
const LETTER_ROWS: [&str; 4] = [
    "ABCDEFGHIJKLM",
    "NOPQRSTUVWXYZ",
    "abcdefghijklm",
    "nopqrstuvwxyz",
];

/// The grid's row of digits, when the rules allow them.
// check-text: not player text
const DIGIT_ROW: &str = "0123456789";

/// One cell of the name grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GridCell {
    /// Types this character.
    Char(char),
    /// Types a space (between two words).
    Space,
    /// Deletes the last character.
    Delete,
    /// Closes the grid, keeping the name.
    Done,
}

impl GridCell {
    /// Text shown for the cell, in `ctx`'s language.
    pub fn label<G: GameData>(self, ctx: &Ctx<G>) -> String {
        let key = match self {
            GridCell::Char(c) => return c.to_string(),
            GridCell::Space => SPACE,
            GridCell::Delete => DELETE,
            GridCell::Done => DONE,
        };
        ctx.text(key).to_owned()
    }
}

/// The name grid's rows: four rows of letters, a row of digits if `rules`
/// allow them, then `-`, `'`, Blank, Delete and Done.
pub fn grid(rules: NameRules) -> Vec<Vec<GridCell>> {
    let chars = |row: &str| row.chars().map(GridCell::Char).collect();
    let mut rows: Vec<Vec<GridCell>> = LETTER_ROWS.iter().map(|r| chars(r)).collect();
    if rules.digits {
        rows.push(chars(DIGIT_ROW));
    }
    rows.push(vec![
        GridCell::Char('-'),
        GridCell::Char('\''),
        GridCell::Space,
        GridCell::Delete,
        GridCell::Done,
    ]);
    rows
}

/// The name grid (a controller, or a device with no keyboard).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NameEntry {
    /// The name being spelled.
    name: String,
    /// The focused cell: row, column.
    at: (usize, usize),
    rules: NameRules,
}

impl NameEntry {
    /// The grid for `name`, on its first letter.
    pub fn new(name: &str, rules: NameRules) -> Self {
        Self {
            name: name.to_owned(),
            at: (0, 0),
            rules,
        }
    }

    /// The name spelled so far.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The focused cell: row, column.
    pub fn focus(&self) -> (usize, usize) {
        self.at
    }

    /// Handles `action`: the cursor keys move over the grid (wrapping along
    /// a row, keeping to the last cell of a shorter row), Confirm uses the
    /// focused cell, Cancel deletes the last character (on an empty name it
    /// asks to close the grid). Plays the menu sounds, but not Cancel's
    /// closing one: that is the screen's.
    pub fn handle<G: GameData>(&mut self, action: G::Action, ctx: &mut Ctx<G>) -> NameEvent {
        let rows = grid(self.rules);
        let (row, col) = self.at;
        let width = |r: usize| rows[r].len();
        let moved = match action.ui() {
            Some(UiAction::Left) => (row, (col + width(row) - 1) % width(row)),
            Some(UiAction::Right) => (row, (col + 1) % width(row)),
            Some(UiAction::Up) => {
                let r = (row + rows.len() - 1) % rows.len();
                (r, col.min(width(r) - 1))
            }
            Some(UiAction::Down) => {
                let r = (row + 1) % rows.len();
                (r, col.min(width(r) - 1))
            }
            Some(UiAction::Confirm) => return self.confirm(rows[row][col], ctx),
            Some(UiAction::Cancel) => {
                if self.name.pop().is_some() {
                    ctx.audio.menu(MenuSound::Cancel);
                    return NameEvent::Open;
                }
                return NameEvent::Cancel;
            }
            Some(UiAction::Debug) | None => return NameEvent::Open,
        };
        if moved != self.at {
            self.at = moved;
            ctx.audio.menu(MenuSound::Move);
        }
        NameEvent::Open
    }

    /// Confirm on `cell`. A character the rules refuse (past the most
    /// characters, a space at the start or after another) and Done on a
    /// blank name are refused.
    fn confirm<G: GameData>(&mut self, cell: GridCell, ctx: &mut Ctx<G>) -> NameEvent {
        let ok = match cell {
            GridCell::Char(c) => add_char(&mut self.name, c, self.rules),
            GridCell::Space => add_char(&mut self.name, ' ', self.rules),
            GridCell::Delete => self.name.pop().is_some(),
            GridCell::Done => {
                if let Some(name) = finished(&self.name) {
                    self.name = name;
                    ctx.audio.menu(MenuSound::Select);
                    return NameEvent::Done;
                }
                false
            }
        };
        ctx.audio.menu(if ok {
            MenuSound::Select
        } else {
            MenuSound::Denied
        });
        NameEvent::Open
    }

    /// What the grid shows, titled `title`, for a skin to paint.
    pub fn view<G: GameData>(&self, ctx: &Ctx<G>, title: &str) -> GridView {
        GridView {
            title: title.to_owned(),
            spelled: self.name.clone(),
            rows: grid(self.rules)
                .iter()
                .map(|row| row.iter().map(|g| g.label(ctx)).collect())
                .collect(),
            focus: self.at,
        }
    }
}

/// The typing box.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypingView {
    /// The box's title.
    pub title: String,
    /// The name typed so far (without a cursor: that is the skin's).
    pub typed: String,
    /// What to do: type.
    pub hint: String,
}

/// The letter grid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GridView {
    /// The grid's title.
    pub title: String,
    /// The name spelled so far (without a cursor: that is the skin's).
    pub spelled: String,
    /// The cells, row by row, as their labels.
    pub rows: Vec<Vec<String>>,
    /// The focused cell: row, column.
    pub focus: (usize, usize),
}

#[cfg(test)]
mod tests;
