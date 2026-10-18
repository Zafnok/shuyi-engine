//! An options screen: the player's settings one per row, changed with the
//! cursor's left and right keys, and rows that do something (open a
//! screen, ask a question). The game gives the rows ([`OptionRow`]), each
//! a label key and a typed value: on or off ([`RowValue::Toggle`]), one of
//! several ([`RowValue::Choice`]), a volume ([`RowValue::Volume`]), or
//! something Confirm does ([`RowValue::Action`]). The engine's own
//! settings come as rows ready made ([`OptionRow::text_speed`],
//! [`OptionRow::fullscreen`], the volumes, [`OptionRow::restore_defaults`]).
//!
//! A change shows at once but is saved only when the player keeps it:
//! leaving with anything changed asks whether to keep it, throw it away or
//! stay ([`LeaveChoice`]). A row whose value isn't a setting
//! ([`OptionRow::unsaved`]) is kept or put back with them. A volume goes
//! from 0 to [`MAX_VOLUME`]: left and right move its slider, Confirm opens
//! a box to type the exact number ([`NumberBox`]). Whatever does something
//! big asks first ([`Question`]).
//!
//! **The look is a skin.** This module decides what the screen shows and
//! does, and says it as plain data: an [`OptionsView`]
//! ([`OptionsScreen::view`]). [`glyph::paint`] draws that view as glyphs.
//! Nothing here knows a cell, a colour or a position, so another look (a
//! bought UI pack) is another painter of the same view. Tests of what
//! happened read the view or the settings; only [`glyph`]'s tests read
//! cells.

pub mod glyph;
pub mod view;

use crate::prelude::*;
use core::fmt;
pub use view::{ChoiceView, LeaveView, OptionsView, QuestionView, RowView, ValueView};

use shuyi_assets::keymap::{ActionSet, UiAction};

use super::text_keys_help;
use crate::audio::MenuSound;
use crate::ctx::{Ctx, GameData};
use crate::frame::glyph_buffer::GlyphBuffer;
use crate::input::fixed::{TextKey, text_key};
use crate::screen::{FrameInput, Screen, Transition};
use crate::settings::{MAX_VOLUME, TextSpeed};

/// Text key ([`Ctx::text`]) of the screen title.
pub const TITLE: &str = "options.title";
/// Text key of the question asked on leaving with changes.
pub const LEAVE_QUESTION: &str = "options.question.keep";
/// Text key of how a yes/no question is answered, with the keys.
pub const QUESTION_ANSWERS: &str = "options.question.answers";
/// Text key of the message if something couldn't be saved.
pub const NOT_SAVED_MESSAGE: &str = "options.message.not_saved";
/// Text key of the message once the defaults are back.
pub const RESTORED_MESSAGE: &str = "options.message.restored";
/// Text keys of an on/off value: off, then on.
pub const ON_OFF: [&str; 2] = ["options.value.off", "options.value.on"];
/// Text key of the help line on a row whose value the cursor changes.
pub const HELP_CHANGE: &str = "options.help.change";
/// Text key of the help line on a volume.
pub const HELP_VOLUME: &str = "options.help.volume";
/// Text key of the help line while the volume box is open on a
/// controller.
pub const HELP_NUMBER_PAD: &str = "options.help.number_pad";
/// Text key of the help line while a yes/no question is open.
pub const HELP_QUESTION: &str = "options.help.question";
/// Text key of the help line while the question asked on leaving is open.
pub const HELP_LEAVE: &str = "options.help.leave";
/// Text key of the help line on a row that opens a screen.
pub const HELP_OPEN: &str = "options.help.open";

/// How far the cursor's left and right keys move a volume's slider.
/// *Tunable.*
pub const VOLUME_STEP: u8 = 5;
/// The most digits the volume box takes.
const MAX_DIGITS: usize = 3;

/// One row of an options screen, as the game gives it.
pub struct OptionRow<G: GameData> {
    /// The text key of its label; also what names the row (the focus).
    pub key: &'static str,
    /// What it shows beside its label, and what changes it.
    pub value: RowValue<G>,
    /// Whether a blank row comes before it (the first row of a group).
    pub starts_group: bool,
    /// Whether its value isn't one of the settings, so the screen keeps or
    /// puts it back itself on leaving (a campaign's own level, say).
    pub unsaved: bool,
}

/// What a row shows beside its label, and what changes it.
pub enum RowValue<G: GameData> {
    /// On or off: the cursor's left key turns it off, the right key on,
    /// Confirm the other way.
    Toggle {
        /// Whether it is on.
        get: fn(&Ctx<G>) -> bool,
        /// Turns it on or off; whether anything changed.
        set: fn(&mut Ctx<G>, bool) -> bool,
    },
    /// One of several values: the cursor's keys step through them,
    /// stopping at the ends, and Confirm goes round.
    Choice {
        /// The text keys of the values offered, in the order they step.
        values: fn(&Ctx<G>) -> Vec<&'static str>,
        /// Which of them the row shows.
        get: fn(&Ctx<G>) -> usize,
        /// Puts value `i` in; whether anything changed.
        set: fn(&mut Ctx<G>, usize) -> bool,
    },
    /// A volume, 0 to [`MAX_VOLUME`]: the keys move its slider by
    /// [`VOLUME_STEP`], Confirm opens a box to type the number.
    Volume {
        /// The volume.
        get: fn(&Ctx<G>) -> u8,
        /// Sets it; whether anything changed.
        set: fn(&mut Ctx<G>, u8) -> bool,
    },
    /// Something Confirm does.
    Action {
        /// What the row shows beside its label, if anything (the layout
        /// in use, say).
        value: fn(&Ctx<G>) -> Option<String>,
        /// What Confirm on it does.
        confirm: fn(&mut Ctx<G>) -> RowAction<G>,
        /// The text key of the help line while it has the focus.
        help: fn(&Ctx<G>) -> &'static str,
    },
}

/// What Confirm on an [`RowValue::Action`] row does.
pub enum RowAction<G: GameData> {
    /// Opens a screen over the options.
    Open(Box<dyn Screen<G>>),
    /// Asks first.
    Ask(Question<G>),
    /// Nothing can be done here: the denied sound, and this message under
    /// the panel if there is one.
    Refuse(Option<&'static str>),
}

/// A yes/no question asked before something that can't be taken back with
/// one key.
pub struct Question<G: GameData> {
    /// The text key of the question; also what names it
    /// ([`OptionsScreen::asking`]).
    pub key: &'static str,
    /// Does it, on a yes: the text key of the message to show.
    pub yes: fn(&mut Ctx<G>) -> &'static str,
}

impl<G: GameData> Clone for Question<G> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<G: GameData> Copy for Question<G> {}

impl<G: GameData> fmt::Debug for Question<G> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Question").field(&self.key).finish()
    }
}

impl<G: GameData> OptionRow<G> {
    /// A row labelled by the text key `key`, showing `value`.
    pub fn new(key: &'static str, value: RowValue<G>) -> Self {
        Self {
            key,
            value,
            starts_group: false,
            unsaved: false,
        }
    }

    /// The same row, first of a group: a blank row comes before it.
    #[must_use]
    pub fn starting_group(mut self) -> Self {
        self.starts_group = true;
        self
    }

    /// The same row, whose value isn't one of the settings: the screen
    /// keeps it or puts it back itself on leaving.
    #[must_use]
    pub fn unsaved(mut self) -> Self {
        self.unsaved = true;
        self
    }

    /// The speed text is revealed at ([`TextSpeed`], in the settings).
    pub fn text_speed() -> Self {
        Self::new(
            "options.row.text_speed",
            RowValue::Choice {
                values: |_| TextSpeed::ALL.map(TextSpeed::key).to_vec(),
                get: |ctx| {
                    let speed = ctx.settings().text_speed;
                    TextSpeed::ALL.iter().position(|&s| s == speed).unwrap_or(0)
                },
                set: |ctx, i| {
                    let speed = TextSpeed::ALL.get(i).copied();
                    ctx.preview_settings(|s| s.text_speed = speed.unwrap_or(s.text_speed))
                },
            },
        )
    }

    /// Whether the game fills the screen (in the settings).
    pub fn fullscreen() -> Self {
        Self::new(
            "options.row.fullscreen",
            RowValue::Toggle {
                get: |ctx| ctx.settings().fullscreen,
                set: |ctx, on| ctx.preview_settings(|s| s.fullscreen = on),
            },
        )
    }

    /// How loud the music plays (in the settings).
    pub fn music_volume() -> Self {
        Self::new(
            "options.row.music_volume",
            RowValue::Volume {
                get: |ctx| ctx.settings().music_volume,
                set: |ctx, v| ctx.preview_settings(|s| s.music_volume = v),
            },
        )
    }

    /// How loud the sounds play (in the settings).
    pub fn sound_volume() -> Self {
        Self::new(
            "options.row.sound_volume",
            RowValue::Volume {
                get: |ctx| ctx.settings().sound_volume,
                set: |ctx, v| ctx.preview_settings(|s| s.sound_volume = v),
            },
        )
    }

    /// Puts every setting back to its default but the layout, asking
    /// first; kept or thrown away on leaving, like any change.
    pub fn restore_defaults() -> Self {
        Self::new(
            "options.row.restore_defaults",
            RowValue::Action {
                value: |_| None,
                confirm: |_| {
                    RowAction::Ask(Question {
                        key: "options.question.restore",
                        yes: |ctx| {
                            ctx.preview_settings(|s| *s = s.restored());
                            RESTORED_MESSAGE
                        },
                    })
                },
                help: |_| "options.help.restore",
            },
        )
    }

    /// Whether the cursor's left and right keys change its value.
    pub fn is_setting(&self) -> bool {
        !matches!(self.value, RowValue::Action { .. })
    }

    /// The text key of the help line while it has the focus.
    fn help(&self, ctx: &Ctx<G>) -> &'static str {
        match &self.value {
            RowValue::Toggle { .. } | RowValue::Choice { .. } => HELP_CHANGE,
            RowValue::Volume { .. } => HELP_VOLUME,
            RowValue::Action { help, .. } => help(ctx),
        }
    }

    /// The value it shows now, as an index: a choice's, on (1) or off (0),
    /// the volume; `None` for a row that only does something.
    fn index(&self, ctx: &Ctx<G>) -> Option<usize> {
        match &self.value {
            RowValue::Toggle { get, .. } => Some(usize::from(get(ctx))),
            RowValue::Choice { get, .. } => Some(get(ctx)),
            RowValue::Volume { get, .. } => Some(usize::from(get(ctx))),
            RowValue::Action { .. } => None,
        }
    }

    /// Puts back the value [`index`](Self::index) said it had.
    fn put_back(&self, ctx: &mut Ctx<G>, index: usize) {
        match &self.value {
            RowValue::Toggle { set, .. } => {
                set(ctx, index == 1);
            }
            RowValue::Choice { set, .. } => {
                set(ctx, index);
            }
            RowValue::Volume { set, .. } => {
                set(ctx, u8::try_from(index).unwrap_or(MAX_VOLUME));
            }
            RowValue::Action { .. } => {}
        }
    }
}

/// `n` values' index one step after (`forward`) or before `current`,
/// stopping at the ends (`wrap`: going round instead).
fn stepped_index(n: usize, current: usize, forward: bool, wrap: bool) -> usize {
    if n == 0 {
        return current;
    }
    let i = current.min(n - 1);
    match (forward, wrap) {
        (true, true) => (i + 1) % n,
        (true, false) => (i + 1).min(n - 1),
        (false, true) => (i + n - 1) % n,
        (false, false) => i.saturating_sub(1),
    }
}

/// `all`'s value one step after (`forward`) or before `current`, stopping
/// at the ends (`wrap`: going round instead). A value not in `all` steps
/// from the first.
pub fn stepped<T: Copy + PartialEq>(all: &[T], current: T, forward: bool, wrap: bool) -> T {
    let i = all.iter().position(|&v| v == current).unwrap_or(0);
    let to = stepped_index(all.len(), i, forward, wrap);
    all.get(to).copied().unwrap_or(current)
}

/// A volume one slider step ([`VOLUME_STEP`]) up or down, within 0 to
/// [`MAX_VOLUME`].
fn stepped_volume(volume: u8, forward: bool) -> u8 {
    if forward {
        volume.saturating_add(VOLUME_STEP).min(MAX_VOLUME)
    } else {
        volume.saturating_sub(VOLUME_STEP)
    }
}

/// The box that takes a volume's exact number. From the keyboard the
/// digits are typed (the game's keys do nothing meanwhile; the text box's
/// fixed keys finish, delete and cancel). Opened with a controller button
/// it holds the volume, and the cursor steps it by one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NumberBox {
    /// What is typed so far (on a controller, the number as stepped).
    digits: String,
    /// Whether a controller opened it.
    pad: bool,
}

impl NumberBox {
    /// The number in the box, at most [`MAX_VOLUME`]; `None` while empty.
    pub fn value(&self) -> Option<u8> {
        let typed: u32 = self.digits.parse().ok()?;
        Some(u8::try_from(typed.min(u32::from(MAX_VOLUME))).unwrap_or(MAX_VOLUME))
    }

    /// The digits in the box.
    pub fn digits(&self) -> &str {
        &self.digits
    }
}

/// An answer to the question asked on leaving with changes
/// ([`LEAVE_QUESTION`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeaveChoice {
    /// Save the changes and leave.
    Keep,
    /// Put every setting back as it was when the screen opened, and leave.
    Discard,
    /// Stay on the screen.
    Back,
}

impl LeaveChoice {
    /// Every answer, in the order shown.
    pub const ALL: [LeaveChoice; 3] = [LeaveChoice::Keep, LeaveChoice::Discard, LeaveChoice::Back];

    /// The text key of its name.
    pub const fn key(self) -> &'static str {
        match self {
            LeaveChoice::Keep => "options.leave.keep",
            LeaveChoice::Discard => "options.leave.discard",
            LeaveChoice::Back => "options.leave.back",
        }
    }
}

/// The game's rows, as they are now: the ones `ctx` shows, top to bottom.
pub type Rows<G> = fn(&Ctx<G>) -> Vec<OptionRow<G>>;

/// The options screen. Cancel closes it, asking first if anything changed.
pub struct OptionsScreen<G: GameData> {
    /// The game's rows.
    rows: Rows<G>,
    /// The focused row's key; the first row's until the cursor moves.
    row: Option<&'static str>,
    /// The settings as they were when the screen opened (taken on its
    /// first frame), for Discard.
    opened_with: Option<crate::settings::Settings<G::Settings>>,
    /// The values of the rows that aren't settings, by key, when the
    /// screen opened (taken with `opened_with`), for Discard.
    opened_unsaved: Vec<(&'static str, usize)>,
    /// The answer in focus while the question asked on leaving is open.
    leaving: Option<LeaveChoice>,
    /// The question open, if any.
    asking: Option<Question<G>>,
    /// The volume box, while it is open on the focused volume row.
    typing: Option<NumberBox>,
    /// The text key of the message under the panel, if any.
    message: Option<&'static str>,
}

impl<G: GameData> fmt::Debug for OptionsScreen<G> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OptionsScreen")
            .field("row", &self.row)
            .field("opened_unsaved", &self.opened_unsaved)
            .field("leaving", &self.leaving)
            .field("asking", &self.asking)
            .field("typing", &self.typing)
            .field("message", &self.message)
            .finish_non_exhaustive()
    }
}

impl<G: GameData> OptionsScreen<G> {
    /// Name reported by [`Screen::name`].
    pub const NAME: &'static str = "options";

    /// The screen of `rows`, its first row focused.
    pub fn new(rows: Rows<G>) -> Self {
        Self {
            rows,
            row: None,
            opened_with: None,
            opened_unsaved: Vec::new(),
            leaving: None,
            asking: None,
            typing: None,
            message: None,
        }
    }

    /// The same screen focused on the row `key` (on the first row while
    /// that one isn't shown).
    #[must_use]
    pub fn focused_on(mut self, key: &'static str) -> Self {
        self.row = Some(key);
        self
    }

    /// The rows `ctx` shows, top to bottom.
    pub fn rows(&self, ctx: &Ctx<G>) -> Vec<OptionRow<G>> {
        (self.rows)(ctx)
    }

    /// The focused row's key (`""` with no rows).
    pub fn focus(&self, ctx: &Ctx<G>) -> &'static str {
        let rows = self.rows(ctx);
        let shown = |key| rows.iter().any(|r| r.key == key);
        match self.row {
            Some(key) if shown(key) => key,
            _ => rows.first().map_or("", |r| r.key),
        }
    }

    /// The focused row.
    fn focused_row(&self, ctx: &Ctx<G>) -> Option<OptionRow<G>> {
        let key = self.focus(ctx);
        self.rows(ctx).into_iter().find(|r| r.key == key)
    }

    /// The key of the question open, if any.
    pub fn asking(&self) -> Option<&'static str> {
        self.asking.map(|q| q.key)
    }

    /// The answer in focus while the question asked on leaving with
    /// changes is open.
    pub fn leaving(&self) -> Option<LeaveChoice> {
        self.leaving
    }

    /// Whether any setting, or any row that isn't one, differs from when
    /// the screen opened (the layout aside: it is saved when picked).
    pub fn changed(&self, ctx: &Ctx<G>) -> bool {
        self.opened_with.as_ref().is_some_and(|opened| {
            let opened = opened.clone().with_layout_of(ctx.settings());
            opened != *ctx.settings() || self.unsaved_changed(ctx)
        })
    }

    /// Whether a row that isn't a setting shows another value than when the
    /// screen opened.
    fn unsaved_changed(&self, ctx: &Ctx<G>) -> bool {
        let rows = self.rows(ctx);
        self.opened_unsaved.iter().any(|&(key, was)| {
            rows.iter()
                .find(|r| r.key == key)
                .is_some_and(|r| r.index(ctx) != Some(was))
        })
    }

    /// The volume box, while it is open.
    pub fn typing(&self) -> Option<&NumberBox> {
        self.typing.as_ref()
    }

    /// The text key of the message under the panel, if any.
    pub fn message(&self) -> Option<&str> {
        self.message
    }

    /// What `row` shows beside its name. (The volume box, while open, is
    /// the screen's: [`view`](Self::view).)
    pub fn value(ctx: &Ctx<G>, row: &OptionRow<G>) -> ValueView {
        let words = |text: String| ValueView::Text {
            text,
            adjustable: row.is_setting(),
        };
        match &row.value {
            RowValue::Toggle { get, .. } => {
                words(ctx.text(ON_OFF[usize::from(get(ctx))]).to_owned())
            }
            RowValue::Choice { values, get, .. } => {
                let values = values(ctx);
                match values.get(get(ctx)).or(values.first()) {
                    Some(key) => words(ctx.text(key).to_owned()),
                    // Nothing to choose from: nothing to show.
                    None => ValueView::None,
                }
            }
            RowValue::Volume { get, .. } => ValueView::Volume {
                level: get(ctx),
                max: MAX_VOLUME,
            },
            RowValue::Action { value, .. } => value(ctx).map_or(ValueView::None, words),
        }
    }

    /// The screen as it is now, as plain data for a skin to paint
    /// ([`glyph::paint`]): every row with its name and value, the row in
    /// focus, the volume box if it is open, the message, the question and
    /// the help line, all in the player's language and naming their keys.
    pub fn view(&self, ctx: &Ctx<G>) -> OptionsView {
        let focus = self.focus(ctx);
        let rows = self.rows(ctx).into_iter().map(|row| {
            let focused = row.key == focus;
            let value = match self.typing.as_ref().filter(|_| focused) {
                Some(typing) => ValueView::NumberBox {
                    digits: typing.digits.clone(),
                    typing: !typing.pad,
                },
                None => Self::value(ctx, &row),
            };
            RowView {
                key: row.key,
                label: ctx.text(row.key).to_owned(),
                value,
                focused,
                starts_group: row.starts_group,
            }
        });
        OptionsView {
            title: ctx.text(TITLE).to_owned(),
            rows: rows.collect(),
            message: self.message.map(|key| ctx.text(key).to_owned()),
            question: self.asking.map(|question| QuestionView {
                text: ctx.text(question.key).to_owned(),
                answers: ctx.text_with(QUESTION_ANSWERS, &[]),
            }),
            leaving: self.leaving.map(|focus| LeaveView {
                text: ctx.text(LEAVE_QUESTION).to_owned(),
                choices: LeaveChoice::ALL
                    .into_iter()
                    .map(|choice| ChoiceView {
                        choice,
                        label: ctx.text(choice.key()).to_owned(),
                        focused: choice == focus,
                    })
                    .collect(),
            }),
            help: self.help(ctx),
        }
    }

    /// Moves the focus one row up or down, wrapping.
    fn move_row(&mut self, ctx: &mut Ctx<G>, down: bool) {
        let keys: Vec<_> = self.rows(ctx).iter().map(|r| r.key).collect();
        self.row = Some(stepped(&keys, self.focus(ctx), down, true));
        self.message = None;
        ctx.audio.menu(MenuSound::Move);
    }

    /// Steps the focused row's value one on (`forward`) or back; `wrap`
    /// goes round at the ends. If that changed anything, plays the move
    /// sound (at the new volume, for the sound volume).
    fn change(&mut self, ctx: &mut Ctx<G>, forward: bool, wrap: bool) {
        let Some(row) = self.focused_row(ctx) else {
            return;
        };
        let changed = match row.value {
            RowValue::Toggle { get, set } => {
                let to = stepped(&[false, true], get(ctx), forward, wrap);
                set(ctx, to)
            }
            RowValue::Choice { values, get, set } => {
                let n = values(ctx).len();
                let to = stepped_index(n, get(ctx), forward, wrap);
                set(ctx, to)
            }
            RowValue::Volume { get, set } => {
                let to = stepped_volume(get(ctx), forward);
                set(ctx, to)
            }
            RowValue::Action { .. } => false,
        };
        if changed {
            self.message = None;
            ctx.audio.menu(MenuSound::Move);
        }
    }

    /// Opens the volume box on the focused volume row: empty, to type in;
    /// or, opened with a controller button (`pad`), holding the volume.
    fn open_box(&mut self, ctx: &mut Ctx<G>, volume: u8, pad: bool) {
        let digits = if pad {
            volume.to_string()
        } else {
            String::new()
        };
        self.typing = Some(NumberBox { digits, pad });
        ctx.audio.menu(MenuSound::Select);
    }

    /// Closes the volume box. `keep`: its number becomes the volume (an
    /// empty box keeps the volume as it was).
    fn close_box(&mut self, ctx: &mut Ctx<G>, keep: bool) {
        let value = self.typing.take().and_then(|b| b.value()).filter(|_| keep);
        let Some(value) = value else {
            ctx.audio.menu(MenuSound::Cancel);
            return;
        };
        ctx.audio.menu(MenuSound::Select);
        let Some(RowValue::Volume { set, .. }) = self.focused_row(ctx).map(|r| r.value) else {
            return;
        };
        if set(ctx, value) {
            self.message = None;
            ctx.audio.menu(MenuSound::Move);
        }
    }

    /// One frame of the volume box.
    fn type_number(&mut self, ctx: &mut Ctx<G>, input: &FrameInput<G::Action>) {
        if self.typing.as_ref().is_some_and(|b| b.pad) {
            for &action in &input.actions {
                let up = match action.ui() {
                    Some(UiAction::Up | UiAction::Right) => true,
                    Some(UiAction::Down | UiAction::Left) => false,
                    Some(UiAction::Confirm) => return self.close_box(ctx, true),
                    Some(UiAction::Cancel) => return self.close_box(ctx, false),
                    _ => continue,
                };
                let Some(b) = &mut self.typing else { return };
                let now = b.value().unwrap_or(0);
                let to = if up {
                    now.saturating_add(1).min(MAX_VOLUME)
                } else {
                    now.saturating_sub(1)
                };
                if to != now {
                    b.digits = to.to_string();
                    ctx.audio.menu(MenuSound::Move);
                }
            }
            return;
        }
        for &c in input.text() {
            let Some(b) = &mut self.typing else { return };
            if c.is_ascii_digit() && b.digits.len() < MAX_DIGITS {
                b.digits.push(c);
            } else {
                ctx.audio.menu(MenuSound::Denied);
            }
        }
        for &chord in input.pressed_chords() {
            match text_key(chord) {
                Some(TextKey::Done) => return self.close_box(ctx, true),
                Some(TextKey::Cancel) => return self.close_box(ctx, false),
                Some(TextKey::Delete) => {
                    let Some(b) = &mut self.typing else { return };
                    if b.digits.pop().is_some() {
                        ctx.audio.menu(MenuSound::Cancel);
                    }
                }
                None => {}
            }
        }
    }

    /// Confirm on the focused row (`pad`: with a controller button).
    fn confirm(&mut self, ctx: &mut Ctx<G>, pad: bool) -> Transition<G> {
        self.message = None;
        let Some(row) = self.focused_row(ctx) else {
            return Transition::None;
        };
        match row.value {
            RowValue::Volume { get, .. } => {
                let volume = get(ctx);
                self.open_box(ctx, volume, pad);
            }
            RowValue::Action { confirm, .. } => match confirm(ctx) {
                RowAction::Open(screen) => {
                    ctx.audio.menu(MenuSound::Select);
                    return Transition::Push(screen);
                }
                RowAction::Ask(question) => {
                    self.asking = Some(question);
                    ctx.audio.menu(MenuSound::Select);
                }
                RowAction::Refuse(message) => {
                    self.message = message;
                    ctx.audio.menu(MenuSound::Denied);
                }
            },
            RowValue::Toggle { .. } | RowValue::Choice { .. } => self.change(ctx, true, true),
        }
        Transition::None
    }

    /// An action while `question` is open: Confirm does it, Cancel backs
    /// out; either closes the question (and returns `true`). Nothing else
    /// does anything.
    fn answer(&mut self, ctx: &mut Ctx<G>, question: Question<G>, action: G::Action) -> bool {
        match action.ui() {
            Some(UiAction::Confirm) => {
                self.message = Some((question.yes)(ctx));
                ctx.audio.menu(MenuSound::Select);
            }
            Some(UiAction::Cancel) => ctx.audio.menu(MenuSound::Cancel),
            _ => return false,
        }
        self.asking = None;
        true
    }

    /// Cancel on the rows: leaves, or with anything changed asks whether
    /// to keep the changes, focused on staying.
    fn leave(&mut self, ctx: &mut Ctx<G>) -> Transition<G> {
        if self.changed(ctx) {
            self.leaving = Some(LeaveChoice::Back);
            self.message = None;
            ctx.audio.menu(MenuSound::Select);
            return Transition::None;
        }
        ctx.audio.menu(MenuSound::Cancel);
        Transition::Pop
    }

    /// An action while the question asked on leaving is open, focused on
    /// `focus`: the cursor moves between the answers, Confirm picks one,
    /// Cancel stays. Returns the transition and whether the question
    /// closed.
    fn answer_leave(
        &mut self,
        ctx: &mut Ctx<G>,
        focus: LeaveChoice,
        action: G::Action,
    ) -> (Transition<G>, bool) {
        let choice = match action.ui() {
            Some(ui @ (UiAction::Up | UiAction::Left | UiAction::Down | UiAction::Right)) => {
                let forward = matches!(ui, UiAction::Down | UiAction::Right);
                let to = stepped(&LeaveChoice::ALL, focus, forward, false);
                if to != focus {
                    self.leaving = Some(to);
                    ctx.audio.menu(MenuSound::Move);
                }
                return (Transition::None, false);
            }
            Some(UiAction::Confirm) => focus,
            Some(UiAction::Cancel) => LeaveChoice::Back,
            _ => return (Transition::None, false),
        };
        self.leaving = None;
        match choice {
            LeaveChoice::Keep => {
                if ctx.keep_settings().is_err() {
                    self.message = Some(NOT_SAVED_MESSAGE);
                    ctx.audio.menu(MenuSound::Denied);
                    return (Transition::None, true);
                }
                ctx.audio.menu(MenuSound::Select);
                (Transition::Pop, true)
            }
            LeaveChoice::Discard => {
                if let Some(opened) = self.opened_with.clone() {
                    // Saves only if something else was saved meanwhile;
                    // the screen closes either way.
                    let _ = ctx.restore_settings(&opened);
                    self.put_back_unsaved(ctx);
                }
                ctx.audio.menu(MenuSound::Cancel);
                (Transition::Pop, true)
            }
            LeaveChoice::Back => {
                ctx.audio.menu(MenuSound::Cancel);
                (Transition::None, true)
            }
        }
    }

    /// Puts each row that isn't a setting back as it was when the screen
    /// opened.
    fn put_back_unsaved(&self, ctx: &mut Ctx<G>) {
        for &(key, was) in &self.opened_unsaved {
            if let Some(row) = self.rows(ctx).into_iter().find(|r| r.key == key) {
                row.put_back(ctx, was);
            }
        }
    }

    /// The bottom help line, naming the keys of the active keymap (or
    /// their buttons, on a controller): what Confirm does on the focused
    /// row, if anything.
    pub fn help(&self, ctx: &Ctx<G>) -> String {
        match &self.typing {
            // Typing: the text box's own keys.
            Some(b) if !b.pad => text_keys_help(ctx),
            Some(_) => ctx.text_with(HELP_NUMBER_PAD, &[]),
            None => {
                let key = if self.leaving.is_some() {
                    HELP_LEAVE
                } else if self.asking.is_some() {
                    HELP_QUESTION
                } else {
                    self.focused_row(ctx).map_or(HELP_CHANGE, |r| r.help(ctx))
                };
                ctx.text_with(key, &[])
            }
        }
    }

    /// Takes what Discard puts back, on the first frame.
    fn take_opened(&mut self, ctx: &Ctx<G>) {
        if self.opened_with.is_some() {
            return;
        }
        self.opened_with = Some(ctx.settings().clone());
        let rows = self.rows(ctx);
        let unsaved = rows.iter().filter(|r| r.unsaved);
        self.opened_unsaved = unsaved
            .filter_map(|r| Some((r.key, r.index(ctx)?)))
            .collect();
    }
}

impl<G: GameData> Screen<G> for OptionsScreen<G> {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn update(&mut self, ctx: &mut Ctx<G>, input: &FrameInput<G::Action>) -> Transition<G> {
        self.take_opened(ctx);
        if self.typing.is_some() {
            // The game's keys do nothing while the box is open.
            self.type_number(ctx, input);
            return Transition::None;
        }
        for &action in &input.actions {
            if let Some(focus) = self.leaving {
                let (transition, closed) = self.answer_leave(ctx, focus, action);
                if closed {
                    // The answer ends this frame's keys.
                    return transition;
                }
                continue;
            }
            if let Some(question) = self.asking {
                // The answer ends this frame's keys.
                if self.answer(ctx, question, action) {
                    break;
                }
                continue;
            }
            match action.ui() {
                Some(UiAction::Up) => self.move_row(ctx, false),
                Some(UiAction::Down) => self.move_row(ctx, true),
                Some(UiAction::Left) => self.change(ctx, false, false),
                Some(UiAction::Right) => self.change(ctx, true, false),
                Some(UiAction::Confirm) => {
                    let transition = self.confirm(ctx, input.pad_pressed());
                    if !matches!(transition, Transition::None) {
                        return transition;
                    }
                    if self.asking.is_some() || self.typing.is_some() {
                        // The key that asked isn't the answer, and the
                        // key that opened the box types nothing.
                        break;
                    }
                }
                Some(UiAction::Cancel) => {
                    let transition = self.leave(ctx);
                    if !matches!(transition, Transition::None) {
                        return transition;
                    }
                    // The key that asked isn't the answer.
                    break;
                }
                _ => {}
            }
        }
        Transition::None
    }

    fn draw(&self, ctx: &Ctx<G>, buf: &mut GlyphBuffer) {
        glyph::paint(ctx, &self.view(ctx), buf);
    }

    fn as_any(&self) -> Option<&dyn core::any::Any> {
        Some(self)
    }
}

#[cfg(test)]
mod tests;
