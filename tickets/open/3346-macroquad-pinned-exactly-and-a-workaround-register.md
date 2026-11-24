---
id: "3346"
title: "macroquad and the crates its vendored JS serves are pinned exactly, every miniquad workaround is in one register, and no new code passes sapp_jsutils objects"
type: infra
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: ["3034"]
nick_input: none
completed:
---

# 3346 — macroquad pinned exactly, a register of its workarounds, and no new sapp_jsutils use

## Context

The macroquad research (3034, PR #532) decided how macroquad leaves the
engine: [ADR-0136](../../docs/adr/0136-replacing-macroquad-winit-wgpu-and-wasm-bindgen.md)
and parked tickets 3325–3333, all waiting for 3079. Until the last of
them, macroquad stays the backend, possibly for a long time. A
blind-spot audit of the engine (2026-11-23) proposed five pieces for
that exit. Checked against PR #532, three are already settled:

- **One lowered draw list for every GPU painter:** `shuyi_gpu::Frame`,
  the list of passes and quads (ADR-0136 §2), is that list, and 3104,
  2326–2328 and 2331 build on it (§6, §8).
- **One engine-owned JS bridge:** 3330 and 3331 move every plugin that
  `crates/cli/web` holds by then to `web-sys` or a small
  `#[wasm_bindgen(module = …)]` import.
- **A separate host crate, and a winit with softbuffer fallback:**
  ADR-0136 rejected a second backend crate (games depend on `shuyi-app`,
  and a feature keeps the switch to one line, as sound's did). A machine
  with no GPU gets wgpu's fallback adapter (WARP, lavapipe) or a reason
  in the log (§3).

What PR #532 leaves open is the time **before** that exit:

1. **The vendored JS and the Rust it serves can drift apart in a
   game.** `crates/cli/web/mq_js_bundle.js` is macroquad 0.4.16's loader
   with a local patch (3307), and its `gl.js` part checks miniquad's
   version at load (`"Version mismatch: gl.js version is …"`, a console
   error, which fails CI's `web-render` job). The same bundle carries
   the `macroquad_audio` plugin that quad-snd 0.2.8's
   `src/web_snd.rs` imports (`audio_init`, `audio_add_buffer`, …) and a
   copy of `sapp_jsutils`'s. Yet the requirements are loose:
   - the root `Cargo.toml`: `macroquad = { version = "0.4", … }`;
   - `crates/app/Cargo.toml`: `quad-snd = "0.2.8"`, `sapp-jsutils = "0.1"`.

   macroquad 0.4.16 itself pins `miniquad = "=0.4.11"`. The engine's own
   `Cargo.lock` holds 0.4.16, 0.4.11, 0.2.8 and 0.1.7, and so does the
   game's today (at engine revision `50472bbc`). But a game's lock
   resolves on its own: `cargo update` there, or a new game made by
   `shuyi new`, takes a macroquad 0.4.17 the day it ships, while
   `shuyi web` keeps writing the 0.4.16 loader. Nothing checks it.
2. **The workarounds are scattered.** Each is written where it lives,
   and the `dependency-upkeep` skill names two. Those found so far:
   - `crates/app/src/drops/windows.rs`: subclasses miniquad's window
     (class `MINIQUADAPP`) because miniquad 0.4.11's `WM_DROPFILES`
     unwraps a failed read ([ADR-0104](../../docs/adr/0104-drops-on-windows-read-by-the-backend-itself-through-a-subclass-of-miniquads-window.md)).
     `crates/app/src/window/win32.rs` finds the window through it too.
   - `mq_js_bundle.js`'s local patch: a dropped file's name sized in
     UTF-8 bytes (3307, `crates/cli/web/README.md`).
   - `crates/cli/web/index.html`: `var register_plugin;` before the
     bundle (its unused `quad_net` plugin assigns the global); resuming
     the page's audio on every gesture (the bundle's unlock gives up
     after the first key); the canvas taking focus inside a site's frame
     (3505); the key backstop for Page Up, Page Down, Home and End.
   - `crates/app/src/pointer.rs`: the web's wheel is `-deltaY`, as the
     bundle negates it.
   - `crates/app/build.rs` and every game's `build.rs`:
     `--allow-undefined`, because the plugins' functions are imports.
   - `deny.toml`: RUSTSEC-2025-0035 and RUSTSEC-2026-0192, both "revisit
     on macroquad upgrade".

   When 3330 to 3333 take macroquad out, each of these is either deleted
   or carried over, and today nothing lists them in one place.
3. **New web code still lands on miniquad's ABI.** Open tickets add
   plugins: 3043 (`lifecycle.js`), 3068 (`clock.js`), 2522 (`net.js`),
   2528 (`dev.js`). Each is code 3331 must move. Three Rust files pass
   `sapp_jsutils` objects today (`pack_file.rs`, `page.rs`,
   `player_files/web.rs`); `storage/web.rs` and `pads/web.rs` pass
   bytes at a pointer and a length, which moves to wasm-bindgen with
   less change. None of the open tickets plans `sapp_jsutils`; a check
   keeps it so.
4. **Rule 12's way out is unwritten for macroquad.** A blocking bug in
   macroquad or miniquad is fixed here (`AGENTS.md`, rule 12). A
   `[patch.crates-io]` in this workspace reaches no game, because Cargo
   reads `[patch]` only at the root of the workspace being built, which
   is the game's. So the way is a workaround in `shuyi-app` (as
   `drops/windows.rs` is) or a local patch to the vendored JS (as 3307's
   is). Nothing says so.

## Nick input

None.

## Scope

**In:**
- **Exact pins.** `macroquad = "=0.4.16"` in the root's
  `[workspace.dependencies]` (miniquad follows, being exact in
  macroquad), `quad-snd = "=0.2.8"` and `sapp-jsutils = "=0.1.7"` in
  `crates/app/Cargo.toml`. Each gets a comment: the vendored JS in
  `crates/cli/web` is from this version.
  - The template names none of them (`templates/shuyi-default` reaches
    macroquad through `shuyi-app`), so `check-pins` rule 4 is unchanged.
  - Dependabot's `backend` group moves exact requirements as it moves
    the others; the `dependency-upkeep` skill's `backend` step says the
    PR moves the requirement, the vendored files and the register
    together.
- **A check that the pins and the vendored files agree**, as rule 5 of
  `cargo xtask check-pins` (`crates/xtask/src/pins.rs`):
  - each of `macroquad`, `quad-snd` and `sapp-jsutils` has an `=`
    requirement;
  - the version in it equals the one the vendored file's row names:
    `mq_js_bundle.js` for macroquad (and quad-snd, whose plugin it
    carries), `sapp_jsutils.js` for sapp-jsutils. Read the rows where
    they are kept: `SHELL_NOTICES` in `crates/cli/src/web.rs` and the
    version lines of `crates/cli/web/README.md`.
- **The register**, a new `crates/app/WORKAROUNDS.md`: one table, a row
  per workaround or local patch for macroquad, miniquad, quad-snd or
  `sapp_jsutils`, with:
  - the file and a marker string found in it;
  - what it works around, and the version it was seen in;
  - the ticket or ADR that added it;
  - what removes it: an upstream release that fixes it (and what to
    check), or the ticket of 3325–3333 that ADR-0136 §16 gives that
    piece, 3333 when no earlier one does;
  - the test that fails if it is lost, where one exists.

  A second table lists every miniquad JS plugin (a `crates/cli/web`
  file that calls `miniquad_add_plugin`), its Rust side, and whether it
  passes `sapp_jsutils` objects. Under the tables, a short section says
  how a blocking bug in macroquad or miniquad is handled under rule 12:
  a workaround in `shuyi-app` or a local patch to the vendored JS, never
  a `[patch]` (it reaches no game), with a row added here.
- **A check that keeps the register whole**, as rule 6 of `check-pins`:
  - every row's file exists and holds its marker;
  - every `crates/cli/web/*.js` that calls `miniquad_add_plugin` has a
    plugin row, and each row's file still does;
  - every `.rs` under `crates/` (`target` skipped) that names
    `sapp_jsutils` is one of the plugin rows marked as passing its
    objects. A new file fails with a message: pass strings and bytes at
    a pointer and a length, as `storage/web.rs` does.
- **Either order with 3043, 3068, 2522 and 2528:** if one of them is
  done first, its plugin gets its row here; if not, that ticket adds its
  own row, and rule 6 fails until it does.
- **Either order with 3052:** if 3052 is done first, a workaround it
  moved from `index.html` into `shell.js` gets its row at the new place;
  if not, 3052 moves the row with the code, and rule 6 fails until it
  does.
- The docs map (`docs/README.md`) gets the register's row; the
  `dependency-upkeep` skill's `backend` step and `crates/cli/web/README.md`
  point at the register instead of naming workarounds one by one.

**Out (do not do):**
- Moving any plugin to wasm-bindgen, or any other piece of 3325–3333.
- Splitting `shuyi-app` into a host crate and a macroquad crate: ADR-0136
  keeps one crate with two backends behind features.
- A winit with softbuffer backend: ADR-0136 §3 decides the no-GPU case.
- Changing any workaround's code, or re-vendoring the JS.
- Editing 3325–3333 (PR #532's files), `docs/platform-backends.md` or
  ADR-0136: the register is the one new place, and those name it later
  on their own.
- A `[patch.crates-io]` or a fork of macroquad.

**Estimated size:** about 700 lines (code 250, tests 250, docs 200).

## Implementation steps

1. Read ADR-0136 (§12, §13, §16), ADR-0104, ADR-0037,
   `crates/cli/web/README.md`, the `dependency-upkeep` skill and
   `crates/xtask/src/pins.rs`.
2. Find every workaround: start from the list in *Context*, then search
   `crates/app`, `crates/cli/src/web.rs`, `crates/cli/web`,
   `crates/sound-out` and `deny.toml` for `miniquad`, `macroquad`,
   `quad-snd`, `sapp`, `bundle` and comments that say a library gets
   something wrong. Write each as a row of `crates/app/WORKAROUNDS.md`.
   A row the search finds beyond the list in *Context* goes in the
   completion notes.
3. Write the plugin table: `storage.js`, `gamepad.js`, `sound.js`,
   `player-files.js`, `page-files.js`, `pack-file.js` and the Rust file
   each pairs with (the README's sections name them).
4. Pin the three crates exactly, with the comments; `cargo update -p`
   must leave `Cargo.lock` unchanged.
5. `pins.rs` rule 5: parse the requirements from the two manifests and
   the versions from `SHELL_NOTICES` and the README, and compare.
6. `pins.rs` rule 6: read the register's tables and check them against
   the files, as *Scope* says. Keep the module doc's numbered list of
   rules current.
7. The docs: the docs map row, the skill's `backend` step, the web
   README's pointer, and rule 12's way out in the register.
8. Run `cargo xtask check-pins`, then the gates.

## Acceptance criteria

- [ ] The root `Cargo.toml` requires `macroquad = "=0.4.16"`, and
      `crates/app/Cargo.toml` requires `quad-snd = "=0.2.8"` and
      `sapp-jsutils = "=0.1.7"`; `Cargo.lock` is unchanged.
- [ ] `cargo xtask check-pins` fails when the macroquad requirement is
      `"0.4"`, and when it is `"=0.4.17"` while the bundle's row says
      0.4.16.
- [ ] `crates/app/WORKAROUNDS.md` has a row for each workaround listed
      in *Context*, each naming what removes it.
- [ ] `check-pins` fails on a row whose marker is gone from its file, on
      a new `crates/cli/web/*.js` that calls `miniquad_add_plugin` with
      no row, and on a new `.rs` file that names `sapp_jsutils`.
- [ ] The register says how a blocking macroquad or miniquad bug is
      handled under rule 12, and why a `[patch]` here reaches no game.
- [ ] The docs map has the register's row, and the `dependency-upkeep`
      skill's `backend` step points at it.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit (`crates/xtask/src/pins.rs`, over fixture trees as its tests do):
  - rule 5: an exact pin equal to the vendored row passes; a loose
    requirement fails; an exact pin of another version fails;
  - rule 6: a register that matches its files passes; a missing marker,
    an unlisted plugin and an unlisted `sapp_jsutils` file each fail
    with the file named.
- Integration: `cargo xtask check-pins` passes on the repository.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
