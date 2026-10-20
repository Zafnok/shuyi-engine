//! The game played by keys from its title to the end of a stage: Play,
//! the scene read, the stage walked to the goal, the end screen and back
//! to the title; caught by the foe; and leaving the stage.

use std::collections::BTreeMap;

use hello_grid::map::MOVE;
use hello_grid::rules::{Outcome, Side, TRIP_PERCENT};
use hello_grid::screens::{End, Leave, StageScreen, Talk, Title};
use shuyi_assets::dialogue::{DialogueTable, NoDirective, parse_dlg};
use shuyi_core::{Pos, Sides, SimRng};
use shuyi_platform::harness::Harness;
use shuyi_ui::grid::MapUnit;
use shuyi_ui::{Game, GameHooks};

use crate::{play_out, stage, title, to_the_foe, to_the_goal, walk_to};

#[test]
fn the_title_opens_the_stage_with_its_scene_read_over_it() {
    let mut h = title(1);
    assert_eq!(h.screens(), ["title"]);
    let snapshot = h.snapshot();
    assert!(snapshot.contains("Hello, Grid"), "{snapshot}");
    assert!(
        snapshot.contains("arrows choose · Enter pick"),
        "{snapshot}"
    );
    h.keys("Enter");
    assert_eq!(h.screens(), ["title", "stage", "talk"]);
    // It types out, then waits for Confirm, which goes on to the next line.
    h.wait(5.0);
    let talk = h.screen::<Talk>().unwrap();
    assert!(talk.text.is_revealed());
    let first = talk.player.current().text.clone().unwrap();
    assert!(first.starts_with("A small grid"), "{first}");
    assert_eq!(talk.speaker(h.ctx()), None);
    // Over the stage, which shows under the text box.
    let snapshot = h.snapshot();
    assert!(snapshot.contains("A small grid"), "{snapshot}");
    assert!(snapshot.contains("Turn 1"), "{snapshot}");
    h.keys("Enter");
    let talk = h.screen::<Talk>().unwrap();
    assert!(!talk.text.is_revealed());
    assert_eq!(talk.speaker(h.ctx()).as_deref(), Some("Guide"));
    // Confirm reveals the page, then goes on, to the scene's end.
    for _ in 0..3 {
        h.keys("Enter Enter");
    }
    assert_eq!(h.screens(), ["title", "stage"]);
}

#[test]
fn quit_on_the_title_quits() {
    let mut h = title(1);
    h.keys("Down Down Down");
    assert_eq!(h.screen::<Title>().map(|t| t.focus), Some(3));
    assert!(!h.quit_requested());
    h.keys("Enter");
    assert!(h.quit_requested());
}

#[test]
fn a_long_line_is_read_a_page_at_a_time() {
    // A line of 200 characters: two pages of the box's three lines of 42.
    let long = "word ".repeat(40);
    let source = format!(
        "@scene long
> {long}
> Short.
@end
"
    );
    let (parsed, errors) = parse_dlg::<NoDirective>("long.dlg", &source);
    assert!(errors.is_empty(), "{errors:?}");
    let mut ctx = hello_grid::ctx(0);
    ctx.game.dialogue = DialogueTable {
        scenes: parsed
            .into_iter()
            .map(|p| (p.scene.id.clone(), p.scene))
            .collect(),
        files: BTreeMap::new(),
    };
    let talk = Talk::new(&ctx, "long");
    let mut h = Harness::new(Game::new(ctx, Box::new(talk), GameHooks::default()));
    let line = |h: &Harness<hello_grid::Hello>| {
        let talk = h.screen::<Talk>().unwrap();
        let text = talk.player.current().text.unwrap_or_default();
        (text.into_owned(), talk.text.clone())
    };
    let (first, page) = line(&h);
    assert!(!page.is_revealed());
    // Confirm shows the whole page, still the first.
    h.keys("Enter");
    let (text, shown) = line(&h);
    assert_eq!(text, first);
    assert!(shown.is_revealed());
    assert_eq!(shown.page_lines(), page.page_lines());
    // Confirm again: the line's next page, typing out.
    h.keys("Enter");
    let (text, next) = line(&h);
    assert_eq!(text, first);
    assert!(!next.is_revealed());
    assert_ne!(next.page_lines(), page.page_lines());
    // Shown whole, then on to the next line.
    h.keys("Enter Enter");
    assert_eq!(line(&h).0, "Short.");
}

#[test]
fn cancel_skips_the_scene() {
    let mut h = title(1);
    h.keys("Enter Backspace");
    assert_eq!(h.screens(), ["title", "stage"]);
}

#[test]
fn the_stage_is_played_by_keys_to_the_goal_and_the_end_says_so() {
    let mut h = title(7);
    h.keys("Enter Backspace");
    let start = stage(&h);
    let goal = start.stage.goal();
    assert_eq!(start.stage.hero(), Pos::new(1, 8));
    assert_eq!(start.stage.foe(), Pos::new(9, 7));
    assert_eq!(goal, Pos::new(10, 1));
    // The map shows both pieces, and the cursor on yours.
    let scene = start.scene(StageScreen::view_tiles(h.ctx()));
    assert_eq!(scene.cursor_tile(), Some(Pos::new(1, 8)));
    assert_eq!(
        scene.unit_at(Pos::new(9, 7)).map(MapUnit::text).as_deref(),
        Some("& (9,7)")
    );
    let turns = play_out(&mut h, to_the_goal);
    assert_eq!(h.screens(), ["title", "end"]);
    let end = h.screen::<End>().unwrap();
    assert_eq!(end.outcome, Outcome::Won { turn: turns });
    let view = end.view(h.ctx());
    assert_eq!(
        view.headline,
        format!("You reached the goal on turn {turns}!")
    );
    assert!(view.won);
    assert!(h.snapshot().contains(&view.headline));
    // Back to the title.
    h.keys("Down Enter");
    assert_eq!(h.screens(), ["title"]);
}

#[test]
fn walking_into_the_foe_gets_you_caught_and_play_again_starts_anew() {
    let mut h = title(3);
    h.keys("Enter Backspace");
    play_out(&mut h, to_the_foe);
    let end = h.screen::<End>().unwrap();
    let Outcome::Caught { turn } = end.outcome else {
        panic!("{:?}", end.outcome);
    };
    let view = end.view(h.ctx());
    assert_eq!(view.headline, format!("The foe caught you on turn {turn}."));
    assert!(!view.won);
    // Play again: a new stage, its scene again, on the next seed.
    h.keys("Enter");
    assert_eq!(h.screens(), ["title", "stage", "talk"]);
    h.keys("Backspace");
    let again = stage(&h);
    assert_eq!(again.stage.turn(), 1);
    assert_eq!(again.stage.side_to_move(), Side::Hero);
    assert!(again.commands.is_empty());
    assert_eq!(h.ctx().game.seed, 5);
}

#[test]
fn a_picked_up_piece_shows_its_reach_and_the_path_follows_the_cursor() {
    let mut h = title(1);
    h.keys("Enter Backspace");
    let help = stage(&h).view(h.ctx()).help;
    assert_eq!(help, "arrows move · Enter pick up · Backspace leave");
    // Confirm on an empty tile does nothing.
    h.keys("Right Enter");
    assert!(stage(&h).selected.is_none());
    h.keys("Left Enter");
    let s = stage(&h);
    let (reach, path) = s.selected.clone().unwrap();
    assert_eq!(path, [Pos::new(1, 8)]);
    assert!(reach.contains(Pos::new(4, 7)));
    let scene = s.scene(StageScreen::view_tiles(h.ctx()));
    let reach_tiles: Vec<Pos> = reach.tiles().iter().collect();
    let mut tinted = scene.tinted(MOVE);
    tinted.sort_by_key(|p| (p.y, p.x));
    assert_eq!(tinted, reach_tiles);
    assert_eq!(scene.path, [Pos::new(1, 8)]);
    let help = s.view(h.ctx()).help;
    assert_eq!(help, "arrows steer · Enter walk · Backspace put down");
    h.keys("Up Right");
    assert_eq!(
        stage(&h).selected.as_ref().unwrap().1,
        [Pos::new(1, 8), Pos::new(1, 7), Pos::new(2, 7)]
    );
    // Confirm off the reach keeps it picked up; Cancel puts it down.
    h.keys("Up Up Enter");
    assert!(stage(&h).selected.is_some());
    assert!(stage(&h).commands.is_empty());
    h.keys("Backspace");
    assert!(stage(&h).selected.is_none());
    assert_eq!(h.screens(), ["title", "stage"]);
}

/// A seed whose foe doesn't trip on its first step.
fn steady() -> u64 {
    (0..1000)
        .find(|&seed| SimRng::new(seed).roll_percent() >= TRIP_PERCENT)
        .unwrap_or_else(|| panic!("no steady seed"))
}

fn p(x: i32, y: i32) -> Pos {
    Pos::new(x, y)
}

#[test]
fn the_path_is_the_one_steered_whichever_way_round() {
    for (keys, path) in [
        (
            "Up Right Right Right",
            [p(1, 8), p(1, 7), p(2, 7), p(3, 7), p(4, 7)],
        ),
        (
            "Right Up Right Right",
            [p(1, 8), p(2, 8), p(2, 7), p(3, 7), p(4, 7)],
        ),
    ] {
        let mut h = title(1);
        h.keys("Enter Backspace Enter");
        h.keys(keys);
        assert_eq!(stage(&h).selected.as_ref().unwrap().1, path, "{keys}");
    }
}

#[test]
fn a_walk_is_shown_a_tile_at_a_time_then_the_foes_step() {
    let mut h = title(steady());
    h.keys("Enter Backspace Enter Up Right Right Right Enter");
    let s = stage(&h);
    // The rules moved your piece at once; the map shows it setting off,
    // and the foe where it stands.
    assert_eq!(s.stage.hero(), p(4, 7));
    let walk = s.walk.clone().unwrap();
    assert_eq!(walk.side, Side::Hero);
    assert_eq!(walk.path, [p(1, 8), p(1, 7), p(2, 7), p(3, 7), p(4, 7)]);
    let shown = |h: &Harness<_>, side| {
        let s = stage(h);
        let scene = s.scene(StageScreen::view_tiles(h.ctx()));
        scene.unit(side).map(|u| u.pos)
    };
    assert_eq!(shown(&h, Side::Hero), Some(p(1, 8)));
    assert_eq!(shown(&h, Side::Foe), Some(p(9, 7)));
    let scene = s.scene(StageScreen::view_tiles(h.ctx()));
    assert!(scene.cursor.is_none());
    assert_eq!(s.view(h.ctx()).help, "");
    // Eight tiles a second: two tiles on a quarter of a second later.
    h.wait(0.25);
    assert_eq!(shown(&h, Side::Hero), Some(p(2, 7)));
    // Keys do nothing while it walks.
    h.keys("Backspace");
    assert!(!stage(&h).leaving);
    // Then the foe's step is shown, your piece on its new tile.
    let mut foe = None;
    for _ in 0..120 {
        h.wait(1.0 / 60.0);
        foe = stage(&h).walk.clone().filter(|w| w.side == Side::Foe);
        if foe.is_some() {
            break;
        }
    }
    let foe = foe.unwrap_or_else(|| panic!("the foe never walked"));
    assert_eq!(foe.path, [p(9, 7), stage(&h).stage.foe()]);
    assert_eq!(shown(&h, Side::Hero), Some(p(4, 7)));
    h.wait(1.0);
    let s = stage(&h);
    assert!(s.walk.is_none());
    assert_eq!(s.stage.side_to_move(), Side::Hero);
    assert_eq!(s.commands.len(), 2);
    assert_eq!(shown(&h, Side::Foe), Some(s.stage.foe()));
    let scene = s.scene(StageScreen::view_tiles(h.ctx()));
    assert_eq!(scene.cursor_tile(), Some(p(4, 7)));
}

#[test]
fn leaving_the_stage_asks_first() {
    let mut h = title(1);
    h.keys("Enter Backspace Backspace");
    // Asked on a screen of its own, over the stage.
    assert_eq!(h.screens(), ["title", "stage", "leave"]);
    assert!(stage(&h).leaving);
    let view = Leave::view(h.ctx());
    assert_eq!(view.question, "Leave the stage?");
    assert_eq!(view.answers, "Enter yes / Backspace no");
    let snapshot = h.snapshot();
    assert!(snapshot.contains("Leave the stage?"), "{snapshot}");
    assert!(snapshot.contains("Turn 1"), "the stage shows: {snapshot}");
    assert!(!snapshot.contains("pick up"), "not its help: {snapshot}");
    let s = stage(&h);
    assert_eq!(s.scene(StageScreen::view_tiles(h.ctx())).cursor, None);
    // The cursor waits meanwhile.
    h.keys("Up");
    assert_eq!(stage(&h).cursor.pos, Pos::new(1, 8));
    // No: back on the stage, which reads the answer.
    h.keys("Backspace Up");
    assert_eq!(h.screens(), ["title", "stage"]);
    assert!(!stage(&h).leaving);
    assert_eq!(stage(&h).cursor.pos, Pos::new(1, 7));
    // Yes: the stage leaves in the same frame, back at the title.
    h.keys("Escape");
    assert_eq!(h.top_screen(), "leave");
    h.keys("Enter");
    assert_eq!(h.screens(), ["title"]);
}

#[test]
fn the_foes_trip_is_said_until_your_next_move() {
    // A seed whose foe trips on its first step.
    let tripped = |seed: u64| {
        let mut h = title(seed);
        h.keys("Enter Backspace");
        walk_to(&mut h, Pos::new(2, 7));
        h
    };
    let seed = (0..1000)
        .find(|&seed| stage(&tripped(seed)).tripped)
        .unwrap();
    let mut h = tripped(seed);
    let s = stage(&h);
    assert_eq!(s.stage.foe(), Pos::new(9, 7));
    assert_eq!(s.view(h.ctx()).message, "The foe tripped!");
    assert_eq!(s.view(h.ctx()).turn, "Turn 2");
    // Your next move: said no more, unless it trips again.
    walk_to(&mut h, Pos::new(3, 7));
    let s = stage(&h);
    let said = s.view(h.ctx()).message;
    assert_eq!(said == "The foe tripped!", s.tripped, "{said}");
    assert_eq!(s.stage.foe() == Pos::new(9, 7), s.tripped);
}
