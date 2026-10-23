---
id: "2230"
title: "JSON and CSV tables: Json<T> and Csv<T, R> as formats of a game's own data kinds, beside Ron<T>, behind features"
type: feature
milestone: After MVP
model: sonnet-5
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2230 — JSON and CSV tables for a game's own kinds

## Context

Found by the comparison with Bevy 0.20, Godot 4.7, Unity 6 and Unreal
5.8 (`docs/decisions.md`, 2026-10-20, "a second comparison"), which puts
it after the MVP. Unreal imports Data Tables from CSV and JSON
(https://dev.epicgames.com/documentation/en-us/unreal-engine/data-driven-gameplay-elements-in-unreal-engine).
Bevy's `bevy_common_assets` loads a game's own types from JSON, CSV,
TOML, YAML and more (https://github.com/NiklasEi/bevy_common_assets).
Godot imports CSV only as translations, and Unity has no data-table
import, so this is a smaller gap than the rest of the comparison found.

What Shuyi has today:

- A game's own table is `shuyi_assets::ron::Ron<T>`
  (`crates/assets/src/ron.rs`), the only format of a game's own kind
  (ADR-0019 §7; `docs/formats.md` line 93, the "its own" row).
- `serde_json` is a dependency of `shuyi-cli` only
  (`crates/cli/Cargo.toml` line 43), for the contract's answers. It is in
  `Cargo.lock` at 1.0.154, with `itoa`, `memchr`, `serde_core` and
  `zmij`. No crate reads CSV.
- 1123 (Aseprite and TexturePacker) and 1124 (Tiled and LDtk), both
  parked, will read JSON, but each is a format of one engine kind, not a
  table of the game's.
- ADR-0007 §3: "A game never has to convert a common file before using
  it." For a game's data that holds only for RON today.

Why it matters:

- **The game first:** Visions of Shuyi keeps its RON tables, and
  nothing changes for it. This is for the games after it.
- **Other 2D games:** designers balance numbers in a spreadsheet, which
  saves CSV. Web tools and agents write JSON well.
- **The end goal** (2026-10-19): an agent asked for a game writes its
  data in whatever format it knows best, and the engine reads it.

## Nick input

None.

## Scope

**In:**
- **An ADR** (the `write-adr` skill) extending ADR-0019 §7: a game's own
  table may be `Ron<T>`, `Json<T>` or `Csv<T, R>`. JSON and CSV are for
  a game's own kinds only. The engine's kinds keep RON as their text
  (ADR-0007 §4, `AGENTS.md` rule 10), and `Formats::standard()`
  registers neither. It also records the CSV rules below and that
  serde_json's recursion limit stays on (ADR-0007 §5).
- **`shuyi_assets::json::Json<T>`** (`crates/assets/src/json.rs`;
  `#[cfg(feature = "json")] pub mod json;` in `crates/assets/src/lib.rs`),
  behind a feature `json = ["dep:serde_json"]` that needs no `std`:
  - `serde_json = { version = "1", optional = true, default-features =
    false, features = ["alloc"] }`: MIT OR Apache-2.0, allowed by
    `deny.toml`. Its dependencies are in `Cargo.lock` already, so no new
    crate comes in. Don't turn on `unbounded_depth`: deep nesting stays an
    error, not a stack overflow;
  - built as `Ron` is: a `PhantomData<fn() -> T>` field, `const fn`
    `Json::new()` (id `json`, extension `json`) and `Json::named(id,
    extensions)` as `Ron::new` and `Ron::named` (`ron.rs` lines 25 and
    30), `Default`, and `Debug` written by hand;
  - `decode` reads the text (`format::text`) and `serde_json::from_str`
    into `T: AssetKind + DeserializeOwned`. The error is at serde_json's
    `Error::line` and `Error::column`. Its message is serde_json's text
    without the " at line N column M" its `Display` adds, since
    `ContentError` shows the place. A line of 0 (an error with no place)
    gives an error with none;
  - `max_bytes` is `TEXT_MAX_BYTES`;
  - `Encode` for `T: Serialize`: `serde_json::to_vec_pretty` (two
    spaces) and a newline at the end. Decoding what it writes gives the
    asset back. serde_json fails only on a map key it can't write as a
    string, or on a `Serialize` that errors. `Encode::encode` has no
    error, so `encode` then returns no bytes, and its doc says so.
- **`shuyi_assets::csv::Csv<T, R>`** (`crates/assets/src/csv.rs`;
  `#[cfg(feature = "csv")] pub mod csv;` in `lib.rs`), behind a feature
  `csv = ["std", "dep:csv"]` (the crate reads through `std::io`):
  - the `csv` crate 1.x, `optional = true`: Unlicense OR MIT, both
    allowed. Its own dependencies (`csv-core` with `memchr`, `itoa`,
    `ryu`, `serde`) are MIT, Unlicense, Apache-2.0 or BSL-1.0, all
    allowed: check with `cargo deny`. Inside the module, call the crate as
    `::csv`, as `ron.rs` calls `::ron`;
  - a file is a header row, then one row per record. Each row reads into
    `R: DeserializeOwned` by the header's names (serde's field names),
    and the rows become the kind: `T: AssetKind + From<Vec<R>>`. The
    field is `PhantomData<fn() -> (T, R)>`, so the format is `Send` and
    `Sync` as `Ron` is. So a game keeps one kind, `struct
    Enemies(Vec<Enemy>)`, and may read it from RON, JSON or CSV;
  - `Csv::new()` (id `csv`, extension `csv`), `Csv::tsv()` (id `tsv`,
    extension `tsv`, tab-separated) and `Csv::named(id, extensions,
    delimiter: u8)`;
  - a row holds flat fields: integers, strings, `bool`, enum variants
    with no data (written as their names), and `Option` (an empty cell
    is `None`). Cells and headers are trimmed of spaces
    (`::csv::Trim::All`). A nested field (a struct or a list) is an
    error that says to use RON or JSON for nested data. Find in a test
    which `::csv::DeserializeErrorKind` the crate gives for one, and map
    that kind to this message;
  - **errors:** every bad row, not only the first, each a
    `ContentError` at the row's line (`::csv::Position::line`, no
    column). The message names the column by its header ("column `hp`:
    invalid digit found in string"); the crate gives the cell's index
    (`::csv::DeserializeError::field`). A row with too many or too few
    cells is an error at its line. A header with a name twice is an
    error at the header's line: check the header row before reading the
    rows, since the crate accepts a repeated name;
  - a UTF-8 byte-order mark at the start (spreadsheets write one) is
    skipped, and CRLF line ends read. Test both; strip the mark before
    the reader if the crate doesn't;
  - `max_bytes` is `TEXT_MAX_BYTES`. No `Encode` (Out).
- **The tests build both formats:** `json` and `csv` join the features
  of the crate's dev-dependency on itself (`crates/assets/Cargo.toml`,
  `features = ["png"]` today), so `cargo test` and `cargo clippy
  --all-targets` build them with no extra flag.
- **One kind, three formats** (a new module
  `crates/assets/tests/it/tables.rs`, with `mod tables;` in
  `tests/it/main.rs`): a test kind `Items(Vec<Item>)` with
  `#[serde(transparent)]`, so its RON and JSON files are plain lists,
  and `From<Vec<Item>>`. A test registers `Ron`, `Json` and `Csv`
  formats of it and reads `items.ron`, `items.json` and `items.csv` to
  one value written out in the test. Each format claims its own
  extension, so they don't clash (ADR-0019 §2). `Formats::find` already
  refuses an id that matches two files (`items` with both `items.ron`
  and `items.json` in the bundle): test that the message names both.
- **`shuyi import`** reads these files with no change, once the game
  registers the formats and the kind
  (`Importers::new(formats).kind::<K>(summary)` in its
  `GameTools::importers`, `crates/cli/src/game.rs` line 131).
- **The pack:** a table read from JSON or CSV bakes like any kind
  (2204, `docs/formats.md`, *The pack*). In `tables.rs`, a test bakes
  `Items` from `items.json` and from `items.csv` (`Bake::new(stamp)
  .asset::<Items>(id)`, then `bake`) and checks the pack equal to its
  source: `Bake::check` returns no error. This keeps the pinned snapshot
  in `crates/assets/src/pack/tests.rs` as it is.
- **Determinism:** decoding is integers and strings unless `T` holds
  floats. The format reads what `T` says; the ADR repeats that a game's
  rules use integers (ADR-0002).
- **The `no_std` build:** CI's `no-std` job
  (`.github/workflows/ci.yml` line 175) and the `run-gates` skill
  (`.claude/skills/run-gates/SKILL.md` line 64) also build
  `shuyi-assets` with `--no-default-features --features json` for
  `thumbv7em-none-eabihf`.
- **Either order with 2515:** if 2515 is done, `Json<T>` implements
  `TextSchema` for `T: JsonSchema` (the schema is exact for a JSON
  file), and `Csv<T, R>` for `R: JsonSchema` (the schema of one row; its
  description says the file is a header row then one row per line),
  each when both the `schema` feature and its own are on. If not, 2515
  does it.
- **Either order with 2020:** if 2020 is done, register `Json<Table>`
  and a `Csv<T, R>` of a small kind and row type of `fuzz.rs`'s own in
  `fuzz::registry()`, and add `json` and `csv` to the `fuzz` feature's
  list in `crates/assets/Cargo.toml`. If not, 2020 does it.
- **Either order with 2116:** if 2116 is done, give each format a row
  in `crates/assets/tests/golden/formats.ron` (`<kind>: json` and
  `<kind>: csv`, the `state_hash` of what it decodes from a small
  fixture). Its `determinism` job builds them through the self
  dev-dependency above. If not, 2116 adds the rows. Until then the
  three-format test, with its value written out, is the fixture.
- **Public types (ADR-0037):** both formats keep their fields private
  behind constructors, as `Ron` does (the ADR's kind 8), so a new field
  breaks no game.
- **Docs:** `docs/formats.md`'s table of formats a game registers gets a
  row for each, and a section "Tables in JSON and CSV" with one table
  written all three ways and the CSV rules above.

No API break: two new modules behind features that are off by default.
The game's next pin bump needs no change.

**Size.** If the PR passes about 600 changed lines, `Csv<T, R>` (its
feature, tests and docs) goes to a follow-up ticket (`write-ticket`),
After MVP, and this PR ships `Json<T>`.

**Out (do not do):**
- JSON or CSV for the engine's own kinds (palette, keymap, font atlas,
  credits and the rest);
- JSON formats of one engine kind (1123's Aseprite and TexturePacker,
  1124's Tiled and LDtk);
- a language pack in CSV for translators (2227, parked, leaves CSV to a
  ticket of its own);
- a CSV writer, and writers of the engine's text (2218);
- TOML, YAML, XLSX or ODS: a follow-up if a game asks;
- floats in simulation types;
- a mod's JSON table over the game's RON one (2228's Out);
- references between rows or files (2516).

## Implementation steps

1. Read ADR-0007, ADR-0019, `crates/assets/src/ron.rs`, `format.rs`
   (`text`, `Encode`, `Formats::find`), `pack/bake.rs` (`Bake`),
   `crates/assets/tests/it/txtimg.rs` (the toy format and its tests) and
   `docs/formats.md`, *Writing a format plugin*.
2. Write the ADR (the `write-adr` skill: the next free number and its
   row in `docs/adr/README.md`).
3. In `crates/assets/Cargo.toml`, add the two optional dependencies,
   the `json` and `csv` features, each with a comment saying what it is
   for, and both features in the self dev-dependency. Add the two
   `pub mod` lines to `lib.rs`.
4. Write `json.rs`: `Json<T>`, its `Format` and `Encode`, and its unit
   and property tests.
5. Write `csv.rs`: `Csv<T, R>`, its `Format`, and its unit and property
   tests.
6. Write `tests/it/tables.rs`: the fixtures as static bytes in a
   `Layer::table` bundle (as `txtimg.rs`'s `bundle` does), the
   three-format test, the `Formats::find` test and the bake test.
7. Add the `--features json` build line to CI's `no-std` job and to the
   `run-gates` skill, and run it.
8. Do the either-order clauses whose tickets are done.
9. Write the docs. Run `cargo deny check` and `cargo deny
   --all-features check licenses`, then the gates.

## Acceptance criteria

- [ ] An ADR extends ADR-0019 §7 to JSON and CSV tables of a game's own
      kinds, and `docs/adr/README.md` lists it.
- [ ] One table kind reads the same value, written out in the test,
      from fixtures in RON, JSON and CSV.
- [ ] With `items.ron` and `items.json` both present, `Formats::find`
      gives one error that names both files.
- [ ] A JSON error names the file, the line and the column, and its
      message holds no second "at line".
- [ ] A CSV file with two bad rows gives two errors, each with its line
      and the column's header.
- [ ] A short row, a repeated header and a nested field each give their
      own error with a line.
- [ ] A CSV file saved by a spreadsheet (a byte-order mark, CRLF line
      ends) reads.
- [ ] `Json<T>`'s `Encode` round trip holds (property test).
- [ ] Random bytes, and text shaped like each format, never panic
      either format (property test).
- [ ] A table baked from JSON and from CSV equals its source:
      `Bake::check` returns no error.
- [ ] `Formats::standard()` registers neither format (a test).
- [ ] `shuyi-assets` builds for `thumbv7em-none-eabihf` with
      `--no-default-features --features json`, and without default
      features. CI's `no-std` job and the `run-gates` skill run the
      first.
- [ ] `docs/formats.md` has a row for each format and the section
      "Tables in JSON and CSV".
- [ ] `cargo deny check` and `cargo deny --all-features check licenses`
      pass.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit (`json.rs`, `csv.rs`): each feature of each format on a small
  fixture; each kind of broken file with its error and line (JSON: bad
  syntax, a wrong type, a missing field, an error with no place; CSV: a
  bad cell, a short row, a long row, a repeated header, a nested field,
  two bad rows); a byte-order mark and CRLF line ends; `Csv::tsv`.
- Property: random bytes, and text shaped like each format, never
  panic (as `crates/assets/tests/it/txtimg.rs` does); `Json<T>`'s
  encode then decode gives the asset back.
- Integration (`crates/assets/tests/it/tables.rs`): the one table in
  three formats; `Formats::find` with two candidates; the bake test.

## Completion notes

*(Filled in by the session that completes the ticket.)*
