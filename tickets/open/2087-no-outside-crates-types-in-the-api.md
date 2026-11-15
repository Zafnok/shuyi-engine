---
id: "2087"
title: "Before the first release, no outside crate's type in a game's way in: run_app starts a game of its own shape, an engine macro embeds its files, and an ADR lists the outside crates the API may name"
type: infra
milestone: MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2087 — No outside crate's type in a game's way in, before the first release

## Context

Nick, 2026-11-14 (`docs/decisions.md`): no run of breaking versions
after the MVP (2082 has his words). A public signature that names
another crate's type ties the engine's version to that crate's. A new
major version of the outside crate becomes a breaking release of ours,
and so does replacing it. The engine plans to replace both crates
below: macroquad leaves for winit and wgpu (3034, parked, [ADR-0021](../../docs/adr/0021-the-platform-layer-in-house-clean-room-pieces-in-their-own-crates.md)),
and `include_dir` for embedding of our own (2215, parked).

The audit of 2026-11-14 found two ways in that name them:

- **A game of its own shape starts through macroquad.** `shuyi-app`'s
  docs (`crates/app/src/lib.rs:36-56`) tell it to write
  `#[macroquad::main(window_conf)]` with `fn window_conf() ->
  macroquad::window::Conf`, calling `shuyi_app::window_conf(&WINDOW)`
  (`lib.rs:418`), and then `shuyi_app::run(config, start).await`
  (`lib.rs:476`). The game does this: its `crates/app/src/main.rs` names
  `window_conf` (`gh search code --repo Zafnok/visions-of-shuyi
  'window_conf'`, game pinned at `4ce11a3b`). A new game from the
  template doesn't: it calls `run_game` (`lib.rs:658`), which opens the
  window itself (`lib.rs:698`).
- **A game embeds its files with `include_dir`.** The template depends
  on `include_dir = "0.7"` (`templates/shuyi-default/base/Cargo.toml:23`),
  writes `static FILES: Dir<'static> = include_dir!("$SHUYI_FILES");`
  (`crates/game/src/lib.rs:21,30`) and passes it to
  `shuyi_assets::bundle::Layer::dir`, whose signature names
  `include_dir::Dir` (`crates/assets/src/bundle.rs:276`), as does the
  variant `Files::Dir` (`bundle.rs:66`).

3044 (joining the release) also changes `window_conf`'s signature. This
ticket takes `window_conf` out of the public API, so 3044 changes a
private function.

## Nick input

None.

## Scope

**In:**
- `shuyi_app::run_app(config: AppConfig, start: Start<G>)`, a plain
  `fn` that opens the window and runs the game as `run_game` does. The
  docs' example of a game of its own shape uses it. `window_conf` and
  the `async fn run` stop being public (private, or `#[doc(hidden)]`
  with a doc line that semver doesn't cover them, whichever `run_game`
  and the capture paths need).
- An engine macro that embeds a folder, `shuyi::files!("$SHUYI_FILES")`
  (in `shuyi-assets`, re-exported by `shuyi`). It expands to
  `include_dir!` through a `#[doc(hidden)]` re-export and gives a
  `Layer` or `Files`. `Files::Dir` holds an engine type that wraps the
  directory, so no `include_dir` type is public. The templates drop
  their `include_dir` dependency.
- A sweep of the public API listing (`cargo xtask api`'s, ADR-0038) for
  every outside crate's path in a public signature or a public
  re-export. Each one is either hidden as above, or kept on purpose.
- An ADR (the `write-adr` skill): the outside crates the public API may
  name, each with why (`serde`'s traits on data; `shuyi-cli`'s
  deliberate `pub use serde_json`, `crates/cli/src/lib.rs:72`; whatever
  the sweep finds), and the rule that any other outside type stays
  behind an engine type or a `#[doc(hidden)]` path.
- A `Breaking:` line in the completion notes: the game's `main.rs`
  moves to `run_app`, and its files to `shuyi::files!`. The game's PR
  that moves the pin follows this one.

**Out (do not do):**
- Replacing macroquad (3034's follow-ups) or `include_dir` (2215).
- `SoundProbeBackend` (2084).
- 3044's own changes to the window.

**Estimated size:** about 900 lines (code 400, tests 250, docs 250).

## Implementation steps

1. `run_app` in `crates/app/src/lib.rs`, beside `run_game`, sharing its
   window opening (`lib.rs:698`). Move the docs' example at `lib.rs:36-56`
   onto it, and hide `window_conf` and `run`.
2. The embedding macro and its wrapper type in `crates/assets/src/bundle.rs`;
   `Layer::dir` takes the wrapper. Move the template, hello-grid, the
   examples and the tutorial steps onto `shuyi::files!`, and drop
   `include_dir` from their manifests. The Playdate's table path
   (`Files::Table`) stays as it is.
3. List the public API's outside paths (grep the listing for each
   dependency's crate name). Hide or record each.
4. The ADR, and the `document` skill's places for `run_app` and the
   macro (the book's chapter on starting a game).

## Acceptance criteria

- [ ] No public signature or public re-export of an engine crate names a
      macroquad, `include_dir`, gilrs or quad-snd type (the listing,
      grepped, in the notes).
- [ ] A game of its own shape starts with `shuyi_app::run_app`, without
      writing `#[macroquad::main]` (the docs' example compiles as a doc
      test or an example).
- [ ] A new game from the template has no `include_dir` dependency and
      passes its gates.
- [ ] The ADR lists every outside crate the public API names, with why.
- [ ] The completion notes' `Breaking:` line names what the game's pin
      move changes.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- An example or doc test of a game of its own shape on `run_app`.
- Unit: the embedding macro's `Layer` reads the same files as
  `Layer::dir` did (the bundle tests on a fixture folder).
- The templates' job and the web build of an example: embedding works on
  the web as before.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
