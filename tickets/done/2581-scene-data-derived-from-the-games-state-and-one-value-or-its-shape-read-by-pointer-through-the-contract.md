---
id: "2581"
title: "Scene data derived from the game's state, and one value or its shape read by pointer through the contract"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: medium
status: done
blocked_by: []
nick_input: none
completed: 2026-11-07
---

# 2581 — Scene data derived from the game's state, and one value or its shape read by pointer

## Context

[ADR-0084](../../docs/adr/0084-one-description-of-a-games-types.md)
(ticket 2907) decides that a game's types are described by their serde
derives, with `schemars::JsonSchema` for their shapes in the tools
(§1–§2), and that a path to one value is a JSON Pointer (§5). Its §8
says what the contract does with that, and this ticket builds it. The
measurements are in
[`docs/research/2907-game-types.md`](../../docs/research/2907-game-types.md).

What there is today:

- `Scene { screen, kind, text, data }` in
  `crates/cli/src/contract/session.rs`, where `data:
  serde_json::Value` is "its stage's state". Each game builds `data` by
  hand with `json!`, field by field: `scene` in
  `examples/hello-grid/src/tools.rs` and in
  `templates/shuyi-default/stages/top-down-grid/crates/game/src/stages/top_down_grid/tools.rs`.
  Yet both stages already derive `Serialize` for their state hash.
- `scene` answers the whole scene (`run::SceneOutput`,
  `crates/cli/src/contract/run.rs`), with no path to one value and no
  shape for `data`. MCP's session `scene` tool (1133, done;
  `crates/cli/src/mcp.rs`) answers the same.
- The skill pack's worked examples read
  `/result/scene/data/hero/x`, `/hero/y`, `/turn` and `/commands`
  (`skills/shuyi-input/example.ron`, `skills/shuyi-stage-grid/example.ron`,
  `skills/shuyi-test/example.ron`).
- `shuyi-core`, `shuyi-ui` and `shuyi-platform` have `schema` features
  with `schemars` (`default-features = false, features = ["derive"]`),
  but the core's grid types (`Pos`, `Dir`, `TerrainId`, `Grid`,
  `SimRng`) don't derive `JsonSchema`.

Why it matters: an agent reads a game's state, one value of it and its
shape with no code the game wrote for it. That is the end goal (Nick,
2026-10-19: the most AI-friendly 2D engine in Rust). Every game with a
stage writes the `json!` closure today.

## Nick input

None.

## Scope

**In:**
- **The engine's types:** the public types a game's state holds derive
  `#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]`. In
  `shuyi-core` that is at least `Pos`, `Dir`, `TerrainId`, `Grid` (and
  its private `RawGrid`, the `try_from` source) and `SimRng`, and any
  other type Hello, Grid's or the template's state holds. The facade
  `shuyi` gains `schema = ["shuyi-core/schema", "shuyi-ui/schema",
  "shuyi-platform/schema"]`. `cargo xtask check-no-std` and the
  `thumbv7em-none-eabihf` builds of CI's `no-std` job still pass with
  `schema` on: 2907's spike built `shuyi-core` and `shuyi-ui` with it
  there.
- **`Scene::with_data`:** `Scene::with_data<T: Serialize +
  JsonSchema>(self, value: &T) -> Result<Self, String>`. It sets `data`
  to `serde_json::to_value(value)`, and keeps the shape of `T` in
  serialize mode
  (`SchemaSettings::default().for_serialize().into_generator().into_root_schema_for::<T>()`,
  ADR-0084 §2), in a field that isn't part of the answer unless asked
  for. `Scene` stays `#[non_exhaustive]` (ADR-0039), and the change is
  additive.
- **`scene --path <pointer>` and `scene --shape`:** options of the
  `scene` operation and its input type, not operations of their own
  (ADR-0084 §8):
  - `--path` answers the value at that JSON Pointer (RFC 6901) in
    `scene.data`, through `serde_json::Value::pointer`. A pointer that
    names nothing is a failed answer with a problem code of its own
    (ADR-0028: a page in `crates/cli/errors/` and its line). Its
    message names the pointer and the keys at the deepest object it
    reached;
  - `--shape` answers the data's JSON Schema, from `with_data`. A scene
    built with no `with_data` answers no shape, and the answer's limits
    say why;
  - both work with `--json` and with the other options of `scene`.
- **MCP:** the session's `scene` tool takes optional `path` and `shape`
  arguments and answers the same. A one-off read stays `run`'s whole
  scene (ADR-0084 §8).
- **The contract's version:** raise `contract::VERSION`
  (`crates/cli/src/contract.rs`), regenerate `crates/cli/schemas/`, and
  add the version's line to `crates/cli/schemas/versions.json`, as the
  schema test says.
- **Hello, Grid:** `scene` in `examples/hello-grid/src/tools.rs` names
  a struct of the game's own, which derives `Serialize`, and
  `JsonSchema` under the `tools` feature. It holds the state with
  `#[serde(flatten)]`, plus what lives on the screen (`commands`, the
  number of commands taken). No `json!` is left. `Stage`, `Outcome`,
  `Side` and `Tuning` derive
  `#[cfg_attr(feature = "tools", derive(schemars::JsonSchema))]`, and
  `tools` turns on `shuyi/schema` and the dependency `schemars`.
- **The template** (`templates/shuyi-default/stages/top-down-grid/`):
  the same. Its rules crate gains a `schema` feature that the game
  crate's `tools` turns on, so the rules crate stays pure. Check its
  `crates/rules/tests/layers.rs` and the template's `Cargo.toml`s
  against `cargo xtask check-pins`. Add `schemars` to the root's
  `[workspace.dependencies]` if the template's crates name it by
  `workspace = true`.
- **The keys agents read:** the derived data keeps `turn`, `hero` (with
  `x` and `y`), `goal`, `foe` / `wanderer`, `outcome` and `commands`.
  Two change form:
  - `outcome` becomes serde's form, `{"Won":{"turn":3}}`, not
    `"Won { turn: 3 }"`;
  - Hello, Grid's `to_move` becomes the state's own `side`.

  The template's `seed` lives on its screen, so the screen's struct
  keeps it. The whole state is in the data, as ADR-0084 §4 allows a
  tool build (`map` and `rng` included). Update every worked example,
  test, snapshot and doc that reads a key whose form changed.
- **Either order with 2524:** if 2524 is done, run its tasks' `Scene`
  checks and the reference solutions against the derived data, and
  change any pointer whose key changed form, in this PR. If not,
  nothing to do: 2524 writes its tasks against the derived data.
- **Docs:** `docs/contract.md` (the `scene` options and the example
  answer), the book's chapter 10
  (`site/content/learn/book/tools-for-agents.md`), and the skill pack's
  skills that teach `scene` (`skills/shuyi-stage-grid/SKILL.md`,
  `skills/shuyi-test/SKILL.md`, and any other that shows a scene's
  data). `cargo shuyi check-skills skills` passes.
- **For the game:** additive (`Scene::with_data`, the new options). The
  game's own `json!` scene keeps working. Its PR that moves the pin may
  switch to `with_data`.

**Out (do not do):**
- the derive of a view type and the property between a view and
  `Determinize` (ADR-0084 §4): games with hidden information write them
  (2101, 1146);
- a registry of types by name (2221's `SceneRegistry`, 2101);
- writing values at run time, and debug commands (2356);
- a path option on `run`, which answers the whole scene;
- `MapScene::to_text` and the other text dumps, which stay hand-written
  (ADR-0084 §6);
- the editor's tree and inspector (2506, 2507, parked).

## Implementation steps

1. Read ADR-0084, ADR-0022 §5, ADR-0028, ADR-0039, ADR-0031 (MCP), the
   2907 research note, `crates/cli/src/contract/session.rs`, `run.rs`,
   `crates/cli/src/contract.rs` and `crates/cli/src/mcp.rs`, and both
   `tools.rs` above.
2. Derive `JsonSchema` on the core's types under `schema`; add the
   facade's feature; build the `no-std` job's commands with `schema` on.
3. Add `Scene::with_data` and the kept shape, with unit tests.
4. Add `--path` and `--shape` to `scene`'s input and answer, the
   problem code and its page, and the MCP tool's arguments. Raise the
   version and regenerate the schemas.
5. Move Hello, Grid onto `with_data`; then the template.
6. Update the worked examples, tests, snapshots and docs that read a
   changed key; run `cargo shuyi check-skills skills`.
7. Run the gates.

## Acceptance criteria

- [x] No `json!` builds a scene's data in Hello, Grid or the template.
- [x] `cargo shuyi scene --script "Enter Right" --path /hero/x --json`
      in `examples/hello-grid` answers the piece's column, and in a game
      made from the template too (a test in each).
- [x] `scene --shape` answers a JSON Schema whose properties include
      `turn`, `hero` and `commands`, and whose `outcome` names `Won` and
      `Caught` (a test).
- [x] A pointer that names nothing gives a failed answer with the new
      problem code, and `shuyi explain` shows its page.
- [x] MCP's session `scene` takes `path` and `shape` (a test in
      `crates/cli/tests/`).
- [x] `contract::VERSION` is raised, and the schema test passes with the
      new line in `versions.json`.
- [x] `cargo build -p shuyi-core -p shuyi-ui --no-default-features
      --features shuyi-core/schema,shuyi-ui/schema --target
      thumbv7em-none-eabihf` passes.
- [x] Hello, Grid's `.wasm` from `cargo shuyi web --release` is no
      larger than before (the description stays in `tools`).
- [x] The skill pack's worked examples pass (`cargo shuyi check-skills skills`).
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `Scene::with_data` (data and shape for a small struct with an
  enum and a skipped field; the shape is in serialize mode); a pointer
  read, one that names nothing, and the error's keys.
- Integration: `scene --path` and `--shape` through the CLI and through
  MCP on Hello, Grid; the template's game test the same; the schema
  test.
- Snapshot: the contract's example answer in `docs/contract.md`, if a
  test checks it.

## Completion notes

What was done:

- **The engine's types.** `Pos`, `Dir`, `TerrainId`, `Grid` (and its
  private `RawGrid`) and `SimRng` derive `JsonSchema` behind
  `shuyi-core`'s `schema` feature. The facade `shuyi` gains `schema`
  (`cargo xtask check-modules` counts it with `std` and `app` as an
  engine feature, not a module; the API check's catalog builds it).
  The `no-std` job's builds and `check-no-std` pass with it on.
- **`Scene::with_data`** keeps the value's serde tree in `data` and the
  JSON Schema of `T` in serialize mode in `Scene::shape`, which serde
  skips: it is never in an answer unless asked for. `session::at_pointer`
  reads a value by JSON Pointer and, when it names nothing, says the
  keys of the deepest object reached.
- **`scene --path <pointer>` and `scene --shape`** are fields of
  `RunInput` (`path`, `shape`); `run` refuses them (`SH0001`: it answers
  the whole scene). The answer, `SceneOutput`, gains `value` and
  `shape`, left out when not asked; with either, the answer leaves the
  scene out, as it answers what was asked instead. A pointer that names
  nothing fails with the new `SH0005` (`no-value`, usage class, exit 2),
  its page in `crates/cli/errors/SH0005.md`. A malformed pointer (not
  empty and not starting with `/`) gets the same code. A scene with no
  shape answers `ok` with the limit `NO_SHAPE`.
- **MCP:** the session's `scene` takes `SceneInput { path, shape }`
  (the headless host and the session attached over the live link, whose
  frame scene has no data).
- **`contract::VERSION` 40** (main took 39 meanwhile), the schemas regenerated, its line in
  `versions.json`.
- **Hello, Grid** names `tools::StageData { #[serde(flatten)] stage,
  commands }`; `Stage`, `Outcome`, `Side` and `Tuning` derive
  `JsonSchema` under `tools`, which turns on `shuyi/schema` and
  `schemars` (added to the root's `[workspace.dependencies]`, so
  `check-pins` holds the template to it).
- **The template:** its rules crate gains `schema`
  (`dep:schemars`, `shuyi-core/schema`) on `Stage` and `Outcome`; the
  stage's part of the game crate's `Cargo.toml` adds to `tools`
  (`rules/schema`, `shuyi/schema`, `dep:schemars`), which `shuyi new`
  merges into the base's. Its `StageData` keeps `seed` and `commands`.
  `plans.golden` re-blessed.
- **Keys:** `turn`, `hero`, `goal`, `foe` / `wanderer`, `outcome` and
  `commands` stay; `outcome` is serde's form, Hello, Grid's `to_move` is
  `side`, and `map`, `rng` (and Hello, Grid's `tuning`, when not the
  default) are new. No worked example, test or doc read a key whose form
  changed but Hello, Grid's contract test, now on `side`.
- **2524** is still open: its clause now says to write its checks
  against the derived data.
- **Docs:** `docs/contract.md` (the `scene` row, a section *The scene's
  data*, the session table, the example answer's version), the book's
  chapter 10, `shuyi-stage-grid` and `shuyi-test`, the template rules'
  `AGENTS.md`, and the module docs of `contract` and `mcp`.

Checked: `cargo shuyi scene --script "Enter Right" --path /hero/x
--json` in `examples/hello-grid` answers `value: 1`, `--path /hero/z`
exits 2 with `SH0005`, and `--shape` lists the keys and `Outcome`'s
`Won` and `Caught`. Hello, Grid's `dist/web` from `cargo shuyi web
--release` is 3,115,476 bytes on `main` and on this branch: `schemars`
isn't in the shipped build's tree. A game made from the `grid` template
passes `fmt`, clippy and its tests, the new one among them.

Deviations: with `path` or `shape`, `scene`'s answer leaves the scene
out (the ticket said only that it "answers the value"). A shape asked of
a scene with none is `ok` with a limit, as the ticket says.

Follow-ups: none.

For the game: additive but for `SceneOutput` (below). The game's own
`json!` scene keeps working; its pin bump may move to `with_data`.

Breaking: shuyi-cli: contract::run::SceneOutput: it is `#[non_exhaustive]` and has two more fields, `value` and `shape`; a game reads it from `scene_output` or an answer instead of building it with a struct literal

```migration
since: 0.1.0
what: SceneOutput can't be built with a struct literal outside shuyi-cli any more; it grew value and shape.
old: SceneOutput { frames, top_screen, scene, log, session }
new: run::scene_output(session)
before: |
  let output = SceneOutput { frames, top_screen, scene, log, session: None };
after: |
  let (output, limits) = shuyi_cli::contract::run::scene_output(&*session);
find: ["SceneOutput {"]
```

Book: chapter 10 (`tools-for-agents.md`) gains *One value of the game*
(`scene --path`, `--shape`, `Scene::with_data` and `StageData`) and the
session's `scene` row.

Nick input: none.
