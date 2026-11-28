---
id: "3271"
title: "Screenshots the player takes: a Screenshot action writes the frame as a PNG to the game's Screenshots folder, or downloads it on the web"
type: feature
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
---

# 3271 — Screenshots the player takes

## Context

2514 (done) notes that "a screenshot the player takes has no ticket":
frame-png and `shuyi attach --screenshot` serve developers. The Bevy
coverage audit of 2026-11-28 (`docs/ROADMAP.md`, *Bevy's game
coverage*) found players of most games expect one key that saves a
picture, and photo modes and shared puzzles build on it. Bevy has a
screenshot API and an easy-screenshot plugin
(`crates/bevy_render/src/view/window/screenshot.rs`,
`crates/bevy_dev_tools/src/easy_screenshot.rs`;
`examples/window/screenshot.rs`).

Shuyi can make the picture deterministically: the raster that
frame-png uses paints the current frame at a whole-number scale.

## Nick input

None.

## Scope

**In:**
- A `UiAction::Screenshot`, bound by default to F12 in the keymap,
  never hard-coded (rule 8; the `input-actions` skill), and listed on
  the key-bindings screen.
- On the action, the app hands the current frame to the raster at the
  window's whole-number scale (2333) and encodes a PNG.
- **Where it goes:** through `Platform` storage into a `Screenshots`
  folder beside the saves (3310's folder), named with a number, never
  the clock (`screenshot-0001.png`, one past the highest in the
  folder). On the web,
  a download through the web backend's JS plugin; if the plugin has no
  download call yet, this ticket adds one.
- A toast confirms it, naming the file; a failure (disk full, storage
  refused) shows a toast with the reason and logs it.
- A game may turn the action off in its project file.

**Out (do not do):**
- a photo mode (free camera, hiding the HUD): a game's own screen;
- capturing the real window's pixels (2018's capture mode serves CI);
- uploading or sharing anywhere.

## Implementation steps

1. Read the `input-actions` skill, the raster in `crates/ui/src/frame/`
   used by frame-png, 3310's storage folders and the web plugin.
2. Add the action and its default binding and help text.
3. Paint and encode the frame; write it through storage with the next
   number.
4. Add the web download path.
5. Add the toast and the project-file switch.
6. Write the docs page.

## Acceptance criteria

- [ ] In the Harness, the action writes a PNG whose pixels equal
  frame-png of the same frame at the same scale.
- [ ] Two screenshots get `0001` and `0002`; with `0001` deleted, the
  next is `0003` (no reuse).
- [ ] A storage error shows a toast and logs it, and the game goes on.
- [ ] With the project file's switch off, the action is absent from the
  key-bindings screen.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] The skill pack (`skills/`) is updated for a changed schema, template, format or problem code, or the completion notes have a `Skills:` line saying why not (`cargo xtask docs-rules`, 3534).
- [ ] Guides in `docs/guides/` updated, if this adds a direction, a module or a move (ADR-0009 §5).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: naming, the switch.
- Snapshot / integration: the PNG against frame-png; the web build
  compiles the download call.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
