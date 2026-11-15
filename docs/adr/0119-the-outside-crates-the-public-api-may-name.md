# ADR-0119: Outside crates in the public API: serde, schemars, and the tools' JSON and versions; every other one behind an engine type or a hidden path

- **Status:** Accepted
- **Date:** 2026-11-14
- **Related tickets:** 2087; 2005 (the first release), 2084 (`SoundProbeBackend`), 2215 (embedding of our own), 3034 (replacing macroquad), 3044 (the window's own changes)

## Context

Nick, 2026-11-14 (`docs/decisions.md`): the engine follows semver, and
after the MVP there is no run of breaking versions. A public signature
that names another crate's type ties the engine's version to that crate's:
a new major version of it, or replacing it, is a breaking release of ours.
The engine plans to replace two crates games named: macroquad, by winit
and wgpu (3034, ADR-0021), and `include_dir`, by embedding of its own
(2215).

The public API listing (`cargo xtask api`'s, ADR-0038: `cargo-public-api
--simplified` of every engine crate, with the features ADR-0038 checks)
was searched on 2026-11-14 for the path of every outside crate an engine
crate depends on (`cargo metadata`'s normal dependencies, 30 crates).
These named one:

| Crate | Where | Items |
| ----- | ----- | ----- |
| `include_dir` | `shuyi-assets` | `bundle::Files::Dir`, `bundle::Layer::dir` |
| `miniquad` (macroquad's) | `shuyi-app` | `window_conf` (`Conf`), `keys::to_key` (`KeyCode`) |
| `quad_snd` | `shuyi-app`, feature `sound-probe` | `SoundProbeBackend`'s methods |
| `serde` (`serde_core`) | every crate with data | `Serialize`, `Deserialize`, `DeserializeOwned`, `Serializer`, `Deserializer` in derived impls and bounds |
| `schemars` | `shuyi-core`, `shuyi-ui`, `shuyi-platform`, `shuyi-cli`, feature `schema` | `JsonSchema`, `Schema`, `SchemaGenerator` in derived impls |
| `serde_json` | `shuyi-cli` | `pub use serde_json`; `Value` in the contract's session and MCP types |
| `semver` | `shuyi-cli` | `template::Template`'s `version` and `engine`, `Template::read`, `template::engine_version` |

No other crate's path is in the listing: not `gilrs`, `image`, `png`,
`ron`, `rhai`, `toml_edit`, `cpal` or the rest. A game of its own shape
was told to write `#[macroquad::main(window_conf)]` and call
`shuyi_app::window_conf` and `shuyi_app::run`, and a new game embedded its
files with `include_dir!`, a dependency of its own.

## Decision

1. **The outside crates the public API may name**, each for good:

   - **`serde`** (1.x): its `Serialize` and `Deserialize` on the engine's
     data, and the bounds that read and write it. Data a game saves,
     loads and sends is the engine's whole way of working (ADR-0002), and
     serde is the vocabulary every Rust crate shares for it; a type of our
     own in its place would make every game's data unreadable by every
     other crate. serde 1 has been stable since 2017.
   - **`schemars`** (1.x), behind the `schema` feature only: `JsonSchema`
     on the same data, so a game's tools describe its shapes to the
     contract (ADR-0084 §2). A shipped game builds no schema, and the
     feature is the tools'.
   - **`serde_json`** (1.x), in `shuyi-cli` only: its deliberate `pub
     use serde_json` (`crates/cli/src/lib.rs`), so a game's tools build
     the scene's JSON with the same crate the contract answers in, and the
     `Value`s of the contract's session and MCP types. The contract is
     JSON (ADR-0022), and `shuyi-cli` is a game's tools, never its program.
   - **`semver`** (1.x), in `shuyi-cli` only: a template's version and the
     engine versions it takes (`template::Template`), the same type Cargo's
     own requirements are read with.

2. **Any other outside type stays behind an engine type or a
   `#[doc(hidden)]` path.** A public item names no other crate's type: not
   in a signature, a field, a variant, a bound, a trait impl, or a `pub
   use`. Where a macro must expand to another crate's code, it does so
   through a `#[doc(hidden)]` re-export, and the hidden item's doc says
   semver doesn't cover it. `cargo-public-api` and `cargo-semver-checks`
   both leave `#[doc(hidden)]` items out, so the listing shows only the
   engine's own.

3. **What this changes now:**
   - `shuyi_app::run_app(config, start)`, a plain `fn`, opens the game's
     window and runs it: a game of its own shape calls it from a plain
     `fn main`, as a game on the short path calls `run_game`, which now
     calls it. `window_conf`, the `async fn run` and `keys::to_key` are
     private. The engine's own render card opens its window with
     `#[doc(hidden)] open_window(&Window, future)`, which names no
     outside type either.
   - `shuyi_assets::files!("<folder>")`, re-exported as `shuyi::files!`,
     embeds a folder as a `bundle::Embedded`, an engine type wrapping the
     embedded directory, which `Layer::dir(name, Embedded)` makes a layer
     of and `Files::Dir` holds. It expands to `include_dir!` through
     `shuyi_assets::__embed`, hidden. A game has no `include_dir`
     dependency. `Embedded` is in `shuyi::prelude` with `std`.
   - `SoundProbeBackend`, the one `quad_snd` path left, is hidden by 2084.

4. **A new outside crate in the public API** is a choice for an ADR that
   amends this one: its crate, why an engine type can't stand in for it,
   and its stability (1.x, or why not).

## Consequences

- Replacing macroquad (3034's follow-ups) and `include_dir` (2215)
  changes no public signature: `run_app`, `run_game`, `files!`,
  `Embedded` and `Layer::dir` keep their shapes over the new backends.
- 3044 changes `window_conf`'s signature as a private function's.
- A game of its own shape and a new game depend on the engine alone, and
  their `main` needs no `async` or attribute macro.
- A major version of serde, schemars, serde_json or semver would be one of
  ours: accepted, as each is a 1.x crate the whole ecosystem would move
  with.
- `files!` names `include_dir`'s crate by a path inside the engine, so a
  game's build has `include_dir` and its proc macro as engine
  dependencies still, until 2215.
- Nothing checks the rule on each PR yet: the listing is searched by
  hand. Ticket 2089 adds the check to `cargo xtask api`.

## Alternatives considered

- **Keep `window_conf` and `run` public, hidden.** A game would still
  write `#[macroquad::main]`, so it still depends on macroquad, and 3034's
  backend would break it.
- **`files!` gives a `Layer`** (`files!("files", "$SHUYI_FILES")`). One call
  less, but a `Layer` built in a `static` can't take the flags a game sets
  with its methods after, and the template's test asks whether a folder
  is empty; `Embedded` keeps both, and `Layer::dir` stays the one way a
  folder becomes a layer.
- **Wrap `serde`'s traits in traits of our own.** Every derive and every
  crate a game uses for its data would need an adapter, for a crate that
  has been 1.x for nine years.
- **Hide `shuyi_cli::template`'s `semver` types.** `shuyi-cli` is a game's
  tools, not its program, and `semver` is how Cargo itself reads versions:
  a type of our own would be a copy of it.
