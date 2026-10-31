---
id: "1129"
title: "The tool contract: shuyi new, check, run, scene, replay, bots, import, doctor, with JSON output"
type: feature
milestone: "Engine: public"
model: opus-5.5 / gpt-6.1-sol
effort: xhigh
status: done
blocked_by: ["game#1131", "2201"]
nick_input: none
completed: 2026-10-20
---

# 1129 — The tool contract

## Context

This is ADR-0005 §1–§2. The research found that REA (MIT) grew by
wrapping tools people already trust behind **one contract exposed three
ways**: a CLI with JSON output, an MCP server, and Agent Skills. Each
answer says what it couldn't determine.

This ticket builds the contract and the CLI surface. The MCP server is
1133 and the skills are 1134. `frame-png`, `lint` and `web` already moved
in 1117.

## Nick input

None.

## Scope

**In:**
- `shuyi-cli` operations as library functions with serde input and
  output types, and `shuyi <op> [--json]`:
  - `new <template>`: makes a new game from `templates/grid` or
    `templates/blank`. The game has its crates, one screen,
    `shuyi.ron`, an `AGENTS.md` and CI with the gates. Skills are
    copied once 1134 exists.
  - `check`: every error from every validator, with file, line and
    message.
  - `run --script "<keys>" [--frames n]`: reports the top screen, the
    scene dump, the events and the sounds started.
  - `scene`: dumps the current scene or stage.
  - `replay <record>`: gives hashes per step and the first step where two
    runs differ.
  - `bots <n>`: gives outcomes and measures.
  - `import <file> [--print]`.
  - `doctor`: checks Rust, targets, MSYS2 on Windows GNU, and the wasm
    target, and says how to fix each.
- Every result has a `limits: [..]` field ("no frame drawn: the screen
  quit on frame 3").
- JSON schemas for the inputs and outputs, generated and checked in.
  A test fails if they change without a version bump.

**Out (do not do):** MCP (1133); skills (1134); questions, templates
made of parts and templates from outside the engine (2501–2505,
ADR-0013). Keep the two templates as plain folders; 2501 turns them into
parts.

## Implementation steps

1. Define the types and the schema test.
2. Implement each operation over the Harness, the importers and the
   validators.
3. Write the templates. CI runs `shuyi new` into a temporary folder,
   then builds and tests the result.

## Acceptance criteria

- [x] CI makes a game with `shuyi new grid`, builds it, and its tests
      pass.
- [x] Each operation has an integration test on `hello-grid` (1118).
- [x] The schema test passes.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Integration: one test per operation, and the template build.

## Completion notes

Done 2026-10-20. ADR-0022 records the choices; `docs/contract.md` is the
page for users.

**What was built**

- `shuyi_cli::contract`: one module per operation (`new`, `check`, `run`
  and `scene`, `replay`, `bots`, `import`, `doctor`), each a function from
  a serde input to an `Answer<T>`: the contract's version, the op, `ok`,
  the result or the error, and `limits`. `shuyi <op> [--json]` prints it
  for people or as one JSON document; the exit code is 0, 1, or 2 for
  wrong arguments. `new`, `doctor` and `schema` need no project.
- The schemas of every input and answer, generated with `schemars` (MIT)
  and checked in under `crates/cli/schemas/`, with `versions.json`
  listing each contract version's fingerprint. The schema test fails when
  the files are stale, and when they changed but `contract::VERSION`
  didn't. `shuyi schema` prints or writes them.
- `GameTools` gained `check`, `session`, `replay`, `bots` and
  `importers`, each with a default, so the game's own tools keep building.
  `run` and `scene` drive a `Session`; `Headless<G>` is one over any
  Harness. The Harness now logs frames: `frame_count`, `screen_log`,
  `quit_frame`, `audio_log`, `music_log`.
- `shuyi_core::state_hash`: a hash of any `Serialize` state in a fixed
  form of the engine's own (no_std), which `replay` compares step by step.
- `templates/grid` and `templates/blank`, embedded in the binary. Each
  game has its crates, one screen, `shuyi.ron`, `AGENTS.md` (and
  `CLAUDE.md`), CI with the gates, and every operation of the contract
  wired (the blank one: those that need no stage). CI's new `templates`
  job makes a game from each with `--engine-path` and runs the gates its
  own CI runs (fmt, clippy, tests, `check`, `check-keys`). Both passed
  locally.
- Hello, Grid wires every operation (`src/tools.rs`); its
  `tests/it/contract.rs` has an integration test per operation. The
  CLI's own tests drive every branch through a made-up game.

**Deviations**

- `limits` is a field of the answer, not of each result, so an operation
  that could not run has one too (ADR-0022 §1).
- `import`: no importer of Tiled, LDtk or Aseprite exists yet (1123 and
  1124 are parked), so it runs over the format registry's kinds (2201).
  No engine format can write its kind yet, so `--print` shows the asset
  as read and says so in its limits; ticket 2218 writes our text formats'
  writers. The files a file names are read from its folder, leaked once
  per call until 2207 (noted there).
- The `new` and `doctor` tests in Hello, Grid's contract tests run on a
  scratch folder and on this machine: neither needs a game.
- A new game depends on the engine's repository at `main` unless
  `--engine-rev` or `--engine-path` says otherwise (the binary doesn't
  know its own commit); the answer's limits say so.
- Skills are not copied by `new`: they are ticket 1134.
- `frame-png` now refuses a `--wait` longer than 33.3 s, which used to
  panic the Harness (its 2,000-frame limit).

**Follow-up tickets**

- 2218: writers of the engine's text formats, so `import --print` writes
  our own text.
- Notes added to 2005 (the templates must be packaged into `shuyi-cli`
  for crates.io) and 2207 (`import`'s folder bundle).

**For the game**

Every API change is additive: new `GameTools` methods with defaults, new
Harness methods, serde on `frame_png::Script`, `Step` and `GameArg`, and
on `shuyi_core::Dir`. A clone of the game at its pin (`939f023`) builds
every target on this branch once ticket 2312's change, already on
`main`, is applied to it (`input.dt` is now `input.dt()`, 18 places);
nothing of this branch needs a change in the game, so no game PR has to
move the pin for it. The game can wire its own `check`, `session`, `replay`
and `bots` in its `xtask` tools whenever it wants `cargo shuyi run` and
the rest on itself; that would be a game ticket of its own.
