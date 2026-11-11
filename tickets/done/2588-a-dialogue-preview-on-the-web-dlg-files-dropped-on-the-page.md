---
id: "2588"
title: "A dialogue preview on the web: .dlg files dropped on the page"
type: feature
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: done
blocked_by: ["3092"]
nick_input: none
completed: 2026-11-10
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

- [x] A `.dlg` file dropped on the preview's web page is listed and
      plays; dropped again changed, it plays again.
- [x] A Harness test drops a scene through the input script and reads it
      in the preview's view.
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Integration: the preview over an empty layer, a dropped scene, then
  the same file dropped changed.

## Completion notes

- `shuyi_ui::dialogue::preview::drop_scenes(game, files)` puts the
  `.dlg` files of a drop in the preview's folder layer (the
  `DialogueFolder` module keeps it on the context) and tells the game's
  modules, as a saved file does; other files are left out.
- `BasicGame::preview_start` and `preview_harness` give the preview a
  system hook that hands it each drop, on the desktop and the web. On
  the web `run_dialogue_preview` starts it over an empty layer, where
  it used to show an error.
- The Scenes tab's note, where nothing is listed and the platform hears
  drops (`ctx.platform.can_drop_files()`), is a new text key,
  `preview.note.no_scenes_drop` ("No scenes: drop a .dlg file here").
  A game's text without the key shows `preview.note.no_scenes` there as
  before, so a game's language file needn't change (the game's preview
  included).
- Tests in `crates/platform/src/basic/tests.rs`: an empty folder, the
  note on each platform, a dropped non-scene left out, a scene dropped
  through the input script's `drop` step played, then the same file
  dropped changed played again; the old note for a text without the
  key; and an `App` started with `preview_start` hearing a drop.
- Docs: the book's chapter 6 *A preview for your writer* and the
  `shuyi-dialogue` skill.
- Not checked in a browser: CI's `web-render` job loads the examples'
  pages, but no example runs `run_dialogue_preview` on the web, and a
  drop can't be scripted there. The web path is the same `App` and hook
  the tests drive, over 3092's drop.

No break: an added function, an added text key constant and a key games
may add.
