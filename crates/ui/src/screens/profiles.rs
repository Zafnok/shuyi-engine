//! A player profiles screen: the profiles, the one in use marked, then a
//! row that adds one. It lets the people who share a computer each keep
//! their own settings and saves under a name, on the [`Ctx`] profile
//! functions ([`Ctx::add_profile`] and the rest).
//!
//! Confirm on a profile opens what can be done to it ([`ProfileAction`]):
//! use it, rename it, delete it (asking first, focus on No; never the only
//! profile). Adding asks for a name and uses the new profile; it starts
//! from the default options. Then the screen closes, or opens what the
//! game gives ([`ProfilesScreen::opening_after_add`]: its first-launch
//! setup, say). A name is typed on the keyboard or spelled on the letter
//! grid with a controller ([`crate::widgets::name_entry`]), up to
//! [`MAX_NAME_CHARS`], not another profile's. Cancel in a name box with a
//! name typed and not kept asks whether to keep it, throw it away or go
//! back, focus on going back.
//!
//! **The look is a skin.** This module says what the screen shows as a
//! [`ProfilesView`] ([`ProfilesScreen::view`]); [`glyph::paint`] draws it,
//! unless the game gives another painter ([`ProfilesScreen::painted_by`]).
//! Tests of what happened read the view or the profiles.

pub mod glyph;
pub mod view;

use crate::prelude::*;
use core::fmt;
pub use view::{ActionsView, ProfilesView, RowView};

use shuyi_assets::keymap::{ActionSet, UiAction};

use super::options::LeaveChoice;
use super::text_keys_help;
use crate::audio::MenuSound;
use crate::ctx::{Ctx, GameData};
use crate::frame::glyph_buffer::GlyphBuffer;
use crate::profiles::{MAX_NAME_CHARS, MAX_PROFILES, ProfileError, ProfileId};
use crate::screen::{FrameInput, Screen, Transition};
use crate::widgets::name_entry::{self, NameBox, NameEntry, NameEvent, NameRules, finished};
use crate::widgets::{Ask, ListCursor, Menu, MenuEvent, MenuItem};

/// Text key ([`Ctx::text`]) of the screen title.
pub const TITLE: &str = "profile.title";
/// Text key of the words marking the profile in use.
pub const CURRENT: &str = "profile.current";
/// Text key of the row that adds a profile.
pub const ADD: &str = "profile.add";
/// Text key of the name box's title.
pub const NAME_BOX: &str = "profile.name_box";
/// Text key of the question asked before deleting, with `{name}`.
pub const DELETE_QUESTION: &str = "profile.question.delete";
/// Text key of the question asked on backing out of a name box.
pub const KEEP_QUESTION: &str = "profile.question.keep";
/// Text key of the message when the name is another profile's.
pub const NAME_TAKEN_MESSAGE: &str = "profile.message.name_taken";
/// Text key of the message when there are the most profiles already, with
/// `{n}`.
pub const TOO_MANY_MESSAGE: &str = "profile.message.too_many";
/// Text key of the message if the profiles couldn't be saved.
pub const NOT_SAVED_MESSAGE: &str = "profile.message.not_saved";
/// Text key of the help line on a profile.
pub const HELP_LIST: &str = "profile.help.list";
/// Text key of the help line on the row that adds one.
pub const HELP_ADD: &str = "profile.help.add";
/// Text key of the help line on the row that adds one, with the most
/// profiles already.
pub const HELP_NONE: &str = "profile.help.none";
/// Text key of the help line while a profile's actions are open.
pub const HELP_ACTIONS: &str = "profile.help.actions";
/// Text key of the help line while a question is open.
pub const HELP_QUESTION: &str = "profile.help.question";

/// What a profile's name may hold: letters and digits (the engine names a
/// profile `Player 2`), up to [`MAX_NAME_CHARS`].
pub const NAME_RULES: NameRules = NameRules {
    max_chars: MAX_NAME_CHARS,
    digits: true,
};

/// One row of the screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowKind {
    /// A profile.
    Profile(ProfileId),
    /// Adds a profile.
    Add,
}

/// What can be done to a profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfileAction {
    /// Use it (not offered for the one in use).
    Switch,
    /// Rename it.
    Rename,
    /// Delete it (not offered for the only profile).
    Delete,
}

impl ProfileAction {
    /// The text key of its name.
    pub const fn key(self) -> &'static str {
        match self {
            ProfileAction::Switch => "profile.action.switch",
            ProfileAction::Rename => "profile.action.rename",
            ProfileAction::Delete => "profile.action.delete",
        }
    }
}

/// An answer to the question asked before deleting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeleteChoice {
    /// Delete it.
    Yes,
    /// Don't.
    No,
}

impl DeleteChoice {
    /// Every answer, in the order shown.
    pub const ALL: [DeleteChoice; 2] = [DeleteChoice::Yes, DeleteChoice::No];

    /// The text key of its name.
    pub const fn key(self) -> &'static str {
        match self {
            DeleteChoice::Yes => "profile.answer.yes",
            DeleteChoice::No => "profile.answer.no",
        }
    }

    /// The question asked before deleting ([`DELETE_QUESTION`]): focused
    /// on No, which Cancel answers too.
    pub fn ask() -> Ask<DeleteChoice> {
        let answers = DeleteChoice::ALL.map(|choice| (choice, choice.key()));
        Ask::new(DELETE_QUESTION, answers)
            .focused_on(DeleteChoice::No)
            .cancelled_by(DeleteChoice::No)
    }
}

/// The question asked on backing out of a name box with a name typed
/// ([`KEEP_QUESTION`]): focused on going back, which Cancel answers too.
fn keep_question() -> Ask<LeaveChoice> {
    let answers = LeaveChoice::ALL.map(|choice| (choice, choice.key()));
    Ask::new(KEEP_QUESTION, answers)
        .focused_on(LeaveChoice::Back)
        .cancelled_by(LeaveChoice::Back)
}

/// What a name box is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NamePurpose {
    /// A new profile.
    Add,
    /// A new name for this profile.
    Rename(ProfileId),
}

/// Where the name is typed.
#[derive(Debug, Clone, PartialEq, Eq)]
enum NameInput {
    /// On the keyboard.
    Typing(NameBox),
    /// On the letter grid (a controller).
    Grid(NameEntry),
}

impl NameInput {
    fn name(&self) -> &str {
        match self {
            NameInput::Typing(b) => b.name(),
            NameInput::Grid(e) => e.name(),
        }
    }
}

/// An open name box.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Naming {
    purpose: NamePurpose,
    /// The name it opened with: the profile's, or nothing for a new one.
    original: String,
    input: NameInput,
    /// The question asked on backing out, while it is open.
    leaving: Option<Ask<LeaveChoice>>,
}

impl Naming {
    /// Whether a name is typed that isn't kept: not blank, and not the one
    /// the box opened with.
    fn unkept(&self) -> bool {
        finished(self.input.name()).is_some_and(|name| name != self.original)
    }
}

/// What paints the screen's view: [`glyph::paint`], or the game's own
/// ([`ProfilesScreen::painted_by`]).
pub type Paint<G> = fn(&Ctx<G>, &ProfilesView, &mut GlyphBuffer);

/// What opens once a profile is added and in use, in the screen's place
/// ([`ProfilesScreen::opening_after_add`]).
pub type AfterAdd<G> = fn(&mut Ctx<G>) -> Box<dyn Screen<G>>;

/// The profiles, the one in use marked, and a row to add one. Cancel
/// closes it.
pub struct ProfilesScreen<G: GameData> {
    /// What paints the view.
    paint: Paint<G>,
    /// What opens once a profile is added, if anything.
    after_add: Option<AfterAdd<G>>,
    /// The focused row: a profile's place in the list, or the list's
    /// length for the row that adds one.
    rows: ListCursor,
    /// The focused profile's actions, while open.
    actions: Option<Menu<ProfileAction>>,
    /// The name box, while open.
    naming: Option<Naming>,
    /// The profile asked about and the question, while the question asked
    /// before deleting is open.
    deleting: Option<(ProfileId, Ask<DeleteChoice>)>,
    /// The text key of the message under the panel, if any.
    message: Option<&'static str>,
}

impl<G: GameData> fmt::Debug for ProfilesScreen<G> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // check-text: not player text, the type name for Debug
        f.debug_struct("ProfilesScreen")
            .field("rows", &self.rows)
            .field("actions", &self.actions)
            .field("naming", &self.naming)
            .field("deleting", &self.deleting)
            .field("message", &self.message)
            .finish_non_exhaustive()
    }
}

impl<G: GameData> ProfilesScreen<G> {
    /// Name reported by [`Screen::name`].
    pub const NAME: &'static str = "profiles";

    /// The screen focused on the profile in use.
    pub fn new(ctx: &Ctx<G>) -> Self {
        let profiles = ctx.profiles();
        let current = profiles
            .all()
            .iter()
            .position(|p| p.id == profiles.current());
        Self {
            paint: glyph::paint,
            after_add: None,
            rows: ListCursor::new(Self::rows(ctx).len()).at(current.unwrap_or(0)),
            actions: None,
            naming: None,
            deleting: None,
            message: None,
        }
    }

    /// The same screen with its view painted by `paint` instead of
    /// [`glyph::paint`]: a game's look for a device of its own.
    #[must_use]
    pub fn painted_by(mut self, paint: Paint<G>) -> Self {
        self.paint = paint;
        self
    }

    /// The same screen opening the screen `open` makes in its place once a
    /// profile is added (and in use), instead of closing: the game's
    /// first-launch setup for the new profile, say.
    #[must_use]
    pub fn opening_after_add(mut self, open: AfterAdd<G>) -> Self {
        self.after_add = Some(open);
        self
    }

    /// Every row, top to bottom.
    pub fn rows(ctx: &Ctx<G>) -> Vec<RowKind> {
        let profiles = ctx.profiles().all().iter();
        let mut rows: Vec<RowKind> = profiles.map(|p| RowKind::Profile(p.id)).collect();
        rows.push(RowKind::Add);
        rows
    }

    /// The focused row.
    pub fn focused(&self, ctx: &Ctx<G>) -> RowKind {
        let rows = Self::rows(ctx);
        rows.get(self.rows.focus()).copied().unwrap_or(RowKind::Add)
    }

    /// The focused profile, if a profile row is focused.
    fn focused_profile(&self, ctx: &Ctx<G>) -> Option<ProfileId> {
        match self.focused(ctx) {
            RowKind::Profile(id) => Some(id),
            RowKind::Add => None,
        }
    }

    /// What can be done to `id`: use it unless it is in use, rename it,
    /// delete it unless it is the only one.
    pub fn actions_for(ctx: &Ctx<G>, id: ProfileId) -> Vec<ProfileAction> {
        let profiles = ctx.profiles();
        let mut actions = Vec::new();
        if profiles.current() != id {
            actions.push(ProfileAction::Switch);
        }
        actions.push(ProfileAction::Rename);
        if profiles.all().len() > 1 {
            actions.push(ProfileAction::Delete);
        }
        actions
    }

    /// Whether another profile may be added.
    fn can_add(ctx: &Ctx<G>) -> bool {
        ctx.profiles().all().len() < MAX_PROFILES
    }

    /// What the name box is for, while it is open.
    pub fn naming(&self) -> Option<NamePurpose> {
        self.naming.as_ref().map(|n| n.purpose)
    }

    /// The name typed so far, while the name box is open.
    pub fn typed(&self) -> Option<&str> {
        self.naming.as_ref().map(|n| n.input.name())
    }

    /// The text key of the message under the panel, if any.
    pub fn message(&self) -> Option<&str> {
        self.message
    }

    /// The screen as it is now, as plain data for a skin to paint
    /// ([`glyph::paint`]).
    pub fn view(&self, ctx: &Ctx<G>) -> ProfilesView {
        let profiles = ctx.profiles();
        let can_add = Self::can_add(ctx);
        let focus = self.rows.focus();
        let rows = Self::rows(ctx).into_iter().enumerate().map(|(i, kind)| {
            let focused = i == focus;
            match kind {
                RowKind::Profile(id) => RowView {
                    kind,
                    label: profile_name(ctx, id),
                    current: id == profiles.current(),
                    focused,
                    enabled: true,
                },
                RowKind::Add => RowView {
                    kind,
                    label: ctx.text(ADD).to_owned(),
                    current: false,
                    focused,
                    enabled: can_add,
                },
            }
        });
        let actions = self.actions.as_ref().zip(self.focused_profile(ctx));
        let title = ctx.text(NAME_BOX);
        let (typing, grid) = match self.naming.as_ref().map(|n| &n.input) {
            Some(NameInput::Typing(b)) => (Some(b.view(ctx, title)), None),
            Some(NameInput::Grid(e)) => (None, Some(e.view(ctx, title))),
            None => (None, None),
        };
        let leaving = self.naming.as_ref().and_then(|n| n.leaving.as_ref());
        ProfilesView {
            title: ctx.text(TITLE).to_owned(),
            rows: rows.collect(),
            current_mark: ctx.text(CURRENT).to_owned(),
            actions: actions.map(|(menu, id)| ActionsView {
                title: profile_name(ctx, id),
                menu: menu.view(),
            }),
            typing,
            grid,
            leaving: leaving.map(|ask| ask.view(ctx)),
            deleting: self.deleting.as_ref().map(|(id, ask)| {
                let name = profile_name(ctx, *id);
                ask.view_with_keys(ctx, ctx.help_keys(), &[("name", &name)])
            }),
            message: self.message.map(|key| match key {
                TOO_MANY_MESSAGE => ctx.text_with(key, &[("n", &MAX_PROFILES)]),
                key => ctx.text(key).to_owned(),
            }),
            help: self.help(ctx),
        }
    }

    /// The bottom help line, naming the keys of the active keymap (or the
    /// text box's own keys while typing).
    pub fn help(&self, ctx: &Ctx<G>) -> String {
        let key = match &self.naming {
            Some(n) if n.leaving.is_some() => HELP_QUESTION,
            Some(Naming {
                input: NameInput::Typing(_),
                ..
            }) => return text_keys_help(ctx),
            Some(_) => name_entry::HELP_GRID,
            None if self.deleting.is_some() => HELP_QUESTION,
            None if self.actions.is_some() => HELP_ACTIONS,
            None => match self.focused(ctx) {
                RowKind::Add if Self::can_add(ctx) => HELP_ADD,
                RowKind::Add => HELP_NONE,
                RowKind::Profile(_) => HELP_LIST,
            },
        };
        ctx.text_with(key, &[])
    }

    /// Opens the name box for `purpose`, starting from `name`: the letter
    /// grid if a controller button opened it (`pad`), else the typing box.
    /// It plays nothing: the key that opened it did.
    fn open_name(&mut self, purpose: NamePurpose, name: &str, pad: bool) {
        let input = if pad {
            NameInput::Grid(NameEntry::new(name, NAME_RULES))
        } else {
            NameInput::Typing(NameBox::new(name, NAME_RULES))
        };
        self.naming = Some(Naming {
            purpose,
            original: name.to_owned(),
            input,
            leaving: None,
        });
    }

    /// Confirm on the list (`pad`: with a controller button).
    fn confirm(&mut self, ctx: &mut Ctx<G>, pad: bool) {
        match self.focused(ctx) {
            RowKind::Profile(id) => {
                let items = Self::actions_for(ctx, id)
                    .into_iter()
                    .map(|action| MenuItem::keyed(action.key()).with_value(action));
                self.actions = Some(Menu::new(items.collect()));
                ctx.audio.menu(MenuSound::Select);
            }
            RowKind::Add if Self::can_add(ctx) => {
                self.open_name(NamePurpose::Add, "", pad);
                ctx.audio.menu(MenuSound::Select);
            }
            RowKind::Add => {
                self.message = Some(TOO_MANY_MESSAGE);
                ctx.audio.menu(MenuSound::Denied);
            }
        }
    }

    /// An action while the focused profile's actions are open.
    fn act(&mut self, ctx: &mut Ctx<G>, action: G::Action, pad: bool) -> Transition<G> {
        let id = self.focused_profile(ctx);
        let (Some(menu), Some(id)) = (&mut self.actions, id) else {
            self.actions = None;
            return Transition::None;
        };
        match menu.handle_with_sound(action, &mut ctx.audio) {
            None => {}
            Some(MenuEvent::Cancelled) => self.actions = None,
            Some(MenuEvent::Chosen(chosen)) => {
                self.actions = None;
                match chosen {
                    ProfileAction::Switch => return Self::switch(ctx, id),
                    ProfileAction::Rename => {
                        let name = profile_name(ctx, id);
                        self.open_name(NamePurpose::Rename(id), &name, pad);
                    }
                    ProfileAction::Delete => self.deleting = Some((id, DeleteChoice::ask())),
                }
            }
        }
        Transition::None
    }

    /// Uses the profile `id` and closes the screen: what is under it then
    /// shows its options and saves.
    fn switch(ctx: &mut Ctx<G>, id: ProfileId) -> Transition<G> {
        // The switch happens even if the list can't be saved (the next
        // launch then opens on the profile used before).
        let _ = ctx.switch_profile(id);
        Transition::Pop
    }

    /// The message for `error`, refused by the profiles.
    fn refused(&mut self, ctx: &mut Ctx<G>, error: &ProfileError) {
        self.message = Some(match error {
            ProfileError::NameTaken => NAME_TAKEN_MESSAGE,
            ProfileError::TooMany => TOO_MANY_MESSAGE,
            _ => NOT_SAVED_MESSAGE,
        });
        ctx.audio.menu(MenuSound::Denied);
    }

    /// Keeps the name typed: adds the profile (then uses it and closes, or
    /// opens what the game gives) or renames one. A name the profiles
    /// refuse leaves the box open with a message.
    fn keep_name(&mut self, ctx: &mut Ctx<G>) -> Transition<G> {
        let Some(naming) = &self.naming else {
            return Transition::None;
        };
        let Some(name) = finished(naming.input.name()) else {
            ctx.audio.menu(MenuSound::Denied);
            return Transition::None;
        };
        match naming.purpose {
            NamePurpose::Rename(id) => {
                if let Err(e) = ctx.rename_profile(id, &name) {
                    self.refused(ctx, &e);
                    return Transition::None;
                }
            }
            NamePurpose::Add => match ctx.add_profile(&name) {
                Ok(id) => {
                    self.naming = None;
                    self.message = None;
                    let _ = ctx.switch_profile(id);
                    return match self.after_add {
                        Some(open) => Transition::Replace(open(ctx)),
                        None => Transition::Pop,
                    };
                }
                Err(e) => {
                    self.refused(ctx, &e);
                    return Transition::None;
                }
            },
        }
        self.naming = None;
        self.message = None;
        Transition::None
    }

    /// The name box asked to close (Cancel): with a name typed and not
    /// kept it asks first, focused on going back; otherwise it closes.
    fn cancel_name(&mut self, ctx: &mut Ctx<G>) {
        let Some(naming) = &mut self.naming else {
            return;
        };
        if naming.unkept() {
            naming.leaving = Some(keep_question());
            ctx.audio.menu(MenuSound::Select);
        } else {
            self.naming = None;
            self.message = None;
            ctx.audio.menu(MenuSound::Cancel);
        }
    }

    /// One frame of the name box (no question open).
    fn type_name(&mut self, ctx: &mut Ctx<G>, input: &FrameInput<G::Action>) -> Transition<G> {
        let Some(naming) = &mut self.naming else {
            return Transition::None;
        };
        let event = match &mut naming.input {
            NameInput::Typing(b) => b.handle(input, ctx),
            NameInput::Grid(e) => {
                let mut event = NameEvent::Open;
                for &action in &input.actions {
                    event = e.handle(action, ctx);
                    if event != NameEvent::Open {
                        break;
                    }
                }
                event
            }
        };
        match event {
            NameEvent::Open => Transition::None,
            NameEvent::Done => self.keep_name(ctx),
            NameEvent::Cancel => {
                self.cancel_name(ctx);
                Transition::None
            }
        }
    }

    /// An action while the question asked on backing out of the name box
    /// is open: the cursor moves between the answers, Confirm picks one,
    /// Cancel goes back to the box. Returns the transition and whether the
    /// question closed.
    fn answer_leave(&mut self, ctx: &mut Ctx<G>, action: G::Action) -> (Transition<G>, bool) {
        let Some(ask) = self.naming.as_mut().and_then(|n| n.leaving.as_mut()) else {
            return (Transition::None, true);
        };
        let Some(choice) = ask.handle(action, &mut ctx.audio) else {
            return (Transition::None, false);
        };
        if let Some(naming) = &mut self.naming {
            naming.leaving = None;
        }
        match choice {
            LeaveChoice::Keep => (self.keep_name(ctx), true),
            LeaveChoice::Discard => {
                self.naming = None;
                self.message = None;
                ctx.audio.menu(MenuSound::Cancel);
                (Transition::None, true)
            }
            LeaveChoice::Back => {
                ctx.audio.menu(MenuSound::Cancel);
                (Transition::None, true)
            }
        }
    }

    /// An action while the question asked before deleting is open. Returns
    /// whether the question closed.
    fn answer_delete(&mut self, ctx: &mut Ctx<G>, action: G::Action) -> bool {
        let Some((id, ask)) = &mut self.deleting else {
            return true;
        };
        let id = *id;
        let Some(choice) = ask.handle(action, &mut ctx.audio) else {
            return false;
        };
        self.deleting = None;
        if choice == DeleteChoice::No {
            ctx.audio.menu(MenuSound::Cancel);
            return true;
        }
        match ctx.delete_profile(id) {
            Ok(()) => ctx.audio.menu(MenuSound::Select),
            Err(e) => self.refused(ctx, &e),
        }
        // A row fewer: the focus stays in the list. (The cursor learns the
        // new number of rows on the next move.)
        let profiles = ctx.profiles().all().len();
        self.rows = self
            .rows
            .clone()
            .at(self.rows.focus().min(profiles.saturating_sub(1)));
        true
    }

    /// Moves the focus one row up or down, wrapping.
    fn move_row(&mut self, ctx: &mut Ctx<G>, down: bool) {
        self.rows.set_len(Self::rows(ctx).len());
        if self.rows.step(down, true, |_| true) {
            ctx.audio.menu(MenuSound::Move);
        }
        self.message = None;
    }
}

/// The name the player gave the profile `id` (empty if there is none).
fn profile_name<G: GameData>(ctx: &Ctx<G>, id: ProfileId) -> String {
    let profile = ctx.profiles().get(id);
    // check-text: not a data name (the player typed it)
    profile.map(|p| p.name.clone()).unwrap_or_default()
}

impl<G: GameData> Screen<G> for ProfilesScreen<G> {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    /// Its rows, its menus and questions, and the name grid.
    fn actions(&self) -> Option<Vec<G::Action>> {
        Some(crate::input::ui_actions(crate::input::NAVIGATION))
    }

    fn update(&mut self, ctx: &mut Ctx<G>, input: &FrameInput<G::Action>) -> Transition<G> {
        let typing = self.naming.as_ref().is_some_and(|n| n.leaving.is_none());
        if typing {
            // The game's keys do nothing while the box is open (on the
            // grid they are its cursor).
            return self.type_name(ctx, input);
        }
        for &action in &input.actions {
            if self.naming.is_some() {
                let (transition, closed) = self.answer_leave(ctx, action);
                if closed {
                    // The answer ends this frame's keys.
                    return transition;
                }
                continue;
            }
            if self.deleting.is_some() {
                if self.answer_delete(ctx, action) {
                    break;
                }
                continue;
            }
            if self.actions.is_some() {
                let transition = self.act(ctx, action, input.pad_pressed());
                if !matches!(transition, Transition::None) {
                    return transition;
                }
                if self.actions.is_none() {
                    // The key that chose isn't the next box's.
                    break;
                }
                continue;
            }
            match action.ui() {
                Some(UiAction::Up) => self.move_row(ctx, false),
                Some(UiAction::Down) => self.move_row(ctx, true),
                Some(UiAction::Confirm) => {
                    self.confirm(ctx, input.pad_pressed());
                    // The key that opened something isn't its first.
                    if self.actions.is_some() || self.naming.is_some() {
                        break;
                    }
                }
                Some(UiAction::Cancel) => {
                    ctx.audio.menu(MenuSound::Cancel);
                    return Transition::Pop;
                }
                _ => {}
            }
        }
        Transition::None
    }

    fn draw(&self, ctx: &Ctx<G>, buf: &mut GlyphBuffer) {
        (self.paint)(ctx, &self.view(ctx), buf);
    }

    fn as_any(&self) -> Option<&dyn core::any::Any> {
        Some(self)
    }
}

#[cfg(test)]
mod tests;
