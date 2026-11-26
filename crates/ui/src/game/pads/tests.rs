//! Several pads as one, at the level of [`Game`] (ticket 2412): the merge
//! `Pads` did before pads had ids, now done where the events arrive.

use alloc::collections::BTreeMap;
use alloc::rc::Rc;
use core::cell::RefCell;

use proptest::prelude::*;
use shuyi_assets::keymap::{Button, StickDef};

use crate::frame::glyph_buffer::GlyphBuffer;
use crate::input::FrameInput;
use crate::input::pad::{ButtonSet, PadId, PadKind, PadState, Pads};
use crate::prelude::*;
use crate::screen::{Screen, Transition};
use crate::toy::{Action, Toy, ToyCtx, ctx};
use crate::{Game, GameHooks, RawInputEvent};

use Button::{LeftStickDown, North, South, Start, West};

/// What a [`Spy`] saw, frame by frame.
type Seen = Rc<RefCell<Vec<FrameInput<Action>>>>;

/// Records what the screen saw.
struct Spy(Seen);

impl Screen<Toy> for Spy {
    fn name(&self) -> &'static str {
        "spy"
    }
    fn update(&mut self, _: &mut ToyCtx, input: &FrameInput<Action>) -> Transition<Toy> {
        self.0.borrow_mut().push(input.clone());
        Transition::None
    }
    fn draw(&self, _: &ToyCtx, _: &mut GlyphBuffer) {}
}

fn spied() -> (Game<Toy>, Seen) {
    let seen = Rc::default();
    let game = Game::new(ctx(), Box::new(Spy(Rc::clone(&seen))), GameHooks::default());
    (game, seen)
}

fn stick() -> StickDef {
    StickDef::DEFAULT
}

/// A pad holding `buttons`, sticks centred.
fn holding(buttons: &[Button]) -> PadState {
    PadState {
        buttons: buttons.iter().copied().collect(),
        ..PadState::default()
    }
}

/// Runs a frame of `game` with what `pads` reports for `connected`.
fn poll(game: &mut Game<Toy>, pads: &mut Pads, connected: &[(PadId, PadKind, PadState)]) {
    let events = RawInputEvent::from_pads(pads, connected);
    game.frame(&events, 0.0);
}

#[test]
fn two_pads_holding_one_button_are_one_press() {
    let (mut game, seen) = spied();
    let mut pads = Pads::new(stick());
    let a = |state| (3, PadKind::Xbox, state);
    let b = |state| (7, PadKind::PlayStation, state);
    poll(
        &mut game,
        &mut pads,
        &[a(holding(&[South])), b(holding(&[]))],
    );
    // The second pad joins in: no second press.
    poll(
        &mut game,
        &mut pads,
        &[a(holding(&[South])), b(holding(&[South]))],
    );
    // The first lets go: still held by the second.
    poll(
        &mut game,
        &mut pads,
        &[a(holding(&[])), b(holding(&[South]))],
    );
    poll(&mut game, &mut pads, &[a(holding(&[])), b(holding(&[]))]);
    // Different buttons on different pads are separate presses, and
    // unplugging one pad releases only what the other doesn't hold.
    poll(
        &mut game,
        &mut pads,
        &[a(holding(&[South, West])), b(holding(&[South, North]))],
    );
    poll(&mut game, &mut pads, &[b(holding(&[South, North]))]);
    let seen = seen.borrow();
    let pressed: Vec<Vec<Button>> = seen.iter().map(|f| f.pressed_buttons().to_vec()).collect();
    let released: Vec<Vec<Button>> = seen.iter().map(|f| f.released_buttons().to_vec()).collect();
    assert_eq!(
        pressed,
        [
            vec![South],
            vec![],
            vec![],
            vec![],
            vec![South, West, North],
            vec![]
        ]
    );
    assert_eq!(
        released,
        [vec![], vec![], vec![], vec![South], vec![], vec![West]]
    );
    let confirms: Vec<usize> = seen
        .iter()
        .map(|f| f.actions.iter().filter(|&&a| a == Action::Confirm).count())
        .collect();
    assert_eq!(confirms, [1, 0, 0, 0, 1, 0]);
    let held: Vec<bool> = seen.iter().map(|f| f.is_held(Action::Confirm)).collect();
    assert_eq!(held, [true, true, true, false, true, true]);
}

#[test]
fn a_removed_pad_releases_what_it_held() {
    let (mut game, seen) = spied();
    let mut pads = Pads::new(stick());
    let held = PadState {
        buttons: [South, Start].into_iter().collect(),
        left_stick: (0.0, 1.0),
        ..PadState::default()
    };
    let xbox = |state| (0, PadKind::Xbox, state);
    poll(&mut game, &mut pads, &[xbox(held)]);
    // Unplugged mid-press.
    poll(&mut game, &mut pads, &[]);
    poll(&mut game, &mut pads, &[]);
    // Plugged back in still holding: pressed again.
    poll(&mut game, &mut pads, &[xbox(held)]);
    let seen = seen.borrow();
    let all = [South, Start, LeftStickDown].to_vec();
    assert_eq!(seen[0].pressed_buttons(), all);
    assert_eq!(seen[1].released_buttons(), all);
    assert!(!seen[1].is_held(Action::Confirm));
    assert_eq!(seen[2].released_buttons(), []);
    assert_eq!(seen[3].pressed_buttons(), all);
}

/// A removal alone, as a front end that sends no releases first would:
/// what the pad held is let go, unless another pad holds it.
#[test]
fn a_removal_lets_go_of_what_only_that_pad_held() {
    let (mut game, seen) = spied();
    let down = |b, pad| RawInputEvent::PadDown(b, PadKind::Xbox, pad);
    game.frame(&[down(South, 1), down(West, 1), down(South, 2)], 0.0);
    game.frame(&[RawInputEvent::PadRemoved(1)], 0.0);
    game.frame(
        &[RawInputEvent::PadRemoved(2), RawInputEvent::PadRemoved(2)],
        0.0,
    );
    let seen = seen.borrow();
    assert_eq!(seen[0].pressed_buttons(), [South, West]);
    assert_eq!(seen[1].released_buttons(), [West]);
    assert!(seen[1].is_held(Action::Confirm));
    assert_eq!(seen[2].released_buttons(), [South]);
    assert!(!seen[2].is_held(Action::Confirm));
}

/// A button let go on one pad and pressed on another in one poll stays
/// held: one stream of every pad saw no change.
#[test]
fn a_button_passed_from_pad_to_pad_in_one_poll_stays_held() {
    let (mut game, seen) = spied();
    let mut pads = Pads::new(stick());
    poll(
        &mut game,
        &mut pads,
        &[(0, PadKind::Xbox, holding(&[South]))],
    );
    let swapped = [
        (0, PadKind::Xbox, holding(&[])),
        (1, PadKind::Xbox, holding(&[South])),
    ];
    poll(&mut game, &mut pads, &swapped);
    poll(&mut game, &mut pads, &[(1, PadKind::Xbox, holding(&[]))]);
    let seen = seen.borrow();
    assert_eq!(seen[0].actions, [Action::Confirm]);
    assert!(seen[1].actions.is_empty(), "{:?}", seen[1].actions);
    assert_eq!(seen[1].pressed_buttons(), []);
    assert_eq!(seen[1].released_buttons(), []);
    assert!(seen[1].is_held(Action::Confirm));
    assert_eq!(seen[2].released_buttons(), [South]);
}

/// One pad's own press and release in one poll (a crank's notch) is a
/// press and a release, as it was.
#[test]
fn one_pads_tap_in_one_poll_is_a_press_and_a_release() {
    let (mut game, seen) = spied();
    let tap = [
        RawInputEvent::PadDown(South, PadKind::Xbox, 0),
        RawInputEvent::PadUp(South, 0),
    ];
    game.frame(&[tap, tap].concat(), 0.0);
    let seen = seen.borrow();
    assert_eq!(seen[0].pressed_buttons(), [South, South]);
    assert_eq!(seen[0].released_buttons(), [South, South]);
    assert!(!seen[0].is_held(Action::Confirm));
}

/// The merge `Pads::update` did before ticket 2412, kept here to test
/// against: every pad's buttons as one set (the old `Pads::down`), its
/// changes as one stream of events on pad 0, releases then presses in
/// button order, each press with the kind of the first pad of `connected`
/// holding it (the old `Pads::kind_holding`). Each pad's buttons come from
/// today's `Pads`, whose sticks and swap didn't change.
struct OldMerge {
    pads: Pads,
    /// Each pad's buttons held.
    held: BTreeMap<PadId, ButtonSet>,
}

impl OldMerge {
    fn new() -> Self {
        Self {
            pads: Pads::new(stick()),
            held: BTreeMap::new(),
        }
    }

    fn down(&self) -> ButtonSet {
        (self.held.values()).fold(ButtonSet::EMPTY, |all, &pad| all.union(pad))
    }

    fn events(&mut self, connected: &[(PadId, PadKind, PadState)]) -> Vec<RawInputEvent> {
        let before = self.down();
        for event in self.pads.update(connected) {
            match event {
                RawInputEvent::PadDown(b, _, pad) => self.held.entry(pad).or_default().insert(b),
                RawInputEvent::PadUp(b, pad) => self.held.entry(pad).or_default().remove(b),
                RawInputEvent::PadRemoved(pad) => drop(self.held.remove(&pad)),
                other => panic!("not a pad's: {other:?}"),
            }
        }
        let after = self.down();
        let kind = |b| {
            let holds = |id: &PadId| self.held.get(id).is_some_and(|set| set.contains(b));
            let pad = connected.iter().find(|(id, _, _)| holds(id));
            pad.map_or_default(|&(_, kind, _)| kind)
        };
        let released = before.iter().filter(|&b| !after.contains(b));
        let pressed = after.iter().filter(|&b| !before.contains(b));
        released
            .map(|b| RawInputEvent::PadUp(b, 0))
            .chain(pressed.map(|b| RawInputEvent::PadDown(b, kind(b), 0)))
            .collect()
    }
}

fn arb_state() -> impl Strategy<Value = PadState> {
    let axis = || prop_oneof![Just(0.0f32), -1.0f32..=1.0];
    (0u32..(1 << 16), axis(), axis(), axis(), axis())
        .prop_map(|(buttons, lx, ly, rx, ry)| PadState::from_standard(buttons, [lx, ly, rx, ry]))
}

/// The pads plugged in in one frame: up to four, of any kind, each id
/// once and in id order, as gilrs and the browser list them (the old
/// merge took a press's kind from the first pad listed holding it; now it
/// is the lowest id's, the same pad when the list is in id order).
fn arb_connected() -> impl Strategy<Value = Vec<(PadId, PadKind, PadState)>> {
    let kind = prop::sample::select(PadKind::ALL.to_vec());
    let pads = prop::collection::btree_map(0usize..4, (kind, arb_state()), 0..=4);
    pads.prop_map(|pads| pads.into_iter().map(|(id, (k, s))| (id, k, s)).collect())
}

proptest! {
    /// Ticket 2412: whatever up to four pads do, the game sees exactly what
    /// it saw when `Pads` merged them: the same input in every frame
    /// (actions, held actions, buttons down and up, the device) and the
    /// same input state at the end. Frame times vary, so held directions
    /// repeat.
    #[test]
    fn a_game_sees_what_the_old_merge_gave(
        frames in prop::collection::vec((arb_connected(), 0u8..4), 0..30),
    ) {
        let (mut game, seen) = spied();
        let (mut old_game, old_seen) = spied();
        let mut pads = Pads::new(stick());
        let mut old = OldMerge::new();
        for (connected, tenths) in &frames {
            let dt = f32::from(*tenths) / 10.0;
            game.frame(&RawInputEvent::from_pads(&mut pads, connected), dt);
            old_game.frame(&old.events(connected), dt);
            prop_assert_eq!(game.ctx().device, old_game.ctx().device);
        }
        prop_assert_eq!(&*seen.borrow(), &*old_seen.borrow());
        let held = |g: &Game<Toy>| {
            Action::ALL.iter().copied().filter(|&a| g.input().is_held(a)).collect::<Vec<_>>()
        };
        prop_assert_eq!(held(&game), held(&old_game));
    }
}
