---
name: shuyi-dialogue
description: Write a game's talk on Shuyi Engine in `.dlg` files (scenes, speakers placed left and right, narration, reply choices, `@if` blocks, music changes), read them through the format registry, check them against the game's rules, and play them one text box at a time in the engine's text box. Use when adding or changing a scene of dialogue, a choice the player answers, the screen that plays a talk, or the checks a game's scenes keep to. Not for a timed sequence of shots with no reading (shuyi-cinematic), or for a screen with no talk in it (shuyi-screen).
license: MIT OR Apache-2.0
metadata:
  version: "1.0"
---

# Dialogue

A game's talk is text in its files, never in its code. A `.dlg` file
holds scenes: who stands on the left and right, what each says, the
narration, and the replies the player picks from. The engine reads the
file and checks it. A `DialoguePlayer` walks a scene one text box at a
time, and a screen paints that in the engine's text box.

## When to use it

- Writing or changing a scene: a talk before a stage, a shopkeeper, a
  sign post, a choice with its reactions.
- A game that has no screen to play a scene yet.
- A program for a writer to check their scenes in (a preview).
- A rule the game's scenes must keep (the box's size, who may speak,
  the expressions each character has).

## When not to

- Shots timed to the music, with nothing to read on: `shuyi-cinematic`.
- A menu, a title or a screen with no scene in it: `shuyi-screen`.
- The keys that read on and pick a reply: `shuyi-input`.

## The format

```text
# A comment.
@scene gate_talk
@caption The east gate, dusk
@left mentor neutral
@right scout happy
mentor: Is the road clear?
scout[sad]: Not since noon.
  Riders came over the hill.
> Rain begins to fall.
@music storm
@choice
* earnest: Then we wait.
  mentor: Agreed.
* wry: Then we get wet.
@endchoice
@if scout
scout: I'll keep watch.
@else
> Nobody keeps watch.
@endif
@end
```

- `@scene <id>` … `@end`: one scene. Its id is unique across every file
  of the game. Ids (scenes, characters, expressions) are lowercase
  letters, digits and `_`.
- `@left <id> <expression>` and `@right …` stand a character on that
  side, in place of whoever was there; `@left clear` empties the side.
- `<id>: text` is said by a character on screen; `<id>[sad]: text`
  changes its expression first. An indented line goes on with the line
  above.
- `> text` is narration: nobody says it.
- `<id> (whispering, frightened): text` and `> (slowly): text` say how a
  line is spoken, for a voice tool or an actor. `@delivery <direction>`
  does it for the lines after it, until `@delivery clear`, the next
  `@delivery` or the end of the scene, or of the `@if` part or reaction
  it is in; a line's own direction wins. A direction is never shown, and
  changing it keeps the line's id.
- `@caption <text>` names the place or the time until the next one.
- `@music <cue>` changes the music when the scene reaches it, and
  `@music stop` fades it out. It stays after the scene.
- `@choice` … `@endchoice`: the replies, each `* <tone>: <text>` with
  its reaction indented under it. The tone is for writers and isn't
  shown. Every reply rejoins the scene after the block.
- `@if <id>` … `@else` … `@endif`: lines that play only while that
  character is there (`Present`), and the lines played instead.
- Text holds any character but a control character. `check` names a
  character the font atlas can't draw, with its file and key; `font-atlas`
  adds it if the font has it.

Every speech, narration and reply line has a line id,
`<scene id>_<8 hex>`, a hash of who says it and its English. Translations
and voice clips attach to it, so rewording a line gives it a new id, and
moving it doesn't.

A game adds lines of its own (a place drawn behind the talk, a cut to a
map) with a type that implements `Directive`: the parser hands it every
`@name` it doesn't know, and the scene keeps the steps it makes in order.

## Steps

1. **Write the file** in `crates/game/files/` and read it:

   ```bash
   cargo shuyi import crates/game/files/talk.dlg
   ```

   `import` reads it through the engine's `.dlg` format: every syntax
   and structure error (an unknown line, a bad id, a block left open, a
   missing `@end`, text that isn't ASCII), each with its line.

2. **Read it into the game's data.** On the short path the game's own
   part of its data is a stage's `Own`, which its `own` function reads
   from the files. Load the file through the registry, then check it:

   ```rust
   pub fn own(bundle: &Bundle) -> Result<Own, Vec<ContentError>> {
       let file = Formats::standard().load::<DialogueFile>(bundle, TALK)?;
       let scripts = scripts_from_parsed([file], &Rules)?;
       Ok(Own { scenes: scripts.table })
   }
   ```

   `scripts_from_parsed` runs `check_scene` on each scene, then the
   game's own `SceneRules::scene_problems`, then `check_duplicates`
   across files. `scripts.table` is a `DialogueTable`: `get(id)` gives a
   scene. Every problem reaches `cargo shuyi check`.

3. **Give the game's rules** with a type that implements `SceneRules`.
   Two methods are needed, and the rest have defaults:

   - `limits`: a `DialogueLimits`, the cells per line of the box
     (`box_width`), its lines per page (`box_lines`), the longest line
     (`max_text`) and reply (`max_option`), how many replies a choice
     offers (`min_options`, `max_options`), and the most lines in one
     reaction (`max_reaction_texts`);
   - `tokens`: the game's own `{tokens}` in text (`NoTokens` for none);
   - and if it wants: `can_speak`, `expression_problem`,
     `speech_problem`, `music_problem` and `scene_problems`.

   The checks then find who speaks while not on screen, characters and
   expressions the game doesn't know, text too long for the box, replies
   too many or too few, an `@if` that leaves the screen different either
   way, music cues the game lacks, and two lines whose ids would clash.

4. **Play it** in a screen ([`example/talk.rs`](example/talk.rs) is a
   whole one):

   - `DialoguePlayer::new(&scene, Plain, &Present::Everyone)` starts a
     scene at its first text box. `Plain` is the hooks of a game with no
     lines or tokens of its own, not even names; `Named::new(names)` is
     those of a game whose only tokens are `{n:<id>}` names. A game with
     tokens or lines of its own implements `DialogueHooks` (`fill` its
     tokens, and its name tokens with `Names::substitute`; `direct` its
     lines).
   - `current()` is a `View`: the portraits on the `left` and `right`,
     which side is the `speaker`, the `text`, the `caption`, whether it
     is `narration`, and the `choices` while a choice is open.
   - `advance()` moves to the next text box; at a choice it waits for
     `choose(n)`. `is_choosing()` and `is_finished()` say where it is.
   - `DialoguePlayer::from_text_box(&scene, hooks, &present, n,
     &replies)` starts at text box `n` (0 is the first), everything
     before it applied as if read and its last `@music` pending; at each
     choice on the way it takes the next of `replies`, else the first.
     `direction()` is how the line on screen is said (its `( … )`, or the
     `@delivery` in force), which the player never sees: a writer's
     preview or a voice tool shows it.
   - For a preview: `scene.conditions()` lists who the `@if` blocks
     test, a switch each; `parsed.text_boxes(&present, &replies)` lists
     each text box's source line and step, and `parsed.text_box_at(line,
     &present, &replies)` which one a typed line falls in. A preview
     program reads its folder with `shuyi_app::read_watched_folder`, so
     an optimized build plays each file the writer saves.
   - `TextBox::new(&text, width, lines, ctx.widths())` pages and types
     a line out, wrapped in cells (a wide character is two; Japanese
     breaks between characters, never before `。` or after `「`):
     `type_on(input.dt(), ctx.settings().text_speed.chars_per_s())` each
     frame, `hold` while the key that goes on is held, and `confirm()`,
     which answers `Revealed`, `NextPage` or `Done`. On `Done`, advance
     the player.
   - The screen's view is a `TextView` (the page, how much is shown,
     `narration`, the replies as a `MenuView`, and a `WaitView` while a
     page waits), and `dialogue::text_box::glyph::paint` paints it in a
     `BoxPlace`. Tests of what is said read the view, not the cells.
   - The text the screen adds is the game's, by key: the speaker's name
     (`character.<id>` in `files/en.ron`) and the hint that goes on
     (`"talk.next": "{Confirm} next"`, which names the player's key).

5. **Look at it** headless: `cargo shuyi run --script "Enter Enter"
   --frames 1` answers with the frame as text (`/result/frame`), and
   `frame-png` draws it. The MCP session's `step` and `frame_png` do the
   same between calls.

## A preview for your writer

A writer with no source checks their scenes in a **dialogue preview**: a
program built from the game's code that lists the scenes of the `.dlg`
files in a folder, plays the one picked in the game's own dialogue
screen, and plays it again from the same text box a second after the
file is saved. Its tabs: the scenes (a file with problems marked); a
switch per character an `@if` block tests, the game's own switches and
a strip with the file, line and direction; the text boxes, and a box to
type a line number; and the problems, with file, line and fix. A file
that no longer parses keeps its last good scenes playing.

1. Implement `shuyi::ui::dialogue::preview::PreviewGame` on a type of
   the game's: its `Directive` type, its `Rules` (the `SceneRules` the
   checks use), and `play`, which builds its dialogue screen from
   `play.player(hooks)`. Optional: `problems` (checks over every scene)
   and `switches` (named switches, read back with `play.switch(i)`).
2. Implement `PreviewScreen` on that screen: `showing()` is
   `Showing::of(&self.player)`.
3. The program: a second binary of the game, `src/bin/preview.rs`,
   whose `main` is one call,
   `shuyi::app::run_dialogue_preview(game(), Path::new(&folder),
   YourPreview)`. It runs the game's window, files and settings with
   `DialoguePreview` as the first screen, the game's modules, a
   `DialogueFolder` over the folder (read with `read_watched_folder`,
   watched in an optimized build too) and no debug screens; a folder that
   can't be read shows why in the window. A `.dlg` file dropped on the
   window joins the folder. On the web, which has no folders, the
   preview starts with none: the writer opens its page and drops their
   `.dlg` files on it, and drops one again after changing it. A game on
   the long path builds
   it itself: `BasicGame::preview_start(folder, preview)`, or
   `DialoguePreview::new(&ctx, YourPreview)` as the root screen,
   `.with_debug_screens(false)`,
   `.with_module(DialogueFolder::new(layer))` and a system hook that
   hands a drop's files to `shuyi::ui::dialogue::preview::drop_scenes`.
4. Give the `preview.*` text keys words in the language file (the book's
   chapter 6 lists them). `preview.note.no_scenes_drop` is the empty
   Scenes tab's note where files can be dropped; without it the tab
   shows `preview.note.no_scenes` there too.
5. Test it through the Harness: `game().preview_harness(layer,
   YourPreview)`, `h.watch_files(true)`, then
   `h.change_file(layer, "scene.dlg", bytes)` and one frame, or drop a
   file with `h.drop_file("scene.dlg", bytes)` (an input script's
   `drop` step); read
   `h.screen::<DialoguePreview<_, _>>()?.view(h.ctx())`.

Its controls are the six `UiAction` moves (Cancel brings the tabs back
over the scene), so the game's keymap needs nothing new.

## Keys and languages

- A line never tells the player a key. `check-keys` reads `.dlg` files
  and fails an instruction to press one; the screen names keys through
  the keymap (`{Confirm}` in its text).
- A game on the short path has its English only (`files/en.ron`). The
  line ids are what a language pack's dialogue text and voice clips
  attach to, in a game whose data gives packs.
- `@music` lines play only in a game with music (its own `GameData` and
  audio manifest): the player keeps the last one reached for
  `take_music()`.

## When it fails

- `import` or `check` answers `ok: false` with each problem in
  `result.errors`: its `file`, its `line` and its `message`. A `.dlg`
  problem carries no `code` of its own: fix the line it names.
- `SH2003`: the file isn't where `own` loads it from: the path is from
  the files folder (`talk.dlg`, not `files/talk.dlg`).
- `SH3005`: the game's tool program didn't build: the answer ends with
  the compiler's output.
- A debug build panics on a text key its English lacks: add the key to
  `files/en.ron`.

## Worked example

[`example.ron`](example.ron) runs on a new `blank` game. It writes a
scene ([`example/talk.dlg`](example/talk.dlg)), a stage that reads and
checks it and a screen that plays it ([`example/stages.rs`](example/stages.rs),
[`example/talk.rs`](example/talk.rs)), and the screen's text. `import`
reads the file as dialogue, and `check` passes. `run` reads the first
line, the narration, both replies, the first reply's reaction, and the
title again after the second. It drops the `@endchoice`, and `import`
names line 9. It makes a line too long for the box, and `check` names
line 11. Fixed, every gate passes.
