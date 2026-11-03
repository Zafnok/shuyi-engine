---
id: "3091"
title: "The engine's built-in target profiles readable by id from a no_std crate, one copy for the engine and its games"
type: feature
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: low
status: todo
blocked_by: []
nick_input: none
completed:
---

# 3091 — Built-in target profiles readable from no_std

## Context

A request from the game.

1. **The game ticket:** game#1227, which drops the game's copy of
   `targets/playdate.ron`
   (`github.com/Zafnok/visions-of-shuyi/blob/main/tickets/open/1227-the-playdate-target-profile-from-the-engine.md`).
   It is not a blocker of this one. The game ticket game#1170 (the Playdate front end on
   the engine's backend) also reads the profile and says what it does
   in either order.
2. **The job in one sentence:** a game or tool asks for "the engine's
   profile of target `playdate`" and gets it, in a `no_std` crate too,
   without copying the file.
3. **The friction numbers:** no friction review: the game/engine
   boundary audit of 2026-11-03 found it. The game keeps
   `crates/ui/src/playdate_target.ron`, the engine's `targets/playdate.ron`
   byte for byte bar its comments, because a game can't `include_str!` a
   file of a git dependency; its own 12-line `playdate_facts` parses it,
   and an xtask test keeps it equal to `shuyi_cli`'s `engine_profiles()`
   (std, the CLI only). The engine itself reads its profiles through
   private `include_str!`s: in production `crates/playdate/src/backend.rs:25`
   and `crates/app/src/facts.rs` (`PROFILES`), and in seven test files
   (`playdate/src/input.rs`, `ui/src/platform_facts.rs`,
   `ui/src/input/fit/tests.rs`, `ui/src/input/pad/tests.rs`,
   `ui/tests/it/screens.rs`, `assets/src/pack/tests.rs`,
   `platform/src/app/tests.rs`).
4. **The generic shape:** `targets/` embedded once in `shuyi-assets`,
   which is `no_std` + `alloc` without its `std` feature, read by id.
5. **Examples from games that aren't tactics:** a puzzle game with its
   own Playdate front end that sets its screens' `PlatformFacts` from the
   Playdate's profile; a roguelike's tool that checks its HUD fits the
   Steam Deck's display.

## Nick input

None.

## Scope

**In:**
- `crates/assets/src/target.rs`: `pub const BUILTIN: &[(&str, &str)]`,
  each built-in profile's id and source (`include_str!` of
  `targets/<id>.ron`, so `no_std` works), and `pub fn builtin(id: &str)
  -> Result<TargetProfile, Vec<ContentError>>` (an id it doesn't know is
  an error naming the known ones), shown as `targets/<id>.ron`.
- `crates/ui/src/platform_facts.rs`: `PlatformFacts::builtin(id) ->
  Result<Self, String>`, `from_profile` of it, errors joined.
- Every site above on `BUILTIN` / `builtin`.
- A test that `BUILTIN` names every file of `targets/` (beside the
  CLI's `engine_profiles`, which keeps `include_dir` for its listing).
- Additive: no `Breaking:` line. The game's PR that moves the pin
  (game#1227) follows this one.

**Out (do not do):**
- A game's own or overriding profiles (they stay files of the game).
- Moving `targets/` into the crate for publishing (the workspace has
  `publish = false`); if that changes, `BUILTIN`'s paths move with it.
- Changing any profile's contents.

## Implementation steps

1. **Read** `target.rs` (`TargetProfile::from_source`), `platform_facts.rs`,
   the `include_str!` sites above and `cli/src/contract/target.rs`
   (`TARGETS`, `engine_profiles`).
2. Add `BUILTIN` and `builtin`, with doc examples, and
   `PlatformFacts::builtin`.
3. Move the sites onto them; delete their constants and test tables.
4. The book's targets chapter: one line on `builtin`.

Expected size: about 40 production lines added, about 25 removed, and
about 40 test lines removed.

## Acceptance criteria

- [ ] `builtin("playdate")` equals `from_source` of `targets/playdate.ron`;
      `builtin("nope")` is an error naming `playdate` (tests).
- [ ] `cargo check -p shuyi-assets --no-default-features` builds with
      `builtin`.
- [ ] `grep -rn 'include_str!(.*targets/' crates` finds only
      `target.rs`.
- [ ] Every file of `targets/` is in `BUILTIN` (a test).
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `builtin` by id, the unknown id, `PlatformFacts::builtin`.
- Integration: the existing backend and app facts tests, on `builtin`.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
