use super::*;

fn p(x: i32, y: i32) -> Pos {
    Pos::new(x, y)
}

/// A stage of `rows` (`.` floor, `#` wall, `>` goal) with the pieces on
/// `hero` and `foe`, its luck seeded with `seed`.
fn stage_of(rows: &str, hero: (i32, i32), foe: (i32, i32), seed: u64) -> Result<Stage, String> {
    let source = format!(
        "(name: \"t\", legend: {{ '.': \"floor\", '#': \"wall\", '>': \"goal\" }}, \
         features: {{ {hero:?}: Hero, {foe:?}: Foe }})\n---\n{rows}"
    );
    Stage::new(&parse_map("t.map", &source)?, seed)
}

/// A seed whose first roll trips the foe (`trips`) or doesn't.
fn seed_where(trips: bool) -> u64 {
    (0..1000)
        .find(|&s| (SimRng::new(s).roll_percent() < TRIP_PERCENT) == trips)
        .unwrap()
}

/// A corridor: hero, then floor, foe, floor, and the goal at the end.
const CORRIDOR: &str = "#######\n#....>#\n#######";

fn corridor(seed: u64) -> Stage {
    stage_of(CORRIDOR, (1, 1), (3, 1), seed).unwrap()
}

#[test]
fn the_stage_starts_on_your_move_with_the_pieces_on_their_tiles() {
    let s = corridor(0);
    assert_eq!((s.hero(), s.foe(), s.goal()), (p(1, 1), p(3, 1), p(5, 1)));
    assert_eq!(
        (s.side_to_move(), s.turn(), s.outcome()),
        (Side::Hero, 1, None)
    );
    assert_eq!((s.map().width(), s.map().height()), (7, 3));
    assert!(s.walkable(p(2, 1)) && s.walkable(p(5, 1)));
    assert!(!s.walkable(p(0, 1)) && !s.walkable(p(9, 9)));
}

#[test]
fn a_map_needs_one_goal_and_one_start_on_floor_for_each_piece() {
    assert_eq!(
        stage_of("#....#", (1, 0), (3, 0), 0).unwrap_err(),
        "the map has 0 goal tiles, not one"
    );
    assert_eq!(
        stage_of(">..>", (1, 0), (2, 0), 0).unwrap_err(),
        "the map has 2 goal tiles, not one"
    );
    assert_eq!(
        stage_of("#..>", (0, 0), (2, 0), 0).unwrap_err(),
        "Hero at (0, 0) is not on floor"
    );
    assert_eq!(
        stage_of("...>", (1, 0), (3, 0), 0).unwrap_err(),
        "Foe at (3, 0) is not on floor"
    );
    let source = "(name: \"t\", legend: { '.': \"floor\", '>': \"goal\" }, \
                  features: { (0, 0): Hero })\n---\n..>";
    let map = parse_map("t.map", source).unwrap();
    assert_eq!(
        Stage::new(&map, 0).unwrap_err(),
        "the map has 0 Foe tiles, not one"
    );
    let bad = "(name: \"t\", legend: { '.': \"lava\" })\n---\n.";
    assert!(
        parse_map("t.map", bad)
            .unwrap_err()
            .contains("unknown terrain \"lava\"")
    );
}

#[test]
fn terrains_are_named_in_id_order() {
    assert_eq!(terrain_named("floor"), Some(FLOOR));
    assert_eq!(terrain_named("wall"), Some(WALL));
    assert_eq!(terrain_named("goal"), Some(GOAL));
    assert_eq!(terrain_named("lava"), None);
}

#[test]
fn your_piece_reaches_four_tiles_of_floor_round_the_foe() {
    let open = "#########\n#.......#\n#.......#\n#......>#\n#########";
    let s = stage_of(open, (1, 1), (2, 1), 0).unwrap();
    let reach = s.reach();
    assert!(reach.contains(p(1, 1)), "its own tile");
    assert!(!reach.contains(p(2, 1)), "the foe's tile");
    assert!(reach.contains(p(4, 2)), "four tiles away");
    assert!(!reach.contains(p(5, 2)), "five tiles away");
    assert!(reach.contains(p(3, 3)) && reach.contains(p(1, 3)));
    assert!(!reach.contains(p(5, 1)), "six tiles away, round the foe");
    assert!(!reach.contains(p(0, 1)), "a wall");
    assert_eq!(reach.budget(), MOVES);
    // Round the foe: (3, 1) is two steps right, but the foe is in the way.
    assert_eq!(reach.cost(p(3, 1)), Some(4));
}

#[test]
fn your_move_walks_your_piece_then_it_is_the_foes() {
    let mut s = corridor(0);
    let events = s.apply(&Command::Move(p(2, 1))).unwrap();
    assert_eq!(
        events,
        [Event::Moved {
            from: p(1, 1),
            to: p(2, 1)
        }]
    );
    assert_eq!(
        (s.hero(), s.side_to_move(), s.turn()),
        (p(2, 1), Side::Foe, 1)
    );
    // Waiting is walking to your own tile.
    let mut s = corridor(0);
    assert_eq!(s.apply(&Command::Move(p(1, 1))).unwrap().len(), 1);
    assert_eq!(s.side_to_move(), Side::Foe);
}

#[test]
fn commands_out_of_turn_or_reach_are_refused_and_change_nothing() {
    let mut s = corridor(0);
    let before = s.clone();
    for (command, error) in [
        (Command::Move(p(3, 1)), Error::OutOfReach(p(3, 1))),
        (Command::Move(p(0, 0)), Error::OutOfReach(p(0, 0))),
        (Command::Step(None), Error::NotYourMove(Side::Hero)),
    ] {
        assert_eq!(s.check(&command), Err(error), "{command:?}");
        assert_eq!(s.apply(&command), Err(error), "{command:?}");
        assert_eq!(s, before);
    }
    s.apply(&Command::Move(p(2, 1))).unwrap();
    let before = s.clone();
    assert_eq!(
        s.apply(&Command::Move(p(1, 1))),
        Err(Error::NotYourMove(Side::Foe))
    );
    assert_eq!(
        s.apply(&Command::Step(Some(Dir::Up))),
        Err(Error::Blocked(Dir::Up))
    );
    assert_eq!(s, before);
    assert_eq!(s.check(&Command::Step(Some(Dir::Left))), Ok(()));
    assert_eq!(s.check(&Command::Step(None)), Ok(()));
}

#[test]
fn the_foe_steps_or_trips_by_the_seeded_luck_and_then_it_is_your_turn() {
    let mut s = corridor(seed_where(false));
    s.apply(&Command::Move(p(1, 1))).unwrap();
    assert_eq!(
        s.apply(&Command::Step(Some(Dir::Right))).unwrap(),
        [Event::Stepped {
            from: p(3, 1),
            to: p(4, 1)
        }]
    );
    assert_eq!(
        (s.foe(), s.side_to_move(), s.turn()),
        (p(4, 1), Side::Hero, 2)
    );

    let mut s = corridor(seed_where(true));
    s.apply(&Command::Move(p(1, 1))).unwrap();
    assert_eq!(
        s.apply(&Command::Step(Some(Dir::Right))).unwrap(),
        [Event::Tripped]
    );
    assert_eq!(
        (s.foe(), s.side_to_move(), s.turn()),
        (p(3, 1), Side::Hero, 2)
    );

    // Staying rolls nothing: the next step is the first roll's.
    let mut s = corridor(seed_where(true));
    s.apply(&Command::Move(p(1, 1))).unwrap();
    assert_eq!(s.apply(&Command::Step(None)).unwrap(), [Event::Stayed]);
    assert_eq!(
        (s.foe(), s.side_to_move(), s.turn()),
        (p(3, 1), Side::Hero, 2)
    );
    s.apply(&Command::Move(p(1, 1))).unwrap();
    assert_eq!(
        s.apply(&Command::Step(Some(Dir::Left))).unwrap(),
        [Event::Tripped]
    );
}

#[test]
fn a_roll_of_exactly_the_trip_chance_does_not_trip() {
    let seed = (0..100_000)
        .find(|&s| SimRng::new(s).roll_percent() == TRIP_PERCENT)
        .unwrap();
    let mut s = corridor(seed);
    s.apply(&Command::Move(p(1, 1))).unwrap();
    assert_eq!(
        s.apply(&Command::Step(Some(Dir::Right))).unwrap(),
        [Event::Stepped {
            from: p(3, 1),
            to: p(4, 1)
        }]
    );
}

#[test]
fn your_piece_on_the_goal_wins_and_the_game_is_over() {
    let mut s = stage_of("#######\n#..>..#\n#######", (1, 1), (5, 1), 0).unwrap();
    let events = s.apply(&Command::Move(p(3, 1))).unwrap();
    assert_eq!(
        events,
        [
            Event::Moved {
                from: p(1, 1),
                to: p(3, 1)
            },
            Event::Reached
        ]
    );
    assert_eq!(s.outcome(), Some(Outcome::Won { turn: 1 }));
    let found = s.legal_commands();
    assert!(found.is_empty(), "{found:?}");
    assert_eq!(s.check(&Command::Step(None)), Err(Error::Over));
    assert_eq!(s.apply(&Command::Move(p(3, 1))), Err(Error::Over));
    assert_eq!(s.side_to_move(), Side::Hero);
}

#[test]
fn the_foe_stepping_onto_your_tile_catches_you() {
    let mut s = corridor(seed_where(false));
    s.apply(&Command::Move(p(2, 1))).unwrap();
    let events = s.apply(&Command::Step(Some(Dir::Left))).unwrap();
    assert_eq!(
        events,
        [
            Event::Stepped {
                from: p(3, 1),
                to: p(2, 1)
            },
            Event::Caught
        ]
    );
    assert_eq!(s.outcome(), Some(Outcome::Caught { turn: 1 }));
    assert_eq!((s.side_to_move(), s.turn()), (Side::Foe, 1));
    let found = s.legal_commands();
    assert!(found.is_empty(), "{found:?}");
    // A trip onto your tile catches no one.
    let mut s = corridor(seed_where(true));
    s.apply(&Command::Move(p(2, 1))).unwrap();
    s.apply(&Command::Step(Some(Dir::Left))).unwrap();
    assert_eq!(s.outcome(), None);
}

#[test]
fn the_legal_commands_are_the_reach_row_by_row_and_the_foes_open_steps() {
    let mut s = corridor(0);
    assert_eq!(
        s.legal_commands(),
        [Command::Move(p(1, 1)), Command::Move(p(2, 1))]
    );
    for command in s.legal_commands() {
        assert_eq!(s.check(&command), Ok(()));
    }
    s.apply(&Command::Move(p(1, 1))).unwrap();
    assert_eq!(
        s.legal_commands(),
        [
            Command::Step(None),
            Command::Step(Some(Dir::Right)),
            Command::Step(Some(Dir::Left))
        ]
    );
}

#[test]
fn the_chaser_steps_toward_you_and_stays_when_it_cant_get_closer() {
    // Open floor: the foe at (4, 2), you at (1, 2). Left gets closer.
    let open = "######\n#....#\n#....#\n#...>#\n######";
    let mut s = stage_of(open, (1, 2), (4, 2), 0).unwrap();
    s.apply(&Command::Move(p(1, 2))).unwrap();
    assert_eq!(Chase.choose(&s), Command::Step(Some(Dir::Left)));
    // Two steps as good (you up and to the left): the first of Dir::ALL,
    // Down before Left.
    let mut s = stage_of(open, (1, 3), (4, 1), 0).unwrap();
    s.apply(&Command::Move(p(1, 3))).unwrap();
    assert_eq!(Chase.choose(&s), Command::Step(Some(Dir::Down)));
    // Walled off from you: it stays.
    let walled = "#######\n#..#..#\n#..#.>#\n#######";
    let mut s = stage_of(walled, (1, 1), (4, 1), 0).unwrap();
    s.apply(&Command::Move(p(1, 1))).unwrap();
    assert_eq!(Chase.choose(&s), Command::Step(None));
    assert_eq!(s.distances_to_hero().get(p(4, 1)), Some(&None));
    assert_eq!(s.distances_to_hero().get(p(2, 2)), Some(&Some(2)));
}

#[test]
fn errors_say_what_was_refused() {
    assert_eq!(Error::Over.to_string(), "the stage is over");
    assert_eq!(
        Error::NotYourMove(Side::Foe).to_string(),
        "it is Foe's move"
    );
    assert_eq!(
        Error::OutOfReach(p(3, 4)).to_string(),
        "(3, 4) is out of reach"
    );
    assert_eq!(Error::Blocked(Dir::Up).to_string(), "the foe can't step Up");
}

fn tuned(seed: u64, tuning: Tuning) -> Stage {
    corridor(seed).retuned(tuning)
}

#[test]
fn the_tuning_sets_the_trip_chance() {
    let never = Tuning {
        trip_percent: 0,
        ..Tuning::default()
    };
    let mut s = tuned(seed_where(true), never);
    s.apply(&Command::Move(p(1, 1))).unwrap();
    assert_eq!(
        s.apply(&Command::Step(Some(Dir::Right))).unwrap(),
        [Event::Stepped {
            from: p(3, 1),
            to: p(4, 1)
        }]
    );
    let always = Tuning {
        trip_percent: 100,
        ..Tuning::default()
    };
    let mut s = tuned(seed_where(false), always);
    s.apply(&Command::Move(p(1, 1))).unwrap();
    assert_eq!(
        s.apply(&Command::Step(Some(Dir::Right))).unwrap(),
        [Event::Tripped]
    );
}

#[test]
fn a_foe_that_waits_skips_its_first_turns() {
    let waits = Tuning {
        foe_waits: 2,
        ..Tuning::default()
    };
    let mut s = tuned(0, waits);
    for turn in 1..=2 {
        assert_eq!(
            s.apply(&Command::Move(p(1, 1))).unwrap().last(),
            Some(&Event::Waited)
        );
        assert_eq!((s.side_to_move(), s.turn()), (Side::Hero, turn + 1));
    }
    s.apply(&Command::Move(p(1, 1))).unwrap();
    assert_eq!((s.side_to_move(), s.turn()), (Side::Foe, 3));
    assert_eq!(s.foe(), p(3, 1));
}

#[test]
fn only_a_tuning_other_than_the_default_is_written() {
    let plain = ron::to_string(&corridor(0)).unwrap();
    assert!(!plain.contains("tuning"), "{plain}");
    assert_eq!(corridor(0).tuning(), Tuning::default());
    let waits = Tuning {
        foe_waits: 1,
        ..Tuning::default()
    };
    let written = ron::to_string(&tuned(0, waits)).unwrap();
    assert!(written.contains("foe_waits:1"), "{written}");
    let map = parse_map(
        "t.map",
        "(name: \"t\", legend: { '.': \"floor\", '#': \"wall\", '>': \"goal\" }, \
         features: { (1, 1): Hero, (3, 1): Foe })\n---\n#######\n#....>#\n#######",
    )
    .unwrap();
    assert_eq!(Stage::with_tuning(&map, 0, waits).unwrap(), tuned(0, waits));
}
