---
id: "2506"
title: "Parked: Research: the editor, a workspace over a running game: where it runs, how it talks to the game, and its panels"
type: research
milestone: "Engine: editor"
model: opus-5.5
effort: high
status: blocked
blocked_by: ["1129", "2316", "2521"]
nick_input: sign-off
completed:
---

# 2506 — Research: the editor, a workspace over a running game

> **Parked** (Nick, 2026-10-14: "document the parked (i.e. after MVP/parity) tickets for networking, styles for later"): after the engine reaches parity with the game (the MVP in `docs/ROADMAP.md`), and after the tool contract (1129), the debug view in real time (2316) and the live link (2521). It belongs with the parked visual scripting research (1135). Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

Nick, 2026-10-14: the engine may ship "eventually with UI / graphical
coding (like Blueprints from Unreal)". Nick, 2026-10-19
(`docs/decisions.md`): "a UI layer for visual code, like Godot editor or
Unreal Blueprints", next to seeing "level layouts and collisions bounds
and skeletons in realtime".

1135 researches the *graph*: its file (`*.graph.ron`, ADR-0005 §5), its
first nodes, and what it compiles to. A Godot- or Unreal-style editor
is more than a graph panel. It is a workspace with these parts:

- a view of the running game;
- a tree of what is in it;
- an inspector over its settings;
- panels for graphs and levels.

Nothing in the roadmap decides that workspace yet. This ticket does.
The graph panel is then one of its panels (2508).

What is already decided, and the editor must keep:

- **Text is the source of truth** (ADR-0005 §4, ADR-0007). The editor
  edits text files: settings as data (ADR-0009 §4), graphs, levels.
  Agents and git read and diff the same files. No binary scene or
  graph files, ever.
- **One contract, many surfaces** (ADR-0005). The CLI, the MCP server
  (1133) and the skills drive a game through the tool contract (1129).
  The editor is one more client of that contract, not a second way in.
- **State changes only through commands or inputs** (ADR-0002). An
  inspector may *show* live state. Changing it while the game runs
  either goes through a command or input that a replay records, or it
  is not allowed.
- **Debug commands are the recorded way in** (2356). A screen declares
  commands with typed arguments, and each changes state only by
  sending the game's own command, which records hold. The inspector's
  live edits can go through the same path.
- **The debug view is shared** (2315, 2316). The editor's live view
  turns on the same channels and uses the same pause and step. It does
  not have its own gizmos.
- **One description of a game's types** (2907's ADR). The tree and the
  inspector read a running game's values, paths and shapes through it.
  A settings file's shape comes from `shuyi kinds` (2515), which agrees
  with it. The editor adds no model of its own.
- **The web build has no `wasm-bindgen`** (game ADR-0034). `egui` on
  the web needs `eframe`, which uses it. So does any toolkit built on
  `web-sys`.

## Nick input

**Sign-off.** The report to Nick says, in plain words:

- where the editor runs: a window of its own, a browser tab, or inside
  the game;
- what it looks like, next to the game's glyph style;
- what a developer can do in it on day one, and what stays in a text
  editor, Tiled or LDtk.

Nick comments on the scope and the look, not on the code.

## Scope

**In:**
- **An ADR** (the `write-adr` skill) that decides:
  1. **Where it runs**, with the alternatives measured:
     - a native app on `egui`/`eframe` (MIT/Apache) that runs the game
       in its own window or in a panel;
     - a web tool served by `shuyi edit`, talking to a native game
       over the live link (2521);
     - an in-game editor in our glyph UI with the pointer (2405).

     For each: its build time on Nick's machine (GNU toolchain, no
     MSVC), its licenses (checked with `cargo deny`), and whether it
     needs `wasm-bindgen`.
  2. **How it talks to the game:** over the live link (2521's ADR),
     which already carries a running build's status, frame text,
     window picture, input (actions by name) and log. Decide only the
     operations the editor adds to the link and the contract, such as
     `watch` (a stream of frames and gizmos) and `channels`. Pause and
     step are 2316's `DebugPause` and `DebugStep` actions, sent as
     `act` steps. What is added goes into both the link and the
     contract, so the CLI and MCP get it too.

     Going back is already in the contract: `--until` on `run`,
     `scene` and `frame-png`, and MCP's `seek` (2357). In a running
     build, 2357's `DebugBack` action goes back, sent as an `act` step
     like pause and step. The editor uses them rather than a way of its
     own.
  3. **Its panels on day one**, each with what it edits:
     - the live view, with the channels, pause, step, slow motion and
       going back (2357);
     - the screen stack, and the stage's tree (layers, units, bodies);
     - the inspector over a module's settings files (their shapes come
       from `shuyi kinds`, 2515; this ADR doesn't pick another schema
       source);
     - the graph panel (1135's format);
     - levels: in-editor, or opened in Tiled or LDtk with hot reload
       (1124).
  4. **Live edits:** which settings take effect by hot reload, which
     need a restart, and whether a value changed in the inspector
     while the game runs is recorded, refused, or written to the file
     only.
  5. **Undo:** the editor's own undo stack, and git as the record.
- **A throwaway spike** (not committed). The chosen shell shows the
  toy's grid game running with the `grid` channel on, and edits one
  setting in a RON file that the game reloads.
- **The tickets.** Rewrite 2507 (the shell) and 2508 (the graph panel)
  to the ADR, splitting them where one is over a PR's worth. Write
  tickets for any other day-one panels. Update 1135's editor point if
  1135 is still open.

**Out (do not do):**
- building the editor;
- the graph's format and nodes (1135);
- a binary or opaque project file. If the editor needs a project file,
  it is `shuyi.ron`, which already exists.

## Implementation steps

1. Read ADR-0002, ADR-0005, ADR-0007, ADR-0009 §4, 2907's ADR, 2315's
   ADR, 2521's ADR and `shuyi_platform::link`, 1129's contract and 1135
   (and its ADR, if it is done).
2. Look at how Godot's editor, Unreal's editor, LDtk and Bevy's editor
   prototypes split panels and talk to a running game. Write down what
   to take and what to leave, with sources. Copy no code or art.
3. Build each candidate shell far enough to time a clean build and a
   rebuild on Nick's machine, and run `cargo deny check` on it.
4. Do the spike in the chosen shell.
5. Write the ADR and the report to Nick. Rewrite and write the tickets.

## Acceptance criteria

- [ ] The ADR exists with the five decisions and the alternatives,
      including the build times and the license check.
- [ ] 2507 and 2508 match the ADR, and any new tickets exist.
- [ ] `cargo xtask ticket-lint` passes.
- [ ] Nick has signed off on the report.

## Tests required

None (research).

## Completion notes

*(Filled in by the session that completes the ticket.)*
