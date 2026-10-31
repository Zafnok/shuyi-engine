---
id: "2232"
title: "Parked: Map objects as the game's own types: Tiled and LDtk objects read into a game's enum, checked in shuyi check, and the game's types written to Tiled and checked against LDtk"
type: feature
milestone: "Engine: real time"
model: opus-5.5 / gpt-6.1-sol
effort: high
status: blocked
blocked_by: ["1119", "1124"]
nick_input: none
completed:
---

# 2232 — Map objects as the game's own types

> **Parked** (Nick, 2026-10-20: "unless these real-time work tickets overlap with visions of shuyi they should be parked until visions of shuyi is complete"): this builds on 1124, which is real-time work, so it waits until Visions of Shuyi launches, unless the game needs it first. The game's ticket 0060 (the army walks around freely) unparks 1119–1125; if the game then draws maps in Tiled or LDtk and places things on them, this comes with 1124. Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

Found by the 2026-10-20 comparison with Bevy 0.20, Godot 4.7, Unity 6
and Unreal 5.8. Shuyi has no scene editor of its own: levels are drawn
in Tiled or LDtk (ADR-0004 §2; the editor, 2506 and 2507, is parked).
The benchmarks give a level designer typed things:

- **Bevy's ecosystem:** `bevy_ecs_ldtk` spawns LDtk entities through a
  derive. `bevy_ecs_tiled` reads Tiled's custom properties into
  registered types, writes those types to a file that Tiled imports as
  its custom types, and loads Tiled worlds
  (https://github.com/adrien-bon/bevy_ecs_tiled).
- **Godot and Unity** edit levels in their own editors, where an
  object's fields have the script's types. Godot's tile sets also have
  typed custom data layers
  (https://docs.godotengine.org/en/stable/tutorials/2d/using_tilesets.html).
- **Unreal's Data Validation** runs rules over assets on save and in CI
  (https://dev.epicgames.com/documentation/en-us/unreal-engine/data-validation-in-unreal-engine).

What Shuyi has today (read on 2026-10-21):

- The grid kit's own `.map` format already does this for its features.
  `MapFeature` (`crates/assets/src/grid_map.rs` line 61) is the game's
  own serde type, written after a tile's `(x, y)`, with a `problem()`
  hook, and `grid_map::parse` (line 296) reports every problem with its
  line and column.
- ADR-0004 §2: Tiled and LDtk "names and custom properties become ids".
  They stay strings. ADR-0004 puts the editors' fixture files in
  `crates/assets/tests/fixtures/`.
- 1124 (parked) imports both editors into one `TileMap`, whose object
  layers are "named boxes and points with properties". Nothing maps
  them to a game's types, nothing checks them, and nothing tells the
  editors what a game's types are.
- `GameTools::check` (`crates/cli/src/game.rs` line 103) returns the
  game's own `ContentError`s, and `shuyi check` lists them with file and
  line. `Formats::load_each` (`crates/assets/src/format.rs` line 730)
  loads every file of a kind directly inside one folder, not below it.
  `Bundle::files_under` (`crates/assets/src/bundle.rs` line 270),
  `Formats::claims` (`format.rs` line 586) and `Formats::load` (line
  678, which takes a full path too) read a tree file by file.
  `Bundle::file` (line 214) gives a file's text, and `error::line_of`
  (`crates/assets/src/error.rs` line 69) finds a line in it.
- `shuyi_core::grid::Pos` (`crates/core/src/grid.rs` line 16) is a tile
  position of two `i32`s.
- `schemars` 1 (MIT) is a dependency of `shuyi-cli`
  (`crates/cli/Cargo.toml` line 44), for the contract's schemas.
- `contract::Op` (`crates/cli/src/contract.rs` line 57) is an
  exhaustive enum of the contract's operations.

What is planned:

- 1124, with this sweep's edit, keeps each object's id, class, place
  and properties with their editor types, in `Fx`, and loads worlds of
  several levels (LDtk's levels and Tiled's `.world`). This ticket reads
  what 1124 keeps.
- 2221 instances ECS scenes with `SceneRegistry` and `MapEntities`. Its
  Out, with this sweep's edit, leaves Tiled and LDtk map objects to
  this ticket.
- 2515 adds a `schema` feature to `shuyi-assets` and moves `schemars`
  into the workspace. 2516 checks `Ref<K>` references between assets in
  `shuyi check`. 2509 gives problems codes. 2013 says which public
  types are `#[non_exhaustive]`, and 2010 asks for a `Breaking:` line
  when a PR breaks the API.
- 1127's platformer example draws two levels in Tiled or LDtk, with
  coins and a door. It is this ticket's first user, and with this
  sweep's edit it waits for it.

Why it matters:

- **The game first.** Visions of Shuyi draws its maps in the grid kit's
  `.map` format and doesn't need this. It would if the army of its
  ticket 0060 walks on maps drawn in Tiled or LDtk.
- **Other 2D games.** Without types, every game parses string
  properties by hand ("class `Door`, property `to`"). A misspelled class
  or property fails when someone walks there, or never. With them, the
  game's loader gets its own enum, `shuyi check` names the file, the
  layer and the object, and the editor offers the right fields.
- **The end goal** (Nick, 2026-10-19: a game "like snake or mario" in
  under an hour). An agent writing a Mario-like gets typed levels
  instead of writing glue, and `shuyi check --json` tells it which
  object is wrong. Under the 2026-10-20 rule this waits for the launch
  with the rest of real time.

## Nick input

None.

## Scope

**In:**
- **An ADR** (the `write-adr` skill) extending ADR-0004 §2 and
  ADR-0019 §5:
  - a map object is read into the game's own serde type after the map
    loads (a check after loading, ADR-0019 §5), never inside a format,
    so it works on the `TileMap` kind whatever file it came from
    (`AGENTS.md` rule 10);
  - the game's type is a plain enum, and the object's class picks its
    variant;
  - no float reaches a game's type: a number with a fraction fills an
    `Fx` or is an error (rule 5);
  - a reference to another object is the editor's id, checked inside
    its file (one Tiled map, or every level of one LDtk file);
  - the editors' type files come from the game's schema (`schemars`).
    Tiled's are written. LDtk's are checked, not written: a designer
    edits them inside the `.ldtk` file itself, and rewriting that file
    would fight the editor.
- **`shuyi_assets::tilemap::things`** (`crates/assets/src/tilemap/things.rs`,
  beside 1124's `TileMap`), `no_std` with `alloc`:
  - `read::<T: DeserializeOwned>(map: &TileMap, file: &AssetPath,
    text: Option<&str>, layers: ObjectLayers) -> Result<Things<T>,
    Vec<ContentError>>`. `ObjectLayers::All` reads every object layer
    (and every LDtk entity layer); `ObjectLayers::Named(&[&str])` reads
    those. `text` is the map file's text, for lines.
  - `Thing<T> { id: String, name: String, level: Option<String>, layer:
    String, place, data: T }`. `id` is the editor's (Tiled's object id
    as text, LDtk's `iid`), never a position in the file (ADR-0004 §2).
    `level` is the LDtk level's identifier, `None` for a Tiled map.
    `place` is the object's point or box as 1124 keeps it, in `Fx`
    pixels.
  - `Things<T>`: the things in file order (level, then layer, then
    object), with `iter()`, `get(&ObjectRef)`, `index_of(&ObjectRef) ->
    Option<usize>` (so a game maps a reference to its own arena id after
    spawning), and `unclassed() -> &[String]`: the ids of objects with
    no class. Those are not things: they are shapes a game uses for
    other jobs, like 2362's rooms.
  - **How `T` reads an object.** `T` is a plain enum, externally tagged
    as serde's derive makes it by default. A small `serde::Deserializer`
    over 1124's property values (`things/de.rs`, built on
    `serde::de::value`) offers each object as an enum
    (`deserialize_enum` and `EnumAccess`): the variant is the class, and
    a struct variant's fields are the object's custom properties, read
    through this same deserializer. Not `#[serde(tag = "class")]`:
    serde reads an internally tagged enum through its own buffer
    (`deserialize_any`), so the `Fx` and float rules below would never
    run. A unit variant is a class with no fields; its properties are
    ignored, as a struct variant ignores unknown fields without
    `#[serde(deny_unknown_fields)]`. Each property gives:
    - `bool`, `int` and `string` as themselves; an `enum` property's
      value as a string, which fills a unit enum; a Tiled class-typed
      property as a map, which fills a struct; LDtk's arrays as a
      sequence, and its points (grid cells) as `{ x, y }`, which fills
      `shuyi_core::grid::Pos`; `colour` as a string;
    - `file` as the bundle path it names: Tiled and LDtk write it
      relative to the map file, so it is joined to the map's folder
      (from `file`). A path that leaves the bundle's root is an error
      naming the field;
    - **`Fx`** (1119): when a field asks for the newtype `Fx` (serde
      calls `deserialize_newtype_struct("Fx", ..)`), a `float` gives its
      `Fx`, and an `int` gives that many whole units, never raw bits. If
      1119's `Fx` doesn't deserialize through that call, change its
      `Deserialize` so it does, keeping its RON form and its hash
      fixtures;
    - **no floats:** `deserialize_f32` and `deserialize_f64` are an
      error naming the field (`speed is a float field; use Fx`), and a
      `float` property in any field but an `Fx` is an error, including
      through `deserialize_any` (an untagged enum or a
      `#[serde(flatten)]` field). The ADR says a things enum avoids
      both;
    - **`ObjectRef`** (`things::ObjectRef(String)`, a newtype, read the
      same way by its name): a Tiled `object` property or an LDtk
      `EntityRef` fills it. Tiled's 0 means none: it fills an
      `Option<ObjectRef>` with `None` and is an error for a plain
      `ObjectRef`;
    - a property left out fills the field's `#[serde(default)]`, or is
      an error naming the field. Tiled leaves out a class member the
      designer didn't change, so the export below writes each field's
      default as the member's value.
  - **Errors** list every object's problems, not only the first. For
    LDtk, the level comes before the layer:

    ```text
    levels/one.tmx:41: layer "things", object 12 "door_a" (Door): missing field `to`
    ```

    serde's message for an unknown variant already lists the classes
    `T` has. The line is the first line holding `<object id="12"` (TMX)
    or the entity's own `"iid"` key with its value (LDtk, so an
    `EntityRef` to it elsewhere isn't found first), through
    `error::line_of`; there is none when `text` is `None`.
  - **References.** After every thing is read, each `ObjectRef` read
    must name an object of the same map (Tiled) or of any level of the
    same file (LDtk). A dangling one is an error naming the object, the
    field and the id.
  - `check_under::<T>(formats: &Formats, bundle: &Bundle, dir: &str,
    layers: ObjectLayers) -> Vec<ContentError>`: every file in `dir`
    and the folders below it (`Bundle::files_under`) that a `TileMap`
    format claims (`Formats::claims::<TileMap>`), each loaded alone
    (`Formats::load` with its path), so a broken map leaves the others
    read, then read into things with its text (`Bundle::file`) for
    lines. A game's `GameTools::check` calls it, so `shuyi check` (and
    `--json`) lists what is wrong in every level, including those the
    game loads only when a door opens.
  - Hot reload needs nothing here: a game reads things again from a
    reloaded `Hot<TileMap>` (2220).
- **`Fx`, `Pos` and `ObjectRef` describe themselves**, for the export.
  Each `JsonSchema` impl is written by hand and inlined
  (`inline_schema` is true), so a field's own schema carries the
  `format` the export looks for:
  - `shuyi-core` gets a `schema` feature (`["std", "dep:schemars"]`),
    off by default, so the `no_std` build never sees it. `Fx` gives
    `{ "type": "number", "format": "fx" }`; `Pos` gives an object of two
    integers `x` and `y` with `"format": "grid-pos"`.
  - `ObjectRef` gives `{ "type": "string", "format": "object-ref" }`,
    behind `shuyi-assets`' `schema` feature, which also turns on
    `shuyi-core/schema`, so a game's `tools` feature turns on one
    feature. **Either order with 2515:** if 2515 is done, use its
    feature and its workspace `schemars`. If not, add both as 2515
    describes them (`schema = ["std", "dep:schemars"]`, and `schemars`
    in `[workspace.dependencies]` with `shuyi-cli`'s settings), and
    2515 builds on them.
- **`shuyi map-types`**, a contract operation (ADR-0022), in
  `crates/cli/src/contract/map_types.rs`:
  - **The game gives its type:** `GameTools::map_types(&self) ->
    Result<schemars::Schema, String>`, by default
    `Err(unsupported("map-types", "map_types"))`. A game writes
    `Ok(schema_for!(Thing))`, its `Thing` deriving `JsonSchema` under
    its `tools` feature.
  - **From the schema to editor types:** `EditorType::Class { name,
    members, use_as }`, `EditorType::Enum { name, values }`, `Member {
    name, kind, optional, default }`.
    - The top schema's `oneOf` (or `anyOf`) entries are its variants.
      A string `const` or `enum` entry gives classes with no members.
      An object entry with one required property gives a class named
      by that property, whose members come from that property's object
      schema (inline, or by `$ref` for a newtype variant). A top schema
      that is only a string `enum` gives classes with no members.
    - A field's `integer` is an int, `boolean` a bool, `string` a
      string, `format: "fx"` a float, and `format: "object-ref"` an
      object. A `$ref` to a unit enum is an enum type; a `$ref` to a
      struct is a class used as a property. An `Option` is the same
      kind, optional.
    - A member's `default` is the schema's default, or else zero,
      false, the empty string or the first value of the enum.
    - Any other shape (a list, a map, a tuple, a plain float) is left
      out of both editors, with a limit naming the class and the
      member. A `Pos` (`format: "grid-pos"`) is left out of Tiled, which
      has no point property, with a limit, and is checked against LDtk.
  - **`--tiled <file.tiled-project>`:** the types are merged into the
    project file's `propertyTypes`
    (https://doc.mapeditor.org/en/stable/manual/custom-properties/). An
    entry whose name the game declares is replaced and keeps its `id`.
    A new one gets the next id after the largest. Every other entry and
    every other key of the file is kept. Enums are written with
    `storageType: "string"`. Tiled reads a project's types when it opens
    the project, so the designer imports nothing.
  - **`--ldtk <file.ldtk>`** (it may be given more than once): the
    file's `defs.entities` and `defs.enums` are read with a private
    serde struct of only those fields (https://ldtk.io/json/), checked
    against the types, and never written. Each of these is a problem:
    - a class with no entity of its name;
    - a member with no field of its name;
    - a field whose `__type` doesn't match: `Int`, `Float` for `Fx`,
      `Bool`, `String`, `Multilines`, `Color` or `FilePath` for a
      string, `EntityRef` for `ObjectRef`, `Point` for `Pos`, and
      `LocalEnum.<Name>` or `ExternEnum.<Name>` for an enum, whose
      values must hold every variant;
    - a field that can be null (`canBeNull`) for a member that isn't an
      `Option`.

    An entity or a field the types don't have is a limit, not a
    problem.
  - **`--check`:** nothing is written, and the answer fails if the
    Tiled project would change (for CI).
  - `MapTypesInput { tiled: Option<String>, ldtk: Vec<String>, check:
    bool }`; `MapTypesOutput { types: Vec<EditorType>, tiled:
    Option<TiledReport>, ldtk: Vec<LdtkReport> }`, with `TiledReport {
    file, added, replaced, kept }` and `LdtkReport { file, problems:
    Vec<Problem> }`. `ok` when there is no problem and, with `--check`,
    nothing to write. Limits: the members left out; "LDtk's definitions
    are checked, not written"; "the project file's keys are written in
    sorted order; Tiled reads it the same".
  - **The contract:** `Op::MapTypes` in `crates/cli/src/contract.rs`
    (`Op::ALL`, `name`, the schema pair in `schemas()`),
    `map-types.input.json` and `map-types.output.json` under
    `crates/cli/schemas/`, `contract::VERSION` raised by one from
    whatever it is when this lands, and its line in `versions.json`
    (`docs/contract.md`, *The schemas*). The usage text in
    `crates/cli/src/lib.rs`, and the operations table and the
    `GameTools` table in `docs/contract.md`, get a row. The fake game
    (`crates/cli/src/contract/fake.rs`) offers a type.
- **Fixtures** in `crates/assets/tests/fixtures/` (where ADR-0004 puts
  the editors' files; 1124 makes the folder), each the size of a
  platformer level and made in its editor, as 1124's are:
  - one from Tiled (a `.tmx` and its `.tiled-project`) and one from
    LDtk (two levels);
  - things of four generic classes (a coin, a door, a spring, a moving
    platform) that use every property kind above, a reference between
    two of them, and a shape with no class;
  - broken copies: a misspelled class, a missing field, a float in an
    int field, a dangling reference;
  - a Tiled project whose types were made by hand in Tiled's Custom
    Types Editor, to compare the export with.
- **Docs:** *Checks after loading* in `docs/formats.md` gets
  `things::read` and `check_under`. A new section of `docs/formats.md`,
  "Levels from Tiled and LDtk", shows a `Thing` enum, the loader, the
  line in `check`, and `map-types`. **From 2802 (done):** the book's
  chapter 5 (`site/content/learn/book/assets-and-formats.md`) links to
  that section.
- **Problem codes:** the new errors and problems get codes from 2509's
  table.
- **Either order with 2516:** if 2516 is done, `read` marks the map's
  file for its reference collection, as `format::decode_with` does, so
  a `file` property read into a `Ref<K>` is checked by `shuyi check`
  under the map's file. If not, write a ticket for it, blocked by 2516
  (`write-ticket`): 2516's Out leaves it to this ticket.
- **Either order with 2907:** if 2907 is done and its ADR picked
  another description of a game's types than `schemars`, follow it for
  the export and say so in this ticket's ADR. If not, use `schemars`,
  and 2907's ADR says whether `map-types` moves onto its choice (its
  clause on this ticket).
- **From 1134 (done):** the skill that makes a new game
  (`skills/shuyi-new-game/SKILL.md`) says to run `map-types` after
  changing the things enum; `check-skills` passes.
- New public types follow 2013's ADR (in the MVP): `MapTypesInput` is
  an input with a `Default`, and `EditorType` may grow, so it is
  `#[non_exhaustive]`.
- **Size, decided before `map-types` is written.** After the reading,
  the check, their fixtures and their docs (step 5), count the changed
  lines, not counting fixtures, snapshots and schema files. If they are
  over about 350, the two `schema` features and `shuyi map-types`, with
  their docs, acceptance criteria and tests, go to a follow-up ticket
  (the `write-ticket` skill), parked like this one with `blocked_by:
  ["2232"]`, written in this PR from this ticket's bullets. This PR
  then keeps the reading and the check, and the lines of 1127 and 2507
  that name `map-types` point at the follow-up, which also takes the
  `shuyi-new-game` line from this ticket.
  The follow-up names 1127 back: if 1127 is done first, its platformer
  has Tiled types written by hand or no LDtk check, so the follow-up
  writes those types with `shuyi map-types --tiled`, or adds CI's
  `shuyi map-types --ldtk` on its LDtk levels; if not, nothing to do
  (1127 does it).

API: a new module, two new features, a new `GameTools` method with a
default (2013's rule for traits), and a new operation. `Op` gains a
variant: if `Op` is still exhaustive when this lands, it becomes
`#[non_exhaustive]` here (2013's rule for enums that grow), and the PR
has a `Breaking:` line (2010) saying no game matches on `Op`, so the
game's next pin bump needs no change. If `Op` is already
`#[non_exhaustive]`, nothing breaks.

**Out (do not do):**
- what 1124 keeps of an object, and worlds of several levels (1124,
  with this sweep's edit). If 1124 landed keeping a place or a fraction
  in another form, convert it here with 1124's rule, as its importer
  does (the reader's float, its exact value rounded with integer
  operations on its bits, never float arithmetic, to the nearest
  1/256, a tie toward negative infinity) and say so in the completion
  notes;
- spawning things into an ECS world through 2221's `SceneRegistry` and
  `MapEntities`: a follow-up when a game on the ECS asks. Things stay
  plain values here, so turn games with plain structs use them too;
- writing LDtk's definitions into a `.ldtk` file;
- editing levels inside Shuyi (2506, 2507), Tiled's scripts and
  extensions, and exporting autotile rules;
- typed custom data on tiles (2709 reads one integer tile property; a
  follow-up if a game asks);
- polylines and polygons as curves (2122). Polygons as obstacles for
  paths aren't 2106's either: a follow-up after 2106 and 1124 if a
  game asks;
- Tiled's colour and file pickers for members (they are exported as
  strings);
- drawing things (2318 outlines objects; 2360 draws tile layers).

## Implementation steps

1. Read 1124's `TileMap` (objects, places, properties, levels), 1119's
   `Fx` and its serde, ADR-0004 §2, ADR-0019 §5, ADR-0022, 2013's ADR,
   `grid_map.rs` (`MapFeature`), `format.rs` (`claims`, `load`,
   `load_each`), `bundle.rs` (`files_under`, `file`), `game.rs`,
   `contract.rs` and `contract/fake.rs`. If 2515, 2516 or 2907 are
   done, read them.
2. Write the ADR.
3. Write `things::de`, the deserializer, with a unit test per property
   kind, and one showing that a float through `deserialize_any` is an
   error.
4. Write `read`, `Things`, the references, the lines and
   `check_under`, and add the fixtures.
5. Write the docs for the reading and the check (*Checks after
   loading*, and the new section without `map-types`). Count the lines
   as *Size* says. If they are over, write the follow-up ticket, point
   1127 and 2507 at it, and go to step 9.
6. Add the `schema` features and the three `JsonSchema` impls.
7. Write `map_types`: the conversion, the Tiled merge, the LDtk check,
   the operation and its wiring (`Op` made `#[non_exhaustive]` if it
   isn't). Regenerate the schemas and add the version line.
8. Add `map-types` to the docs and to `docs/contract.md`.
9. Run the gates.

## Acceptance criteria

- [ ] The Tiled and LDtk fixtures read into the test's `Thing` enum, and
      a text dump of their things matches a committed snapshot.
- [ ] Each broken fixture fails with an error naming the file, the line,
      the layer (and the level, for LDtk), the object's id and the
      field.
- [ ] An `f32` field, and a float property in an `i32` field or in an
      untagged enum field, are each an error. An `int` property in an
      `Fx` field gives whole units.
- [ ] A `file` property gives the bundle path joined from the map's
      folder, and one that leaves the root is an error.
- [ ] A dangling `ObjectRef` is an error, and `index_of` maps every
      resolved reference to its thing.
- [ ] On a fake game whose `check` calls `check_under` over the broken
      fixtures (one of them in a folder below `dir`), `shuyi check
      --json` lists every error with its file and line.
- [ ] `shuyi-core` and `shuyi-assets` still build for
      `thumbv7em-none-eabihf` without default features.
- [ ] `cargo deny check` passes, with no new dependency.
- [ ] Either `map-types` is in this PR and the next five criteria hold,
      or the follow-up ticket is in `tickets/open/`, parked, with
      `blocked_by: ["2232"]` and those five criteria, and 1127 and 2507
      name it.
- [ ] `map-types --tiled` writes the test type's classes and enums into
      a copy of the fixture's project (snapshot), keeping its other
      entries and keys. `--check` then passes, and on the old copy it
      fails.
- [ ] The written `propertyTypes` equal, field for field (leaving out
      `id` and `color`), those of the fixture project made by hand in
      Tiled.
- [ ] `map-types --ldtk` on a fixture with a mismatched field names the
      entity, the field and both types.
- [ ] The contract's schema test passes with `VERSION` raised and the
      new schemas checked in.
- [ ] If `Op` became `#[non_exhaustive]` here, the PR body has a
      `Breaking:` line.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: each property kind in each field type it fills, and in each it
  refuses; `Fx` from an int and from a float, at a tie; `Option`s and
  defaults; an unknown class; an unknown property (with
  `deny_unknown_fields`); a unit variant; a float through
  `deserialize_any`; `file` paths joined and refused; references;
  objects with no class; lines (a TMX object id, an LDtk `iid` with an
  `EntityRef` to it above); `check_under` over nested folders.
- Unit, with `map-types`: the schema to editor types for each shape
  (unit, struct and newtype variants), and the shapes left out; the
  Tiled merge (ids kept, new ids, other entries kept); each LDtk check.
- Property: random things of a test enum, turned into property values
  by a test helper, read back equal; `read` twice gives equal things.
- Snapshot (insta): the fixtures' things; with `map-types`, the written
  `propertyTypes` and the `map-types` answers.
- Fuzz-style: random property values never panic `read` (errors only).
- Integration: `check` on the fake game; `map-types` on it if it stayed
  in this PR.

## Completion notes

*(Filled in by the session that completes the ticket.)*
