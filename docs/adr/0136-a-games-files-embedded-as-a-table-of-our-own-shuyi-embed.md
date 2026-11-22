# ADR-0136: A game's files embedded as a table of our own: `shuyi-embed` for build scripts, and `files!` over it with no build script

- **Status:** Accepted
- **Date:** 2026-11-22
- **Related tickets:** 2215; 2087 (`files!` and `Embedded`), 3300 (the
  packed build), 2204 (the pack)
- **Amends:** ADR-0119 §3 (what `files!` expands to)

## Context

A game puts its files in its program in one of two ways
(`crates/assets/src/bundle.rs`): a folder `shuyi::files!` embeds, which
expanded to `include_dir!` (0.7.4, June 2024, ADR-0021's table), and a
table of `(path, bytes)` a build script writes (`Layer::table`), until
now the way without `std` (the Playdate). The table reads exactly as the
folder does. Nick unparked moving off `include_dir` on 2026-11-22
(`docs/decisions.md`). ADR-0021 §2 asks for our own code, written from
documentation; §4 puts it in a crate of its own.

ADR-0119 keeps `files!`, `Embedded` and `Layer::dir` for good, so that
replacing `include_dir` breaks no game. A `macro_rules!` can't list a
folder, so `files!` with no build script needs a procedural macro. A
procedural macro can't tell Cargo that a folder's list of files matters,
on stable Rust: `include_bytes!` makes a changed file rebuild the crate,
but a file added or removed rebuilds it only when a build script watches
the folder (`cargo::rerun-if-changed=<folder>`, which Cargo scans whole:
Cargo's build-script documentation). `include_dir!` had the same limit.

Visions of Shuyi writes its own tables in its `build.rs` (game ADR-0105)
and names neither `files!` nor `Layer::dir`.

## Decision

1. **`shuyi-embed`** (`crates/embed`) is the walk and the table, with no
   engine crate below it:
   - `walk(folder, skip)` lists every file at any depth, with its path
     from the folder, `/`-separated on every host, sorted by bytes, as
     `Layer::table` searches it. A name that isn't UTF-8, and two paths
     that differ only by case (one file on Windows and macOS), are errors
     naming the paths.
   - `table_source` writes the table as Rust: a
     `&'static [(&'static str, &'static [u8])]` of `include_bytes!` with
     absolute paths.
   - `Table::new(name).dir(folder).skip(pattern).write()`, from a build
     script, writes `$OUT_DIR/shuyi_embed_<name>.rs` and
     `shuyi_embed_<name>_skipped.rs` (the paths a pattern left out, for
     `Layer::leaving_out`), and prints `cargo::rerun-if-changed` for the
     folder and every file and folder in it. `shuyi_embed::files!(name)`
     and `skipped!(name)` include them. A walk error fails the build with
     the path.
   - A skip pattern matches a whole path: `*` and `?` inside a part,
     `**` for any number of folders.
2. **`shuyi-embed-macros`** (`crates/embed-macros`) is a procedural
   macro, `table!("<folder>")`, that runs the same walk at compile time
   and expands to the same table. The folder may name environment
   variables (`$CARGO_MANIFEST_DIR`, a build script's `$SHUYI_FILES`), and
   a relative one is the crate's. It reads the string literal itself,
   with no `syn` or `quote`.
3. **`shuyi_assets::files!`** expands to `Embedded::__from_table(table!(…))`
   through the hidden `__embed`, so `Embedded` wraps a table, and
   `Layer::dir` makes a `Files::Table` layer of it. `Files::Dir` goes:
   every embedded layer is a table. `files!`, `Embedded` and `Layer::dir`
   no longer need `std`.
4. **Who watches the folder:** a build script. The template's `build.rs`
   prints `rerun-if-changed` for the files folder. Hello, Grid's and the
   tutorial's print no `rerun-if-changed`, so Cargo reruns them when any
   file of the package changes. A game without a build script rebuilds
   on a changed file, as with `include_dir!`, and writes one to rebuild
   on an added one.
5. **`shuyi-cli`** embeds the template, the skill pack and the target
   profiles with `Table` in its `build.rs`.

## Consequences

- No `include_dir` and no `include_dir_macros` in any build: the engine
  owns embedding on every target.
- No game changes: `files!`, `Embedded` and `Layer::dir` keep their
  shapes. The removed `Files::Dir` variant is a break only for a game
  that matches on `Files` (it is `#[non_exhaustive]`; Visions of Shuyi
  matches none).
- `shuyi-cli`'s `targets/` is now watched too, which it wasn't.
- `shuyi-assets` depends on a procedural macro on every target, the
  no-`std` build too: it runs on the host, and builds in about a second.
- A game whose build script prints `rerun-if-changed` for other paths
  only (as the template's did) rebuilds on an added file only once it
  lists its files folder too.

## Alternatives considered

- **A build script in every game, as the ticket first planned**
  (`files!` including `$OUT_DIR`). It rebuilds on an added file always,
  but every game must write one, and a game of its own shape breaks:
  ADR-0119 keeps `files!` working without it.
- **Only the procedural macro.** `shuyi-cli` would then have no way to
  watch its folders outside its crate, and a build script couldn't leave
  files out (game ADR-0105's READMEs).
- **`syn` and `quote` for the macro.** Two more dependencies for reading
  one string literal.
- **Keep `include_dir`.** Its last release is from June 2024, and
  ADR-0021 puts the platform's pieces in our own code.
