---
id: "3453"
title: "Debug menu tools: a state browser of live scene data as a folding tree filtered by fuzzy search, an input viewer, and a fuzzy matcher the console completes with"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: ["3265", "2581"]
nick_input: none
completed:
---

# 3453 — A state browser and an input viewer in the debug menu

## Context

The debug view (2315 and 2316, done) draws gizmos, pauses, steps and
shows a readout; the debug menu (`crates/ui/src/debug.rs`,
`DebugMenuScreen`) lists its entries. 2581 (done) derives scene data
from the game's state and reads one value by JSON Pointer through the
contract, so agents can look inside a running game, but a developer at
the game's window can't. The Bevy coverage audit of 2026-11-28
(`docs/ROADMAP.md`, *Bevy's game coverage*) found three tools Bevy has
that Shuyi lacks:

- **an in-game inspector**: browse the running state as a tree, search
  it, watch values change (`crates/bevy_inspector/src/entity_tree.rs`,
  `details_panel.rs`; `examples/inspector/local_inspector.rs`);
- **fuzzy search** over names, for pickers and command palettes
  (`crates/bevy_fuzzy/src/lib.rs`, `no_std`);
- **an input viewer** that shows each pad's buttons and sticks live
  (`examples/tools/gamepad_viewer.rs`).

The parked editor (2507) runs in its own process; this one is inside
the game's window, on every backend, the Playdate included.

## Nick input

None.

## Scope

**In:**
- **`shuyi_ui::fuzzy`**, `no_std`: whitespace-separated atoms, matched
  case-insensitively as subsequences, scored as integers (runs, word
  starts and camel-case humps score more), ties broken by original
  order, so ranks are equal on native and wasm32; the matched indices,
  for highlighting with a colour role (2347).
- **A filter line in the debug menu:** typing filters its entries by
  fuzzy rank.
- **The state browser:** a debug entry that shows the top screen's scene
  data (2581's value tree) in 3265's tree widget, by JSON Pointer, with
  values updated each frame and a fuzzy filter over keys. It is read
  only; a value's edit is sent as a 2356 debug command, recorded
  through the game's own commands, so replays keep their hashes.
  **Either order with 2356:** if 2356 is done, add the edit path and
  make its console complete command names and argument enums through
  `shuyi_ui::fuzzy`; if not, the browser is read only, and 2356 adds
  both when it lands.
- **The input viewer:** a debug entry painting a plain-data view built
  from `FrameInput`: each action of the game's `ActionSet` by name and
  state, each pad by kind with its buttons and axes, keys held named
  through the keymap (rule 8), and the pointer.
- All of it in debug builds only, English only, read only by the layer
  rules.

**Out (do not do):**
- the editor's inspector (2507, parked) and 3418's inspector hints;
- editing values directly in memory (never: edits go through commands);
- a fuzzy file picker for the CLI.

**Estimated size:** about 1,300 lines (code 650, tests 500, docs 150).

**Split:** past 1,000 lines, split off the input viewer into a new
ticket.

## Implementation steps

1. Read `crates/ui/src/debug.rs`, 2581's scene data, 3265's tree widget,
   2347's styled text and `FrameInput`.
2. Write `shuyi_ui::fuzzy` with its tests.
3. Add the filter line to the debug menu.
4. Write the state browser over the scene data tree.
5. Write the input viewer.
6. Add the 2356 parts as the clause says.
7. Write the docs page.

## Acceptance criteria

- [ ] `fuzzy::rank("pz", ["pause", "puzzle", "pizza"])` puts `pizza`
  before `puzzle` (word start and run) and leaves out `pause`.
- [ ] Ranks of a fixed list are equal on native and wasm32 (golden).
- [ ] The state browser shows a value change the frame after the state
  changes, and its tree's pointer equals 2581's for that value.
- [ ] The input viewer shows a held pad button as held in a Harness run,
  named through the keymap.
- [ ] None of it is in a release build (a build test).
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] The skill pack (`skills/`) is updated for a changed schema, template, format or problem code, or the completion notes have a `Skills:` line saying why not (`cargo xtask docs-rules`, 3534).
- [ ] Guides in `docs/guides/` updated, if this adds a direction, a module or a move (ADR-0009 §5).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the matcher's scoring and ties.
- Property: a pattern that is a prefix of a name always matches it.
- Snapshot / integration: snapshots of the browser and the viewer.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
