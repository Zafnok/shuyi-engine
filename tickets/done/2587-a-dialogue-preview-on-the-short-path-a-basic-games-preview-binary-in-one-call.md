---
id: "2587"
title: "A dialogue preview on the short path: a BasicGame's preview binary in one call"
type: feature
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: done
blocked_by: ["2586"]
nick_input: none
completed: 2026-11-10
---

# 2587 — A dialogue preview on the short path

## Context

**From 2586 (done):** the engine has the dialogue preview a game hands a
writer (`shuyi_ui::dialogue::preview`): the `DialoguePreview` screen, the
`PreviewGame` and `PreviewScreen` traits a game implements, and the
`DialogueFolder` module that reads a folder's `.dlg` files again when one
is saved. Hello, Grid has a preview binary (`hello-grid-preview`,
`examples/hello-grid/src/bin/preview.rs`), but it is a game on the long
path: its binary builds its own `AppConfig` and `Start`.

A game on the short path (`BasicGame`, `run_game`, what `shuyi new`
makes) can't do the same without leaving the short path: `run_game`
starts the game's first screen, a `BasicGame`'s modules are a `fn()` that
can't hold the folder's layer, and `basic_config` and `look` are private
to the backend and `shuyi-platform`. The book's chapter 6 (*A preview for
your writer*) shows the preview's parts and its test on Moon Garden for
that reason, and says Hello, Grid's binary is how a program starts it.

The generic shape: one call that runs a short-path game's dialogue
preview over a folder, as `run_game` runs the game. A visual novel made
with `shuyi new` hands its writer a second binary of three lines.

## Nick input

None.

## Scope

**In:**
- In `shuyi-app`: `run_dialogue_preview(game: BasicGame<A, T>, folder:
  &Path, preview: P)` (or the shape the session finds better, said in an
  ADR if it is new): reads `folder` with `read_watched_folder`, and runs
  the game's window, data and settings with `DialoguePreview` as its first
  screen, `DialogueFolder` over the folder's layer, and no debug screens.
  A folder that can't be read shows its error in the window, as a load
  error does.
- In `shuyi-platform`'s `BasicGame` what that needs (a `Start` with
  another first screen and an extra module), additive.
- The book's chapter 6 section *A preview for your writer* gains the
  binary, its code a snippet in `site/snippets/`; the `shuyi-dialogue`
  skill's *A preview for your writer* names the call.
- Harness tests of the start with the headless backend.

**Out (do not do):**
- Any change to the preview screen itself (2586's).
- A web preview (3092).
- A template change: `shuyi new` makes no preview binary.

## Implementation steps

1. Read `crates/app/src/lib.rs` (`run_game`, `basic_window`,
   `basic_config`), `crates/platform/src/basic.rs` (`BasicGame::start`,
   `game`) and `crates/ui/src/dialogue/preview.rs`.
2. Add the start of a preview to `BasicGame`, then the call in
   `shuyi-app`.
3. Tests, the snippet and the book, the skill.
4. The gates.

## Acceptance criteria

- [x] A short-path game's preview binary is one call, shown in the book
      from a snippet CI builds.
- [x] A Harness test starts the preview of a `BasicGame` over a folder's
      layer and plays a scene.
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Integration: the preview's start on the headless backend, with a
  changed file through `Harness::change_file`.

## Completion notes

- `shuyi-platform`: `BasicGame::preview_game(ctx, folder, preview)` (the
  `DialoguePreview` as the root screen, the game's own modules, a
  `DialogueFolder` over the folder's layer, no debug screens),
  `BasicGame::preview_start(folder, preview)`, whose `folder` is a
  `Result<Layer, String>` so a folder that couldn't be read is the load's
  error the platform shows, and `BasicGame::preview_harness(folder,
  preview)` for tests. All additive.
- `shuyi-app`: `run_dialogue_preview(game, folder: &Path, preview)`, as
  `run_game`: the game's window, files (with `--watch`'s layer) and
  settings; the folder read with `read_watched_folder` into a layer named
  `scenes`. On the web it compiles and the window says the web has no
  folders (a web preview, of dropped files, is 2588).
- Tests: `crates/platform/src/basic/tests.rs` starts a `BasicGame`'s
  preview in the Harness over a folder's layer, plays its scene, changes
  the file with `Harness::change_file` and reads the new line, then reads
  the scene to its end; and on the headless backend the start shows the
  card then the preview with debug tools off, and a folder's read error
  and a module error are the load's error. The test language file
  `preview_en.ron` holds the preview's text keys, which the blank
  template's has not.
- The book's chapter 6 *A preview for your writer* now shows Moon
  Garden's preview binary (`src/bin/preview.rs`, the `main` anchor of
  `site/snippets/src/preview.rs`), and its test starts with
  `game().preview_harness`; the snippet's hand-built `preview()` is gone.
  The `shuyi-dialogue` skill names the call.
- No ADR: the call follows `run_game`'s shape, and the start
  `BasicGame::start`'s.

No break.
