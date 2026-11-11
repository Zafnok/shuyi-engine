# ADR-0084: One description of a game's types: serde for values, `schemars` for shapes in the tools, names given at registration, and views as the game's own types

- **Status:** Accepted; extends ADR-0005 §1 (the `scene` dump), ADR-0009
  §8 (named in types) and ADR-0022 §5 (schemas with `schemars`)
- **Date:** 2026-11-05
- **Related tickets:** 2907 (this), 2581 (the follow-up: `Scene.data`
  derived, paths and shapes in the contract), 2101 (scripts), 2221
  (scenes), 2107 (the ECS's dump), 2113 and 2125 (bots), 2515 (a file's
  schema), 2524 (the agent eval), 2356 (debug commands), 1133 (MCP),
  1135, 2506, 2507, 2232 and 1146 (parked)

## Context

Bevy, Godot, Unity and Unreal each describe a game's types once, and
their tools read that one description. Shuyi had no rule for it. The
contract's `Scene.data` (`crates/cli/src/contract/session.rs`) was a
`serde_json::Value` each game built by hand with `json!`, field by
field (`examples/hello-grid/src/tools.rs`, and the grid template's
`tools.rs`). Five planned tickets would each have described types their
own way: 2101's view types for scripts, 2221's `SceneRegistry`, 2507's
inspector, 1135's node ports and 2356's `ArgKind`.

Every stage already derives `Serialize` for its state hash
(`shuyi_core::state_hash`, ADR-0022 §4). `schemars` 1 is already in
`shuyi-cli`, and behind `schema` features of `shuyi-core`, `shuyi-ui`
and `shuyi-platform` with `default-features = false`. The skill pack's
worked examples already read scene values by JSON Pointer
(`/result/scene/data/hero/x`).

Ticket 2907 spiked three candidates on Hello, Grid's `Stage` and
`Outcome`. The numbers are in
[`docs/research/2907-game-types.md`](../research/2907-game-types.md):

| | serde + `schemars` | `serde-reflection` 0.6.0 | `bevy_reflect` 0.20.1 |
| --- | --- | --- | --- |
| Without `std` | builds | doesn't (`once_cell`) | builds; the core's tree 10 → 46 crates |
| `.wasm` added, shipped | +172,764 (value tree +21,968, schema +165,085) | +170,862 | +675,322 |
| `.wasm` added, tools only | 0 | 0 | 0 |
| Rebuild, tools (median, s; baseline 3.88) | 4.12 | 2.55 | 5.10 |
| Build units added | 16 | 7 | 31 |
| Needs met (of 8) | 1, 2, 3, 5, 7, 8 | 5, 7 | 1, 2, 3, 5, 8 |

`serde-reflection` couldn't trace `Stage`. `skip_serializing_if`
breaks it, and so does a type whose `Deserialize` checks its value
(`Grid`). Its `Value` has no field names. `bevy_reflect` serializes
through reflection, not serde's attributes, so it wrote a field the
state hash skips. Its names are module paths. No candidate gives a
stable name (need 4) or keeps state and view apart (need 6) on its own.

## Decision

### 1. The description is serde, with `schemars` for shapes

A game's types are described by their serde derives (`Serialize`,
`Deserialize`), and their shapes by `schemars::JsonSchema`. There is no
reflection crate and no derive of our own. `serde-reflection` and
`bevy_reflect` are not used. A tool that needs a value, a path, a shape
or a reading from text gets it from these derives.

### 2. Where it lives

- **The engine's types.** Every public type a game's state or view may
  hold (`Pos`, `Dir`, `TerrainId`, `Grid`, `SimRng`, the arena's ids,
  fixed-point numbers, and the like) derives `Serialize` and
  `Deserialize`, as now, and
  `#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]`. That
  goes behind its crate's `schema` feature: `shuyi-core`, `shuyi-ui` and
  `shuyi-platform` have one, and `shuyi-assets` gains one (2515). The
  dependency is `schemars = { version = "1", default-features = false,
  features = ["derive"] }`, which builds without `std`. A private type
  behind a serde `try_from` (`RawGrid`) derives it too. The facade
  `shuyi` forwards a `schema` feature to them. A pure crate gains no
  other dependency.
- **A game's types.** A game derives `JsonSchema` on its state and view
  types under its tools feature:
  `#[cfg_attr(feature = "tools", derive(schemars::JsonSchema))]`. Its
  tools feature turns on `shuyi/schema`. A shipped game builds no
  schema: the spike's shipped call cost 165 KB, and nothing outside the
  tools reads one.
- **The value tree** is `serde_json::Value` in the tools (`shuyi-cli`).
  `serde_json` with `alloc` builds without `std` too, so no `ron::Value`
  is needed. A shipped runtime that needs values builds no JSON tree. It
  goes through its own serde bridge (`rhai::serde::to_dynamic` and
  `from_dynamic` for Rhai, 2101).
- **Serialize mode and deserialize mode.** A shape of something a game
  writes (scene data, a view) uses
  `SchemaSettings::default().for_serialize()`. A shape of something
  read (a file, 2515; a component a scene loads; what a script returns)
  uses the default, deserialize mode. They differ where serde's
  attributes differ (`skip_serializing_if`, `default`, `try_from`).

### 3. Names are given at registration

A registry of types (2221's `SceneRegistry`, the types 2101 names to a
script, 1135's port types) names each type with the name the game gives
it: `register::<T>("Pos")`. That name is the one written in files and
shown to agents. The registry refuses a duplicate name and a type
registered twice. A name is never `core::any::type_name`, never
`JsonSchema::schema_name` (`Grid` for both `Grid<TerrainId>` and
`Grid<u8>`) or `schema_id` (a module path), and never a reflection
crate's type path. A schema listed under a registry is titled with the
registered name. `schemars`' own names stay keys of `$defs` inside one
schema document and are never stored.

### 4. State and view

- **The state** is the type the rules own. Its serde form is its hash
  form (ADR-0022 §4), and it is described whole. A tool build may read
  all of it: an agent debugging sees the luck to come through `scene`.
- **A view** is a type of the game's own: a plain struct or enum,
  `Serialize` (and `JsonSchema` in the tools), made from the state for
  one side by a function the game writes. A script (2101) reads a view,
  never the state. What a side can't see is left out of the view type,
  never hidden with serde attributes on the state: a `skip` on a state's
  field takes it out of the state hash too. A game whose state holds no
  luck and nothing hidden may name its state as its view. A state with a
  `SimRng` never can, since the luck to come is in it.
- **1146's `PlayerView`** is this: `type View` is the view type, which
  is `Serialize` (and `JsonSchema` in the tools), and `view_for` is the
  function.
- **2125's `Determinize`** stays as ADR-0027 decides. A bot needs a
  whole state to play the rules forward, so it gets a determinized copy
  of the state, not a view. The two agree by a property a game with both
  can test: for any seed, `view(&state.determinize(side, seed), side)
  == view(&state, side)`. A view is what `determinize` keeps
  (ADR-0027 §8). 2113's bots add no view.

### 5. Paths are JSON Pointer

A path to one value is a JSON Pointer (RFC 6901) into the value's serde
tree: `/hero/x`, `/outcome/Won/turn` (serde's default, externally tagged
enums put the variant in the path), `/units/3/hp` (a sequence by
index). The contract, 2524's grader, the skill pack's worked examples,
the inspector (2507) and scenes (2221, a path into a component's value)
all use it, through `serde_json::Value::pointer` in the tools. A path
is relative to the value it reads: `scene --path /hero/x` reads
`scene.data`.

### 6. Text dumps stay hand-written

`MapScene::to_text` (`crates/ui/src/grid/scene.rs`) and the kits'
planned text dumps (1119's arena, 2701, 2721, 2727) stay forms of their
own for snapshots and people, beside the derived data. They are a look,
like a grid drawn in characters, not a value. An agent reads values
from the data, by pointer. Nothing parses a text dump back. ADR-0003
§4's "a text dump for tests and agents" now means: the dump for tests
and people, the data for agents.

### 7. Each consumer

- **2101 (scripts, open):** the views a script reads and the values it
  returns go through the runtime's serde bridge, with no registry of the
  module's own. A script reads view types (§4), named as §3 says where
  the runtime names them. Their shapes come in the tools only.
- **2221 (scenes, open):** `SceneRegistry::register::<T>(name)` takes
  §3's name. A component is written and read back through its serde
  derives (its RON is serde's), its shape is `T: JsonSchema` in the
  tools, and a field's path is §5's pointer.
- **2107 (the ECS, open):** `World::dump` and `World::load` go through
  serde by §3's registered name, with no reflection and no registry of
  the module's own. A component that derives serde is dumped.
- **2113 (search bots, open):** they search determinized copies (§4),
  with no change.
- **2506 and 2507 (the editor, parked):** the tree reads a running
  game's values as its serde tree, the inspector names a field by §5's
  pointer, and its drop-downs read an enum's variants from the type's
  schema in serialize mode. A file's shape comes from `shuyi kinds`
  (2515), in deserialize mode.
- **1135 (visual scripting, parked):** a port's type is a registered
  name (§3) with its schema.
- **2515 (`shuyi kinds`, open):** as it planned, `schemars` on the type
  a format reads, behind `shuyi-assets`' `schema` feature, in
  deserialize mode. This ADR adds serialize mode for state and views.
- **2524 (the agent eval, open):** its `Scene` checks are §5's
  pointers into `result.scene.data`, which 2581 derives. It reads the
  keys as derived.
- **2356's `ArgKind` (open):** it stays the console's own. It says how
  one typed word parses and how the help names it, and those words are
  not a game's values. An argument of a game's own type, if 2356 adds
  one, is read back through its `Deserialize`, not given a kind of its
  own.
- **1133's `scene` tool (done):** MCP's session `scene` takes 2581's
  path and shape. 2581 carries the move.
- **The contract's `scene` (1129, done):** 2581 (§8).
- **2232's `map-types` (parked):** `schemars` as it planned, so nothing
  moves.
- **2316's readout (open):** free text for people, not a description.
- **1146 (parked):** §4.

### 8. The contract's part

Ticket 2581 builds it:

- **`Scene.data` is serde's tree of a value the game names**, by
  default its stage's state. A game writes no `json!`:
  `Scene::new(screen, kind, text).with_data(&value)` serializes it. To
  add what lives on the screen and not in the state (the commands
  taken), a game names a small struct of its own that holds the state
  with `#[serde(flatten)]`. Hello, Grid and the template keep the keys
  their worked examples and 2524's tasks read (`turn`, `hero`,
  `commands`). Keys whose form changes are listed in 2581.
- **The "no scene" default stays** for a game that names no value. The
  engine can't tell which of a game's screens holds its stage.
- **One value is read with `scene --path <pointer>`**, an option of
  `scene`, not an operation of its own. It answers the value at that
  pointer in `scene.data`. **`scene --shape`** answers the data's JSON
  Schema in serialize mode, from the `JsonSchema` the game derived.
  MCP's session `scene` takes `path` and `shape`. A one-off read stays
  `run`'s whole scene.
- Both raise `contract::VERSION` (`crates/cli/src/contract.rs`) and add
  a line to `crates/cli/schemas/versions.json`.

## Consequences

- An agent reads a game's whole state, one value of it or its shape
  through the contract. The game writes one `cfg_attr` per type and
  names its scene's value.
- The engine's public types gain a `JsonSchema` derive behind their
  crates' `schema` features. A new public type a game may hold needs
  one, or the game's schema can't name it. 2581 adds the derive to the
  types Hello, Grid and the template hold.
- A shipped game pays nothing for the description unless it builds a
  tree or a schema at run time, and nothing reads a schema there.
- serde's attributes are the one place a type's written form is
  decided. A tool reads what the hash reads, so no second form drifts.
- A game with hidden information or luck writes a view type and a
  function to make it, which a field annotation can't replace. A script
  can't see the luck to come.
- Hello, Grid's derived `outcome` changes form, from `"Won { turn: 3 }"`
  (`Debug`) to `{"Won":{"turn":3}}` (serde). Its `to_move` becomes
  `side`. 2581 lists and moves any task or worked example that reads
  them.
- JSON Schema is approximate for RON's forms (ADR-0019, 2515). A shape
  of RON a person writes comes with an example, as 2515 decides.

## Alternatives considered

- **`bevy_reflect` 0.20** met the most needs alone: in-place paths
  (`hero.x`), a registry and an enum's variants. It lost on three
  counts. It added 675,322 bytes to the shipped `.wasm`, 31 build units
  and about a second to each tools rebuild. It serializes through
  reflection, not serde's attributes, so it wrote `Stage`'s `tuning`
  that serde skips, and every `skip`, `rename` and `try_from` would be
  repeated by hand. And the orphan rule puts its derive in `shuyi-core`.
  Its names are module paths (`hello_grid::rules::Stage`), and its
  registry lists types in hash order.
- **`serde-reflection` 0.6.0** couldn't trace `Stage`. A field with
  `skip_serializing_if` gives two formats, and dropping the attribute
  changes the state hash. `Grid`'s checked `try_from` refused the
  tracer's samples. Its `Value` has no field names, so no paths. It
  doesn't build without `std`, and it adds `thiserror` 1 beside 2.
- **A derive of our own** (an `Inspect` trait): the needs serde and
  `schemars` miss, a stable name and a view, need no derive (§3, §4).
  It would add a proc-macro crate, and a third description to keep in
  step with serde.
- **Fields left out of the description instead of a view type:** any
  serde attribute that hides a field also takes it out of the state
  hash. A second description that could hide fields is what this ADR
  avoids.
- **`schemars`' or serde's container name as the registry's name:** it
  is the bare type name, the same for two instances of a generic type,
  and a type's rename would break every saved file.
- **`ron::Value` as the tree where `std` is missing:** `serde_json`
  with `alloc` built for `thumbv7em-none-eabihf` in the spike. One tree
  type is simpler, and `ron::Value` has no pointer.
- **A `path` operation of its own:** a pointer reads a scene's data, so
  it is an option of `scene`, and MCP's `scene` tool gains a field, not
  a tool.
