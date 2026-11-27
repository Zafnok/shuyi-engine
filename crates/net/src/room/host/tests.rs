use shuyi_core::SimRng;
use shuyi_core::tick::{ActionId, SeatInput, TickInput, TickSim};

use super::{Outbox, Room};
use crate::PeerId;
use crate::room::testing::{Dots, DotsSim, OUT, RIGHT, UP};
use crate::room::wire::{ClientMsg, Frame, RoundStart};
use crate::room::{Full, HASH_EVERY, Joined, RoomGame, Seat, SeatKind};

/// The steps of silence after which a seat is the bot's.
const SILENCE: u32 = 5 * Dots::STEPS_PER_SECOND;

/// A press of `action`, held.
fn press(action: ActionId) -> SeatInput {
    SeatInput::new(vec![action], vec![action], vec![])
}

/// Steps `room` once and gives what it wrote.
fn step(room: &mut Room<Dots>) -> Outbox {
    let mut out = Outbox::new();
    room.step(&mut out);
    out
}

/// The frames of `out` that go to `peer`.
fn to(out: &Outbox, peer: PeerId) -> Vec<&Frame> {
    out.frames
        .iter()
        .filter(|(p, _)| *p == peer)
        .map(|(_, f)| f)
        .collect()
}

/// The inputs of the step frame of `out`, which every peer gets alike.
fn inputs(out: &Outbox) -> &TickInput {
    out.frames
        .iter()
        .find_map(|(_, frame)| match frame {
            Frame::Step { inputs, .. } => Some(inputs),
            _ => None,
        })
        .expect("a step frame")
}

/// A room whose seats are all `peers`', seated before its first step.
fn full_room(seed: u64) -> (Room<Dots>, Vec<PeerId>) {
    let mut room = Room::new(Dots, seed);
    let peers: Vec<PeerId> = (1..=8).map(PeerId).collect();
    for peer in &peers {
        assert!(matches!(room.join(*peer), Ok(Joined::Seat(_))));
    }
    (room, peers)
}

/// The sim a catch-up builds: the round's start, then each step.
fn replay(start: &RoundStart, steps: &[TickInput]) -> DotsSim {
    let mut sim = Dots.new_round(start.seed, start.round);
    for inputs in steps {
        sim.step(inputs);
    }
    sim
}

#[test]
fn a_join_takes_the_lowest_bot_seat_whose_rider_is_alive() {
    let (mut room, peers) = full_room(1);
    // Seats 0 and 3 go out, then their players leave.
    room.receive(peers[0], ClientMsg::Input(press(OUT)));
    room.receive(peers[3], ClientMsg::Input(press(OUT)));
    step(&mut room);
    room.leave(peers[0]);
    room.leave(peers[3]);
    room.leave(peers[5]);
    assert_eq!(room.join(PeerId(20)), Ok(Joined::Seat(5)));
    // No bot seat is alive now: the next joiner watches.
    assert_eq!(room.join(PeerId(21)), Ok(Joined::Watching));
    assert_eq!(room.stats().watchers, 1);
    // A peer already in gets its place back, and nothing changes.
    assert_eq!(room.join(PeerId(20)), Ok(Joined::Seat(5)));
    assert_eq!(room.join(PeerId(21)), Ok(Joined::Watching));
    assert_eq!(room.stats().watchers, 1);
    assert_eq!(room.place(PeerId(22)), None);
}

/// A round of seven players in seats 0 to 6, who keep talking, stepped
/// until seat 7's bot is out: then a joiner can only watch.
fn bots_out(seed: u64) -> (Room<Dots>, Vec<PeerId>) {
    let mut room = Room::new(Dots, seed);
    let peers: Vec<PeerId> = (1..=7).map(PeerId).collect();
    for peer in &peers {
        assert!(matches!(room.join(*peer), Ok(Joined::Seat(_))));
    }
    for _ in 0..Dots::MAX_ROUND_STEPS {
        if !Dots::seat_alive(room.sim(), 7) {
            break;
        }
        for peer in &peers {
            room.receive(*peer, ClientMsg::Alive);
        }
        step(&mut room);
    }
    assert!(!Dots::seat_alive(room.sim(), 7));
    assert_eq!(room.stats().round, 1);
    (room, peers)
}

#[test]
fn a_watcher_is_seated_when_the_next_round_starts() {
    let (mut room, peers) = full_room(2);
    // All but seat 7 go out: seat 7 wins at this step.
    for peer in &peers[..7] {
        room.receive(*peer, ClientMsg::Input(press(OUT)));
    }
    let out = step(&mut room);
    assert!(to(&out, peers[7]).contains(&&Frame::RoundOver {
        winner: Some(7),
        wins: vec![0, 0, 0, 0, 0, 0, 0, 1],
    }));
    // Between rounds, seat 1's player leaves; its dot is out, so a joiner
    // watches.
    room.leave(peers[1]);
    assert_eq!(room.join(PeerId(30)), Ok(Joined::Watching));
    let mut seated = None;
    for _ in 0..=Dots::PAUSE_STEPS {
        let out = step(&mut room);
        for frame in to(&out, PeerId(30)) {
            if let Frame::Roster { place, roster } = frame {
                seated = Some(*place);
                assert_eq!(roster.seats[1], SeatKind::Human);
            }
        }
    }
    assert_eq!(seated, Some(Joined::Seat(1)));
    assert_eq!(room.place(PeerId(30)), Some(Joined::Seat(1)));
    assert_eq!(room.stats().watchers, 0);
}

#[test]
fn a_room_of_players_and_watchers_is_full() {
    let (mut room, _) = bots_out(3);
    assert_eq!(room.join(PeerId(20)), Ok(Joined::Watching));
    assert_eq!(room.join(PeerId(21)), Err(Full));
    assert_eq!(
        Full.to_string(),
        "the room is full: players hold every seat"
    );
}

#[test]
fn a_leave_makes_the_seat_the_bots_at_the_next_step() {
    let (mut room, peers) = full_room(4);
    room.receive(peers[2], ClientMsg::Input(press(UP)));
    let out = step(&mut room);
    assert_eq!(inputs(&out).seats[2].held, [UP]);
    room.leave(peers[2]);
    assert_eq!(room.seats()[2], Seat::Bot);
    let out = step(&mut room);
    // The bot holds nothing, and the peer is sent nothing more.
    assert_eq!(inputs(&out).seats[2].held, []);
    assert_eq!(to(&out, peers[2]).len(), 0);
    assert!(to(&out, peers[0]).iter().any(|f| matches!(
        f,
        Frame::Roster { roster, .. } if roster.seats[2] == SeatKind::Bot
    )));
    assert_eq!(out.drop, []);
}

#[test]
fn five_seconds_of_silence_make_the_seat_the_bots_and_drop_the_peer() {
    let (mut room, peers) = full_room(5);
    for n in 1..=SILENCE {
        // Every peer but seat 0's keeps talking.
        for peer in &peers[1..] {
            room.receive(*peer, ClientMsg::Alive);
        }
        let out = step(&mut room);
        assert_eq!(room.seats()[0], Seat::Human(peers[0]), "step {n}");
        assert_eq!(out.drop, []);
    }
    let out = step(&mut room);
    assert_eq!(out.drop, [peers[0]]);
    assert_eq!(room.seats()[0], Seat::Bot);
    assert_eq!(to(&out, peers[0]).len(), 0);
    assert_eq!(room.stats().humans, 7);
}

#[test]
fn an_input_or_an_alive_keeps_a_seat() {
    let (mut room, peers) = full_room(6);
    for n in 0..3 * SILENCE {
        for (seat, peer) in peers.iter().enumerate() {
            if n % SILENCE == SILENCE - 1 {
                let msg = if seat % 2 == 0 {
                    ClientMsg::Alive
                } else {
                    ClientMsg::Input(SeatInput::default())
                };
                room.receive(*peer, msg);
            }
        }
        assert_eq!(step(&mut room).drop, []);
    }
    assert_eq!(room.stats().humans, 8);
}

#[test]
fn a_silent_watcher_is_dropped_too() {
    let (mut room, peers) = bots_out(7);
    assert_eq!(room.join(PeerId(40)), Ok(Joined::Watching));
    room.leave(PeerId(40));
    assert_eq!(room.stats().watchers, 0);
    assert_eq!(room.join(PeerId(41)), Ok(Joined::Watching));
    let mut dropped = Vec::new();
    for n in 0..=SILENCE {
        for peer in &peers {
            room.receive(*peer, ClientMsg::Alive);
        }
        let out = step(&mut room);
        if n < SILENCE {
            assert_eq!(room.place(PeerId(41)), Some(Joined::Watching));
        }
        dropped.extend(out.drop);
    }
    assert_eq!(dropped, [PeerId(41)]);
    assert_eq!(room.place(PeerId(41)), None);
}

#[test]
fn two_quick_presses_between_two_steps_are_played_on_two_steps() {
    let (mut room, peers) = full_room(8);
    room.receive(peers[4], ClientMsg::Input(press(UP)));
    room.receive(
        peers[4],
        ClientMsg::Input(SeatInput::new(vec![RIGHT], vec![RIGHT], vec![UP])),
    );
    // A third edge waits for no step: two at most.
    room.receive(
        peers[4],
        ClientMsg::Input(SeatInput::new(vec![], vec![], vec![RIGHT])),
    );
    let first = step(&mut room);
    let second = step(&mut room);
    let third = step(&mut room);
    let seat = |out: &Outbox| inputs(out).seats[4].clone();
    // The last held set counts on every step.
    assert_eq!(seat(&first), SeatInput::new(vec![UP], vec![], vec![]));
    assert_eq!(seat(&second), SeatInput::new(vec![RIGHT], vec![], vec![UP]));
    assert_eq!(seat(&third), SeatInput::default());
    assert_eq!(room.sim().dots[4].heading, 1);
}

#[test]
fn an_input_from_a_watcher_or_a_stranger_is_ignored() {
    let (mut room, _) = bots_out(9);
    let (mut twin, _) = bots_out(9);
    for room in [&mut room, &mut twin] {
        assert_eq!(room.join(PeerId(60)), Ok(Joined::Watching));
    }
    room.receive(PeerId(60), ClientMsg::Input(press(OUT)));
    room.receive(PeerId(77), ClientMsg::Input(press(OUT)));
    room.receive(PeerId(77), ClientMsg::Resync);
    let out = step(&mut room);
    assert_eq!(out, step(&mut twin));
    assert!(out.frames.iter().all(|(p, _)| *p != PeerId(77)));
}

#[test]
fn every_fifteenth_step_carries_the_sims_hash() {
    let (mut room, peers) = full_room(10);
    for n in 1..=45_u32 {
        let out = step(&mut room);
        let steps: Vec<_> = to(&out, peers[0])
            .into_iter()
            .filter_map(|frame| match frame {
                Frame::Step { step, hash, .. } => Some((*step, *hash)),
                _ => None,
            })
            .collect();
        let expected = n.is_multiple_of(HASH_EVERY).then(|| room.sim().hash());
        assert_eq!(steps, [(n, expected)]);
    }
}

#[test]
fn rounds_restart_after_the_pause_with_wins_counted() {
    let (mut room, peers) = full_room(11);
    for round in 1..=3_u32 {
        // Seat `round` wins: every other seat goes out.
        let winner = usize::try_from(round).expect("a seat");
        for (seat, peer) in peers.iter().enumerate() {
            if seat == winner {
                room.receive(*peer, ClientMsg::Alive);
            } else {
                room.receive(*peer, ClientMsg::Input(press(OUT)));
            }
        }
        let out = step(&mut room);
        let wins: Vec<u32> = (0..8)
            .map(|seat| u32::from(seat <= winner && seat > 0))
            .collect();
        assert_eq!(
            to(&out, peers[0]).last(),
            Some(&&Frame::RoundOver {
                winner: Some(winner),
                wins: wins.clone(),
            })
        );
        for _ in 0..Dots::PAUSE_STEPS {
            assert_eq!(step(&mut room).frames, []);
        }
        let out = step(&mut room);
        let frames = to(&out, peers[0]);
        let Frame::RoundStart(start) = frames[0] else {
            panic!("the round's start first: {frames:?}");
        };
        assert_eq!((start.round, &start.wins), (round + 1, &wins));
        assert!(matches!(frames[1], Frame::Step { step: 1, .. }));
        assert_eq!(room.stats().round, round + 1);
        assert_eq!(room.sim(), &{
            let mut sim = Dots.new_round(start.seed, start.round);
            sim.step(inputs(&out));
            sim
        });
    }
}

#[test]
fn a_round_at_its_longest_ends_as_a_draw() {
    let (mut room, peers) = full_room(12);
    for n in 1..=Dots::MAX_ROUND_STEPS {
        for peer in &peers {
            room.receive(*peer, ClientMsg::Alive);
        }
        let out = step(&mut room);
        let over = to(&out, peers[0])
            .into_iter()
            .any(|f| matches!(f, Frame::RoundOver { .. }));
        assert_eq!(over, n == Dots::MAX_ROUND_STEPS, "step {n}");
    }
    assert_eq!(room.wins(), [0; 8]);
    assert_eq!(room.stats().step, Dots::MAX_ROUND_STEPS);
}

#[test]
fn a_joiner_mid_round_gets_a_catch_up_that_reaches_the_rooms_sim() {
    let mut room = Room::new(Dots, 13);
    for _ in 0..20 {
        step(&mut room);
    }
    let seat = room.join(PeerId(50)).expect("a place");
    let out = step(&mut room);
    let frames = to(&out, PeerId(50));
    let Frame::Joined {
        place,
        round_start,
        steps,
        roster,
    } = frames[0]
    else {
        panic!("the catch-up first: {frames:?}");
    };
    assert_eq!(*place, seat);
    assert_eq!(steps.len(), 20);
    assert_eq!(roster.humans(), usize::from(seat != Joined::Watching));
    let mut sim = replay(round_start, steps);
    sim.step(inputs(&out));
    assert_eq!(&sim, room.sim());
    // A resync is sent the same, at the next step.
    room.receive(PeerId(50), ClientMsg::Resync);
    room.receive(PeerId(50), ClientMsg::Resync);
    let out = step(&mut room);
    let frames = to(&out, PeerId(50));
    assert_eq!(frames.len(), 2);
    let Frame::Joined {
        round_start, steps, ..
    } = frames[0]
    else {
        panic!("a catch-up: {frames:?}");
    };
    assert_eq!(steps.len(), 21);
    let mut sim = replay(round_start, steps);
    sim.step(inputs(&out));
    assert_eq!(&sim, room.sim());
}

#[test]
fn a_catch_up_never_holds_more_than_a_rounds_steps() {
    let (mut room, peers) = full_room(14);
    let mut longest = 0;
    for n in 0..2 * (Dots::MAX_ROUND_STEPS + Dots::PAUSE_STEPS) {
        for peer in &peers {
            room.receive(*peer, ClientMsg::Alive);
        }
        if n % 50 == 0 {
            room.receive(peers[0], ClientMsg::Resync);
        }
        for (_, frame) in &step(&mut room).frames {
            if let Frame::Joined { steps, .. } = frame {
                longest = longest.max(steps.len());
            }
        }
    }
    assert!(longest > 0);
    assert!(longest <= usize::try_from(Dots::MAX_ROUND_STEPS).expect("a count"));
}

#[test]
fn stats_count_the_room() {
    let mut room = Room::new(Dots, 15);
    assert_eq!(
        (room.stats().humans, room.stats().bots, room.stats().round),
        (0, 8, 1)
    );
    room.join(PeerId(1)).expect("a seat");
    room.join(PeerId(2)).expect("a seat");
    step(&mut room);
    let stats = room.stats();
    assert_eq!((stats.humans, stats.bots, stats.watchers), (2, 6, 0));
    assert_eq!(stats.step, 1);
    assert_eq!(room.game(), &Dots);
}

/// Seeded churn: peers join, leave, talk, resync and fall silent while the
/// room steps through rounds, and the room's books stay straight.
#[test]
fn churn_keeps_the_seats_straight() {
    let mut rng = SimRng::new(2614);
    let mut room = Room::new(Dots, 16);
    for _ in 0..20_000 {
        let peer = PeerId(rng.roll_below(14));
        match rng.roll_below(8) {
            0 | 1 => {
                let before = room.place(peer);
                if let Ok(place) = room.join(peer) {
                    assert!(before.is_none_or(|b| b == place));
                } else {
                    assert_eq!(before, None);
                    let counts = room.stats();
                    assert_eq!(counts.humans + counts.watchers, Dots::SEATS);
                }
            }
            2 => room.leave(peer),
            3 => room.receive(peer, ClientMsg::Leave),
            4 => room.receive(peer, ClientMsg::Resync),
            5 => room.receive(peer, ClientMsg::Input(press(OUT))),
            _ => {
                let out = step(&mut room);
                for (to, frame) in &out.frames {
                    assert!(room.place(*to).is_some(), "{to} is sent {frame:?}");
                    assert_eq!(Frame::decode::<Dots>(&frame.encode()).as_ref(), Ok(frame));
                }
                for dropped in &out.drop {
                    assert_eq!(room.place(*dropped), None);
                }
            }
        }
        let humans: Vec<PeerId> = room
            .seats()
            .iter()
            .filter_map(|s| match s {
                Seat::Human(p) => Some(*p),
                Seat::Bot => None,
            })
            .collect();
        let counts = room.stats();
        assert_eq!(counts.humans, humans.len());
        assert_eq!(counts.humans + counts.bots, Dots::SEATS);
        assert!(counts.humans + counts.watchers <= Dots::SEATS);
        // A peer holds one seat or one watching place, at most.
        for p in (0..14).map(PeerId) {
            let seats = humans.iter().filter(|h| **h == p).count();
            let watching = room.watchers.iter().filter(|(w, _)| *w == p).count();
            assert!(
                seats + watching <= 1,
                "{p}: {seats} seats, {watching} places"
            );
        }
    }
}
