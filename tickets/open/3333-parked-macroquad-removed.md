---
id: "3333"
title: "Parked: macroquad removed: miniquad, sapp-jsutils and the vendored loader gone, the macroquad backend's code and CI runs deleted, and doctor's Linux table, setup-linux.sh and the libraries' audit changed to what winit and wgpu load"
type: infra
milestone: "Engine: in-house platform"
model: opus-5.5 / gpt-6.1-sol
effort: high
status: blocked
blocked_by: ["3332"]
nick_input: none
completed:
---

# 3333 — macroquad removed

> **Parked** (Nick, 2026-11-22, `docs/decisions.md`): written by research 3034, whose research alone he unparked. The build waits for the MVP with the rest of the in-house platform layer (2026-10-19), and for 3079 through 3332.

## Context

3332 made winit `shuyi-app`'s default backend, with macroquad one line
away. ADR-0021 §5 says the old crate leaves in the PR of its last
replacement, and `cargo tree` shows it gone.

[ADR-0136](../../docs/adr/0136-replacing-macroquad-winit-wgpu-and-wasm-bindgen.md)
§16 takes it out once the game has run on the default, as 3079 does for
quad-snd (ADR-0068 §6).

**Start only when** the game's pin includes 3332 and the game has
shipped a build on the winit backend: its ticket from 3332 is done.

## Nick input

None.

## Scope

**In:**
- **From `Cargo.toml`, `Cargo.lock` and every manifest:** `macroquad`,
  `miniquad` and `sapp-jsutils`; `cargo tree -i macroquad` finds
  nothing.
- **Code:**
  - The macroquad backend's code goes: `render.rs`, the macroquad half
    of `keys.rs` and `focus.rs`, `readback.rs`'s macroquad read, the
    macroquad loop in `lib.rs`, and every `cfg` naming the `macroquad`
    feature.
  - The `macroquad` feature stays, doing nothing, as ADR-0068 §8 keeps
    `in-house-sound`.
  - `winit` stays a default that does nothing more than it does today.
- **The web shell:** `mq_js_bundle.js`, `sapp_jsutils.js`, their license
  files, the macroquad `index.html` and the plugins' miniquad halves go,
  and so do their rows in `SHELL_NOTICES` and `THIRD_PARTY_ASSETS.md`, if
  listed.
- **CI:** the `render` and `web-render` jobs' macroquad runs go.
- **Linux's libraries** (ADR-0136 §14):
  - doctor's Linux table (`crates/cli/src/contract/doctor/linux.rs`) and
    `scripts/setup-linux.sh` (2513) drop the build-time GL and X11
    headers macroquad linked (`libgl1-mesa-dev` and its `dnf` and
    `pacman` names; `libx11-dev`, `libxi-dev` and `libxkbcommon-dev`, if
    nothing else needs them).
  - They keep the run-time libraries winit and wgpu open:
    libX11, libX11-xcb, libXcursor, libXi, libxcb, libxkbcommon,
    libxkbcommon-x11, libwayland-client, libvulkan.so.1 and libEGL.so.1.
  - Check every remaining entry against `cargo tree` (alsa for
    `shuyi-sound-out`, udev for gilrs).
  - **Either order with 3071:** if 3071 is done, check
    `crates/cli/export/libraries.ron` for every target against what
    winit and wgpu load (ADR-0136 §14), and update it; if not, nothing
    to do (3071's audit reads the libraries of the backend there is
    then).
- **The docs:**
  - `AGENTS.md`'s rule 5 and *Environment*;
  - `docs/platform-backends.md`'s backends table and every "macroquad"
    in it;
  - `crates/app`'s crate docs and `Cargo.toml` description;
  - `docs/modules.md`'s `shuyi-app` row;
  - ADR-0021 §5's check, that the old crate is gone, recorded in the
    completion notes.
- **One commit holding the removal alone,** so that reverting it brings
  macroquad back. Prove that on a scratch branch: revert it, and see the
  `render` job pass on macroquad, before this merges.

**Out (do not do):**
- quad-snd, gilrs and the sound path: 3079 removed quad-snd, and the
  controllers are 2903's.

**Estimated size:** about 1,100 lines, most of them removed (code 700,
CI and scripts 150, docs 250).

**Split:** past 1,000 counted lines, split off the web shell's files and
the Linux lists into a new ticket.

## Implementation steps

1. Read ADR-0021 §5, ADR-0136 §14 and §16, ADR-0068 §6, 2513's
   `doctor` table and `setup-linux.sh`.
2. The removal commit: manifests, code, the shell's files, CI's runs.
3. The Linux lists, the clause with 3071.
4. The docs.
5. The revert's proof on a scratch branch.

## Acceptance criteria

- [ ] `cargo tree -i macroquad`, `-i miniquad` and `-i sapp-jsutils`
      find nothing, for the host and for `wasm32-unknown-unknown`.
- [ ] `shuyi doctor` on a fresh Ubuntu container with only the new list
      installed reports nothing missing, and `hello-grid` opens there
      under the `render` job's `xvfb` (CI).
- [ ] A game naming `features = ["macroquad"]` still builds (a test).
- [ ] The revert's proof is linked in the completion notes.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- The existing `render`, `web-render` and template jobs, on winit alone.
- Unit: doctor's table (each entry's `apt`, `dnf` and `pacman` names).

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
