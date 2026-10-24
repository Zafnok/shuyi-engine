# An autobalance plan for a grid game

`cargo shuyi autobalance` needs the stage's tools to give an
`AutobalancePlan`. This one goes on top of the playtest of the worked
example (`example/bots.rs`: the entrants `HEROES`, `checks()` reading
`playtest.ron`, the rules' `Determinize`). Its stage type `S` is the
map, `GridMap<TerrainId, Feature>`, and its one knob moves the
wanderer's start along its row.

Add to the imports of `bots.rs`:

```rust
use std::ops::RangeInclusive;
use std::path::PathBuf;

use shuyi::assets::grid_map::{GridMap, print};
use shuyi::core::Knob;
use shuyi_cli::{AutobalancePlan, Tunable};
```

The knob: 0 is the map as authored, and a step that lands off a free
floor tile can't apply.

```rust
/// A knob of the wanderer's start: it starts `step` tiles right of where
/// the map puts it (left, below 0), on a floor tile no one else holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WandererStart;

impl Knob<GridMap<TerrainId, Feature>> for WandererStart {
    fn name(&self) -> &'static str {
        "wanderer start"
    }

    fn range(&self) -> RangeInclusive<i32> {
        -4..=4
    }

    fn apply(
        &self,
        map: &GridMap<TerrainId, Feature>,
        step: i32,
    ) -> Option<GridMap<TerrainId, Feature>> {
        let (&from, _) = map
            .features
            .iter()
            .find(|(_, f)| **f == Feature::Wanderer)?;
        let to = Pos::new(from.x + step, from.y);
        let free = map.tiles.get(to) == Some(&rules::FLOOR) && !map.features.contains_key(&to);
        if !free {
            return None;
        }
        let mut changed = map.clone();
        changed.features.remove(&from);
        changed.features.insert(to, Feature::Wanderer);
        Some(changed)
    }

    fn describe(&self, step: i32) -> String {
        match step {
            0 => "the wanderer starts where the map puts it".to_owned(),
            n if n > 0 => format!("the wanderer starts {n} tile(s) right"),
            n => format!("the wanderer starts {} tile(s) left", -n),
        }
    }
}
```

The plan: the playtest's entrants and checks, and a `Tunable` of the
map with its knobs, a try on a changed map, and the writer `--apply`
uses (`grid_map::print` writes a `.map` file back; it doesn't keep the
file's comments).

```rust
/// The autobalance of the stage of `map`, written to `file` (`--apply`):
/// [`HEROES`] judged by [`checks`], the stage changed by
/// [`WandererStart`], each try played by `play_tries` from its seed alone.
pub fn tuning(map: GridMap<TerrainId, Feature>, file: PathBuf) -> Result<AutobalancePlan, String> {
    let checks = checks()?;
    Ok(AutobalancePlan::new(
        "the stage (files/stage.map)",
        HEROES.iter().map(|h| (h.name.to_owned(), None)).collect(),
        Box::new(move |names| {
            checks
                .iter()
                .filter(|c| c.entrants().iter().all(|e| names.contains(e)))
                .cloned()
                .collect()
        }),
        Box::new(Tunable::new(
            map,
            vec![Box::new(WandererStart)],
            Box::new(|map, entrant, seed, turns| {
                let stage = Stage::new(map, 0)?;
                let mut tries = play_tries(&stage, &HEROES[entrant], seed..seed + 1, turns)
                    .map_err(|e| e.to_string())?;
                tries.pop().ok_or_else(|| "no try".to_owned())
            }),
            Box::new(move |map| {
                let text = print(map).ok_or("the map has a tile its legend can't write")?;
                std::fs::write(&file, text).map_err(|e| format!("{}: {e}", file.display()))?;
                Ok(vec!["crates/game/files/stage.map".to_owned()])
            }),
        )),
    ))
}
```

And the method of the stage's `GameTools`, in `tools.rs`:

```rust
    /// `autobalance`: the wanderer's start moved until the bots land in
    /// `playtest.ron`'s bands.
    fn autobalance_plan(
        &self,
        _: Option<&str>,
        _: &shuyi_cli::contract::autobalance::AutobalanceInput,
    ) -> Result<shuyi_cli::AutobalancePlan, String> {
        let file = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("files/stage.map");
        super::bots::tuning(self.map()?, file)
    }
```

Then:

```bash
cargo shuyi autobalance --budget 30
cargo shuyi autobalance --entrant rollout --apply
```

On the template's map both bots are already in their bands ("already in
every band: nothing to change"). Narrow a band the knob can move, and
the answer lists each change in the knob's words, the reports before
and after, and why it stopped. A band no knob can move says it is stuck.
Knobs that matter change what the bots meet: one more wanderer, a wall
moved, a slower or a luckier foe.
