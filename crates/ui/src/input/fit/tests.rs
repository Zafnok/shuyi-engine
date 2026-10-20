use proptest::prelude::*;
use shuyi_assets::keymap::{ActionSet, ButtonChord, Chord, KeymapDef, PadKeys, UiAction};
use shuyi_assets::target::TargetProfile;

use super::*;

/// A fixture game: the sample game's seven required actions (four
/// directions, Confirm, Cancel, End turn), an eighth that is required
/// too (Info), and an optional one (Log).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum Act {
    Up,
    Down,
    Left,
    Right,
    Confirm,
    Cancel,
    EndTurn,
    Info,
    Log,
}

impl ActionSet for Act {
    type Group = ();
    const ALL: &'static [Self] = &[
        Self::Up,
        Self::Down,
        Self::Left,
        Self::Right,
        Self::Confirm,
        Self::Cancel,
        Self::EndTurn,
        Self::Info,
        Self::Log,
    ];

    fn name(self) -> &'static str {
        match self {
            Self::Up => "Up",
            Self::Down => "Down",
            Self::Left => "Left",
            Self::Right => "Right",
            Self::Confirm => "Confirm",
            Self::Cancel => "Cancel",
            Self::EndTurn => "EndTurn",
            Self::Info => "Info",
            Self::Log => "Log",
        }
    }

    fn is_repeatable(self) -> bool {
        false
    }

    fn is_required(self) -> bool {
        self != Self::Log
    }

    fn is_rebindable(self) -> bool {
        true
    }

    fn group(self) {}

    fn ui(self) -> Option<UiAction> {
        None
    }

    fn fixed_keys() -> &'static [(Chord, Self)] {
        &[]
    }
}

/// The seven the sample game requires.
const SEVEN: [Act; 7] = [
    Act::Up,
    Act::Down,
    Act::Left,
    Act::Right,
    Act::Confirm,
    Act::Cancel,
    Act::EndTurn,
];

fn profile(id: &str) -> TargetProfile {
    let source = match id {
        "playdate" => include_str!("../../../../../targets/playdate.ron"),
        "steam-deck" => include_str!("../../../../../targets/steam-deck.ron"),
        "desktop" => include_str!("../../../../../targets/desktop.ron"),
        _ => unreachable!("{id}"),
    };
    TargetProfile::from_source(id, id, source).unwrap()
}

/// A layout of `bindings`; every other action has no button.
fn layout(bindings: &[(Act, &[&str])]) -> PadKeys<Act> {
    let mut out: PadKeys<Act> = Act::ALL.iter().map(|&a| (a, Vec::new())).collect();
    for &(action, names) in bindings {
        out.insert(
            action,
            names
                .iter()
                .map(|n| ButtonChord::parse(n).unwrap())
                .collect(),
        );
    }
    out
}

/// The Playdate's buttons for the seven, with End turn on `end_turn`.
fn playdate(end_turn: &'static str) -> Vec<(Act, &'static [&'static str])> {
    let end: &'static [&'static str] = match end_turn {
        "East+West" => &["East+West"],
        "CrankForward" => &["CrankForward"],
        _ => unreachable!(),
    };
    vec![
        (Act::Up, &["DpadUp"]),
        (Act::Down, &["DpadDown"]),
        (Act::Left, &["DpadLeft"]),
        (Act::Right, &["DpadRight"]),
        (Act::Confirm, &["East"]),
        (Act::Cancel, &["West"]),
        (Act::EndTurn, end),
    ]
}

fn battle(actions: &[Act]) -> ScreenActions<Act> {
    ScreenActions::declared("battle", actions.to_vec())
}

fn rules(problems: &[FitProblem]) -> Vec<FitRule> {
    problems.iter().map(|p| p.rule).collect()
}

#[test]
fn seven_required_actions_fit_the_playdate_with_end_turn_on_the_chord_or_the_crank() {
    let pd = profile("playdate");
    for end_turn in ["East+West", "CrankForward"] {
        let problems = controls_fit(&pd, &layout(&playdate(end_turn)), &[battle(&SEVEN)]);
        assert_eq!(problems, [], "{end_turn}");
    }
}

#[test]
fn an_eighth_required_action_fails_on_the_playdate_naming_the_screen_and_the_action() {
    let pd = profile("playdate");
    let mut eight = SEVEN.to_vec();
    eight.push(Act::Info);
    // With no button left for it.
    let problems = controls_fit(&pd, &layout(&playdate("East+West")), &[battle(&eight)]);
    assert_eq!(rules(&problems), [FitRule::Unreachable]);
    let p = &problems[0];
    assert_eq!(
        (p.screen, p.actions.as_slice()),
        (Some("battle"), &["Info"][..])
    );
    assert_eq!(
        p.message,
        "screen \"battle\": required action \"Info\" has no input playdate offers (it has no \
         button)"
    );
    // On a button the Playdate hasn't: that binding fails too.
    let mut bindings = playdate("East+West");
    bindings.push((Act::Info, &["North"]));
    let problems = controls_fit(&pd, &layout(&bindings), &[battle(&eight)]);
    assert_eq!(
        rules(&problems),
        [FitRule::NotOnTarget, FitRule::Unreachable]
    );
    assert_eq!(
        problems[0].message,
        "action \"Info\" is bound to North, which playdate doesn't have"
    );
    assert!(problems[1].message.ends_with("(it is bound to North)"));
    // On End turn's chord: the two share it.
    let mut bindings = playdate("East+West");
    bindings.push((Act::Info, &["East+West"]));
    let problems = controls_fit(&pd, &layout(&bindings), &[battle(&eight)]);
    assert_eq!(rules(&problems), [FitRule::Shared]);
    assert_eq!(problems[0].actions, ["EndTurn", "Info"]);
    assert_eq!(problems[0].inputs, ["East+West"]);
    assert_eq!(
        problems[0].message,
        "screen \"battle\": required actions \"EndTurn\" and \"Info\" share East+West on playdate"
    );
    // On the crank, the one input left: it fits.
    let mut bindings = playdate("East+West");
    bindings.push((Act::Info, &["CrankBack"]));
    assert_eq!(controls_fit(&pd, &layout(&bindings), &[battle(&eight)]), []);
}

#[test]
fn two_required_actions_on_one_input_fail_and_on_different_screens_pass() {
    let pd = profile("playdate");
    let mut bindings = playdate("East+West");
    bindings.push((Act::Info, &["East"]));
    let both = [battle(&[Act::Confirm, Act::Info])];
    let problems = controls_fit(&pd, &layout(&bindings), &both);
    assert_eq!(rules(&problems), [FitRule::Shared]);
    assert_eq!(problems[0].actions, ["Confirm", "Info"]);
    assert_eq!(problems[0].screen, Some("battle"));
    let apart = [
        ScreenActions::declared("map", vec![Act::Confirm]),
        ScreenActions::declared("info", vec![Act::Info]),
    ];
    assert_eq!(controls_fit(&pd, &layout(&bindings), &apart), []);
    // An optional action may share a required one's input.
    bindings.push((Act::Log, &["East"]));
    let with_log = [battle(&[Act::Confirm, Act::Log])];
    assert_eq!(controls_fit(&pd, &layout(&bindings), &with_log), []);
}

#[test]
fn three_actions_on_one_input_are_one_problem() {
    let pd = profile("playdate");
    let bindings = [
        (Act::Up, &["East"][..]),
        (Act::Down, &["East"][..]),
        (Act::Left, &["East"][..]),
    ];
    let screen = [battle(&[Act::Up, Act::Down, Act::Left])];
    let problems = controls_fit(&pd, &layout(&bindings), &screen);
    assert_eq!(rules(&problems), [FitRule::Shared]);
    assert!(
        problems[0]
            .message
            .contains("\"Up\", \"Down\" and \"Left\" share East"),
        "{}",
        problems[0].message
    );
}

#[test]
fn the_system_menu_holds_optional_actions_and_only_as_many_as_the_target_has() {
    let pd = profile("playdate");
    assert_eq!(pd.input.system_menu_slots, 3);
    let screen = [battle(&[Act::Info, Act::Log])];
    // A required action bound only to the system menu.
    let problems = controls_fit(&pd, &layout(&[(Act::Info, &["SystemMenu0"])]), &screen);
    assert_eq!(rules(&problems), [FitRule::Unreachable]);
    assert!(
        problems[0]
            .message
            .ends_with("(a system menu item holds optional actions only)")
    );
    // An optional one there: fine, in each of the three slots.
    for slot in ["SystemMenu0", "SystemMenu1", "SystemMenu2"] {
        let bindings = [(Act::Info, &["East"][..]), (Act::Log, &[slot][..])];
        assert_eq!(controls_fit(&pd, &layout(&bindings), &screen), [], "{slot}");
    }
    // The fourth slot isn't the Playdate's.
    let bindings = [(Act::Info, &["East"][..]), (Act::Log, &["SystemMenu3"][..])];
    let problems = controls_fit(&pd, &layout(&bindings), &screen);
    assert_eq!(rules(&problems), [FitRule::MenuSlot]);
    assert_eq!(problems[0].screen, None);
    assert_eq!(
        problems[0].message,
        "action \"Log\" is bound to SystemMenu3, but playdate's system menu holds 3 item(s) of \
         the game's"
    );
    // The Deck has no system menu of the game's.
    let deck = profile("steam-deck");
    let problems = controls_fit(&deck, &layout(&[(Act::Log, &["SystemMenu0"])]), &[]);
    assert_eq!(rules(&problems), [FitRule::MenuSlot]);
}

#[test]
fn a_binding_on_an_input_the_target_lacks_fails_on_any_action() {
    let pd = profile("playdate");
    // A chord the Playdate doesn't list, on an action no screen declares.
    let problems = controls_fit(&pd, &layout(&[(Act::Log, &["West+DpadLeft"])]), &[]);
    assert_eq!(rules(&problems), [FitRule::NotOnTarget]);
    assert_eq!(problems[0].inputs, ["West+DpadLeft"]);
    assert_eq!(
        problems[0].message,
        "action \"Log\" is bound to the chord West+DpadLeft, which playdate doesn't list"
    );
    // The listed chord, written either way, is the Playdate's.
    let bindings = [(Act::Log, &["West+East"][..])];
    assert_eq!(controls_fit(&pd, &layout(&bindings), &[]), []);
    // Its buttons and its crank are.
    let bindings = [(Act::Log, &["DpadUp", "CrankBack", "CrankForward"][..])];
    assert_eq!(controls_fit(&pd, &layout(&bindings), &[]), []);
    // A required action keeps an input it has when another is lacking.
    let bindings = [(Act::Info, &["North", "East"][..])];
    let problems = controls_fit(&pd, &layout(&bindings), &[battle(&[Act::Info])]);
    assert_eq!(rules(&problems), [FitRule::NotOnTarget]);
}

#[test]
fn a_screen_that_declares_nothing_counts_every_action() {
    let pd = profile("playdate");
    let open = ScreenActions {
        screen: "old",
        actions: None,
    };
    // Info is required, so the old screen needs it too.
    let problems = controls_fit(&pd, &layout(&playdate("East+West")), &[open]);
    assert_eq!(rules(&problems), [FitRule::Unreachable]);
    assert_eq!(problems[0].actions, ["Info"]);
}

/// A keymap whose `targets` table has `buttons` for `target`.
fn keymap(tables: &[(&str, PadKeys<Act>)]) -> KeymapDef<Act> {
    let mut def = KeymapDef::default();
    for (target, buttons) in tables {
        def.targets.insert((*target).to_owned(), buttons.clone());
    }
    def
}

#[test]
fn a_game_is_checked_on_the_table_named_after_the_target() {
    let screens = [
        battle(&SEVEN),
        ScreenActions {
            screen: "title",
            actions: None,
        },
    ];
    let mut game = GameControls::new(keymap(&[("playdate", layout(&playdate("CrankForward")))]));
    for s in screens {
        game = game.with(s);
    }
    assert_eq!(game.screens().len(), 2);
    // The title doesn't declare: a warning, and every action counts on it,
    // Info too.
    let problems = game.fit(&profile("playdate"));
    assert_eq!(
        rules(&problems),
        [FitRule::Undeclared, FitRule::Unreachable]
    );
    assert!(!problems[0].is_error());
    assert!(problems[1].is_error());
    assert_eq!(
        problems[0].message,
        "screen \"title\" doesn't declare its actions (Screen::actions): every action counts as \
         its"
    );
    assert_eq!(problems[1].screen, Some("title"));
    // No table for the Deck, which has no keyboard.
    let deck = game.fit(&profile("steam-deck"));
    assert_eq!(rules(&deck), [FitRule::Undeclared, FitRule::NoLayout]);
    assert_eq!(
        deck[1].message,
        "the keymap file has no buttons for steam-deck, which has no keyboard: give it a table \
         in `targets` (\"steam-deck\")"
    );
    // The desktop has a keyboard: its keys reach everything.
    assert_eq!(rules(&game.fit(&profile("desktop"))), [FitRule::Undeclared]);
    // Through the trait, as the tools ask.
    let controls: &dyn Controls = &game;
    assert_eq!(controls.fit(&profile("steam-deck")), deck);
}

#[test]
fn a_screens_actions_are_what_it_declares() {
    let declared = battle(&[Act::Info, Act::Log, Act::Up, Act::Up]);
    assert_eq!(
        declared.required().collect::<Vec<_>>(),
        [Act::Up, Act::Info]
    );
    let open = ScreenActions::<Act> {
        screen: "x",
        actions: None,
    };
    assert_eq!(open.required().count(), 8);
}

#[test]
fn names_are_listed_as_a_sentence() {
    assert_eq!(quoted_list(&[]), "");
    assert_eq!(quoted_list(&["a"]), "\"a\"");
    assert_eq!(quoted_list(&["a", "b"]), "\"a\" and \"b\"");
    assert_eq!(quoted_list(&["a", "b", "c"]), "\"a\", \"b\" and \"c\"");
}

/// The Playdate's inputs, a lacking-free universe: its buttons, its
/// chord and its three system menu items.
const OFFERED: [&str; 12] = [
    "DpadUp",
    "DpadDown",
    "DpadLeft",
    "DpadRight",
    "East",
    "West",
    "CrankForward",
    "CrankBack",
    "East+West",
    "SystemMenu0",
    "SystemMenu1",
    "SystemMenu2",
];

fn any_layout() -> impl Strategy<Value = PadKeys<Act>> {
    let bindings = prop::collection::btree_set(0..OFFERED.len(), 0..3);
    prop::collection::vec(bindings, Act::ALL.len()).prop_map(|per_action| {
        Act::ALL
            .iter()
            .zip(per_action)
            .map(|(&a, picks)| {
                let chords = picks
                    .into_iter()
                    .map(|i| ButtonChord::parse(OFFERED[i]).unwrap())
                    .collect();
                (a, chords)
            })
            .collect()
    })
}

fn any_screens() -> impl Strategy<Value = Vec<ScreenActions<Act>>> {
    let names = ["a", "b", "c"];
    prop::collection::vec(
        prop::collection::vec(prop::sample::select(Act::ALL), 0..6),
        0..3,
    )
    .prop_map(move |lists| {
        lists
            .into_iter()
            .zip(names)
            .map(|(actions, screen)| ScreenActions::declared(screen, actions))
            .collect()
    })
}

proptest! {
    /// On inputs the target has, the check fails exactly when some
    /// screen has a required action with no input (the system menu
    /// doesn't count) or two required actions on one input.
    #[test]
    fn the_check_fails_exactly_when_a_screen_has_a_required_action_short_or_two_on_one_input(
        layout in any_layout(),
        screens in any_screens(),
    ) {
        let pd = profile("playdate");
        let usable = |a: Act| -> Vec<ButtonChord> {
            layout[&a]
                .iter()
                .copied()
                .filter(|b| b.single_button().is_none_or(|b| b.system_menu_slot().is_none()))
                .collect()
        };
        let broken = screens.iter().any(|s| {
            let required: Vec<Act> = Act::ALL
                .iter()
                .copied()
                .filter(|a| a.is_required() && s.actions.as_ref().is_some_and(|d| d.contains(a)))
                .collect();
            let short = required.iter().any(|&a| usable(a).is_empty());
            let shared = required.iter().enumerate().any(|(i, &a)| {
                required[i + 1..]
                    .iter()
                    .any(|&b| usable(a).iter().any(|x| usable(b).contains(x)))
            });
            short || shared
        });
        let problems = controls_fit(&pd, &layout, &screens);
        prop_assert_eq!(!problems.is_empty(), broken, "{:?}", problems);
        prop_assert!(problems.iter().all(|p| matches!(p.rule, FitRule::Unreachable | FitRule::Shared)));
    }
}
