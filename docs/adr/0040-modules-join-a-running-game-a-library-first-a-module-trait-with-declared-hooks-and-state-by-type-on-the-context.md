# ADR-0040: Modules join a running game: a library first, a `Module` trait with declared hooks, and state by type on the context

- **Status:** Accepted
- **Date:** 2026-10-23
- **Related tickets:** 2358 (this), 2372 (debug entries), 2220 (hot reload, the first new
  hook), 2003 (the catalog, `shuyi add` and the modules a feature turns
  on), 2101 (scripts), 1128 (physics), 1140 (networking), 2701
  (procedural animation), 2315 and 2316 (the debug view), 2367 (screens
  over a running game, parked), 3103 (the frame's module layers,
  parked), 2526 (other people's crates)
- **Extends:** ADR-0009 §6 and §8 (turning a module on; named, not
  hidden), ADR-0024 §2 and §5 (the platform trait; the system's events)

## Context

ADR-0009 says what a module is (a crate no default crate depends on),
what it ships with and how `shuyi add` turns it on. It doesn't say how a
module's code reaches a running `Game`.

- **The shared types are the default engine's.** `Ctx`
  (`crates/ui/src/ctx.rs`) and `GameHooks` (`crates/ui/src/game.rs`)
  grow by a field per feature of `shuyi-ui`. A module can't add one: no
  default crate depends on it (ADR-0009 §4).
- **The extension points are single and fixed.** `GameHooks` has three
  fields, and games build it by literal, so a fourth breaks them.
  `Game::step` updates only the screen stack and `Game::redraw` draws
  only the stack. `DebugMenuScreen::new` lists only the game's entries.
  `Start::on_system` is one function. `Platform` has the seven required
  methods of ADR-0024 §2.
- **Planned tickets widen them one at a time.** 2220 as first written
  added `GameHooks::reloaded` (a break for every game) and
  `Platform::changed_files`; 2315, 2321, 2322, 2325 and 3036 add fields
  to `Ctx` and methods to `Platform`. That suits the default engine,
  which owns those types; a module can do none of it.
- **Other engines** each give code outside the engine one place to join
  a running game (the 2026-10-20 comparison in 2358):
  - Bevy: a `Plugin`'s `build` adds resources, systems and assets to the
    `App`, and the engine itself is plugins
    (<https://docs.rs/bevy/latest/bevy/app/trait.Plugin.html>);
  - Godot: autoloads, nodes made at start that live across scenes,
    reached by name
    (<https://docs.godotengine.org/en/stable/tutorials/scripting/singletons_autoload.html>);
  - Unreal: subsystems, made and ended by the engine with the game
    instance or a world
    (<https://dev.epicgames.com/documentation/en-us/unreal-engine/programming-subsystems-in-unreal-engine>);
  - Defold: native extensions with init, update and final callbacks,
    declared in `ext.manifest` (<https://defold.com/manuals/extensions/>).
- **ADR-0009 §8 sets the limit:** "what a module can do is named in
  types and in the contract, not hidden in callbacks".
- **A game's data loads before its `Game` exists.**
  `examples/hello-grid/src/lib.rs`: `Hello::load` reads the files with
  the game's `formats()`, and `game(ctx)` makes the `Game` after.
- **`SystemEvent` is `shuyi-platform`'s**, which `shuyi-ui` can't name
  (ADR-0024 §1).

## Decision

### 1. A library call is the default

A kit, a runtime or a module's types are called by the screens that use
them: the grid kit, cinematics, 2101's scripts, 1140's transport. A
piece installs a `Module` only when it must act on every frame whatever
screen is on top, or be reached by screens it didn't make. And it lives
in a crate the default engine can't depend on, or is a game's own piece
of that kind.

### 2. The default engine keeps plain code

- `shuyi-ui`'s own features stay plain fields on `Ctx` and explicit
  steps in `Game` (2315's gizmos, 2321's frame cap, 2340's shake), so a
  reader of `Game::frame` sees every step.
- `GameHooks` keeps its three fields and gains no more.
- A new frame-loop hook is a `Hook` variant and a defaulted `Module`
  method, added by the ticket that needs it. 2220 adds `Reloaded`.

### 3. What a hook may change

- A hook changes the module's own state (in `self`, or its state on
  `Ctx`) and the context's UI side (a sound request).
- Never the game's data or a stage's state. Those change only through
  commands or inputs per step (ADR-0002 §3).
- A module takes no actions (`AGENTS.md` rule 8, ADR-0025). One that
  needs input has a screen, opened by the game or by the module's debug
  entry, and the screen declares its actions.
- A module that draws builds a plain-data view and paints it
  (`AGENTS.md` rule 9).

### 4. Named, not hidden

`shuyi_ui::module` (`crates/ui/src/module.rs`, `no_std` with `alloc`):

```rust
pub trait Module<G: GameData>: 'static {
    fn name(&self) -> &'static str;                            // snake_case, unique in a game
    fn install(&mut self, install: &mut Install<'_, G>) {}     // declares what it does
    fn before_update(&mut self, ctx: &mut Ctx<G>, dt: f32) {}  // Hook::BeforeUpdate
    fn draw_over(&self, ctx: &Ctx<G>, buf: &mut GlyphBuffer) {} // Hook::DrawOver
}
```

- **`install` declares everything**, on `Install`:
  - `hook(Hook)`: a hook the game calls. `Hook` is `#[non_exhaustive]`
    (`BeforeUpdate`, `DrawOver`; `Hook::name()` gives `before_update`
    and `draw_over`);
  - `state(value)`: the module's state on `Ctx`, one value per type,
    which any screen reads with `ctx.module_state::<T>()` and
    `module_state_mut`, and gets `None` without the module;
  - `debug_entry(fn() -> DebugEntry<G>)`: an entry for the debug menu,
    made each time the menu opens. `DebugMenuScreen::new` lists the
    game's entries, then the modules', in install order. A game without
    a debug menu (`GameHooks::debug` is `None`) shows none. 2372 builds
    this one, after 2358.
- **`Game` calls only the hooks a module declared**, in install order:
  `before_update` in `Game::step` after the input's update and the
  clock, before the first-key and Debug hooks and the stack's update,
  never in `Game::act` (no time passes) or after a quit; `draw_over` in
  `Game::redraw` after the stack, so the later module draws on top.
- **It is listed.** `Game::with_module` and `with_modules` install and
  keep modules, and `Game::modules()` gives an `InstalledModule` each:
  its name, its hooks, its states by `core::any::type_name` and (2372)
  its debug entries by label. `run --json` lists them (`RunOutput::modules`,
  `ModuleEntry`), so an agent sees every hook a game has without reading
  its code.
- **Two mistakes fail at install** with a `ModuleError`: a name
  installed twice (`SameName`), and a type two modules put on `Ctx`
  (`SameState`, naming both).
- `Ctx` stays a plain struct: the engine's services are fields, and a
  module's state is found by its type, put there by its `install` and
  listed.

### 5. Formats are the game's call

A module that adds formats exports a function the game calls in its own
`formats()` (ADR-0007, ADR-0019). A game's formats are built and its
data loaded before its `Game` exists, so `install` comes too late.

### 6. System events stay with `Start::on_system`

`SystemEvent` is `shuyi-platform`'s, which `shuyi-ui` can't name. The
first module that must hear one adds a hook on the platform side, in its
own ticket.

### 7. The platform stays small

Every method added to `Platform` after ADR-0024 has a default. A device
service a backend may lack returns an `Option`, as `text_input` does. A
backend that implements the seven methods of ADR-0024 §2, a private
console's among them, keeps building. `crates/platform/tests/it/bare.rs`
holds such a backend and runs a frame on it, so a required method added
later fails to compile there; `docs/platform-backends.md` says the rule.

### 8. The frame's module layers are another seam

3103's module layers and painters (parked) add content to the frame a
skin paints. They stay a separate seam: a `Module` is about when code
runs, not what a frame holds.

### 9. The alternatives weighed

Bevy's `App` and `Plugin` as the only way in, a field on `Ctx` or
`GameHooks` per feature, `Platform::service::<T>()`, closures registered
per hook, and a module's state only in `self`. Each lost, for the
reason *Alternatives considered* gives.

## Consequences

- Left to the tickets that need them: a reload hook (2220), system
  events for modules (§6), option rows from modules (a player setting
  needs a place in the saved settings), screens a module pushes over a
  running game (2367), importers, bake steps and contract operations
  from modules, and `shuyi add` installing what it turns on with
  `BasicGame` taking modules (2003).

- 2220's reload is a `Module` hook, not a `GameHooks` field, so the
  game's pin bump doesn't break.
- A module from anyone's crate can run each frame, draw over the
  screens, keep state screens read and add debug tools, with no fork
  and no change to `Ctx`, `GameHooks` or `Platform`.
- What runs each frame is still visible: the default engine's steps in
  `Game::frame`, and every module's hooks in `Game::modules()` and `run
  --json`.
- `Ctx` keeps a list of boxed values found by type. A lookup is a short
  scan by `TypeId`, and only modules use it.
- A module's hooks can't change the game's data, by rule, not by type: a
  hook holds `&mut Ctx`, whose `game` field is public. Review and the
  rule in §3 hold it.
- One more concept for a game that turns a module on. A game that turns
  none on sees nothing new: `Game::new`, `GameHooks` and
  `DebugMenuScreen::new` keep their signatures.

## Alternatives considered

- **Bevy's `App` and `Plugin` as the only way in.** Everything, the
  engine included, would be a plugin adding systems to a schedule. It
  hides the order of a frame in registrations (against ADR-0009 §8), and
  it brings the ECS with it, where ADR-0003 §4 keeps plain structs as the
  default beside 2107's optional ECS.
- **A field on `Ctx` or `GameHooks` per feature.** A module can't add
  one (ADR-0009 §4), and a new `GameHooks` field breaks every game that
  builds it.
- **`Platform::service::<T>()`, a service by type on the platform.** A
  method with a default already adds a service without touching any
  backend, and is easier to find in the docs.
- **Closures registered per hook.** They can't be listed by name, so
  `run --json` couldn't say what a module does.
- **A module's state only in `self`.** Screens the module didn't make
  couldn't read it, and a reach by name (Godot's autoloads) needs a
  cast anyway; by type it is checked.
