use std::collections::VecDeque;

use shuyi_core::tick::{SeatInput, TickInput, TickSim};

use super::{Advance, RoomClient};
use crate::PeerId;
use crate::room::testing::{DotEvent, Dots, OUT, RIGHT, UP};
use crate::room::wire::{ClientMsg, Frame};
use crate::room::{HASH_EVERY, Joined, Outbox, Room, RoomGame, SeatKind};

/// The peer the client is.
const ME: PeerId = PeerId(1);

/// A press of `action`, held.
fn press(action: shuyi_core::tick::ActionId) -> SeatInput {
    SeatInput::new(vec![action], vec![action], vec![])
}

/// Steps `room` once and gives the bytes of the frames it wrote to `peer`.
fn step(room: &mut Room<Dots>, peer: PeerId) -> Vec<Vec<u8>> {
    let mut out = Outbox::new();
    room.step(&mut out);
    out.frames
        .iter()
        .filter(|(p, _)| *p == peer)
        .map(|(_, frame)| frame.encode())
        .collect()
}

/// Hands every frame of `frames` to `client`.
fn feed(client: &mut RoomClient<Dots>, frames: &[Vec<u8>]) {
    for bytes in frames {
        client.on_frame(bytes).expect("the room's frames decode");
    }
}

/// Steps `room` once and hands the client `ME`'s frames.
fn step_and_feed(room: &mut Room<Dots>, client: &mut RoomClient<Dots>) {
    let frames = step(room, ME);
    feed(client, &frames);
}

/// A room of seed `seed` that `ME` joined, and its client.
fn joined(seed: u64) -> (Room<Dots>, RoomClient<Dots>) {
    let mut room = Room::new(Dots, seed);
    assert_eq!(room.join(ME), Ok(Joined::Seat(0)));
    (room, RoomClient::new(Dots))
}

/// A room of eight players, peers 10 to 17, all of whom but seat 7's
/// press out before step 1, so seat 7 wins the round at step 1.
fn seat_7_wins(seed: u64) -> (Room<Dots>, Vec<PeerId>) {
    let mut room = Room::new(Dots, seed);
    let peers: Vec<PeerId> = (10..18).map(PeerId).collect();
    for peer in &peers {
        assert!(room.join(*peer).is_ok());
    }
    for peer in &peers[..7] {
        room.receive(*peer, ClientMsg::Input(press(OUT)));
    }
    (room, peers)
}

/// Whether the client's sim is the room's.
fn equal(room: &Room<Dots>, client: &RoomClient<Dots>) -> bool {
    client.sim().map(TickSim::hash) == Some(room.sim().hash())
}

#[test]
fn the_buffer_starts_at_two_steps_and_again_after_it_ran_dry() {
    let (mut room, mut client) = joined(1);
    // The catch-up and step 1: one step waits.
    step_and_feed(&mut room, &mut client);
    assert_eq!(client.steps_behind(), 1);
    assert_eq!(client.advance(), Advance::Waiting);
    // Two wait: one is applied.
    step_and_feed(&mut room, &mut client);
    assert_eq!(client.advance(), Advance::Stepped(1));
    assert_eq!(client.steps_behind(), 1);
    // Three wait: it catches up by two.
    step_and_feed(&mut room, &mut client);
    step_and_feed(&mut room, &mut client);
    assert_eq!(client.steps_behind(), 3);
    assert_eq!(client.advance(), Advance::Stepped(2));
    assert_eq!(client.advance(), Advance::Stepped(1));
    assert!(equal(&room, &client));
    // It ran dry: it waits for two again.
    assert_eq!(client.advance(), Advance::Waiting);
    step_and_feed(&mut room, &mut client);
    assert_eq!(client.advance(), Advance::Waiting);
    step_and_feed(&mut room, &mut client);
    assert_eq!(client.advance(), Advance::Stepped(1));
    assert_eq!(client.advance(), Advance::Stepped(1));
    assert!(equal(&room, &client));
    assert_eq!(client.steps_behind(), 0);
}

#[test]
fn a_catch_up_mid_round_reaches_the_rooms_sim_and_its_next_check_passes() {
    let mut room = Room::new(Dots, 2);
    for _ in 0..20 {
        step(&mut room, ME);
    }
    let before = room.sim().clone();
    let place = room.join(ME).expect("a bot seat is alive");
    let mut client = RoomClient::new(Dots);
    assert_eq!(client.place(), None);
    assert_eq!(client.round(), 0);
    step_and_feed(&mut room, &mut client);
    // At once, before step 21 is applied.
    assert_eq!(client.sim(), Some(&before));
    assert_eq!(client.place(), Some(place));
    assert_eq!(client.my_seat(), place.seat());
    assert_eq!(client.round(), 1);
    assert_eq!(client.wins(), &[0; Dots::SEATS]);
    assert_eq!(client.roster().humans(), 1);
    assert_eq!(client.steps_behind(), 1);
    // Past the next hash, at step 30, every advance steps.
    for _ in 21..2 * HASH_EVERY + 2 {
        room.receive(ME, ClientMsg::Alive);
        step_and_feed(&mut room, &mut client);
        assert_eq!(client.advance(), Advance::Stepped(1));
    }
    assert_eq!(client.sim().map(|sim| sim.steps), Some(2 * HASH_EVERY + 1));
    assert_eq!(client.steps_behind(), 1);
}

#[test]
fn frames_that_are_not_steps_apply_in_order_after_their_step() {
    let (mut room, peers) = seat_7_wins(3);
    let me = peers[7];
    let mut client = RoomClient::new(Dots);
    feed(&mut client, &step(&mut room, me));
    // The step and the round's end wait; the end isn't applied yet.
    assert_eq!(client.advance(), Advance::Waiting);
    assert_eq!(client.wins(), &[0; Dots::SEATS]);
    // In the pause seat 1's player leaves, then the next round starts.
    room.leave(peers[1]);
    for _ in 0..=Dots::PAUSE_STEPS {
        feed(&mut client, &step(&mut room, me));
    }
    assert_eq!(room.stats().round, 2);
    assert_eq!(client.steps_behind(), 2);
    // Step 1, then the round's end, the roster and the next round's
    // start: the next round's step 1 waits.
    assert_eq!(client.advance(), Advance::Stepped(1));
    assert_eq!(client.wins(), &[0, 0, 0, 0, 0, 0, 0, 1]);
    assert_eq!(client.roster().seats[1], SeatKind::Bot);
    assert_eq!(client.round(), 2);
    assert_eq!(client.sim().map(|sim| sim.steps), Some(0));
    assert_eq!(client.steps_behind(), 1);
    feed(&mut client, &step(&mut room, me));
    assert_eq!(client.advance(), Advance::Stepped(1));
    assert_eq!(client.advance(), Advance::Stepped(1));
    assert!(equal(&room, &client));
}

#[test]
fn a_step_out_of_order_asks_for_a_catch_up() {
    let (mut room, mut client) = joined(5);
    step_and_feed(&mut room, &mut client);
    // Step 2 never reaches the client.
    step(&mut room, ME);
    step_and_feed(&mut room, &mut client);
    step_and_feed(&mut room, &mut client);
    let Advance::Resync(bytes) = client.advance() else {
        panic!("step 3 after step 1 is no next step");
    };
    assert_eq!(ClientMsg::decode(&bytes), Ok(ClientMsg::Resync));
    // Until the catch-up, it waits and keeps no frame.
    step_and_feed(&mut room, &mut client);
    assert_eq!(client.steps_behind(), 0);
    assert_eq!(client.advance(), Advance::Waiting);
    room.receive(ME, ClientMsg::decode(&bytes).unwrap());
    step_and_feed(&mut room, &mut client);
    step_and_feed(&mut room, &mut client);
    assert_eq!(client.advance(), Advance::Stepped(1));
    assert_eq!(client.advance(), Advance::Stepped(1));
    assert!(equal(&room, &client));
}

#[test]
fn a_corrupted_sim_asks_for_a_catch_up_at_the_next_hash_and_is_equal_after_it() {
    let (mut room, mut client) = joined(6);
    step_and_feed(&mut room, &mut client);
    step_and_feed(&mut room, &mut client);
    assert_eq!(client.advance(), Advance::Stepped(1));
    client.sim.as_mut().expect("caught up").steps += 1000;
    let mut resync = None;
    for _ in 0..HASH_EVERY {
        room.receive(ME, ClientMsg::Alive);
        step_and_feed(&mut room, &mut client);
        match client.advance() {
            Advance::Stepped(1) => {}
            Advance::Resync(bytes) => {
                resync = Some(bytes);
                break;
            }
            other => panic!("{other:?} before the hash"),
        }
    }
    // Step 15 carries the hash, and is the step that differs.
    assert_eq!(room.stats().step, HASH_EVERY + 1);
    let bytes = resync.expect("a resync at the hash");
    assert_eq!(client.advance(), Advance::Waiting);
    room.receive(ME, ClientMsg::decode(&bytes).unwrap());
    step_and_feed(&mut room, &mut client);
    assert_eq!(client.sim().map(|sim| sim.steps), Some(HASH_EVERY + 1));
    // The next check, at step 30, passes.
    for _ in 0..HASH_EVERY {
        room.receive(ME, ClientMsg::Alive);
        step_and_feed(&mut room, &mut client);
        assert_eq!(client.advance(), Advance::Stepped(1));
    }
    assert_eq!(client.advance(), Advance::Stepped(1));
    assert_eq!(client.steps_behind(), 0);
    assert_eq!(room.stats().step, 2 * HASH_EVERY + 2);
    assert!(equal(&room, &client));
}

#[test]
fn events_hold_each_applied_steps_events_once_and_a_catch_ups_none() {
    let mut room = Room::new(Dots, 7);
    let (first, second) = (PeerId(2), PeerId(3));
    assert_eq!(room.join(first), Ok(Joined::Seat(0)));
    assert_eq!(room.join(second), Ok(Joined::Seat(1)));
    // Seat 0 goes out at step 1, before the client joins.
    room.receive(first, ClientMsg::Input(press(OUT)));
    for _ in 0..5 {
        step(&mut room, ME);
    }
    assert!(room.join(ME).is_ok());
    let mut client = RoomClient::new(Dots);
    let mut mirror = None;
    let mut waiting = VecDeque::new();
    let mut seen = Vec::new();
    for n in 0..12 {
        if n == 3 {
            // Seat 1 goes out at the client's fourth step.
            room.receive(second, ClientMsg::Input(press(OUT)));
        }
        for bytes in step(&mut room, ME) {
            match Frame::decode::<Dots>(&bytes).unwrap() {
                Frame::Joined { .. } => {}
                Frame::Step { inputs, .. } => waiting.push_back(inputs),
                other => panic!("no {other:?} in this round"),
            }
            client.on_frame(&bytes).unwrap();
        }
        let mirror = mirror.get_or_insert_with(|| client.sim().expect("caught up").clone());
        let mut want = Vec::new();
        if let Advance::Stepped(steps) = client.advance() {
            for _ in 0..steps {
                let inputs: TickInput = waiting.pop_front().unwrap();
                want.extend(mirror.step(&inputs));
            }
        }
        assert_eq!(client.events(), want);
        seen.extend_from_slice(client.events());
    }
    assert_eq!(seen.iter().filter(|e| **e == DotEvent::Out(0)).count(), 0);
    assert_eq!(seen.iter().filter(|e| **e == DotEvent::Out(1)).count(), 1);
}

#[test]
fn input_is_sent_on_each_press_and_on_a_new_held_set() {
    let mut client = RoomClient::new(Dots);
    let none = SeatInput::new(vec![], vec![], vec![]);
    assert_eq!(client.input(&none), None);
    let up = press(UP);
    let sent = Some(ClientMsg::Input(up.clone()).encode());
    assert_eq!(client.input(&up), sent);
    // The same press on the next step is sent again.
    assert_eq!(client.input(&up), sent);
    let held = SeatInput::new(vec![], vec![UP], vec![]);
    assert_eq!(client.input(&held), None);
    let both = SeatInput::new(vec![], vec![UP, RIGHT], vec![]);
    assert_eq!(
        client.input(&both),
        Some(ClientMsg::Input(both.clone()).encode())
    );
    let release = SeatInput::new(vec![], vec![RIGHT], vec![UP]);
    assert!(client.input(&release).is_some());
    assert_eq!(
        client.input(&SeatInput::new(vec![], vec![RIGHT], vec![])),
        None
    );
    assert_eq!(client.input(&none), Some(ClientMsg::Input(none).encode()));
    assert_eq!(ClientMsg::decode(&client.leave()), Ok(ClientMsg::Leave));
}

#[test]
fn alive_is_sent_after_a_second_of_advances_with_nothing_sent() {
    let mut client = RoomClient::new(Dots);
    let alive = Some(ClientMsg::Alive.encode());
    for _ in 1..Dots::STEPS_PER_SECOND {
        client.advance();
        assert_eq!(client.alive(), None);
    }
    client.advance();
    assert_eq!(client.alive(), alive);
    assert_eq!(client.alive(), None);
    // An input counts as sent.
    for _ in 1..Dots::STEPS_PER_SECOND {
        client.advance();
    }
    assert!(client.input(&press(UP)).is_some());
    client.advance();
    assert_eq!(client.alive(), None);
    for _ in 1..Dots::STEPS_PER_SECOND {
        client.advance();
    }
    assert_eq!(client.alive(), alive);
}

#[test]
fn a_watcher_has_no_seat_until_the_next_rounds_roster() {
    let (mut room, peers) = seat_7_wins(8);
    step(&mut room, ME);
    // Seat 1's dot is out: a joiner watches.
    room.leave(peers[1]);
    assert_eq!(room.join(ME), Ok(Joined::Watching));
    let mut client = RoomClient::new(Dots);
    step_and_feed(&mut room, &mut client);
    assert_eq!(client.place(), Some(Joined::Watching));
    assert_eq!(client.my_seat(), None);
    // The pause: no seat until the next round's roster is applied.
    while client.round() < 2 {
        assert_eq!(client.my_seat(), None);
        step_and_feed(&mut room, &mut client);
        assert_eq!(client.advance(), Advance::Waiting);
    }
    assert_eq!(client.place(), Some(Joined::Seat(1)));
    assert_eq!(client.my_seat(), Some(1));
    assert_eq!(client.roster().seats[1], SeatKind::Human);
    assert_eq!(client.round(), 2);
}

#[test]
fn frames_before_the_catch_up_are_dropped_and_closing_closes() {
    let mut room = Room::new(Dots, 9);
    let other = PeerId(2);
    assert!(room.join(other).is_ok());
    let mut client = RoomClient::new(Dots);
    step(&mut room, other);
    for _ in 0..3 {
        // Steps, which come before any catch-up for this client.
        feed(&mut client, &step(&mut room, other));
    }
    assert_eq!(client.steps_behind(), 0);
    assert_eq!(client.sim(), None);
    assert_eq!(client.advance(), Advance::Waiting);
    assert!(client.on_frame(&[0x7F]).is_err());
    assert_eq!(client.on_frame(&Frame::Closing.encode()), Ok(()));
    assert_eq!(client.advance(), Advance::Closed);
    assert_eq!(client.advance(), Advance::Closed);
}
