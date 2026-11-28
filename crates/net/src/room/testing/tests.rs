use shuyi_core::SimRng;
use shuyi_core::tick::{ActionId, SeatInput, TickInput, TickSim};

use super::{DOWN, Dot, DotEvent, Dots, DotsSim, LEFT, OUT, RIGHT, SIDE, UP};
use crate::room::{RoomGame, RoundEnd};

/// A round with one dot in seat 0 and the others out.
fn one(dot: Dot) -> DotsSim {
    let out = Dot {
        alive: false,
        ..dot
    };
    let mut dots = [out; 8];
    dots[0] = dot;
    DotsSim { dots, steps: 0 }
}

/// A step where seat 0 presses `pressed`, in order.
fn pressing(pressed: &[ActionId]) -> TickInput {
    TickInput::one(SeatInput::new(pressed.to_vec(), vec![], vec![]))
}

#[test]
fn a_dot_moves_a_cell_a_step_and_stops_at_the_walls() {
    assert_eq!(SIDE, 16);
    // Each heading from two cells before its wall: one step, then stopped.
    let cases = [
        (0, (5, 1), (5, 0)),
        (1, (14, 5), (15, 5)),
        (2, (5, 14), (5, 15)),
        (3, (1, 5), (0, 5)),
    ];
    for (heading, (x, y), wall) in cases {
        let mut sim = one(Dot {
            x,
            y,
            heading,
            alive: true,
        });
        let idle = TickInput::new(vec![SeatInput::default(); 8]);
        for _ in 0..3 {
            assert_eq!(sim.step(&idle), []);
            assert_eq!((sim.dots[0].x, sim.dots[0].y), wall, "heading {heading}");
        }
        assert_eq!(sim.steps, 3);
    }
}

#[test]
fn a_press_turns_a_dot_and_the_last_turn_counts() {
    let mut sim = one(Dot {
        x: 8,
        y: 8,
        heading: 0,
        alive: true,
    });
    sim.step(&pressing(&[RIGHT]));
    assert_eq!((sim.dots[0].heading, sim.dots[0].x), (1, 9));
    sim.step(&pressing(&[UP, LEFT]));
    assert_eq!((sim.dots[0].heading, sim.dots[0].x), (3, 8));
    sim.step(&pressing(&[DOWN, ActionId(9)]));
    assert_eq!((sim.dots[0].heading, sim.dots[0].y), (2, 9));
}

#[test]
fn out_takes_a_dot_out_and_a_dot_out_stays_put() {
    let mut sim = one(Dot {
        x: 8,
        y: 8,
        heading: 1,
        alive: true,
    });
    assert_eq!(sim.step(&pressing(&[OUT])), [DotEvent::Out(0)]);
    assert!(!Dots::seat_alive(&sim, 0));
    sim.step(&pressing(&[UP]));
    assert_eq!((sim.dots[0].x, sim.dots[0].y), (8, 8));
    assert!(!Dots::seat_alive(&sim, 8));
}

#[test]
fn the_last_dot_in_wins_and_none_left_is_a_draw() {
    let dot = Dot {
        x: 3,
        y: 3,
        heading: 0,
        alive: true,
    };
    let mut sim = one(dot);
    assert_eq!(Dots::round_winner(&sim), RoundEnd::Won(0));
    sim.dots[5] = dot;
    assert_eq!(Dots::round_winner(&sim), RoundEnd::Going);
    sim.dots[0].alive = false;
    assert_eq!(Dots::round_winner(&sim), RoundEnd::Won(5));
    sim.dots[5].alive = false;
    assert_eq!(Dots::round_winner(&sim), RoundEnd::Draw);
}

/// FNV-1a, 64 bits: the reference, checked against its published values.
fn fnv1a(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    hash
}

#[test]
fn the_hash_is_fnv_1a_of_the_steps_then_each_dot() {
    assert_eq!(fnv1a(b""), 0xcbf2_9ce4_8422_2325);
    assert_eq!(fnv1a(b"a"), 0xaf63_dc4c_8601_ec8c);
    let mut sim = Dots.new_round(2614, 1);
    sim.steps = 0x0102_0304;
    let mut bytes = vec![4, 3, 2, 1];
    for dot in &sim.dots {
        bytes.extend([dot.x, dot.y, dot.heading, u8::from(dot.alive)]);
    }
    assert_eq!(sim.hash(), fnv1a(&bytes));
}

#[test]
fn the_seed_picks_each_dots_cell_and_heading() {
    let starts: Vec<DotsSim> = (0..32).map(|seed| Dots.new_round(seed, 1)).collect();
    for (seed, sim) in (0..).zip(&starts) {
        assert_eq!(&Dots.new_round(seed, 7), sim);
        assert_eq!(sim.steps, 0);
        for dot in &sim.dots {
            assert!(dot.alive && dot.x < SIDE && dot.y < SIDE && dot.heading < 4);
        }
    }
    // Every heading and every column comes up, and no two seeds agree.
    let dots = || starts.iter().flat_map(|sim| sim.dots);
    for heading in 0..4 {
        assert!(dots().any(|dot| dot.heading == heading));
    }
    for x in 0..SIDE {
        assert!(dots().any(|dot| dot.x == x));
    }
    for (n, sim) in starts.iter().enumerate() {
        assert!(starts[n + 1..].iter().all(|other| other != sim));
    }
}

#[test]
fn the_bot_turns_one_step_in_six_and_goes_out_one_in_forty() {
    let sim = Dots.new_round(1, 1);
    let mut rng = SimRng::new(2614);
    let (mut turns, mut outs) = (0, 0);
    for _ in 0..12_000 {
        let input = Dots::bot_input(&sim, 0, &mut rng);
        turns += input
            .pressed
            .iter()
            .filter(|a| [UP, RIGHT, DOWN, LEFT].contains(a))
            .count();
        outs += usize::from(input.is_pressed(OUT));
        assert!(input.held.is_empty() && input.released.is_empty());
    }
    // 2,000 and 300 expected.
    assert!((1_700..2_300).contains(&turns), "{turns} turns");
    assert!((200..400).contains(&outs), "{outs} outs");
}

#[test]
fn the_games_numbers_and_stamp() {
    assert_eq!(
        (Dots::SEATS, Dots::STEPS_PER_SECOND, Dots::MAX_ROUND_STEPS),
        (8, 15, 600)
    );
    assert_eq!(Dots::PAUSE_STEPS, 3);
    assert_eq!(Dots.made().game, "dots");
}
