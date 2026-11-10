---
id: "2586"
title: "The dialogue preview screen: a game's scenes from a watched folder, with switches, lines, problems and reload, and Hello, Grid's preview"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: done
blocked_by: ["2585"]
nick_input: none
completed: 2026-11-09
---

# 2586 — The dialogue preview screen

## Context

1. **The game ticket:** game#0764, *A script preview program for
   writers*
   (`github.com/Zafnok/visions-of-shuyi/blob/main/tickets/open/0764-a-script-preview-program-for-writers.md`).
   The game is looking for a writer who doesn't program, and needs to
   hand them a program, not the source.
2. **The job in one sentence:** a writer who has no source and no Rust
   saves a `.dlg` file and, a second later, sees the scene as the game
   shows it: any scene, from any line, with any character present or
   gone, and the file's problems listed instead of a crash.
3. **Friction numbers:** no friction review: a missing feature.
4. **The generic shape:** a **dialogue preview**, a small program a game
   builds from its own code. It lists the scenes of the `.dlg` files in a
   folder on disk; plays the one picked in the game's own dialogue screen
   (which the game hands in); has a switch for each character the scene's
   `@if` blocks test; jumps to any text box; shows a line's direction;
   reloads a file when it is saved; and shows the format's problems and
   the game's own checks' problems in the window, keeping the last good
   version playing until the file is fixed. It also takes the game's own
   named switches, so a game with a player-named hero can show both forms
   of a token without the engine knowing what the token is.
5. **An example from a game that isn't tactics:** a visual novel's
   writer checks a route's scene where a friend may have left the town
   (`@if friend` / `@else`), reads the scene again from its middle after
   rewording one line, and sees the line break where the box is full.

This is the second half of 2585, split off by 2585's size estimate
(its step 2). **From 2585 (done):** the pieces this screen is built on:

- `Scene::conditions` (`shuyi-assets`): the characters a scene's `@if`
  blocks test, once each, in order.
- `ParsedScene::text_boxes(present, replies)` and
  `ParsedScene::text_box_at(line, present, replies)`: each text box's
  source line and step as the scene plays, and which one a typed line
  number falls in.
- `DialoguePlayer::from_text_box(scene, hooks, present, text_box,
  replies)` (`shuyi-ui`): a player started at a text box, everything
  before it applied as if read, the last music pending; and
  `DialoguePlayer::direction()`.
- `shuyi_app::read_watched_folder` and `Platform::watches_files`
  (ADR-0095, amending ADR-0071 §1): a folder watched in an optimized
  build without debug tools, its changes reaching the game's modules
  (`Module::reloaded`); `Harness::watch_files` for tests.

A dialogue screen is the game's, not the engine's
(`crates/ui/src/dialogue.rs`). Hello, Grid has one
(`examples/hello-grid/src/screens.rs`, `Talk`, over `files/intro.dlg`).

- **Either order with 3082** (demo editions): if 3082 is done, the
  book's section says a preview program is built as an edition that
  leaves the game's own scripts out, so the program holds only the
  scenes in its folder; if not, nothing to do (3082 adds the sentence if
  this lands first).
- **3092** (files dropped on the window) would give a web preview; Out.

## Nick input

None.

## Scope

**In:**
- **The preview screen**, in `shuyi-ui` (beside `dialogue`, or with the
  ready-made screens in `crates/ui/src/screens/`), split as every screen
  is: logic, a plain-data view the tests read, a skin that paints it
  (`AGENTS.md` rule 9). The view is a view a skin reads (ADR-0039 kind
  3: `#[non_exhaustive]` with a `new`). Its parts:
  - the scene list, by file and in file order, a file with problems
    marked;
  - playing: the screen the game hands in for the picked scene, those
    present and the first text box; over it, when asked, a strip with
    the file, the line number and the line's direction
    (`DialoguePlayer::direction`);
  - the switches: one per character of `Scene::conditions`, all present
    at first, and the game's own named switches; changing one plays the
    scene again from the same text box (`from_text_box`);
  - the lines: the scene's text boxes (`ParsedScene::text_boxes`:
    speaker, the start of the words, the source line), to jump to one;
    and a jump to a typed line number (`text_box_at`);
  - the problems: file, line and message, from the format's checks
    (`scripts_from_files`) and the game's, the fix where a message
    carries one.
- **What the game hands in**, a trait (or a struct of functions; the
  session picks and says why), with no game word in it: the game's
  `Directive` type; its checks over the files (problems with file and
  line); its own switches; and "build the screen that plays this scene
  for these present, from this text box, with these switch values".
- **Reload:** the preview reads its folder with `read_watched_folder`;
  a module of it hears `Module::reloaded`, reads the files again and
  rebuilds the scenes; the scene playing goes on from the same text box
  if it still has one, else from its start; a file that no longer parses
  keeps its last good scenes and shows its problems.
- **Controls** are actions (`AGENTS.md` rule 8): the preview declares the
  ones it needs, and a game binds them, or maps them onto actions it
  already has.
- **Hello, Grid** gets a preview (a second binary or a start option) over
  its `files/` dialogue, and Harness tests of it.
- **Docs:** the `shuyi-dialogue` skill (`skills/shuyi-dialogue/SKILL.md`)
  gains a section "A preview for your writer"; the book's chapter where
  dialogue is (or a new section), its code a snippet in
  `site/snippets/`.
- The API change is additive; if a change turns out to break, a
  `Breaking:` line and migration block (ADR-0038). The game's PR that
  moves the pin follows this one (game#0764).

**Out (do not do):**
- A web preview (3092 gives dropped files; a later ticket).
- An editor that changes the file from the window (2506–2508, parked):
  the writer edits in their own text editor.
- Who is present in a real playthrough, or any rule about it: the
  preview passes a `Present`; the game says what it means.
- Playing a scene at the moment a game would play it (a turn, an event):
  the game's job.
- Voice clips.
- Changing how a folder is watched: 2585 did it (ADR-0095).
- Anything of one game: no class, unit, character, story or file of the
  game's (rule 4).

## Implementation steps

1. Read ADR-0071, ADR-0095, `player.rs` (`from_text_box`, `direction`),
   `present.rs` (`conditions`, `text_boxes`), `folder.rs`
   (`read_watched_folder`), Hello, Grid's `Talk`, and a ready-made screen
   (`crates/ui/src/screens/options.rs`) for the logic / view / skin split.
   Check whether 3082 is done.
2. The trait the game hands in, and the screen's logic and view, with
   unit tests reading the view.
3. The skin.
4. The reload module, and its Harness tests through the headless
   backend's `change_file` with `watch_files(true)` and debug tools off.
5. Hello, Grid's preview and its Harness tests; check it through the MCP
   session tools (`start`, `step`, `scene`, `frame_png`).
6. The skill and the book; the gates.

## Acceptance criteria

- [x] On Hello, Grid's preview, with a character switched off, the
      scene plays its `@else` line (Harness test reading the view).
- [x] A changed file, through the headless backend's script, is played
      on the next frame from the same text box; a broken one keeps the
      last good scene and lists its problem with file and line (Harness
      tests).
- [x] A jump to a typed line number shows the text box it falls in
      (Harness test reading the view).
- [x] The preview's controls are actions a game binds; no key is named
      in code or text (`input-actions` skill).
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the screen's logic over its view (the list, the switches, the
  lines, the problems).
- Integration: Hello, Grid's preview through the Harness and the
  headless backend's changed files.

## Completion notes

What was done:

- **`shuyi_ui::dialogue::preview`** (new): the `DialoguePreview` screen,
  split as every screen is: logic (`preview.rs`), a plain-data view
  (`view.rs`: `PreviewView` and its rows, each `#[non_exhaustive]` with a
  `new`, ADR-0039 kind 3) and a glyph skin (`glyph.rs`). Four tabs:
  Scenes (by file, in file order, a file with problems marked),
  Switches (one per character of `Scene::conditions`, all there at
  first; the game's own; and the strip's), Lines (`text_boxes`, and a
  line box that takes typed digits or the cursor and plays from
  `text_box_at`), Problems (file, line, message, and the fix: the part
  of a check's message after its first `; `). While a scene plays the
  game's screen takes the input but Cancel, which brings the tabs back
  over it; a strip on the top row shows the file, the line and the
  line's direction.
- **What the game hands in** is a trait, `PreviewGame<G>`: its
  `Directive` and `Rules` types, `rules`, `problems` (defaulted),
  `switches` (defaulted) and `play`, which builds the game's dialogue
  screen for a `Play` (scene, present, text box, switch values;
  `Play::player` starts a `DialoguePlayer` there). A trait, not a struct
  of functions, because the game's lines and rules are types, which a
  struct of functions would carry as type parameters on every use (said
  on the trait). The screen it builds implements `PreviewScreen`
  (`showing()`, `Showing::of(&player)`), so the preview follows the text
  box the writer is on.
- **Reload:** the `DialogueFolder` module keeps the folder's `.dlg` files
  on the context (`DialogueFiles`, with a version) and reads them again
  when a `.dlg` of it changes (`Module::reloaded`); the screen takes them
  on its next update. Each file is parsed alone, so a file that no longer
  parses keeps its last good scenes and lists its problems; the format's
  checks (`scripts_from_parsed`) and the game's run over the files that
  parsed. The scene playing goes on from the same text box if it still
  has one, else from its start; a scene taken out stops.
- **Controls** are the six `UiAction` moves a game already maps its
  actions onto, and typed digits in the line box (the text box's fixed
  keys finish, delete and cancel it, as the options' number box).
- **Hello, Grid:** `hello_grid::preview` (`HelloPreview`, its rules,
  `game`, `start`), a second binary `hello-grid-preview`
  (`src/bin/preview.rs`, reading its folder with `read_watched_folder`),
  and a `--preview` start option for its tools, so the MCP session
  starts it. `files/intro.dlg` gained an `@if guide` block around its
  last line; the game plays everyone there, so its scene is unchanged.
  Checked through the MCP session tools: the tabs, the strip, the guide
  sent away and the `@else` line in the game's text box.
- **Tests:** 22 unit tests over the view on the toy game (the list, the
  switches, the lines and the line box, the problems, reload, a broken
  file, the skin's rows) and 7 Harness tests of Hello, Grid's preview
  (`tests/it/preview.rs`), the reload ones with debug tools off and
  `watch_files(true)`.
- **Docs:** the book's chapter 6 section *A preview for your writer*,
  its code `site/snippets/src/preview.rs` (with a Harness test) and the
  words in `site/snippets/files/en.ron`; the `shuyi-dialogue` skill's
  section *A preview for your writer*; Hello, Grid's README. The
  preview's text keys (`preview.*`) are in the toy's and Hello, Grid's
  English.

Deviations:

- No ADR: the one choice the ticket left open (trait or struct of
  functions) is said on the trait; the rest follows ADR-0040 (a module
  with state on the context), ADR-0071/0095 (reload) and ADR-0039.
- The strip is turned on and off from a row of the Switches tab rather
  than by an action of its own, so the preview needs no action a game
  lacks.
- The Lines tab and the reload follow each reply choice's first reply,
  as `text_boxes` and `from_text_box` do with no replies given.
- `cargo shuyi web` builds every binary of a package, so
  `hello-grid-preview` builds for the web too: there, with no folder to
  read, it plays the game's own scene file and reads nothing again (a
  writer's web preview stays 3092's dropped files).
- The unit tests that change a layer's bytes are `std`-only
  (`Layer::replace` is). Seen, not done: on this machine the `shuyi-ui`
  tests without `std` don't build, for `frame/raster/card/tests.rs`
  (`vec!` and `String` unimported), a file this ticket doesn't touch.
- A short-path game (`BasicGame`, `run_game`) can't start a preview
  binary without leaving the short path, so the book shows the preview's
  parts and its test, and names Hello, Grid's binary for the program:
  **2587** gives the short path a preview binary in one call.

Follow-up tickets: 2587 (a dialogue preview on the short path). 3082's
clause now says it adds the editions sentence to the book's section.

The game: game#0764 needs this; the game's PR that moves the pin is
game#0764's own (it writes the game's `PreviewGame` and binary).

Breaking: hello-grid: tests/golden/hello-grid.ron "asset en.ron", "asset intro.dlg" and "fingerprint files": the example's English has the preview's words and its scene an `@if` block; only the example's table moves, and a game changes nothing.

```migration
since: 0.1.0
what: Hello, Grid's golden table moved with its files; nothing in a game changes.
old: the example's table before
new: the example's table blessed
before: |
  // nothing
after: |
  // nothing
```

No other break: every change to the engine's public API is an addition.
