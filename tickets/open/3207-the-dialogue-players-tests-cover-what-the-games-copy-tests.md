---
id: "3207"
title: "The dialogue player's tests cover what the game's copy tests: three property tests and the cases of replies, music and absent speakers it lacks"
type: infra
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
---

# 3207 — The dialogue player's tests cover what the game's copy tests

## Context

A request from the game.

1. **The game ticket:** game#1218, which cuts the game's
   `crates/ui/src/dialogue/tests.rs` (1,184 lines) down to its own hooks
   once this lands
   (`github.com/Zafnok/visions-of-shuyi/blob/main/tickets/open/1218-the-dialogue-tests-keep-only-the-games-hooks.md`).
   It is not a blocker of this one.
2. **The job in one sentence:** the engine's own tests prove everything
   the dialogue player promises, so no game has to re-prove it.
3. **The friction numbers:** no friction review. The game/engine boundary
   audit of 2026-11-03 found that game#1112 moved `DialoguePlayer<H>`
   here with its unit tests ported to the toy hooks, but the game kept
   its own copy of the machine's tests, and some of them test what
   `crates/ui/src/dialogue/player/tests.rs` (431 lines) does not. About
   650 game test lines wait on this.
4. **The generic shape:** tests of `shuyi_ui::dialogue::DialoguePlayer`
   on `Plain` and the toy `Lamps` hooks. Nothing new in the API.
5. **An example from a game that isn't tactics:** a farming game whose
   villagers talk in `.dlg` scenes with replies and `@if` parts for who
   came to the festival relies on the same promises (each line once, in
   order, whatever is replied; skipped or read, the same music).

## Nick input

None.

## Scope

**In** (tests only, in `crates/ui/src/dialogue/player/tests.rs`):
- Three property tests, over scenes of random steps on `Plain`:
  every reply path shows the rest once in order, with the picked
  reactions between; read or skipped, a scene leaves its last `@music`;
  advancing until finished shows every speech and narration once, in
  script order, then no text.
- The cases the file lacks (the game's names in brackets, so game#1218
  can check them off): advancing a finished scene leaves the player
  equal to before (`advancing_a_finished_scene_does_nothing`); a scene
  with no text starts finished with its portraits and caption shown
  (`a_scene_without_text_starts_finished`); a reaction that places other
  speakers on both sides leaves them there at the rejoin
  (`the_reaction_leaves_the_portraits_as_they_are`); every reply rejoins
  to the same views (`every_reply_rejoins_the_same_way`); a `@music` line
  after the last text box is reached as the scene ends
  (`music_is_asked_for_when_its_line_is_reached`); a reaction's `@music`
  plays only for that reply (`music_in_a_reaction_plays_only_for_that_reply`);
  `@if` inside a reaction and a choice inside an `@if`
  (`a_block_plays_the_part_for_who_is_there`); the part not played puts
  no portrait up and asks for no music (`the_part_not_played_leaves_no_trace`);
  skipping stops only at the choices of the parts played
  (`skipping_follows_the_parts_played`); a scene whose every line is an
  absent speaker's starts finished
  (`a_scene_with_nothing_to_say_for_those_there_starts_finished`).
- No API change, so no `Breaking:` line. The game's PR that moves the
  pin and deletes its copies follows this one (game#1218).

**Out (do not do):**
- `player.rs` itself: a case that fails is a bug; fix it here and say so
  in the completion notes.
- The game's tests of its own hooks (its lead and name tokens, its place
  and opening lines, its shipped scenes): they stay in the game.
- A backlog of seen lines over the player (parked 2766's note).

## Implementation steps

1. Read the game's tests at the game's pin
   (`crates/ui/src/dialogue/tests.rs`, lines 221-300, 409-557, 642-797
   and 800-988) for the shapes. Rewrite each with neutral ids
   (`guide`, `scout`, as `HALL` and `ASK` use) and the `.dlg` text the
   file already parses with `scenes`, or `Scene` literals where a test
   needs an odd scene.
2. Add the strategies `arb_step` (caption, place, clear, say, narrate,
   music), `arb_step_or_choice` (a choice of 1-3 replies whose reactions
   are `arb_step`s) and the oracles `expected_texts` and `last_music`
   (the latter two over `Step::text` and `Step::Music`). Bound every walk
   (a `MAX_BOXES` constant) so a player that never finishes fails.
3. Add the three `proptest!` tests, each with its own doc comment (the
   game's copy has one doc comment stranded on the wrong test, at its
   lines 754-757).
4. Add the cases of the Scope's list; where an existing test already
   holds most of one (`it_plays_each_text_box_…` asserts only
   `is_finished` after a second advance), extend it instead of adding a
   near copy.

## Acceptance criteria

- [ ] `cargo test -p shuyi-ui dialogue::player` runs the three property
      tests and each case of the Scope's list, and passes.
- [ ] The file's tests name no game's characters, places or tokens.
- [ ] `git diff --stat` shows no production file changed (or the
      completion notes name the bug fixed).
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the cases above.
- Property: the three walks, on `Plain`.
- Snapshot / integration: none.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
