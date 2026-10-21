---
id: "2358"
title: "Modules join a running game: an ADR, a Module trait, two hooks and state by type on Ctx, with debug menu entries, all listed in the contract"
type: feature
milestone: After MVP
model: opus-5.5
effort: high
status: todo
blocked_by: ["2013"]
nick_input: none
completed:
---

# 2358 — Modules join a running game

## Context

Found by the 2026-10-20 comparison with Bevy 0.20, Godot 4.7, Unity 6,
Unreal 5.8 and Defold 1.14. Each gives code outside the engine one
supported place to join a running game:

- Bevy: a `Plugin`'s `build` adds resources, systems and assets to the
  `App`, and the engine itself is made of plugins
  (<https://docs.rs/bevy/latest/bevy/app/trait.Plugin.html>);
- Godot: autoloads, nodes created at start that live across scenes and
  that any script reaches by name
  (<https://docs.godotengine.org/en/stable/tutorials/scripting/singletons_autoload.html>);
- Unreal: subsystems, classes the engine creates and ends with the game
  instance or a world, with no change to the engine's own classes
  (<https://dev.epicgames.com/documentation/en-us/unreal-engine/programming-subsystems-in-unreal-engine>);
- Defold: native extensions, declared in `ext.manifest`, with their own
  init, update and final callbacks (<https://defold.com/manuals/extensions/>).

What Shuyi has today (checked 2026-10-20):

- **What a module is, but not how it joins a game.**
  [ADR-0009](../../docs/adr/0009-modules-a-small-default-engine-and-opt-in-crates.md)
  §4 and §6, with 2003, define a module crate, its metadata, the
  catalog and `shuyi add`, which turns on a cargo feature, copies the
  module's skill and prints its README. Nothing says how the module's
  code reaches a running `Game`.
- **A module can't widen the shared types.** ADR-0009 §4: no default
  crate depends on a module. So a module crate can't add a field to
  `Ctx` (`crates/ui/src/ctx.rs:201`) or `GameHooks`
  (`crates/ui/src/game.rs:134`) the way the default engine's features
  do.
- **The extension points are single and fixed:**
  - `GameHooks` has three public fields (`first_key`, `debug`,
    `debug_screens`). Games build it as a struct literal, so a new field
    breaks them (2339 says the same);
  - `Game::step` (`game.rs:293`) updates only the stack, and
    `Game::redraw` (`game.rs:418`) draws only the stack;
  - `DebugMenuScreen::new` (`crates/ui/src/debug.rs:131`) lists only the
    entries the game hands it;
  - `Start::on_system` (`crates/platform/src/app.rs:177`) is one
    `Option<SystemHook<G>>`;
  - `Platform` (`crates/platform/src/platform.rs:189`) has seven
    required methods.
- **Planned tickets widen them one at a time:** 2220, as first
  written, adds `GameHooks::reloaded` (a break for every game that
  builds `GameHooks`) and `Platform::changed_files`; 2315 adds
  `Ctx::debug_view`; 2321 `Ctx::request_frame_cap`; 2322 `Ctx::announce`
  and `Platform::access`; 2325 `Ctx::span`; 3036 `Ctx::unlock` and a
  service through `Platform`. That is fine for the default engine,
  which owns those types. A module can do none of it.
- **Registries built one by one:** `Formats` (2201, done,
  `crates/assets/src/format.rs`), `GameTools` (`crates/cli/src/game.rs`),
  and the frame's module layers and painters (3103, parked).

ADR-0009 §8 sets the limit: "what a module can do is named in types and
in the contract, not hidden in callbacks". So this ticket is not
Bevy's design. A module is a library first: the game's screens call it.
It installs a hook only for what must happen on every frame whatever
screen is on top, and every hook it installs is declared and listed.

Why it matters:

- **The game first.** 2220's hot reload is the first real user: with
  this seam it adds a `Module` hook instead of a `GameHooks` field, so
  Visions of Shuyi's pin bump doesn't break. 2101's script module
  (game#1017) gets a written rule: it stays a library the game's rules
  call.
- **Other games.** Physics (1128), networking (1140), procedural
  animation (2701) and scripts (2101) are all modules. Without one
  agreed seam each invents its own wiring, and other people's crates
  can't reach the frame loop or the debug menu without a fork.
- **The end goal** (2026-10-19, `docs/decisions.md`: "Every new API is
  judged against it"): turning a module on should be one line an agent
  can't get wrong, and none once 2003's `shuyi` crate installs the
  modules its features turn on.

The comparison also proposed `Platform::service::<T>()`, a type-erased
service on the platform. It is not built: a method with a default
already adds a service without touching any backend, and it is easier
to find. The ADR makes that the rule, and a test keeps it.

## Nick input

None. How other people's crates reach `shuyi add` is 2526's, and
whether the website names them is Nick's decision in 2809. The trait
works for any crate either way.

## Scope

**In:**
- **An ADR** (the `write-adr` skill, the next free number) extending
  ADR-0009 §6 and §8 and ADR-0024 §2 and §5. It records:
  1. **A library call is the default.** A kit, a runtime or a module's
     types are called by the screens that use them (the grid kit,
     cinematics, 2101's scripts, 1140's transport). A piece installs a
     `Module` only when it must act on every frame whatever screen is
     on top, or be reached by screens it didn't make, and it lives in a
     crate the default engine can't depend on, or is a game's own piece
     of that kind.
  2. **The default engine keeps plain code.** `shuyi-ui`'s own features
     stay plain fields on `Ctx` and explicit steps in `Game` (2315's
     gizmos, 2321's cap, 2340's shake), so a reader of `Game::frame`
     sees every step. `GameHooks` keeps its three fields and gains no
     more. A new frame-loop hook is a `Hook` variant and a defaulted
     `Module` method, added by the ticket that needs it (2220 adds
     `Reloaded`).
  3. **What a hook may change:** the module's own state (in `self`, or
     its state on `Ctx`) and the context's UI side (a sound request).
     Never the game's data or a stage's state, which change only
     through commands or inputs per step (ADR-0002 §3). A module takes
     no actions (`AGENTS.md` rule 8, ADR-0025): one that needs input
     has a screen, opened by the game or by the module's debug entry,
     which declares its actions. A module that draws builds a
     plain-data view and paints it (rule 9).
  4. **Named, not hidden** (ADR-0009 §8): a module declares its hooks,
     its state and its debug entries in `install`. `Game` calls only the
     hooks a module declared. `Game::modules()` and `run --json` list
     every module, in install order, with what it declared.
  5. **Formats:** a module that adds formats exports a function the
     game calls in its own `formats()` (ADR-0007, ADR-0019). A game's
     `Formats` are built and its data loaded before its `Game` exists
     (`examples/hello-grid/src/lib.rs`: `Hello::load` reads the files
     with `formats()`, and `game(ctx)` makes the `Game` after), so
     `install` comes too late for them.
  6. **System events** stay with `Start::on_system`. `SystemEvent` is
     `shuyi-platform`'s, which `shuyi-ui` can't name. The first module
     that must hear one adds a hook on the platform side.
  7. **The platform stays small:** every method added to `Platform`
     after ADR-0024 has a default, and a device service a backend may
     lack returns an `Option`, as `text_input` does. A backend that
     implements the seven methods of ADR-0024 §2, a private console's
     among them, keeps building.
  8. **The frame's module layers** (3103, parked) are a separate seam
     for frame content, and stay so.
  9. **Alternatives weighed:** Bevy's `App` and `Plugin` as the only way
     in (against ADR-0009 §8, and against the plain structs of ADR-0003
     §4, which stay the default beside the optional ECS of 2107); a
     field on `Ctx` or `GameHooks` per feature (a module can't, and a
     `GameHooks` field breaks games); `Platform::service::<T>()`
     (defaults already do it, and it is harder to find); closures
     registered per hook (they can't be listed); a module's state only
     in `self` (screens can't read it).

  Add its row to `docs/adr/README.md`. Don't edit ADR-0009 or
  ADR-0024; the new one says what it extends.
- **`shuyi_ui::module`** (`crates/ui/src/module.rs`, with `mod tests;`
  in `module/tests.rs`), `no_std` with `alloc`:
  - `pub trait Module<G: GameData>: 'static`:
    - `fn name(&self) -> &'static str`, required: `snake_case` ASCII,
      unique in a game;
    - `fn install(&mut self, install: &mut Install<'_, G>)`, defaulted
      to nothing;
    - `fn before_update(&mut self, ctx: &mut Ctx<G>, dt: f32)`,
      defaulted to nothing;
    - `fn draw_over(&self, ctx: &Ctx<G>, buf: &mut GlyphBuffer)`,
      defaulted to nothing.
  - `pub enum Hook { BeforeUpdate, DrawOver }`, `#[non_exhaustive]`,
    with `Hook::name()` giving `before_update` and `draw_over`.
  - `pub struct Install<'a, G>`, which `Game` hands to `install`:
    - `hook(&mut self, hook: Hook)`: declares a hook;
    - `state<T: Any>(&mut self, value: T)`: puts the module's state on
      `Ctx`;
    - `debug_entry(&mut self, make: fn() -> DebugEntry<G>)`: an entry
      for the debug menu, made each time the menu opens.
  - `pub struct InstalledModule { pub name: &'static str, pub hooks:
    Vec<Hook>, pub states: Vec<&'static str>, pub debug_entries:
    Vec<String> }`, `#[non_exhaustive]`: the states by
    `core::any::type_name`, the entries by their label read at install
    (`DebugEntry::label` with the `Ctx` that `Install` holds).
  - `pub enum ModuleError`, `#[non_exhaustive]`, a `thiserror::Error`
    as `BindError` is (ADR-0002 §9): `SameName { name }` (a module
    installed twice) and `SameState { state, module, other }` (two
    modules put the same type on `Ctx`).
  - `#[non_exhaustive]` follows 2013's ADR (in `blocked_by`).
- **State by type on `Ctx`** (`crates/ui/src/ctx.rs`):
  - a private field holding each module's state, one value per type, in
    install order (its `TypeId`, its type name and a `Box<dyn Any>`),
    and the debug entry makers;
  - `Ctx::module_state::<T: Any>(&self) -> Option<&T>` and
    `Ctx::module_state_mut::<T: Any>(&mut self) -> Option<&mut T>`:
    `None` when no module put a `T` there;
  - the field's type has its own `Debug`, which lists the states by
    type name, so `Ctx` keeps `#[derive(Debug)]`;
  - `ctx.rs`'s top doc comment, "A plain struct: no service locator, no
    globals", gains a sentence: the engine's services are fields; a
    module's state is found by its type, put there by its `install` and
    listed.
- **`Game`** (`crates/ui/src/game.rs`):
  - `Game::with_module(self, module: impl Module<G>) -> Result<Self,
    ModuleError>` and `Game::with_modules(self, modules: impl
    IntoIterator<Item = Box<dyn Module<G>>>) -> Result<Self,
    ModuleError>`: each runs `install`, keeps the module, and redraws,
    so the first frame has the modules' drawing;
  - `before_update` runs in `Game::step`, after the input's update and
    `clock_s`, before the first-key and Debug hooks and the stack's
    update (between `self.held()` and opening the first-key screen), on
    each module that declared it, in install order. It doesn't run in
    `Game::act` (no time passes there) or after a quit;
  - `draw_over` runs in `Game::redraw`, after `self.stack.draw`, on each
    module that declared it, in install order;
  - `Game::modules(&self) -> &[InstalledModule]`.
- **The debug menu:** `DebugMenuScreen::new(ctx, entries)` lists the
  game's entries, then each module's, made from the makers on `ctx`, in
  install order. With no module installed it is as today. A game whose
  `GameHooks::debug` is `None` has no menu, so its modules' entries
  don't show.
- **The contract** (`crates/cli/src/contract/`):
  - `ModuleEntry { name: String, hooks: Vec<String>, states:
    Vec<String>, debug_entries: Vec<String> }` in `session.rs`, with
    `Serialize`, `Deserialize`, `JsonSchema` and `deny_unknown_fields`,
    made from an `InstalledModule` (hooks by `Hook::name`);
  - `Session::modules(&self) -> Vec<ModuleEntry>`, defaulted to none,
    and `Headless` gives its Harness's `game().modules()`;
  - `RunOutput::modules: Vec<ModuleEntry>`, filled by `run`;
  - raise `contract::VERSION` by one, regenerate the schemas, and add
    the version's line to `crates/cli/schemas/versions.json`
    (ADR-0022 §5);
  - `docs/contract.md`: `run`'s row says it lists the modules.
- **Backends stay small:** `crates/platform/tests/it/bare.rs` (a module
  of `tests/it/main.rs`) has `Bare`, a backend that implements only the
  seven methods of ADR-0024 §2 (a silent `Audio`, no text input, a
  `MemoryStorage`), and a test that runs one frame of the toy game on it
  (`App::running(toy::game())`, then `App::frame`). Its doc comment
  says: a method added to `Platform` without a default fails to compile
  here; give it a default instead. `docs/platform-backends.md` gains
  the rule under *The rules*.
- **Either order with 2316** (pause and step in the debug view): if
  2316 is done, the modules' `before_update` follows its pause as the
  stack's update does: none while paused, one with `dt` = 1/60 s per
  step, and one on one frame in `slow` in slow motion; `draw_over` still
  draws every frame. If 2316 isn't done, 2316 does it (an edit to 2316
  says so).
- **Either order with 2315:** its gizmos stay last, over the modules'
  drawing. If 2315 is done, put the `draw_over` loop before its gizmo
  painting and check it in the snapshot. If not, add a line to 2315's
  Scope in this PR: its gizmos are painted after the modules'
  `draw_over` loop.
- **For the game:** additive. No type the game builds changes shape:
  `Game::new`, `GameHooks` and `DebugMenuScreen::new` keep their
  signatures, and `Session::modules` has a default. The game's PR that
  moves the pin needs no change.
- **Size:** if the PR would pass about 600 changed lines (the ADR and
  the regenerated schemas not counted), the debug menu's entries
  (`Install::debug_entry`, the `debug_entries` lists and step 5) go to
  a follow-up ticket written with the `write-ticket` skill (After MVP,
  blocked by 2358). The PR says so.

**Out (do not do):**
- moving any default-engine feature, `GameHooks` field or `Ctx` field
  onto modules;
- a reload hook: 2220 adds `Hook::Reloaded` and `Module::reloaded` (an
  edit to 2220 says so);
- system events for modules (ADR point 6);
- option rows from modules: a module's player setting needs a place in
  the saved settings, and no module has one yet. A follow-up when one
  does;
- screens a module pushes over the game: how a screen over a running
  game takes input is 2367's (parked), and 3036's notice follows 2367;
- formats through `install` (ADR point 5);
- importers, bake steps and check items from modules in `GameTools`,
  and contract ops a module adds (`shuyi <module> <op>`): a follow-up,
  written when the first module needs one (2101's check of scripts, or
  1128's materials);
- `shuyi::enabled_modules` and `shuyi add` (2003, an edit says so), and
  other people's crates in the catalog (2526);
- the short path taking modules: a way for 2511's `BasicGame` to install
  them, and `Module`, `Hook` and `Install` in `shuyi_platform::prelude`.
  2003 adds both when its `enabled_modules` goes into the templates,
  which are built on `BasicGame`;
- the frame's module layers and painters (3103, parked; ADR point 8);
- `Platform::service::<T>()` (ADR point 9);
- modules loaded at run time (ADR-0009, *Alternatives*).

## Implementation steps

1. Read ADR-0009 §4, §6 and §8, ADR-0024 §2 and §5, ADR-0022 §5,
   ADR-0025, 2013's ADR, `crates/ui/src/game.rs`, `ctx.rs`, `debug.rs`,
   `crates/platform/src/app.rs`, `platform.rs`,
   `crates/cli/src/contract/session.rs` and `run.rs`.
2. Write the ADR and its index row.
3. Write `module.rs`: the trait, `Hook`, `Install`, `InstalledModule`,
   `ModuleError` and the state by type, with unit tests.
4. Add the state to `Ctx`, and `with_module`, `with_modules`, the two
   hook calls and `modules()` to `Game`.
5. Add the modules' entries to `DebugMenuScreen::new`.
6. Add `ModuleEntry`, `Session::modules` and `RunOutput::modules`; raise
   the contract's version, regenerate the schemas, and update
   `docs/contract.md`.
7. Add `bare.rs` and the rule in `docs/platform-backends.md`.
8. Write the Harness tests and the snapshot; work the either-order
   clauses (2315's line if it isn't done); run the gates.

## Acceptance criteria

- [ ] The ADR records the nine points above, and the index lists it.
- [ ] Harness: with modules A then B installed, each frame runs A's
      `before_update`, B's, then the top screen's `update`, in that
      order (a log both modules and the screen write to, kept as A's
      state on `Ctx`).
- [ ] Harness: A and B each add one overlay (`GlyphBuffer::add_overlay`)
      and set one glyph on the same cell in `draw_over`. Their overlays
      come after any of the stack's in the snapshot, A's before B's, and
      the shared cell shows B's glyph. The snapshot is committed.
- [ ] A module that overrides `before_update` and `draw_over` but
      declares no hook is never called (a counting module).
- [ ] A screen reads with `ctx.module_state::<T>()` what a module's
      `before_update` wrote; without the module it gets `None`.
- [ ] Installing a module of the same name twice gives `SameName`, and
      two modules putting the same type on `Ctx` give `SameState`
      naming both.
- [ ] The toy game played by the same keys with a module that only
      draws has the same `screen_log`, `audio_log` and `quit_frame` in
      the Harness as without it.
- [ ] `DebugMenuScreen::new` lists a module's entries after the game's;
      with no module its menu is as before.
- [ ] `run --json` lists the modules with their hooks, states and debug
      entries; the schema test passes at the new version.
- [ ] `Bare` builds and runs a frame.
- [ ] `shuyi-ui` still builds for `thumbv7em-none-eabihf`.
- [ ] `examples/hello-grid` and the templates build with no change.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `Install` records hooks, states and entries; both errors;
  `Hook::name` (`crates/ui/src/module/tests.rs`); `module_state` by
  type, `None` for a type not put there (`ctx/tests.rs`); the debug
  menu's order of entries (`debug/tests.rs`).
- Integration: the Harness tests above in `crates/ui/tests/it/`
  (a new `modules.rs`); `Bare` in `crates/platform/tests/it/bare.rs`;
  `Headless::modules` and `run`'s answer in
  `crates/cli/src/contract/session/tests.rs` and `run/tests.rs`.
- Snapshot: the toy's home with the two drawing modules.

## Completion notes

*(Filled in by the session that completes the ticket.)*
