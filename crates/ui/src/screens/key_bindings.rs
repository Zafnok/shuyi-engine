//! The Key bindings screen: every action of the game's rows
//! ([`KeyRow`]) with its [`SLOTS`] key slots for the layout in use, or, on
//! its controller [`Side`], its [`SLOTS`] button slots. A row at the top switches between
//! the two. Confirm on a slot captures the next key or button pressed into
//! it; one taken from another slot moves; leaving is blocked while a
//! required action has no key or no button; Restore defaults asks first.
//!
//! The screen edits a copy of the layout's [`LayoutBindings`] and of the
//! [`PadBindings`] and hands them to [`Ctx`] when it closes, so it is steered
//! with the keys and buttons the player had when they opened it: moving
//! every cursor key elsewhere can't trap them here.
//!
//! No controller button is fixed, so the screen works with only a
//! controller: a button goes in the slot when it is let go, holding any
//! button backs out of a capture, and when the controller was used last
//! Confirm on a slot offers [`CHOICES`] (change it or clear it). A second
//! button pressed while the first is held makes the slot a button chord
//! of the two (ticket 1045, game ADR-0079). A button of the capture still held
//! when it ends does nothing when let go: on a pad whose buttons act when
//! let go (the Playdate's B), it would otherwise back out of the screen.
//!
//! On a platform with no keyboard ([`Keyboard::Absent`], the Playdate) the
//! screen shows the controller side alone: no switch, and leaving asks
//! only that every required action has a button.
//!
//! Capture is the one place that reads raw key presses
//! ([`FrameInput::pressed_chords`]) and raw button presses
//! ([`FrameInput::pressed_buttons`]); which keys back out or clear a slot is
//! asked of `input` ([`is_capture_abort`], [`is_clear_slot`]), and a
//! button's name of [`PadKind::button_name`], never named here.
//!
//! **The look is a skin.** This module decides what the screen
//! shows and does, and says it as plain data: a [`KeyBindingsView`]
//! ([`KeyBindingsScreen::view`]). [`glyph::paint`] draws that view as
//! glyphs, unless the game gives another painter
//! ([`KeyBindingsScreen::painted_by`]). Tests of what happened read the
//! view; only the painters' tests read cells.

pub mod glyph;
pub mod view;

use crate::prelude::*;
pub use view::{
    ChoicesView, GroupView, KeyBindingsView, RestoreView, RowView, SlotView, SwitchView,
};

use core::fmt;

use shuyi_assets::keymap::{ActionSet, Button, ButtonChord, Chord, Layout, SLOTS, UiAction};

use super::layout_picker;
use crate::audio::MenuSound;
use crate::ctx::{CHORD_JOIN_TEXT, Ctx, GameData, Keyboard};
use crate::frame::glyph_buffer::GlyphBuffer;
use crate::input::bindings::{LayoutBindings, PadBindings};
use crate::input::fixed::{
    capture_abort_key_name, clear_slot_key_name, is_capture_abort, is_clear_slot,
};
use crate::input::help_keys::{HelpKeys, SEPARATOR};
use crate::input::keymap::Keymap;
use crate::input::pad::PadKind;
use crate::input::state::Device;
use crate::screen::{FrameInput, Screen, Transition};
use crate::widgets::{Ask, AskView};

/// What a row of the Key bindings screen sits under.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowGroup {
    /// The actions that must keep a key (or a button), under the screen's
    /// own heading for them.
    Required,
    /// The other actions of the main key group.
    Optional,
    /// A key group of its own, whose keys may overlap the rest's
    /// (game ADR-0068), under the heading with this text key.
    Shared(&'static str),
}

/// One row of the Key bindings screen, as the game gives it: the action,
/// the text key of its player-facing label (`Unit info`) and what it sits
/// under. Rows of one group come together, in the order the screen lists
/// them.
pub type KeyRow<A> = (A, &'static str, RowGroup);

/// Which bindings the screen shows and edits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    /// The keys of the layout in use.
    Keyboard,
    /// The controller's buttons (one setup for both layouts).
    Controller,
}

impl Side {
    /// The text key of what a slot on this side shows while it waits.
    fn prompt(self) -> &'static str {
        match self {
            Self::Keyboard => CAPTURE_PROMPT,
            Self::Controller => CAPTURE_BUTTON_PROMPT,
        }
    }
}

/// The text key of what a slot shows while it waits for a key.
pub const CAPTURE_PROMPT: &str = "key_bindings.press_a_key";

/// The text key of what a slot shows while it waits for a controller
/// button.
pub const CAPTURE_BUTTON_PROMPT: &str = "key_bindings.press_a_button";

/// Text key of the screen's title.
pub const TITLE: &str = "key_bindings.title";
/// Text key of the switch row's name for the keyboard side, which the
/// layout's name follows.
pub const KEYBOARD_SIDE: &str = "key_bindings.keyboard";
/// Text key of the switch row's name for the controller side.
pub const CONTROLLER_SIDE: &str = "key_bindings.controller";
/// Text key of the heading over the required actions, on the keyboard
/// side.
pub const REQUIRED_HEADING: &str = "key_bindings.must_have_key";
/// Text key of the heading over the required actions, on the controller
/// side.
pub const REQUIRED_BUTTON_HEADING: &str = "key_bindings.must_have_button";
/// Text key of the heading over the optional actions.
pub const OPTIONAL_HEADING: &str = "key_bindings.optional";
/// Text key of the row under the actions that puts the shown side's
/// defaults back.
pub const RESTORE_DEFAULTS: &str = "key_bindings.restore_defaults";
/// Text key of the help while a slot waits for a key.
pub const CAPTURE_HELP: &str = "key_bindings.capture_help";
/// Text key of the help for backing out of a capture with only a
/// controller.
pub const HOLD_TO_CANCEL: &str = "key_bindings.hold_to_cancel";
/// Text keys of what Confirm on a slot offers when the controller was used
/// last: put another key or button in it, or empty it.
pub const CHOICES: [&str; 2] = ["key_bindings.change", "key_bindings.clear"];
/// Text key of what shows when the key pressed for a slot is reserved.
pub const RESERVED_MESSAGE: &str = "key_bindings.reserved";
/// How long the row that just lost its key to another slot stays
/// highlighted, in seconds (*tunable*).
pub const MOVED_FLASH_SECS: f32 = 1.5;
/// How long a controller button is held to back out of a capture, in
/// seconds (*tunable*). Let go sooner, it goes in the slot.
pub const HOLD_TO_CANCEL_SECS: f32 = 1.0;

/// What paints the screen's view: [`glyph::paint`], or the game's own
/// ([`KeyBindingsScreen::painted_by`]).
pub type Paint<G> = fn(&Ctx<G>, &KeyBindingsView<<G as GameData>::Action>, &mut GlyphBuffer);

/// The controller buttons being pressed for a slot: the first to go down,
/// for how many seconds it has been held, and the second, if one went down
/// while it was held (a chord).
#[derive(Debug, Clone, Copy, PartialEq)]
struct Held {
    first: Button,
    secs: f32,
    second: Option<Button>,
}

impl Held {
    /// What the buttons make: the first alone, or the chord of the two.
    fn chord(self) -> Option<ButtonChord> {
        match self.second {
            Some(second) => ButtonChord::pair(self.first, second),
            None => Some(ButtonChord::single(self.first)),
        }
    }

    /// Whether `button` is one of them.
    fn has(self, button: Button) -> bool {
        self.first == button || self.second == Some(button)
    }
}

/// Whether `action` is one the game takes for a move (up, down, left,
/// right): what moves the focus.
fn is_cursor<A: ActionSet>(action: A) -> bool {
    matches!(
        action.ui(),
        Some(UiAction::Up | UiAction::Down | UiAction::Left | UiAction::Right)
    )
}

/// The player-facing name of `action` (its row's label) in `ctx`'s
/// language; its keymap name for an action not on the screen (Debug).
pub fn label<'a, G: GameData>(
    ctx: &'a Ctx<G>,
    rows: &[KeyRow<G::Action>],
    action: G::Action,
) -> &'a str {
    rows.iter()
        .find(|&&(a, ..)| a == action)
        .map_or(action.name(), |&(_, key, _)| ctx.text(key))
}

/// The blocked-leave message for `action` on `side` (`Give Cancel a key
/// first`, `Give Cancel a button first`).
pub fn blocked_message<G: GameData>(
    ctx: &Ctx<G>,
    rows: &[KeyRow<G::Action>],
    side: Side,
    action: G::Action,
) -> String {
    let key = match side {
        Side::Keyboard => "key_bindings.blocked_key",
        Side::Controller => "key_bindings.blocked_button",
    };
    ctx.text_with(key, &[("action", &label(ctx, rows, action))])
}

/// The Key bindings screen for one layout and the controller. Cancel saves
/// and closes it, unless a required action has no key or no button.
pub struct KeyBindingsScreen<G: GameData> {
    /// The game's rows, in the order the screen lists them.
    rows: &'static [KeyRow<G::Action>],
    /// What paints the view.
    paint: Paint<G>,
    /// Whether the platform has a keyboard: without one only the
    /// controller side is shown.
    keyboard: Keyboard,
    /// The layout being edited.
    layout: Layout,
    /// The keymap the screen was opened with: what steers it (`Game` keeps
    /// using it until the edits are saved on leaving) and what its help
    /// text names.
    opened_with: Keymap<G::Action>,
    /// Which bindings are shown.
    side: Side,
    /// The edited copy of the layout's keys, saved on leaving.
    bindings: LayoutBindings<G::Action>,
    /// The edited copy of the controller's buttons, saved on leaving.
    pad: PadBindings<G::Action>,
    /// Focused row: an index into `rows`, then Restore defaults
    /// ([`restore_row`](Self::restore_row)) and the switch
    /// ([`switch_row`](Self::switch_row)).
    row: usize,
    /// Focused slot (kept while on a row that has none).
    slot: usize,
    /// Whether the focused slot is waiting for a key or button.
    capturing: bool,
    /// The controller buttons held since they went down during a capture.
    held: Option<Held>,
    /// Buttons of a capture that ended while they were held: the screen
    /// does nothing until they are let go.
    releasing: Vec<Button>,
    /// The focused line of [`CHOICES`], while they are open.
    choice: Option<usize>,
    /// The message under the panel, if any.
    message: Option<String>,
    /// The action that just lost a key to another slot, and the seconds its
    /// row stays highlighted.
    moved: Option<(G::Action, f32)>,
    /// The "restore the defaults?" question, while it is open.
    restoring: Option<Ask<bool>>,
    /// Set when a capture ends with a cursor key or button held: cursor
    /// moves are ignored until none is, because the one just pressed for
    /// the slot (or held to back out) may be one under the opened-with
    /// keymap, still down and repeating.
    await_release: bool,
}

impl<G: GameData> fmt::Debug for KeyBindingsScreen<G> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("KeyBindingsScreen")
            .field("layout", &self.layout)
            .field("side", &self.side)
            .field("row", &self.row)
            .field("slot", &self.slot)
            .field("capturing", &self.capturing)
            .field("choice", &self.choice)
            .field("restoring", &self.restoring.as_ref().map(Ask::key))
            .finish_non_exhaustive()
    }
}

impl<G: GameData> Clone for KeyBindingsScreen<G> {
    fn clone(&self) -> Self {
        Self {
            rows: self.rows,
            paint: self.paint,
            keyboard: self.keyboard,
            layout: self.layout,
            opened_with: self.opened_with.clone(),
            side: self.side,
            bindings: self.bindings.clone(),
            pad: self.pad.clone(),
            row: self.row,
            slot: self.slot,
            capturing: self.capturing,
            held: self.held,
            releasing: self.releasing.clone(),
            choice: self.choice,
            message: self.message.clone(),
            moved: self.moved,
            restoring: self.restoring.clone(),
            await_release: self.await_release,
        }
    }
}

impl<G: GameData> KeyBindingsScreen<G> {
    /// Name reported by [`Screen::name`].
    pub const NAME: &'static str = "key_bindings";

    /// The screen listing `rows`, for the layout in use (the keymap file's
    /// first if none is chosen yet), focused on the first action's first
    /// slot, showing the side the player pressed last: the controller's
    /// buttons after a button press, else the keys. With no keyboard
    /// ([`Keyboard::Absent`]) it shows the controller's alone.
    pub fn new(ctx: &Ctx<G>, rows: &'static [KeyRow<G::Action>]) -> Self {
        // The loader refuses a keymap without layouts.
        let layout = ctx.shown_layout().unwrap_or(Layout::named("none"));
        let side = match ctx.device {
            _ if ctx.keyboard == Keyboard::Absent => Side::Controller,
            Device::Keyboard => Side::Keyboard,
            Device::Pad(_) => Side::Controller,
        };
        Self {
            rows,
            paint: glyph::paint,
            keyboard: ctx.keyboard,
            layout,
            opened_with: ctx.keymap.clone(),
            side,
            bindings: ctx.layout_bindings(layout),
            pad: ctx.pad_bindings(),
            row: 0,
            slot: 0,
            capturing: false,
            held: None,
            releasing: Vec::new(),
            choice: None,
            message: None,
            moved: None,
            restoring: None,
            await_release: false,
        }
    }

    /// The same screen with its view painted by `paint` instead of
    /// [`glyph::paint`]: a game's look for a device of its own.
    #[must_use]
    pub fn painted_by(mut self, paint: Paint<G>) -> Self {
        self.paint = paint;
        self
    }

    /// Whether only the controller side is shown: the platform has no
    /// keyboard, so there is no switch.
    pub fn is_controller_only(&self) -> bool {
        self.keyboard == Keyboard::Absent
    }

    /// The same screen editing `bindings` instead of the saved ones.
    #[must_use]
    pub fn with_bindings(mut self, bindings: LayoutBindings<G::Action>) -> Self {
        self.bindings = bindings;
        self
    }

    /// The edited keys (saved when the screen closes).
    pub fn bindings(&self) -> &LayoutBindings<G::Action> {
        &self.bindings
    }

    /// The edited controller buttons (saved when the screen closes).
    pub fn pad_bindings(&self) -> &PadBindings<G::Action> {
        &self.pad
    }

    /// Which bindings are shown.
    pub fn side(&self) -> Side {
        self.side
    }

    /// The focused action and slot; `None` on Restore defaults and on the
    /// keyboard / controller switch.
    pub fn focus(&self) -> Option<(G::Action, usize)> {
        self.rows
            .get(self.row)
            .map(|&(action, ..)| (action, self.slot))
    }

    /// Whether the focus is on the keyboard / controller switch.
    pub fn is_on_switch(&self) -> bool {
        !self.is_controller_only() && self.row == self.switch_row()
    }

    /// [`row`](Self::row) on Restore defaults, the row after the actions.
    fn restore_row(&self) -> usize {
        self.rows.len()
    }

    /// [`row`](Self::row) on the keyboard / controller switch: above the
    /// first action, so one up from it and one down from Restore defaults.
    fn switch_row(&self) -> usize {
        self.rows.len() + 1
    }

    /// Whether the focused slot is waiting for a key or button.
    pub fn is_capturing(&self) -> bool {
        self.capturing
    }

    /// The focused line of [`CHOICES`], while they are open.
    pub fn choice(&self) -> Option<usize> {
        self.choice
    }

    /// Whether the screen is asking before it restores the defaults.
    pub fn is_asking_restore(&self) -> bool {
        self.restoring.is_some()
    }

    /// The question asked before restoring the shown side's defaults.
    pub fn restore_question(&self, ctx: &Ctx<G>) -> String {
        Self::restore_view(ctx, &self.restore_ask(), self.layout, ctx.help_keys()).text
    }

    /// The question asked before restoring the shown side's defaults, as
    /// it opens: yes on Confirm, no on Cancel.
    fn restore_ask(&self) -> Ask<bool> {
        let key = match self.side {
            Side::Keyboard => "key_bindings.restore_keys",
            Side::Controller => "key_bindings.restore_buttons",
        };
        Ask::yes_no(key, "key_bindings.restore_answers")
    }

    /// `ask`'s view, the keyboard's question naming `layout` and its
    /// answers' keys those of `keys`.
    fn restore_view(
        ctx: &Ctx<G>,
        ask: &Ask<bool>,
        layout: Layout,
        keys: HelpKeys<'_, G::Action>,
    ) -> AskView<bool> {
        let layout = layout_picker::label(ctx, layout);
        ask.view_with_keys(ctx, keys, &[("layout", &layout)])
    }

    /// The message under the panel, if any.
    pub fn message(&self) -> Option<&str> {
        self.message.as_deref()
    }

    /// The names in `action`'s slots on the shown side; buttons as the
    /// controller used last names them (an Xbox pad's names until one is
    /// used).
    fn slot_names(&self, ctx: &Ctx<G>, action: G::Action) -> [Option<String>; SLOTS] {
        match self.side {
            Side::Keyboard => self
                .bindings
                .slots(action)
                .map(|c| c.map(|c| c.to_string())),
            Side::Controller => {
                let kind = match ctx.device {
                    Device::Pad(kind) => kind,
                    Device::Keyboard => PadKind::default(),
                };
                let join = ctx.text(CHORD_JOIN_TEXT);
                let name = |b| kind.chord_name(b, join);
                self.pad.slots(action).map(|b| b.map(name))
            }
        }
    }

    /// Whether `action` has nothing in any slot on the shown side.
    fn is_unmapped(&self, action: G::Action) -> bool {
        match self.side {
            Side::Keyboard => self.bindings.is_unmapped(action),
            Side::Controller => self.pad.is_unmapped(action),
        }
    }

    /// The first row of the screen whose required action has nothing on
    /// `side`.
    fn first_unmapped(&self, side: Side) -> Option<G::Action> {
        let unmapped = match side {
            Side::Keyboard => self.bindings.unmapped_required(),
            Side::Controller => self.pad.unmapped_required(),
        };
        let mut rows = self.rows.iter().map(|&(action, ..)| action);
        rows.find(|action| unmapped.contains(action))
    }

    /// Counts the moved-row highlight down.
    fn tick(&mut self, dt: f32) {
        if let Some((_, left)) = &mut self.moved {
            *left -= dt;
            // A NaN frame time ends the highlight too.
            if left.is_nan() || *left <= 0.0 {
                self.moved = None;
            }
        }
    }

    /// Moves the focus one row up or down, wrapping: the actions, Restore
    /// defaults, the switch (unless there is none), and the first action
    /// again.
    fn move_row(&mut self, ctx: &mut Ctx<G>, down: bool) {
        let rows = self.rows.len() + if self.is_controller_only() { 1 } else { 2 };
        self.row = if down {
            (self.row + 1) % rows
        } else {
            (self.row + rows - 1) % rows
        };
        self.message = None;
        ctx.audio.menu(MenuSound::Move);
    }

    /// Left or right: on the switch, shows the keyboard (left) or the
    /// controller (right); on a slot, moves the focus one slot, stopping at
    /// the ends. Does nothing on Restore defaults.
    fn move_across(&mut self, ctx: &mut Ctx<G>, right: bool) {
        if self.is_on_switch() {
            let side = if right {
                Side::Controller
            } else {
                Side::Keyboard
            };
            self.show(ctx, side);
            return;
        }
        if self.focus().is_none() {
            return;
        }
        let to = if right {
            (self.slot + 1).min(SLOTS - 1)
        } else {
            self.slot.saturating_sub(1)
        };
        if to != self.slot {
            self.slot = to;
            self.message = None;
            ctx.audio.menu(MenuSound::Move);
        }
    }

    /// Shows `side`, if it isn't shown already.
    fn show(&mut self, ctx: &mut Ctx<G>, side: Side) {
        if side != self.side {
            self.side = side;
            self.message = None;
            self.moved = None;
            ctx.audio.menu(MenuSound::Move);
        }
    }

    /// Confirm: on a slot, starts capturing (after the controller was used,
    /// opens the [`CHOICES`] instead); on Restore defaults, asks whether to
    /// restore; on the switch, shows the other side.
    fn confirm(&mut self, ctx: &mut Ctx<G>) {
        self.message = None;
        if self.is_on_switch() {
            let other = match self.side {
                Side::Keyboard => Side::Controller,
                Side::Controller => Side::Keyboard,
            };
            self.show(ctx, other);
            return;
        }
        if self.focus().is_none() {
            self.restoring = Some(self.restore_ask());
        } else if matches!(ctx.device, Device::Pad(_)) {
            self.choice = Some(0);
        } else {
            self.capturing = true;
        }
        ctx.audio.menu(MenuSound::Select);
    }

    /// An action while the [`CHOICES`] are open: up and down move between
    /// them, Confirm takes the focused one (capture, or empty the slot) and
    /// Cancel closes them. Returns whether they closed.
    fn answer_choice(&mut self, ctx: &mut Ctx<G>, action: G::Action) -> bool {
        let Some(line) = self.choice else {
            return true;
        };
        match action.ui() {
            Some(UiAction::Up | UiAction::Down) => {
                self.choice = Some((line + 1) % CHOICES.len());
                ctx.audio.menu(MenuSound::Move);
                return false;
            }
            Some(UiAction::Confirm) if line == 0 => {
                self.capturing = true;
                ctx.audio.menu(MenuSound::Select);
            }
            Some(UiAction::Confirm) => {
                if !self.clear_slot(ctx) {
                    ctx.audio.menu(MenuSound::Select);
                }
            }
            Some(UiAction::Cancel) => ctx.audio.menu(MenuSound::Cancel),
            _ => return false,
        }
        self.choice = None;
        true
    }

    /// An action while the restore question is open: Confirm restores the
    /// shown side's defaults, Cancel backs out; either closes the question
    /// (and returns `true`). Nothing else does anything.
    fn answer_restore(&mut self, ctx: &mut Ctx<G>, action: G::Action) -> bool {
        let Some(ask) = &mut self.restoring else {
            return true;
        };
        match ask.handle(action, &mut ctx.audio) {
            Some(true) => {
                let def = ctx.game.keymap_def();
                match self.side {
                    Side::Keyboard => self.bindings = LayoutBindings::defaults(def, self.layout),
                    Side::Controller => self.pad = PadBindings::defaults(def),
                }
                self.moved = None;
                ctx.audio.menu(MenuSound::Select);
            }
            Some(false) => ctx.audio.menu(MenuSound::Cancel),
            None => return false,
        }
        self.restoring = None;
        true
    }

    /// Empties the focused slot on the shown side. Returns whether it held
    /// anything.
    fn clear_slot(&mut self, ctx: &mut Ctx<G>) -> bool {
        let Some((action, slot)) = self.focus() else {
            return false;
        };
        if self.slot_names(ctx, action)[slot].is_none() {
            return false;
        }
        match self.side {
            Side::Keyboard => self.bindings.clear(action, slot),
            Side::Controller => self.pad.clear(action, slot),
        }
        self.message = None;
        ctx.audio.menu(MenuSound::Select);
        true
    }

    /// Ends the capture with nothing changed.
    fn abort_capture(&mut self, ctx: &mut Ctx<G>) {
        self.end_capture();
        ctx.audio.menu(MenuSound::Cancel);
    }

    /// Ends the capture, clearing what belongs to it.
    fn end_capture(&mut self) {
        self.capturing = false;
        self.held = None;
        self.message = None;
    }

    /// The capture took something that was in slot `from`, if any: ends it
    /// and lights up the action that lost it.
    fn captured(&mut self, ctx: &mut Ctx<G>, from: Option<(G::Action, usize)>) {
        self.end_capture();
        self.moved = from.map(|(lost, _)| (lost, MOVED_FLASH_SECS));
        ctx.audio.menu(MenuSound::Select);
    }

    /// A key pressed while capturing. Returns whether capturing ended.
    fn capture(&mut self, ctx: &mut Ctx<G>, chord: Chord) -> bool {
        if is_clear_slot(chord) {
            return false;
        }
        if is_capture_abort(chord) {
            self.abort_capture(ctx);
            return true;
        }
        let Some((action, slot)) = self.focus() else {
            self.end_capture();
            return true;
        };
        if self.side == Side::Controller {
            // A slot waiting for a button takes no key.
            return false;
        }
        let Ok(from) = self.bindings.bind(action, slot, chord) else {
            self.message = Some(ctx.text(RESERVED_MESSAGE).to_owned());
            ctx.audio.menu(MenuSound::Denied);
            return false;
        };
        self.captured(ctx, from);
        true
    }

    /// This frame's controller buttons while capturing: the first button
    /// to go down is watched, and a second that goes down while it is held.
    /// When either is let go, the first alone, or the chord of the two,
    /// goes in a slot that waits for a button. The first held alone for
    /// [`HOLD_TO_CANCEL_SECS`] backs out of the capture with nothing
    /// changed.
    fn capture_buttons(&mut self, ctx: &mut Ctx<G>, input: &FrameInput<G::Action>) {
        let fresh = self.held.is_none();
        let mut pressed = input.pressed_buttons().iter().copied();
        if fresh {
            self.held = pressed.next().map(|first| Held {
                first,
                secs: 0.0,
                second: None,
            });
        }
        let Some(mut held) = self.held else {
            return;
        };
        if held.second.is_none() {
            held.second = pressed.find(|&b| b != held.first);
        }
        let released = input.released_buttons();
        if released.iter().any(|&b| held.has(b)) {
            self.held = None;
            let still_down = [Some(held.first), held.second].into_iter().flatten();
            self.releasing = still_down.filter(|b| !released.contains(b)).collect();
            if let Some(chord) = held.chord() {
                self.tapped(ctx, chord);
            }
            return;
        }
        // The frame it went down in doesn't count; nor does a NaN frame.
        if !fresh {
            held.secs += input.dt.max(0.0);
        }
        self.held = Some(held);
        if held.second.is_none() && held.secs >= HOLD_TO_CANCEL_SECS {
            self.abort_capture(ctx);
            self.releasing = vec![held.first];
        }
    }

    /// `chord` (a button, or two) was pressed and let go while capturing:
    /// on the controller side it goes in the focused slot.
    fn tapped(&mut self, ctx: &mut Ctx<G>, chord: ButtonChord) {
        let (Side::Controller, Some((action, slot))) = (self.side, self.focus()) else {
            return;
        };
        // The focused action is rebindable and its slot exists.
        if let Ok(from) = self.pad.bind(action, slot, chord) {
            self.captured(ctx, from);
        }
    }

    /// Cancel: saves and returns `true` if every required action has a key
    /// and a button (only a button, with no keyboard); otherwise shows the
    /// side that lacks one and names the first action on the screen that
    /// does (the shown side first).
    fn leave(&mut self, ctx: &mut Ctx<G>) -> bool {
        let other = match self.side {
            Side::Keyboard => Side::Controller,
            Side::Controller => Side::Keyboard,
        };
        let sides = if self.is_controller_only() {
            &[Side::Controller][..]
        } else {
            &[self.side, other][..]
        };
        let blocked = sides
            .iter()
            .find_map(|&side| Some((side, self.first_unmapped(side)?)));
        if let Some((side, action)) = blocked {
            self.side = side;
            self.message = Some(blocked_message(ctx, self.rows, side, action));
            ctx.audio.menu(MenuSound::Denied);
            return false;
        }
        // If saving fails the keys and buttons still apply for this session.
        if self.bindings != ctx.layout_bindings(self.layout) {
            let _ = ctx.set_layout_bindings(self.layout, self.bindings.clone());
        }
        if self.pad != ctx.pad_bindings() {
            let _ = ctx.set_pad_bindings(self.pad.clone());
        }
        ctx.audio.menu(MenuSound::Cancel);
        true
    }

    /// The bottom help line, naming the keys the screen was opened with
    /// (or their buttons, on a controller): the same as `ctx`'s until the
    /// screen closes.
    pub fn help(&self, ctx: &Ctx<G>) -> String {
        let km = ctx.help_keys().of(&self.opened_with);
        let on_pad = matches!(ctx.device, Device::Pad(_));
        if self.capturing {
            let what = ctx.text(match self.side {
                Side::Keyboard => CAPTURE_HELP,
                Side::Controller => CAPTURE_BUTTON_PROMPT,
            });
            let hold = on_pad || self.side == Side::Controller;
            let hold = hold.then(|| ctx.text(HOLD_TO_CANCEL));
            let back = (!on_pad).then(|| {
                let key = capture_abort_key_name();
                ctx.text_with("key_bindings.help_capture_back", &[("key", &key)])
            });
            let hints = [Some(what), hold, back.as_deref()];
            return hints
                .into_iter()
                .flatten()
                .collect::<Vec<_>>()
                .join(SEPARATOR);
        }
        if self.restoring.is_some() {
            return ctx.text_with_keys("key_bindings.help_restore", km, &[]);
        }
        if self.choice.is_some() {
            return ctx.text_with("key_bindings.help_choice", &[]);
        }
        if self.is_on_switch() {
            return ctx.text_with("key_bindings.help_switch", &[]);
        }
        let on_slot = self.focus().is_some();
        if on_slot && on_pad {
            return ctx.text_with("key_bindings.help_slot_pad", &[]);
        }
        if on_slot {
            let clear = clear_slot_key_name();
            return ctx.text_with_keys("key_bindings.help_slot", km, &[("clear", &clear)]);
        }
        ctx.text_with_keys("key_bindings.help_restore_row", km, &[])
    }

    /// The screen as it is now, as plain data for a skin to paint
    /// ([`glyph::paint`]): the switch, the slot columns, every action with
    /// what its slots hold, the restore row, the message, the question or
    /// the choices open, and the help line, all in the player's language
    /// and naming their keys.
    pub fn view(&self, ctx: &Ctx<G>) -> KeyBindingsView<G::Action> {
        let columns = (0..SLOTS).map(|j| match self.side {
            Side::Keyboard => ctx.text_with("key_bindings.key_column", &[("n", &(j + 1))]),
            Side::Controller => ctx.text_with("key_bindings.button_column", &[("n", &(j + 1))]),
        });
        let mut groups: Vec<GroupView<G::Action>> = Vec::new();
        let mut last = None;
        for (i, &(_, _, group)) in self.rows.iter().enumerate() {
            if last != Some(group) {
                last = Some(group);
                groups.push(GroupView {
                    heading: self.group_heading(ctx, group),
                    required: group == RowGroup::Required,
                    rows: Vec::new(),
                });
            }
            if let Some(group) = groups.last_mut() {
                group.rows.push(self.row_view(ctx, i));
            }
        }
        let km = ctx.help_keys().of(&self.opened_with);
        KeyBindingsView {
            title: ctx.text(TITLE).to_owned(),
            switch: (!self.is_controller_only()).then(|| SwitchView {
                keyboard: format!(
                    "{}{SEPARATOR}{}",
                    ctx.text(KEYBOARD_SIDE),
                    layout_picker::label(ctx, self.layout)
                ),
                controller: ctx.text(CONTROLLER_SIDE).to_owned(),
                shown: self.side,
                focused: self.is_on_switch(),
            }),
            columns: columns.collect(),
            groups,
            restore: RestoreView {
                label: ctx.text(RESTORE_DEFAULTS).to_owned(),
                focused: self.row == self.restore_row(),
            },
            not_mapped: ctx.help_keys().not_mapped().to_owned(),
            message: self.message.clone(),
            question: self
                .restoring
                .as_ref()
                .map(|ask| Self::restore_view(ctx, ask, self.layout, km)),
            choices: self.choice.map(|focused| ChoicesView {
                lines: CHOICES.map(|key| ctx.text(key).to_owned()).to_vec(),
                focused,
                slot: self.slot,
            }),
            help: self.help(ctx),
        }
    }

    /// The heading over the rows of `group`.
    fn group_heading(&self, ctx: &Ctx<G>, group: RowGroup) -> String {
        let key = match (group, self.side) {
            (RowGroup::Shared(heading), _) => heading,
            (RowGroup::Required, Side::Keyboard) => REQUIRED_HEADING,
            (RowGroup::Required, Side::Controller) => REQUIRED_BUTTON_HEADING,
            (RowGroup::Optional, _) => OPTIONAL_HEADING,
        };
        ctx.text(key).to_owned()
    }

    /// Row `i` of `rows`, as the screen shows it.
    fn row_view(&self, ctx: &Ctx<G>, i: usize) -> RowView<G::Action> {
        let (action, label, _) = self.rows[i];
        let label = ctx.text(label);
        let focused = self.row == i;
        let fixed = match self.side {
            Side::Keyboard => Keymap::fixed_chords_for(action)
                .iter()
                .map(ToString::to_string)
                .collect(),
            Side::Controller => Vec::new(),
        };
        let slots = self.slot_names(ctx, action).into_iter().enumerate();
        let slots = slots.map(|(j, name)| match name {
            _ if focused && self.slot == j && self.capturing => {
                SlotView::Capturing(ctx.text(self.side.prompt()).to_owned())
            }
            Some(name) => SlotView::Bound(name),
            None => SlotView::Empty,
        });
        let unmapped = self.is_unmapped(action);
        RowView {
            action,
            label: label.to_owned(),
            fixed,
            slots: slots.collect(),
            focus: focused.then_some(self.slot),
            unmapped,
            blocks_leaving: unmapped && action.is_required(),
            lost_key: self.moved.is_some_and(|(lost, _)| lost == action),
        }
    }
}

impl<G: GameData> Screen<G> for KeyBindingsScreen<G> {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn update(&mut self, ctx: &mut Ctx<G>, input: &FrameInput<G::Action>) -> Transition<G> {
        self.tick(input.dt);
        if !self.releasing.is_empty() {
            // What the capture's buttons do when let go isn't for here.
            let up = input.released_buttons();
            self.releasing.retain(|b| !up.contains(b));
            self.await_release &= <G::Action as ActionSet>::ALL
                .iter()
                .any(|&a| is_cursor(a) && input.is_held(a));
            return Transition::None;
        }
        if self.capturing {
            // This frame's actions and repeats are ignored: the key or
            // button is for the slot.
            if !input.pressed_chords().iter().any(|&c| self.capture(ctx, c)) {
                self.capture_buttons(ctx, input);
            }
            if !self.capturing {
                self.await_release = <G::Action as ActionSet>::ALL
                    .iter()
                    .any(|&a| is_cursor(a) && input.is_held(a));
            }
            return Transition::None;
        }
        if self.restoring.is_some() {
            for &action in &input.actions {
                if self.answer_restore(ctx, action) {
                    break;
                }
            }
            return Transition::None;
        }
        if self.choice.is_some() {
            for &action in &input.actions {
                if self.answer_choice(ctx, action) {
                    break;
                }
            }
            return Transition::None;
        }
        if input.pressed_chords().iter().any(|&c| is_clear_slot(c)) {
            self.clear_slot(ctx);
        }
        self.await_release &= <G::Action as ActionSet>::ALL
            .iter()
            .any(|&a| is_cursor(a) && input.is_held(a));
        for &action in &input.actions {
            match action.ui() {
                _ if self.await_release && is_cursor(action) => {}
                Some(UiAction::Up) => self.move_row(ctx, false),
                Some(UiAction::Down) => self.move_row(ctx, true),
                Some(UiAction::Left) => self.move_across(ctx, false),
                Some(UiAction::Right) => self.move_across(ctx, true),
                Some(UiAction::Confirm) => {
                    self.confirm(ctx);
                    if self.capturing || self.restoring.is_some() || self.choice.is_some() {
                        // The key that confirmed isn't the key to bind (or
                        // the answer), and nothing after it this frame
                        // counts.
                        break;
                    }
                }
                Some(UiAction::Cancel) if self.leave(ctx) => return Transition::Pop,
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
