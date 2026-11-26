//! The byte form: round trips, a fixture written by hand from the layout,
//! every refusal, and properties over random inputs and random bytes.

use alloc::vec;
use alloc::vec::Vec;

use proptest::prelude::*;
use proptest::test_runner::RngSeed;

use super::*;

/// `ids` as actions.
fn ids(ids: &[u16]) -> Vec<ActionId> {
    ids.iter().copied().map(ActionId).collect()
}

/// `input`'s byte form.
fn encoded(input: &TickInput) -> Vec<u8> {
    let mut out = Vec::new();
    input.encode(&mut out).unwrap_or_else(|e| panic!("{e}"));
    out
}

#[test]
fn hand_picked_inputs_round_trip() {
    let full = SeatInput::new(ids(&[1, 2]), ids(&[2, 3, 65_535]), ids(&[0]));
    for input in [
        TickInput::default(),
        TickInput::one(SeatInput::default()),
        TickInput::new(vec![full.clone(); MAX_SEATS]),
        TickInput::one(full),
    ] {
        let bytes = encoded(&input);
        assert_eq!(TickInput::decode(&bytes), Some((input, bytes.len())));
    }
}

#[test]
fn a_known_input_has_the_bytes_of_its_layout() {
    let input = TickInput::new(vec![
        SeatInput::new(ids(&[1]), ids(&[1, 0x0203]), vec![]),
        SeatInput::new(vec![], vec![], ids(&[0x1234])),
    ]);
    // Written from the layout, not from the code's output.
    let expected = [
        2, // two seats
        0b011, 1, 0x01, 0x00, 2, 0x01, 0x00, 0x03, 0x02, // pressed, held
        0b100, 1, 0x34, 0x12, // released
    ];
    assert_eq!(encoded(&input), expected);
    // Bytes after it are left to the caller.
    let mut more = expected.to_vec();
    more.push(0xff);
    assert_eq!(TickInput::decode(&more), Some((input, expected.len())));
}

#[test]
fn an_idle_step_of_eight_seats_is_nine_bytes_and_a_press_adds_three() {
    let mut seats = vec![SeatInput::default(); MAX_SEATS];
    assert_eq!(encoded(&TickInput::new(seats.clone())).len(), 9);
    seats[5] = SeatInput::new(ids(&[4]), vec![], vec![]);
    assert_eq!(encoded(&TickInput::new(seats)).len(), 12);
}

#[test]
fn more_seats_than_the_limit_are_left_out_as_new_does() {
    // Past `new`, as a game can by pushing to its public `seats`.
    let input = TickInput {
        seats: vec![SeatInput::default(); MAX_SEATS + 1],
    };
    let bytes = encoded(&input);
    assert_eq!(bytes[0], 8);
    assert_eq!(
        TickInput::decode(&bytes).map(|(i, _)| i.seats.len()),
        Some(MAX_SEATS)
    );
}

#[test]
fn a_list_of_more_than_255_actions_writes_nothing() {
    let long = SeatInput::new(vec![], vec![ActionId(1); 256], vec![]);
    let mut out = vec![9];
    let refused = long.encode(&mut out).unwrap_err();
    assert_eq!(out, [9]);
    assert_eq!(
        alloc::format!("{refused}"),
        "a seat's list holds 256 actions; its byte form holds at most 255"
    );
    let input = TickInput::new(vec![SeatInput::default(), long]);
    assert!(input.encode(&mut out).is_err());
    assert_eq!(out, [9]);
    // 255 fits.
    let most = SeatInput::new(vec![ActionId(1); 255], vec![], vec![]);
    assert!(most.encode(&mut out).is_ok());
    assert_eq!(out.len(), 1 + 1 + 1 + 2 * 255);
}

#[test]
fn bad_bytes_are_refused() {
    for bytes in [
        &[][..],              // nothing
        &[9],                 // nine seats
        &[255],               // and more
        &[1],                 // a seat missing
        &[1, 0b1000],         // a reserved bit
        &[1, 0x80],           // the top one
        &[1, 0b001, 0],       // a count of 0
        &[1, 0b001],          // a count missing
        &[1, 0b001, 1, 7],    // half an action
        &[1, 0b011, 1, 7, 0], // held missing
        &[2, 0],              // the second seat missing
    ] {
        assert_eq!(TickInput::decode(bytes), None, "{bytes:?}");
    }
    assert_eq!(SeatInput::decode(&[]), None);
    assert_eq!(SeatInput::decode(&[0]), Some((SeatInput::default(), 1)));
}

/// A random list of 0 to 16 actions.
fn any_list() -> impl Strategy<Value = Vec<ActionId>> {
    proptest::collection::vec(any::<u16>().prop_map(ActionId), 0..=16)
}

/// A random step of 0 to 8 seats.
fn any_input() -> impl Strategy<Value = TickInput> {
    let seat = (any_list(), any_list(), any_list())
        .prop_map(|(pressed, held, released)| SeatInput::new(pressed, held, released));
    proptest::collection::vec(seat, 0..=MAX_SEATS).prop_map(TickInput::new)
}

proptest! {
    #![proptest_config(ProptestConfig {
        rng_seed: RngSeed::Fixed(2612),
        ..ProptestConfig::default()
    })]

    #[test]
    fn every_input_round_trips(input in any_input()) {
        let bytes = encoded(&input);
        prop_assert_eq!(TickInput::decode(&bytes), Some((input, bytes.len())));
    }

    #[test]
    fn any_bytes_decode_without_panic_and_back_to_what_was_read(
        bytes in proptest::collection::vec(any::<u8>(), 0..=256),
    ) {
        if let Some((input, read)) = TickInput::decode(&bytes) {
            let again = encoded(&input);
            prop_assert_eq!(again.as_slice(), &bytes[..read]);
        }
        if let Some((seat, read)) = SeatInput::decode(&bytes) {
            let mut out = Vec::new();
            prop_assert!(seat.encode(&mut out).is_ok());
            prop_assert_eq!(out.as_slice(), &bytes[..read]);
        }
    }
}
