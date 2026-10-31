---
id: "2515"
title: "shuyi kinds: every kind a game's tools register, with its formats, their extensions and a JSON schema of each format's text, as a contract op"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2515 — shuyi kinds: every kind with its formats and a schema

## Context

Found by the 2026-10-20 comparison with Bevy 0.20, Godot 4.7, Unity 6
and Unreal 5.8. Bevy's remote protocol answers `registry.schema` (a JSON
schema of each registered type) and `rpc.discover`
(https://docs.rs/bevy_remote/latest/bevy_remote/), and `bevy_brp_mcp`
hands them to agents. Unreal's Data Tables are typed by a struct its
editor shows. An agent there learns a file's shape without reading the
engine's source.

What Shuyi has today:

- `shuyi schema` prints only the contract's own input and answer
  schemas (`contract::schemas()`, `crates/cli/src/contract.rs` line
  263; the files under `crates/cli/schemas/`).
- `Importers::kinds()` (`crates/cli/src/contract/import.rs` line 236)
  returns the kinds' names, for `import`'s messages. No operation lists
  the kinds with their formats, extensions or shapes.
- `Formats` keeps a private `Kind` trait with a kind's name and its
  formats' ids (`crates/assets/src/format.rs` line 370). Nothing public
  lists them: `Formats::ids::<K>()` needs the kind's type.
- **A kind is not its file.** Several standard formats read a private
  raw type and build the kind from it:
  - `PaletteDef::from_source` reads a map of names to `"#rrggbb"`
    strings (`crates/assets/src/palette.rs` line 82), and `PaletteDef`
    holds RGB numbers;
  - the lights file is a map of `RawLight` (`lighting.rs` line 166);
  - the audio manifest is `RawFile` and the raw types under it
    (`audio.rs` line 135);
  - the credits file is its own `RawFile` (`credits.rs` line 192);
  - the names file is a map of strings (`names.rs` line 252).

  So a schema derived on the kinds would describe the wrong thing. The
  shape an agent needs is the shape of a **format's text**: the type
  serde reads.
- `schemars` 1 (MIT) is already a dependency of `shuyi-cli`
  (`crates/cli/Cargo.toml` line 44), for the contract's schemas.

What is planned: 2506 and 2507 (parked) already take a settings file's
shape from `shuyi kinds`, and 2507 is blocked by this ticket. 1133
makes each contract operation an MCP tool. 1134 wrote the skills
(`skills/`).
2907 researches one description of a game's types for scripts,
scenes, schemas and agents.
2230 adds JSON and CSV tables; 2516 adds typed references between
assets. 2116 and 2020 walk the format registry too, and 2509 gives the
contract's errors codes.

Why it matters:

- **The game first.** Visions of Shuyi's agents write its RON tables and
  sidecars. With a schema and an example per kind they get the shape
  right the first time, without opening the engine's Rust.
- **Other 2D games.** Every game made with the engine has a palette, a
  font atlas, credits and target profiles, plus tables of its own.
- **The end goal** (2026-10-19): the most AI-friendly 2D engine in Rust.
  With no editor (2506 and 2507 are parked), this is how an agent sees a
  file's fields.

## Nick input

None.

## Scope

**In:**
- **An ADR** (the `write-adr` skill), extending ADR-0019 and ADR-0022,
  that decides where a schema comes from:
  - a schema belongs to a **format**, not to a kind, because the text a
    format reads is often not the kind (the palette above);
  - it is derived with `schemars` on the type serde reads, behind a
    `schema` feature of `shuyi-assets`;
  - RON's structs `(a: 1)`, tuples, enum variants `Name(x)` and
    `implicit_some` map to JSON Schema only approximately, so every
    described format gives a small example file beside its schema;
  - `shuyi kinds` is the one place agents, MCP (1133), the skills (1134)
    and a later inspector (2507) read shapes from.

  **Either order with 2907:** if 2907 is done and its ADR picked another
  source for schemas, follow it and say so in this ADR. If not, this ADR
  decides, and 2907 starts from it.
- **`Formats::kinds()`** in `crates/assets/src/format.rs`:
  `Vec<KindFormats>`, every kind with a format in the order it was first
  registered. `KindFormats { name: &'static str, formats:
  Vec<FormatInfo> }`, `FormatInfo { id: &'static str, extensions:
  &'static [&'static str] }`. The private `Kind` trait gains the method
  that lists them. No `std`, no new dependency.
  - **From 2116 (done):** its guard `missing_rows` in
    `crates/assets/src/format/golden_tests.rs` checks that every id
    `Formats::standard().ids::<K>()` lists for a fixed list of covered
    kinds has a row `<kind>: <format id>`. Move it onto
    `Formats::standard().kinds()` and drop the list, so it fails on any
    standard format with no row; its test
    `the_golden_guard_fails_on_a_format_of_a_covered_kind_with_no_row`
    keeps passing.
  - **Either order with 2020:** both add a method to the private `Kind`
    trait. If 2020 is done, put this one beside its fuzz method. If not,
    2020 puts its method beside this one.
- **`shuyi_assets::format::TextSchema`**, behind a new feature
  `schema = ["std", "dep:schemars"]`:

  ```rust
  pub trait TextSchema: Format {
      /// The JSON Schema of the file's text, as serde reads it.
      fn text_schema(&self) -> schemars::Schema;
      /// A small file of this format that decodes, for people and agents.
      fn example(&self) -> Option<&'static str> {
          None
      }
  }
  ```

  Implemented by:
  - `ron::Ron<T>` for `T: JsonSchema` (the file is `T` itself), with
    `Ron::with_example(text)` so a game can give one. The example field
    and `with_example` are behind the `schema` feature, so the `no_std`
    build has no field it never reads.
  - the standard RON formats `PaletteRon`, `NamesRon`, `LightingRon`,
    `AudioManifestRon`, `CreditsRon`, `FontAtlasRon` and `TargetRon`
    (and, if parked 2408 is done, its `RumbleRon` with
    `kinds/rumble.ron`, as 2408's *either order with 2515* says),
    each with the schema of the type its parser reads:
    - a palette and a names table read `BTreeMap<String, String>`, which
      has no type of ours to derive on: their schema is
      `schemars::schema_for!(BTreeMap<String, String>)` with a
      description;
    - the others read a type that gets
      `#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]`,
      with every type under it: `RawLight`; the audio manifest's
      `RawFile`, `RawSound`, `RawMusic`, `RawPool`, `RawCredit` and
      `CreditRef`; the credits file's `RawFile`, `FileCredit` and
      `CreditGroup`; `FontAtlasDef` (`font.rs` line 127 reads it
      itself); `TargetProfile` and the types under it (`target.rs`;
      read with `implicit_some`, which the ADR's note covers);
    - `keymap::Button` has its own `Deserialize`, a string read by its
      name (`keymap.rs` line 380), so its `JsonSchema` is written by
      hand: a string, one of `Button::ALL`'s names.

    Where a string has a rule serde doesn't see (a palette colour is
    `#rrggbb`), the schema's description says it.
- **An example per described engine format**, in
  `crates/assets/kinds/<format id>.ron` (`palette.ron`, `names.ron`,
  `lights.ron`, `audio-manifest.ron`, `credits.ron`, `font-atlas.ron`,
  `target-ron.ron`), included with `include_str!`. A test decodes each
  through its format. An example names no other file (an empty sound
  list), or the test gives it small fixtures in a bundle (the font
  atlas's picture). A target profile's id must be its file's name, so
  the test decodes that example at the path its `id` names.
  `shuyi-assets`' dev-dependency on itself (`crates/assets/Cargo.toml`)
  turns on `schema` beside `png`, so its tests always build these.
- **`schemars` moves to `[workspace.dependencies]`** in the root
  `Cargo.toml`, with the settings `shuyi-cli` uses today
  (`default-features = false, features = ["derive", "std"]`), so both
  crates share one version. `shuyi-cli` turns on `shuyi-assets/schema`.
  It re-exports `schemars` as it re-exports `serde_json`
  (`crates/cli/src/lib.rs` line 39), so a game's tools can derive
  through `shuyi_cli::schemars`.
  - **If 2232 (parked) was unparked and done first,** the `schema`
    feature and the workspace `schemars` exist already: build on them.
- **`Importers` carry the schemas** (`crates/cli/src/contract/import.rs`):
  `Importers::described<F: TextSchema>(self, format: &F) -> Self`
  keeps the schema and the example of `(kind name, format id)`.
  `Importers::standard()` (line 170) describes the standard RON formats,
  and adds the target profile kind (`target::TargetProfile`, which
  `Formats::standard` has and `Importers::standard` lacks) with a
  summary.
- **The `kinds` op**, in `crates/cli/src/contract/kinds.rs`:
  - `shuyi kinds [<kind>] [--json]`. Without a kind, every kind of the
    game's registry with its formats' ids and extensions, and whether
    each format has a schema. With a kind (`shuyi kinds palette`, or
    `shuyi kinds "font atlas"` for a name with a space), that kind
    alone, each format with its schema and its example.
  - `KindsInput { kind: Option<String> }`; `KindsOutput { kinds:
    Vec<KindInfo> }`; `KindInfo { kind: String, formats:
    Vec<FormatEntry> }`; `FormatEntry { id: String, extensions:
    Vec<String>, described: bool, schema: Option<serde_json::Value>,
    example: Option<String> }`. `described` is always set; `schema` and
    `example` are filled only when a kind is asked for.
  - The kinds come from `importers.formats.kinds()`, so a kind the game
    registered but gave no importer entry is listed too, with no
    schema.
  - **Limits:** the formats with no schema, by kind and id ("map: no
    schema: its text isn't serde's"); and, when an answer holds a RON
    format's schema, one line on how RON writes what the schema
    describes, pointing at the example.
  - An unknown kind answers an error listing the kinds, as `import`
    does (`import.rs` line 300).
  - It runs where `import` runs: in the game's tool program, over
    `GameTools::importers()` (`crates/cli/src/game.rs` line 131). With no
    project, or a project with no game program, it lists the engine's
    standard kinds, with the limit `import` gives there
    (`crates/cli/src/lib.rs` lines 95 and 137). No new `GameTools`
    method: `kinds` and `import` read the same `Importers`.
  - The wiring: `runs_the_game` in `crates/cli/src/contract/cli.rs`
    (line 168) names `kinds`, and `with_game` gets a `"kinds"` arm
    before its last arm, which sends every other name to `import`. A
    `kinds` function beside `import` there parses and prints, and the
    two no-game branches in `lib.rs` take `kinds` as they take `import`.
  - The contract: `Op::Kinds` in `crates/cli/src/contract.rs`
    (`Op::ALL`, `name`, the schema pair in `schemas()`),
    `kinds.input.json` and `kinds.output.json` under
    `crates/cli/schemas/`, `contract::VERSION` raised by one from
    whatever it is when this lands, and its line in `versions.json`
    (`docs/contract.md`, *The schemas*).
  - The usage text in `crates/cli/src/lib.rs` (beside `import`, line
    56) gets a `kinds` line. `docs/contract.md` gets a `kinds` row in
    *The operations*, and its *What a game gives* table's `importers`
    row names `kinds` beside `import`.
- **Hello, Grid describes its own kind:** its `importers()`
  (`examples/hello-grid/src/tools.rs` line 168) registers its `English`
  kind (`Ron<English>`, `examples/hello-grid/src/lib.rs` line 92), which
  it lacks today, and describes it. `English` derives `JsonSchema` under
  the `tools` feature, through `shuyi_cli::schemars`, with
  `#[schemars(crate = "shuyi_cli::schemars")]`. `English` is a map of
  strings, as a palette's text is, so `import files/palette.ron` reads
  it as `english` too, after `palette`; the import test in
  `examples/hello-grid/tests/it/contract.rs` checks `read_as[0]`, so
  check it still passes.
- **2507's text** (parked; only its text changes): its schema line
  already takes a file's shape from `shuyi kinds` ("2515 and its ADR:
  `schemars` behind `shuyi-assets`' `schema` feature"). Put this ADR's
  number in it, and check the line agrees with this ADR (2907 may have
  changed it).
- **Either order with 2230:** if 2230 is done, `Json<T>` implements
  `TextSchema` for `T: JsonSchema` (exact for a JSON file), and
  `Csv<T, R>` for `R: JsonSchema` (the schema of one row; the
  description says the file is a header row then one row per line),
  each when both the `schema` feature and its own are on. If not, 2230
  does it.
- **Either order with 2516:** if 2516 is done, `Ref<K>` implements
  `JsonSchema` behind the `schema` feature: a string whose description
  names kind `K`. If not, the ADR notes `Ref<K>` for 2516, which adds it.
- **From 1134 (done):** add a line to the skills that write data files
  (`skills/shuyi-new-game/SKILL.md`, `skills/shuyi-import-art/SKILL.md`
  and `skills/shuyi-ship/SKILL.md`): read the file's shape with `shuyi
  kinds <kind> --json` before writing it. `check-skills` fails a skill
  that names a command `shuyi` lacks, so the line lands with `kinds`.
- **MCP:** `shuyi mcp` (1133, done) makes its tools from `Op::ALL`; a new operation needs its arm in `mcp::host::one_off` and in `mcp::description`, which the compiler asks for, and is in `contract::cli::runs_the_game` if it runs the game (so it takes `project` and runs in the game's tool program). `kinds` runs the game's importers, so it runs
  the game.
- **Problem codes (2509):** the unknown kind takes `SH0001`
  (`wrong-arguments`), as `import --kind` with no such kind does.
- **Public types (ADR-0039):** the new public types (`KindFormats`,
  `FormatInfo`, the op's input and output) follow it.
- `docs/formats.md`: a section "A format's schema", on `TextSchema`, the
  examples and `shuyi kinds`.

**The public API.** The rest is new methods, new types and a new
feature. `Op` gains a variant and `Op::ALL` a length. If 2010's check
reports that as a break (it does: `Op` is still exhaustive), the PR has
its `Breaking:` line, saying the game
builds unchanged at its next pin bump.

**Size.** If the PR passes about 600 changed lines (the ADR, the docs,
the generated schemas, the examples and the snapshots not counted),
`TargetRon`'s schema (its derives and `Button`'s hand-written one) goes
to a follow-up ticket (`write-ticket`), After MVP, and the PR says so.

**Out (do not do):**
- schemas for `.dlg`, `.map`, PNG, WAV, OGG and one-bit pictures (their
  text or bytes aren't serde's), and for keymaps, cinematics and
  language packs (comments, directives and several files a schema
  doesn't cover): a follow-up when a game asks;
- checking files against schemas in `shuyi check` (the formats already
  check them; schemas are for writing them);
- the inspector (2507) and a reflection system (2907's research);
- references between assets (2516), past the either-order line above;
- the sections of import settings files (2229 leaves them out too), and
  a curve file's schema (2122, parked, adds it when it lands).

## Implementation steps

1. Read ADR-0019, ADR-0022, `crates/assets/src/format.rs`,
   `crates/assets/src/ron.rs`, `crates/cli/src/contract/import.rs`,
   `contract.rs` and `contract/cli.rs`, and the standard RON formats'
   parsers named above. Read 2907's ADR if it is done, and check which
   of 2020, 2230 and 2516 are done.
2. Write the ADR.
3. Add `Formats::kinds()` with its tests. Move 2116's guard onto it;
   do the 2020 part if 2020 is done.
4. Move `schemars` to the workspace. Add the `schema` feature,
   `TextSchema`, `Ron::with_example`, the derives on the raw types,
   `Button`'s schema by hand, the examples and their tests.
5. Add `Importers::described`, describe the standard formats, and add
   the target profile kind to `Importers::standard()`.
6. Add the `kinds` op, its wiring in `contract/cli.rs` and `lib.rs`,
   its arms in `mcp::host::one_off` and `mcp::description`, `SH0001`
   for an unknown kind, the schemas and the version line.
7. Describe Hello, Grid's `English`; update `docs/formats.md`,
   `docs/contract.md`, the usage text and 2507's line.
8. Do the either-order parts whose tickets are done (2230, 2516), and
   add the `shuyi kinds` line to the three skills (from 1134).
9. Run the gates.

## Acceptance criteria

- [ ] `cargo shuyi kinds` in `examples/hello-grid` lists every kind of
      its registry (the standard ones, `target profile`, `map`,
      `keymap` and `english`) with each format's id and extensions.
- [ ] With `--json` there, `described` is true for the seven standard
      RON formats and `english`, and false for every other format.
- [ ] `cargo shuyi kinds palette --json` there answers the palette
      format's schema (a map of strings) and its example, and the
      example decodes with `PaletteRon` (a test).
- [ ] `cargo shuyi kinds english --json` there answers `English`'s
      schema.
- [ ] `cargo shuyi kinds nope` exits non-zero with an error that lists
      the kinds.
- [ ] A format with no schema is named in the answer's limits.
- [ ] Outside a project, `shuyi kinds` lists the engine's standard
      kinds, with the limit `import` gives.
- [ ] The contract's schema test passes with `VERSION` raised and
      `kinds.input.json` and `kinds.output.json` checked in.
- [ ] 2507's schema line names this ADR by number and `shuyi kinds`.
- [ ] `shuyi-assets` still builds for `thumbv7em-none-eabihf` without
      default features (the `schema` feature is off there).
- [ ] `cargo deny check` passes.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `Formats::kinds()` (order, a replaced format, a kind with two
  formats); `Importers::described` keyed by kind and format; `kinds`
  with no kind, one kind, an unknown kind; `parse_args`; `kinds` among
  the ops that run the game (the test at `contract/cli.rs` line 288).
- Fixture: each engine example decodes through its format.
- Snapshot (insta): each standard format's `text_schema()`, so a change
  to a file's shape shows in review.
- Validation: each example, as JSON, validates against its schema with
  a JSON Schema validator as a dev-dependency of `shuyi-cli` (for
  example `jsonschema` with `default-features = false`, MIT; `cargo deny
  check` must pass). Get the JSON with `serde_json::to_value` of the
  decoded kind where it is `Serialize` and has its text's shape
  (`FontAtlasDef`, `TargetProfile`, `CreditsFile`, `English`), and with
  `ron::from_str::<serde_json::Value>` for the rest. That second way
  turns a unit variant (`Own`) into `null` and drops a variant's name
  (`Credit("a")` becomes `["a"]`), so those examples hold no enum
  variant. If no validator passes `cargo deny`, drop this test and say
  so in the completion notes.
- Integration: `kinds` in `examples/hello-grid`
  (`tests/it/contract.rs`, beside the import tests), and with no
  project.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
