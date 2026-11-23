---
id: "2145"
title: "A script is an asset kind: a .rhai file loads through the registry, and an edited script reloads in the running game"
type: feature
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2145 — A script is an asset kind that hot reloads

## Context

`shuyi-script` (ticket 2101, done;
[ADR-0093](../../docs/adr/0093-scripted-rules-rhai-integer-only-with-no-clock-no-luck-no-files-and-limits-views-in-and-values-out-by-serde.md))
runs a modder's rule written in Rhai. Its one way in is
`Runtime::compile(name, source)` in `crates/script/src/lib.rs`. The
game must find the file, read its bytes and compile it once, at load.
A script is not an asset kind: nothing in `crates/script` implements
`shuyi_assets::format::AssetKind` or `Format`, and the crate depends on
no engine crate (`tools/layers.ron`, `"shuyi-script"`: `engine: []`).

Assets reload already. 2220 (done) built
`shuyi_assets::hot::Hot<T: AssetKind>` (ADR-0071). It decodes a value
through the registry, notes every file the decode read, and decodes
again when one changes. If the new decode fails, it keeps the old value
and returns `Reload::Failed` with the errors. A game's module calls
`Hot::refresh` from `Module::reloaded` and hands the answer to
`Ctx::note_reload`, which shows a failure's errors over the frame
(`crates/ui/src/module.rs`, `crates/ui/src/ctx.rs`). Hello, Grid does
this for its palette (`examples/hello-grid/src/hot_palette.rs`), under
`cargo shuyi run --watch`. But `Hot` reloads only values decoded through
the registry, so it can't reload a script.

So a modder who edits a script restarts the game. Changing a rule
without a rebuild is the main reason to use scripts at all. Godot's
GDScript, Defold's Lua and LÖVE all reload scripts in a running game.

The gap is named twice and was never ticketed:

- 2905 (research, open), option 4: "No ticket makes that work yet …
  Say what it needs (a script `AssetKind` and `Format` in
  `shuyi-script`, say)".
- `docs/research/2916-code-pad.md`, *What it needs that isn't there*:
  "a script asset kind and format, so a script is a file the registry
  decodes and 2220 reloads … Neither exists."

This ticket builds it now. It doesn't wait for 2905, whose question is
reloading Rust code. Found by the blind-spot audit of 2026-11-23
(scripting, hot reload and modding).

A Rhai `Script` is run "with the runtime that compiled it" (the doc on
`Script` in `crates/script/src/lib.rs`), and a `Runtime` isn't `Send`.
A `Format` must be `Send + Sync` (`crates/assets/src/format.rs`). So
the format checks that the script compiles, and keeps its text. The
game compiles that text with its own runtime, and a small `HotScript`
does that again after each reload. This keeps ADR-0093's rule that a
script runs on the runtime that compiled it.

## Nick input

None.

## Scope

**In:**
- An optional cargo feature `assets` on `shuyi-script`. It depends on
  `shuyi-assets` with `std` and adds a module `shuyi_script::asset`.
  The `shuyi` facade's `script` feature turns it on
  (`crates/shuyi/Cargo.toml`: `script = ["std", "dep:shuyi-script",
  "shuyi-script/assets"]`).
- `tools/layers.ron`: `"shuyi-script"`'s `engine` list gains
  `"shuyi-assets"`, and its comment says why (`AGENTS.md` rule 5). The
  crate stays pure: no I/O, no clock, no floats.
- `ScriptFile`, an `AssetKind` with `NAME = "script"`. It holds the
  file's display path and its source text, with `name()`, `source()`,
  `compile(&Runtime) -> Result<Script, ScriptError>`, and `PartialEq`
  by name and source. `check --baked` compares it that way. Its fields
  are private and it grows only by methods (ADR-0039).
- `RhaiFormat`, a `Format` for `ScriptFile`:
  - id `"rhai"`, extension `rhai`, no magic bytes;
  - `max_bytes` is `TEXT_MAX_BYTES`;
  - `RhaiFormat::new(limits: Limits)`. Its `decode` makes a `Runtime`
    with those limits and compiles the text, to check it. It keeps the
    text, not the compiled script.
  - Bytes that aren't UTF-8 give `codes::NOT_TEXT`. A compile error,
    or the expression-depth limit, gives one `ContentError` at the
    script's line and column, with `codes::PARSE`, and its message is
    the `ScriptError`'s. It never panics.
  - It is not in `Formats::standard()`. A game that uses scripts
    registers it with `Formats::register`, as it registers its own
    data tables.
- `HotScript`, a `Hot<ScriptFile>` together with the `Script` the
  game's runtime compiled from it:
  - `HotScript::load(&formats, &bundle, id, &runtime)`;
  - `script() -> &Script`, and `id()`;
  - `refresh(&runtime, changed) -> Reload`. When the file changed and
    decodes, it compiles the new text. If the decode fails, or the
    game's runtime refuses the text (its limits may be tighter than
    the format's), the answer is `Reload::Failed` with the errors as
    `ContentError`s. The old `Script` stays.
  - A game's module calls it from `Module::reloaded` and passes the
    answer to `Ctx::note_reload`, as `hot_palette.rs` does with
    `Hot<PaletteDef>`.
- A `From<&ScriptError> for ContentError`, or a named function, that
  turns a script's error into a content error with its file, line and
  column. Release builds load scripts the same way. They just never
  refresh: `tell_changed` in `crates/platform/src/app.rs` asks only with
  debug tools on, or when the platform watches files.
- Docs:
  - `crates/script/README.md` gets a section, *Scripts from the
    bundle, reloaded*, with a module like `hot_palette.rs`. Its
    *How to start* names `RhaiFormat`.
  - `docs/formats.md` gets the format's row or section.
  - The `shuyi-script` skill, if 2102 has written it, gets one step.
- ADR-0093 is amended (the `write-adr` skill picks an amendment or a
  new ADR): the crate's new edge to `shuyi-assets` behind `assets`, and
  why the format keeps the text and the game compiles it.
- **Either order with 2905:** if 2905 is done, read its ADR: if it
  chose something else for scripts (a reload that keeps a script's
  state, say), follow that and note it in the completion notes. If not,
  nothing to do here. 2905's option 4 then measures this ticket's
  reload and writes no new ticket for scripts.
- **Either order with 2102:** if 2102 is done, `examples/scripted-rule`
  loads its rule through `RhaiFormat` and keeps it in a `HotScript`
  module. Its integration test edits the script in the bundle
  (`Bundle::replace`), tells the game the file changed, and checks the
  next move follows the new rule. A broken edit keeps the old rule and
  shows the error in `Ctx::reload_errors`. If not, nothing in the
  example: 2102 loads its script this way and writes that test.
- **Either order with 2144:** if 2144 is done, `shuyi-script` already
  has its `assets` feature and the `shuyi-assets` dependency (with its
  line in `tools/layers.ron`), `ScriptError::to_content_error` and
  `check::bundle`: build the asset kind on them, and let
  `check::bundle` compile through the new loader if that is simpler.
  Use the rest of what it built too:
  - `RhaiFormat::decode` reads each script an `import` names through
    `DecodeCx` and hands it in with `Runtime::add_module`, so editing
    an imported script reloads the script that imports it;
  - the game's helpers (`Runtime::register`) are registered on the
    format's runtime too;
  - the format's errors go through `to_content_error`, so its script
    codes (`SH2011` and on) replace `codes::PARSE`.

  If not, add the `shuyi-assets` dependency here; 2144 then puts
  `to_content_error` and `check::bundle` beside the asset kind.
  `import` stays off here and errors take `codes::PARSE`; when 2144
  turns `import` on, it has the format read imports through
  `DecodeCx` as above, and gives the format its codes.

**Out (do not do):**
- Keeping a script's own variables across a reload. A script holds no
  state between runs (ADR-0093 §4), so there is nothing to keep.
- Reloading Rust code: that is 2905's research.
- A whole game's rules written as a script (`ScriptedTurnRules`,
  `ScriptedTickSim`): that is 2146, parked. Its adapters take a
  compiled `Script`, so a reload builds a new adapter.
- Making `BasicGame` reload scripts with no game code. A game keeps its
  own small module, as Hello, Grid keeps `hot_palette.rs`.
- Changing what a script may do (ADR-0093 §1–§3).

**Estimated size:** about 800 lines (code 300, tests 350, docs 150).

## Implementation steps

1. Read `crates/assets/src/format.rs` (`AssetKind`, `Format`,
   `DecodeCx`, `Formats::register`, `codes`), `crates/assets/src/hot.rs`
   and `examples/hello-grid/src/hot_palette.rs`.
2. In `crates/script/Cargo.toml`, add the optional `shuyi-assets`
   dependency (`default-features = false, features = ["std"]`) and the
   `assets` feature. In `tools/layers.ron`, add the edge. In
   `crates/shuyi/Cargo.toml`, turn it on from `script`.
3. Write `crates/script/src/asset.rs` with `ScriptFile`, `RhaiFormat`,
   `HotScript` and the error conversion. Re-export them from `lib.rs`
   behind `#[cfg(feature = "assets")]`.
4. Write the tests (below), and a row in the golden table
   `crates/script/tests/golden/scripts.ron`. Bless it with
   `cargo xtask golden --bless`. The `asset` module is behind
   `assets`, so make sure CI's `test` and `determinism` jobs build the
   crate's tests with it on (`-p shuyi-script --features assets`, or
   `required-features` on the test target). Otherwise the new row is
   never checked.
5. Write the docs and the ADR amendment. Apply the either-order clauses
   for 2905, 2102 and 2144.

## Acceptance criteria

- [ ] A `.rhai` file in a bundle loads as a `ScriptFile` through
      `Formats::load` once `RhaiFormat` is registered, and its
      `compile` gives a `Script` that `Runtime::call` runs.
- [ ] A script that doesn't compile gives a `ContentError` naming its
      file, line and column, with code `SH2001` (`codes::PARSE`). Bytes
      that aren't UTF-8 give `SH2002`. A file over 4 MiB is refused
      before decoding.
- [ ] A `HotScript` whose file is replaced with a new rule answers
      `Reload::Reloaded`, and the next call gives the new rule's value.
- [ ] A `HotScript` whose file is replaced with a broken script answers
      `Reload::Failed` with the error's line, and the next call still
      gives the old rule's value. Fixing the file reloads it.
- [ ] A text the format accepts but the game's runtime refuses (an
      expression deeper than the runtime's `expression_depth`) answers
      `Reload::Failed`, and keeps the old `Script`.
- [ ] A change to a file the script didn't read answers
      `Reload::Unchanged` and compiles nothing.
- [ ] `cargo xtask check-layers` passes with the new edge, and
      `shuyi-script` without `assets` still builds with no engine crate.
- [ ] The golden table has a row `script: rhai`: what a fixture loaded
      through the format gives on a fixed view. CI's `determinism` job
      checks it on wasm32.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit (`crates/script/src/asset.rs`): decode of a good script, of a
  compile error (line and column), of bytes that aren't UTF-8, and of a
  file over the limit. `ScriptFile` equality.
- Property: arbitrary bytes into `RhaiFormat::decode` return `Ok` or
  errors, never a panic (ADR-0007's fuzz-style test).
- Integration (`crates/script/tests/`, with the `assets` feature): the
  `HotScript` cases above, on a `Bundle` of an owned layer, with
  `Bundle::replace` and `refresh`.
- Golden: the `script: rhai` row.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
