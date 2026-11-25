use shuyi::core::ScriptedRng;
use shuyi::core::tick::SeatInput;

use super::*;

/// A test board's header, up to its features.
const LEGEND: &str = "(name: \"t\", legend: { '.': \"floor\", '#': \"wall\" }, features: { ";

/// The board of `rows`, `S` the head's start.
fn board(rows: &str) -> Board {
    let mut start = String::new();
    let mut tiles = String::new();
    for (y, row) in rows.lines().enumerate() {
        for (x, c) in row.chars().enumerate() {
            if c == 'S' {
                start = format!("({x}, {y}): Start");
            }
            tiles.push(if c == 'S' { '.' } else { c });
        }
        tiles.push('\n');
    }
    let source = format!("{LEGEND}{start} }})\n---\n{tiles}");
    grid_map::parse("t.map", &source, &FORMAT, terrain_named).unwrap_or_else(|e| panic!("{e:?}"))
}

/// Numbers that move the snake every `period` steps, from 3 tiles long,
/// a step sooner every 2 points down to 2.
fn tuning(period: u32) -> Tuning {
    Tuning {
        start_length: 3,
        first_period: period,
        fastest_period: 2,
        points_per_speed_up: 2,
    }
}

/// A run of `rows`, moving every step, seeded `seed`.
fn run(rows: &str, seed: u64) -> Snake {
    Snake::new(&board(rows), tuning(1), seed).unwrap_or_else(|e| panic!("{e}"))
}

/// A step's input pressing `actions`.
fn press(actions: &[UiAction]) -> TickInput {
    let ids = actions.iter().map(|&a| id(a)).collect();
    TickInput::one(SeatInput::new(ids, Vec::new(), Vec::new()))
}

/// A step with nothing pressed.
fn idle() -> TickInput {
    TickInput::default()
}

const OPEN: &str = "\
##########
#........#
#..S.....#
#........#
#........#
##########";

#[test]
fn a_run_starts_three_long_heading_right_with_food_off_its_body() {
    let snake = run(OPEN, 1);
    assert_eq!(
        snake.body(),
        [Pos::new(3, 2), Pos::new(2, 2), Pos::new(1, 2)]
    );
    assert_eq!(snake.heading(), Dir::Right);
    let food = snake.food().unwrap_or_else(|| panic!("no food"));
    assert!(!snake.body().contains(&food), "{food:?}");
    assert_eq!(snake.board().get(food), Some(&FLOOR));
    assert_eq!(
        (snake.score(), snake.length(), snake.outcome()),
        (0, 3, None)
    );
    assert_eq!(snake.floor_tiles(), 32);
    // The same seed, the same food.
    assert_eq!(run(OPEN, 1).food(), snake.food());
}

#[test]
fn a_board_needs_one_start_with_floor_under_the_snake_and_to_fit() {
    let tuning = tuning(8);
    let none = Snake::new(&board("#####\n#...#\n#####"), tuning, 0).unwrap_err();
    assert!(none.contains("0 Start tiles"), "{none}");
    let walled = Snake::new(&board("#####\n#.S.#\n#####"), tuning, 0).unwrap_err();
    assert!(walled.contains("(0, 1), which is not floor"), "{walled}");
    let wide = format!(
        "{}\n#S{}#\n{}",
        "#".repeat(26),
        ".".repeat(23),
        "#".repeat(26)
    );
    let wide = Snake::new(&board(&wide), tuning, 0).unwrap_err();
    assert!(wide.contains("26 × 3 tiles"), "{wide}");
    assert_eq!(terrain_named("wall"), Some(WALL));
    assert_eq!(terrain_named("lava"), None);
}

#[test]
fn it_moves_one_tile_each_time_its_timer_fires() {
    let mut snake = Snake::new(&board(OPEN), tuning(3), 0).unwrap_or_else(|e| panic!("{e}"));
    for _ in 0..2 {
        let events = snake.step(&idle());
        assert!(events.is_empty(), "{events:?}");
    }
    let events = snake.step(&idle());
    assert_eq!(
        events,
        [Event::Moved {
            head: Pos::new(4, 2)
        }]
    );
    assert_eq!(
        snake.body(),
        [Pos::new(4, 2), Pos::new(3, 2), Pos::new(2, 2)]
    );
    assert_eq!((snake.steps(), snake.period()), (3, 3));
}

#[test]
fn a_press_turns_it_at_its_next_move_and_two_quick_presses_both_count() {
    let mut snake = Snake::new(&board(OPEN), tuning(4), 0).unwrap_or_else(|e| panic!("{e}"));
    // Down and Left between two moves: both wait.
    snake.step(&press(&[UiAction::Down]));
    snake.step(&press(&[UiAction::Left]));
    assert_eq!(snake.turns(), [Dir::Down, Dir::Left]);
    // A third waits for none.
    snake.step(&press(&[UiAction::Up]));
    assert_eq!(snake.turns(), [Dir::Down, Dir::Left]);
    let events = snake.step(&idle());
    assert_eq!(
        events,
        [
            Event::Turned { to: Dir::Down },
            Event::Moved {
                head: Pos::new(3, 3)
            }
        ]
    );
    for _ in 0..4 {
        snake.step(&idle());
    }
    assert_eq!((snake.head(), snake.heading()), (Pos::new(2, 3), Dir::Left));
    let waiting = snake.turns();
    assert!(waiting.is_empty(), "{waiting:?}");
}

#[test]
fn a_turn_back_into_its_neck_or_the_way_it_goes_is_ignored() {
    let mut snake = run(OPEN, 0);
    snake.step(&press(&[UiAction::Left, UiAction::Right]));
    assert_eq!(
        (snake.head(), snake.heading()),
        (Pos::new(4, 2), Dir::Right)
    );
    // Behind a turn that waits, it is the turn's way back that is ignored.
    let mut snake = Snake::new(&board(OPEN), tuning(4), 0).unwrap_or_else(|e| panic!("{e}"));
    snake.step(&press(&[UiAction::Down, UiAction::Up, UiAction::Left]));
    assert_eq!(snake.turns(), [Dir::Down, Dir::Left]);
    // Confirm and Cancel turn nothing.
    snake.step(&press(&[UiAction::Confirm, UiAction::Cancel]));
    assert_eq!(snake.turns(), [Dir::Down, Dir::Left]);
}

#[test]
fn eating_scores_and_grows_it_by_one_at_its_next_move() {
    let mut snake = run(OPEN, 0);
    // The food just ahead, as the luck may put it.
    snake.food = Some(Pos::new(4, 2));
    let events = snake.step(&idle());
    assert_eq!(
        events[..2],
        [
            Event::Moved {
                head: Pos::new(4, 2)
            },
            Event::Ate { score: 1 }
        ]
    );
    assert_eq!((snake.score(), snake.length()), (1, 3));
    let next = snake.food().unwrap_or_else(|| panic!("no food"));
    assert!(!snake.body().contains(&next), "{next:?}");
    snake.step(&idle());
    assert_eq!(snake.length(), 4);
    assert_eq!(snake.body().last(), Some(&Pos::new(2, 2)));
    snake.step(&idle());
    assert_eq!(snake.length(), 4);
}

#[test]
fn every_few_points_it_moves_a_step_sooner_down_to_the_fastest() {
    let mut snake = Snake::new(&board(OPEN), tuning(4), 0).unwrap_or_else(|e| panic!("{e}"));
    let mut periods = Vec::new();
    for _ in 0..6 {
        snake.eat(&mut Vec::new());
        periods.push(snake.period());
    }
    // Every 2 points, from 4 down to 2.
    assert_eq!(periods, [4, 3, 3, 2, 2, 2]);
}

#[test]
fn a_wall_ends_the_run_and_nothing_moves_after() {
    let mut snake = run("#####\n#..S#\n#####", 0);
    snake.food = None;
    let events = snake.step(&idle());
    assert_eq!(events, [Event::Died]);
    assert_eq!(snake.outcome(), Some(Outcome::Died { step: 1, length: 3 }));
    assert_eq!(snake.head(), Pos::new(3, 1));
    let events = snake.step(&press(&[UiAction::Up]));
    assert!(events.is_empty(), "{events:?}");
    assert_eq!(snake.steps(), 1);
}

#[test]
fn its_own_body_ends_the_run_but_its_tail_moves_away() {
    // Five long, curled: right, down, left, then up into its body.
    let curled = "\
########
#......#
#....S.#
#......#
#......#
########";
    let mut snake = Snake::new(
        &board(curled),
        Tuning {
            start_length: 5,
            ..tuning(1)
        },
        0,
    )
    .unwrap_or_else(|e| panic!("{e}"));
    snake.food = None;
    snake.step(&press(&[UiAction::Down]));
    snake.step(&press(&[UiAction::Left]));
    let events = snake.step(&press(&[UiAction::Up]));
    assert_eq!(events.last(), Some(&Event::Died));
    assert!(matches!(
        snake.outcome(),
        Some(Outcome::Died { step: 3, .. })
    ));

    // Four long, it chases its tail round a square and lives.
    let mut snake = Snake::new(
        &board(curled),
        Tuning {
            start_length: 4,
            ..tuning(1)
        },
        0,
    )
    .unwrap_or_else(|e| panic!("{e}"));
    snake.food = None;
    for turn in [
        UiAction::Down,
        UiAction::Left,
        UiAction::Up,
        UiAction::Right,
    ] {
        snake.step(&press(&[turn]));
    }
    assert_eq!(snake.outcome(), None);
}

#[test]
fn filling_the_board_ends_the_run() {
    // Four floor tiles, the snake two long on the top two.
    let small = board("####\n#.S#\n#..#\n####");
    let two = Tuning {
        start_length: 2,
        ..tuning(1)
    };
    let mut snake = Snake::new(&small, two, 0).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(snake.floor_tiles(), 4);
    snake.food = Some(Pos::new(2, 2));
    let events = snake.step(&press(&[UiAction::Down]));
    assert!(events.contains(&Event::Ate { score: 1 }), "{events:?}");
    snake.food = Some(Pos::new(1, 2));
    let events = snake.step(&press(&[UiAction::Left]));
    assert!(events.contains(&Event::Ate { score: 2 }), "{events:?}");
    // The one tile left, and no food: it grows into it.
    assert_eq!(snake.food(), Some(Pos::new(1, 1)));
    snake.food = None;
    let events = snake.step(&press(&[UiAction::Up]));
    assert_eq!(events.last(), Some(&Event::Filled));
    assert_eq!(snake.outcome(), Some(Outcome::Filled));
}

#[test]
fn food_is_never_placed_on_the_body() {
    let open = board(OPEN);
    let body = [Pos::new(1, 1), Pos::new(2, 1), Pos::new(3, 1)];
    // Draw 0 is the first free tile in reading order: past the body.
    let mut rng = ScriptedRng::new([]).with_below([0, 1]);
    assert_eq!(
        place_food(&open.tiles, &body, &mut rng),
        Some(Pos::new(4, 1))
    );
    assert_eq!(
        place_food(&open.tiles, &body, &mut rng),
        Some(Pos::new(5, 1))
    );
    // A board the body fills has no tile for it, and draws nothing.
    let full = board("###\n#S#\n###");
    let mut none = ScriptedRng::new([]);
    assert_eq!(place_food(&full.tiles, &[Pos::new(1, 1)], &mut none), None);
}

#[test]
fn the_same_seed_and_inputs_give_the_same_run() {
    let inputs = [
        press(&[UiAction::Down]),
        idle(),
        press(&[UiAction::Left]),
        idle(),
    ];
    let play = || {
        let mut snake = run(OPEN, 7);
        inputs
            .iter()
            .map(|i| (snake.step(i), snake.hash()))
            .collect::<Vec<_>>()
    };
    assert_eq!(play(), play());
    assert_ne!(run(OPEN, 7).hash(), run(OPEN, 8).hash());
}
