---
id: "1135"
title: "Parked: Research: visual scripting as a view over text — the graph file, what it compiles to, and the editor"
type: research
milestone: "Engine: public"
model: opus-5.5
effort: high
status: blocked
blocked_by: ["game#1017", "1125", "1127"]
nick_input: none
completed:
---

# 1135 — Research: visual scripting as a view over text

> **Parked** (Nick, 2026-10-14: "document the parked (i.e. after MVP/parity) tickets for networking, styles for later"): after the engine reaches parity with the game (the MVP in `docs/ROADMAP.md`), and after the scripting runtime (the game's ticket 1017) and the examples 1125 and 1127. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

Nick, 2026-10-14: the engine may ship "eventually with UI / graphical
coding (like Blueprints from Unreal)".

ADR-0005 §5 decides that a node graph is a text file (`*.graph.ron`). It
compiles to what the engine already runs, so there is no second runtime:

- state machines;
- triggers and their actions;
- dialogue branches;
- scripts for the scripting runtime (1011, 1017).

The research behind that decision:

- Godot removed VisualScript in 4.0. About 0.5% of users used it as
  their main language, and it lacked high-level building blocks.
- Unreal's Blueprints work because of the framework under them, but they
  are binary files that agents and git can't merge.

This ticket waits for the scripting runtime and for two examples
(1125, 1127). Those show which nodes people would actually need.

## Nick input

None. If the ADR proposes an editor that runs outside the game (a web
tool), it says so plainly in the report to Nick. It is a different look
from the game's glyph style.

## Scope

**In:**
- An ADR that decides four things:
  - the graph file format;
  - the first node library, taken from what 1125 and 1127 needed in code;
  - what each graph kind compiles to;
  - the editor: a web tool with `egui-snarl` (MIT/Apache), or our own
    glyph UI with the pointer (1121), or both.
- A throwaway spike (not committed) that builds one graph by hand,
  compiles it, and runs it in the platformer example.
- Implementation tickets for the format, the compiler and the editor.

**Out (do not do):**
- building the editor;
- a binary graph format.

## Implementation steps

1. Read ADR-0005, game ADR-0021, 1011's ADR and 1017's runtime.
2. List the logic in 1125 and 1127 that a designer would want to change
   without code.
3. Do the spike. Write the ADR and the tickets.

## Acceptance criteria

- [ ] The ADR exists with the four decisions and the alternatives.
- [ ] Tickets exist, and `cargo xtask ticket-lint` passes.

## Tests required

None (research).

## Completion notes
