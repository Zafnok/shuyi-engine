---
id: "3519"
title: "Modules add contract tools, saved option rows, importers and bake steps, and a backend outside this repository runs a conformance harness"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: []
nick_input: none
completed:
---

# 3519 — Modules add contract tools, saved option rows, importers and bake steps, and an outside backend runs a conformance harness

## Context

[ADR-0040](../../docs/adr/0040-modules-join-a-running-game-a-library-first-a-module-trait-with-declared-hooks-and-state-by-type-on-the-context.md)
gave a module one way into a running game: `Module::install` declares
hooks, state and debug entries (`crates/ui/src/module.rs`). Its
*Consequences* (line 167 on) left the rest to "the tickets that need
them": system events for modules, option rows from modules ("a player
setting needs a place in the saved settings"), screens a module pushes
(2367, parked), and "importers, bake steps and contract operations from
modules". No ticket plans the option rows, the importers, the bake steps
or the contract operations. Source: the blind-spot audit of 2026-11-22
(*Architecture*: "ADR-0040's deferred extension points have no
tickets"; *Platforms*: no conformance suite an out-of-tree backend can
run).

What the code has today:

- **Contract operations are a closed list.** `Op`
  (`crates/cli/src/contract.rs:84`) is `#[non_exhaustive]`, and `Op::ALL`
  is a fixed slice. The MCP server's `tools()` (`crates/cli/src/mcp.rs`)
  lists `Op::ALL` and `Op::SESSION` and nothing else. `GameTools`
  (`crates/cli/src/game.rs:470`) lets the *game* fill in the engine's
  operations, but a module can add none. So a physics or networking
  module can't give an agent a tool of its own, though the engine sells
  itself on agent tooling (ADR-0005, ADR-0022).
- **Option rows are the game's alone.** `OptionsScreen::new(rows: Rows<G>)`
  (`crates/ui/src/screens/options.rs:641`; `Rows<G> = fn(&Ctx<G>) -> Vec<OptionRow<G>>`)
  shows only the game's rows. The saved form of the settings is the
  game's (`GameSettings::to_ron` and `from_ron`,
  `crates/ui/src/settings.rs`), written under `SETTINGS_KEY` by
  `Ctx::write_settings` (`crates/ui/src/ctx.rs:1369`). A module has no
  setting it can save without the game's own record carrying it.
- **Importers and bake steps already have builders, but no rule.**
  `GameTools::importers` returns an `Importers` that `Importers::kind`
  and `kind_with_text` extend (`crates/cli/src/contract/import.rs:151`).
  `GameTools::bake` returns a `GameBake` whose `Bake` takes `under`,
  `under_as` (an optimized format derived at bake, ADR-0007 §4) and
  `first` (`crates/assets/src/pack/bake.rs:351-423`). A module could
  export functions that extend them, as ADR-0040 §5 says for formats.
  Nothing says so, and no test proves it.
- **A backend outside this repository has nothing to run.** Only
  storage has a shared suite: `shuyi_ui::storage::conformance_suite`
  behind the `storage-conformance` feature (`crates/ui/Cargo.toml:24`,
  `crates/ui/src/storage.rs:129`), which `shuyi-app` and
  `shuyi-playdate` call. The frame test of a seven-method backend is a
  private test (`crates/platform/tests/it/bare.rs`). The raster parity
  of ADR-0052 runs only in this repository's CI, through the `capture`
  feature of `shuyi-platform` and `shuyi-app`'s capture mode. A private
  console backend (ADR-0008 §2) can't check itself against any of them.

Exporters from outside are 3335's (a `shuyi-export-<id>` program on
`PATH`). Its Out line hands this ticket the rest of ADR-0040's deferred
list and the conformance harness.

The game is unaffected. It pins engine `50472bbc`.
`gh search code --repo Zafnok/visions-of-shuyi 'impl GameTools'` finds
`crates/tools/src/game_tools.rs` and `crates/tools/src/skirmish.rs`:
every addition here to `GameTools` is a defaulted method, so they keep
building. `gh search code --repo Zafnok/visions-of-shuyi 'with_module'`
finds `crates/ui/src/hot.rs` and `crates/ui/src/story_preview.rs`: those
modules declare no option rows, so the game's options screen shows what
it shows today.

## Nick input

None.

## Scope

**In:**
- **An ADR** (the `write-adr` skill), amending ADR-0040's
  *Consequences*, at the next free number (PR 532 takes ADR-0136). It
  has a table of every extension point a module may have: who owns it
  (the module's `install`, a function the game calls, or the module's
  tools half), where it is listed, and whether it is additive before
  2005 freezes the API. Rows:
  - hooks, state, debug entries (ADR-0040, as they are);
  - formats, importers and bake steps: functions the game calls;
  - option rows and their saved values: `install`;
  - contract tools: the module's tools half, through `GameTools`;
  - exporters: 3335;
  - system events and pushed screens: still deferred, with the reason.
  It also weighs and refuses two options:
  - a tool listed in MCP's `tools/list` for each module tool (that needs
    the game's tool program built before `tools/list` can answer);
  - module settings inside the game's own `GameSettings` record (every
    game's `to_ron` would have to carry them).
- **Contract tools from modules.** A trait `ModuleTool` in
  `shuyi_cli::game`, with:
  - `module()`, the `Module::name` it belongs to;
  - `name()`, `snake_case`, unique within its module;
  - `summary()`;
  - `input_schema()` and `output_schema()` (`schemars::Schema`);
  - `run(&self, root: &Path, input: serde_json::Value) -> Result<serde_json::Value, String>`.

  `GameTools` gains `fn module_tools(&self) -> Vec<Box<dyn ModuleTool>>`,
  default empty. A module crate gives its tools behind a `tools` feature
  of its own, which the game's tools crate (already a `shuyi-cli` user)
  turns on.

  A module tool is the route for a `shuyi-script` tool that runs a script
  on a JSON view and returns its value and trace (2144 leaves it here).
  This ticket only makes module tools possible; it doesn't write that one.
- **One new operation, `tool`** (`Op::Tool`, in `Op::ALL`):
  - `shuyi tool` with no name lists every module tool: module, name,
    summary and both schemas;
  - `shuyi tool <module>.<name> --input '<json>'` runs one and answers
    an `Answer` whose result is the tool's JSON;
  - the MCP server has a `tool` tool with input `{ module, name, input }`.

  It runs in the game's tool program, as `check` and `bake` do
  (`in_tools`). An unknown module or name, and an input that its schema
  refuses, get a new problem code (the next free `SH30xx`, its page in
  `crates/cli/errors/`, ADR-0028). A tool's own `Err` answers
  `ok: false` with its text. Regenerate `crates/cli/schemas/` and add
  their line to `versions.json`; a new operation keeps
  `contract::VERSION` (ADR-0150).
- **Option rows from modules, saved apart from the game's settings.**
  - `Install::option_row(make: fn(&Ctx<G>) -> OptionRow<G>)`.
  - `InstalledModule` gains `option_rows: Vec<&'static str>`, the rows'
    keys. It is `#[non_exhaustive]`, so the field is additive.
    `run --json`'s `ModuleEntry` (`crates/cli/src/contract/session.rs:276`)
    lists them.
  - `OptionsScreen::rows` shows the game's rows, then each module's in
    install order. They go before the game's
    `OptionRow::restore_defaults` row when it is there, and last when it
    isn't. A game with no module rows sees the same rows as before.
  - A module's values live in `ModuleSettings`, a map from a module's
    name and a key to a short string, saved as `<module>.<key>`. `Ctx`
    reads it with `module_setting(module, key) -> Option<&str>` and
    changes it with `preview_module_setting(module, key, value)`. It is saved under a storage
    key of its own, `MODULE_SETTINGS_KEY = "module_settings"`, as RON,
    per profile in the same way `SETTINGS_KEY` is. A value that can't be
    read is dropped with a warning in the log, as the settings' are.
  - Keep, throw away and *Restore defaults* cover module values as they
    cover the engine's. The screen's `opened_with` snapshot holds them,
    and `Settings::restored` clears them.
  - New public types are `#[non_exhaustive]` with constructors
    (ADR-0039). `Settings` itself doesn't change.
- **Importers and bake steps from modules**, as the game's call. The ADR
  and the docs say the shape: a module exports
  `fn importers(i: Importers) -> Importers` and `fn bake(b: Bake) -> Bake`,
  and the game calls them in its `GameTools::importers` and `bake`, as it
  calls a module's format function in `formats()` (ADR-0040 §5). No new
  type. A test module proves it (steps).
- **A backend conformance harness**: a feature `conformance` of
  `shuyi-platform` (`no_std` with `alloc`, as `capture` is), turning on
  `capture`, `shuyi-ui/toy` and `shuyi-ui/storage-conformance`. Module
  `shuyi_platform::conformance`, with
  `run(platform: &mut dyn Platform, capture: Option<&mut dyn Capture>) -> Report`.
  Each check is one row, passed, failed with why, or skipped with why:
  - **frames**: the toy game runs 120 frames through `App::frame`. Each
    one is presented once, and a quit request ends the loop with
    `FrameOutcome::Quit`. This is what `bare.rs` checks, made public.
  - **storage**: `conformance_suite` on `platform.storage()`.
  - **audio**: a sound and a music request of the toy reach
    `Audio::play`. `stop_all`, `suspend` and `resume` don't panic.
    `music_playing` gives the track the toy asked for, or `None` on a
    backend whose `Audio` is `Silent` (skipped, and the report says so).
  - **facts**: `facts()` names a target whose profile exists
    (ADR-0060).
  - **raster**: when the backend gives a `Capture` (a trait: paint this
    `Frame` and give its RGBA pixels), the harness plays the engine's
    standard plan and compares each captured frame with the raster by
    `raster::compare` and the plan's `Tolerance` (ADR-0052). The plan is
    a RON file in the crate, made from the toy game. With no `Capture`,
    the row is skipped.

  `Report` is plain data with a `passed()` and a `Display` a test
  prints. `crates/platform/tests/it/bare.rs` runs it on `Bare`.
  `shuyi-app`'s tests run it on the headless form of its platform, the
  way they call the storage suite today.
- Docs (the `document` skill):
  - `docs/modules.md` gets a section *What a module adds*, with the
    ADR's table and a worked example of each extension point.
  - `docs/contract.md` gets the `tool` row.
  - `docs/platform-backends.md` gets *Checking a backend*, on the
    harness. PR 532 edits that file too, so merge `main` before writing
    the section.
- **Either order with 2526:** if 2526 is done, the *What a module adds*
  section goes in its `docs/writing-a-module.md`, and its outside-module
  probe lists a crate's module tools and option rows; if not, write the
  section in `docs/modules.md` (2526 moves it when it writes its page).
- **Either order with 3518:** if 3518 is done, its throwaway outside
  module also adds one module tool and one option row, and its findings
  file lists any friction with them; if not, nothing to do here (3518's
  module uses whatever extension points exist when it runs).

**Out (do not do):**
- Exporters from outside the engine (`shuyi export <id>` for a private
  backend): 3335, a `shuyi-export-<id>` program on `PATH`.
- The `shuyi-script` tool that runs a script on a view: a later ticket
  over this one's `ModuleTool` (2144 leaves it here).
- Screens a module pushes over a running game: 2367 (parked).
- System events for modules: ADR-0040 §6 holds. The first module that
  must hear one adds the platform-side hook in its own ticket.
- A module tool that reads a running session's state, such as a physics
  inspector over the MCP session. `Session` exposes no module state
  today. If the ADR keeps the idea, the completion notes file the
  ticket.
- Each module tool as its own MCP tool in `tools/list`: refused in the
  ADR (see In).
- Installing a module's tools half with `shuyi add`: 2526 adds crates
  and their metadata.
- Which targets must pass the harness to be called supported: not this
  ticket's. It only makes the harness.
- Any console's backend or SDK (`AGENTS.md` rule 4, ADR-0008 §2).
- Changing a built-in operation, the options screen's rows for a game
  without module rows, or the game's `GameSettings` record.

**Estimated size:** about 2,200 lines (code 1,000, tests 750, docs 450).

**Split:** past 1,000 lines, split off steps 8 and 9 (the conformance
harness) into a new ticket; past 1,500, split off steps 5 and 6 (option
rows from modules) too.

## Implementation steps

1. Write the ADR, as the Scope says. Read first: ADR-0040, ADR-0009,
   ADR-0022, ADR-0039, ADR-0052, ADR-0060, and 3335.
2. `crates/cli/src/game.rs`: add `ModuleTool` and
   `GameTools::module_tools`. In `crates/cli/src/contract/`, add
   `tool.rs` with `ToolInput` (`module`, `name`, `input`), `ToolOutput`
   (the list, or the one tool's result), `tool(tools: &dyn GameTools, root, input)`,
   `parse_args` and `text`. Add `Op::Tool` to `Op`, `Op::ALL`,
   `Op::name` and the usage table in `contract.rs`'s module doc.
3. Validate the input against the tool's `input_schema` before `run`,
   with the JSON-schema checking the contract already uses for its own
   inputs. If it has none, check required fields and types by hand, and
   say so in the ADR. Add the problem code and its page.
4. The MCP server: list `tool` through `tool_listing`, and dispatch it
   to the game's tool program as the other in-tools operations are
   (`crates/cli/src/mcp/host.rs`). Regenerate the schemas and add the
   fingerprint to `schemas/versions.json`.
5. `crates/ui/src/module.rs`: `Install::option_row`,
   `InstalledModule::option_rows`, and the makers kept on `Ctx`'s module
   list beside the debug-entry makers. `crates/ui/src/settings.rs` (or a
   new `settings/modules.rs`): `ModuleSettings` with its RON form and
   `MODULE_SETTINGS_KEY`.
6. `crates/ui/src/ctx.rs`: load `ModuleSettings` where the settings load
   (`load_settings`), and give `module_setting` and
   `preview_module_setting`. Keep, discard and restore through the paths
   `preview_settings` and `keep_settings` use. In
   `crates/ui/src/screens/options.rs`, add the module rows in
   `OptionsScreen::rows`, put them in the `opened_with` snapshot, and
   clear them in the *Restore defaults* row. List `option_rows` in
   `ModuleEntry` (`crates/cli/src/contract/session.rs`).
7. The test module: `crates/cli/tests/it/module_tools.rs` with a small
   module in the test. It installs a `Module` with an option row. It has
   an `importers` function adding a kind, a `bake` function adding an
   `under_as` step, and a `ModuleTool`. A test `GameTools` over the toy
   game calls them all.
8. `crates/platform/src/conformance.rs` behind the `conformance` feature,
   with `Capture`, `Report`, `run` and the plan file
   `crates/platform/src/conformance/standard.ron`. Add the feature line
   to `crates/platform/Cargo.toml` with a comment, as the others have.
   Check `tools/layers.ron` needs no new edge (`shuyi-platform` already
   depends on `shuyi-ui`). Run `cargo xtask check-layers`.
9. Run `conformance::run` in `bare.rs` and in `shuyi-app`'s tests. Build
   it in CI's `no-std` job with `--features conformance` beside
   `capture`.
10. Write the docs, as the Scope says.

## Acceptance criteria

- [ ] With the test module's tools, `shuyi tool --json` lists exactly
      its one tool, with module, name, summary and both schemas.
- [ ] `shuyi tool <module>.<name> --input '{…}' --json` answers
      `ok: true` with the tool's JSON. The MCP `tool` tool answers the
      same.
- [ ] An unknown module, an unknown name and an input missing a required
      field each answer the new problem code. The tool's `run` isn't
      called for the last (a counter in the test proves it).
      `shuyi explain` prints the code's page.
- [ ] A game whose `GameTools` leaves `module_tools` at its default
      lists no tools and answers every other operation as before.
- [ ] The toy game with the test module installed shows the module's row
      after the game's rows and before *Restore defaults*. Without the
      module, its rows are exactly what they were.
- [ ] Changing the module's row and keeping it writes `module_settings`
      in storage, and a new `Ctx` over that storage reads the value back.
      Throwing the change away writes nothing. *Restore defaults* clears
      the value.
- [ ] A `module_settings` record that isn't RON loads as empty, with one
      warning in the log.
- [ ] `run --json` lists the module's option row keys in its
      `ModuleEntry`.
- [ ] `shuyi import` of a file of the test module's kind reads it through
      the module's `importers` function. `shuyi bake` followed by
      `shuyi check --baked` passes with the module's `under_as` step.
- [ ] `conformance::run` on `Bare` passes frames, storage, audio and
      facts, and skips raster with its reason. On a `Bare` whose
      `present` never counts a frame, it fails frames and names the frame.
- [ ] `conformance::run` with a `Capture` that paints with the raster
      passes raster. One that flips a pixel past the tolerance fails it,
      naming the plan's capture.
- [ ] `shuyi-platform` builds with `--no-default-features --features conformance`
      for the Playdate target in CI's `no-std` job.
- [ ] `cargo xtask check-layers` passes.
- [ ] The schemas have their line in `versions.json`, and
      `contract::VERSION` rose only for a break (`cargo xtask contract`
      passes).
- [ ] The ADR is in `docs/adr/`, and ADR-0040 says it is amended by it.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `ModuleSettings` round-trips through RON; two modules' values
  under the same key stay apart; two modules with the same tool name are
  told apart by module; `Report`'s
  `passed()` is false with one failed row and true with skipped ones.
- Integration (`crates/cli/tests/it/module_tools.rs`): each `tool`,
  `import` and `bake` acceptance case, on the toy project with the test
  module.
- Integration (`crates/ui`): the options screen cases (rows' order,
  keep, discard, restore, an unreadable record), reading the
  `OptionsView` and storage, never cells.
- Integration (`crates/platform/tests/it/bare.rs`): the harness on
  `Bare`, a broken `Bare`, and the two `Capture` cases.
- Snapshot: the `tool` operation's schemas.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
