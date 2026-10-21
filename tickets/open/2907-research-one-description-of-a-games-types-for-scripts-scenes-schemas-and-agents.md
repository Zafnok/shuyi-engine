---
id: "2907"
title: "Research: one description of a game's types and values for scripts, scenes, schemas and agents, decided in an ADR before 2101 and 2221 start"
type: research
milestone: After MVP
model: opus-5.5
effort: high
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2907 — Research: one description of a game's types

## Context

Found by the 2026-10-20 comparison with Bevy 0.20, Godot 4.7, Unity 6
and Unreal 5.8. Each of them describes a game's types once, and its
tools read that one description:

- **Bevy:** `bevy_reflect`. The remote protocol's
  `world.get_components`, `world.query`, `world.mutate_components` and
  `registry.schema` name types from its registry
  (<https://docs.rs/bevy_remote/latest/bevy_remote/>), and
  `bevy_brp_mcp` hands them to agents. Bevy's scenes use the same
  registry.
- **Godot:** every class's properties are registered once (`ClassDB`).
  The inspector, the remote scene tree of a running game and saved
  scenes read them.
- **Unity:** the serializer. The Inspector shows the fields it saves.
- **Unreal:** `UPROPERTY` and `UFUNCTION`. The Details panel, Blueprints
  and saving read them.

**What Shuyi has today** (read on 2026-10-20):

- `crates/cli/src/contract/session.rs`: `Scene { screen, kind, text,
  data }` (line 21), where `data: serde_json::Value` is "its stage's
  state". A game gives it with `Headless::with_scene` (line 152).
  Without it, `Headless::new` (line 137) answers "the game's tools give
  no scene".
- Each game builds `data` by hand with `json!`, field by field:
  `examples/hello-grid/src/tools.rs` lines 235–256, and
  `templates/grid/crates/game/src/tools.rs` (`scene`, line 84). Yet both
  stages already derive `Serialize` for their state hash
  (`examples/hello-grid/src/rules.rs` line 155,
  `templates/grid/crates/rules/src/lib.rs` line 128, read by
  `shuyi_core::state_hash` in `crates/core/src/hash.rs` line 61). So
  serde describes the state already, and the closure only reshapes it.
- `scene` answers the whole scene (`run::SceneOutput`,
  `crates/cli/src/contract/run.rs` line 58). It has no path to one value
  and no shape for `data`.
- `schemars` 1 (MIT) is in `shuyi-cli` only, with its `std` feature
  (`crates/cli/Cargo.toml` line 44), for the contract's own schemas
  (ADR-0022 §5). `shuyi-core` uses `serde` without `std` (and `ron` only
  in its tests). `shuyi-ui` and `shuyi-assets` use `serde` and `ron`
  without `std`.
- ADR-0009 §8 says what a module can do "is named in types and in the
  contract". No ADR says how a game's own types are described.

**What is planned**, each with a description of its own:

- 2101 (the script runtime, for game#1017): "a way for the game to
  register its read-only view types";
- 2221 (scenes): a `SceneRegistry` the game fills with
  `register::<T>("Pos")`, for the ECS only, over 2107's `World::dump()`;
- 2507 (parked): the inspector's schema is left to 2506: "a derive,
  `serde-reflection`, or the format's own `Format`";
- 1135 (parked): a node library with no source of types;
- 2515: `schemars` on the type a format reads, for a file's shape;
- 2356: its own `ArgKind` for a console's arguments;
- 1133: an MCP `scene` tool that answers the whole scene;
- 2524: its grader reads `result.scene.data` by JSON pointer, so tasks
  already depend on the shape of the templates' `data`;
- text dumps in the kits (1119's arena, 2701, 2721, 2727), beside
  `MapScene::to_text` (`crates/ui/src/grid/scene.rs` line 424).

Two designs already keep a view apart from the state: 1146 (parked)
gives each player a `PlayerView::View` the game writes, and 2125's
`Determinize` redraws the luck to come before a bot searches.

Built apart, that is four or more registries to keep in step for years.

**Why it matters:**

- **The game first.** Visions of Shuyi's scripted rules (game#1017) are
  2101's first user, and the contract's `scene` is how an agent reads a
  stage. 2101 is a request from the game and had no blocker, so it
  could start soon and make this choice alone. So 2101 and 2221 list
  this ticket in `blocked_by`, and the choice lands before either
  starts.
- **Other 2D games.** Every game with a stage writes the same `json!`
  closure today, and each will meet scripts, scenes and an inspector.
- **The end goal** (Nick, 2026-10-19): the most AI-friendly 2D engine in
  Rust. An agent reads one value of a game, and its shape, with no code
  the game wrote for it.

The answer is likely small. serde already describes every stage, and
`schemars` already makes schemas. Rhai's `serde` feature turns any
`Serialize` value into a script value (`rhai::serde::to_dynamic`).
2221's components are RON through serde. So the default is serde and
`schemars`, and a derive of our own only if the measurements show a need
serde can't meet. This ticket checks that and writes it down.

## Nick input

None. This is technical.

## Scope

**In:**
- **The needs**, written first in `docs/research/game-types.md` (create
  `docs/research/` if 2904 or 2906 hasn't), each with the tickets that
  have it:
  1. a value as a tree of plain data (`scene`, MCP, 2101's views,
     2221's components);
  2. a path to one value inside it (one field for an agent, 2524's
     grader, a field of 2507's inspector);
  3. a shape: field names and types, and an enum's variants (2507's
     drop-downs, 1135's node ports, 2515's schemas);
  4. a stable name per type, for a registry (2221, 2101, 1135).
     `core::any::type_name` can't be it: its documentation says the
     output may change between compiler versions;
  5. a value read back from text (2221 loads components by name; 2101
     reads back what a script returns);
  6. **state and view kept apart.** The state hash needs every field
     (ADR-0022 §4), and Hello, Grid's `Stage` keeps the luck to come in
     a `SimRng` field. An agent debugging may see all of it. A script
     (2101) or a bot (2113) must not, and neither may a player under a
     referee server's hidden information (1146). Decide how a view
     differs from the state: a type the game writes, or fields the
     description leaves out;
  7. the same answer on every run: maps ordered, as the state hash
     already needs;
  8. no cost in a shipped game that uses no scripts or scenes: the
     description's code only in tool builds (Hello, Grid's `tools`
     feature, `examples/hello-grid/Cargo.toml`) or behind a feature.
- **The candidates**, each spiked on Hello, Grid's `Stage` and its
  `Outcome` enum (`examples/hello-grid/src/rules.rs` line 137):
  - **serde to a value tree, with `schemars` for shapes.** The tree is
    `serde_json::Value` in the tools, or `ron::Value` where `std` is
    missing. Paths are JSON Pointer (RFC 6901), through
    `serde_json::Value::pointer`. Check whether `schemars` builds
    without its `std` feature for `thumbv7em-none-eabihf`.
  - **`serde-reflection`** (its latest release; MIT OR Apache-2.0):
    traces `Serialize` and `Deserialize` types into a registry of
    formats, with names and variants. Tracing a type needs
    `Deserialize`, which `Stage` doesn't derive, so the spike adds it
    (every field's type derives it already).
  - **`bevy_reflect`** 0.20 (MIT OR Apache-2.0): a `Reflect` derive,
    paths (`GetPath`) and a type registry. `Stage`'s fields are the
    core's types (`Pos`, `Grid`, `SimRng`), so they need the derive in
    `shuyi-core` too, or are marked opaque. The note says which.
  - **A small derive of our own** (an `Inspect` trait in `shuyi-core`),
    sized from the needs the others miss. Not built.

  For each, measure and record:
  - whether it builds without `std` where the types live (`shuyi-core`,
    `shuyi-ui`; 3005's `thumbv7em-none-eabihf` gate);
  - the bytes it adds to Hello, Grid's `.wasm` (`cargo shuyi web
    --release` in `examples/hello-grid`), with the description in the
    shipped build, and with it in the `tools` feature only;
  - the rebuild after a one-line change in Hello, Grid: 2009's `cargo
    xtask compile-times` if 2009 is done, else `cargo build --timings`;
  - which of the eight needs it meets, and the code a game writes for
    each;
  - its license against `deny.toml`, and its default features (ADR-0004
    §7).

  The spike lives on a local branch of its own (`spike/2907`), never
  pushed. The ticket's branch gets only the note, the ADR and the
  ticket edits, so its `Cargo.lock` doesn't change. The note says what
  the spike built and the commands it ran, so it can be redone.
- **An ADR** (the `write-adr` skill, the next free number), extending
  ADR-0005 §1 (the `scene` dump), ADR-0009 §8 and ADR-0022 §5. It
  decides:
  1. **The description:** one of the candidates, with the numbers.
     Default: serde and `schemars`.
  2. **Where it lives:** which crates derive what, under which
     features. `shuyi-core` stays `no_std` and gains nothing it doesn't
     need.
  3. **Names:** how a type gets its stable name (serde's container name,
     `schemars`' schema name, or a name given at registration).
  4. **State and view:** the rule for need 6, and how it fits 1146's
     `PlayerView` and 2125's `Determinize`.
  5. **Paths:** the syntax, used alike by the contract, 2524's grader,
     the inspector (2507) and scenes (2221).
  6. **Text dumps:** whether `MapScene::to_text` and the kits' planned
     dumps stay hand-written forms for snapshots and people, beside the
     derived data, or come from the description.
  7. **Each consumer**, a line each: 2101, 2221, 2107, 2113, 2506 and
     2507, 1135, 2515, 2524, 2356's `ArgKind`, 1133's `scene` tool, and
     2316's readout (which stays free text for people). For a consumer
     that is already done, its line says whether it moves onto the
     description; a build ticket does the move.
  8. **The contract's part:** whether `Scene.data` comes from the
     stage's state by default, so the `json!` closures and the "no
     scene" default go for a game that derives; and whether one value
     is read with `scene --path <pointer>` or an operation of its own.
     Either is a raise of `contract::VERSION`
     (`crates/cli/src/contract.rs`) and a new line in
     `crates/cli/schemas/versions.json`.
- **The follow-up ticket** (the `write-ticket` skill, the next free 25xx
  number, After MVP, not parked, with no parked ticket in its
  `blocked_by`) for point 8: `Scene.data` derived in Hello, Grid and the
  templates, and path reads in the contract. It keeps the keys 2524's
  tasks read in `scene.data` (`turn` and the others), or changes those
  tasks in the same PR. If 1133 is still open, its `scene` tool gains
  the path read in its scope, edited in this PR; if 1133 is done, the
  follow-up adds it.
- **Tickets this changes**, in the same PR: 2101, 2221, 2506, 2507 and
  1135 name the ADR by number and its choice in their scope (where they
  say "2907's ADR", put the number). 2107, 2356, 2515, 2524, 2904 and
  the kits with a planned text dump (1119, 2701, 2721, 2727) too, if
  they are still open and the ADR changes them. If one needs no change,
  the completion notes say why.
- **Either order with 2904:** `bevy_ecs` 0.20 turns on `bevy_reflect`
  among its default features. The arena stays the default (ADR-0003
  §4) and an ECS is opt-in (2107), so the description can't be one only
  ECS games have. If 2904 is done, read its ADR and say how its choice
  fits this one. If not, the ADR says how each of 2904's candidates
  (`bevy_ecs`, `hecs`, our own) would meet it, and 2904's clause on this
  ticket is checked against the ADR.
- **Either order with 2515:** if 2515 is done, its ADR already derives a
  format's schema with `schemars` on the type serde reads. Start from
  it, extend it to a game's state and view types, and state any
  difference. If not, this ADR decides, and 2515 follows it (its own
  clause says so).
- **Either order with 2125:** if 2125 is done, point 4 starts from its
  `Determinize`. If not, 2125's ADR says how `Determinize` fits this one
  (its own clause says so).
- `docs/adr/README.md`: the new ADR's row, and "extended by ADR-NNNN" on
  the rows of ADR-0005, ADR-0009 and ADR-0022.

**For the game:** nothing changes until 2101. The ADR shapes what 2101
hands the game.

**Out (do not do):**
- building the description, derived `Scene.data` or path reads (the
  follow-up ticket);
- a derive of our own in the core: if the ADR picks one, a build ticket
  does it;
- writing fields at run time, and debug commands that change state
  (2356);
- the editor and its inspector (2506–2508, parked);
- the ECS choice (2904);
- a file's shape for `shuyi kinds` (2515);
- a view per player under a referee server (1146, parked);
- reading a running windowed build (2521's live link);
- 2316's readout, which stays free text.

## Implementation steps

1. Read ADR-0002, ADR-0003 §4, ADR-0004 §7, ADR-0005 §1, ADR-0009 §8
   and ADR-0022 §4–§5; `crates/cli/src/contract/session.rs`, `run.rs`
   and `crates/cli/src/contract.rs`; Hello, Grid's `tools.rs` and
   `rules.rs`; and tickets 2101, 2107, 2113, 2221, 2356, 2506, 2507,
   1135, 1146, 2515, 2524 and 2904, with their ADRs if they are done.
2. Write the eight needs in `docs/research/game-types.md`.
3. On the local branch `spike/2907`, spike each candidate on Hello,
   Grid's `Stage` and `Outcome`, and take the measurements above. Note
   every command.
4. Switch back to the ticket's branch. Write the numbers in the
   research note, then the ADR and its rows in `docs/adr/README.md`.
5. Write the follow-up ticket, and edit the tickets above.
6. Run the gates.

## Acceptance criteria

- [ ] `docs/research/game-types.md` lists the eight needs and, for each
      candidate, the `no_std` result, the `.wasm` bytes it adds (shipped
      and tools only), the rebuild time, the needs it meets, its
      license and the commands that measured it.
- [ ] The ADR decides the eight points, lists the candidates not chosen
      with their numbers, and `docs/adr/README.md` lists it and marks
      ADR-0005, ADR-0009 and ADR-0022 as extended by it.
- [ ] 2101, 2221, 2506, 2507 and 1135 name the ADR by number in their
      scope; 2107, 2356, 2515, 2524 and 2904 do too, or the completion
      notes say why not.
- [ ] The follow-up ticket is in `tickets/open/`, After MVP, with no
      parked ticket in its `blocked_by`.
- [ ] `Cargo.lock` is unchanged (the spike isn't committed).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

None (research). The ticket lint covers the new and edited tickets.

## Completion notes

*(Filled in by the session that completes the ticket.)*
