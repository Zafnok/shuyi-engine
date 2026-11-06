# One description of a game's types

Research for ticket 2907, measured **2026-11-05** on Nick's machine
(Windows 11, the GNU host, Rust 1.99.1) at engine commit `e2697683`.
[ADR-0083](../adr/0083-one-description-of-a-games-types-serde-for-values-schemars-for-shapes-in-the-tools-names-given-at-registration-and-views-as-the-games-own-types.md)
records the decision.

**serde is the description, and `schemars` gives its shapes in the
tools.** Hello, Grid's `Stage` already derives `Serialize` for its state
hash. `schemars` adds field names, types, doc comments and an enum's
variants with one `cfg_attr` per type. It builds without `std` and adds
nothing to a shipped `.wasm` when it stays in the tools. JSON Pointer
gives a path, and the skill pack already reads values that way.
`serde-reflection` couldn't trace `Stage` at all. `bevy_reflect` meets
the most needs on its own, but it adds 675 KB to the shipped `.wasm`,
31 build units and a second serialized form that ignores serde's
attributes. No candidate gives a stable name or keeps state and view
apart by itself. A name given at registration and a view type of the
game's own meet those two, so no derive of our own is needed.

## The eight needs

Each need is listed with the tickets that have it.

1. **A value as a tree of plain data.** These read it: the contract's
   `scene` (1129, done) and MCP's session `scene` (1133, done) through
   `Scene.data`, 2101's read-only views for a script, 2221's saved
   components and 2507's tree of a running game.
2. **A path to one value inside it.** An agent reads one field this way,
   and so does 2524's grader (`Scene { has: [(pointer, equals)] }`). The
   skill pack's worked examples already read
   `/result/scene/data/hero/x` (`skills/shuyi-input/example.ron`,
   `shuyi-stage-grid`, `shuyi-test`). 2507's inspector names a field.
3. **A shape: field names and types, and an enum's variants.** 2507's
   drop-downs, 1135's node ports, 2515's schemas, 2232's `map-types` and
   2356's typed arguments need one.
4. **A stable name per type, for a registry.** 2221 has
   `register::<T>("Pos")`, 2101 the view types a script reads, 1135 a
   port's type. `core::any::type_name` can't be the name: its
   documentation says its output may change between compiler versions.
5. **A value read back from text.** 2221 loads components by name, 2101
   reads back what a script returns, and 2356 parses a console's
   arguments.
6. **State and view kept apart.** The state hash needs every field
   (ADR-0022 §4), and `Stage` keeps the luck to come in `rng: SimRng`.
   An agent debugging may see all of it. A script (2101), a bot (2113)
   and a player under a referee server (1146) must not.
7. **The same answer on every run.** Maps must be ordered, as the state
   hash already needs.
8. **No cost in a shipped game that uses no scripts or scenes.** The
   description's code goes only in tool builds (Hello, Grid's `tools`
   feature) or behind a feature.

## What the spike built

The spike lives on the local branch `spike/2907`, one commit per
candidate. The branch was never pushed. Each commit adds a `describe`
feature to `examples/hello-grid` and three pieces of code:

- `src/describe.rs`, the candidate's description of `Stage`;
- a call to it in `StageScreen::new` (`src/screens/stage.rs`) under
  `core::hint::black_box`, so the shipped build keeps the code;
- `tests/spike.rs`, which prints what the candidate gives for `Stage`
  and `Outcome`.

Each was measured in two setups:

- **shipped:** `describe` in the default features, so the description
  and its call are in the web build;
- **tools only:** `describe` in the `tools` feature, as a game would
  keep it. The derives are `cfg_attr(feature = …)`.

`Stage` is the stage of seed 7 on Hello, Grid's own map.

### A: serde to a value tree, `schemars` for shapes

The core's `Pos`, `TerrainId`, `Grid` (and its private `RawGrid`) and
`SimRng` gained
`#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]`, the
form `shuyi-core` already uses for its playtest types behind its
`schema` feature. The facade `shuyi` gained `schema =
["shuyi-core/schema"]`. Hello, Grid's `Stage`, `Outcome`, `Side` and
`Tuning` gained
`#[cfg_attr(feature = "describe", derive(schemars::JsonSchema))]`. The
description is two calls, `serde_json::to_value(&stage)` and
`SchemaSettings::default().for_serialize().into_generator().into_root_schema_for::<Stage>()`.
Its dependencies are `schemars` 1 with `default-features = false,
features = ["derive"]`, and `serde_json` 1 with `default-features =
false, features = ["alloc"]`.

What it gave:

- **The value:** the state, every field (`map`, `goal`, `hero`, `foe`,
  `side`, `turn`, `rng`, `outcome`), keys sorted
  (`serde_json::Map` without `preserve_order` is a `BTreeMap`).
  `Outcome::Won { turn: 3 }` is `{"Won":{"turn":3}}`, so its path is
  `/outcome/Won/turn`.
- **The path:** `/hero/x` gave `1` and `/rng` gave
  `{"inc":109,"state":…}`.
- **The shape:** a JSON Schema with each field's type and each doc
  comment as its `description`. `Outcome` is a `oneOf` of `Won` and
  `Caught`, and `Side` a `oneOf` of two `const`s.
- **Two catches.** The default settings describe what serde
  *deserializes*: that schema called `tuning` required, though `Stage`
  skips it when it is the default. `for_serialize()` fixed it (`tuning`
  optional, `outcome` required and nullable). Also,
  `JsonSchema::schema_name` is `Grid` for both `Grid<TerrainId>` and
  `Grid<u8>`, and `schema_id` is the module path
  (`shuyi_core::grid::Grid<shuyi_core::grid::TerrainId>`). Neither can
  name a type in a file.
- **Read back:** `Pos` from the value's `hero`, and `Outcome` from the
  RON `Won(turn: 3)`, both through serde.

### B: `serde-reflection` 0.6.0

`Stage` gained `Deserialize` (with `#[serde(default)]` on `tuning`).
`Tracer::trace_value` traced the stage's value, and `trace_type` traced
`Outcome` and `Stage`. Its dependency is `serde-reflection` with
`default-features = false`; its default feature is `json`.

What it gave:

- **Tracing `Stage` failed twice.** First, "Incompatible formats
  detected": the value's format has no `tuning`, because of
  `skip_serializing_if`, and the type's does. Removing that attribute
  would change `Stage`'s state hash. Second, with types alone, `Grid`'s
  `try_from` refused the tracer's samples: "the grid's cell count is
  not width × height". So any type whose `Deserialize` checks its value
  needs hand-made samples.
- `Outcome` alone traced: `TypeName("Outcome")` with its variants.
- **Its `Value` has no field names.** The stage traced to a bare `Seq`
  (`Seq([Seq([U16(12), U16(10), …`), so no path can name a field.

### C: `bevy_reflect` 0.20.1

The core's `Pos`, `TerrainId`, `Grid` and `SimRng` gained
`#[cfg_attr(feature = "reflect", derive(bevy_reflect::Reflect))]`
behind a new `reflect` feature of `shuyi-core`. The orphan rule left
two ways: the derive in the core, or `#[reflect(remote = …)]` wrappers
in every game. Hello, Grid's four types gained `derive(Reflect)` under
`describe`. The description registers `Stage` in a `TypeRegistry` and
reads the stage through `GetPath`. Its dependency is `bevy_reflect`
with `default-features = false`. Its defaults are `std`, `smallvec`,
`indexmap`, `debug` and `auto_register_inventory`; the last pulls
`inventory`, a link-time registry.

What it gave:

- **Paths:** `hero.x` and `.hero.x` gave `1`, read in place with no
  copy.
- **Shapes:** each field's name and type path, and `Outcome`'s variants
  `["Won", "Caught"]`. Doc comments only come with its
  `reflect_documentation` feature.
- **Names:** type paths, `hello_grid::rules::Stage`. They change when a
  type moves module, unless each type gets `#[type_path = …]`. Its
  registry listed 28 types in hash order, so a listing needs sorting.
- **A second serialized form.** `ReflectSerializer` wrote
  `tuning:(trip_percent:25,foe_waits:0)`, which serde skips. It reads
  types through reflection, not through serde's attributes, so every
  `skip`, `rename` and `try_from` has to be kept in step by hand.

## The numbers

Each `.wasm` size is that of `dist/web/hello-grid.wasm` from `cargo
shuyi web --release` in `examples/hello-grid`. Each rebuild time is
cargo's `Finished … in` after a one-line change in `src/rules.rs`,
three times, with the median in bold. `compile-times` measures a new
game's rebuild the same way. "Shipped" is `cargo build -p hello-grid`
with `describe` in the defaults; "tools" is `cargo build -p hello-grid
--features tools --example tools` with `describe` in `tools`.

| | Baseline | A: serde + `schemars` | B: `serde-reflection` | C: `bevy_reflect` |
| --- | --- | --- | --- | --- |
| Builds without `std` (`thumbv7em-none-eabihf`) | yes | **yes**: `shuyi-core` and `shuyi-ui` with `schema`, and Hello, Grid's Playdate lib with the description | **no**: `once_cell` needs `std` | **yes**, but `shuyi-core`'s tree grows from 10 crates to 46 |
| `.wasm`, shipped, built and called | 2,990,261 | 3,163,025 (**+172,764**); value tree alone +21,968; schema alone +165,085 | 3,161,123 (**+170,862**) | 3,665,583 (**+675,322**) |
| `.wasm`, shipped, derived but not called | — | 2,990,251 (−10) | — | — |
| `.wasm`, tools only | — | 2,990,261 (**+0**) | 2,990,261 (**+0**) | 2,990,261 (**+0**) |
| Rebuild, shipped (s) | 3.21, 2.98, 3.43: **3.21** | 2.93, 2.85, 2.95: **2.93** | 2.84, 2.73, 2.50: **2.73** | 3.02, 3.94, 3.31: **3.31** |
| Rebuild, tools (s) | 3.88, 3.00, 4.61: **3.88** | 9.04, 3.76, 4.12: **4.12**; again 3.74, 4.06, 4.27: **4.06** | 3.19, 2.55, 2.39: **2.55** | 2.84, 5.42, 5.10: **5.10** |
| Build units added, clean lib build | — | 16, 20.5 s summed | 7, 8.7 s summed | 31, 95.2 s summed |
| Crates added | — | `schemars`, `schemars_derive`, `serde_derive_internals`, `serde_json`, `dyn-clone`, `ref-cast`, `ref-cast-impl`, `itoa`, `memchr`, `zmij`; all already in the tools build, through `shuyi-cli` | `serde-reflection`, `erased-discriminant`, `thiserror` 1 (beside 2) | `bevy_reflect` and 6 more `bevy_*`, `syn` 3 beside 2, two `hashbrown`s, `indexmap`, `derive_more`, `toml_edit`, `winnow`, `erased-serde`, `downcast-rs`, `spin` and others |
| License, `cargo deny check licenses bans` | | MIT; MIT OR Apache-2.0. Passes | MIT OR Apache-2.0. Passes, with a duplicate `thiserror` | MIT OR Apache-2.0. Passes |

The first set of A's tools rebuilds was 45.11, 41.46 and 5.98 s, and
the next two sets were under 10 s each. The two long ones match no
change in the code, so they are left out above and recorded here.
Rebuild times move by a second or more between sets on this machine, so
A, B and the baseline are the same within noise. C's tools rebuild is
about a second longer.

From `cargo build --timings` on a clean build of the lib alone
(`--no-default-features --features std,describe`), the game crate
itself took 7.35 s with A, 4.84 s with B and 7.06 s with C, against
5.78 s and 7.64 s in two baseline runs. Wall-clock totals of those
clean builds swung between 28.6 s and 49.7 s with machine load, so the
added units above compare better than the totals do.

**Bytes in the shipped build come from calls, not derives.** A's derive
with no call adds nothing (−10 bytes). Building the JSON tree costs
22 KB, and generating a schema at run time costs 165 KB, mostly the
generator and the doc strings. A shipped game that runs scripts needs
values, through its runtime's serde bridge, and no schemas. So the
schemas belong in the tools.

## Which needs each meets

"Yes" means a game gets it with no code written for the purpose.

| Need | A: serde + `schemars` | B: `serde-reflection` | C: `bevy_reflect` |
| --- | --- | --- | --- |
| 1. Value tree | yes: `serde_json::Value`, which also builds without `std` (`alloc`), so no `ron::Value` is needed | no: a `Value` with no field names | yes: the value itself, or a `DynamicStruct` |
| 2. Path | yes: JSON Pointer, `Value::pointer` | no | yes: `GetPath`, `hero.x` |
| 3. Shape | yes, with doc comments; serialize mode for what a game writes, deserialize mode for what it reads | no: fails on `Stage` (`skip_serializing_if`, `Grid`'s check) | yes, without docs unless `reflect_documentation` |
| 4. Stable name | no: `schema_name` is the bare name, the same for two `Grid`s | no: serde's container name, the same problem | no: the module path, unless each type gets `#[type_path]` |
| 5. Read back | yes: serde, any format (RON, JSON) | yes: serde | yes, but through reflection, not serde's attributes |
| 6. State and view | no by itself; a `skip` on the state also changes its hash | no; the same | no by itself; `#[reflect(ignore)]` would hide a field from reflection only, a second description to keep in step |
| 7. Same every run | yes: sorted keys | yes: a `BTreeMap` registry | values yes; the registry iterates in hash order |
| 8. No cost shipped | yes, behind a feature: +0 in the tools, +22 KB for a value tree | yes in the tools; `std` only | yes in the tools; +675 KB shipped |

The code a game writes for each:

- **A:** one line per state or view type,
  `#[cfg_attr(feature = "tools", derive(schemars::JsonSchema))]`. Its
  `Serialize` and `Deserialize` derives are there already. The engine's
  types derive `JsonSchema` behind their crates' `schema` features.
- **B:** `Deserialize` on every type, no `skip_serializing_if`, which
  changes the state hash, and hand-made samples for every type whose
  `Deserialize` checks its value.
- **C:** `derive(Reflect)` on every type, and on every engine type a
  state holds (in the engine, because of the orphan rule).
  `#[type_path]` for a stable name, each root type registered, and every
  serde attribute repeated in reflection's terms.

### D: a derive of our own, not built

A derive of our own (an `Inspect` trait in `shuyi-core`) would cover
only what A misses, needs 4 and 6. Neither needs a derive:

- **Need 4:** a registry takes the name it is given (2221 already
  writes `register::<T>("Pos")`). That name survives a type's rename or
  move, which no derived name does.
- **Need 6:** a view is a type of the game's own, because any attribute
  that hides a field from serde also hides it from the state hash.

A derive would cost a proc-macro crate (more `syn` in every game's
build), a third description beside serde and `schemars` to keep in
step, and its own schema writer. So it isn't built.

## How the ECS candidates of 2904 fit

2904 is open. `bevy_ecs` 0.20 turns `bevy_reflect` on among its default
features, and the ECS is opt-in (ADR-0003 §4, 2107), so a game on the
arena must be described as well. Under ADR-0083:

- **`bevy_ecs`:** with `default-features = false` and `bevy_reflect`
  off, components derive serde (and `JsonSchema` in the tools) like any
  type, and `World::dump` writes them through serde by registered name.
  With `bevy_reflect` on, the C column applies: 675 KB, and a second
  serialized form.
- **`hecs`:** it has no reflection. Its serialization features ask the
  game for each component's id, and the registered name is that id.
- **Our own:** the same as `hecs`, with the names in `shuyi-ecs`.

## Commands

The spike's commits are on `spike/2907`. Each was measured from the
repository's root, in `examples/hello-grid` for the web builds:

```text
# a candidate's output
cargo test -p hello-grid --features describe,tools --test spike -- --nocapture
# without std
cargo build -p shuyi-core -p shuyi-ui --no-default-features --features shuyi-core/schema,shuyi-ui/schema --target thumbv7em-none-eabihf
cargo build -p hello-grid --lib --no-default-features --features playdate,describe --target thumbv7em-none-eabihf
cargo build -p shuyi-core --no-default-features --features reflect --target thumbv7em-none-eabihf
# B: a crate of its own, `serde-reflection = { version = "0.6", default-features = false }`
cargo build --target thumbv7em-none-eabihf
# the .wasm, with `describe` in `default`, then in `tools`
cargo shuyi web --release
# the rebuilds: append `// rebuild N` to src/rules.rs, build, restore; three times
cargo build -p hello-grid
cargo build -p hello-grid --features tools --example tools
# where the time goes, in a target folder of its own
cargo build -p hello-grid --lib --no-default-features --features std,describe --timings
cargo deny check licenses bans
```

The shipped `.wasm` was also measured with `StageScreen::new` calling
only `serde_json::to_value`, then only `schemars::schema_for!`, then
nothing.
