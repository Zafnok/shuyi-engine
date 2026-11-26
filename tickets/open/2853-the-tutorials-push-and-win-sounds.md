---
id: "2853"
title: "The book's box pusher plays a sound on a push and on a win, made with shuyi sfx, and chapter 08 says how"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: ["2277"]
nick_input: none
completed:
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

- [ ] Steps 08 to 10 play a sound on a push and on a win, and their
      menus' cues: each step's test checks the cues are in its manifest
      and `format_problem` accepts what `audio::playable` gives for each
      file, and a board test checks which cue each move plays.
- [ ] Each step builds and passes its tests with `--locked`, and its
      exercises pass (`cargo xtask exercises`).
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- The two tests above in each of steps 08, 09 and 10.

## Completion notes

*(Filled in by the session that completes the ticket.)*
