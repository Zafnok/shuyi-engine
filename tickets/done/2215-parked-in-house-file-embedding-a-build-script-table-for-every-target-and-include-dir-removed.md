---
id: "2215"
title: "In-house file embedding: the shuyi-embed crate writes a game's file table at build time on every target, and include_dir is removed"
type: feature
milestone: "Engine: in-house platform"
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: done
blocked_by: []
nick_input: none
completed: 2026-11-22
---

# 2215 — In-house file embedding, and include_dir removed

> **Unparked** (Nick, 2026-11-22, "Unpark 2215 now", `docs/decisions.md`). It was parked on 2026-10-19 with the in-house platform layer, which stays parked.

## Context

Nick, 2026-10-19 (`docs/decisions.md`): "the two small ones (web saves,
asset packing)" go in-house. A game puts its files in its binary in one of
two ways (`crates/assets/src/bundle.rs`):

- `Files::Dir`: `include_dir!` (`include_dir` 0.7.4, last release June
  2024), only with `std`;
- `Files::Table`: a sorted table of `(path, bytes)` that a build script
  writes, for builds without `std` (the Playdate).

The table already does everything the directory does, on every target. So
the fix is to make the table easy and drop the directory. The work is
ours: a build-script helper in a crate of its own
([ADR-0021](../../docs/adr/0021-the-platform-layer-in-house-clean-room-pieces-in-their-own-crates.md)
§4), written from Cargo's build-script documentation, never from
include_dir's source (§2).

Related: 2204 (the optimized pack) is a different way to ship files and
doesn't change. 2207 (done) added layers from outside the binary
(`Files::Owned`); it doesn't touch how embedded layers are built.

**From 3300 (done):** the template's game crate has a `packed` feature
whose path reads a pack beside the program (`files()` in its `lib.rs`,
through `shuyi::app::pack_beside_program`) and an unpacked path that still
uses `include_dir!`; Hello, Grid's `layer()` is the same. Replace only the
unpacked path with the table and leave the packed one alone.

**From 2087 (done):** a game no longer names `include_dir`. It embeds a
folder with `shuyi::files!("<folder>")`, which gives a
`shuyi_assets::bundle::Embedded`, an engine type that `Files::Dir` holds
and `Layer::dir(name, Embedded)` takes; `files!` expands to
`include_dir!` through the hidden `shuyi_assets::__embed`, and the
template, Hello, Grid, the book's snippets and its steps have no
`include_dir` dependency. ADR-0119 §3 keeps `files!`, `Embedded` and
`Layer::dir` for good, so this ticket changes what `files!` expands to and
what `Embedded` wraps, and the removals below of `Files::Dir` and
`Layer::dir`, and the game's `build.rs` calls, are read as that change
instead: no break for a game.

## Nick input

None.

## Scope

**In:**
- **The crate** `crates/embed` (`shuyi-embed`), MIT OR Apache-2.0, no
  `shuyi-*` dependency, used as a build dependency:
  - `shuyi_embed::Table::new("files").dir("files").write()`, called from a
    game's `build.rs`: walks the folder, sorts paths (`/`-separated,
    relative to the folder), and writes
    `$OUT_DIR/shuyi_embed_files.rs` holding a
    `&'static [(&'static str, &'static [u8])]` of `include_bytes!` with
    absolute paths;
  - prints `cargo::rerun-if-changed` for the folder and every file and
    sub-folder, so adding, changing or removing a file rebuilds;
  - `.skip(pattern)` for files to leave out, which also lists them for
    `Layer::leaving_out`;
  - an error (the build fails with the path) for non-UTF-8 paths, and for
    two paths that differ only by case, since they collide on Windows and
    macOS;
  - `shuyi_embed::files!("files")`, a `macro_rules!` that expands to
    `include!(concat!(env!("OUT_DIR"), "/shuyi_embed_files.rs"))`.
- **`shuyi-assets`**: remove `Files::Dir` and `Layer::dir`, and the
  `include_dir` dependency from `crates/assets/Cargo.toml` and the
  workspace. `Files::Table` and `Layer::table` stay as they are.
- **`examples/hello-grid`** moves to `shuyi-embed`.
- **Every other `include_dir!`** moves to `shuyi-embed` too, so the
  `cargo tree` line below can pass. Done tickets added these after
  this ticket was written:
  - the templates under `templates/` (1129; 2511's `blank` hands
    `Layer::dir` to `BasicGame::new`): each game crate gets a
    `build.rs` and `Layer::table`, `include_dir` leaves the template's
    `[workspace.dependencies]` for `shuyi-embed` as a build dependency
    (`cargo xtask check-pins` compares them with the root's), and CI's
    `templates` job still passes;
  - `shuyi-cli`'s embedded `templates/` and `targets/` folders
    (`crates/cli/src/contract/new.rs` and `target.rs`, from 1129 and
    3001): a `build.rs` in `shuyi-cli` writes them as tables, which
    `new` and `check --target` read.
- **Docs**: the bundle's docs and any guide that shows `include_dir!`
  show `shuyi-embed` instead.
- **For the game:** this breaks the game's API use: `Layer::dir` is gone.
  The PR says so, and the game's PR that moves the pin follows it,
  replacing `include_dir!` with a `build.rs` call (a few lines per layer).
  Write that game ticket (`write-ticket`, *A request from the game*, in the
  other direction) and link it.

**Out (do not do):**
- Compression or packing (2204).
- Changing what a layer is or how it's searched.

## Implementation steps

1. `crates/embed` with `src/lib.rs` (the builder, the writer, the macro).
2. Tests: the builder on a temporary folder writes the table you expect;
   the generated file compiles (an integration test crate with a
   `build.rs` that uses it).
3. Move `hello-grid`, the templates and `shuyi-cli`'s embedded folders,
   then remove `Files::Dir`, `Layer::dir` and `include_dir`.
4. Build the game against this working copy (`run-gates`, *The game at the
   next pin*) with its own change applied in its checkout, so the game's
   PR is known to work.
5. In the completion notes: the game ticket and the documents used.

## Acceptance criteria

- [x] `hello-grid` builds and runs natively and on the web with its files
      from `shuyi-embed`.
- [x] A game from each template builds with its files from
      `shuyi-embed` (CI's `templates` job), and `shuyi new` and
      `shuyi check --target` still work from the binary alone.
- [x] Changing, adding or removing a file under `files/` rebuilds the
      table on the next `cargo build`.
- [x] Two files differing only by case fail the build with both paths.
- [x] `cargo tree --workspace` shows no `include_dir`.
- [x] The game ticket for the pin move exists and is linked: none needed, the game breaks nowhere (Completion notes).
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: path sorting and `/` separators on Windows; skip patterns; the
  case collision; non-UTF-8 paths.
- Integration: the test crate (step 2).

## Completion notes

**Done.** Nick unparked this ticket alone on 2026-11-22
(`docs/decisions.md`); the rest of the in-house platform layer stays
parked. The design is ADR-0138, which amends ADR-0119 §3.

- **`crates/embed`** (`shuyi-embed`), no engine crate below it:
  - `walk` lists a folder, paths `/`-separated and sorted by bytes;
  - `table_source` writes the table of `include_bytes!` with absolute
    paths;
  - `Table::new(name).dir(..).skip(..).write()` writes
    `$OUT_DIR/shuyi_embed_<name>.rs` and `_skipped.rs`, and prints
    `cargo::rerun-if-changed` for the folder and every file and folder
    in it;
  - `files!(name)` and `skipped!(name)` include those two files;
  - a name that isn't UTF-8, two paths that differ only by case, and a
    bad table name each fail the build, naming the paths.
- **`crates/embed-macros`** (`shuyi-embed-macros`, a procedural macro):
  `table!("<folder>")` runs the same walk at compile time, with
  `$VARIABLES` and a folder relative to the crate. It reads the literal
  itself, with no `syn` or `quote`.
- **`shuyi-assets`:** `files!` expands to `Embedded::__from_table(table!(..))`,
  `Embedded` wraps the table, and `Layer::dir` makes a `Files::Table`
  layer of it. `Files::Dir` and `__from_include_dir` are removed, and so
  is the `include_dir` dependency, from the workspace and from
  `tools/layers.ron`. `files!`, `Embedded` (in `shuyi::prelude` too) and
  `Layer::dir` no longer need `std`. The bundle's tests' folders are
  tables now.
- **`shuyi-cli`:** `build.rs` writes the tables of
  `templates/shuyi-default`, `skills/` and `targets/`. `new`,
  `check --target` and the skill pack read them. `targets/` is now
  watched, which it wasn't before. The template plans' golden file is
  re-blessed for the template's `build.rs`.
- **The template's `build.rs`** watches the files folder, so a file
  added or removed rebuilds the game. Hello, Grid's `build.rs` says why it
  already does (it prints no `rerun-if-changed`).
- **The tutorial's ten `Cargo.lock`s** lose `include_dir` and
  `include_dir_macros`, and gain the two new crates.
- **Docs:** the crates table in `README.md`, ADR-0138 and its row,
  ADR-0119's status, the bundle's and `files!`'s docs, the docs of the two
  new crates, and the disclosure check's comment on paths with `\`.
- **Documents used:** Cargo's *Build Scripts* page (`OUT_DIR`,
  `cargo::rerun-if-changed`: a folder is scanned whole, and a build script
  that prints none reruns on any change in the package) and the Rust
  Reference's *Procedural Macros* and *Tokens* (string and raw-string
  literals). Not `include_dir`'s source.

**Deviations** (the ticket was 34 days old; each was fixed against `main`):

- **The game's `build.rs` calls are dropped, and `Layer::dir` stays.** The
  ticket's own 2087 note, and ADR-0119, keep `files!`, `Embedded` and
  `Layer::dir`. A `macro_rules!` `files!` over `$OUT_DIR` would make every
  game write a build script, so `files!` is a procedural macro over
  `shuyi-embed`'s walk instead. `Table` and its `files!` stay for build
  scripts: `shuyi-cli`, and a game that wants `skip`.
- **The templates and Hello, Grid needed no move to `Layer::table`.**
  2087 had already put them on `shuyi::files!`, which now builds the
  table. No template's `[workspace.dependencies]` named `include_dir`, so
  `check-pins` has nothing to compare.
- **`shuyi-cli` embeds three folders, not two:** `skills/` too.
- **The case-collision walk test** runs only where the disk keeps two
  such names (Linux CI). On Windows and macOS the check itself is tested
  on paths.
- **No game ticket:** Visions of Shuyi names neither `files!`,
  `Layer::dir` nor `Files` (searched with `gh search code --repo
  Zafnok/visions-of-shuyi` for `Files::Dir`, `files!`, `Layer::dir`,
  `include_dir`, `__embed` and `__from_include_dir`: hits are only in its
  docs and done tickets). Its layers are tables its own `build.rs` writes
  (game ADR-0105).

Size: 1,578 counted lines, not split: the clean cut is `shuyi-embed` and its fixture alone, then the move of `files!`, `shuyi-cli` and `include_dir`'s removal onto it, but the first part alone lands a crate nothing in the engine uses, and the 120 lines CI's mutants asked for test the macro and the build-script path of both parts.

Game: nothing to adopt, the game's layers are its own build script's tables (game ADR-0105), and it names no removed item.

Breaking: shuyi-assets: `bundle::Files::Dir`: the variant is gone; a layer from `Layer::dir` is a `Files::Table`, so a game that matches on `Files` matches `Table`.

```migration
since: 0.1.0
what: An embedded folder's layer is a file table; the `Files::Dir` variant is gone.
old: Files::Dir(Embedded)
new: Files::Table(&'static [(&'static str, &'static [u8])])
before: |
  if let Files::Dir(_) = files { /* embedded */ }
after: |
  if let Files::Table(_) = files { /* embedded */ }
find: ["Files::Dir"]
```

Behaviour: shuyi-assets, shuyi: `files!` fails the build for a file or folder name that isn't UTF-8, and for two paths that differ only by case; `include_dir!` embedded both (the first silently lost on Windows and macOS).

Behaviour: shuyi-assets, shuyi: `files!`, `Embedded` and `Layer::dir` build without `std`, as `Layer::table` did.

Book: none, the book's chapters already show `shuyi::files!`, `Embedded` and `Layer::dir`, which keep their shapes, and none names `include_dir`.
