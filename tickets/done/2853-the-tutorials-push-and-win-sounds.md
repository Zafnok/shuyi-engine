---
id: "2853"
title: "The book's box pusher plays a sound on a push and on a win, made with shuyi sfx, and chapter 08 says how"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: done
blocked_by: ["2277"]
nick_input: none
completed: 2026-11-25
---

# 2853 — The book's box pusher plays a sound on a push and on a win, made with `shuyi sfx`, and chapter 08 says how

## Context

Split from 2277 at its own split point (past 1,000 lines): the
tutorial's sounds and the book's part on them.

- **From 2277 (done):** `shuyi sfx <preset | file.sfx.ron> [--seed N]
  [--out <file.sfx.ron>] [--wav <file.wav>]`, a contract op that needs
  no game (`crates/cli/src/contract/sfx.rs`, `docs/contract.md`), the
  template's `AGENTS.md` row for it, and the `shuyi-new-game` skill's
  section *Sounds* (`skills/shuyi-new-game/SKILL.md`), which is the model
  for the chapter's words. Hello, Grid's sounds are `.sfx.ron` files
  (`examples/hello-grid/files/sfx/`), and Snake plays a cue on a rules'
  event (`examples/snake/src/screens.rs`, `ctx.audio.play_sound(EAT)` on
  `Event::Ate`; `examples/snake/files/audio.ron`).
- **From 2225 (done):** `.sfx.ron` sound effects (ADR-0144), the
  template's four menu sounds (`templates/shuyi-default/base/crates/game/files/audio.ron`
  and `files/sfx/*.sfx.ron`, the `sfx` feature of `shuyi-assets` in the
  game crate's `Cargo.toml` and the workspace's `shuyi-assets` line),
  read by `BasicData::load` and played through `run_game`'s sound reader.
  Docs: `docs/formats.md`, *Sound effects (`.sfx.ron`)*.
- **What is left:** the tutorial's box pusher (`site/tutorial/08-polish`,
  `09-test`, `10-ship`, ticket 2803) is silent: its menus play the
  widgets' cues (`Menu::handle_with_sound` in `levels.rs`), which its
  files leave without sounds, and a push or a win plays nothing. Chapter
  08 (`site/content/learn/book/box-pusher/08-polish.md`) says nothing of
  sound.

## Nick input

None.

## Scope

**In:**

- `site/tutorial/08-polish`, and `09-test` and `10-ship` after it, get
  two sounds made with `shuyi sfx`: one on a push (the rules'
  `Event::Pushed`, from `Puzzle::apply` in `Board::send`) and one on a
  win (the move that makes `puzzle.outcome()` `Some`), each a cue of
  `files/audio.ron` the board plays through `ctx.audio.play_sound`. An
  undo plays nothing. **Either order with 2850:** if 2850 is done, steps
  08–10 already have the four menu sounds (`files/audio.ron`,
  `files/sfx/*.sfx.ron`, the `sfx` feature): add only the push and win
  cues and files; if not, add the menu sounds to 08–10 as the template
  has them, with the two cues, and 2850 brings steps 01–07 up to them.
- Each step's tests: a test in `tests/it/main.rs` that every cue the
  board and the menus play is in the manifest and `format_problem`
  accepts what `audio::playable` gives for each file (the template's
  `every_sound_the_menus_play_is_in_the_manifest_and_plays` is the
  model), and a test in `tests/board.rs` that a push plays the push cue,
  a walk none, and the winning push the win cue (the Harness's
  `sounds()`).
- `site/content/learn/book/box-pusher/08-polish.md`: a part on sound
  after the moves counter: what a `.sfx.ron` is (a few numbers, rendered
  as the game loads), `cargo shuyi sfx coin --out
  crates/game/files/sfx/push.sfx.ron`, `--wav` to listen, which fields
  to change (`docs/formats.md`'s table, linked), the cue in `audio.ron`,
  and the board's lines (by `snippet` anchors of step 08), with its row
  in the page's file table, a quick check, and the glossary's rows for
  any new bold term (`cargo xtask check-book`). The board's `send`
  anchor stays one whole item.
- The steps' `Cargo.lock` files follow (`--locked` in CI).
  `site/tutorial/tries.toml`'s pictures don't change (sound isn't
  drawn).

**Out (do not do):**

- `shuyi sfx`, the format, its presets (2277, 2225);
- steps 01–07 (2850);
- music, a sound on a refused step, or a sound option of the menus.

**Estimated size:** about 500 lines (code 60, tests 150, docs 90, data
200).

## Implementation steps

1. Read `site/README.md` (*The rules*, *Exercises*: a step's exercises
   splice checks into a copy of it, `site/exercises/08-polish.rs`), the
   template's game crate, and Snake's sounds.
2. Make the two sounds with `cargo shuyi sfx` (a push: something short
   and low, from `hit` or `blip`; a win: `power_up` or `coin`), listen to
   each with `--wav`, and write them into `files/sfx/` of step 08, with
   their cues in `files/audio.ron`.
3. `board.rs`: `send` takes the `Ctx`, plays the push cue on
   `Event::Pushed` and the win cue on the move that wins; `update` passes
   its `ctx`. Copy to 09 and 10.
4. The tests, in each step; build and test each step (`site/tutorial`'s
   README), and `cargo xtask exercises`.
5. Chapter 08's part; `cargo xtask check-book`.
6. Run the gates.

## Acceptance criteria

- [x] Steps 08 to 10 play a sound on a push and on a win, and their
      menus' cues: each step's test checks the cues are in its manifest
      and `format_problem` accepts what `audio::playable` gives for each
      file, and a board test checks which cue each move plays.
- [x] Each step builds and passes its tests with `--locked`, and its
      exercises pass (`cargo xtask exercises`).
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- The two tests above in each of steps 08, 09 and 10.

## Completion notes

- **Done.** Steps `08-polish`, `09-test` and `10-ship` play `push` on a
  push and `win` on the move that wins, through `ctx.audio.play_sound`
  in `Board::send`, which now takes the `Ctx` (`update` passes its own).
  The sounds are `files/sfx/push.sfx.ron` (from the preset `hit`, made
  lower and shorter: noise from 1,200 to 300 Hz, 100 ms) and
  `files/sfx/win.sfx.ron` (the preset `power_up` as it is, 550 ms),
  with their cues in `files/audio.ron`. Each step has the tests the
  ticket asks: `every_sound_the_board_and_the_menus_play_is_in_the_manifest_and_plays`
  (`tests/it/main.rs`, the template's test with `PUSH` and `WIN` added)
  and `a_push_plays_its_sound_a_walk_none_and_the_winning_push_the_win`
  (`tests/board.rs`). Chapter 08 has a section *Sounds* after the moves
  counter, its file table's rows, a quick check and the glossary's new
  row **Cue** (and step 8 on *Sound effect*).
- **2850 was not done**, so this took the clause's *if not* branch:
  steps 08–10 also get the template's four menu sounds
  (`files/sfx/{move,select,cancel,cursor}.sfx.ron` and their cues), the
  `sfx` feature of `shuyi-assets` in the game crate's `Cargo.toml` and
  the workspace's `shuyi-assets` line, as the template writes them.
  2850's clause now says so ("**From 2853 (done):** …"), and it brings
  steps 01–07 up to them.
- **Deviations.**
  - Steps 01–07 get the two Cargo lines too (the `sfx` feature and the
    workspace's `shuyi-assets` line), and their locks follow, though the
    ticket's **Out** left those steps to 2850: `cargo xtask
    check-devcontainers` (2839) fails a step that resolves `shuyi-assets`
    with other features than step 01, as the book's Codespace would
    build the engine twice. Their files and sounds stay 2850's, which
    now says the feature is there.
  - The move that wins plays `win` alone, not `push` and then `win`:
    one sound a move, and the win's says more. The chapter's check asks
    which a reader hears.
  - The chapter starts the two files from `hit` and `power_up` (the
    ticket's step 2), not from `coin` as its Scope's example command
    had it.
  - The chapter says a reader's game already has the menu sounds, which
    is true of a game `shuyi new` makes today; the book's steps 01–07
    don't yet (2850).
  - New anchors, changing nothing else in the steps: `sounds` and
    `steps` in step 08's `board.rs`, `board` in its `audio.ron`
    (`sounds` in 09 and 10 too). `send` stays one whole item.
  - Listening: the session can't hear, so each file was checked with
    the MCP `sfx` tool (`--wav`): its length (100 ms, 550 ms), its peak
    and no problems. Nick may want to play them once.
- **Follow-up:** 2279, `sfx::to_text` writes CRLF line ends on Windows
  (`ron`'s `PrettyConfig` default), so `shuyi sfx --out` there writes a
  file of mixed line ends; found while making these files, which were
  written by hand with LF.
- `site/tutorial/tries.toml`'s pictures don't change (sound isn't
  drawn).
- No break: the engine's code is unchanged; only the book's steps,
  chapter and glossary change.
- Book: chapter 08 (`site/content/learn/book/box-pusher/08-polish.md`)
  and the glossary.
- Skills: none, no schema, template, format or problem code changed;
  the `shuyi-new-game` skill's *Sounds* already says how.
