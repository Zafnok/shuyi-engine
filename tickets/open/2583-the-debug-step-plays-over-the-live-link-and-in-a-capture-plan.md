---
id: "2583"
title: The debug step plays over the live link and in a capture plan
type: feature
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2583 — The debug step plays over the live link and in a capture plan

## Context

2315 (#284) added the debug view and a `debug` step to the input script
(`Step::Debug`, ADR-0082's script), which the Harness plays with
`Harness::show_debug`. Its completion notes say: "The live link and
capture plans refuse it for now." No ticket was written for "for now".
The retro of 2026-11-08 found it. An agent attached to a running debug
build over the live link (`start` with `link`) can't turn on a debug
channel. So it can see the gizmos only in a headless `frame-png`, not in
the game the player is looking at.

## Nick input

None.

## Scope

**In:**
- The live link plays a `debug` step: the running game opens the debug
  view with the step's channels, as `Harness::show_debug` does.
- A capture plan accepts the step in the same way.
- The contract's version rises, and the schemas and `docs/contract.md`
  say so.

**Out (do not do):**
- New debug channels.

## Implementation steps

1. Find where the live link and the capture plan refuse `Step::Debug`
   (`grep -rn "Debug" crates/cli/src` near the link and capture code)
   and what they return.
2. Route the step to the running game through the live link's message
   set, and handle it in `shuyi-app`'s link handler by calling what
   `Harness::show_debug` calls.
3. Update the contract docs and schemas, and raise
   `contract::VERSION`.

## Acceptance criteria

- [ ] A script with a `debug` step, sent over the live link to Hello,
      Grid in a debug build, shows the grid channel (a `frame_png` over
      the link in the notes).
- [ ] A capture plan with a `debug` step is accepted.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Integration: the live-link test harness plays the step.
- Unit: the capture plan accepts it.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
