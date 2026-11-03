---
id: "2247"
title: "Loader errors in one call each: a bundle file's text or its not-found error, an error at an entry's id line, and a duplicate id with its code"
type: feature
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: low
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2247 — Loader errors in one call each

## Context

A request from the game.

1. **The game ticket:** game#1223, content loaders on the engine's error
   helpers
   (`github.com/Zafnok/visions-of-shuyi/blob/main/tickets/open/1223-content-loaders-on-the-engines-error-helpers.md`).
   It is not a blocker of this one.
2. **The job in one sentence:** a loader of a hand-written data file
   fetches it, reports a problem at the entry it is about, and refuses a
   second entry with the same id, each in one call and each with its
   problem code.
3. **The friction numbers:** no friction review: found by the
   game/engine boundary audit of 2026-11-03. The game hand-writes the
   fetch ("file not found in asset bundle" built with `ContentError::new`,
   so without `SH2003`) in 11 loaders, the "at the entry's `id:` line"
   closure 12 times, and the duplicate-id check 6 times, none with
   `SH2009`; about 90 lines in all. This engine's `crates/assets/src/audio.rs`
   writes the same two id patterns (`Check::err`, `new_cue`, `credit`).
4. **The generic shape:** in `shuyi_assets`, a bundle file's shown name
   and text or the right error; an error moved to the line naming a
   needle; and a small per-file helper for RON lists of entries keyed by
   an `id:` field (the shape `audio.rs`, `credits.rs` and `tip.rs` read).
5. **Examples from games that aren't tactics:** a farming game's
   `crops.ron` (two crops with `id: "turnip"`) and a rhythm game's song
   list, each reporting the duplicate at its line with `SH2009`, so
   `shuyi explain` and the tool contract see a coded problem.

## Nick input

None.

## Scope

**In:**
- `Bundle::source(&self, path) -> Result<(String, &'static str),
  ContentError>` (`crates/assets/src/bundle.rs`): the shown name
  (`display_path`) and the text; a missing file is
  `ContentError::not_found` (`SH2003`), a file that isn't UTF-8 is
  `NotText`'s error (`SH2002`). `ron::load_ron` uses it.
- `ContentError::near(self, source, needle) -> Self`
  (`crates/assets/src/error.rs`): at `line_of(source, needle)` with no
  column when found, unchanged when not.
- `error::Entries<'a>` (name to taste; `#[non_exhaustive]`, built by
  `Entries::new(file, source, kind)`): `err(id, message)` is the error at
  the line holding `id: "<id>"`; `duplicate(id)` is "duplicate {kind} id
  \"{id}\"" there with `codes::DUPLICATE`; `insert_unique(map, key, id,
  value) -> Option<ContentError>` inserts into a `BTreeMap` only when
  `key` is absent and otherwise gives `duplicate(id)`.
- `audio.rs`: `Check::err` on `Entries::err`, `new_cue` and `credit` on
  `duplicate`; their messages and lines unchanged, now coded `SH2009`.
- No API break (additions only). The game's PR that moves the pin
  follows this one (game#1223).

**Out (do not do):**
- An empty-id check: the messages' articles differ ("a cue", "an art");
  callers keep their one line.
- Codes and lines on `.dlg` and cinematic errors, and moving the keymap's
  `line_of_quoted` (2237's scope).
- Errors of the audio manifest's ducking rules, which have no id (2343).
- The game's use (game#1223).

## Implementation steps

1. Read `error.rs`, `bundle.rs` (`file`, `display_path`, `NotText`),
   `ron.rs` (`load_ron`) and `audio.rs` (`Check`).
2. Add `Bundle::source` and move `load_ron` onto it.
3. Add `ContentError::near` and `Entries`, with doc examples.
4. Move `audio.rs`'s `Check` onto them; its tests'
   messages (`duplicate_ids_are_refused`) stay as they are.
5. Expected size: about 50 production lines added, about 15 removed from
   `audio.rs` and `ron.rs`; about 60 lines of tests.

## Acceptance criteria

- [ ] `Bundle::source` on a missing file gives code `SH2003` and the
      message `NOT_FOUND`; on a non-UTF-8 file, `SH2002`; on a file, its
      shown name and text (unit tests).
- [ ] `near` finds the first line holding the needle and leaves an error
      alone when there is none.
- [ ] `insert_unique` keeps the first value and gives one `SH2009` error
      at the second entry's `id:` line.
- [ ] `audio.rs`'s duplicate cue and credit ids carry `SH2009`; its
      existing tests pass with the same messages.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `Bundle::source` (three cases), `near`, `Entries::err`,
  `duplicate`, `insert_unique`; `audio.rs`'s existing tests, plus one
  asserting the code.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
