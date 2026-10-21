//! Hello, Grid played through the engine's test Harness, by its bots and
//! through its tools: the playthrough to the win, the bots over its
//! `TurnRules` and a playtest of them, the tool contract's operations,
//! the same seed and commands giving the same game, its options and key
//! rebinding, its profiles, its files, its look on a one-bit screen, its pack, pictures
//! of its frames, and a sleep and a wake.

mod bots;
mod contract;
mod controls;
mod determinism;
mod files;
mod onebit;
mod options;
mod pack;
mod playtest;
mod playthrough;
mod profiles;
mod snapshot;
mod suspend;

use hello_grid::Hello;
use hello_grid::rules::Stage;
use hello_grid::screens::StageScreen;
use hello_grid::tools::Start;
use shuyi_core::{Pos, TurnRules, flow_field};
use shuyi_platform::harness::Harness;

/// The game at its title, its first stage seeded with `seed`.
pub fn title(seed: u64) -> Harness<Hello> {
    Start {
        seed,
        ..Start::default()
    }
    .harness()
}

/// The stage screen `h` shows.
pub fn stage(h: &Harness<Hello>) -> &StageScreen {
    h.screen::<StageScreen>()
        .unwrap_or_else(|| panic!("no stage: {:?}", h.screens()))
}

/// The arrow keys that take the cursor from `from` to `to`: across, then
/// down or up.
pub fn arrows(from: Pos, to: Pos) -> String {
    let across = if to.x > from.x { "Right" } else { "Left" };
    let down = if to.y > from.y { "Down" } else { "Up" };
    let steps =
        |n: i32, key: &'static str| vec![key; usize::try_from(n.unsigned_abs()).unwrap_or(0)];
    let mut keys = steps(to.x - from.x, across);
    keys.extend(steps(to.y - from.y, down));
    keys.join(" ")
}

/// The tile of your reach nearest `to`, counting steps over the tiles
/// `cost` lets you walk.
fn nearest(stage: &Stage, to: Pos, cost: impl Fn(Pos) -> Option<u32>) -> Pos {
    let map = stage.map();
    let far = flow_field(map.width(), map.height(), [to], cost);
    let distance = |pos: Pos| far.get(pos).copied().flatten().unwrap_or(u32::MAX);
    let reach = stage.reach().tiles();
    reach
        .iter()
        .min_by_key(|&pos| distance(pos))
        .unwrap_or(stage.hero())
}

/// Where to walk this turn to head for the goal: the tile of your reach
/// nearest it, round the foe and the tiles beside it.
pub fn to_the_goal(stage: &Stage) -> Pos {
    let (foe, goal) = (stage.foe(), stage.goal());
    nearest(stage, goal, |pos| {
        let clear = Pos::manhattan(pos, foe) > 1 || pos == goal;
        (stage.walkable(pos) && clear).then_some(1)
    })
}

/// Where to walk this turn to meet the foe: the tile of your reach
/// nearest it.
pub fn to_the_foe(stage: &Stage) -> Pos {
    nearest(stage, stage.foe(), |pos| stage.walkable(pos).then_some(1))
}

/// Plays one turn on the stage by keys: the cursor to your piece, Confirm,
/// the cursor to `to`, Confirm, and time for both pieces to walk.
pub fn walk_to(h: &mut Harness<Hello>, to: Pos) {
    let s = stage(h);
    let (cursor, hero) = (s.cursor.pos, s.stage.hero());
    h.keys(&arrows(cursor, hero));
    h.keys("Enter");
    h.keys(&arrows(hero, to));
    h.keys("Enter");
    h.wait(1.0);
}

/// Plays the stage `h` shows to its end, each turn walking to where
/// `choose` says; returns the turns played.
pub fn play_out(h: &mut Harness<Hello>, choose: impl Fn(&Stage) -> Pos) -> u32 {
    for turn in 1..=30 {
        if h.top_screen() != "stage" {
            return turn - 1;
        }
        let s = &stage(h).stage;
        assert!(s.outcome().is_none());
        let to = choose(s);
        walk_to(h, to);
    }
    panic!("no end after 30 turns: {:?}", h.screens());
}
