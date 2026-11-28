---
id: "2844"
title: docs/architecture.md shows how a frame and a command flow through the crates
type: infra
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2844 — docs/architecture.md shows how a frame and a command flow through the crates

## Context

The engine is 14 crates in `tools/layers.ron` (plus `xtask`) and 138
ADRs. No page shows how they fit together. A blind-spot audit of the
engine (2026-11) found this, and checking confirms it:

- `docs/` has no architecture page. The only hit for "architecture" is
  `docs/adr/0001-record-architecture-decisions.md`.
- The docs map's *Engine developers* table (`docs/README.md`) has rows for
  the rules, ADRs, research, skills, tickets, the carve, the layers and
  the budgets. None says how a frame or a command moves through the crates.
- Its *What each task reads* rows send `21xx`–`35xx` work to the book's
  chapters, which are written for people making games, and to ADRs whose
  titles are a sentence long.
- The frame's call order exists only in `docs/platform-backends.md`,
  *What one frame does* (line 128), written for backend authors.
- No page in `docs/` or the book has a diagram.

A contributor, human or agent, rebuilds the engine's shape from all of
that before each first change. rust-analyzer's `ARCHITECTURE.md` and
Bevy's crate overview exist for the same reason.

This ticket writes one page for engine developers. It never copies a
fact that has a home elsewhere. It links it, as the docs map's rule 1
says. Its crate graph is generated from `tools/layers.ron` and checked,
as rule 2 says.

Builds on: [ADR-0002](../../docs/adr/0002-inherited-rules-layers-determinism-skins-actions-tests-licenses.md)
(layers, determinism, skins),
[ADR-0032](../../docs/adr/0032-the-layers-as-data-checked-over-cargo-metadata-a-float-ban-in-the-simulations-and-a-new-games-rules-crate-kept-pure.md)
(the layers as data and `cargo xtask check-layers`), ticket 2810 (the
docs map and `cargo xtask check-docs`).

## Nick input

None.

## Scope

**In:**
- A new `docs/architecture.md` for engine developers, with five parts:
  1. **The crates.** A Mermaid graph of the engine crates and the edges
     `tools/layers.ron` allows. It marks the pure crates (`devices:
     false`) and the simulations (`floats: false`). The graph is
     generated between two marker comments. Each crate gets one line on
     what it is for, linking its crate doc.
  2. **One frame.** From the backend's loop to the picture, at the level
     of crates and types, with a file pointer for each step:
     `shuyi-app`'s loop → `shuyi_platform::App::frame`
     (`crates/platform/src/app.rs`) → `shuyi_ui::Game::frame`
     (`crates/ui/src/game.rs`) → `ScreenStack` and `Screen::update`
     (`crates/ui/src/screen.rs`) → the rules → `Screen::draw` into the
     `GlyphBuffer` → the `Frame` → `Platform::present`
     (`crates/platform/src/platform.rs`). The platform's own call order
     stays in `platform-backends.md`. This page links it and never
     lists it again.
  3. **One command.** A turn game's command from an action to a saved
     record: an action from the keymap → the screen builds a command →
     `TurnRules::apply` (`crates/core/src/turn.rs`) → events → `History`
     (`crates/core/src/history.rs`) → the view the screen draws → the
     save and the replay. Also the tick game's input per step:
     `TickInput` and `TickSim` (`crates/core/src/tick.rs`).
  4. **The invariants and their checks.** One table: each invariant of
     `AGENTS.md` rule 5 and ADR-0002 that a check enforces, with the
     check and the CI job. At least: the layers and device crates
     (`cargo xtask check-layers`, job `layers`), the float ban
     (`check-layers`'s `denies_floats`), determinism (the golden tables,
     `cargo xtask golden`, job `determinism` on wasm32), `no_std`
     (`cargo xtask check-no-std`, job `no-std`), no hard-coded keys (the
     engine lint, `shuyi lint`), modules (`cargo xtask check-modules`),
     and the public API (job `api`, ADR-0038). An invariant that nothing
     checks says "nothing yet".
  5. **Read these ADRs first.** About 10 to 15 ADRs that govern today,
     each with one line on what it holds. Start from those the *What each
     task reads* rows name already (0002, 0003, 0005, 0006, 0007, 0008,
     0009, 0019, 0022, 0025, 0032, 0038, 0039, 0060). Name the superseded
     ones a reader may meet by mistake.
- A generator and a check for part 1, in `crates/xtask/src/layers.rs`:
  `cargo xtask check-layers` fails when the generated block in
  `docs/architecture.md` differs from what `tools/layers.ron` gives, and
  says the command that rewrites it. A new crate or edge in `layers.ron`
  then cannot leave the graph stale.
- A row in the docs map (`docs/README.md`, *Engine developers*): "How the
  crates fit together: one frame, one command, the invariants and their
  checks, the ADRs to read first" → `architecture.md`, updated on "a
  change that adds a crate, changes the frame's or a command's path, or
  adds or removes a check of an invariant". `cargo xtask check-docs` fails
  without it.
- `architecture.md` added to the *Read first* cell of the `20xx`, `21xx`,
  `23xx`, `30xx`, `32xx`, `33xx` and `35xx` rows of *What each task
  reads*.
- About 300 lines of Markdown at most, so a session can read it whole.

**Out (do not do):**
- No book chapter, no website page. This is a page for engine
  developers. The book teaches people making games, and nothing of this
  repository's process goes there (docs map rule 4).
- No copy of `platform-backends.md`'s call order, of `layers.ron`'s rules
  or of an ADR's text. Link them.
- No edit to `docs/platform-backends.md`: open PR 532 (the macroquad
  research, ADR-0136) changes it. When that PR's tickets (3325–3333)
  replace the backend, the page's frame part is updated by the docs
  map's row, as with any other change.
- ADR metadata, amended-by blocks and a generated standing-rules page
  are 3516's. Part 5 here is a short hand-picked reading order.
  **Either order with 3516:** if 3516 is done, part 5 links
  `docs/standing-rules.md` and `cargo xtask adrs-for`; if not, nothing to
  do (3516 adds the links to part 5).
- A byte budget on the agents' required reading is 3513's.
- The README as a front door is 2843's. It may link this page; this
  ticket doesn't edit `README.md`.
- No guides (2004), and no cleanup of the ticket citations in rustdoc
  (2034 and 3538).

**Estimated size:** about 900 lines (code 250, tests 150, docs 500),
doubled from the steps.

**Split:** past 900 lines, split off the generated graph and its check
(steps 2 and 3) into a new ticket, and land the page with a hand-drawn
graph marked "until NNNN".

**Either order with 2099 and 3513** (the redundancy sweep of 2026-11-28): a block between
marker comments is read, compared and rewritten by
`crates/xtask/src/generated.rs`, with its CRLF handling, its missing or
misordered markers, and its `--write`/check convention. If 2099 or 3513
is done, use it. If neither is, write `generated.rs` as 3513's Scope
says, and they use it.

## Implementation steps

1. Read `tools/layers.ron`, `crates/xtask/src/layers.rs`,
   `docs/platform-backends.md` (*What one frame does*), `docs/README.md`
   and the ADRs of part 5. Trace one frame of `examples/hello-grid`
   through the code, and one command, and note each file.
2. In `crates/xtask/src/layers.rs`, add `pub fn mermaid(layers: &Layers)
   -> String`. It writes one node per crate in `layers.crates`, one edge
   per entry of each crate's `engine` list, and a class for the pure
   crates and the simulations. The order is sorted, so the output is
   stable.
3. In `layers::run`, read `docs/architecture.md`, find the block between
   `<!-- generated by cargo xtask check-layers: crates -->` and
   `<!-- end generated -->`, and report an error when it differs from
   `mermaid`'s output. Add `cargo xtask check-layers --write` to rewrite
   it, and name that command in the error and in the xtask's usage text.
4. Write `docs/architecture.md`, parts 1 to 5 of the scope, with
   relative links to each file and ADR. Generate part 1 with step 3's
   `--write`.
5. Add the docs map row and the *Read first* links (scope).
6. Run `cargo xtask check-layers`, `check-docs` and `check-links`.

## Acceptance criteria

- [ ] `docs/architecture.md` exists, with the five parts of the scope,
      and is 300 lines or fewer.
- [ ] Its crate graph lists every crate of `tools/layers.ron` and every
      allowed engine edge, and nothing else.
- [ ] Adding a crate or an edge to `layers.ron` without rewriting the
      page fails `cargo xtask check-layers`, with an error naming
      `--write`. After `--write`, it passes.
- [ ] The frame part names each crate and type from the backend's loop
      to `Platform::present` with a file path that exists, and links
      `platform-backends.md` for the call order instead of listing it.
- [ ] The command part follows a `TurnRules` command and a `TickSim`
      step from the input to the record.
- [ ] Each row of the invariants table names a check that exists in
      `crates/xtask/src/gates.rs` or `.github/workflows/ci.yml`, or says
      "nothing yet".
- [ ] The docs map has the page's row, and the seven *Read first* cells
      link it. `cargo xtask check-docs` and `cargo xtask check-links` pass.
- [ ] The completion notes have a `Book:` line saying why not (the
      page is for engine developers, not for people making games).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit (`crates/xtask`): `mermaid` on a small `Layers` with three crates
  gives the expected text, sorted; a pure crate and a simulation get
  their classes.
- Unit: the block check passes on a matching page, fails on a stale
  block, and fails with a clear error when the markers are missing.
- Integration: `check-layers` on the repository passes with the
  committed page.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
