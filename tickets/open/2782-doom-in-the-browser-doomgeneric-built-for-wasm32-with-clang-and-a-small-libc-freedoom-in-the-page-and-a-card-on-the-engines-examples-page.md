---
id: "2782"
title: "DOOM in the browser: doomgeneric built for wasm32 with Clang and a small libc of the port's own, Freedoom in the page, and a card on the engine's examples page"
type: feature
milestone: "Engine: demos"
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: ["2780"]
nick_input: none
completed:
---

# 2782 — DOOM in the browser

## Context

Nick, 2026-11-03 ([`docs/decisions.md`](../../docs/decisions.md),
*Bad Apple and DOOM*). 2780 runs DOOM on the engine on the desktop,
from `github.com/Zafnok/shuyi-doom` (GPL). The web is where people
will play it: one click from the engine's site.

What makes the web harder than the desktop:

- **The web build uses macroquad's own loader, not `wasm-bindgen`**
  (`AGENTS.md` *Environment*, game ADR-0034): the target is
  `wasm32-unknown-unknown`, which has no C library. doomgeneric needs
  one: `malloc` and `free`, string and memory functions, `printf` and
  `sprintf`, `atoi`, `qsort`, a few `ctype` and `math` calls, and file
  functions (the port's memory file layer from 2780 replaces most of
  those);
- **C for wasm32 needs Clang** (`--target=wasm32-unknown-unknown`); GCC
  doesn't target it. CI installs it; Nick's machine has MSYS2, where
  `pacman -S mingw-w64-x86_64-clang` adds it (the README says so);
- **the WAD**: Freedoom Phase 1 is tens of megabytes. It is fetched by
  the page beside the `.wasm` (not embedded in it), and handed to the
  game before its first frame.

## Nick input

None.

## Scope

**In** (in `Zafnok/shuyi-doom`, unless it says this repository):
- **A small libc of the port's own** (`libc-wasm/`, C and Rust): the
  functions doomgeneric calls and nothing more, found by linking and
  reading the undefined symbols. `malloc`, `calloc`, `realloc` and
  `free` go to Rust's global allocator through `extern "C"` functions;
  `printf` and its kin format into the engine's log; math is
  `libm` (`MIT OR Apache-2.0`; check) if any is needed. Its headers
  are minimal ones of its own, not a copied libc. Check the license of
  anything taken, and prefer writing the function. **Either order with
  2783:** if 2783 is done, nothing to do (it used the SDK's newlib on
  the device); if not, nothing to do (2783 may take this libc).
- **The build**: `build.rs` compiles the C with Clang for wasm32;
  the port's web build goes through the engine's `shuyi export web`
  (3004), or the engine's web build steps if export can't take a C
  build. If the engine's web build can't take a crate that compiles C
  (an extra object, a linker flag), write the engine ticket for it
  (the `write-ticket` skill, in this repository) and block on it.
- **The WAD in the page**: the page fetches `freedoom1.wad` with a
  download progress line of the port's own and hands its bytes to the
  game before its first frame, through a small JS plugin of the port's
  own, made as the engine's are (`crates/cli/web/gamepad.js`, a version
  the loader checks).
- **From 3092 (done):** a WAD dropped on the page's canvas starts the
  game with it (the player's own `doom2.wad`, say), and the page says
  so: the game's system hook hears `SystemEvent::FilesDropped` (a file
  up to 256 MB), where `ctx.platform.can_drop_files()`.
- **Pages**: the port's own GitHub Pages site plays it, with the GPL
  notice and a link to the source on the page (the GPL asks that the
  source be offered with the program).
- **This repository**: a card in `site/content/examples/` that links to
  the port's page and says it is a separate, GPL project using the
  engine; the PR moves the ticket to `done/`.
- **wasm32 demo test**: **Either order with 2781:** if 2781 is done,
  add a wasm32 run of its demo test to the port's CI, with the same
  hashes; if not, nothing to do (2781 adds it).

**Out (do not do):**
- Embedding the WAD in the `.wasm`.
- Music (2784, parked).
- Touch controls on phones (2407, parked).
- Hosting the port's page on the engine's site: it is a link out, so the
  engine's site serves only permissive code.

## Implementation steps

1. Read 2780's port, the engine's web build (`crates/cli/src/web.rs`,
   its JS plugins) and 3004's `shuyi export web`.
2. Link doomgeneric for wasm32 with Clang against nothing, list the
   undefined symbols, and write the libc for them.
3. The build through the engine's export; the JS plugin for the WAD.
4. The drop (3092).
5. The port's Pages site; play Freedoom's first map in a browser.
6. The wasm32 demo test, if 2781 is done.
7. The card and the PR here.

## Acceptance criteria

- [ ] DOOM plays Freedoom Phase 1 in Chrome and Firefox from the port's
      Pages site: menus, a map played through, sound, saves (in the
      browser's storage).
- [ ] The libc is the port's own, small, and every function in it has
      a test on native.
- [ ] The page offers the source (a link to the repository) beside the
      game.
- [ ] The engine's examples page links to it, marked as a separate GPL
      project.
- [ ] A dropped WAD plays (3092).
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass, here and the port's own.

## Tests required

- Unit (in the port): each libc function against the platform's own on
  native (formatting, memory, strings, `qsort`).
- Snapshot / integration (in the port): a headless wasm32 run of
  `DEMO1` for 1,000 tics with the same scene as native (under
  `wasmtime` or Node in CI, if the engine's headless platform builds
  there; the completion notes say how it ran).

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
