---
id: "2735"
title: "Parked: Example and template: a small roguelike with generated floors, field of view, and runs from a seed"
type: feature
milestone: "Engine: roguelikes"
model: opus-5.5
effort: high
status: blocked
blocked_by: ["2118", "2119", "2120", "2114", "2116", "2003", "2511", "2501", "2004"]
nick_input: none
completed:
---

# 2735 — Example and template: a small roguelike

> **Parked** (Nick, 2026-10-20, choosing roguelikes as a kind of game the engine serves; Visions of Shuyi is "the first game I want to be launched with the engine"): like real-time strategy, visual novels and detective games, this waits until after Visions of Shuyi launches. Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

Found by the 2026-10-20 comparison with Bevy 0.20, Godot 4.7, Unity 6,
Unreal 5.8, GameMaker and Construct 3. Most of them teach a kind of
game with a small working one: Bevy's examples, Godot's demo projects,
and the starter templates of GameMaker and Construct 3.

What Shuyi has today:

- one example, `examples/hello-grid`: a turn game on a hand-made `.map`,
  with a Harness playthrough, bots, replay determinism and `frame-png`
  snapshots (`tests/it/`). Its `src/main.rs` seeds each launch from the
  platform (`ctx.game.seed = ctx.music_seed`);
- two templates, `templates/blank` and `templates/grid`;
- [ADR-0009](../../docs/adr/0009-modules-a-small-default-engine-and-opt-in-crates.md)
  §4: a module ships with a README in a fixed order, an example, a skill
  and settings as data, and its README claims a direction only when its
  example passes CI;
- 2301 and 2304 already name roguelikes as games outside tactics that
  the glyph console and big grid maps serve.

What it builds on (all in `blocked_by`):

- `shuyi-procgen` (2118, 2119): integer noise (`Noise`, `Noise::grid`),
  `Grid::map`, the generators `caves`, `rooms` and `walk`, `repair`,
  `WeightedTable`, and 2118's ADR on what a seed promises;
- `shuyi_core::sight` (2120): `field_of_view`, `sees`, `line`;
- 2114: named RNG streams (`SimRng::stream`), `mix_at` and `chance`, so
  a floor depends on the run's seed and its depth alone;
- 2116: golden tables of a record's hashes and CI's `determinism` job;
- 2003: the catalog, the `shuyi` crate and `cargo xtask check-modules`;
- 2511: the short path (`shuyi::prelude`, `shuyi_app::run_game`,
  `BasicTools`);
- 2501: templates as data made of parts; 2004: the guides' form and
  their check.

Why it matters:

- **The game first:** Visions of Shuyi needs none of this, which is why
  it waits for the launch.
- **Other 2D games:** Nick added roguelikes on 2026-10-20
  (`docs/decisions.md`). This ticket proves the direction and finishes
  the module.
- **The end goal** (2026-10-19): an agent asked "make me a roguelike
  with Shuyi Engine" starts from this template and this example. The
  agent eval (2524) grades against it (`docs/ROADMAP.md`).

The rule of 2026-10-16 and 2026-10-18 (`docs/decisions.md`), "inspired
by, nothing copied", holds: nothing comes from an existing roguelike (no
names, monsters, items or maps), and nothing from Visions of Shuyi
(`AGENTS.md` rule 4).

## Nick input

None.

## Scope

**In:**
- **`examples/roguelike`**, a turn game on the `shuyi` crate with the
  `procgen` feature, drawn on the glyph console. The root `Cargo.toml`'s
  `examples/*` makes it a workspace member.
  - **The short path:** built as 2511's minimal template builds a game
    (`shuyi::prelude::*`, `shuyi_app::run_game`, and `BasicTools` for
    its tool program), so a reader sees how little a game needs.
  - **The rules** (`TurnRules` and `Sides`, `src/rules.rs`): a hero and
    foes on a grid.
    - Commands: `Move(Dir)`, `Wait`, `Descend` (on the stairs) and
      `Drink` (a carried draught). Moving into a foe attacks it. The
      hero starts with one draught and picks one up by stepping on it.
    - The foes act inside `apply`, after each hero command, in the order
      they were placed. So the game has one side, the hero, and `turn`
      counts the hero's commands.
    - An attack hits on `chance(3, 4)` and takes the attacker's `attack`
      from the target's health.
    - Moves are four-way. If 2117 is done, they are eight-way with its
      corner rule (`Topology::Square8(Diagonals::KING)`): `Move` takes
      its `Dir8`, the keymap gains four diagonal actions, the foes chase
      by `flow_field_on`, and the stairs are placed by `flow_field_on`
      and checked by `reachable_on`, all with that topology. The tests
      and the bot use it too. The example's README says which.
  - **Three floors, each from the run's seed and its depth alone**
    (`src/floor.rs`, `Floor::new(run_seed, depth, &Tables)`):
    - a floor's generator, its foes and its things draw from the
      floor's own stream, `SimRng::stream(run_seed, "floor <depth>")`
      (2114), and its noise takes `mix_at(run_seed, depth)` as its seed.
      So nothing done on floor 1 changes floor 2;
    - combat and the foes' moves draw from
      `SimRng::stream(run_seed, "combat")`;
    - floor 1: `rooms`;
    - floor 2: `caves`, with shallow water: an open tile whose
      `Noise::grid` sample (gradient noise) is above a threshold (say
      `ONE / 4`) becomes water. Water costs the foes 2 to enter in their
      `flow_field`, and the hero 1;
    - floor 3: `walk`;
    - each floor is repaired with `repair(.., Repair::Connect)` and
      turned into terrain with `Grid::map`. The hero starts at the first
      room's centre (`Room::center`), or else at the first open tile,
      row-major;
    - the stairs go on the open tile farthest from the start by
      `flow_field` (ties: the first, row-major);
    - each floor places `4 + depth` foes and 2 things on open tiles at
      least 5 steps from the start by `flow_field`.
  - **Foes and things from weighted tables**, in `files/tables.ron`: the
    game's own data table, read with `shuyi_assets::ron::Ron<T>`. It
    holds the hero's numbers, two kinds of foe as data (a text key,
    health, attack, sight radius), a healing draught, and the
    `WeightedTable`s the floors draw from.
  - **Sight:**
    - the hero's `field_of_view` each turn, radius 8, and a remembered
      `TileSet` per floor merged with `TileSet::union_with`;
    - a foe within its sight radius (`dx² + dy² ≤ r² + r`, the rule
      `field_of_view` uses) that `sees` the hero chases it: it steps to
      the neighbour with the least `flow_field` distance to the hero,
      never onto another foe. A step onto the hero attacks it;
    - a foe that doesn't see the hero wanders: `roll_below(5)` picks one
      of `Dir::ALL` or waiting.
  - **The scene** (`MapScene`, `crates/ui/src/grid/scene.rs`), built with
    `MapScene::new` and `set_terrain`:
    - a tile never seen gets `None` as its terrain, so the glyph skin
      leaves it blank (`draw_tiles` in
      `crates/ui/src/grid/glyph/style.rs`);
    - a tile remembered but not seen now is tinted with
      `MapScene::tint(tiles, RangeName("remembered"))`. Its palette
      colour, `remembered` in `files/palette.ron`, darkens it;
    - foes and things are pushed as units only on tiles seen now;
    - if the tint can't make remembered tiles read as dimmed, write a
      ticket for a scene field (the `write-ticket` skill) instead of
      adding one here.
  - **A run:** the title screen starts a run from the platform's seed,
    as `examples/hello-grid/src/main.rs` does. A side panel shows the
    seed, the floor, health and the turn. A run ends when the hero dies
    or descends from floor 3, and the end screen shows the seed. Nothing
    is saved: an ended run is gone.
  - **A map larger than the view** (60 × 30): the grid kit's `Camera`
    (`crates/ui/src/grid/camera.rs`) follows the hero with
    `Camera::follow`.
  - **Its keymap**, `files/keymap.ron` in hello-grid's form: a keyboard
    layout, pad buttons and a `steam-deck` entry under `targets` (the
    `input-actions` skill; nothing names a key). Every screen declares
    its actions, as hello-grid's do.
  - **`shuyi.ron`** with the `desktop`, `web` and `steam-deck` targets
    and a `web` section. The font atlas is copied from
    `examples/hello-grid/files/font` (Terminus, with its license file),
    and `THIRD_PARTY_ASSETS.md` gains a row for the copy. The game draws
    only glyphs the atlas has (hello-grid's `required_glyphs`).
  - **A README** in hello-grid's form: how to run it, its layout, its
    targets, what each engine part does in it, four-way or eight-way,
    and the lines of its own code without tests and data (`cargo xtask
    template-lines` if 2512 is done; counted, and said how, if not). The
    engine's `README.md` names the example beside hello-grid.
- **Tests of the example** (`tests/it/`, as hello-grid's):
  - a Harness playthrough through actions only, from the title to
    floor 2. Each turn the test finds the path from the hero to the
    stairs with `reachable(..).path_to` and presses the action for its
    first step. It plays seed 1, or the first seed whose run reaches
    floor 2 this way (the test finds it, as hello-grid's playthrough
    finds a steady seed);
  - a bot that walks to the stairs by `flow_field` and drinks below
    half health plays 50 seeds through `bots::play` with a limit of
    2 000 turns, with no rules error, each ending in a death or an
    escape;
  - replay determinism, through 2116: a golden table
    `examples/roguelike/tests/golden/roguelike.ron` checked by
    `tests/golden.rs`, with a record `tests/records/seed-<n>.ron` made
    by `cargo shuyi bots`, in the form 2116 gives hello-grid's
    (`step NNN` rows and `outcome`). Add a step to the `determinism`
    job of `.github/workflows/ci.yml` that runs it on wasm32, as
    hello-grid's does;
  - floor 2 (terrain, stairs, foes and things) is the same after two
    different records on floor 1;
  - on every floor of 200 seeds, the stairs can be reached from the
    start (`reachable`);
  - on every turn of the playthrough, every foe in the scene stands on a
    tile in the hero's view;
  - `frame-png` pictures: floor 1 at the start, a remembered corridor,
    and floor 2's water;
  - `cargo shuyi check`, `cargo shuyi check --target all` and
    `cargo shuyi check-keys` report nothing;
  - if 2527 is done, `shuyi soak` over 20 seeds passes.
- **CI and Pages,** either order with the other examples (2736, 2737,
  2738, 2739 and 2740):
  - if the `wasm` job of `.github/workflows/ci.yml` and
    `.github/workflows/pages.yml` still name each example by hand
    (today `examples/hello-grid` alone), change both to a loop over
    every `examples/*/shuyi.ron` that has a `web` section:
    `cargo shuyi check --target all` and `cargo shuyi web --release` in
    each folder, an uploaded artifact per example, and in `pages.yml`
    each `dist/web` copied to a folder named after the example, with a
    line on the index page from its `web.title`. The `run-gates` skill's
    web-build step loops the same way;
  - if an earlier example made the loop, this one needs no workflow
    change;
  - if 2801 is done, the example gets its card in the website's Examples
    section. If 2019 is done, its headless browser job opens it too.
- **The template part** (2501): in `templates/shuyi-default/`, a part
  `kind-roguelike`, `when: Picked(kind, roguelike)`:
  - it turns on `procgen` and adds a stage with one generated floor, the
    hero's view, stairs, and a starter `tables.ron`;
  - a game made with it builds, and its test walks to the stairs. Add
    it to the `templates` job of `.github/workflows/ci.yml`, which makes
    a game and runs its gates;
  - if 2502 is done, `shuyi new demo --kind roguelike` makes it. If not,
    `shuyi new demo --set kind=roguelike` does, and 2502 adds the flag.
- **The direction guide** `docs/guides/roguelike.md`, in full and in
  2004's order, linked from `docs/guides/README.md`:
  - which template starts it (the part above);
  - which kits and modules it uses: the grid kit, `shuyi_core::sight`,
    `shuyi-procgen`;
  - what looks related but isn't for it: fog per team in `shuyi-rts`
    (2713), hand-made `.map` levels, real-time movement;
  - which example to read first.
- **The README** of `crates/procgen`, finished in ADR-0009 §4's order
  (2118 wrote the first two sections):
  - the cost: the WASM size it adds, measured on the example (the README
    says how), its dependencies and their licenses;
  - how to start, as the smallest working code (a doc test): a floor
    from a seed, mapped to terrain, with the hero's view;
  - how it sits beside the other `kind` answers (a tactics game with a
    generated map, say).
- **The skill** `shuyi-procgen`: if 1134 is done, finish it with a worked
  example that `shuyi add procgen` and a few lines make run; it passes
  `check-skills`. If not, add the worked example to 1134's list in this
  PR.
- **The catalog:** set `shuyi-procgen`'s row in `docs/modules.md` and the
  `roguelike` answer in `docs/modules.ron` to ready.
- **The eval:** if 2524 is done, add a roguelike task whose reference
  solution is this example, graded through the contract. If not, add the
  task to 2524's scope in this PR.

**Out (do not do):**
- saving a run halfway: 2768's run save (parked), removed when the run
  ends; daily seeds (they need the date from the platform) and a seed
  the player types: follow-ups if a game asks;
- a high-score table of runs (3054, parked);
- more floors, foes or things than listed, an inventory, classes, shops;
- a sprite skin: the glyph skin only;
- wave function collapse and prefabs (2119's Out);
- hex floors (2121, 2740);
- the gallery of small games (2736–2738);
- anything from an existing roguelike or from Visions of Shuyi;
- new engine features. A gap the example shows becomes a ticket
  (`write-ticket` skill).
- If the PR passes about 600 changed lines without fixtures and
  snapshots, move the template part and the eval task to one follow-up
  ticket, parked with this one and blocked by it (the `write-ticket`
  skill). Their acceptance line moves with them. Add the follow-up to
  the `blocked_by` of 2768, which grows the `kind-roguelike` part. Say
  so in the PR.

## Implementation steps

1. Read `examples/hello-grid` (its README, `src/rules.rs`,
   `src/screens.rs` and `tests/it/`), 2511's short path, 2118's ADR and
   README, and the APIs of 2119 (`carve`, `regions`, `WeightedTable`),
   2120 (`shuyi_core::sight`), 2114 (`SimRng::stream`, `mix_at`,
   `chance`) and 2116 (hello-grid's golden table). Check which of 2117,
   2502, 2512, 2524, 2527, 2801, 2019, 1134 and the other examples
   (2736–2739) are done.
2. Make `examples/roguelike`: its `Cargo.toml` on `shuyi` with
   `procgen`, `files/tables.ron`, and `src/floor.rs` with
   `Floor::new`, with unit tests.
3. Write `src/rules.rs` with its unit tests in `src/rules/tests.rs`,
   then the bot.
4. Write the screens (title, run, end), the scene and the side panel,
   the keymap, the palette, `shuyi.ron`, `src/main.rs` and the tool
   program. Copy the font and add its `THIRD_PARTY_ASSETS.md` row.
5. Write `tests/it/`, the golden table and its record, and the
   pictures. Run `cargo shuyi check --target all` and
   `cargo shuyi check-keys` in the example's folder.
6. Make CI and Pages build every example, if no earlier example did,
   and add the golden test to the `determinism` job.
7. Count the diff (`git diff --stat main`, fixtures and snapshots
   aside). Under about 600 lines, write the template part and its
   `templates` job entry, and run the made game's own gates
   (`cargo fmt --all` from the root doesn't reach `templates/`). Over
   it, write the follow-up ticket instead (**Out**).
8. Measure the WASM size. Write the example's README, the procgen
   README, the guide, the skill and the eval task. Set the catalog. Run
   the gates.

## Acceptance criteria

- [ ] The Harness playthrough goes from the title to floor 2 through
      actions only.
- [ ] The bot plays 50 seeds with no rules error, each ending in a death
      or an escape.
- [ ] The golden table of a run's record matches in CI's `test` job and
      on wasm32 in its `determinism` job.
- [ ] Floor 2 is the same after two different floor 1 records, and on
      every floor of 200 seeds the stairs can be reached from the start.
- [ ] On every turn of the playthrough, every foe in the scene stands on
      a tile in the hero's view.
- [ ] The `frame-png` pictures are checked in.
- [ ] `cargo shuyi check`, `cargo shuyi check --target all` and
      `cargo shuyi check-keys` pass in `examples/roguelike`.
- [ ] CI builds the example for the web, and Pages publishes it.
- [ ] A game made from the `kind-roguelike` part builds and passes its
      gates in CI's `templates` job, unless the part moved to a
      follow-up as **Out** says.
- [ ] `docs/guides/roguelike.md` is written in full, and
      `cargo xtask check-modules` passes, 2004's guide check included.
- [ ] `crates/procgen/README.md` has all five sections of ADR-0009 §4
      with the measured WASM size, and the catalog row and the
      `roguelike` answer are ready.
- [ ] `THIRD_PARTY_ASSETS.md` lists the font's copy in
      `examples/roguelike`.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: a floor built for a seed and a depth; where the start and the
  stairs go; an attack that hits and one that misses, with a
  `ScriptedRng`; a foe's choice (chase when it sees the hero, wander
  when not) with a `ScriptedRng`; a draught picked from a table by a
  scripted roll.
- Integration: the Harness playthrough; the bot over 50 seeds; floors
  from the seed and depth alone; stairs reachable on 200 seeds; foes
  only in view; `frame-png`; `cargo shuyi check` and `check-keys`; the
  template part's game and its test.
- Golden: the run's record, natively and on wasm32 (2116).
- Doc test: the procgen README's smallest working code.

## Completion notes

*(Filled in by the session that completes the ticket.)*
