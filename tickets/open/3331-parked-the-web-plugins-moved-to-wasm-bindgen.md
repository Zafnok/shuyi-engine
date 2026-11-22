---
id: "3331"
title: "Parked: The web plugins moved to wasm-bindgen: controllers through web-sys, sound output's worklet imported by shuyi-sound-out, the pack's chunks, the page's files, the player's files and the lifecycle"
type: feature
milestone: "Engine: in-house platform"
model: opus-5.5 / gpt-6.1-sol
effort: high
status: blocked
blocked_by: ["3330"]
nick_input: none
completed:
---

# 3331 — The web plugins moved to wasm-bindgen

> **Parked** (Nick, 2026-11-22, `docs/decisions.md`): written by research 3034, whose research alone he unparked. The build waits for the MVP with the rest of the in-house platform layer (2026-10-19), and for 3079 through 3330.

## Context

3330 runs a game on the web with the winit backend, its saves and its
console. The engine's other web plugins are miniquad plugins today:

- JS files that `register_plugin` with macroquad's loader;
- `extern "C"` imports on the Rust side;
- strings and bytes passed as `sapp_jsutils.js` objects.

[ADR-0136](../../docs/adr/0136-replacing-macroquad-winit-wgpu-and-wasm-bindgen.md)
§13 moves each to `web-sys` calls or a small
`#[wasm_bindgen(module = "…")]` import, whichever is less code. After
this, a winit web build does everything a macroquad one does.

## Nick input

None.

## Scope

**In:**
- **Controllers** (`gamepad.js`, `pads/web.rs`): `Navigator::get_gamepads`
  through `web-sys` (the `Navigator` and `Gamepad` features). The same
  pads, buttons and standard mapping reach `shuyi_ui::input::pad`.
- **Sound output** (`sound.js`, `shuyi-sound-out`'s `sys.rs`):
  - The AudioWorklet stays JS, as it must. `shuyi-sound-out` gains a
    feature, `wasm-bindgen`, under which `sys.rs` declares the same
    `shuyi_sound_*` functions with `#[wasm_bindgen(module =
    "/web/sound.js")]`.
  - `shuyi_sound_fill` is exported with `#[wasm_bindgen]`.
  - `shuyi-app`'s `winit` feature turns that feature on.
  - The feature's name stays for good (ADR-0068 §8).
  - shuyi-cli's test that compares the JS's function list with the Rust
    one covers both.
  - Run `cargo xtask sound-parity` on the winit web build: its web cells
    pass as on the macroquad build.
- **The pack's chunks** (`pack-file.js`, `pack_file.rs`), **the page's
  files** (`page-files.js`, `page.rs`) and **the player's files**
  (`player-files.js`, `player_files/web.rs`): each becomes a small import
  of the same JS, its strings and bytes passed by wasm-bindgen, with no
  `sapp_jsutils.js`.
- **Either order with 3043:** if 3043 is done, its `lifecycle.js`
  becomes `visibilitychange` and `pagehide` listeners made with
  `web-sys` (`Document`, `Window`, `PageTransitionEvent`). They run a
  frame from Rust inside the event, the game's save in it, as its
  `wasm_exports.frame()` does today, and its tests pass under `winit`;
  if not, nothing to do (3043 writes its listeners for each backend that
  exists then).
- **Either order with 2601:** if 2601's web backend is done, its
  `http.js` and `crates/app/src/http/web.rs` move the same way, and the
  completion notes say whether it becomes a crate of its own; if not,
  nothing to do (2601 writes it for each backend that exists then).
- **Either order with 2909:** if 2909 is done and planned the video
  through a miniquad JS plugin of ours, move that plugin too; if not,
  nothing to do (2909 plans the video on this web build).
- Every plugin that `crates/cli/web` holds by then moves. The shell
  under `winit` ships none of `mq_js_bundle.js` and `sapp_jsutils.js`.

**Out (do not do):**
- Removing the macroquad plugins: the macroquad web build keeps them
  until 3333.
- The IME on the web (3329).

**Estimated size:** about 1,400 lines (code 800, tests 400, docs 200).

**Split:** past 1,000 lines, split off the page's files and the player's
files into a new ticket.

## Implementation steps

1. Read ADR-0136 §13, `crates/cli/web/README.md`, each plugin and its
   Rust side, ADR-0066 and ADR-0068 (sound's gates and its footprint),
   and wasm-bindgen's guide on importing JS.
2. Controllers.
3. Sound output's feature, the export and the import; the sound-parity
   web cells.
4. The pack, the page's files and the player's files.
5. The clauses with 3043, 2601 and 2909.
6. The docs: `crates/cli/web/README.md`'s files under `winit`, and
   `NOTICES.txt`'s list (`SHELL_NOTICES`) without the vendored files.

## Acceptance criteria

- [ ] A winit web build of the game plays with a controller, with sound
      and music, reads its pack while it plays, takes a dropped page file
      and offers a file, in Chrome and Firefox (manual, in the notes).
- [ ] `cargo xtask sound-parity`'s web cells pass on the winit web build.
- [ ] The `web-render` job stays green on both backends.
- [ ] The winit shell ships no vendored loader file, and `NOTICES.txt`
      lists none.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the gamepad mapping from a `Gamepad`'s buttons; the
  function-list test over both `sys.rs` forms.
- Browser (`tools/web-check`): sound starts after a gesture, a pad is
  read, the pack's chunks arrive, under `winit`.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
