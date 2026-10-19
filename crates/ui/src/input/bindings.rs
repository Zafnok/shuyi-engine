//! The player's key and button bindings (tickets 0217 and 0816, ADR-0031,
//! ADR-0053): every rebindable action has [`SLOTS`] key slots and [`SLOTS`]
//! controller-button slots. Each layout keeps its own keys
//! ([`LayoutBindings`]); the buttons are one setup shared by both layouts
//! ([`PadBindings`]); the lot is saved as [`PlayerKeys`]. Rules from
//! `docs/design/controls.md`, *Rebinding keys* and *Rebinding buttons*.
//! Pure: the game reads and writes the saved text. Generic over the game's
//! [`ActionSet`] (ticket 1106).
//!
//! Invariants: a chord is in at most one slot of a [`LayoutBindings`]'s
//! actions of each [group](ActionSet::Group), and no
//! [reserved](LayoutBindings::is_reserved) chord is in any; a button is in
//! at most one slot of a [`PadBindings`]'s actions of each group. Actions
//! of different groups may share one (`docs/design/controls.md`, *Dialogue
//! log*).

use crate::prelude::*;
use alloc::collections::{BTreeMap, BTreeSet};
use core::fmt;

use serde::{Deserialize, Serialize};

use shuyi_assets::keymap::KeymapDef;
use shuyi_assets::keymap::{ActionSet, ButtonChord, Chord, Layout, RepeatDef, SLOTS};

use super::keymap::Keymap;
use crate::DEBUG_TOOLS;

/// One action's key slots; `None` is an empty slot.
pub type Slots = [Option<Chord>; SLOTS];

/// One action's controller-button slots, each a button or a
/// [button chord](ButtonChord); `None` is an empty slot.
pub type ButtonSlots = [Option<ButtonChord>; SLOTS];

/// The saved form of a set of slots: action name → its slots' names.
type Saved = BTreeMap<String, Vec<Option<String>>>;

/// Every rebindable action's slots of keys or of buttons, with the rules
/// both share: a thing is in at most one slot of each group's actions, and binding one that is already in a slot of that group moves
/// it.
#[derive(Debug, Clone, PartialEq, Eq)]
struct SlotTable<A, T> {
    slots: BTreeMap<A, [Option<T>; SLOTS]>,
}

impl<A: ActionSet, T: Copy + Eq> SlotTable<A, T> {
    /// Every rebindable action, with empty slots.
    fn empty() -> Self {
        Self {
            slots: A::ALL
                .iter()
                .copied()
                .filter(|a| a.is_rebindable())
                .map(|a| (a, [None; SLOTS]))
                .collect(),
        }
    }

    /// `action`'s slots (all empty for an action that has none).
    fn get(&self, action: A) -> [Option<T>; SLOTS] {
        self.slots.get(&action).copied().unwrap_or([None; SLOTS])
    }

    /// The slot of one of `group`'s actions holding `item`, if any.
    fn find(&self, group: A::Group, item: T) -> Option<(A, usize)> {
        let in_group = self.slots.iter().filter(|(a, _)| a.group() == group);
        in_group.into_iter().find_map(|(&action, slots)| {
            slots
                .iter()
                .position(|&c| c == Some(item))
                .map(|i| (action, i))
        })
    }

    /// Whether `action` has a slot `i`.
    fn check(action: A, i: usize) -> Result<(), BindError<A>> {
        if !action.is_rebindable() {
            return Err(BindError::NotRebindable(action));
        }
        if i >= SLOTS {
            return Err(BindError::NoSuchSlot(i));
        }
        Ok(())
    }

    /// Puts `item` in `action`'s slot `i`, replacing what was there. If it
    /// was in another slot of `action`'s group, that slot is emptied and
    /// returned.
    fn place(&mut self, action: A, i: usize, item: T) -> Result<Option<(A, usize)>, BindError<A>> {
        Self::check(action, i)?;
        let from = self.find(action.group(), item);
        if from == Some((action, i)) {
            return Ok(None);
        }
        if let Some((other, j)) = from {
            self.clear(other, j);
        }
        self.slots.entry(action).or_insert([None; SLOTS])[i] = Some(item);
        Ok(from)
    }

    /// Empties `action`'s slot `i` (nothing happens for a slot that doesn't
    /// exist).
    fn clear(&mut self, action: A, i: usize) {
        if let Some(slot) = self.slots.get_mut(&action).and_then(|s| s.get_mut(i)) {
            *slot = None;
        }
    }

    /// Whether every slot of `action` is empty.
    fn is_unmapped(&self, action: A) -> bool {
        self.get(action).iter().all(Option::is_none)
    }

    /// The [required](ActionSet::is_required) actions with every slot empty,
    /// in [`ActionSet::ALL`] order.
    fn unmapped_required(&self) -> Vec<A> {
        A::ALL
            .iter()
            .copied()
            .filter(|&a| a.is_required() && self.is_unmapped(a))
            .collect()
    }

    /// Every filled slot with its action, in action then slot order.
    fn pairs(&self) -> impl Iterator<Item = (T, A)> + '_ {
        self.slots
            .iter()
            .flat_map(|(&action, slots)| slots.iter().flatten().map(move |&c| (c, action)))
    }
}

impl<A: ActionSet, T: Copy + fmt::Display> SlotTable<A, T> {
    /// The saved form: every action with all its slots.
    fn saved(&self) -> Saved {
        self.slots
            .iter()
            .map(|(action, slots)| {
                let names = slots.iter().map(|c| c.map(|c| c.to_string())).collect();
                (action.name().to_owned(), names)
            })
            .collect()
    }
}

/// Why [`LayoutBindings::bind`] or [`PadBindings::bind`] refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum BindError<A: ActionSet> {
    /// The chord is fixed (`Escape`, `Delete`) or is the key of an action
    /// that can't be rebound (Debug) in a build with debug tools.
    #[error("{0} is a fixed key and can't be bound")]
    Reserved(Chord),
    /// The action can't be rebound (Debug).
    #[error("{} can't be rebound", .0.name())]
    NotRebindable(A),
    /// Slot index past the last slot.
    #[error("there is no key slot {0}")]
    NoSuchSlot(usize),
}

/// One layout's player-edited bindings: every rebindable action's slots,
/// plus the keys of the actions that can't be rebound (Debug), from the
/// keymap file.
///
/// The pure editing API for the Key bindings screen (0815): [`bind`],
/// [`clear`], [`defaults`] (restore defaults), [`unmapped_required`].
///
/// [`bind`]: Self::bind
/// [`clear`]: Self::clear
/// [`defaults`]: Self::defaults
/// [`unmapped_required`]: Self::unmapped_required
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayoutBindings<A: ActionSet> {
    /// Every rebindable action → its slots.
    table: SlotTable<A, Chord>,
    /// The chords of the actions that can't be rebound (Debug), as in the
    /// keymap file.
    fixed: Vec<(Chord, A)>,
    /// Whether those chords are reserved (builds with debug tools,
    /// ADR-0023). Without debug tools a player may bind them, and their
    /// slot wins over Debug.
    fixed_reserved: bool,
}

impl<A: ActionSet> LayoutBindings<A> {
    /// `layout`'s default keys from `keymap.ron`, each action's chords in
    /// its slots in file order.
    pub fn defaults(def: &KeymapDef<A>, layout: Layout) -> Self {
        Self::from_def(def, layout, DEBUG_TOOLS)
    }

    /// [`defaults`](Self::defaults), with the chords of the actions that
    /// can't be rebound reserved or not.
    pub fn from_def(def: &KeymapDef<A>, layout: Layout, fixed_reserved: bool) -> Self {
        let fixed = A::ALL.iter().filter(|a| !a.is_rebindable());
        let mut bindings = Self {
            table: SlotTable::empty(),
            fixed: fixed
                .flat_map(|&a| def.chords(layout, a).iter().map(move |&c| (c, a)))
                .collect(),
            fixed_reserved,
        };
        for (&action, chords) in def.layouts.get(&layout).into_iter().flatten() {
            for (i, &chord) in chords.iter().take(SLOTS).enumerate() {
                // A loaded keymap can't break the invariant; one built by
                // hand is repaired the same way a player's bind would be.
                bindings.bind(action, i, chord).ok();
            }
        }
        bindings
    }

    /// `action`'s slots (all empty for Debug, which has none).
    pub fn slots(&self, action: A) -> Slots {
        self.table.get(action)
    }

    /// Whether `chord` can never be put in a slot: a [fixed chord] (plain
    /// `Escape` or `Delete`; their `Shift+` chords are ordinary), or the
    /// chord of an action that can't be rebound (Debug) in a build with
    /// debug tools.
    ///
    /// [fixed chord]: Chord::is_reserved
    pub fn is_reserved(&self, chord: Chord) -> bool {
        chord.is_reserved() || (self.fixed_reserved && self.fixed.iter().any(|&(c, _)| c == chord))
    }

    /// The slot of one of `group`'s actions holding `chord`, if any.
    pub fn find(&self, group: A::Group, chord: Chord) -> Option<(A, usize)> {
        self.table.find(group, chord)
    }

    /// Puts `chord` in `action`'s slot `i`, replacing what was there. If
    /// `chord` was in another slot (of any action of `action`'s
    /// group, including `action`), that slot is emptied and
    /// returned: the key *moves* (`docs/design/controls.md`), which may
    /// leave that action with no key. An action of another group keeps it.
    pub fn bind(
        &mut self,
        action: A,
        i: usize,
        chord: Chord,
    ) -> Result<Option<(A, usize)>, BindError<A>> {
        SlotTable::<A, Chord>::check(action, i)?;
        self.allowed(chord)?;
        self.table.place(action, i, chord)
    }

    /// [`BindError::Reserved`] if `chord` [is reserved](Self::is_reserved).
    fn allowed(&self, chord: Chord) -> Result<(), BindError<A>> {
        if self.is_reserved(chord) {
            return Err(BindError::Reserved(chord));
        }
        Ok(())
    }

    /// Empties `action`'s slot `i` (nothing happens for a slot that doesn't
    /// exist).
    pub fn clear(&mut self, action: A, i: usize) {
        self.table.clear(action, i);
    }

    /// Whether every slot of `action` is empty (shows `! not mapped`).
    pub fn is_unmapped(&self, action: A) -> bool {
        self.table.is_unmapped(action)
    }

    /// The [required](ActionSet::is_required) actions with no key, in
    /// [`ActionSet::ALL`] order. The Key bindings screen can't be left while
    /// this isn't empty.
    pub fn unmapped_required(&self) -> Vec<A> {
        self.table.unmapped_required()
    }

    /// The keymap these bindings give: every slot, the chords of the
    /// actions that can't be rebound (unless a slot took one), and the
    /// fixed keys ([`Keymap::new`]). Keys only:
    /// controller buttons don't belong to a layout ([`PadBindings`],
    /// [`Keymap::with_pad`]).
    pub fn keymap(&self, repeat: RepeatDef) -> Keymap<A> {
        let fixed = self.fixed.iter().copied();
        // A chord given twice keeps its last action, so a slot beats Debug.
        Keymap::new(fixed.chain(self.table.pairs()), repeat)
    }
}

/// The player-edited controller buttons: every rebindable action's button
/// slots, each holding a button or a [button chord](ButtonChord) (ticket
/// 1034; a chord and its buttons on their own are different bindings).
/// One setup for both keyboard layouts (`docs/design/controls.md`,
/// *Rebinding buttons*); no button is reserved, and both sticks' directions
/// count as buttons.
///
/// The pure editing API for the Key bindings screen (0816), the same shape
/// as [`LayoutBindings`]: [`bind`](Self::bind), [`clear`](Self::clear),
/// [`defaults`](Self::defaults) (restore defaults),
/// [`unmapped_required`](Self::unmapped_required).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PadBindings<A: ActionSet> {
    table: SlotTable<A, ButtonChord>,
}

impl<A: ActionSet> PadBindings<A> {
    /// The default buttons from `keymap.ron`'s `pad` table, each action's
    /// buttons in its slots in file order.
    pub fn defaults(def: &KeymapDef<A>) -> Self {
        let mut bindings = Self {
            table: SlotTable::empty(),
        };
        for (&action, buttons) in &def.pad {
            for (i, &button) in buttons.iter().take(SLOTS).enumerate() {
                // As in `LayoutBindings::from_def`.
                bindings.bind(action, i, button).ok();
            }
        }
        bindings
    }

    /// `action`'s button slots (all empty for Debug, which has none).
    pub fn slots(&self, action: A) -> ButtonSlots {
        self.table.get(action)
    }

    /// The slot of one of `group`'s actions holding `button` (a button or
    /// a button chord), if any.
    pub fn find(&self, group: A::Group, button: impl Into<ButtonChord>) -> Option<(A, usize)> {
        self.table.find(group, button.into())
    }

    /// Puts `button` (a button or a button chord) in `action`'s slot `i`,
    /// replacing what was there. If `button` was in another slot (of any
    /// action of `action`'s group, including `action`), that slot is
    /// emptied and returned: the button *moves*, which may leave that
    /// action with no button. An action of another group keeps it.
    pub fn bind(
        &mut self,
        action: A,
        i: usize,
        button: impl Into<ButtonChord>,
    ) -> Result<Option<(A, usize)>, BindError<A>> {
        self.table.place(action, i, button.into())
    }

    /// Empties `action`'s slot `i` (nothing happens for a slot that doesn't
    /// exist).
    pub fn clear(&mut self, action: A, i: usize) {
        self.table.clear(action, i);
    }

    /// Whether every slot of `action` is empty (shows `! not mapped`).
    pub fn is_unmapped(&self, action: A) -> bool {
        self.table.is_unmapped(action)
    }

    /// The [required](ActionSet::is_required) actions with no button, in
    /// [`ActionSet::ALL`] order. The Key bindings screen can't be left while
    /// this isn't empty.
    pub fn unmapped_required(&self) -> Vec<A> {
        self.table.unmapped_required()
    }

    /// Every bound button and button chord with its action, for
    /// [`Keymap::with_pad`]: in action then slot order.
    pub fn pairs(&self) -> impl Iterator<Item = (ButtonChord, A)> + '_ {
        self.table.pairs()
    }
}

/// Version written in, and required of, the saved config.
pub const PLAYER_KEYS_VERSION: u32 = 2;

/// The oldest saved version still read: version 1 (ticket 0217) had keys
/// only, and loads with the default buttons.
const OLDEST_READABLE_VERSION: u32 = 1;

/// What repair warnings about the controller buttons start with (a
/// layout's start with its name).
const PAD_WARNING: &str = "controller";

/// Every layout's player bindings and the controller buttons, saved under
/// whatever name the game stores them under. A
/// layout the player hasn't changed has no entry and uses its defaults, and
/// so do the buttons. **Each layout keeps its own keys; the buttons are
/// shared by every layout.**
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerKeys<A: ActionSet> {
    layouts: BTreeMap<Layout, LayoutBindings<A>>,
    /// The player's buttons; `None` while they are the defaults.
    pad: Option<PadBindings<A>>,
}

impl<A: ActionSet> Default for PlayerKeys<A> {
    fn default() -> Self {
        Self {
            layouts: BTreeMap::new(),
            pad: None,
        }
    }
}

/// The saved form: action and chord names as text, so an unknown name
/// drops only itself on load instead of the whole file.
#[derive(Serialize, Deserialize)]
#[serde(rename = "PlayerKeys")]
struct PlayerKeysFile {
    version: u32,
    layouts: BTreeMap<String, Saved>,
    /// The controller buttons, if the player changed them (since version
    /// 2).
    #[serde(default)]
    pad: Option<Saved>,
}

/// Just the version, read first so a future format still gives a clear
/// warning.
#[derive(Deserialize)]
#[serde(rename = "PlayerKeys")]
struct VersionOnly {
    version: u32,
}

impl<A: ActionSet> PlayerKeys<A> {
    /// `layout`'s bindings: the player's, or the defaults if they haven't
    /// changed that layout.
    pub fn bindings(&self, def: &KeymapDef<A>, layout: Layout) -> LayoutBindings<A> {
        self.layouts
            .get(&layout)
            .cloned()
            .unwrap_or_else(|| LayoutBindings::defaults(def, layout))
    }

    /// The controller buttons: the player's, or the defaults if they
    /// haven't changed them. The same whatever the layout.
    pub fn pad_bindings(&self, def: &KeymapDef<A>) -> PadBindings<A> {
        self.pad
            .clone()
            .unwrap_or_else(|| PadBindings::defaults(def))
    }

    /// The keymap for `layout` with the player's keys and buttons.
    pub fn keymap(&self, def: &KeymapDef<A>, layout: Layout) -> Keymap<A> {
        let keys = self.bindings(def, layout).keymap(def.repeat);
        keys.with_pad(self.pad_bindings(def).pairs())
            .with_chord(def.chord)
    }

    /// The keymap before any layout is chosen ([`Keymap::layout_picker`]),
    /// with the player's buttons: someone playing only with a controller
    /// never picks a layout.
    pub fn layout_picker_keymap(&self, def: &KeymapDef<A>) -> Keymap<A> {
        Keymap::layout_picker(def).with_pad(self.pad_bindings(def).pairs())
    }

    /// Replaces the controller buttons. Buttons equal to the defaults are
    /// stored as "no entry", as in [`set`](Self::set).
    pub fn set_pad(&mut self, def: &KeymapDef<A>, bindings: PadBindings<A>) {
        self.pad = (bindings != PadBindings::defaults(def)).then_some(bindings);
    }

    /// Whether the player has changed the controller buttons.
    pub fn is_custom_pad(&self) -> bool {
        self.pad.is_some()
    }

    /// Replaces `layout`'s bindings; the other layout is untouched.
    /// Bindings equal to the defaults are stored as "no entry", so a later
    /// change to the default keys reaches a player who restored them.
    pub fn set(&mut self, def: &KeymapDef<A>, layout: Layout, bindings: LayoutBindings<A>) {
        if bindings == LayoutBindings::defaults(def, layout) {
            self.layouts.remove(&layout);
        } else {
            self.layouts.insert(layout, bindings);
        }
    }

    /// Whether the player has changed `layout`'s keys.
    pub fn is_custom(&self, layout: Layout) -> bool {
        self.layouts.contains_key(&layout)
    }

    /// The saved form (RON): every changed layout with all its actions'
    /// slots, e.g. `"Confirm": [Some("f"), Some("Enter"), None]`, and the
    /// buttons the same way under `pad` if changed, e.g. `"Confirm":
    /// [Some("South"), None, None]`, a button chord as `"South+East"`
    /// (ticket 1034: saved bindings without chords read as before).
    pub fn to_ron(&self) -> String {
        let layouts = self
            .layouts
            .iter()
            .map(|(layout, bindings)| (layout.name().to_owned(), bindings.table.saved()))
            .collect();
        let file = PlayerKeysFile {
            version: PLAYER_KEYS_VERSION,
            layouts,
            pad: self.pad.as_ref().map(|pad| pad.table.saved()),
        };
        let config = ron::ser::PrettyConfig::new()
            .struct_names(true)
            .compact_arrays(true);
        // Strings, maps and options always serialise.
        ron::ser::to_string_pretty(&file, config).unwrap_or_default()
    }

    /// Reads the saved form, repairing rather than failing, and returns
    /// the result with a warning for everything it had to fix:
    ///
    /// - unreadable text or an unknown version: defaults for every layout
    ///   and for the buttons;
    /// - version 1 (keys only): the default buttons;
    /// - layouts the keymap doesn't have, unknown actions, unreadable
    ///   chords or buttons and extra slots are dropped;
    /// - reserved chords are dropped;
    /// - a chord in two stored slots of one group's actions stays in
    ///   the first (in [`ActionSet::ALL`] and slot order); a default slot of
    ///   that group holding a stored chord is emptied (the key moves, as
    ///   with [`LayoutBindings::bind`]);
    /// - an action the layout doesn't list keeps its default slots;
    /// - a layout left with a required action unmapped goes back to its
    ///   defaults;
    /// - the buttons are repaired by the same rules as one layout's keys.
    // check-text: not player text (warnings for the log)
    pub fn from_ron(text: &str, def: &KeymapDef<A>) -> (Self, Vec<String>) {
        let mut warnings = Vec::new();
        let version = match ron::from_str::<VersionOnly>(text) {
            Ok(v) => v.version,
            Err(e) => {
                warnings.push(format!("unreadable, using the default keys: {e}"));
                return (Self::default(), warnings);
            }
        };
        if !(OLDEST_READABLE_VERSION..=PLAYER_KEYS_VERSION).contains(&version) {
            warnings.push(format!(
                "version {version} isn't {PLAYER_KEYS_VERSION}, using the default keys"
            ));
            return (Self::default(), warnings);
        }
        let file = match ron::from_str::<PlayerKeysFile>(text) {
            Ok(file) => file,
            Err(e) => {
                warnings.push(format!("unreadable, using the default keys: {e}"));
                return (Self::default(), warnings);
            }
        };
        let mut keys = Self::default();
        for (name, actions) in &file.layouts {
            let Some(layout) = Layout::from_name(name).filter(|&l| def.has_layout(l)) else {
                warnings.push(format!("unknown layout \"{name}\" dropped"));
                continue;
            };
            let defaults = LayoutBindings::defaults(def, layout);
            let warn = |message: String| warnings.push(format!("{layout}: {message}"));
            let allowed = |chord| defaults.allowed(chord);
            let table = repair(&defaults.table, actions, "key", Chord::parse, allowed, warn);
            keys.set(def, layout, LayoutBindings { table, ..defaults });
        }
        if let Some(actions) = &file.pad {
            let defaults = PadBindings::defaults(def).table;
            let warn = |message: String| warnings.push(format!("{PAD_WARNING}: {message}"));
            let table = repair(
                &defaults,
                actions,
                "button",
                ButtonChord::parse,
                |_| Ok(()),
                warn,
            );
            keys.set_pad(def, PadBindings { table });
        }
        (keys, warnings)
    }
}

/// The slots saved as `actions` over `defaults` (see
/// [`PlayerKeys::from_ron`]), calling `warn` for each fix. `what` is what a
/// slot holds ("key" or "button"), `parse` reads one from its saved name
/// and `allowed` refuses the ones that may not be in a slot.
fn repair<A: ActionSet, T: Copy + Ord + fmt::Display>(
    defaults: &SlotTable<A, T>,
    actions: &Saved,
    what: &str,
    parse: impl Fn(&str) -> Result<T, String>,
    allowed: impl Fn(T) -> Result<(), BindError<A>>,
    mut warn: impl FnMut(String),
) -> SlotTable<A, T> {
    let mut stored = BTreeMap::new();
    for (name, slots) in actions {
        match A::from_name(name).filter(|a| a.is_rebindable()) {
            Some(action) => {
                stored.insert(action, slots);
            }
            None => warn(format!("unknown action \"{name}\" dropped")),
        }
    }
    let mut table = defaults.clone();
    for &action in stored.keys() {
        for i in 0..SLOTS {
            table.clear(action, i);
        }
    }
    let mut claimed = BTreeSet::new();
    for (&action, slots) in &stored {
        if slots.len() > SLOTS {
            warn(format!(
                "{}: {} slots, only the first {SLOTS} kept",
                action.name(),
                slots.len()
            ));
        }
        for (i, name) in slots.iter().enumerate().take(SLOTS) {
            let Some(name) = name else { continue };
            let item = match parse(name) {
                Ok(item) => item,
                Err(e) => {
                    warn(format!("{}: {e}", action.name()));
                    continue;
                }
            };
            if !claimed.insert((action.group(), item)) {
                warn(format!(
                    "{}: {item} is already on another action",
                    action.name()
                ));
                continue;
            }
            if let Err(e) = allowed(item).and_then(|()| table.place(action, i, item)) {
                warn(format!("{}: {e}", action.name()));
            }
        }
    }
    let unmapped = table.unmapped_required();
    if unmapped.is_empty() {
        table
    } else {
        let names: Vec<&str> = unmapped.iter().map(|a| a.name()).collect();
        warn(format!(
            "{} would have no {what}, using the default {what}s",
            names.join(", ")
        ));
        defaults.clone()
    }
}

#[cfg(test)]
mod tests;
