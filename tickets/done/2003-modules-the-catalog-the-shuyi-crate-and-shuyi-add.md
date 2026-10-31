---
id: "2003"
title: "Modules: the catalog by concern, module metadata, the shuyi crate with a feature per module, and shuyi modules and shuyi add"
type: infra
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: done
blocked_by: ["game#1131", "1129", "2015", "2358", "2511"]
nick_input: none
completed: 2026-10-23
---

# 2003 — Modules: the catalog, the `shuyi` crate, `shuyi add`

## Context

Nick, 2026-10-16 (`docs/decisions.md`): "we should support plenty of
modules for our engine, so that we don't overload devs", and "a dev will
set a direction and import non-conflicting modules to help them out".
[ADR-0009](../../docs/adr/0009-modules-a-small-default-engine-and-opt-in-crates.md)
says what a direction is (§2), what a module ships with (§4), and how a
game turns one on (§6). This ticket builds §4's metadata and §6, so that
the first module (physics, 1128, or any other) lands in a place that
already works.

The perspective markers (§3) come with the tickets that add
perspectives: 1120, 1122, 1126 and 1127. The guides (§5) are 2004.

It needs the engine's code here (game#1131), the contract with
`shuyi new` (1129), 2015's `cargo metadata` reader and
`tools/layers.ron`, 2358's `Module` trait, through which a module's
code joins a running `Game`, and 2511's short path (its prelude and
`BasicGame`).

## Nick input

None.

## Scope

**In:**
- **The `shuyi` crate** (`crates/shuyi`):
  - it re-exports `shuyi-core`, `shuyi-assets`, `shuyi-ui`,
    `shuyi-platform` and `shuyi-app` under their short names
    (`shuyi::core`, …, `shuyi::platform`), and has `shuyi::prelude`:
    `pub use shuyi_platform::prelude::*;` and
    `pub use shuyi_app::run_game;` (2511's short path);
  - it has one cargo feature per module, all off by default. Until a
    module crate exists, it has no feature; add none in advance;
  - the example `hello-grid` (1118) depends on `shuyi` only;
  - the name is already held on crates.io by an empty 0.0.0
    placeholder (`docs/decisions.md`, 2026-10-22). The crate's first
    real version goes through trusted publishing like the others (2005),
    never a publish by hand.
- **Modules that are turned on join the game** (2358's ADR):
  `shuyi::enabled_modules<G: GameData>() -> Vec<Box<dyn
  shuyi_ui::module::Module<G>>>` gives the `Module` of each module
  whose feature is on, cfg-gated, as Bevy's `DefaultPlugins` does. It is
  empty until a module exists. A module listed there is made with its
  defaults. One that needs arguments is installed by the game's own
  code, and its README says how.
  - The templates install them once, so `shuyi add` needs no edit to a
    new game's code. The grid template, on the long form until 2501
    moves it, calls `Game::with_modules(shuyi::enabled_modules())`. The
    blank template, on 2511's `BasicGame`, passes
    `shuyi::enabled_modules` to a new `BasicGame` builder,
    `modules(fn() -> Vec<Box<dyn Module<BasicData<A, T>>>>)` (default:
    none), whose `game(ctx)` installs them with `Game::with_modules`.
    The engine's own modules have distinct names and states, so with
    every feature on they install with no `ModuleError` (a test).
  - `Module`, `Hook` and `Install` join `shuyi_platform::prelude`
    (2358 leaves both of these to this ticket).
  - For a game whose `src/` never calls `enabled_modules`, `shuyi add`
    prints the one line to add. For a module that adds formats, it also
    prints the line for the game's `formats()` (2358's ADR, point 5).
- **Module metadata** (ADR-0009 §4): `[package.metadata.shuyi]` with
  `concern`, `answer`, `serves`, `alternatives` and `works_with` (the
  parked 3017 later adds `needs`, ADR-0012 §1). One module may answer
  several concerns, so `concern` and `answer` come as a list of pairs
  (the parked `shuyi-3d` answers three perspectives and *World
  drawing*). They are read
  into a `ModuleInfo` type in `shuyi-cli`. The concerns and their
  answers are the table of ADR-0009 §2, with the answers ADR-0011 §2
  adds (first person, third person, top-down 3D; the *World drawing*
  concern) and the *Kind of game* concern ADR-0015 §1 adds (`kind`:
  `tactics` ready once the carve is done; `rts`, `visual-novel`,
  `mystery`, `roguelike`, `dungeon-crawler`, `party-rpg` and
  `action-rpg` parked), kept as data in
  `docs/modules.ron`, which the catalog and the check read. The
  Perspective concern also lists `hex grid`, parked (2121, 2740),
  answered by the default grid kit rather than a module (2121's ADR adds
  it to ADR-0009 §2's answers), and `first-person grid`, parked,
  answered by `shuyi-crawl` (ADR-0033 §1). A concern can be marked
  `alternatives: false` when its answers can share a stage; `kind` is
  the first. Every module crate also has its entry in
  `tools/layers.ron` (2015): the engine crates it may use, its outside
  crates, `devices` and `floats`. `cargo xtask check-layers` fails
  without it.
- **The catalog**, `docs/modules.md`, grouped by concern. It has a row
  per module, format plugin and platform backend: the name, the
  directions it serves, its status (planned, parked or ready) and its
  tickets. Start it with:
  - `shuyi-physics` (1128, parked);
  - `shuyi-net` (1140–1150, parked);
  - `shuyi-procanim` (2701–2704, parked);
  - `shuyi-3d`, `shuyi-app-3d` and `shuyi-diorama` (3101–3109, parked);
  - `shuyi-rts` (2705–2720, 2733–2734), `shuyi-vn` (2721–2726) and
    `shuyi-mystery` (2727–2732), parked until after the game launches;
  - `shuyi-crawl` (2741–2744, 2753–2755), `shuyi-rpg` (2745–2752,
    2758, 2759) and `shuyi-action` (2756, 2757, 2760–2762, 2764),
    answering `kind: dungeon-crawler`, `party-rpg` and `action-rpg`,
    parked until after the game launches (ADR-0033, ADR-0034);
  - `shuyi-procgen` (2118, 2119, 2735), answering `kind: roguelike`,
    parked until after the game launches; its field of view is in
    `shuyi-core` (2120, parked with it);
  - the format plugins of 2202–2204, and the `sfx` feature of
    `shuyi-assets` (2225: `.sfx.ron` sound effects, planned or ready as
    it stands);
  - the backends `shuyi-app` and `shuyi-playdate` (3010), and
    `shuyi-gba` (3021) and `shuyi-nds` (3024), parked.
- **A check**, `cargo xtask check-modules`, run in CI and listed in the
  `run-gates` skill. Build it on 2015's `crates/xtask/src/layers.rs`
  (its `Graph`, read from `cargo metadata`), not on a second
  `cargo metadata` reader. It fails when:
  - a default crate (`shuyi-core`, `shuyi-assets`, `shuyi-ui`,
    `shuyi-platform`, `shuyi-app`, `shuyi-cli`) depends on a module;
  - a crate under `crates/` outside the default six, the `shuyi` crate
    (the front door, not a module) and `xtask` (this repository's
    tooling, never published) has no README, no metadata, or no catalog
    row;
  - a module's `concern` or `answer` isn't in `docs/modules.ron`;
  - two modules answer the same concern without naming each other in
    `alternatives`, unless the concern is marked `alternatives: false`;
  - a feature of `shuyi` names no catalog row.
- **Two contract operations** (ADR-0005), as library functions with JSON
  output like the rest of 1129:
  - `shuyi modules`: the catalog by concern, plus which modules this
    game has on;
  - `shuyi add <module>`:
    - turns on the feature of `shuyi` in the game's `Cargo.toml`;
    - copies the module's skill into the game's skills folder, if the
      skill pack (1134) is there;
    - prints the first paragraph of the module's README;
    - the skill copy and the README's first paragraph are functions
      that take a package folder, so 2526 can call them for a crate
      outside `crates/`;
    - if the game already has an alternative on, says so and links the
      README section on using both;
    - running it twice changes nothing the second time, and an unknown
      name lists the known ones.
- **`shuyi new`** templates depend on `shuyi` and turn on only the
  modules their direction needs (today: none). The blank template
  (2511) imports `shuyi::prelude::*` instead of
  `shuyi_platform::prelude::*`, and its `main.rs` calls the prelude's
  `run_game`.
- **CI** builds the default engine, each module alone, and every module
  together.

**Out (do not do):**
- writing any module (each has its own ticket);
- the perspective markers (1120, 1122, 1126, 1127);
- the guides (2004);
- `shuyi remove` (add it when someone asks);
- the wizard and its catalog fields (`pick`, `question`: 2502). Write `shuyi add`'s `Cargo.toml` edit
  as a library function, because 2501's composer calls it too
  (ADR-0013 §3);
- the `Module` trait and how a module's code joins a running `Game`
  (2358);
- outside crates in `shuyi add` and `shuyi modules` (crates.io at a
  version, git at a revision, a folder): 2526, after this ticket;
- moving anything out of the default crates (ADR-0009 §1).

## Implementation steps

1. Add `crates/shuyi` with its prelude and `enabled_modules`, and move
   `hello-grid` onto it. Add the `BasicGame` builder and the prelude's
   module names, and move the templates onto `shuyi`. `cargo fmt --all`
   doesn't reach `templates/`: run `rustfmt --edition 2024` on their
   `.rs` files.
2. Write `docs/modules.ron`, `ModuleInfo`, `docs/modules.md` and
   `cargo xtask check-modules` (reuse 2015's `layers.rs` graph), with a
   fixture workspace for each failure.
3. Add `modules` and `add` to the contract, with integration tests on a
   game made by `shuyi new` in a temporary folder. Use two fixture
   modules that are alternatives to test the warning.
4. Add the CI jobs. Add `check-modules` to the `run-gates` skill.
5. Link `docs/modules.md` from the README.

## Acceptance criteria

- [x] `hello-grid` builds and passes its tests depending on `shuyi`
      alone.
- [x] A game from each template installs `shuyi::enabled_modules` once,
      and passes CI's templates job.
- [x] `cargo xtask check-modules` passes on the repository, and fails
      on each fixture with a message naming the crate and the rule.
- [x] `shuyi add` on a fresh game edits `Cargo.toml` once, and a second
      run reports that nothing changed.
- [x] `shuyi add` of a module whose alternative is on prints the
      warning and the link.
- [x] `shuyi modules --json` matches its schema (1129's schema test).
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the check, on fixture workspaces; `ModuleInfo` parsing.
- Integration: `shuyi modules` and `shuyi add` on a game from
  `shuyi new`.

## Completion notes

Done as scoped, with [ADR-0041](../../docs/adr/0041-the-module-catalog-concerns-as-data-metadata-answers-as-pairs-and-modules-and-add-reading-the-engine-the-game-builds-with.md)
for the choices the scope left open.

- **`crates/shuyi`**: the default engine's crates as `shuyi::core` to
  `shuyi::app`, `shuyi::prelude` (`shuyi-platform`'s prelude and
  `run_game`), and `enabled_modules`, empty, with no feature. A test
  installs every module on the toy game with no `ModuleError`; CI's
  `modules` job runs it with every feature. It has its `tools/layers.ron`
  entry, and `check-layers` now counts `shuyi` as an engine crate. It
  joins the API gate's crates (`API_CRATES`).
- **`BasicGame::modules`**, and `Module`, `Hook` and `Install` in
  `shuyi_platform::prelude`. `BasicGame::game` returns
  `Result<Game, ModuleError>`, and `start` makes the error the load's.
- **Templates** depend on `shuyi` (`{{engine.shuyi}}`, a new
  `ENGINE_CRATES` entry) and install `shuyi::enabled_modules` once:
  `grid` in `Game::with_modules` (its `game()` returns a `Result`),
  `blank` in `BasicGame::modules`. `blank` imports `shuyi::prelude::*`
  and runs the prelude's `run_game`. `grid`'s rules crate keeps
  `shuyi-core` and `shuyi-assets` alone, and `shuyi` joins the crates it
  may never reach (`lint::NEVER` and the template's `layers.rs`).
- **`hello-grid`** depends on `shuyi` alone for the engine. Deviation:
  it keeps `shuyi-cli`, optional, for its `tools` feature (the tools
  aren't a crate `shuyi` re-exports, and a `tools` feature would be a
  feature with no catalog row), and `macroquad`, whose
  `#[macroquad::main]` must name its crate.
- **The catalog**: `docs/modules.ron` (six concerns, every answer with
  `by`, `status` and `tickets`), `docs/modules.md` (a section per
  concern, the format plugins and the backends, all rows of the scope),
  read by `shuyi_cli::catalog` (`Concerns`, `ModuleInfo`, `Catalog`,
  the `Cargo.toml` edit, the README paragraph, the skill copy).
  Metadata is `answers = [{ concern, answer }]`, `serves`, and optional
  `alternatives`, `works_with` and `formats` (the function a game's
  `formats()` calls, for 2358's point 5).
- **`cargo xtask check-modules`** on `layers.rs`'s graph (which gains
  members' features and `metadata.shuyi`), with a fixture workspace per
  failure, run through `cargo metadata`. Beyond the scope's rules it
  fails on a module whose `serves` is empty (ADR-0009 §7) and on an
  unknown metadata key. CI's `modules` job runs it and builds `shuyi`
  as the default engine, with each feature alone, and with all;
  `changes.yml` counts the two catalog files as code.
- **`modules` and `add`** in the contract (version 16), on the CLI and
  as MCP tools taking `project`, with the problem codes `SH0003`
  (`unknown-module`) and `SH1004` (`engine-dependency`). They read the
  engine the game builds with (a path, or where `cargo metadata`
  resolved `shuyi`), not a copy in the binary. A module's skill is its
  package's `skill/` or the skill pack's `skills/<name>` beside it,
  copied to `.claude/skills/` when the game keeps that folder. The
  README's section on using both is `## Beside its alternatives`.
- Settled the clauses that waited on 2003 in 2105, 2118, 2121, 2217,
  2225, 2601, 2707, 2721, 2727, 2736, 2737, 2738, 2740, 2745 and 3101.
  2601's `http` feature will need a row named `shuyi-http`, or a change
  to the check's rule 5; its ticket says so.
- Docs: `docs/contract.md` (the two rows, the templates), `README.md`
  (the `shuyi` row, the catalog), the docs map (the catalog and its
  pair), the templates' `AGENTS.md`, the `run-gates` skill.
- Not checked through the MCP session tools: nothing a player sees
  changed.

Breaking: shuyi-platform: `BasicGame::game`: returns `Result<Game, ModuleError>`; a game that calls it handles the error (`?`, or `map_err(|e| e.to_string())` in a `Start`).
Breaking: shuyi-cli: `contract::Op`: gains `Modules` and `Add`; an exhaustive `match` on it adds their arms.
Breaking: shuyi-cli: `contract::new::ENGINE_CRATES`: is a `[&str; 7]` with `shuyi` first; code naming its type changes it.

The game uses neither `BasicGame` nor `contract::Op`, so its next pin
bump needs no change for these. No game PR or ticket moves the pin to
this change yet: the game's next pin bump picks it up.
