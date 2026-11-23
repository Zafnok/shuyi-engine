---
id: "3518"
title: "A throwaway module built outside the workspace uses the extension API before it freezes, and an ADR decides how the extension traits are versioned"
type: infra
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: []
nick_input: none
completed:
---

# 3518 — A module from outside the workspace tries the extension API, and the extension traits' versioning is decided

## Context

Found by the blind-spot audit of 2026-11-23 (its findings "the extension
API is frozen at the first release before any outside crate has used
it", high, and "one version for every crate means each engine minor
release breaks every third-party crate", medium).

**No crate outside this repository has used the extension API.**
[ADR-0040](../../docs/adr/0040-modules-join-a-running-game-a-library-first-a-module-trait-with-declared-hooks-and-state-by-type-on-the-context.md)
gives outside crates these ways in:

- `Module<G: GameData>` (`crates/ui/src/module.rs:33`), with `Install`
  (`hook`, `state`, `debug_entry`, lines 186, 195 and 213), `Hook`, and
  `Ctx::module_state` / `module_state_mut` (`crates/ui/src/ctx.rs:541`
  and 546);
- the format registry: `AssetKind` and `Format`
  (`crates/assets/src/format.rs:61` and 79), `Formats::register` (721)
  and `Formats::register_import_settings` (781).

`grep -rn "impl.*Module<" crates examples` finds two impls outside test
files, both in this workspace: `DialogueFolder`
(`crates/ui/src/dialogue/preview/folder.rs:111`) and Hello, Grid's
`HotPalette` (`examples/hello-grid/src/hot_palette.rs:35`). The others
are in `tests.rs` files. In `docs/modules.md` every module is parked or
planned, apart from the formats `shuyi-assets` carries.

The outside path is planned only after the release. 2526 (outside
modules in `shuyi add`) has `blocked_by: ["2003", "2005"]`, and 2552
(`shuyi module new`) has `blocked_by: ["2526"]`. Nick's rule of 2026-11-14
(`docs/decisions.md`, *no run of breaking versions after the MVP*) puts
every planned break before 2005. So the first outside author finds the
gaps after the freeze, and each fix is a breaking release. Bevy's
`Plugin` and Godot's GDExtension were shaped by outside crates.

**One version for every crate.** 2005's version-policy ADR plans "one
version for `shuyi` and all `shuyi-*` crates", and 2028 says "All crates
share one version". The root `Cargo.toml` has `version = "0.1.0"` under
`[workspace.package]`, and every crate takes `version.workspace = true`.
`Module` lives in `shuyi-ui`. So an outside module names the whole of
`shuyi-ui` to name `Module`, `Ctx` and `GlyphBuffer`. A break anywhere in
`shuyi-ui`, `shuyi-assets` or `shuyi-platform` raises the shared 0.x
minor, and every outside module is then incompatible until its author
publishes again. Bevy has this problem: most third-party plugins break
on each minor, and their READMEs keep compatibility tables. Lockstep 0.x
still works (Bevy's ecosystem lives with it), but the choice has to be
made before 2005. Splitting the traits into a crate of their own after
the release would itself be a break.

The audit proposed a throwaway module. This ticket keeps it as a CI
check instead of deleting it: a probe that is gone guards nothing, and
building it costs one step in the existing `modules` job. It is still
throwaway in the sense that it is never published, never an engine
feature, and nothing in a game uses it.

Whether this ticket and the tickets it files come before the first
release is Nick's call in 3528 (the release's guards). This ticket does
not add itself to 2005's `blocked_by`.

## Nick input

`None.` The question of whether it blocks the release is in 3528.

## Scope

**In:**
- **An outside module and a game that uses it**, in
  `tools/outside-module/`, which is not a workspace member (its own
  `[workspace]` table, like 2020's `fuzz/`):
  - `probe-module/`, a crate `shuyi-probe` that is generic over any
    game (`impl<G: GameData> Module<G>`). It installs a hook
    (`before_update` and `draw_over`), keeps state on the `Ctx`
    (`Install::state`, read back with `Ctx::module_state`), and adds a
    debug entry (`Install::debug_entry`). It adds an asset kind with a
    `Format` and an `ImportSettings`, registered through
    `Formats::register` and `Formats::register_import_settings`. It
    has `[package.metadata.shuyi]` as ADR-0041 §2 says, a README in
    ADR-0009 §4's order, and a skill in `skill/SKILL.md`
    (ADR-0041 §5).
  - `probe-game/`, a small game that it did not write. It installs the
    module with `Game::with_module`, loads one file of the module's
    format, and is driven by the `Harness` in a test. No window.
  - Both name the engine as an outside author would:
    `shuyi = { git = "https://github.com/Zafnok/shuyi-engine", ... }`.
    The CI step and the local command point that at the checkout with
    `--config 'patch."https://github.com/Zafnok/shuyi-engine".<crate>.path=...'`,
    so the files never hold a path into this repository.
- **A CI step** in the `modules` job (`.github/workflows/ci.yml`, the
  job at line 1104). It builds and tests `tools/outside-module` against
  the checkout, and runs `cargo fmt --check` and `cargo clippy` there.
  `cargo fmt --all` and the workspace's clippy don't reach a crate
  outside the workspace. The step is in a job `ci-result` already
  needs, so `ci-result`'s list doesn't change.
- **A friction list.** Each thing an outside crate can't do, or does
  awkwardly, is recorded: a `pub(crate)` item it needs, a type it must
  name from a deep path, a generic it can't satisfy, a `fn` pointer
  where it needs a closure, a feature it must turn on that it shouldn't,
  a second copy of an engine crate in the graph. Each item is fixed
  here if it is a few lines and breaks nothing, or filed as a ticket
  (the `write-ticket` skill). Each filed ticket says whether it breaks
  the API.
- **An ADR on how the extension traits are versioned** (the `write-adr`
  skill). It decides between:
  1. a small crate of extension traits (for example `shuyi-ext`, or the
     traits kept at the bottom of `shuyi-core` and `shuyi-assets`) with
     its own semver track and a promise to break less often than the
     engine. The old paths stay as `pub use`, so no game breaks;
  2. one version for every crate, with each outside module saying which
     engine versions it supports, in its metadata and in a table in its
     README.

  It names the traits it covers, at least `Module`, `Hook`, `Install`,
  `DebugEntry`, `AssetKind`, `Format`, `ImportSettings` and `Platform`.
  The `Exporter` trait (`crates/cli/src/export.rs`) is one of the
  extension traits decided here; 3335 versions only its own
  outside-exporter protocol (`PROTOCOL = 1`). It says how the friction list fed the choice. It cites 2005's version
  policy rather than restating it.
- **If the ADR chooses the split crate,** this ticket moves the traits
  and leaves `pub use` at the old paths. Before moving them, search the
  game for what it implements:
  `gh search code --repo Zafnok/visions-of-shuyi 'impl Module'` and
  `'impl Format'`, with the engine revision the game pins
  (`gh api repos/Zafnok/visions-of-shuyi/contents/shuyi-engine.rev`).
  The completion notes name what the search found. A moved trait that
  needs no change in a game gets a `Behaviour:` line, not a `Breaking:`
  one. New public types follow ADR-0039: one that may grow is
  `#[non_exhaustive]` with a constructor.
- **A new engine crate** (only with the split) adds its line to
  `tools/layers.ron` in this PR (`AGENTS.md` rule 5).
- **Either order with 2005:** if 2005 is done, the ADR amends 2005's
  version-policy ADR, and a split crate is a 0.x break under that
  policy (a `Breaking:` line and a `migration` block), even with the
  `pub use` paths, if the move changes anything a game names; if not,
  nothing to do here (2005's version-policy ADR cites this ADR and
  follows its choice).
- **Either order with 2526:** if 2526 is done, the CI step adds the
  module to the game with `shuyi add --path` in place of an edited
  `Cargo.toml`, and the ADR's choice 2 uses 2526's metadata read; if
  not, nothing to do (2526 tests `shuyi add --path` on
  `tools/outside-module/probe-module` and reads the engine versions it
  supports as this ADR says).
- **Either order with 3528:** if 3528 is done, each friction ticket that
  breaks the API goes where Nick's answer puts it (2005's `blocked_by`,
  or After MVP); if not, they are filed After MVP, and the completion
  notes list them for 3528's answer to place.
- **Either order with 3519:** if 3519 is done, the throwaway outside
  module also adds one module tool (`ModuleTool`, through
  `GameTools::module_tools`) and one option row (`Install::option_row`),
  and the friction list names any friction with them; if not, nothing
  to do (3519 adds them, and its own test module covers them).

**Out (do not do):**
- `shuyi add` of a crate from outside this repository, and the ADR that
  extends ADR-0009 for it: 2526.
- A scaffold for outside modules (`shuyi module new`) and a demo game:
  2552.
- Contract operations, importers, bake steps and settings rows from
  modules, and a backend conformance harness: 3519.
- An exporter from an outside backend: 3335. It versions its own
  protocol.
- Putting the genre kits behind default-on features: 3520.
- A golden row for the probe's format. It is not an engine format, so
  ADR-0007's fixture rule doesn't apply.
- Publishing the probe, or naming it in `docs/modules.ron`.
- Adding this ticket to 2005's `blocked_by`: 3528 decides that.

**Estimated size:** about 1,100 lines (code 500, tests 250, docs 350).

**Split:** past 1,000 lines, split off steps 6 and 7 (moving the traits
into a crate of their own, if the ADR chooses it) into a new ticket.

## Implementation steps

1. Write `tools/outside-module/` with its own `[workspace]`, and
   `probe-module/` and `probe-game/` as in the Scope. Name the engine by
   its git URL. Add the folder to the root `Cargo.toml`'s
   `[workspace] exclude` if cargo needs it.
2. Write the local command in `tools/outside-module/README.md`: the
   `cargo test --manifest-path ... --config 'patch...'` line that points
   the git dependency at the checkout.
3. Add the CI step to the `modules` job: build, test, `cargo fmt
   --check` and `cargo clippy -- -D warnings` on `tools/outside-module`.
4. In `probe-module`'s tests, parse its `[package.metadata.shuyi]` with
   `shuyi_cli::catalog::ModuleInfo` (`crates/cli/src/catalog.rs:155`, a
   dev-dependency), so its metadata is the shape the catalog reads.
5. Keep the friction list as you go. Fix the small, non-breaking items
   in this PR. File a ticket for each of the others.
6. Write the ADR on the extension traits' versioning, citing the
   friction list.
7. If it chooses the split crate: search the game, move the traits,
   leave `pub use` at the old paths, add the crate to
   `tools/layers.ron`, and update `docs/modules.md` and the crates'
   docs.
8. Document the probe and the ADR's rule (the `document` skill): a
   short section in `docs/modules.md` on what an outside module can use
   today, and which engine versions it should name.

## Acceptance criteria

- [ ] `tools/outside-module/` is outside the workspace. `cargo metadata
      --no-deps` at the root lists neither `shuyi-probe` nor
      `probe-game`.
- [ ] The module is generic over `G: GameData`, and installs a hook,
      state on the `Ctx` and a debug entry.
- [ ] Its asset kind loads in `probe-game` through
      `Formats::register`, with its import settings read through
      `Formats::register_import_settings`.
- [ ] `probe-game`'s `Harness` test shows the module's state read by a
      screen through `Ctx::module_state`, its debug entry listed in the
      debug menu, and its hooks named in `Game::modules()`.
- [ ] No file under `tools/outside-module/` names a path into the
      repository's `crates/`.
- [ ] CI's `modules` job builds and tests it, and checks its formatting
      and clippy lints.
- [ ] The completion notes list each friction point, with the fix or
      the ticket filed for it, and whether that ticket breaks the API.
- [ ] An ADR in `docs/adr/` decides between a crate of extension traits
      and one version with stated compatibility, and names the traits
      it covers.
- [ ] If the traits move, every old path still compiles (a test in
      `probe-module` uses one old path), and the new crate has its line
      in `tools/layers.ron`.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: in `probe-module`, its format decodes a small file, refuses a
  broken one with an error, and its metadata parses as a `ModuleInfo`.
- Integration: `probe-game`'s `Harness` test (state, debug entry,
  hooks, the loaded asset).
- Snapshot / integration: none beyond the above; the frame the module
  draws over is checked by reading the scene or the state, not cells
  (`AGENTS.md` rule 9).

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
