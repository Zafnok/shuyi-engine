---
id: "2208"
title: "A folder loader on the bundle: parse every file of one extension in a folder into a map by name, with every error"
type: feature
milestone: After MVP
model: sonnet-5
effort: low
status: done
blocked_by: []
nick_input: none
completed: 2026-10-20
---

# 2208 — A folder loader on the bundle

## Context

- **The game ticket:** game#1169
  ([`1169-content-loaders-on-the-engines-folder-loader-and-wrap.md`](https://github.com/Zafnok/visions-of-shuyi/blob/main/tickets/open/1169-content-loaders-on-the-engines-folder-loader-and-wrap.md))
  waits for this one. Found by the game's first engine-friction sweep,
  game#1153.
- **The job:** "load every file of a kind from a folder, keyed by its
  name, and report every broken one at once".
- **Friction:** the game's cinematics (1614 lines against 1050, 1.5×)
  and seven more of its data loaders, outcome 1: the same 25-line loop
  in eight files of the game's content crate (about 180 lines; the
  distinctive line is the error `"file is not valid UTF-8"`). Ranked
  fifth of the sweep's findings. Each copy lists the folder with
  `Bundle::files_in`, strips the folder and the extension to get the
  name, reads the text with `Bundle::file` (an error if it isn't
  UTF-8), parses with the kind's own function, and gathers either a
  `BTreeMap<name, T>` or every error.
- **The generic shape:** `Bundle::load_each<T, E>(&self, dir, ext,
  parse: impl FnMut(&LoadFile) -> Result<T, Vec<E>>) -> Result<BTreeMap<String,
  T>, Vec<E>>` where `LoadFile` gives the name (`stem`), the display path
  and the text, and `E: From<NotText>` (or a `ContentError` the engine
  already has) for a file that isn't UTF-8. The session fixes the names
  and the error type.
- **Outside tactics:** a farming game's folders of crops and recipes;
  a platformer's folder of levels.

## Nick input

None.

## Scope

**In:**
- The method on `Bundle` (and on whatever free-function front the
  engine offers for the default bundle, as `files_in` has), documented,
  with a doc example.
- The engine's own loaders of the same shape use it (search the engine
  for `files_in(`).

**Out (do not do):**
- Hot reload and disk folders (1124, 2207).
- Format plugins (2201); this takes a plain parse function.
- Any game's loaders.

**For the game:** additive. The game's PR that moves its pin follows
this one (game#1169).

## Implementation steps

1. Read `crates/assets/src/bundle.rs` and the engine's uses of
   `files_in`.
2. Write the method and its tests.
3. Move the engine's own loops onto it.

## Acceptance criteria

- [x] Unit: a folder with two good files, one bad one and one of
      another extension gives the two errors of the bad one only; with
      no bad file, the map of two by name.
- [x] Unit: a file that isn't UTF-8 gives one error naming its display
      path.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit, as above.

## Completion notes

Added `Bundle::load_each(dir, ext, parse)` in `crates/assets/src/bundle.rs`, with `LoadFile` (stem, display path, text) and `NotText` (converts into `ContentError`; any `E: From<NotText>` works). Every error is gathered, in file order. No other engine loop uses `files_in(` and the engine has no free-function front for the default bundle, so nothing else moved. No follow-ups. Game PR to move the pin: game#1169 (not yet opened). Nick: nothing.
