//! Whether a game's controls fit a target (ADR-0008 §3): on each screen,
//! the required actions it declares ([`Screen::actions`]) must bind to
//! *distinct* inputs the target's profile offers.
//!
//! [`controls_fit`] is the check, pure: a target's profile, the default
//! buttons the game plays it with (the keymap file's table named after
//! the target, [`KeymapDef::target_buttons`]) and the screens' actions in,
//! every [`FitProblem`] out. It fails when:
//!
//! - a binding uses an input the profile lacks: a button it doesn't have,
//!   a chord it doesn't list, a system menu item beyond its slots;
//! - a required action a screen declares has no input the profile offers;
//! - two required actions of one screen share an input.
//!
//! **What counts as an input** is the profile's: its `buttons` (a crank's
//! notches among them, `CrankForward` and `CrankBack`), and each chord it
//! lists as one more. **What doesn't:** long and double presses (a keymap
//! can't say them), chords the profile doesn't list, the system's own
//! buttons (no keymap can name them: the Playdate's Menu and Lock, the
//! Deck's Steam and Quick Access), and the system menu's items
//! (`SystemMenu0` and on), which may hold optional actions only.
//! The engine never folds actions onto fewer buttons to make a game fit:
//! the game redesigns the screen.
//!
//! [`GameControls`] holds a game's keymap and screens for `shuyi check
//! --target`, which asks it through the [`Controls`] trait, by target.
//!
//! [`Screen::actions`]: crate::screen::Screen::actions

use crate::prelude::*;
use alloc::collections::BTreeMap;

use shuyi_assets::keymap::{ActionSet, ButtonChord, KeymapDef, PadKeys};
use shuyi_assets::target::TargetProfile;

use crate::ctx::GameData;
use crate::screen::Screen;

/// One screen's actions, as the check sees them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScreenActions<A> {
    /// The screen's [`name`](Screen::name).
    pub screen: &'static str,
    /// What it declares ([`Screen::actions`]); `None` if it doesn't, and
    /// then every action counts as its.
    pub actions: Option<Vec<A>>,
}

impl<A: ActionSet> ScreenActions<A> {
    /// `screen`'s name and the actions it declares.
    pub fn of<G: GameData<Action = A>>(screen: &dyn Screen<G>) -> Self {
        Self {
            screen: screen.name(),
            actions: screen.actions(),
        }
    }

    /// The screen named `screen`, declaring `actions`.
    pub fn declared(screen: &'static str, actions: Vec<A>) -> Self {
        Self {
            screen,
            actions: Some(actions),
        }
    }

    /// Its required actions, each once, in [`ActionSet::ALL`] order.
    fn required(&self) -> impl Iterator<Item = A> + '_ {
        A::ALL.iter().copied().filter(|a| {
            a.is_required()
                && self
                    .actions
                    .as_ref()
                    .is_none_or(|declared| declared.contains(a))
        })
    }
}

/// Which rule a [`FitProblem`] breaks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FitRule {
    /// The keymap has no buttons for a target that has no keyboard.
    NoLayout,
    /// A binding uses a button the target doesn't have, or a chord it
    /// doesn't list.
    NotOnTarget,
    /// A binding uses a system menu item beyond the target's slots.
    MenuSlot,
    /// A required action of a screen has no input the target offers.
    Unreachable,
    /// Two required actions of one screen share an input.
    Shared,
    /// A screen doesn't declare its actions, so every action counts as
    /// its: a warning, not an error.
    Undeclared,
}

impl FitRule {
    /// Whether it fails the check (all but [`Undeclared`](Self::Undeclared)).
    pub fn is_error(self) -> bool {
        self != Self::Undeclared
    }
}

/// A way a game's controls don't fit a target, naming what it is about.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FitProblem {
    /// The rule it breaks.
    pub rule: FitRule,
    /// The screen it is about, if one.
    pub screen: Option<&'static str>,
    /// The actions it is about, by name.
    pub actions: Vec<&'static str>,
    /// The inputs it is about, as the keymap file names them (`East`,
    /// `East+West`).
    pub inputs: Vec<String>,
    /// What is wrong, with the names.
    pub message: String,
}

impl FitProblem {
    /// Whether it fails the check ([`FitRule::is_error`]).
    pub fn is_error(&self) -> bool {
        self.rule.is_error()
    }
}

/// What a binding is on a target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Offer {
    /// An input of the target's: a button it has, or a chord it lists.
    Input,
    /// One of the target's system menu items: for optional actions only.
    Menu,
    /// A button the target doesn't have, or a chord it doesn't list.
    Lacking,
    /// A system menu item beyond the target's slots.
    PastMenu,
}

/// What `binding` is on `profile`'s target.
fn offer(profile: &TargetProfile, binding: ButtonChord) -> Offer {
    let input = &profile.input;
    if binding.buttons().is_some() {
        let listed = input
            .chords
            .iter()
            .any(|&[a, b]| ButtonChord::pair(a, b) == Some(binding));
        return if listed { Offer::Input } else { Offer::Lacking };
    }
    let button = binding.first();
    match button.system_menu_slot() {
        Some(slot) if slot < input.system_menu_slots => Offer::Menu,
        Some(_) => Offer::PastMenu,
        None if input.buttons.contains(&button) => Offer::Input,
        None => Offer::Lacking,
    }
}

/// `names` as a sentence: `"a"`, `"a" and "b"`, `"a", "b" and "c"`.
fn quoted_list(names: &[&str]) -> String {
    let quoted: Vec<String> = names.iter().map(|n| format!("\"{n}\"")).collect();
    match quoted.split_last() {
        Some((last, rest)) if !rest.is_empty() => format!("{} and {last}", rest.join(", ")),
        Some((last, _)) => last.clone(),
        None => String::new(),
    }
}

/// Whether the controls of `layout` (a target's default buttons) fit
/// `profile` on each of `screens`: every problem, the layout's first (in
/// action order), then each screen's in the order given (see the [module
/// docs](self)).
pub fn controls_fit<A: ActionSet>(
    profile: &TargetProfile,
    layout: &PadKeys<A>,
    screens: &[ScreenActions<A>],
) -> Vec<FitProblem> {
    let target = profile.id.as_str();
    let mut out = Vec::new();
    for (&action, bindings) in layout {
        for &binding in bindings {
            let name = action.name();
            let (rule, message) = match offer(profile, binding) {
                Offer::Input | Offer::Menu => continue,
                Offer::Lacking if binding.buttons().is_some() => (
                    FitRule::NotOnTarget,
                    format!(
                        "action \"{name}\" is bound to the chord {binding}, which {target} \
                         doesn't list"
                    ),
                ),
                Offer::Lacking => (
                    FitRule::NotOnTarget,
                    format!("action \"{name}\" is bound to {binding}, which {target} doesn't have"),
                ),
                Offer::PastMenu => (
                    FitRule::MenuSlot,
                    format!(
                        "action \"{name}\" is bound to {binding}, but {target}'s system menu \
                         holds {} item(s) of the game's",
                        profile.input.system_menu_slots
                    ),
                ),
            };
            out.push(FitProblem {
                rule,
                screen: None,
                actions: vec![name],
                inputs: vec![binding.to_string()],
                message,
            });
        }
    }
    for screen in screens {
        let name = screen.screen;
        let mut on: BTreeMap<ButtonChord, Vec<&'static str>> = BTreeMap::new();
        for action in screen.required() {
            let bindings = layout.get(&action).map_or(&[][..], Vec::as_slice);
            let usable: Vec<ButtonChord> = bindings
                .iter()
                .copied()
                .filter(|&b| offer(profile, b) == Offer::Input)
                .collect();
            if usable.is_empty() {
                let why = if bindings.is_empty() {
                    "it has no button".to_owned()
                } else if bindings.iter().all(|&b| offer(profile, b) == Offer::Menu) {
                    "a system menu item holds optional actions only".to_owned()
                } else {
                    let shown: Vec<String> = bindings.iter().map(ToString::to_string).collect();
                    format!("it is bound to {}", shown.join(", "))
                };
                out.push(FitProblem {
                    rule: FitRule::Unreachable,
                    screen: Some(name),
                    actions: vec![action.name()],
                    inputs: bindings.iter().map(ToString::to_string).collect(),
                    message: format!(
                        "screen \"{name}\": required action \"{}\" has no input {target} offers \
                         ({why})",
                        action.name()
                    ),
                });
            }
            for binding in usable {
                on.entry(binding).or_default().push(action.name());
            }
        }
        for (binding, actions) in on {
            if actions.len() > 1 {
                out.push(FitProblem {
                    rule: FitRule::Shared,
                    screen: Some(name),
                    message: format!(
                        "screen \"{name}\": required actions {} share {binding} on {target}",
                        quoted_list(&actions)
                    ),
                    actions,
                    inputs: vec![binding.to_string()],
                });
            }
        }
    }
    out
}

/// A game's controls as `shuyi check --target` checks them, by target
/// (`shuyi_cli::GameTools::controls`).
pub trait Controls {
    /// Every problem of the game's controls on the target `profile`
    /// describes ([`GameControls::fit`]).
    fn fit(&self, profile: &TargetProfile) -> Vec<FitProblem>;
}

/// A game's keymap and its screens, for the check on every target.
#[derive(Debug, Clone)]
pub struct GameControls<A: ActionSet> {
    keymap: KeymapDef<A>,
    screens: Vec<ScreenActions<A>>,
}

impl<A: ActionSet> GameControls<A> {
    /// The game whose default bindings are `keymap`, with no screens yet.
    pub fn new(keymap: KeymapDef<A>) -> Self {
        Self {
            keymap,
            screens: Vec::new(),
        }
    }

    /// The same, with `screen` too. Every screen the game can show is
    /// listed, overlays and popups on their own.
    #[must_use]
    pub fn with(mut self, screen: ScreenActions<A>) -> Self {
        self.screens.push(screen);
        self
    }

    /// The screens listed, in order.
    pub fn screens(&self) -> &[ScreenActions<A>] {
        &self.screens
    }

    /// Every problem on `profile`'s target: a warning for each screen that
    /// doesn't declare its actions, then [`controls_fit`] on the keymap's
    /// table for the target. A target without a table plays with the keys
    /// if it has a keyboard (the keymap file's own check covers them), and
    /// fails if it hasn't.
    pub fn fit(&self, profile: &TargetProfile) -> Vec<FitProblem> {
        let target = profile.id.as_str();
        let mut out: Vec<FitProblem> = self
            .screens
            .iter()
            .filter(|s| s.actions.is_none())
            .map(|s| FitProblem {
                rule: FitRule::Undeclared,
                screen: Some(s.screen),
                actions: Vec::new(),
                inputs: Vec::new(),
                message: format!(
                    "screen \"{}\" doesn't declare its actions (Screen::actions): every action \
                     counts as its",
                    s.screen
                ),
            })
            .collect();
        match self.keymap.target_buttons(target) {
            Some(layout) => out.extend(controls_fit(profile, layout, &self.screens)),
            None if profile.input.keyboard => {}
            None => out.push(FitProblem {
                rule: FitRule::NoLayout,
                screen: None,
                actions: Vec::new(),
                inputs: Vec::new(),
                message: format!(
                    "the keymap file has no buttons for {target}, which has no keyboard: give \
                     it a table in `targets` (\"{target}\")"
                ),
            }),
        }
        out
    }
}

impl<A: ActionSet> Controls for GameControls<A> {
    fn fit(&self, profile: &TargetProfile) -> Vec<FitProblem> {
        GameControls::fit(self, profile)
    }
}

#[cfg(test)]
mod tests;
