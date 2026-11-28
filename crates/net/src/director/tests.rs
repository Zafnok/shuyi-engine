use shuyi_core::{ContentHash, Made, SimRng};

use super::{Director, DirectorConfig, DirectorStats, Pick, Refusals, RoomId, RoomSeats, pick};
use crate::room::testing::Dots;
use crate::room::wire::{ClientMsg, Frame, PROTOCOL, Refusal};
use crate::room::{Outbox, RoomGame, Seat};
use crate::{Channel, DisconnectReason, NetEvent, PeerId};

fn made() -> Made {
    Made::new("dots", ContentHash(0))
}

fn director(config: DirectorConfig) -> Director<Dots> {
    Director::new(config, Dots, made(), 1)
}

fn bytes(d: &mut Director<Dots>, peer: u32, bytes: Vec<u8>) {
    d.on_event(NetEvent::Message {
        peer: PeerId(peer),
        channel: Channel::Reliable,
        bytes,
    });
}

fn say(d: &mut Director<Dots>, peer: u32, msg: &ClientMsg) {
    bytes(d, peer, msg.encode());
}

fn greet(d: &mut Director<Dots>, peer: u32) {
    d.on_event(NetEvent::Connected(PeerId(peer)));
    say(d, peer, &ClientMsg::hello(made()));
}

fn play(d: &mut Director<Dots>, peer: u32) {
    greet(d, peer);
    say(d, peer, &ClientMsg::Play);
}

fn step(d: &mut Director<Dots>) -> Outbox {
    let mut out = Outbox::new();
    d.step(&mut out);
    out
}

fn steps(d: &mut Director<Dots>, n: u32) {
    for _ in 0..n {
        step(d);
    }
}

/// The refusals `out` sends `peer`.
fn refusals(out: &Outbox, peer: u32) -> Vec<Refusal> {
    out.frames
        .iter()
        .filter(|(p, _)| *p == PeerId(peer))
        .filter_map(|(_, frame)| match frame {
            Frame::Refused { why } => Some(why.clone()),
            _ => None,
        })
        .collect()
}

/// Each room's players, seated or watching, by id.
fn players(d: &Director<Dots>) -> Vec<(u16, usize)> {
    d.rooms()
        .map(|(id, room)| (id.0, room.stats().humans + room.stats().watchers))
        .collect()
}

fn seats(players: &[usize]) -> Vec<RoomSeats> {
    (0..)
        .zip(players)
        .map(|(id, players)| RoomSeats {
            id: RoomId(id),
            players: *players,
            seats: 8,
        })
        .collect()
}

/// The steps of `seconds` of Dots.
fn of(seconds: u32) -> u32 {
    seconds * Dots::STEPS_PER_SECOND
}

#[test]
fn the_policy_picks_the_fullest_room_that_takes_one_ties_to_the_lowest_id() {
    assert_eq!(pick(&seats(&[5, 7, 7]), 10), Pick::Room(RoomId(1)));
    assert_eq!(pick(&seats(&[8, 8, 2]), 10), Pick::Room(RoomId(2)));
    assert_eq!(pick(&seats(&[0, 0, 0]), 10), Pick::Room(RoomId(0)));
    let apart = [
        RoomSeats {
            id: RoomId(9),
            players: 4,
            seats: 8,
        },
        RoomSeats {
            id: RoomId(3),
            players: 4,
            seats: 8,
        },
    ];
    assert_eq!(pick(&apart, 10), Pick::Room(RoomId(3)));
}

#[test]
fn the_policy_opens_a_room_when_every_room_is_full_until_the_cap() {
    assert_eq!(pick(&[], 10), Pick::Open);
    assert_eq!(pick(&seats(&[8, 8]), 3), Pick::Open);
    assert_eq!(pick(&seats(&[8, 8, 8]), 3), Pick::Busy);
    assert_eq!(pick(&[], 0), Pick::Busy);
    // Full rooms are skipped, whatever their count.
    assert_eq!(pick(&seats(&[8, 1, 8]), 3), Pick::Room(RoomId(1)));
}

#[test]
fn eighty_players_fill_ten_rooms_and_the_81st_is_busy_until_one_leaves() {
    let mut d = director(DirectorConfig::new());
    for peer in 0..80 {
        play(&mut d, peer);
    }
    step(&mut d);
    assert_eq!(players(&d), (0..10).map(|id| (id, 8)).collect::<Vec<_>>());
    assert_eq!(d.stats().humans, 80);
    play(&mut d, 80);
    let out = step(&mut d);
    let busy = Refusal::Busy { retry_after_s: 10 };
    assert_eq!(refusals(&out, 80), [busy]);
    assert!(!out.drop.contains(&PeerId(80)), "a busy client stays");
    assert_eq!(d.stats().rooms, 10);
    let freed = d.room_of(PeerId(37));
    say(&mut d, 37, &ClientMsg::Leave);
    say(&mut d, 80, &ClientMsg::Play);
    let out = step(&mut d);
    assert_eq!(refusals(&out, 80), Vec::new());
    assert_eq!(d.room_of(PeerId(80)), freed);
    assert_eq!(d.room_of(PeerId(37)), None);
}

/// A director of three rooms, its players leaving each till it holds
/// `left` of them; the rooms' players are 0–7, 8–15 and 16–23.
fn three_rooms(left: [usize; 3]) -> Director<Dots> {
    let mut d = director(DirectorConfig::new().with_rooms_max(3));
    for peer in 0..24 {
        play(&mut d, peer);
    }
    for (room, left) in (0..).zip(left) {
        let keep = u32::try_from(left).unwrap_or(8);
        for peer in room * 8 + keep..room * 8 + 8 {
            say(&mut d, peer, &ClientMsg::Leave);
        }
    }
    step(&mut d);
    d
}

#[test]
fn play_seats_the_player_in_the_fullest_room_with_a_free_seat() {
    let mut d = three_rooms([5, 7, 7]);
    assert_eq!(players(&d), [(0, 5), (1, 7), (2, 7)]);
    play(&mut d, 99);
    assert_eq!(d.room_of(PeerId(99)), Some(RoomId(1)));
    let mut d = three_rooms([8, 8, 2]);
    play(&mut d, 99);
    assert_eq!(d.room_of(PeerId(99)), Some(RoomId(2)));
}

#[test]
fn with_every_room_open_and_full_play_is_busy_and_opens_none() {
    let mut d = three_rooms([8, 8, 8]);
    play(&mut d, 99);
    let out = step(&mut d);
    assert_eq!(refusals(&out, 99), [Refusal::Busy { retry_after_s: 10 }]);
    assert_eq!(d.stats().rooms, 3);
    assert_eq!(d.room_of(PeerId(99)), None);
    assert_eq!(d.stats().refused.busy, 1);
}

#[test]
fn another_build_is_refused_with_the_directors_stamp_and_closed() {
    let mut d = director(DirectorConfig::new());
    d.on_event(NetEvent::Connected(PeerId(1)));
    say(
        &mut d,
        1,
        &ClientMsg::hello(Made::new("dots 2", ContentHash(0))),
    );
    let out = step(&mut d);
    assert_eq!(refusals(&out, 1), [Refusal::OtherBuild { theirs: made() }]);
    assert_eq!(out.drop, [PeerId(1)]);
    assert_eq!(d.stats().peers, 0);
    assert_eq!(d.stats().refused.other_build, 1);
}

#[test]
fn another_protocol_a_play_first_or_no_message_first_is_refused_and_closed() {
    let mut d = director(DirectorConfig::new());
    let ours = Refusal::Protocol { ours: PROTOCOL };
    for peer in 1..=3 {
        d.on_event(NetEvent::Connected(PeerId(peer)));
    }
    let other = ClientMsg::Hello {
        protocol: PROTOCOL + 1,
        made: made(),
    };
    say(&mut d, 1, &other);
    say(&mut d, 2, &ClientMsg::Play);
    bytes(&mut d, 3, vec![0x84, 2]);
    let out = step(&mut d);
    for peer in 1..=3 {
        assert_eq!(refusals(&out, peer), core::slice::from_ref(&ours), "{peer}");
    }
    assert_eq!(out.drop, [PeerId(1), PeerId(2), PeerId(3)]);
    assert_eq!(d.stats().refused.protocol, 3);
    // Nothing more is read from them.
    say(&mut d, 2, &ClientMsg::hello(made()));
    assert_eq!(step(&mut d).frames, Vec::new());
}

#[test]
fn a_good_hello_is_welcomed_and_a_second_hello_or_play_is_ignored() {
    let mut d = director(DirectorConfig::new());
    play(&mut d, 1);
    let out = step(&mut d);
    assert_eq!(out.frames[0], (PeerId(1), Frame::Welcome));
    let place = d.room(RoomId(0)).and_then(|room| room.place(PeerId(1)));
    say(&mut d, 1, &ClientMsg::hello(made()));
    say(&mut d, 1, &ClientMsg::Play);
    let out = step(&mut d);
    assert!(!out.frames.iter().any(|(_, f)| *f == Frame::Welcome));
    assert_eq!(refusals(&out, 1), Vec::new());
    assert_eq!(d.room(RoomId(0)).and_then(|r| r.place(PeerId(1))), place);
    assert_eq!(d.stats().rooms, 1);
}

#[test]
fn a_client_with_no_hello_in_time_is_let_go_at_the_time_not_a_step_sooner() {
    let mut d = director(DirectorConfig::new());
    d.on_event(NetEvent::Connected(PeerId(1)));
    for _ in 1..of(5) {
        assert_eq!(step(&mut d).drop, Vec::new());
    }
    assert_eq!(d.stats().peers, 1);
    assert_eq!(step(&mut d).drop, [PeerId(1)]);
    assert_eq!(d.stats().peers, 0);
}

#[test]
fn a_welcomed_client_out_of_a_room_is_let_go_after_five_silent_seconds() {
    let mut d = director(DirectorConfig::new());
    greet(&mut d, 1);
    steps(&mut d, of(5) - 1);
    say(&mut d, 1, &ClientMsg::Alive);
    steps(&mut d, of(5) - 1);
    assert_eq!(d.stats().peers, 1);
    assert_eq!(step(&mut d).drop, [PeerId(1)]);
}

#[test]
fn leaving_frees_the_seat_and_the_player_may_play_again() {
    let mut d = director(DirectorConfig::new());
    play(&mut d, 1);
    step(&mut d);
    say(&mut d, 1, &ClientMsg::Leave);
    step(&mut d);
    assert_eq!(d.room_of(PeerId(1)), None);
    assert_eq!(d.stats().humans, 0);
    assert_eq!(d.stats().peers, 1);
    say(&mut d, 1, &ClientMsg::Play);
    step(&mut d);
    assert_eq!(d.room_of(PeerId(1)), Some(RoomId(0)));
}

#[test]
fn a_disconnect_frees_the_seat_and_forgets_the_player() {
    let mut d = director(DirectorConfig::new());
    play(&mut d, 1);
    step(&mut d);
    d.on_event(NetEvent::Disconnected {
        peer: PeerId(1),
        reason: DisconnectReason::Closed,
    });
    step(&mut d);
    assert_eq!(d.stats().humans, 0);
    assert_eq!(d.stats().peers, 0);
}

#[test]
fn a_player_silent_in_its_room_is_let_go_and_forgotten() {
    let mut d = director(DirectorConfig::new());
    play(&mut d, 1);
    let mut dropped = false;
    for _ in 0..=of(5) {
        dropped |= step(&mut d).drop.contains(&PeerId(1));
    }
    assert!(dropped);
    assert_eq!(d.stats().peers, 0);
    assert_eq!(d.room_of(PeerId(1)), None);
}

#[test]
fn the_warm_room_runs_from_the_start_and_never_closes() {
    let mut d = director(DirectorConfig::new());
    assert_eq!(players(&d), [(0, 0)]);
    steps(&mut d, of(60) * 3);
    assert_eq!(players(&d), [(0, 0)]);
    assert!(d.room(RoomId(0)).is_some_and(|room| room.stats().round > 1));
    let none = director(DirectorConfig::new().with_rooms_warm(0));
    assert_eq!(none.stats().rooms, 0);
    let all = director(DirectorConfig::new().with_rooms_max(3).with_rooms_warm(7));
    assert_eq!(all.stats().rooms, 3);
}

#[test]
fn a_room_whose_last_player_left_closes_at_its_idle_time_not_a_step_sooner() {
    let mut d = director(DirectorConfig::new());
    for peer in 0..9 {
        play(&mut d, peer);
    }
    step(&mut d);
    assert_eq!(d.room_of(PeerId(8)), Some(RoomId(1)));
    say(&mut d, 8, &ClientMsg::Leave);
    for _ in 1..of(60) {
        step(&mut d);
    }
    assert!(d.room(RoomId(1)).is_some());
    step(&mut d);
    assert!(d.room(RoomId(1)).is_none());
    // Room 0's players fell silent, but it is the warm room.
    assert_eq!(players(&d), [(0, 0)]);
}

#[test]
fn a_room_id_is_never_reused() {
    let mut d = director(
        DirectorConfig::new()
            .with_rooms_warm(0)
            .with_idle_close_s(1),
    );
    for round in 0..3 {
        play(&mut d, round);
        step(&mut d);
        assert_eq!(
            d.room_of(PeerId(round)),
            Some(RoomId(u16::try_from(round).unwrap_or(0)))
        );
        say(&mut d, round, &ClientMsg::Leave);
        steps(&mut d, of(1));
        assert_eq!(d.stats().rooms, 0);
    }
}

#[test]
fn a_client_past_its_message_budget_is_cut_off_and_one_at_it_is_not() {
    let mut d = director(DirectorConfig::new());
    play(&mut d, 1);
    // Its first second, the `Hello` and the `Play`, runs out.
    steps(&mut d, of(1));
    let seat = d
        .room(RoomId(0))
        .and_then(|room| room.place(PeerId(1)))
        .and_then(crate::room::Joined::seat);
    for _ in 0..30 {
        say(&mut d, 1, &ClientMsg::Alive);
    }
    assert_eq!(step(&mut d).drop, Vec::new());
    steps(&mut d, of(1));
    for _ in 0..31 {
        say(&mut d, 1, &ClientMsg::Alive);
    }
    let out = step(&mut d);
    assert_eq!(out.drop, [PeerId(1)]);
    assert!(out.frames.iter().all(|(peer, _)| *peer != PeerId(1)));
    let seats = d.room(RoomId(0)).map(|room| room.seats().to_vec());
    let seat = seat.unwrap_or(0);
    assert_eq!(seats.map(|seats| seats[seat]), Some(Seat::Bot));
    assert_eq!(d.stats().refused.flood, 1);
    assert_eq!(d.stats().peers, 0);
}

#[test]
fn a_flood_before_the_next_step_sends_the_client_nothing_more() {
    let mut d = director(DirectorConfig::new().with_msgs_per_second(2));
    d.on_event(NetEvent::Connected(PeerId(1)));
    say(&mut d, 1, &ClientMsg::hello(made()));
    say(&mut d, 1, &ClientMsg::Play);
    say(&mut d, 1, &ClientMsg::Alive);
    let out = step(&mut d);
    // Not even the welcome or the room's catch-up.
    assert!(out.frames.iter().all(|(peer, _)| *peer != PeerId(1)));
    assert_eq!(out.drop, [PeerId(1)]);
    assert_eq!(d.stats().humans, 0);
}

#[test]
fn stats_count_rooms_seats_clients_and_refusals() {
    let mut d = director(DirectorConfig::new().with_rooms_max(2));
    for peer in 0..10 {
        play(&mut d, peer);
    }
    greet(&mut d, 20);
    d.on_event(NetEvent::Connected(PeerId(21)));
    d.on_event(NetEvent::Connected(PeerId(22)));
    say(&mut d, 22, &ClientMsg::Play);
    step(&mut d);
    let stats = d.stats();
    let want = DirectorStats {
        rooms: 2,
        humans: stats.humans,
        bots: stats.bots,
        watchers: stats.watchers,
        peers: 12,
        refused: Refusals {
            protocol: 1,
            ..Refusals::default()
        },
    };
    assert_eq!(stats, want);
    assert_eq!(stats.humans + stats.watchers, 10);
    assert_eq!(stats.humans + stats.bots, 2 * Dots::SEATS);
}

/// A seeded run of events, the same for every director it is fed to.
fn feed(d: &mut Director<Dots>, seed: u64) -> Vec<Vec<u8>> {
    let mut rng = SimRng::new(seed);
    let mut written = Vec::new();
    for _ in 0..2_000 {
        let peer = rng.roll_below(40);
        match rng.roll_below(8) {
            0 => d.on_event(NetEvent::Connected(PeerId(peer))),
            1 => say(d, peer, &ClientMsg::hello(made())),
            2 => say(d, peer, &ClientMsg::Play),
            3 => say(d, peer, &ClientMsg::Leave),
            4 => d.on_event(NetEvent::Disconnected {
                peer: PeerId(peer),
                reason: DisconnectReason::Closed,
            }),
            _ => {
                let out = step(d);
                for (peer, frame) in &out.frames {
                    let mut bytes = peer.0.to_le_bytes().to_vec();
                    bytes.extend(frame.encode());
                    written.push(bytes);
                }
                written.extend(out.drop.iter().map(|peer| peer.0.to_le_bytes().to_vec()));
            }
        }
    }
    written
}

#[test]
fn two_directors_of_one_seed_fed_the_same_events_write_the_same_bytes() {
    let config = DirectorConfig::new().with_rooms_max(3).with_idle_close_s(2);
    let mut one = Director::new(config, Dots, made(), 77);
    let mut two = Director::new(config, Dots, made(), 77);
    let written = feed(&mut one, 5);
    assert!(written.len() > 1_000);
    assert_eq!(written, feed(&mut two, 5));
    let mut other = Director::new(config, Dots, made(), 78);
    assert_ne!(
        written,
        feed(&mut other, 5),
        "the seed picks the rooms' seeds"
    );
}

#[test]
fn a_room_id_reads_as_its_number() {
    assert_eq!(RoomId(7).to_string(), "room 7");
    assert_eq!(DirectorConfig::default(), DirectorConfig::new());
    let config = DirectorConfig::new()
        .with_hello_timeout_s(2)
        .with_retry_after_s(3)
        .with_msgs_per_second(4)
        .with_idle_close_s(5);
    assert_eq!(
        (
            config.hello_timeout_s(),
            config.retry_after_s(),
            config.msgs_per_second(),
            config.idle_close_s()
        ),
        (2, 3, 4, 5)
    );
    let d = director(config);
    assert_eq!((d.config(), d.made()), (config, &made()));
}
