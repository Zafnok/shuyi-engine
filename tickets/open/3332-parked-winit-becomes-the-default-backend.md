---
id: "3332"
title: "Parked: winit becomes shuyi-app's default backend, with macroquad still selectable by one line, the template and the book on wasm-bindgen, and AGENTS.md's web rule changed"
type: infra
milestone: "Engine: in-house platform"
model: opus-5.5 / gpt-6.1-sol
effort: high
status: blocked
blocked_by: ["3327", "3328", "3329", "3331"]
nick_input: none
completed:
---

# 3332 — winit becomes the default backend

> **Parked** (Nick, 2026-11-22, `docs/decisions.md`): written by research 3034, whose research alone he unparked. The build waits for the MVP with the rest of the in-house platform layer (2026-10-19), and for 3079 through 3327–3331.

## Context

By now the winit backend (3326–3331) does on the desktop and the web
everything the macroquad one does. It also does what macroquad couldn't:

- every graphics API;
- the IME everywhere;
- the window facts.

It passes the `render` and `web-render` jobs.

[ADR-0136](../../docs/adr/0136-replacing-macroquad-winit-wgpu-and-wasm-bindgen.md)
§16 moves the default alone, as sound's 3084 does (ADR-0068 §5): one
line to undo. The game then moves its pin to it and runs on it before
3333 removes macroquad.

## Nick input

None.

## Scope

**In:**
- **`shuyi-app`'s default features** become `winit`, in place of
  `macroquad`. `default-features = false, features = ["macroquad"]` goes
  back, and the feature docs say so.
- **Every engine program builds on winit:** the examples, the render
  card, the template's game and the book's steps. Their web builds go
  through `wasm-bindgen` (3330).
- **The template** (`templates/shuyi-default/**`):
  - a new game pins `wasm-bindgen` exactly, as the workspace does;
  - its CI workflow installs the matching `wasm-bindgen-cli` (`cargo
    install --locked`) before its web build;
  - `cargo xtask check-pins` covers the pin.
- **CI:**
  - The jobs that build programs build them on winit.
  - The `render` and `web-render` jobs keep their macroquad runs until
    3333, now as the second backend.
- **`AGENTS.md`:**
  - *Environment*'s line on macroquad's loader becomes the web build on
    `wasm-bindgen`, with the CLI installed with `cargo install --locked
    wasm-bindgen-cli --version <the pin>`.
  - Rule 5's backend list says `shuyi-app` (winit and wgpu).
- **The docs:**
  - `docs/platform-backends.md`'s backends table;
  - `crates/cli/web/README.md`;
  - the book's chapter on the web build and its setup page (what to
    install);
  - `docs/decisions.md` needs nothing: no product choice changes.
- **A `Behaviour:` line** for each change a game sees, among them:
  - the key table's differences from 3327's check;
  - the window's first size (3328);
  - the web build's files: the glue and `_bg.wasm` in place of
    `mq_js_bundle.js`.
- **Either order with 3104:** if 3104 is done and `shuyi-app-3d` still
  draws through miniquad, move it to `shuyi-gpu`'s raw pass (ADR-0136
  §6) in this ticket, with its look test green in the `render` job on
  winit; if not, nothing to do (3104 writes it over `shuyi-gpu`).
- **Either order with 1149:** if 1149 is done, its paste and copy work
  on winit before the default moves: `arboard` on the desktop (check its
  license with `cargo deny`), the browser's `paste` and `copy` events
  through `web-sys` on the web; if not, nothing to do (1149 adds them on
  the default backend of its day).
- **The game:** write the game's ticket that moves its pin to this
  commit and checks the game on winit, on Windows, Linux, the Steam Deck,
  macOS and the web, and link it from the completion notes.

**Out (do not do):**
- Removing macroquad, miniquad, `sapp-jsutils` or the vendored loader
  (3333).
- Changing doctor's Linux table or `setup-linux.sh` (3333).

**Estimated size:** about 900 lines (code 250, CI and templates 250,
docs 400).

## Implementation steps

1. Read ADR-0136 §12, §13 and §16, ADR-0068 §5 and §8, and the template's
   workflows.
2. The default features, and every program built on winit; fix what
   breaks.
3. The template, its CI and `check-pins`.
4. CI's jobs.
5. `AGENTS.md`, the docs and the book.
6. The clause with 3104; the game's ticket.

## Acceptance criteria

- [ ] `cargo build -p hello-grid` builds on winit, and with
      `default-features = false, features = ["macroquad"]` on macroquad
      (a test that the default holds winit and that the line undoes it).
- [ ] A new game made by `shuyi new` builds natively and for the web on
      winit, with its CI green (the template's job).
- [ ] The `render` and `web-render` jobs pass on both backends.
- [ ] `AGENTS.md` no longer says the web build uses macroquad's loader.
- [ ] The game's ticket exists and is linked.
- [ ] The book's chapter is updated (2802), or the completion notes have
      a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the default features (an xtask test, as ADR-0068 §7's).
- The template's job: a new game's native and web builds.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
