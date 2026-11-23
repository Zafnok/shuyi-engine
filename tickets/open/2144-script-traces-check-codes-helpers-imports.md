---
id: "2144"
title: Scripts report traces and errors a modder can read, shuyi check compiles every script, and games register helpers and imports
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2144 — Script traces, problem codes, a check of every script, helpers and imports

## Context

`shuyi-script` (2101, done,
[ADR-0093](../../docs/adr/0093-scripted-rules-rhai-integer-only-with-no-clock-no-luck-no-files-and-limits-views-in-and-values-out-by-serde.md))
is Rhai cut down to what a rule may do. Its audience is modders and
designers who don't write Rust. A blind-spot audit of the engine
(2026-11-23) found that such a person gets almost no help, and checking
the code agrees:

- **No way to see a value.** `crates/script/src/lib.rs:75` disables
  `print`, `debug` and `eval` (`const DISABLED`), and nothing replaces
  them. The crate has no `on_print`, no `on_debug` and no trace.
- **No problem codes.** `ScriptError` (`crates/script/src/error.rs`)
  names a file, a line and a column, but has no code. Every other
  content error the contract shows carries one
  ([ADR-0028](../../docs/adr/0028-problem-codes-a-stable-code-by-class-a-help-line-a-page-and-exit-code-3-for-the-setup.md)):
  the content codes are `shuyi_assets::error::codes`, `SH2001` to
  `SH2010` today, each with a row in `crates/cli/src/contract/codes.rs`
  and a page in `crates/cli/errors/`.
- **`shuyi check` skips scripts.** A broken `.rhai` file is found only
  when the game runs it. 2101's **Out** says a check item for scripts
  waits for check items from modules, "written when a module first needs
  one"; none was written. `check` (`crates/cli/src/contract/check.rs`)
  runs the game's own `GameTools::check`, so the game can compile its
  scripts there if the module gives it one call to do it with.
- **No editor help.** No definition file says what functions a script
  may call. ADR-0093 counts Rune's LSP as an advantage it gave up.
- **No shared code and no helpers.** The `no_module` feature
  (`crates/script/Cargo.toml`) takes out `import`, so a mod with several
  scripts can't share a helper file. The public API is `Runtime::new`,
  `compile`, `run`, `call`, `Script::name` and `has_function`: a game
  can't hand its scripts a pure helper of its own (a grid distance, a
  lookup into a data table), so each script writes it again, and can
  drift from the Rust rules. ADR-0093 §2 dropped the maths and extra
  string packages with "a later ticket can add one back"; this is not
  that ticket (**Out**).

Rhai 1.27 has what each piece needs, checked in its source
(`~/.cargo/registry/src/*/rhai-1.27.0`): `NativeCallContext::call_position`
and `source` for a trace line's place, `Engine::definitions` behind its
`metadata` feature (which brings `serde_json`), and, without
`no_module`, `StaticModuleResolver` and
`Engine::compile_into_self_contained`, which resolves every `import`
while compiling. `Engine::new_raw` sets no module resolver, so nothing
reads a file unless we install one.

**The game.** No game code uses the module yet: at the game's pin
`50472bbcd3084555a61728a5d07518f720bd6514`,
`gh search code --repo Zafnok/visions-of-shuyi 'shuyi_script'` finds
nothing, and `'rhai'` finds only tickets 1011 and 1017 and
`docs/engine/feasibility.md`. game#1017 (its scripted skill effects) is
the first user and gains all of this.

Builds on 2101 and ADR-0093, ADR-0030 (the log a trace goes to),
ADR-0028 (problem codes) and ADR-0039 (types that grow).

## Nick input

None.

## Scope

**In:**
- **A trace.** A script calls `trace(value)`. With tracing off (the
  default) it does nothing and returns `()`. With it on, the runtime
  keeps a line: the script's name, the line and column of the call,
  and the value in `Value`'s `Display` (the script's own form, in key
  order). `Runtime::set_tracing(bool)` turns it on, and
  `Runtime::take_trace() -> Vec<TraceLine>` hands the lines over and
  empties the list. A new `Limits::trace_lines` (default 64) caps the
  lines kept per run; past it the rest are dropped and the last line
  says how many. `TraceLine` is `#[non_exhaustive]` with public fields
  and no constructor a game needs (a value the runtime makes,
  ADR-0039). The `Limits` field is additive (`Limits` is already
  `#[non_exhaustive]` with `Default`).
- The trace never changes what a script gives: the same script on the
  same view returns the same values with tracing on and off. A game
  turns it on where it builds its `Runtime`, for example when
  `Ctx::debug_tools` is on, and hands the lines to `Ctx::log`
  (ADR-0030: output only, nothing reads them to decide). `print`,
  `debug` and `eval` stay disabled.
- **Problem codes** for scripts, new content codes in
  `shuyi_assets::error::codes`, each with its row in
  `crates/cli/src/contract/codes.rs` and its page in
  `crates/cli/errors/` (ADR-0028):
  - `SH2011` `script-compile`: a script doesn't compile, or an `import`
    names no script the game handed in;
  - `SH2012` `script-limit`: a script stopped at one of its limits;
  - `SH2013` `script-run`: a script failed while running, or its view or
    its value didn't convert.
  Take the next free numbers if another ticket took these first.
- **A check of every script.** A new optional feature `assets` of
  `shuyi-script` adds `shuyi-assets` (`default-features = false`) and:
  - `ScriptError::to_content_error(&self) -> ContentError`, with its
    file, line, column and code;
  - `check::bundle(runtime: &Runtime, bundle: &Bundle) -> Vec<ContentError>`,
    which compiles every `.rhai` file of the bundle (`Bundle::files_under`)
    with the game's runtime, in path order, and names each by
    `Bundle::display_path`. It only compiles: it calls no function.
  A game's `GameTools::check` calls it, so `cargo shuyi check` and the
  MCP `check` list a broken script with its file, line and code.
- **Definition files.** A new optional feature `definitions` turns on
  Rhai's `metadata` and adds `Runtime::definitions() -> String`: a
  `.d.rhai` text of every function a script may call with this runtime,
  the game's helpers included, for an editor. It is off by default, so a
  game's `.wasm` doesn't pay for `serde_json`.
- **Helpers a game registers.** `Runtime::register(&mut self, name, f)`
  registers a pure function of one to four arguments. Its arguments
  are read through `Deserialize` and its result is written through
  `Serialize`, so the API names no Rhai type (ADR-0093 §4). `f` is
  `Fn`, not `FnMut`, and returns `Result<R, String>`; an `Err` is a
  `ScriptError` of kind `Run` at the call's place. The doc says what a
  helper must not do: read a clock, draw luck, touch a file, keep state.
- **Imports from what the game hands in.** `no_module` comes off.
  `Runtime::add_module(&mut self, path: &str, script: &Script)` makes a
  compiled script importable as `import "<path>" as name;` (the bundle
  path without `.rhai`). `compile` then resolves every import while
  compiling (`compile_into_self_contained`) from those modules only,
  through a `StaticModuleResolver`. An import of anything else is a
  compile error (`SH2011`). Nothing ever reads a file. A module's
  top-level code runs once, when it is added, with no view.
- An ADR that extends ADR-0093 (the `write-adr` skill): the trace, the
  codes, the features, `register` and imports, and the `.wasm` bytes
  each costs, measured as ADR-0093 measured them (Hello, Grid calling a
  script, release build).
- `crates/script/README.md` and the crate docs: *What a script can't
  do* and *How to start* say the new pieces.

**Out (do not do):**
- Breakpoints and stepping through Rhai's `debugging` feature, over the
  live link (2521) or MCP. Not planned by any ticket; write one when a
  modder needs it.
- Typing a view's fields for an editor (completing `view.growth`): a
  view is a map in Rhai, and its definitions can't type a map's keys.
- A contract op or CLI command that runs a script on a view: a module's
  own op, which 3519 lets modules add.
- Scripts as an asset kind and their hot reload (2145), except as the
  **Either order with 2145** clause below says.
- A game's rules written as a script (2146), mods' discovery and order
  (2147), and visual scripting (1135).
- Adding back Rhai's maths or extra string packages, and an engine
  package of grid or fixed-point helpers: the numbers it would wrap are
  2139's, which is parked.
- Any change to what a script may do besides `trace` and `import`: no
  clock, no luck, no floats, no files, as ADR-0093 keeps.

**Either order with 2145:** if 2145 is done, `shuyi-script` already
depends on `shuyi-assets`, and this ticket builds on its format:
- put `to_content_error` and `check::bundle` beside its asset kind
  (replacing its own conversion of a `ScriptError`, if it wrote one),
  and let `check::bundle` reuse its loader if it compiles through the
  registry;
- imports read the imported script through `RhaiFormat`'s `DecodeCx`,
  so a `HotScript` reloads the script that imports an edited one;
- the game's helper functions are registered on the format's checking
  runtime too;
- the new codes replace `codes::PARSE` in `RhaiFormat`'s errors.

If not, add the `assets` feature and the dependency here, with its line
in `tools/layers.ron`, and nothing else: 2145 uses what this ticket
builds when it lands second.

**Either order with 2102:** if 2102 is done, its skill
`skills/shuyi-script` gains a *When it fails* section: turning the trace
on, `check::bundle` in the game's `check`, and the three codes, with its
worked example run again. If not, nothing to do here (2102 writes them
into the skill).

**For the game:** additive. Two behaviours change, each a `Behaviour:`
line in the completion notes: `import` of a handed-in path now compiles
(an import of anything else still doesn't), and `trace` is a function
name a script can call. The game's PR that moves the pin follows this
one.

**Estimated size:** about 1,600 lines (code 750, tests 550, docs 300).

**Split:** past 900 lines, split off steps 6 and 7 (`register` and
imports) into a new ticket, and leave the ADR's part on them to it.

## Implementation steps

1. Read ADR-0093, `crates/script/src/lib.rs`, `error.rs`, `limits.rs`
   and `tests.rs`, and ADR-0028's rules for a new code.
2. **Trace.** In `lib.rs`, register a native `trace` function on the
   engine that pushes into a shared buffer (`Rc<RefCell<…>>`; the
   `Runtime` isn't `Send` already) when tracing is on. Take the place
   from `NativeCallContext::call_position` and the name from `source`.
   Read the value into `Value` for its text. Add `trace_lines` to
   `Limits` (default 64, 0 taken as 1 as the others are). Add
   `TraceLine`, `set_tracing` and `take_trace`.
3. **Codes.** Add `SCRIPT_COMPILE`, `SCRIPT_LIMIT` and `SCRIPT_RUN` to
   `shuyi_assets::error::codes`, their rows to the table in
   `crates/cli/src/contract/codes.rs`, and `SH2011.md` to `SH2013.md` in
   `crates/cli/errors/`, each with an example and how to fix it.
4. **Check.** Add the `assets` feature to `crates/script/Cargo.toml`
   and `shuyi-assets` to `shuyi-script`'s `engine` list in
   `tools/layers.ron` (as the **Either order with 2145** clause says).
   Write `ScriptError::to_content_error` (its `usize` line and column
   to `u32`, saturating) and `check::bundle`.
5. **Definitions.** Add the `definitions` feature (`rhai/metadata`)
   and `Runtime::definitions`. Check the licenses of what `metadata`
   brings with `cargo deny check` and name them in the README's *What
   it costs*.
6. **Helpers.** Add `Runtime::register` for one to four arguments
   through a small sealed trait over `Fn(A) -> Result<R, String>`,
   `Fn(A, B) -> …` and so on, converting through `rhai::serde`.
7. **Imports.** Drop `no_module` from the `rhai` features. Keep a
   `StaticModuleResolver` in the runtime, install it on the engine, and
   compile with `compile_into_self_contained`. `add_module` evaluates
   the script into a module (`Module::eval_ast_as_new`) with the
   runtime's limits. Turn Rhai's errors for a missing or circular
   import into `ErrorKind::Compile`.
8. Measure the `.wasm` and `.exe` bytes with each piece, and write the
   ADR. Update `crates/script/README.md` and the crate docs.
9. Run the golden table (`crates/script/tests/golden`) on x64 and, as
   CI's `determinism` job does, on wasm32: its values don't change.

## Acceptance criteria

- [ ] A script that calls `trace(view.growth)` on a view with growth 3,
      with tracing on, gives one `TraceLine` naming its file, the call's
      line and column, and the text `3`; with tracing off it gives none,
      and both runs return the same value.
- [ ] A run that calls `trace` 100 times with the default limits keeps
      64 lines, and the last says how many were dropped.
- [ ] `print`, `debug` and `eval` still don't compile.
- [ ] A script that doesn't compile gives a `ContentError` with code
      `SH2011` and its line; one that loops forever, `SH2012`; one that
      divides by zero, `SH2013`. `cargo shuyi explain SH2011` (and 2012,
      2013) prints its page.
- [ ] `check::bundle` over a bundle with two good scripts and one broken
      one returns one error, naming the broken file's display path.
- [ ] With `definitions` on, `Runtime::definitions()` names a helper the
      game registered and the built-in `trace`. Without it, the crate
      builds with no `serde_json` in its tree (`cargo tree -p
      shuyi-script -e normal`).
- [ ] A registered helper `dist(a, b)` is called from a script and its
      value comes back; a helper that returns `Err` gives a
      `ScriptError` of kind `Run` at the call's line.
- [ ] `import "mods/util" as util; util::double(view.growth)` compiles
      and runs once `mods/util` was added; an import of any other path
      fails to compile with `SH2011`; no file is read (the runtime has
      no file resolver, and a test imports a path that exists on disk
      and gets the error).
- [ ] The golden table's values are unchanged on x64 and wasm32, and
      the property test of two runtimes agreeing still passes, with
      tracing on in one and off in the other.
- [ ] `tools/layers.ron` names `shuyi-assets` in `shuyi-script`'s line,
      and `cargo xtask check-layers` passes.
- [ ] The new ADR is written and ADR-0093's status line says it is
      extended.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit (`crates/script/src/tests.rs`): the trace on and off, its place,
  its cap; each code from each `ErrorKind`; `check::bundle` over an owned
  bundle (`Layer::owned`); `register` at each arity and its `Err`;
  `import` of an added module, of an unknown path, of a path on disk,
  and a circular pair.
- Property: two runtimes, one tracing and one not, give the same value
  for generated views, and the same as the Rust oracle.
- Golden: the existing table, unchanged, plus a fixture script that
  imports a helper module, as a new row.
- Snapshot / integration: `cargo shuyi explain` for the three new codes.
  The codes' gate (`crates/cli/src/contract/codes/tests.rs`,
  `every_code_is_well_formed_has_its_page_and_is_used`) passes with
  their rows, pages and uses.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
