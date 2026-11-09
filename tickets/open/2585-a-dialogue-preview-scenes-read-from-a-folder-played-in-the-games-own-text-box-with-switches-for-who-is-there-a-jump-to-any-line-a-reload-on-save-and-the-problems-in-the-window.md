---
id: "2585"
title: "A dialogue preview: scenes read from a folder, played in the game's own text box, with switches for who is there, a jump to any line, a reload on save and the problems in the window"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2585 — A dialogue preview

## Context

1. **The game ticket:** game#0764, *A script preview program for
   writers*
   (`github.com/Zafnok/visions-of-shuyi/blob/main/tickets/open/0764-a-script-preview-program-for-writers.md`;
   the game's repository may be private by the time this is read, see
   game#0153). The game is looking for a writer who doesn't program, and
   needs to hand them a program, not the source.
2. **The job in one sentence:** a writer who has no source and no Rust
   saves a `.dlg` file and, a second later, sees the scene as the game
   shows it: any scene, from any line, with any character present or
   gone, and the file's problems listed instead of a crash.
3. **Friction numbers:** no friction review: a missing feature.
4. **The generic shape:** a **dialogue preview**, a small program a game
   builds from its own code. It lists the scenes of the `.dlg` files in a
   folder on disk; plays the one picked in the game's own text box,
   portraits and music (the game's dialogue screen, which the game hands
   in); has a switch for each character the scene's `@if` blocks test,
   so each branch can be seen; jumps to any text box of the scene;
   shows a line's direction (the `(…)` and `@delivery` that the player
   never sees); reloads a file when it is saved; and shows the format's
   problems, and the game's own checks' problems, in the window, keeping
   the last good version playing until the file is fixed. It also takes
   the game's own switches (a named switch and its values that the game
   reads when it builds its screen), so a game with a player-named hero
   can show both forms of a token without the engine knowing what the
   token is.
5. **An example from a game that isn't tactics:** a visual novel's
   writer checks a route's scene where a friend may have left the town
   (`@if friend` / `@else`), reads the scene again from its middle after
   rewording one line, and sees the line break where the box is full. A
   farming game's writer does the same for a festival scene where a
   villager may have moved away.

What there is:

- **The format**, `shuyi_assets::dialogue` (`crates/assets/src/dialogue.rs`):
  `parse_dlg`, `ParsedScene` with `Lines`, the 1-based source line of
  each step (`crates/assets/src/dialogue/parse.rs:12`, `:26`);
  `Present` and `Scene::resolved`, which drops every `@if` block's other
  part (`crates/assets/src/dialogue/present.rs`); `scripts_from_files`
  and `check_scene` for the format's problems. Nothing lists the
  characters a scene's `@if` blocks test.
- **The player**, `shuyi_ui::dialogue::DialoguePlayer`
  (`crates/ui/src/dialogue/player.rs`): `new(scene, hooks, present)`
  (`:221`) starts at the first text box; `advance`, `choose`,
  `skip_to_choice` (`:380`), `take_music`. It can't start at a later
  text box, and doesn't say the direction of the line on screen.
- **Hot reload** (2220, done; ADR-0071): `Platform::changed_files`,
  `shuyi_app::read_folder` (`crates/app/src/folder.rs:79`) and the poller
  (`crates/app/src/watch.rs`), `shuyi_assets::hot::Hot`
  (`crates/assets/src/hot.rs:69`), the reload errors drawn over the
  frame. All of it works only with debug tools on
  (`shuyi_app::watched_layer`, `folder.rs:125`, returns `None` without
  `shuyi_ui::DEBUG_TOOLS`), so a shipped build never polls. A preview
  program given to someone is an optimized build, and must watch its
  one folder.
- **A dialogue screen** is the game's, not the engine's
  (`crates/ui/src/dialogue.rs`: "A game's dialogue screen puts them
  together with its portraits and whatever it draws behind the talk").
  Hello, Grid has one (`examples/hello-grid/src/screens.rs`, `Talk`,
  over `files/intro.dlg`).
- **Either order with 3082** (demo editions): nothing here depends on
  it; if 3082 is done, the book's section says a preview program is
  built as an edition that leaves the game's own scripts out, so the
  program holds only the scenes in its folder; if not, nothing to do.
- **3092** (files dropped on the window) would give a web preview; Out.

## Nick input

None.

## Scope

**In:**
- **What a scene tests:** `Scene::conditions(&self) -> Vec<CharacterId>`
  (`shuyi-assets`), each character an `@if` block tests, in reactions and
  nested blocks too, once each, in the order they first appear.
- **The player from any text box:** a way to start a `DialoguePlayer`
  at the n-th text box of the scene as it plays for those present, with
  every portrait, caption, game directive (through its hooks) and music
  line before it applied, as if read; the last music line reached is
  pending, as after a skip. A choice on the way takes a reply from a
  list the caller gives (the first reply when the list runs out). Also:
  which text box a source line falls in (from `ParsedScene`'s `Lines`),
  and `DialoguePlayer::direction()`, the direction of the line on
  screen (its own, or the `@delivery` in force).
- **The preview screen**, in `shuyi-ui` (beside `dialogue`, or with the
  ready-made screens in `crates/ui/src/screens/`), split as every screen
  is: logic, a plain-data view the tests read, a skin that paints it
  (`AGENTS.md` rule 9). Its parts:
  - the scene list, by file and in file order, a file with problems
    marked;
  - playing: the screen the game hands in for the picked scene, those
    present and the first text box; over it, when asked, a strip with
    the file, the line number and the line's direction;
  - the switches: one per character of `Scene::conditions`, all present
    at first, and the game's own named switches; changing one plays the
    scene again from the same text box;
  - the lines: the scene's text boxes (speaker and the start of the
    words, with the source line), to jump to one; and a jump to a typed
    line number;
  - the problems: file, line and message, from the format's checks and
    the game's, the fix where a message carries one.
- **What the game hands in**, a trait (or a struct of functions; the
  session picks and says why), with no game word in it: the game's
  `Directive` type; its checks over the files (problems with file and
  line); its own switches; and "build the screen that plays this scene
  for these present, from this text box, with these switch values".
- **A folder watched in any build that asks:** a game opts in for one
  folder it names (for example `read_folder` with a watch flag, or a
  `BasicGame` setting); the poller then runs in an optimized build too,
  for that folder only. A build that doesn't opt in still never polls
  (2220's test with the counting fake clock stays). ADR-0071 gets a
  section saying so.
- **Reload:** a saved file is read again and the scenes rebuilt; the
  scene playing goes on from the same text box if it still has one,
  else from its start; a file that no longer parses keeps its last good
  scenes and shows its problems.
- **Controls** are actions (`AGENTS.md` rule 8): the preview declares the
  ones it needs, and a game binds them, or maps them onto actions it
  already has.
- **Hello, Grid** gets a preview (a second binary or a start option) over
  its `files/` dialogue, and a Harness test of it.
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
- Anything of one game: no class, unit, character, story or file of the
  game's (rule 4).

## Implementation steps

1. Read ADR-0071, 2220's completion notes, `player.rs`, `present.rs`,
   `parse.rs`, `folder.rs`, `watch.rs` and Hello, Grid's `Talk`. Check
   whether 3082 is done.
2. Write the size estimate (about 1,350 lines with tests: conditions
   50, the player's start and direction 240, the watch opt-in 120, the
   screen 500, Hello, Grid 50, docs 90, this ticket 300). Above about
   1,500, split the screen and Hello, Grid off as their own ticket
   blocked by this one, and tell game#0764 to wait for both.
3. `Scene::conditions`; the player's start at a text box, the line to
   text box lookup and `direction()`.
4. The watch opt-in and its ADR-0071 section.
5. The screen (logic, view, skin) and the trait the game hands in.
6. Hello, Grid's preview; the skill and the book; the gates.

## Acceptance criteria

- [ ] `Scene::conditions` lists each tested character once, nested and
      in reactions included (unit tests).
- [ ] A player started at text box 3 shows the same view (portraits,
      caption, text) as one advanced twice from the start, and has the
      same pending music (property test over generated scenes).
- [ ] On Hello, Grid's preview, with a character switched off, the
      scene plays its `@else` line (Harness test reading the view).
- [ ] A changed file, through the headless backend's script, is played
      on the next frame from the same text box; a broken one keeps the
      last good scene and lists its problem with file and line (Harness
      tests).
- [ ] A build that opts in polls its folder with debug tools off; one
      that doesn't never polls (tests with the counting fake clock).
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `conditions`; the line to text box lookup; `direction()`.
- Property: starting at a text box equals advancing to it.
- Integration: Hello, Grid's preview through the Harness and the
  headless backend's changed files.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
