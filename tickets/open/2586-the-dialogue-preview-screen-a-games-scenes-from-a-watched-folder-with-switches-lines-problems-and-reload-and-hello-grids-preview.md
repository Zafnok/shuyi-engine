---
id: "2586"
title: "The dialogue preview screen: a game's scenes from a watched folder, with switches, lines, problems and reload, and Hello, Grid's preview"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: ["2585"]
nick_input: none
completed:
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

- [ ] On Hello, Grid's preview, with a character switched off, the
      scene plays its `@else` line (Harness test reading the view).
- [ ] A changed file, through the headless backend's script, is played
      on the next frame from the same text box; a broken one keeps the
      last good scene and lists its problem with file and line (Harness
      tests).
- [ ] A jump to a typed line number shows the text box it falls in
      (Harness test reading the view).
- [ ] The preview's controls are actions a game binds; no key is named
      in code or text (`input-actions` skill).
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the screen's logic over its view (the list, the switches, the
  lines, the problems).
- Integration: Hello, Grid's preview through the Harness and the
  headless backend's changed files.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
