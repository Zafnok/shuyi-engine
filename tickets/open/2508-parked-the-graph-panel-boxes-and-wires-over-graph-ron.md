---
id: "2508"
title: "Parked: The graph panel: boxes and wires over graph.ron files, compile errors on the node, and the running node lit up"
type: feature
milestone: "Engine: editor"
model: reasoning
effort: high
status: blocked
blocked_by: ["1135", "2507"]
nick_input: sign-off
completed:
---

# 2508 — The graph panel: boxes and wires over `*.graph.ron`

> **Parked** (Nick, 2026-10-14: "document the parked (i.e. after MVP/parity) tickets for networking, styles for later"): a follow-up of the parked visual scripting research (1135) and the editor shell (2507). 1135's and 2506's ADRs rewrite this ticket before anyone starts it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

Nick, 2026-10-14: "UI / graphical coding (like Blueprints from Unreal)".
Nick, 2026-10-19 (`docs/decisions.md`): "a UI layer for visual code, like
Godot editor or Unreal Blueprints".

ADR-0005 §5 decides that a node graph is a text file (`*.graph.ron`). It
compiles to what the engine already runs:

- state machines;
- triggers and their actions;
- dialogue branches;
- scripts (2101).

1135 decides the format, the first nodes and the compiler. This ticket
is the visual part: the panel in the editor (2507) where a person edits
that file as boxes and wires. Agents and git keep reading and diffing
the text.

Like Unreal's Blueprint debugger, the panel also shows the graph
*running*: which state a state machine is in, which trigger just
fired. That is a debug channel (2315), so the in-game view and
`frame-png` can show it too.

This ticket was written before 1135 and 2506. Their decisions are
marked **[1135]** and **[2506]**. Each of them rewrites the parts
marked with its id when it lands, and the later one checks the whole
ticket against both ADRs.

## Nick input

**Sign-off.** Nick builds a small state machine for the platformer
example in the panel (a door that opens when a switch is hit), runs
it, and comments on how it feels.

## Scope

**In:**
- **The panel** in the editor shell, in **[2506]**'s toolkit (with
  `egui-snarl`, MIT/Apache, if 2506 chose `egui`):
  - it opens a `*.graph.ron` and draws its nodes as boxes, at the
    positions the file stores, and its edges as wires;
  - it adds a node from the node library **[1135]**, with a search
    box;
  - it connects ports only where the types match **[1135]**;
  - it moves, deletes and renames nodes, and edits a node's fields
    with the inspector's widgets (2507).
- **Saving:** the file is written in a stable order (nodes by id,
  edges by `(from, to)`), so moving one node changes one line in
  `git diff`. A file opened and saved with no change is the same
  bytes.
- **Compiling:** on save, the panel runs 1135's compiler through the
  tool contract. Each error is shown on the node it names. A graph
  that compiles is hot reloaded into the running game (2220's
  `Hot<T>` and `Module::reloaded`).
- **Running:** a channel `graph` (2315) for each loaded graph. The
  running game reports, through a contract operation, the active
  state and the last nodes that fired. The panel lights them up, and
  in the game the channel draws them as a readout (2316).
- **An example:** the door and switch state machine in the platformer
  example (1127), saved as a `*.graph.ron` with its test.
- **Either order with 2104** (animation state machines, parked): if
  2104 is done, the panel also shows its `*.anim_graph.ron`, read
  only: states as boxes and transitions as wires, each with its
  conditions. If not, nothing here; 2104 does it when it is worked
  (its scope says so).

**Out (do not do):**
- a graph runtime. Graphs compile to what exists (ADR-0005 §5);
- a binary graph format;
- new node kinds beyond 1135's first library (each one later is its
  own small ticket).

## Implementation steps

1. Read ADR-0005 §5, 1135's ADR, 2506's ADR and 2507's shell.
2. Build the panel: open, draw, add, connect, edit, save.
3. Add the round-trip and stable-order tests.
4. Wire compiling and errors through the contract.
5. Add the `graph` channel and the active-node report.
6. Make the door and switch example, and its test.
7. Run the gates.

## Acceptance criteria

- [ ] Every `*.graph.ron` fixture of 1135 opens and saves byte for byte.
- [ ] Moving one node changes one line of the file.
- [ ] Connecting two ports of different types is refused, with the
      reason shown.
- [ ] A compile error is shown on the node it names.
- [ ] With the game running, hitting the switch lights up the "open"
      state in the panel, and the `graph` channel's readout names it.
- [ ] The example's replay hash is the same with the panel connected
      and without it.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: stable order; the port type check.
- Property: for any graph the library can build, save then load gives
  the same graph, and compiling it gives the same output as compiling
  the hand-written text.
- Integration: the door and switch example through the contract, with
  no window.

## Completion notes

*(Filled in by the session that completes the ticket.)*
