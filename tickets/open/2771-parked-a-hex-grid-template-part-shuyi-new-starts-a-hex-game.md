---
id: "2771"
title: "Parked: A hex-grid template part: shuyi new starts a hex game from Hello, Hex's stage"
type: feature
milestone: "Engine: hex grids"
model: opus-5.5
effort: medium
status: blocked
blocked_by: ["2740", "2501"]
nick_input: none
completed:
---

# 2771 — A hex-grid template part

> **Parked** (Nick, 2026-10-20, choosing hex grids as a kind of game the engine serves; Visions of Shuyi is "the first game I want to be launched with the engine"): a follow-up of parked work is parked. This is 2740's template part, so like 2740 it waits until after Visions of Shuyi launches. Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

[ADR-0013](../../docs/adr/0013-starting-a-game-a-wizard-over-the-catalog-and-templates-made-of-parts.md)
§3 makes the template `shuyi new` copies a folder of parts, each
applied when an answer is picked. 2501 builds it:
`templates/shuyi-default/`, with the part `base` (always) and the part
`stage-top-down-grid` (when the perspective is `top-down-grid`).

2740 (parked, before this) draws hex maps: `Projection::Hex`,
`GlyphSkin::with_hex_rows`, the example `examples/hello-hex`, the
guide `docs/guides/hex-grid.md`, and the catalog's `hex-grid` answer
set to ready. It leaves the template part to this ticket (its *Out*),
and its guide says Hello, Hex starts a hex game until a part exists.
So an agent asked for "a hex strategy game" copies an example instead
of running `shuyi new`.

This ticket adds the part `stage-hex-grid`. Its stage is Hello, Hex's
map, piece and foes, written on the short path, so `shuyi new demo
--template hex` makes a hex game that passes its gates in CI. An agent
reads the part beside the grid part to see what changes on a hex map.

What it builds on:

- 2501 (before this): `template.ron`, `Part`, `When`, `compose`, the
  merged files (`Cargo.toml`, `shuyi.ron`, `src/stages.rs`), the short
  names (`grid`, `blank`), `docs/templates.md` and CI's `templates`
  job over the short names;
- 2740 (before this): Hello, Hex's rules, map and screen. Through it,
  2121's `shuyi_core::hex` and `Topology::Hex`, and 2511's short path
  (`BasicGame`, `BasicData`, `run_game`, `BasicTools`);
- the grid part's rules crate and its fence (2015,
  [ADR-0032](../../docs/adr/0032-the-layers-as-data-checked-over-cargo-metadata-a-float-ban-in-the-simulations-and-a-new-games-rules-crate-kept-pure.md)
  §4), which 2501 moved into `stage-top-down-grid`.

Why it matters:

- **The game first:** Visions of Shuyi needs none of this, so it is
  parked with 2740.
- **Other 2D games:** a wargame, a hex strategy game or a hex board
  game starts from a part that builds and passes its gates, as a grid
  game does.
- **The end goal** (2026-10-19): an agent asked for a hex game runs
  `shuyi new`, and gets the hex stage, its fenced rules and the guide.

## Nick input

None.

## Scope

**In:**
- **The part `stage-hex-grid`** in `templates/shuyi-default/`, written
  in `template.ron` beside `stage-top-down-grid` and in its form:
  - `when: Picked("perspective", "hex-grid")`, no `requires` and no
    `conflicts`;
  - `stage: "hex"`, its screen being `HexScreen` (below);
  - no modules (hex maps are in the default kits, 2121's ADR) and no
    targets of its own (base's).
- **How it adds to base's files.** 2501 (done) decided how a stage part
  adds to files the `base` part writes (ADR-0042): the game crate's
  `Cargo.toml`, its `lib.rs` through the generated `src/stages.rs`, the
  language file and `shuyi.ron`, each a piece the composer merges.
  Until 2503, the composer refuses two stages in one game, so the hex
  part composes beside `stage-top-down-grid` only once 2503 is done. The hex part does each the way the grid part does,
  under names of its own, and writes only paths no other part writes.
  So it composes alone, and beside `stage-top-down-grid` with no
  `ComposeError::SameFile` (2501's `--set` takes several answers, and
  2502's wizard lets several perspectives be picked):
  - its map is read from the bundle (`BasicData::bundle`) when the
    stage is made, not as the game's own part
    (`BasicGame::with_own`): the grid part holds that, and a game has
    only one;
  - its text is under `hex.` keys in the game's language file;
  - it reads the same six moves as the grid part's stage (the four
    directions, Confirm and Cancel) from the game's action set, as
    the grid part's stage reads them. Nothing names a key (the
    `input-actions` skill).
- **The rules crate**, `crates/hex-rules/`: package
  `{{crate_name}}-hex-rules`, lib name `hex_rules`, depending on
  `shuyi-core`, `shuyi-assets` and `serde` alone, as the grid part's
  `crates/rules` does. The game crate depends on it, and the root's
  `[workspace.dependencies]` gain what it needs, the way the grid part
  adds `rules`. Its `src/lib.rs` is Hello, Hex's rules, copied, with
  the names the grid part's rules use where they mean the same
  (`Stage`, `Command`, `Outcome`, `parse_map`), so the two read side by
  side, and nothing named after the example:
  - the map parsed from text with `shuyi_assets::grid_map` (no I/O):
    open ground, water that costs 2 to enter, walls and a goal tile,
    with the piece's start and the two foes;
  - the piece has 3 move points a turn. Its range is `reachable_on`
    with `Topology::Hex(HexLayout { orientation: Orientation::Pointy,
    offset: Offset::Odd })`. A `Command` moves it along a path inside
    that range. Then each foe steps one tile along `find_path` toward
    it;
  - reaching the goal tile wins; a foe stepping onto the piece's tile
    loses;
  - `TurnRules` and `Sides`, as Hello, Hex has them, so
    `shuyi_core::bots::play` runs it.
- **The fence**, as 2501 moved it into `stage-top-down-grid`:
  - `crates/hex-rules/clippy.toml` and
    `crates/hex-rules/tests/layers.rs` are copies of the grid part's
    `crates/rules/clippy.toml` and `crates/rules/tests/layers.rs`, and
    `src/lib.rs` starts with the float ban as the grid part's does;
  - a test in `crates/xtask/src/layers.rs`, beside
    `a_new_games_rules_test_keeps_a_copy_of_the_devices`, keeps both
    copies equal to the grid part's. So one proof in CI covers both:
    the steps *The rules crate's fence holds* and *shuyi lint finds
    what breaks the game's code* stay on `grid`;
  - base's `AGENTS.md`, *The layers*: where 2501 wrote that the grid
    stage's rules crate comes with its fence, say it of each stage's
    rules crate (`crates/rules`, `crates/hex-rules`), in words true
    whichever stage parts a game has;
  - the merged `shuyi.ron` names the hex rules crate in `rules` (for
    `lint`), and `crates/hex-rules/src` in `check_keys`'s `rust` list,
    through the same pieces the grid part adds. `Project::rules`
    (`crates/cli/src/project.rs`) names one package today. If it still
    does, and both stage parts are picked, make it take one name or a
    list (a lone string still reads, so no game's `shuyi.ron` breaks),
    and fence each in `lint`. The field's type changes, so write the
    `Breaking:` line CI's `api` job asks for (ADR-0038 §6).
- **The map**, `crates/game/files/hex/stage.map`: Hello, Hex's 9 × 7
  map, the rows of a pointy-top, odd-r layout. It leaves a winning
  line. It joins the `credits` part's `own` list through the merged
  `shuyi.ron`, as the grid part's `stage.map` does.
- **The screen**, `crates/game/src/hex.rs`: `HexScreen`, Hello, Hex's
  stage screen on the short path:
  - it shows the piece's range; the player steers a path with the
    cursor (`steer`, the cheapest path from the range's
    `Reach::path_to`) and moves along it with Confirm; then the foes
    step and the turn count goes up;
  - at the end, a line says whether the piece won or lost, and Confirm
    starts again. Cancel leaves the game, as on the grid part's stage;
  - it builds a `MapScene` and paints it with the glyph skin
    `with_hex_rows(Some(Offset::Odd))`, on base's console. Tests of
    what happened read the state or the scene (`AGENTS.md` rule 9).
- **The short name `hex`**: `shuyi new demo --template hex` and
  `shuyi new hex <dir>` pick `perspective=hex-grid`, as `grid` and
  `blank` name their answers. Every list of the built-in short names
  gains it: `docs/templates.md`, `docs/contract.md` (*The templates*),
  `new`'s help and schema, and the MCP `new` tool's description
  (search `crates/cli` for `"blank"`).
- **CI and the gates:** the `templates` job's matrix in
  `.github/workflows/ci.yml` gains `hex`, so a hex game is made and
  runs its gates (fmt, clippy, tests, `check`, `check --target all`,
  `check-keys` and `lint`) and the long-name fmt step. The `run-gates`
  skill's *A new game from each template* names `hex` beside `grid`
  and `blank`. `cargo xtask check-pins` (2011) checks the part's
  `[workspace.dependencies]` requirements against the root's, as it
  does the grid part's.
- **The docs:**
  - `docs/templates.md`: the part in its list of parts, with its
    `when`, and the short name;
  - `docs/guides/hex-grid.md` (2740): its *what starts it* names
    `shuyi new demo --template hex` (the part `stage-hex-grid`, also
    picked by `--set perspective=hex-grid`), and Hello, Hex stays the
    example to read first.
- **Either order with 2502** (the wizard): if 2502 is done, its
  prompts offer the `hex-grid` answer from the catalog with no change,
  and `shuyi new demo --perspective hex-grid --no-input` picks this
  part. The guide names that command beside `--template hex`, and a
  test checks that the two make the same plan. If not, the guide names
  `--template hex` and `--set perspective=hex-grid`, and 2502 adds the
  `--perspective` command and its test (its clause with 2771 says so).
- **Either order with 2503** (several perspectives in one game): if
  2503 is done, its CI job makes and builds a game for every pair of
  ready perspectives, so the pairs with `hex-grid` must build and pass
  their generated `tests/moves.rs` through the base part's door: check
  that they do. If not, nothing more to do: the compose test below is
  the check here, and 2503's job builds the hex pairs when it lands
  (its clause with 2771 says so).
- **What changes for the game:** nothing, unless `Project::rules`
  changes type (above). The PR says which.

**Out (do not do):**
- the projection, the skin, the example, the guide's first version and
  the catalog's answer (2740), and the hex maths and searches (2121);
- a `GameTools` of the part's own. Base's `BasicTools` serves the hex
  stage, so the contract's `bots` and `replay` stay unsupported on a
  new hex game; the rules' own tests play the bots;
- `frame-png` pictures of the part's game: Hello, Hex's pictures cover
  the skin;
- a hex sprite skin, flat-top maps and hex tile layers (2740's *Out*);
- a skill for hex stages: the part and the guide are what an agent
  reads;
- new engine features. A gap the part shows becomes a ticket (the
  `write-ticket` skill).

## Implementation steps

1. Read 2501's `templates/shuyi-default/` (`template.ron`, `base`,
   `stage-top-down-grid`), `docs/templates.md` and the `templates`
   job; how the grid part adds to base's files and carries its fence;
   `examples/hello-hex` and `docs/guides/hex-grid.md` (2740); and
   `Project::rules` and `lint` in `crates/cli`. Check whether 2502 and
   2503 are done.
2. Write `crates/hex-rules` from Hello, Hex's rules, with its fence
   copies and its tests.
3. Write the map and `HexScreen`, and the part's pieces of base's
   files, the way the grid part adds its own.
4. Add the part to `template.ron`, the short name `hex`, the compose
   tests in `crates/cli` and the copy test in `crates/xtask`.
5. Add `hex` to the `templates` job and the `run-gates` skill, make a
   game with it, and run its gates.
6. Compose both perspectives and build that game once. Make
   `Project::rules` take a list if it has to. Do the 2502 and 2503
   items and the docs, and run the gates.

## Acceptance criteria

- [ ] `shuyi new demo --template hex` and `shuyi new hex <dir>` make a
      game that passes its gates in CI's `templates` job (fmt, clippy,
      tests, `check`, `check --target all`, `check-keys` and `lint`).
- [ ] In that game, a Harness playthrough reaches the goal through
      actions only, and a line of moves that lets a foe reach the piece
      ends the stage as a loss.
- [ ] The rules' bot plays 20 seeds with no rules error.
- [ ] `--set perspective=hex-grid` makes the same plan as `--template
      hex`, and its answer has no `limits` entry saying that `hex-grid`
      has no part.
- [ ] `--set perspective=top-down-grid,hex-grid` composes with no
      `ComposeError`, its plan has both stages, and the game it makes
      builds and passes `cargo test` (said in the PR; 2503's job runs
      it if 2503 is done).
- [ ] The hex rules crate's `clippy.toml` and `tests/layers.rs` equal
      the grid part's (a test), and `cargo shuyi lint` on a hex game
      fences the hex rules crate (no limit saying it names no rules
      package).
- [ ] `docs/templates.md` lists the part and the short name, and
      `docs/guides/hex-grid.md`'s *what starts it* names them.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit (`crates/hex-rules`, in the made game): the map parses, with the
  piece, two foes and the goal; the range from the start counts water
  as 2; a path out of range is not among the legal commands; each foe
  steps one tile along `find_path`; the goal wins and a foe on the
  piece loses; `bots::play` over 20 seeds.
- Integration (`crates/game/tests/hex.rs`, in the made game): the
  playthrough to the goal through `Harness::act`; the loss; Confirm
  starts again after the end and Cancel leaves.
- Engine (`crates/cli`): `compose` with `hex` has the part's files,
  `src/stages.rs` names the hex stage and no placeholder is left;
  2501's golden test of the plans gains `hex`; the two perspectives
  compose together. `crates/xtask`: the fence copies are equal.
- CI: the `templates` job's `hex` entry.

## Completion notes

*(Filled in by the session that completes the ticket.)*
