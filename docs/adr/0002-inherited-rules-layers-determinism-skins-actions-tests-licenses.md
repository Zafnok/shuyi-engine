# ADR-0002: Inherited rules: layers, determinism, skins, actions, tests, licenses

- **Status:** Accepted
- **Date:** 2026-10-14
- **Related tickets:** 2001; the game's 1101–1118, 1131

## Context

The engine is being carved out of a working game. That game's rules are
in its ADRs: crate layers, determinism, graphics as a skin, input by
actions, tests as the review, a strict license policy. These rules are
why its code can be split at all. The game's ADR-0063 decided the split:
each game crate sits on the engine crate of the same layer.

This ADR restates the rules that bind the engine, so this repository
stands on its own. Each section names the game ADR it comes from, for the
full reasoning.

## Decision

### 1. Five crates, one direction

| Crate | Owns | Must not |
| ----- | ---- | -------- |
| `shuyi-core` | Simulations (turn rules, tick steps), the seeded RNG, fixed-point numbers, grids and paths, collision, entity arenas | Do I/O, read a clock, use floats in a simulation, depend on any other engine crate except through traits it defines |
| `shuyi-assets` | The asset bundle and its layers, asset kinds, file formats as plugins (ADR-0007), text by key and languages, dialogue, keymap files, validation that names file and line | Do I/O itself (it is given bytes); contain rules |
| `shuyi-ui` | The frame (glyph cells, rectangles, sprite items, world layers), screens and the screen stack, `Ctx<G>`, input actions, widgets, skins, cinematics, the headless Harness | Touch the platform; read the clock or the keyboard directly |
| `shuyi-app` | Window, drawing, input devices, audio playback, storage, the run loop (macroquad lives only here) | Contain game or screen logic |
| `shuyi-cli` | The `shuyi` tool: the contract operations (ADR-0005) | Be linked into a game |

Dependencies point down the table only. Later crates (`shuyi-net`,
`shuyi-physics`, servers) follow the same rules for their layer. Source:
game ADR-0004, game ADR-0063.

### 2. Determinism

- A simulation's randomness comes from a seeded RNG it owns. Time is an
  argument. No crate but `shuyi-app` reads a clock.
- Simulations use integer and fixed-point numbers only (ADR-0003 §3).
  Floats appear only in drawing.
- Same seed and same inputs give the same state, on every platform.

Source: game ADR-0004, game ADR-0019, ADR-0003.

### 3. State changes in one place

A turn game changes state only through commands: validate, then apply,
then return events. An error changes nothing. A tick game changes state
only by stepping with that step's inputs. The UI animates events; it
never computes outcomes. Bots issue the same commands as players.

Source: game ADR-0004, ADR-0003 §2.

### 4. Graphics are a skin; screens are logic, a view and a skin

- Logic builds plain-data scenes and views. A skin paints them into the
  frame. Nothing about the look is in a simulation, a bot or a record.
- Pictures are sprite items from asset files, never pixels drawn as
  cells.
- A new thing on a map is added to the scene and painted by every skin.

Source: game ADR-0038, game ADR-0054.

### 5. Input is actions

Games react to actions, never keys or buttons. Every key and button is
rebindable, and text names them through the keymap. The `input-actions`
skill has the details. Source: game ADR-0015, game ADR-0031, game
ADR-0034, game ADR-0053.

### 6. Text by key

Player-facing text comes from language files by key; code never holds a
literal sentence a player reads. Source: game ADR-0045.

### 7. Tests are the review

Every change carries the tests its ticket lists:

- unit and property tests for logic;
- Harness and snapshot tests for screens;
- `frame-png` pictures for looks.

Mutation testing runs in CI on every pull request. A gate is never
weakened to pass. Tests of behavior read scenes and state, not cells.
Source: game ADR-0007, game ADR-0047.

### 8. Licenses

- **The engine.** It is MIT OR Apache-2.0.
- **Shipped dependencies and assets** must be under one of these:
  - code: `MIT`, `MIT-0`, `Apache-2.0`, `Apache-2.0 WITH LLVM-exception`,
    `BSD-2-Clause`, `BSD-3-Clause`, `0BSD`, `ISC`, `Zlib`, `BSL-1.0`,
    `Unicode-3.0`, `Unicode-DFS-2016`, `CC0-1.0`, `Unlicense`;
  - fonts: the code licenses, plus `OFL-1.1`;
  - art and audio: `CC0-1.0`, `CC-BY-3.0`, `CC-BY-4.0`.
- **Denied:**
  - copyleft (GPL, LGPL, AGPL, MPL, EPL, CDDL, EUPL, OSL, CC-BY-SA);
  - non-commercial or restricted licenses (CC-BY-NC, CC-BY-ND, PolyForm,
    BUSL, SSPL);
  - anything that needs payment;
  - unknown licenses.
- **Enforcement:**
  - `cargo deny` checks crates (2002);
  - `THIRD_PARTY_ASSETS.md` lists every other third-party file.

Source: game ADR-0013, game ADR-0027.

### 9. Errors and safety

- Libraries return typed errors (`thiserror`).
- There is no `unwrap()` or `expect()` in library code outside tests,
  except where a comment documents the invariant.
- `unsafe` is forbidden outside `shuyi-app`.

Source: game ADR-0004.

## Consequences

- An agent working here needs only this repository to know the rules.
- The game repository's ADRs stay the record of why. If one of them is
  superseded there, this ADR is checked and superseded here if the
  engine follows.

## Alternatives considered

- **Copy every game ADR here** — over sixty records, most about the game;
  agents would read game rules as engine rules.
- **Only link the game's ADRs** — the engine wouldn't stand alone, and a
  game-side change could silently change engine rules.
