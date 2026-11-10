---
id: "2588"
title: "A dialogue preview on the web: .dlg files dropped on the page"
type: feature
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: ["3092"]
nick_input: none
completed:
---

# 2588 — A dialogue preview on the web

## Context

**From 2587 (done):** a game on the short path hands its writer a
dialogue preview in one call, `shuyi_app::run_dialogue_preview(game,
folder, preview)`, which reads the folder with `read_watched_folder`
and starts `BasicGame::preview_start(folder, preview)`. The web has no
folders, so on the web it compiles and the window says so: a writer
can't run the preview in a browser. Hello, Grid's long-path preview
(`examples/hello-grid/src/bin/preview.rs`) plays only the game's own
scene file there.

3092 hands files dropped on the window to the game, on the desktop and
the web, as a bundle layer named `dropped`. With it a writer could open
the preview's page and drop their `.dlg` files on it.

## Nick input

None.

## Scope

**In:**
- On the web, `run_dialogue_preview` starts the preview with an empty
  folder layer, and a `.dlg` file dropped on the page joins it as a
  changed file (the `DialogueFolder` module hears of it, as of a saved
  file on the desktop). The Scenes tab's note says to drop files where
  nothing is listed yet, through the platform fact 3092 adds, never a
  `cfg!`.
- The desktop gains the same: a dropped `.dlg` file joins the folder's
  layer.
- The book's chapter 6 section *A preview for your writer* and the
  `shuyi-dialogue` skill say how a writer uses it on the web.

**Out (do not do):**
- Any change to how the preview screen lists, plays or checks scenes
  beyond the note.
- A file picker (3092's **Out**).

## Implementation steps

1. Read `crates/app/src/lib.rs` (`run_dialogue_preview`,
   `preview_folder`), `crates/platform/src/basic.rs`
   (`BasicGame::preview_start`), `crates/ui/src/dialogue/preview/folder.rs`
   and 3092's drop event.
2. Hand dropped `.dlg` files to the folder's layer; the note.
3. Harness tests with 3092's drop step; the book and skill; the gates.

## Acceptance criteria

- [ ] A `.dlg` file dropped on the preview's web page is listed and
      plays; dropped again changed, it plays again.
- [ ] A Harness test drops a scene through the input script and reads it
      in the preview's view.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Integration: the preview over an empty layer, a dropped scene, then
  the same file dropped changed.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
