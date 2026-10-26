---
id: "2563"
title: "Worked examples call the MCP session tools: an Mcp step in the script runner, and shuyi-perf's example takes a heap snapshot and a diff"
type: feature
milestone: MVP
model: sonnet-5
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2563 — Worked examples call the MCP session tools

## Context

A skill of the pack keeps a worked example, a script of contract
commands that `cargo xtask skill-examples` runs on a fresh `shuyi new`
game ([ADR-0043](../../docs/adr/0043-the-skill-pack-skills-at-the-root-worked-examples-run-by-the-xtask-check-skills-in-the-cli-and-the-pack-folder-as-the-plugin.md),
`crates/xtask/src/script.rs`). Its steps are `Shuyi` (`cargo shuyi
<args> --json`), `Cargo`, `Write`, `Edit` and `Holds`. None of them
reaches the MCP server's session tools (`start`, `step`, `perf`, …),
which have no command of their own: a skill that teaches them can't
check them in its example.

2554 asked `shuyi-perf`'s example to check "that a `diff_from` that
isn't kept answers its code" (MCP's `perf` tool, `SH0001`). Its example
checks the rest (`heap_tags`, `heap_growth`, the trace's snapshot
lines); this piece waits for a step that calls a session tool. The
behaviour itself is tested in `crates/cli/tests/it/mcp.rs`
(`take_heap_snapshots_and_a_diff`) and `crates/cli/src/mcp/host/tests.rs`.

## Nick input

None.

## Scope

**In:**
- A new `Step::Mcp { calls: Vec<McpCall> }` in `crates/xtask/src/script.rs`:
  the runner starts `cargo shuyi mcp` in the game's folder once for the
  step, speaks the MCP JSON-RPC lines the server reads (`initialize`,
  then a `tools/call` per call, as `crates/cli/tests/it/mcp.rs`'s
  `Client` does), and ends it. An `McpCall { tool: String, input:
  serde_json::Value as RON text or a map, ok: bool (default true),
  expect: Vec<Expect> }` checks the call's `structuredContent` with the
  same `Expect`s as `Shuyi` (`Is`, `Has`, `Holds`), and `isError` against
  `ok`. The xtask still doesn't depend on `shuyi-cli`.
- `shuyi-perf`'s `example.ron` gains an `Mcp` step: `start` with
  `project: "."`, `perf` with `snapshot: true`, `step` 30 frames, `perf`
  with `snapshot: true` and `diff_from: 0` (expect `Has("/result/diff/tags")`),
  then `perf` with `diff_from: 7`, `ok: false`, expect
  `Is("/code", "SH0001")`, then `stop`.
- The runner's module docs list the new step.

**Out (do not do):**
- MCP calls over the live link (`start` with `link`): a running build
  has no place in a worked example.
- Other skills' examples: each skill that wants a session step adds it
  in its own ticket.

## Implementation steps

1. Read `crates/xtask/src/script.rs` and its tests, and
   `crates/cli/tests/it/mcp.rs`'s `Client` (how it starts the server and
   reads a reply line).
2. Add `Step::Mcp` and `McpCall`, run them, and give a verdict per step
   that names the failed call.
3. Add the step to `skills/shuyi-perf/example.ron` as above.
4. Run `cargo xtask skill-examples shuyi-perf` and the gates.

## Acceptance criteria

- [ ] A script's `Mcp` step starts the server once, makes its calls in
      order, and fails naming the call whose `ok` or `expect` didn't
      hold.
- [ ] `shuyi-perf`'s example checks that a `diff_from` that isn't kept
      answers `SH0001`, and `cargo xtask skill-examples shuyi-perf` runs
      green.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: an `Mcp` step read from RON; the verdict of a call that fails
  its `ok`, and of one that fails an `expect` (over a fake server's
  reply lines, no process).
- Integration: the `shuyi-perf` example.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
