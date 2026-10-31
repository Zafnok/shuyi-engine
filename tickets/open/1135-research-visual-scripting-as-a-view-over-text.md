---
id: "1135"
title: "Parked: Research: visual scripting as a view over text — the graph file, what it compiles to, and the editor"
type: research
milestone: "Engine: public"
model: reasoning
effort: high
status: blocked
blocked_by: ["2101", "1125", "1127", "2907"]
nick_input: none
completed:
---

# 1135 — Research: visual scripting as a view over text

> **Parked** (Nick, 2026-10-14: "document the parked (i.e. after MVP/parity) tickets for networking, styles for later"): after the engine reaches parity with the game (the MVP in `docs/ROADMAP.md`), and after the scripting runtime (2101, `shuyi-script`) and the examples 1125 and 1127. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

Nick, 2026-10-14: the engine may ship "eventually with UI / graphical
coding (like Blueprints from Unreal)".

ADR-0005 §5 decides that a node graph is a text file (`*.graph.ron`). It
compiles to what the engine already runs, so there is no second runtime:

- state machines;
- triggers and their actions;
- dialogue branches;
- scripts for the scripting runtime (2101, which game#1017 uses for
  the game's scripted rules).

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
  - the first node library, taken from what 1125 and 1127 needed in code,
    with node inputs and outputs typed through 2907's ADR (one
    description of a game's types);
  - what each graph kind compiles to;
  - the graph panel's needs: what 2508 (the panel in the editor) must
    show and check. Where the editor runs, and in what toolkit, is the
    editor research's decision (2506). If 2506 is done, follow its ADR.
    If it isn't, write down the panel's needs here, and 2506 picks the
    toolkit with them in hand.
- A throwaway spike (not committed) that builds one graph by hand,
  compiles it, and runs it in the platformer example.
- Implementation tickets for the format and the compiler, and a rewrite
  of 2508 (the graph panel) to this ADR.

**Out (do not do):**
- building the editor;
- a binary graph format.

## Implementation steps

1. Read ADR-0005, game ADR-0021, the game's mods ADR (game#1011),
   2101's runtime and 2907's ADR.
2. List the logic in 1125 and 1127 that a designer would want to change
   without code.
3. Do the spike. Write the ADR and the tickets.

## Acceptance criteria

- [ ] The ADR exists with the four decisions and the alternatives.
- [ ] Tickets exist, and `cargo xtask ticket-lint` passes.

## Tests required

None (research).

## Completion notes
