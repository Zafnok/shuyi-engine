//! The fixture scripts' golden table: what three scripts give on fixed
//! views, the same on every platform (`shuyi_assets::golden`, ticket 2116).
//! A row that moves is a break: a game's rule would decide another way.

use serde::Serialize;
use shuyi_script::{Runtime, Script, Value};

fn compile(runtime: &Runtime, name: &str, source: &str) -> Script {
    runtime
        .compile(name, source)
        .unwrap_or_else(|e| panic!("{e}"))
}

#[derive(Serialize)]
enum Weather {
    Sun,
    Rain,
}

#[derive(Serialize)]
struct Crop {
    kind: &'static str,
    height: i64,
    base: i64,
    ripe_at: i64,
    weather: Weather,
    neighbours: Vec<&'static str>,
}

#[derive(Serialize)]
struct Switch {
    name: &'static str,
    on: bool,
}

#[derive(Serialize)]
struct Level {
    switches: Vec<Switch>,
}

#[derive(Serialize)]
struct Card {
    ally: bool,
}

#[derive(Serialize)]
struct Table {
    mine: Vec<Card>,
    hand: i64,
    opponent_hand: i64,
}

fn crop(weather: Weather, neighbours: Vec<&'static str>) -> Crop {
    Crop {
        kind: "wheat",
        height: 4,
        base: 3,
        ripe_at: 9,
        weather,
        neighbours,
    }
}

fn level(on: [bool; 3]) -> Level {
    let names = ["red", "green", "blue"];
    Level {
        switches: names
            .iter()
            .zip(on)
            .map(|(name, on)| Switch { name, on })
            .collect(),
    }
}

fn table(allies: usize, others: usize, hand: i64) -> Table {
    let mut mine: Vec<Card> = (0..allies).map(|_| Card { ally: true }).collect();
    mine.extend((0..others).map(|_| Card { ally: false }));
    Table {
        mine,
        hand,
        opponent_hand: 5,
    }
}

#[test]
fn golden_fixture_scripts() {
    let runtime = Runtime::default();
    let crops = compile(&runtime, "crop.rhai", include_str!("scripts/crop.rhai"));
    let door = compile(&runtime, "door.rhai", include_str!("scripts/door.rhai"));
    let card = compile(&runtime, "card.rhai", include_str!("scripts/card.rhai"));
    let mut golden = shuyi_assets::golden!("tests/golden/scripts.ron");

    let mut grows = |row: &str, view: Crop| {
        let out: Value = runtime
            .call(&crops, "grows", &view)
            .unwrap_or_else(|e| panic!("{e}"));
        golden.check(row, out);
    };
    grows("crop: dry, alone", crop(Weather::Sun, vec![]));
    grows("crop: rain, alone", crop(Weather::Rain, vec!["corn"]));
    grows(
        "crop: rain, crowded",
        crop(Weather::Rain, vec!["wheat", "wheat", "corn", "wheat"]),
    );

    for (row, on) in [
        ("door: none on", [false, false, false]),
        ("door: two on", [true, false, true]),
        ("door: all on", [true, true, true]),
    ] {
        let out: Value = runtime
            .run(&door, &level(on))
            .unwrap_or_else(|e| panic!("{e}"));
        golden.check(row, out);
    }

    for (row, view) in [
        ("card: no allies, more cards", table(0, 2, 6)),
        ("card: three allies, fewer cards", table(3, 1, 2)),
    ] {
        let out: Value = runtime
            .call(&card, "play", &view)
            .unwrap_or_else(|e| panic!("{e}"));
        golden.check(row, out);
    }
    golden.assert();
}
