---
id: "3068"
title: "A monotonic clock for the web build: the pacer reads performance.now() through a JS plugin of our own"
type: bug
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: low
status: todo
blocked_by: ["3067"]
nick_input: none
completed:
---

# 3068 — A monotonic clock for the web build

## Context

Found by an analysis of the engine for areas that do the same job twice
(2026-10-28), beside 3067.

`shuyi_app::run` reads its time from `pacing::seconds()`
(`crates/app/src/pacing.rs`). On native that is an `Instant`, which
never goes back. On the web it is `macroquad::miniquad::date::now()`,
which the JS bundle answers with `Date.now() / 1e3`
(`crates/cli/web/mq_js_bundle.js`): the system's time, which jumps when
the clock is set by hand or synced. 3067 (done) made `pacing::seconds()` the
loop's one clock (the pacer, `App::frame`'s time and the perf clock), so
after it every reading of the web build's frame jumps with the system's
time. A jump forward is one frame whose `dt` is the whole jump (timers
and animations skip ahead, the music's reported position leaps); a jump
back is a frame of no time and a music position stuck at 0 until the
clock catches up (3067's Context has the details).

3067 keeps the web on `Date.now()` and puts a monotonic web clock under
**Out**: the browser's `performance.now()` needs a JS plugin of our own,
since the web build uses macroquad's loader and no `wasm-bindgen` (game
ADR-0034). The web build already has one such plugin:
`crates/cli/web/gamepad.js` (the `shuyi_gamepad` plugin), declared on
the Rust side in `crates/app/src/pads/web.rs`, with `shuyi-cli` tests
that keep the two lists of functions and the plugin version equal
(`crates/cli/src/web.rs`, `gamepad_js_provides_exactly_the_functions_the_app_imports`
and `gamepad_js_version_matches_the_app`). This ticket adds a second,
smaller one the same way.

`performance.now()` counts milliseconds from the page's time origin and
never goes back. Every browser the web build runs in has it.

## Nick input

None.

## Scope

**In:**
- **The plugin:** `crates/cli/web/clock.js`, the engine's own code, a
  miniquad plugin named `shuyi_clock`, `version: 1`, written like
  `gamepad.js` (an IIFE in strict mode, a header comment naming its
  Rust side). It provides one function:
  `importObject.env.shuyi_clock_now = function () { return performance.now() / 1000; }`,
  seconds since the page's time origin, as an `f64`.
- **The Rust side:** `crates/app/src/pacing/web.rs` (a `#[cfg(target_arch =
  "wasm32")]` module of `pacing`), written like `pads/web.rs`:
  `PLUGIN_VERSION: u32 = 1`, an `#[allow(unsafe_code)] unsafe extern "C"`
  block declaring `safe fn shuyi_clock_now() -> f64`, and
  `#[unsafe(no_mangle)] pub extern "C" fn shuyi_clock_crate_version() -> u32`.
  `pacing::seconds()`'s web branch returns `web::now()`, a safe wrapper.
- **`unsafe` stays in `shuyi-app` only** (ADR-0002: "`unsafe` is
  forbidden outside `shuyi-app`"). The comments that call `pads/web.rs`
  the crate's only `#[allow(unsafe_code)]` (`crates/app/Cargo.toml`,
  the comment above `[lints.rust]`, and `pads/web.rs`'s comment above
  its `extern` block) name both files: the web build's two JS plugins.
- **The shell:** `crates/cli/src/web.rs`:
  - `SHELL` gains `("clock.js", include_str!("../web/clock.js"))`, and
    `SHELL_NOTICES` gains `("clock.js", Origin::Engine)`;
  - `crates/cli/web/index.html` loads `<script src="clock.js"></script>`
    after `gamepad.js`, with a comment line like the others, so it comes
    after `mq_js_bundle.js` (it calls `miniquad_add_plugin`) and before
    `load(...)`;
  - the tests: make the gamepad tests' checks a loop, or one more test
    each, so `clock.js` and `pacing/web.rs` provide and declare the same
    functions and the same version, and `index.html` loads `clock.js`
    between the bundle and the game. Keep the gamepad tests' names.
- **Docs:**
  - `pacing::seconds`'s doc comment: on the web, the page's
    `performance.now()` through `clock.js`, which never goes back;
  - `crates/cli/web/README.md`: a line for `clock.js` in its *Files*
    list, and a short section *The engine's own `clock.js`* after the
    gamepad one (what it provides, its Rust side, the version rule, the
    load order);
  - `docs/platform-backends.md`: where 3067 says the web reads
    `Date.now()`, it says `performance.now()`.
- **From 3066 (done):** `package` writes a `/*!` header only for an
  `Origin::Vendored` row (`shipped_shell_file`). `clock.js` is
  `Origin::Engine`, so `dist` has it with no header:
  `package_copies_wasm_and_writes_the_shell` checks it is `shell(name)`
  alone, as it does `gamepad.js`.
- **For the game:** no API change (`pacing` is private, the two exports
  are the loader's). The game's web build gets `clock.js` from
  `shuyi web` at its next pin bump, with no code change. No `Breaking:`
  line (ADR-0038 §6).

**Out (do not do):**
- **The audio's random seed** (`miniquad::date::now().to_bits()`,
  `crates/app/src/backend.rs`): it should differ each launch, which the
  system's time does. It stays (3067 keeps it too).
- macroquad's own `get_time()` and `miniquad::date::now()`: the engine
  stops reading them for frame time (3067), and the bundle stays as it
  is vendored.
- The other web plugins planned elsewhere (`lifecycle.js`, `motion.js`,
  `languages.js`, `net.js`, …): each adds its own file and its own row
  to `SHELL`; nothing to settle.

## Implementation steps

1. Read 3067's completion notes, `pacing.rs`, `pads/web.rs`,
   `gamepad.js`, and `web.rs`'s `SHELL`, `SHELL_NOTICES` and gamepad
   tests.
2. Write `clock.js` and `pacing/web.rs`, and point `pacing::seconds`'s
   web branch at it.
3. Add the file to `SHELL`, `SHELL_NOTICES` and `index.html`, and the
   tests.
4. Update the comments on `unsafe` and the docs Scope lists.
5. Build the web build of Hello, Grid (`cargo shuyi web` in
   `examples/hello-grid`) and open it in a browser: it runs and steps as
   before, the console shows no plugin warning, and
   `wasm_exports.shuyi_clock_crate_version()` is 1. In a hidden pane
   where `requestAnimationFrame` doesn't run, drive frames with
   `wasm_exports.frame()`.
6. Run the gates (the `run-gates` skill, the examples' web build among
   them).

## Acceptance criteria

- [ ] `grep -rn "date::now" crates/app/src` finds only the audio seed
      (`backend.rs`, `.to_bits()`).
- [ ] `clock.js` provides exactly the `shuyi_clock_*` functions
      `pacing/web.rs` declares, with the same version, and `index.html`
      loads it between `mq_js_bundle.js` and the game (tests in
      `crates/cli/src/web.rs`).
- [ ] `cargo shuyi web --release` in `examples/hello-grid` writes
      `clock.js` to `dist`, and the page runs with no console error.
- [ ] `NOTICES.txt` in `dist` lists `clock.js` as the engine's own.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit (`crates/cli/src/web.rs`): the functions, the version and the
  load order of `clock.js`, as for `gamepad.js`.
- Manual: the web build in a browser (step 5).

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
