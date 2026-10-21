---
id: "2507"
title: "Parked: The editor shell: shuyi edit, the live view with the debug channels, the tree of what is running, and an inspector over settings files"
type: feature
milestone: "Engine: editor"
model: opus-5.5
effort: high
status: blocked
blocked_by: ["2506", "1124", "2515", "2907", "2521"]
nick_input: sign-off
completed:
---

# 2507 — The editor shell: live view, tree, inspector

> **Parked** (Nick, 2026-10-14: "document the parked (i.e. after MVP/parity) tickets for networking, styles for later"): a follow-up of the parked editor research (2506), which rewrites this ticket to its ADR before anyone starts it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

Nick, 2026-10-19 (`docs/decisions.md`): "a UI layer for visual code, like
Godot editor or Unreal Blueprints". 2506 decides where the editor runs
and what it adds to the live link (2521) to talk to a running game.
This ticket builds the shell every panel lives in, with its first three
panels:

- the live view;
- the tree;
- the inspector.

The graph panel is 2508.

This ticket was written before 2506. Its steps name the decisions 2506
makes as **[2506]**. 2506 replaces each with the choice and splits the
ticket if it is over a PR's worth. What is fixed already:

- the editor is a client of the tool contract (1129, ADR-0005), and
  reaches a running game over the live link (2521);
- it edits text files only;
- its live view uses the debug view's channels, pause and step (2315,
  2316);
- it changes running state only the way 2506's ADR allows.

## Nick input

**Sign-off.** Nick opens the editor on the toy game, turns a channel
on, pauses and steps, and changes one setting in the inspector. He
comments on how it feels and looks.

## Scope

**In:**
- **`shuyi edit`** in `shuyi-cli`. It opens the editor on the project
  of the `shuyi.ron` in the current folder, starts the game's windowed
  build with its `link` feature (`SHUYI_LINK`) and connects to it over
  the live link (2521), with the operations 2506 adds. The toy
  (`crates/cli/toy`) has only a tool program today: the checks below
  run on a windowed toy or on Hello, Grid (2521 gives it `link`), per
  **[2506]**.
- **The live view:**
  - the running game's frames;
  - a list of the channels the game offers, each with a checkbox;
  - pause, step and slow motion.

  All of these go through the contract operations 2506 adds.
  Gizmos come from the game as data and are drawn by the editor's own
  painter, so the editor's look doesn't need the game's skin.
- **The tree:** the screen stack, top first, with each screen's
  `name()`. Under a stage, what its `debug_readout` (2316) says. Under
  a stage, the entries of its dump: the `data` of the contract's
  `scene` answer (`Scene` in `crates/cli/src/contract/session.rs`, which
  a game builds with `Headless::with_scene`). That holds whatever store
  the stage keeps them in: a plain struct behind `TurnRules`, 1119's
  `Arena<T>` or `shuyi-ecs`'s `World` (2107). The tree reads the dump,
  never a store, as 2904's ADR says. The live link doesn't carry
  `data` (2521's Out), so 2506 adds it. Under a stage that has them,
  its world layers (1120). Choosing a node turns on the matching
  channel and, if the node has a file, opens it in the inspector.
- **The inspector** over settings as data (ADR-0009 §4):
  - it opens a module's RON settings file;
  - it shows fields by the file's schema, with a text box per number
    and string, and a drop-down per enum (its text boxes paste from
    the clipboard on an action, never a key: miniquad has
    `window::clipboard_get` and `clipboard_set` on desktop, and the
    window 2506 picks says how elsewhere; typing follows 2346's name
    box);
  - it writes the file back, keeping comments and field order;
  - the game reloads it by 1124's hot reload;
  - a load error from the reload shows next to the field.

  The schema of each file comes from the format that reads it, as
  `shuyi kinds` gives it (2515 and its ADR: `schemars` behind
  `shuyi-assets`' `schema` feature). A field's path and an enum's
  variants for the drop-downs come from 2907's ADR (one description of
  a game's types). 2506 adds only what the inspector needs beyond them.
- **Undo and redo** within a session, per **[2506]**.
- A **README** section and a skill entry (1134) on `shuyi edit`.

**Out (do not do):**
- the graph panel (2508);
- a level editor (2506 decides; Tiled and LDtk otherwise, ADR-0004,
  with the game's types written to Tiled, checked against LDtk, and map
  objects read as the game's types by 2232);
- editing running state other than as 2506's ADR allows;
- a project file other than `shuyi.ron`.

## Implementation steps

1. Read the ADRs of 2506, 2515, 2907, 2904 (what the tools read) and
   2521; the code of 2521, 2315 and 2316; 1129's contract and 1124's
   hot reload.
2. Add the contract operations 2506 named, with their JSON and CLI
   forms and their tests.
3. Build the shell in **[2506]**, with `shuyi edit` starting the game
   and connecting.
4. Build the live view, then the tree, then the inspector.
5. Write the README section and the skill entry.
6. Run the gates.

## Acceptance criteria

- [ ] `cargo shuyi edit` in `crates/cli/toy` opens the editor on the
      toy game, running.
- [ ] Turning the `grid` channel on in the editor shows its gizmos, and
      the game's own snapshot lists the same gizmos.
- [ ] Pause and step in the editor give the same final hash as 2316's
      Harness test.
- [ ] Editing a number in a settings file from the inspector changes
      the file on disk (comments kept, one line in `git diff`), and
      the game shows the new value after its next reload.
- [ ] A bad value shows the reload's error next to the field, and the
      game keeps running.
- [ ] Licenses of every new dependency pass `cargo deny`; no
      `wasm-bindgen` enters the game's web build.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the inspector's round trip. A RON file opened and saved with
  no change is the same bytes.
- Property: for any value of a field's type, write then read gives the
  value back.
- Integration: the contract operations, driven from a test the way the
  editor drives them, on the toy game without a window.

## Completion notes

*(Filled in by the session that completes the ticket.)*
