//! [`Keymap`]: the lookup from key chords and controller buttons to a
//! game's actions, plus the key-repeat timings (ADR-0015, ADR-0031,
//! ADR-0034). Generic over the game's [`ActionSet`] (ticket 1106).

use crate::prelude::*;
use alloc::collections::BTreeMap;
use hashbrown::HashMap;
use shuyi_assets::keymap::KeymapDef;
use shuyi_assets::keymap::{ActionSet, Button, Chord, Key, Layout, RepeatDef, UiAction};

use super::pad::{self, PadKind};
use super::state::Device;

/// Lookup from chord, and from controller button, to actions, plus repeat
/// timings. A chord or button is on at most one action of each
/// [group](ActionSet::Group) (ADR-0068), so one press can give an action of
/// each group: the screen on top reacts to the ones it uses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Keymap<A: ActionSet> {
    /// Every chord that does something, fixed keys included, with its
    /// actions in group order.
    bindings: HashMap<Chord, Vec<A>>,
    /// Each action's own chords (not the fixed keys), in the order given.
    chords: BTreeMap<A, Vec<Chord>>,
    /// Every controller button that does something, with its actions in
    /// group order.
    pad: HashMap<Button, Vec<A>>,
    /// Each action's buttons, in the order given.
    buttons: BTreeMap<A, Vec<Button>>,
    repeat: RepeatDef,
}

impl<A: ActionSet> Keymap<A> {
    /// A keymap with `bindings` plus the game's
    /// [fixed keys](ActionSet::fixed_keys). A chord listed twice for one
    /// group keeps its last action of that group;
    /// [reserved](Chord::is_reserved) chords (plain `Escape` and `Delete`)
    /// are left out.
    pub fn new(bindings: impl IntoIterator<Item = (Chord, A)>, repeat: RepeatDef) -> Self {
        let mut lookup = HashMap::new();
        let mut chords: BTreeMap<A, Vec<Chord>> = BTreeMap::new();
        for (chord, action) in bindings {
            if chord.is_reserved() {
                continue;
            }
            if let Some(old) = put(&mut lookup, chord, action)
                && let Some(list) = chords.get_mut(&old)
            {
                list.retain(|&c| c != chord);
            }
            chords.entry(action).or_default().push(chord);
        }
        for &(chord, action) in A::fixed_keys() {
            put(&mut lookup, chord, action);
        }
        Self {
            bindings: lookup,
            chords,
            pad: HashMap::new(),
            buttons: BTreeMap::new(),
            repeat,
        }
    }

    /// This keymap with `pad` as its controller buttons, replacing any it
    /// had. A button listed twice keeps its last action.
    #[must_use]
    pub fn with_pad(mut self, pad: impl IntoIterator<Item = (Button, A)>) -> Self {
        self.pad.clear();
        self.buttons.clear();
        for (button, action) in pad {
            if let Some(old) = put(&mut self.pad, button, action)
                && let Some(list) = self.buttons.get_mut(&old)
            {
                list.retain(|&b| b != button);
            }
            self.buttons.entry(action).or_default().push(button);
        }
        self
    }

    /// This keymap with the default controller buttons from a validated
    /// keymap definition: the same in every layout and the layout picker.
    #[must_use]
    pub fn with_default_pad(self, def: &KeymapDef<A>) -> Self {
        let pad = def.pad.iter();
        self.with_pad(pad.flat_map(|(&a, buttons)| buttons.iter().map(move |&b| (b, a))))
    }

    /// `layout`'s default bindings from a validated keymap definition (no
    /// player changes; for those see
    /// [`PlayerKeys`](super::bindings::PlayerKeys)), with its repeat
    /// timings and the default controller buttons. A definition without
    /// that layout gives a keymap with only the fixed keys and the buttons.
    pub fn for_layout(def: &KeymapDef<A>, layout: Layout) -> Self {
        let keys = def.layouts.get(&layout).into_iter().flatten();
        let bindings = keys.flat_map(|(&a, chords)| chords.iter().map(move |&c| (c, a)));
        Self::new(bindings, def.repeat).with_default_pad(def)
    }

    /// The keys that work before any layout is chosen, from the keymap
    /// definition's `layout_picker` section, so the layout picker can be
    /// used whichever hand the player types with. Actions it doesn't list
    /// have no key. The controller buttons are the default ones, as in
    /// every layout.
    pub fn layout_picker(def: &KeymapDef<A>) -> Self {
        let bindings = def.layout_picker.iter().map(|(&c, &a)| (c, a));
        Self::new(bindings, def.repeat).with_default_pad(def)
    }

    /// The actions bound to `chord`, one at most of each group, in group
    /// order. `Shift+h` and `h` are distinct: there is no fallback from a
    /// shifted chord to the plain one.
    pub fn actions(&self, chord: Chord) -> &[A] {
        self.bindings.get(&chord).map_or(&[], Vec::as_slice)
    }

    /// The first of [`actions`](Self::actions), if any: the only one for a
    /// chord of a single group.
    pub fn action(&self, chord: Chord) -> Option<A> {
        self.actions(chord).first().copied()
    }

    /// The actions bound to the controller button `button`, as
    /// [`actions`](Self::actions).
    pub fn pad_actions(&self, button: Button) -> &[A] {
        self.pad.get(&button).map_or(&[], Vec::as_slice)
    }

    /// The first of [`pad_actions`](Self::pad_actions), if any.
    pub fn pad_action(&self, button: Button) -> Option<A> {
        self.pad_actions(button).first().copied()
    }

    /// Every controller button bound to `action`, in the order they were
    /// given (file order for the keymap file).
    pub fn buttons_for(&self, action: A) -> Vec<Button> {
        self.buttons.get(&action).cloned().unwrap_or_default()
    }

    /// Key-repeat timings.
    pub fn repeat(&self) -> RepeatDef {
        self.repeat
    }

    /// Every chord bound to `action` (its key slots), in the order they
    /// were given: slot order for the player's bindings, file order for
    /// the keymap file. Leaves out the fixed keys (see
    /// [`fixed_chords_for`](Self::fixed_chords_for)).
    pub fn chords_for(&self, action: A) -> Vec<Chord> {
        self.chords.get(&action).cloned().unwrap_or_default()
    }

    /// The [fixed chords](ActionSet::fixed_keys) that also trigger
    /// `action` in every keymap and can't be rebound.
    pub fn fixed_chords_for(action: A) -> Vec<Chord> {
        A::fixed_keys()
            .iter()
            .filter(|&&(_, a)| a == action)
            .map(|&(c, _)| c)
            .collect()
    }

    /// What moves the cursor, for help text: `arrows` (the language's word
    /// for them) when the primary chords of the actions the game takes for
    /// [up, left, down and right](ActionSet::ui) are the plain arrow keys,
    /// otherwise those chords in up-left-down-right order (`wasd`). `None`
    /// if any of them is unbound, or the game has no such action. Lives
    /// here, not with the help text, because it names keys.
    pub fn cursor_keys_name(&self, arrows_name: &str) -> Option<String> {
        let order = [
            (UiAction::Up, Key::Up),
            (UiAction::Left, Key::Left),
            (UiAction::Down, Key::Down),
            (UiAction::Right, Key::Right),
        ];
        let chords = order
            .iter()
            .map(|&(ui, _)| self.primary(A::for_ui(ui)?))
            .collect::<Option<Vec<_>>>()?;
        let arrows = chords
            .iter()
            .zip(order)
            .all(|(chord, (_, arrow))| *chord == Chord::plain(arrow));
        Some(if arrows {
            arrows_name.to_owned()
        } else {
            chords.iter().map(ToString::to_string).collect()
        })
    }

    /// What moves the cursor on a pad of `kind`, for help text: the whole
    /// sets of direction buttons the four cursor actions are on, joined
    /// with `/` (`D-pad/L-stick` with this game's defaults); if they are on
    /// no whole set, each one's [`primary_button`](Self::primary_button)'s
    /// name in up-left-down-right order. `None` if any cursor action has no
    /// button.
    pub fn cursor_buttons_name(&self, kind: PadKind) -> Option<String> {
        let mut bound: [Vec<Button>; 4] = Default::default();
        let order = [
            UiAction::Up,
            UiAction::Down,
            UiAction::Left,
            UiAction::Right,
        ];
        for (buttons, ui) in bound.iter_mut().zip(order) {
            *buttons = self.buttons_for(A::for_ui(ui)?);
        }
        pad::cursor_buttons_name(kind, &bound)
    }

    /// The button help text names for `action`: the first of
    /// [`buttons_for`](Self::buttons_for), or `None` if it has none.
    pub fn primary_button(&self, action: A) -> Option<Button> {
        self.buttons_for(action).into_iter().next()
    }

    /// The chord help text names for `action`: the first of
    /// [`chords_for`](Self::chords_for), or `None` if it has none.
    pub fn primary(&self, action: A) -> Option<Chord> {
        self.chords_for(action).into_iter().next()
    }

    /// Whether `action` has a key (keyboard) or a button (pad) for `device`.
    pub fn has_input(&self, action: A, device: Device) -> bool {
        match device {
            Device::Keyboard => self.primary(action).is_some(),
            Device::Pad(_) => self.primary_button(action).is_some(),
        }
    }
}

/// Binds `item` to `action` in `lookup`, replacing (and returning) the
/// action of the same group it had; its actions stay in group order.
fn put<T: Copy + Eq + core::hash::Hash, A: ActionSet>(
    lookup: &mut HashMap<T, Vec<A>>,
    item: T,
    action: A,
) -> Option<A> {
    let actions = lookup.entry(item).or_default();
    let old = actions
        .iter()
        .position(|a| a.group() == action.group())
        .map(|i| actions.remove(i));
    actions.push(action);
    actions.sort_by_key(|a| a.group());
    old
}
