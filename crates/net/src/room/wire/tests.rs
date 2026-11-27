use shuyi_core::SimRng;
use shuyi_core::tick::{ActionId, SeatInput, TickInput};

use super::{ClientMsg, Frame, Roster, RoundStart, WireError};
use crate::room::testing::Dots;
use crate::room::{Joined, RoomGame, SeatKind};

fn press(action: u16) -> SeatInput {
    SeatInput::new(vec![ActionId(action)], vec![ActionId(action)], vec![])
}

fn inputs() -> TickInput {
    let mut seats = vec![SeatInput::default(); Dots::SEATS];
    seats[2] = press(3);
    seats[7] = SeatInput::new(vec![], vec![], vec![ActionId(1), ActionId(4)]);
    TickInput::new(seats)
}

fn roster() -> Roster {
    let mut seats = vec![SeatKind::Bot; Dots::SEATS];
    seats[0] = SeatKind::Human;
    seats[7] = SeatKind::Human;
    Roster { seats }
}

fn round_start() -> RoundStart {
    RoundStart {
        round: 12,
        seed: 0x0123_4567_89ab_cdef,
        wins: (0..8).collect(),
    }
}

/// One of every frame.
fn frames() -> Vec<Frame> {
    vec![
        Frame::Step {
            step: 1,
            inputs: inputs(),
            hash: None,
        },
        Frame::Step {
            step: 15,
            inputs: TickInput::default(),
            hash: Some(u64::MAX),
        },
        Frame::Joined {
            place: Joined::Seat(7),
            roster: roster(),
            round_start: round_start(),
            steps: vec![inputs(); 3],
        },
        Frame::Joined {
            place: Joined::Watching,
            roster: Roster::default(),
            round_start: round_start(),
            steps: Vec::new(),
        },
        Frame::Roster {
            place: Joined::Seat(0),
            roster: roster(),
        },
        Frame::RoundOver {
            winner: Some(3),
            wins: vec![1; 8],
        },
        Frame::RoundOver {
            winner: None,
            wins: Vec::new(),
        },
        Frame::RoundStart(round_start()),
        Frame::Closing,
    ]
}

/// One of every client message.
fn msgs() -> Vec<ClientMsg> {
    vec![
        ClientMsg::Input(press(2)),
        ClientMsg::Input(SeatInput::default()),
        ClientMsg::Alive,
        ClientMsg::Resync,
        ClientMsg::Leave,
    ]
}

#[test]
fn every_frame_round_trips() {
    for frame in frames() {
        let bytes = frame.encode();
        assert_eq!(Frame::decode::<Dots>(&bytes), Ok(frame));
    }
}

#[test]
fn every_client_message_round_trips() {
    for msg in msgs() {
        let bytes = msg.encode();
        assert_eq!(ClientMsg::decode(&bytes), Ok(msg));
    }
}

#[test]
fn a_step_is_its_number_its_hash_and_the_inputs_byte_form() {
    let idle = TickInput::new(vec![SeatInput::default(); 8]);
    let frame = Frame::Step {
        step: 2,
        inputs: idle,
        hash: None,
    };
    // Tag, step, no hash, 8 idle seats.
    assert_eq!(
        frame.encode(),
        [1, 2, 0, 0, 0, 0, 8, 0, 0, 0, 0, 0, 0, 0, 0]
    );
}

#[test]
fn a_list_past_255_actions_is_cut_to_its_first_255() {
    let long = SeatInput::new((0..300).map(ActionId).collect(), vec![], vec![]);
    let Ok(ClientMsg::Input(cut)) = ClientMsg::decode(&ClientMsg::Input(long).encode()) else {
        panic!("the message decodes");
    };
    assert_eq!(cut.pressed, (0..255).map(ActionId).collect::<Vec<_>>());
    let frame = Frame::Step {
        step: 1,
        inputs: TickInput::one(cut.clone()),
        hash: None,
    };
    let mut long_inputs = TickInput::one(cut);
    long_inputs.seats[0].pressed.push(ActionId(999));
    let long_frame = Frame::Step {
        step: 1,
        inputs: long_inputs,
        hash: None,
    };
    assert_eq!(long_frame.encode(), frame.encode());
}

#[test]
fn bytes_that_end_early_are_short() {
    assert_eq!(Frame::decode::<Dots>(&[]), Err(WireError::Short));
    assert_eq!(ClientMsg::decode(&[]), Err(WireError::Short));
    for frame in frames() {
        let bytes = frame.encode();
        for len in 1..bytes.len() {
            let got = Frame::decode::<Dots>(&bytes[..len]);
            assert!(
                matches!(got, Err(WireError::Short | WireError::BadInput)),
                "{frame:?} cut at {len}: {got:?}"
            );
        }
    }
}

#[test]
fn bytes_past_the_message_are_overlong() {
    let mut bytes = Frame::Closing.encode();
    bytes.extend([0, 0]);
    assert_eq!(
        Frame::decode::<Dots>(&bytes),
        Err(WireError::Overlong { extra: 2 })
    );
    let mut bytes = ClientMsg::Alive.encode();
    bytes.push(9);
    assert_eq!(
        ClientMsg::decode(&bytes),
        Err(WireError::Overlong { extra: 1 })
    );
}

#[test]
fn an_unknown_first_byte_names_no_message() {
    assert_eq!(Frame::decode::<Dots>(&[0]), Err(WireError::UnknownTag(0)));
    // A client's message is no frame, and a frame no client's message.
    assert_eq!(
        Frame::decode::<Dots>(&ClientMsg::Alive.encode()),
        Err(WireError::UnknownTag(0x81))
    );
    assert_eq!(
        ClientMsg::decode(&Frame::Closing.encode()),
        Err(WireError::UnknownTag(0x06))
    );
}

/// A game of two seats, to read Dots' frames against.
struct Two;

impl RoomGame for Two {
    type Sim = <Dots as RoomGame>::Sim;
    const SEATS: usize = 2;
    const STEPS_PER_SECOND: u32 = 15;
    const MAX_ROUND_STEPS: u32 = 2;
    const PAUSE_STEPS: u32 = 0;

    fn made(&self) -> shuyi_core::Made {
        Dots.made()
    }

    fn new_round(&self, seed: u64, round: u32) -> Self::Sim {
        Dots.new_round(seed, round)
    }

    fn round_winner(sim: &Self::Sim) -> crate::room::RoundEnd {
        Dots::round_winner(sim)
    }

    fn seat_alive(sim: &Self::Sim, seat: usize) -> bool {
        Dots::seat_alive(sim, seat)
    }

    fn bot_input(sim: &Self::Sim, seat: usize, rng: &mut SimRng) -> SeatInput {
        Dots::bot_input(sim, seat, rng)
    }
}

#[test]
fn a_seat_past_the_games_seats_is_refused() {
    let place = Frame::Roster {
        place: Joined::Seat(5),
        roster: Roster::default(),
    };
    assert_eq!(
        Frame::decode::<Two>(&place.encode()),
        Err(WireError::NoSuchSeat { seat: 5, seats: 2 })
    );
    let winner = Frame::RoundOver {
        winner: Some(2),
        wins: Vec::new(),
    };
    assert_eq!(
        Frame::decode::<Two>(&winner.encode()),
        Err(WireError::NoSuchSeat { seat: 2, seats: 2 })
    );
    let step = Frame::Step {
        step: 1,
        inputs: inputs(),
        hash: None,
    };
    assert_eq!(
        Frame::decode::<Two>(&step.encode()),
        Err(WireError::TooManySeats { count: 8, seats: 2 })
    );
    let roster = Frame::Roster {
        place: Joined::Watching,
        roster: roster(),
    };
    assert_eq!(
        Frame::decode::<Two>(&roster.encode()),
        Err(WireError::TooManySeats { count: 8, seats: 2 })
    );
    assert_eq!(
        Frame::decode::<Two>(&Frame::RoundStart(round_start()).encode()),
        Err(WireError::TooManySeats { count: 8, seats: 2 })
    );
}

#[test]
fn a_catch_up_longer_than_a_round_is_refused() {
    let catch_up = |steps: usize| Frame::Joined {
        place: Joined::Watching,
        roster: Roster::default(),
        round_start: RoundStart {
            round: 1,
            seed: 0,
            wins: Vec::new(),
        },
        steps: vec![TickInput::default(); steps],
    };
    assert!(Frame::decode::<Two>(&catch_up(2).encode()).is_ok());
    assert_eq!(
        Frame::decode::<Two>(&catch_up(3).encode()),
        Err(WireError::CatchUp { steps: 3, max: 2 })
    );
    // A count past the bytes left is short, before anything is kept.
    let mut bytes = catch_up(0).encode();
    let count = bytes.len() - 4;
    bytes[count..].copy_from_slice(&2_u32.to_le_bytes());
    assert_eq!(Frame::decode::<Two>(&bytes), Err(WireError::Short));
}

#[test]
fn a_fields_byte_out_of_its_values_is_refused() {
    let mut bytes = Frame::Step {
        step: 1,
        inputs: TickInput::default(),
        hash: None,
    }
    .encode();
    bytes[5] = 2;
    assert_eq!(
        Frame::decode::<Dots>(&bytes),
        Err(WireError::BadField("hash flag"))
    );
    let mut bytes = Frame::Roster {
        place: Joined::Watching,
        roster: Roster {
            seats: vec![SeatKind::Bot; 2],
        },
    }
    .encode();
    bytes[3] = 0b100;
    assert_eq!(
        Frame::decode::<Dots>(&bytes),
        Err(WireError::BadField("roster"))
    );
    // A seat's header with a reserved bit set.
    assert_eq!(ClientMsg::decode(&[0x80, 0b1000]), Err(WireError::BadInput));
}

#[test]
fn each_error_says_what_is_wrong() {
    let texts = [
        (WireError::Short, "the message ends early"),
        (
            WireError::Overlong { extra: 3 },
            "3 bytes follow the message",
        ),
        (
            WireError::UnknownTag(0x7f),
            "no message starts with byte 0x7f",
        ),
        (
            WireError::NoSuchSeat { seat: 9, seats: 8 },
            "seat 9 is past the game's 8 seats",
        ),
        (
            WireError::TooManySeats { count: 9, seats: 8 },
            "9 seats are more than the game's 8",
        ),
        (
            WireError::CatchUp { steps: 9, max: 8 },
            "a catch-up of 9 steps is longer than a round's 8",
        ),
        (
            WireError::BadInput,
            "a seat's input is not in its byte form",
        ),
        (
            WireError::BadField("roster"),
            "the roster holds no allowed value",
        ),
    ];
    for (error, text) in texts {
        assert_eq!(error.to_string(), text);
    }
}

/// Checks that `bytes` decode without a panic, and that what decodes
/// encodes back to the same bytes: each message has one form.
fn decodes_or_refuses(bytes: &[u8]) {
    if let Ok(frame) = Frame::decode::<Dots>(bytes) {
        assert_eq!(frame.encode(), bytes, "{frame:?}");
    }
    if let Ok(msg) = ClientMsg::decode(bytes) {
        assert_eq!(msg.encode(), bytes, "{msg:?}");
    }
}

fn byte(rng: &mut SimRng) -> u8 {
    u8::try_from(rng.roll_below(256)).unwrap_or(0)
}

#[test]
fn random_bytes_never_panic() {
    let mut rng = SimRng::new(2614);
    for _ in 0..100_000 {
        let len = rng.roll_below(40);
        let mut bytes: Vec<u8> = (0..len).map(|_| byte(&mut rng)).collect();
        // Most strings start with a real tag, so the fields are read.
        if let Some(first) = bytes.first_mut()
            && rng.roll_below(4) != 0
        {
            *first = [1, 2, 3, 4, 5, 6, 0x80, 0x81, 0x82, 0x83][usize::from(*first % 10)];
        }
        decodes_or_refuses(&bytes);
    }
}

#[test]
fn mutations_of_valid_messages_never_panic() {
    let mut rng = SimRng::new(2615);
    let valid: Vec<Vec<u8>> = frames()
        .iter()
        .map(Frame::encode)
        .chain(msgs().iter().map(ClientMsg::encode))
        .collect();
    for n in 0..100_000_usize {
        let mut bytes = valid[n % valid.len()].clone();
        for _ in 0..=rng.roll_below(3) {
            let at = usize::try_from(rng.roll_below(u32::try_from(bytes.len()).unwrap_or(1)))
                .unwrap_or(0);
            match rng.roll_below(3) {
                0 if at < bytes.len() => bytes[at] = byte(&mut rng),
                1 => bytes.insert(at.min(bytes.len()), byte(&mut rng)),
                _ => bytes.truncate(at),
            }
        }
        decodes_or_refuses(&bytes);
    }
}
