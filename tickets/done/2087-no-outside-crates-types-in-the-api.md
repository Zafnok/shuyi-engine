---
id: "2087"
title: "Before the first release, no outside crate's type in a game's way in: run_app starts a game of its own shape, an engine macro embeds its files, and an ADR lists the outside crates the API may name"
type: infra
milestone: MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: done
blocked_by: []
nick_input: none
completed: 2026-11-14
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

- [x] No public signature or public re-export of an engine crate names a
      macroquad, `include_dir`, gilrs or quad-snd type (the listing,
      grepped, in the notes).
- [x] A game of its own shape starts with `shuyi_app::run_app`, without
      writing `#[macroquad::main]` (the docs' example compiles as a doc
      test or an example).
- [x] A new game from the template has no `include_dir` dependency and
      passes its gates.
- [x] The ADR lists every outside crate the public API names, with why.
- [x] The completion notes' `Breaking:` line names what the game's pin
      move changes.
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- An example or doc test of a game of its own shape on `run_app`.
- Unit: the embedding macro's `Layer` reads the same files as
  `Layer::dir` did (the bundle tests on a fixture folder).
- The templates' job and the web build of an example: embedding works on
  the web as before.

## Completion notes

**What was done:**

- `shuyi_app::run_app(config, start)`, a plain `fn`, opens the game's
  window and runs it; `run_game` and `run_dialogue_preview` call it.
  `window_conf`, the `async fn run` and `keys::to_key` (the sweep found
  it naming miniquad's `KeyCode`) are private. The render card opens its
  window with `#[doc(hidden)] open_window(&Window, future)`, which names
  no outside type. The crate doc's example of a game of its own shape is
  a `no_run` doc test on `run_app`.
- `shuyi_assets::files!("<folder>")`, re-exported as `shuyi::files!`,
  gives a `bundle::Embedded` (an engine type wrapping the embedded
  folder, in `shuyi::prelude` with `std`), which `Layer::dir(name,
  Embedded)` takes and `Files::Dir` holds. It expands to `include_dir!`
  through the hidden `shuyi_assets::__embed`. `Embedded::is_empty` answers
  the template's test of an empty private folder. Bundle tests on
  `crates/assets/tests/fixtures/embedded` check the macro's layer reads,
  lists and fingerprints as the hand-built folder and the table do.
- Hello, Grid (both binaries), the template, the book's snippets and the
  ten tutorial steps use `run_app` and `shuyi::files!`, and have no
  `include_dir` or `macroquad` dependency of their own; the steps' locks
  are refreshed. Hello, Grid's dialogue preview shows a folder it can't
  read in its window, as `run_dialogue_preview` does, where it logged it.
- [ADR-0119](../../docs/adr/0119-the-outside-crates-the-public-api-may-name.md):
  the outside crates the public API may name (serde; schemars behind
  `schema`; serde_json and semver in `shuyi-cli`), and the rule for the
  rest.

**The sweep** (`cargo +nightly-2026-07-06 public-api -p <crate>
--simplified` with `API_CRATES`' features, every engine crate, grepped for
each of the 30 outside crates the engine crates depend on, as
`<crate>::` with no `::` or word character before it):

- before: `include_dir` (`Files::Dir`, `Layer::dir`), `miniquad`
  (`window_conf`, `keys::to_key`), `quad_snd` (`SoundProbeBackend`'s 4
  lines), `serde_core`, `schemars`, `serde_json`, `semver`;
- after: `quad_snd` 4 lines (`SoundProbeBackend`, which 2084 hides),
  `serde_core` 1,485, `schemars` 334, `serde_json` 21, `semver` 4. No
  macroquad, miniquad, `include_dir` or gilrs path. The acceptance
  criterion's quad-snd part is 2084's (this ticket's **Out**).

**Deviations:**

- `files!` gives an `Embedded`, not a `Layer` or `Files`: the wrapper
  `Layer::dir` takes, as step 2 says, so a game keeps setting its
  layer's flags with `Layer`'s methods (ADR-0119, *Alternatives*).
- The doc example is a doc test, not an `own_shape` example: it compiles
  with the crate's toy game behind hidden lines.
- `keys::to_key` made private: the sweep found it (no game names it:
  `gh search code --repo Zafnok/visions-of-shuyi 'keys::to_key'`, none).
- `semver` in `shuyi-cli`'s `template` module kept on purpose (ADR-0119
  §1), not in the ticket's list.

**Follow-ups:** 2089 checks the rule on each PR in `cargo xtask api`
(the sweep here was by hand). Handoffs written in 2084 (the `quad_snd`
line), 3044 (`window_conf` is private, `run_app` calls it), 3034
(`run_app`'s signature kept at the switch), 2215 (`files!`, `Embedded`
and `Layer::dir` kept over its embedding) and 2005 (blocked by 2089).

**The game** (pinned at `4ce11a3b`): `gh search code --repo
Zafnok/visions-of-shuyi 'window_conf'` and `'shuyi_app::run'` find
`crates/app/src/main.rs` and `crates/story-preview/src/main.rs`, each
`#[macroquad::main]` with `shuyi_app::run`; `'include_dir'` and
`'Layer::dir'` find no code (a lock file and the copied skill pack).

The game's PR that moves the pin follows this one and changes those two
`main.rs` files, as the first `Breaking:` line says.

Breaking: shuyi-app: window_conf, run: private; a game of its own shape calls `shuyi_app::run_app(config, start)` from a plain `fn main`, with no `#[macroquad::main]`, no `window_conf` and no `.await`, and drops its `macroquad` dependency if nothing else names it.

```migration
since: 0.1.0
what: A game of its own shape starts from a plain main with run_app, which opens its window.
old: "#[macroquad::main(window_conf)] async fn main() { shuyi_app::run(config, start).await }"
new: "fn main() { shuyi_app::run_app(config, start) }"
before: |
  fn window_conf() -> macroquad::window::Conf {
      shuyi_app::window_conf(&WINDOW)
  }

  #[macroquad::main(window_conf)]
  async fn main() {
      shuyi_app::run(config(), start()).await;
  }
after: |
  fn main() {
      shuyi_app::run_app(config(), start());
  }
find: ["#[macroquad::main(window_conf)]", "shuyi_app::window_conf(", "shuyi_app::run("]
```

Breaking: shuyi-app: keys::to_key: private; no game calls it, and nothing changes.

```migration
since: 0.1.0
what: The macroquad key code's translation is the backend's own.
old: shuyi_app::keys::to_key(KeyCode) -> Option<Key>
new: none; the backend's keyboard events carry the engine's Key
before: |
  let key = shuyi_app::keys::to_key(code);
after: |
  // Read keys from the frame's input, as every screen does.
find: ["keys::to_key("]
```

Breaking: shuyi-assets, shuyi: bundle::Layer::dir, bundle::Files::Dir: take an `Embedded`, which `shuyi::files!` gives, not an `include_dir::Dir`; a game embeds with `shuyi::files!` and drops its `include_dir` dependency.

```migration
since: 0.1.0
what: A folder is embedded with shuyi::files!, an Embedded that Layer::dir takes by value.
old: Layer::dir(&'static str, &'static include_dir::Dir<'static>)
new: Layer::dir(&'static str, Embedded)
before: |
  use include_dir::{Dir, include_dir};

  static FILES: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/files");
  let layer = Layer::dir("files", &FILES);
after: |
  static FILES: Embedded = shuyi::files!("$CARGO_MANIFEST_DIR/files");
  let layer = Layer::dir("files", FILES);
find: ["include_dir!(", "use include_dir::"]
```

Book: the chapters that show the embedding (the quick start, the box pusher's steps 1 and 2, editions in *Targets and export*) name `shuyi::files!` and `Embedded`, and *Games and screens*' long form names `run_app`.

**Docs:** rustdoc of `run_app`, `open_window`, `files!`, `Embedded`,
`Layer::dir` and the crates' docs; `docs/formats.md`'s target layers;
`SH3012.md`; the `shuyi-new-game` skill and the template's `AGENTS.md`
(the long form); the `shuyi-cinematic` worked example's anchors, which
were on the template's `include_dir` lines.

**For Nick:** a game starts from a plain `main` now, whatever its shape,
and names neither macroquad nor the crate that embeds its files, so
replacing either later doesn't break it.
