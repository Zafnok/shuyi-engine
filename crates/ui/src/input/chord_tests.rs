//! Button chords (ticket 1034): how they fire in [`InputState`], over the
//! sample game's actions with a Playdate's buttons: the D-pad moves the
//! cursor, `A` (`South`) confirms, `B` (`East`) cancels, `A+B` ends the
//! turn and `B+←` / `B+→` pick the previous / next unit.
use core::time::Duration;

use proptest::prelude::*;

use super::sample::{
    Action, InputState, KeyGroup, Keymap, PadBindings, PlayerKeys, RIGHT_HANDED, load_keymap,
};
use super::*;
use Action::{
    Cancel, Confirm, CursorDown, CursorLeft, CursorRight, CursorUp, EndTurn, NextUnit, PrevUnit,
};
use Button::{DpadDown, DpadLeft, DpadRight, DpadUp, East, North, South, West};

const PLAYDATE: PadKind = PadKind::Playdate;

/// The chord window of [`playdate`], in milliseconds.
const WINDOW_MS: u32 = 80;

/// The repeat timings of [`playdate`].
const REPEAT: RepeatDef = RepeatDef {
    delay_ms: 300,
    interval_ms: 55,
};

fn pair(a: Button, b: Button) -> ButtonChord {
    ButtonChord::pair(a, b).unwrap_or(ButtonChord::single(a))
}

/// A Playdate's buttons, as `docs/design/playdate.md` has them.
fn playdate_keymap() -> Keymap {
    Keymap::new([], REPEAT)
        .with_pad([
            (DpadLeft.into(), CursorLeft),
            (DpadRight.into(), CursorRight),
            (DpadUp.into(), CursorUp),
            (DpadDown.into(), CursorDown),
            (South.into(), Confirm),
            (East.into(), Cancel),
            (pair(South, East), EndTurn),
            (pair(East, DpadLeft), PrevUnit),
            (pair(East, DpadRight), NextUnit),
        ])
        .with_chord(ChordDef {
            window_ms: WINDOW_MS,
        })
}

fn playdate() -> InputState {
    InputState::new(playdate_keymap())
}

/// Seconds for `ms` milliseconds.
fn ms(ms: u32) -> f32 {
    Duration::from_millis(u64::from(ms)).as_secs_f32()
}

/// One frame: `events` (a button and whether it went down), then an
/// update of `dt_ms`. Returns the update's actions.
fn frame(s: &mut InputState, events: &[(Button, bool)], dt_ms: u32) -> Vec<Action> {
    for &(button, down) in events {
        if down {
            s.pad_down(button, PLAYDATE);
        } else {
            s.pad_up(button);
        }
    }
    s.update(ms(dt_ms))
}

/// Every action of frames of `dt_ms` each until nothing is held or
/// waiting and `frames` have passed.
fn idle(s: &mut InputState, frames: usize, dt_ms: u32) -> Vec<Action> {
    (0..frames).flat_map(|_| s.update(ms(dt_ms))).collect()
}

#[test]
fn who_waits_follows_the_buttons_own_actions() {
    let km = playdate_keymap();
    // A: its partner B waits too, so A waits only the window.
    assert_eq!(km.chord_wait(South), ChordWait::Window);
    // B: a partner (the D-pad) acts on press, so B waits until let go.
    assert_eq!(km.chord_wait(East), ChordWait::Release);
    // The D-pad moves the cursor, which repeats: never held back.
    assert_eq!(km.chord_wait(DpadLeft), ChordWait::None);
    assert_eq!(km.chord_wait(DpadRight), ChordWait::None);
    // In no chord.
    assert_eq!(km.chord_wait(DpadUp), ChordWait::None);
    assert_eq!(km.chord_wait(West), ChordWait::None);
    // A chord button with no action of its own waits until let go.
    let lone = Keymap::new([], REPEAT).with_pad([(pair(South, West), EndTurn)]);
    assert_eq!(lone.chord_wait(South), ChordWait::Release);
    // A pad without chords: everything acts on press.
    let pc = Keymap::new([], REPEAT).with_pad([(South, Confirm), (East, Cancel)]);
    assert!(
        Button::ALL
            .iter()
            .all(|&b| pc.chord_wait(b) == ChordWait::None)
    );
}

#[test]
fn the_keymap_looks_chords_up_in_either_order() {
    let km = playdate_keymap();
    assert_eq!(km.chord_actions(pair(East, South)), [EndTurn]);
    assert_eq!(km.chord_actions(pair(South, East)), [EndTurn]);
    assert_eq!(km.chord_actions(pair(South, West)), []);
    assert_eq!(km.pad_actions(South), [Confirm]);
    assert_eq!(km.buttons_for(EndTurn), [pair(South, East)]);
    assert_eq!(km.primary_button(PrevUnit), Some(pair(East, DpadLeft)));
    assert_eq!(km.chord(), ChordDef { window_ms: 80 });
    assert_eq!(Keymap::new([], REPEAT).chord(), ChordDef::default());
    // The cursor's name leaves the chords out.
    assert_eq!(km.cursor_buttons_name(PLAYDATE).as_deref(), Some("D-pad"));
}

#[test]
fn a_and_b_together_end_the_turn_in_either_order() {
    for (first, second) in [(South, East), (East, South)] {
        // In one frame.
        let mut s = playdate();
        assert_eq!(
            frame(&mut s, &[(first, true), (second, true)], 33),
            [EndTurn]
        );
        assert!(s.is_held(EndTurn));
        assert!(!s.is_held(Confirm) && !s.is_held(Cancel));
        assert_eq!(frame(&mut s, &[(first, false)], 33), []);
        assert!(!s.is_held(EndTurn), "letting go of either ends it");
        assert_eq!(frame(&mut s, &[(second, false)], 33), []);
        assert_eq!(idle(&mut s, 30, 33), []);
        // A frame apart, inside the window.
        let mut s = playdate();
        assert_eq!(frame(&mut s, &[(first, true)], 33), []);
        assert_eq!(frame(&mut s, &[], 33), []);
        assert_eq!(frame(&mut s, &[(second, true)], 33), [EndTurn], "{first:?}");
        assert_eq!(frame(&mut s, &[(second, false), (first, false)], 33), []);
        assert_eq!(idle(&mut s, 30, 33), []);
        assert_eq!(s.device(), Device::Pad(PLAYDATE));
    }
}

#[test]
fn b_held_then_a_any_time_ends_the_turn() {
    let mut s = playdate();
    assert_eq!(frame(&mut s, &[(East, true)], 33), []);
    assert_eq!(idle(&mut s, 60, 33), [], "B waits while held");
    assert_eq!(frame(&mut s, &[(South, true)], 33), [EndTurn]);
    assert_eq!(frame(&mut s, &[(South, false)], 33), []);
    // Still held: A again ends it again.
    assert_eq!(frame(&mut s, &[(South, true)], 33), [EndTurn]);
    assert_eq!(frame(&mut s, &[(South, false), (East, false)], 33), []);
    assert_eq!(idle(&mut s, 30, 33), []);
}

#[test]
fn a_alone_confirms_after_the_window_or_when_let_go() {
    // Tapped: on release.
    let mut s = playdate();
    assert_eq!(frame(&mut s, &[(South, true)], 33), []);
    assert!(!s.is_held(Confirm), "held back");
    assert_eq!(frame(&mut s, &[(South, false)], 33), [Confirm]);
    assert!(!s.is_held(Confirm));
    assert_eq!(idle(&mut s, 30, 33), []);
    // Held: once the window has passed, counted from the frame after
    // the press, and then held (hold Confirm to fast-forward).
    let mut s = playdate();
    assert_eq!(frame(&mut s, &[(South, true)], 33), []);
    assert_eq!(frame(&mut s, &[], 40), []);
    assert_eq!(frame(&mut s, &[], 39), []);
    assert_eq!(frame(&mut s, &[], 1), [Confirm]);
    assert!(s.is_held(Confirm));
    assert_eq!(idle(&mut s, 60, 33), [], "Confirm doesn't repeat");
    assert_eq!(frame(&mut s, &[(South, false)], 33), []);
    assert!(!s.is_held(Confirm));
}

#[test]
fn a_long_frame_never_eats_the_window_of_a_press_made_during_it() {
    let mut s = playdate();
    assert_eq!(frame(&mut s, &[(South, true)], 500), []);
    assert_eq!(frame(&mut s, &[(East, true)], 500), [EndTurn]);
}

#[test]
fn b_after_a_has_confirmed_is_b_on_its_own() {
    let mut s = playdate();
    frame(&mut s, &[(South, true)], 33);
    assert_eq!(frame(&mut s, &[], 100), [Confirm]);
    assert_eq!(frame(&mut s, &[(East, true)], 33), []);
    assert_eq!(frame(&mut s, &[(East, false)], 33), [Cancel]);
    assert_eq!(frame(&mut s, &[(South, false)], 33), []);
}

#[test]
fn b_alone_cancels_when_let_go() {
    for held_ms in [0, 33, 80, 2000] {
        let mut s = playdate();
        assert_eq!(frame(&mut s, &[(East, true)], held_ms), []);
        assert!(!s.is_held(Cancel));
        assert_eq!(frame(&mut s, &[(East, false)], 33), [Cancel], "{held_ms}");
        assert_eq!(idle(&mut s, 30, 33), []);
    }
}

#[test]
fn hold_b_and_left_or_right_picks_the_previous_or_next_unit() {
    let mut s = playdate();
    assert_eq!(frame(&mut s, &[(East, true)], 33), []);
    assert_eq!(frame(&mut s, &[(DpadLeft, true)], 33), [PrevUnit]);
    assert!(!s.is_held(CursorLeft) && s.is_held(PrevUnit));
    assert_eq!(frame(&mut s, &[(DpadLeft, false)], 33), []);
    assert_eq!(frame(&mut s, &[(DpadLeft, true)], 33), [PrevUnit]);
    assert_eq!(
        frame(&mut s, &[(DpadLeft, false), (DpadRight, true)], 33),
        [NextUnit]
    );
    assert_eq!(frame(&mut s, &[(DpadRight, false)], 33), []);
    // B's own Cancel is gone for this hold.
    assert_eq!(frame(&mut s, &[(East, false)], 33), []);
    assert_eq!(idle(&mut s, 30, 33), []);
    // The next hold of B cancels again.
    assert_eq!(frame(&mut s, &[(East, true)], 33), []);
    assert_eq!(frame(&mut s, &[(East, false)], 33), [Cancel]);
}

#[test]
fn hold_b_and_left_held_repeats_as_the_cursor_does() {
    let mut s = playdate();
    frame(&mut s, &[(East, true)], 33);
    assert_eq!(frame(&mut s, &[(DpadLeft, true)], 0), [PrevUnit]);
    assert_eq!(s.update(ms(299)), []);
    assert_eq!(s.update(ms(1)), [PrevUnit]);
    assert_eq!(s.update(ms(55)), [PrevUnit]);
    // Letting go of B ends it; ← still down does nothing more.
    assert_eq!(frame(&mut s, &[(East, false)], 500), []);
    assert!(!s.is_held(PrevUnit) && !s.is_held(CursorLeft));
    assert_eq!(frame(&mut s, &[(DpadLeft, false)], 33), []);
    assert_eq!(idle(&mut s, 30, 33), []);
}

#[test]
fn left_before_b_moves_the_cursor_and_b_still_cancels() {
    let mut s = playdate();
    assert_eq!(frame(&mut s, &[(DpadLeft, true)], 33), [CursorLeft]);
    assert_eq!(frame(&mut s, &[(East, true)], 33), []);
    assert_eq!(
        frame(&mut s, &[(East, false), (DpadLeft, false)], 33),
        [Cancel]
    );
}

#[test]
fn up_while_b_is_held_moves_the_cursor_and_b_still_cancels() {
    let mut s = playdate();
    frame(&mut s, &[(East, true)], 33);
    assert_eq!(frame(&mut s, &[(DpadUp, true)], 33), [CursorUp]);
    assert_eq!(
        frame(&mut s, &[(DpadUp, false), (East, false)], 33),
        [Cancel]
    );
}

#[test]
fn the_cursor_acts_on_press_and_repeats_on_a_pad_with_chords() {
    let mut s = playdate();
    assert_eq!(frame(&mut s, &[(DpadDown, true)], 0), [CursorDown]);
    assert_eq!(s.update(ms(300)), [CursorDown]);
    assert_eq!(frame(&mut s, &[(DpadLeft, true)], 0), [CursorLeft]);
    assert_eq!(s.update(ms(300)), [CursorLeft]);
}

#[test]
fn a_new_keymap_forgets_waiting_buttons() {
    let mut s = playdate();
    frame(&mut s, &[(East, true), (South, true)], 0);
    s.set_keymap(playdate_keymap());
    assert_eq!(s.update(0.0), []);
    assert_eq!(frame(&mut s, &[(South, false), (East, false)], 500), []);
    // B pressed again works as usual.
    frame(&mut s, &[(East, true)], 0);
    assert_eq!(frame(&mut s, &[(East, false)], 0), [Cancel]);
}

#[test]
fn chords_bound_by_the_player_work_and_save() {
    let def = load_keymap().unwrap_or_default();
    let mut pad = PadBindings::defaults(&def);
    assert_eq!(pad.bind(EndTurn, 1, pair(West, North)), Ok(None));
    assert_eq!(pad.slots(EndTurn)[1], Some(pair(North, West)));
    assert_eq!(
        pad.find(KeyGroup::General, pair(North, West)),
        Some((EndTurn, 1))
    );
    // A chord moves like a button.
    assert_eq!(
        pad.bind(Confirm, 2, pair(West, North)),
        Ok(Some((EndTurn, 1)))
    );
    pad.clear(Confirm, 2);
    assert!(!pad.is_unmapped(EndTurn), "its own button is still there");
    // Only a chord on End turn: still mapped.
    let mut only = PadBindings::defaults(&def);
    for i in 0..SLOTS {
        only.clear(EndTurn, i);
    }
    assert_eq!(only.unmapped_required(), [EndTurn]);
    assert_eq!(only.bind(EndTurn, 0, pair(South, East)), Ok(None));
    assert!(only.unmapped_required().is_empty());
    let km = Keymap::new([], REPEAT).with_pad(only.pairs());
    let mut s = InputState::new(km);
    assert_eq!(frame(&mut s, &[(South, true), (East, true)], 0), [EndTurn]);
    // Saved and loaded with the rest.
    let mut keys = PlayerKeys::default();
    keys.set_pad(&def, only.clone());
    let saved = keys.to_ron();
    assert!(
        saved.contains("\"EndTurn\": [Some(\"South+East\"), None, None]"),
        "{saved}"
    );
    let (loaded, warnings) = PlayerKeys::from_ron(&saved, &def);
    assert_eq!(warnings, Vec::<String>::new());
    assert_eq!(loaded.pad_bindings(&def), only);
    assert_eq!(loaded.keymap(&def, RIGHT_HANDED).chord(), def.chord);
}

/// A press or release of `A` or `B`, or a frame passing.
#[derive(Debug, Clone, Copy)]
enum Step {
    Down(Button),
    Up(Button),
    Wait(u32),
}

/// Runs `steps` (each press and release applied, then an update of 0 ms;
/// a wait is an update of its length) and then lets 2 s pass in frames of
/// 33 ms, releasing everything first. Returns every action.
fn run(steps: &[Step]) -> Vec<Action> {
    let mut s = playdate();
    let mut actions = Vec::new();
    for &step in steps {
        match step {
            Step::Down(b) => s.pad_down(b, PLAYDATE),
            Step::Up(b) => s.pad_up(b),
            Step::Wait(dt) => actions.extend(s.update(ms(dt))),
        }
    }
    actions.extend(s.update(0.0));
    s.pad_up(South);
    s.pad_up(East);
    actions.extend(idle(&mut s, 60, 33));
    actions
}

/// Frame times adding up to less than the chord window.
fn inside_window() -> impl Strategy<Value = Vec<u32>> {
    prop::collection::vec(1..WINDOW_MS, 0..4).prop_filter("inside the window", |dts| {
        dts.iter().sum::<u32>() < WINDOW_MS
    })
}

proptest! {
    /// The ticket: any order and timing of A and B within the window
    /// gives exactly one End turn and no Confirm or Cancel, however long
    /// they are held and in whichever order they are let go.
    #[test]
    fn a_and_b_within_the_window_end_the_turn_once(
        b_first in any::<bool>(),
        first_frame in 0..500u32,
        between in inside_window(),
        held in prop::collection::vec(1..400u32, 0..4),
        a_up_first in any::<bool>(),
    ) {
        let (first, second) = if b_first { (East, South) } else { (South, East) };
        let mut steps = vec![Step::Down(first), Step::Wait(first_frame)];
        steps.extend(between.into_iter().map(Step::Wait));
        steps.push(Step::Down(second));
        steps.extend(held.into_iter().map(Step::Wait));
        let ups = if a_up_first { [South, East] } else { [East, South] };
        steps.extend(ups.map(Step::Up));
        prop_assert_eq!(run(&steps), vec![EndTurn]);
    }

    /// B tapped or held alone, for any time in any frames, gives one
    /// Cancel and nothing else.
    #[test]
    fn b_alone_always_gives_one_cancel(frames in prop::collection::vec(0..300u32, 0..8)) {
        let mut steps = vec![Step::Down(East)];
        steps.extend(frames.into_iter().map(Step::Wait));
        steps.push(Step::Up(East));
        prop_assert_eq!(run(&steps), vec![Cancel]);
    }

    /// A tapped or held alone gives one Confirm and nothing else.
    #[test]
    fn a_alone_always_gives_one_confirm(frames in prop::collection::vec(0..300u32, 0..8)) {
        let mut steps = vec![Step::Down(South)];
        steps.extend(frames.into_iter().map(Step::Wait));
        steps.push(Step::Up(South));
        prop_assert_eq!(run(&steps), vec![Confirm]);
    }

    /// Whatever A and B do, every press of either ends up as exactly one
    /// action, or as part of exactly one End turn.
    #[test]
    fn every_press_gives_one_action_or_one_end_turn(
        steps in prop::collection::vec(
            prop_oneof![
                prop::sample::select(vec![South, East]).prop_map(Step::Down),
                prop::sample::select(vec![South, East]).prop_map(Step::Up),
                (0..200u32).prop_map(Step::Wait),
            ],
            0..30,
        )
    ) {
        let actions = run(&steps);
        let mut down = [false; 2];
        let mut presses = 0;
        for &step in &steps {
            if let Step::Down(b) = step {
                let i = usize::from(b == East);
                if !down[i] {
                    presses += 1;
                }
                down[i] = true;
            } else if let Step::Up(b) = step {
                down[usize::from(b == East)] = false;
            }
        }
        let ends = actions.iter().filter(|&&a| a == EndTurn).count();
        let singles = actions.iter().filter(|&&a| a == Confirm || a == Cancel).count();
        prop_assert!(actions.iter().all(|a| [EndTurn, Confirm, Cancel].contains(a)));
        prop_assert!(singles + ends <= presses, "{:?} from {} presses", actions, presses);
        prop_assert!(singles + 2 * ends >= presses, "{:?} from {} presses", actions, presses);
    }
}

/// Help text names a chord's two buttons as the pad labels them, joined by
/// the language's word.
#[test]
fn help_text_names_both_buttons_of_a_chord() {
    use super::help_keys::{CHORD_JOIN, all_key_names, key_name};
    let km = playdate_keymap();
    let keys = HelpKeys::new(&km, Device::Pad(PLAYDATE));
    assert_eq!(CHORD_JOIN, "+");
    assert_eq!(key_name(keys, EndTurn), "A+B");
    assert_eq!(key_name(keys, PrevUnit), "B+←");
    assert_eq!(key_name(keys, NextUnit), "B+→");
    assert_eq!(key_name(keys, Confirm), "A");
    let worded = keys.chord_joined(" & ");
    assert_eq!(key_name(worded, EndTurn), "A & B");
    assert_eq!(
        key_name(worded.of(&km), EndTurn),
        "A & B",
        "kept for other bindings"
    );
    let both = km
        .clone()
        .with_pad([(pair(South, East), EndTurn), (West.into(), EndTurn)]);
    assert_eq!(all_key_names(keys.of(&both), EndTurn), "A+B/X");
    assert_eq!(
        PadKind::PlayStation.chord_name(pair(East, South), "+"),
        "\u{2715}+\u{25ef}"
    );
    assert_eq!(PadKind::Xbox.chord_name(West.into(), "+"), "X");
    // The keyboard is untouched.
    assert_eq!(
        key_name(keys.on_keyboard(), EndTurn),
        super::help_keys::NOT_MAPPED
    );
}

/// A press the game takes for itself (it ended the title's wait) gives
/// nothing, even when a chord made it wait.
#[test]
fn a_taken_press_gives_nothing() {
    for button in [South, East] {
        let mut s = playdate();
        s.pad_down(button, PLAYDATE);
        s.pad_taken(button);
        assert_eq!(idle(&mut s, 10, 33), [], "{button:?} held");
        assert_eq!(
            frame(&mut s, &[(button, false)], 33),
            [],
            "{button:?} let go"
        );
        assert_eq!(
            frame(&mut s, &[(button, true), (button, false)], 0).len(),
            1
        );
    }
    // A button that acts on press: nothing to take back.
    let mut s = playdate();
    s.pad_down(DpadUp, PLAYDATE);
    s.pad_taken(DpadUp);
    assert_eq!(s.update(0.0), [CursorUp]);
}

/// The window is the keymap's, not a fixed 80 ms.
#[test]
fn the_window_is_the_keymaps() {
    let slow = ChordDef { window_ms: 300 };
    let km = playdate_keymap().with_chord(slow);
    assert_eq!(km.chord(), slow);
    let mut s = InputState::new(km);
    assert_eq!(frame(&mut s, &[(South, true)], 0), []);
    assert_eq!(frame(&mut s, &[], 200), [], "still inside 300 ms");
    assert_eq!(frame(&mut s, &[(East, true)], 0), [EndTurn]);
    let mut s = InputState::new(playdate_keymap().with_chord(slow));
    frame(&mut s, &[(South, true)], 0);
    assert_eq!(frame(&mut s, &[], 299), []);
    assert_eq!(frame(&mut s, &[], 1), [Confirm]);
}

/// A chord that has ended leaves nothing behind: ← held on its own later
/// keeps moving the cursor when B is tapped, whichever of B and ← was let
/// go first after the chord.
#[test]
fn an_ended_chord_never_touches_a_later_press() {
    for b_first in [false, true] {
        let mut s = playdate();
        frame(&mut s, &[(East, true)], 0);
        assert_eq!(frame(&mut s, &[(DpadLeft, true)], 0), [PrevUnit]);
        let ups = if b_first {
            [(East, false), (DpadLeft, false)]
        } else {
            [(DpadLeft, false), (East, false)]
        };
        assert_eq!(frame(&mut s, &ups, 0), []);
        // ← alone, held; B tapped while it is held.
        assert_eq!(frame(&mut s, &[(DpadLeft, true)], 0), [CursorLeft]);
        assert_eq!(frame(&mut s, &[(East, true)], 0), []);
        assert_eq!(frame(&mut s, &[(East, false)], 0), [Cancel]);
        assert!(s.is_held(CursorLeft), "b_first {b_first}");
        assert_eq!(s.update(ms(300)), [CursorLeft], "b_first {b_first}");
    }
}
