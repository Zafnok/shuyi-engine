---
id: "1129"
title: "The tool contract: shuyi new, check, run, scene, replay, bots, import, doctor, with JSON output"
type: feature
milestone: "Engine: public"
model: opus-5.5
effort: xhigh
status: todo
blocked_by: ["game#1131", "2201"]
nick_input: none
completed:
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
ADR-0011). Keep the two templates as plain folders; 2501 turns them into
parts.

## Implementation steps

1. Define the types and the schema test.
2. Implement each operation over the Harness, the importers and the
   validators.
3. Write the templates. CI runs `shuyi new` into a temporary folder,
   then builds and tests the result.

## Acceptance criteria

- [ ] CI makes a game with `shuyi new grid`, builds it, and its tests
      pass.
- [ ] Each operation has an integration test on `hello-grid` (1118).
- [ ] The schema test passes.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Integration: one test per operation, and the template build.

## Completion notes
