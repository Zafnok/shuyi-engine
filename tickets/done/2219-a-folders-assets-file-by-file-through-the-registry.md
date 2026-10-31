---
id: "2219"
title: "A folder's assets file by file through the registry: each file of a kind in one folder, read or with its errors"
type: feature
milestone: After MVP
model: implementation
effort: low
status: done
blocked_by: []
nick_input: none
completed: 2026-10-20
---

# 2219 — A folder's assets file by file through the registry

## Context

1. **The game ticket:**
   [game#1173](https://github.com/Zafnok/visions-of-shuyi/blob/main/tickets/done/1173-the-games-loaders-on-the-engines-format-registry.md)
   moved the game's loaders onto the format registry (ADR-0007,
   ADR-0019, ticket 2201). Its maps, cinematics and dialogue are folders
   of files, read with a check after each file
   (`cinematic::check`, `dialogue::scripts_from_parsed`).
2. **The job:** read every file of one kind in a folder, keep the ones
   that read, and report the errors of the ones that didn't, so the
   checks that run after still see every good file.
3. **The friction numbers:** no friction over the ratio (game#1173 was
   under 2×); a workaround signal. The game wrote its own
   `formats::load_in` (about 15 lines,
   `crates/content/src/formats.rs`, marked
   `// engine-friction: engine#2219`), because
   [`Formats::load_under`](../../crates/assets/src/format.rs) does two
   things a loader with checks after can't use:
   - it gives the assets only when **every** file reads: one file with
     a syntax error hides every other file, so the checks after report
     nothing on them, and a writer fixing one file at a time sees their
     errors only one round later;
   - it reads the folder **and every folder below it**, where the game's
     loaders always read one folder: a subfolder holding another kind
     with the same extension (every RON file is `.ron`) would be read
     as the wrong kind.
4. **The generic shape:** `Formats::load_each::<K>(bundle, dir)`: each
   file directly inside `dir` whose extension a format of kind `K`
   claims, by path, with its asset or its errors:
   `BTreeMap<&'static str, Result<K, Vec<ContentError>>>`.
5. **An example outside tactics:** a farming game's `crops/` folder, one
   `.ron` per crop, each checked after against the seasons file; a typo
   in `turnip.ron` should not hide that `pumpkin.ron` names a season
   that doesn't exist.

## Nick input

None.

## Scope

**In:**
- `Formats::load_each` in `crates/assets/src/format.rs`, as above.
  Additive: no API breaks.
- `Formats::load_under` keeps its behaviour; its doc says when to use
  which (all-or-nothing over a tree, or file by file in one folder).
- `docs/formats.md`: one line under "An asset is asked for by id".

**Out (do not do):**
- Changing `Formats::load_under` or `Bundle::load_each` (ticket 2208,
  text files with a parser closure, no registry).
- The game's change: game ticket 1176 deletes its `formats::load_in`
  and moves the pin, after this one.

## Implementation steps

1. Add `load_each` beside `load_under`: `bundle.files_in(dir)`, keep the
   paths `self.claims::<K>(path)` says yes to, and read each with the
   private `read` (so limits apply as in `load`).
2. Tests in `crates/assets/src/format/tests.rs` with a table bundle:
   two files of the kind (one broken), one of another extension and one
   in a subfolder: the map has the two, the broken one with its errors
   naming its file as the bundle shows it; the others are not there.
3. The doc lines.

## Acceptance criteria

- [x] `Formats::load_each` returns each claimed file directly in the
      folder, read or with its errors, by path.
- [x] A broken file leaves the others read (a test).
- [x] Files in subfolders and of other extensions are not read (a test).
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the cases of step 2; an empty or missing folder gives an empty
  map.

## Completion notes

- Added `Formats::load_each::<K>(bundle, dir)`: each file directly in
  `dir` that a format of `K` claims, by path, with its asset or its errors
  (`BTreeMap<&'static str, Result<K, Vec<ContentError>>>`), read through
  the private `read` so the registry's limits apply. Additive, no API
  break.
- `load_under`'s doc now says it is all or nothing over a tree and points
  to `load_each`; `docs/formats.md` has a line for both.
- One test: a broken file leaves the other read, with its errors naming
  its file as the bundle shows it; a subfolder's file and another
  extension are not read; an empty, missing or file-named folder gives an
  empty map.
- No deviations, no follow-up tickets. The game's side is game ticket
  1176 (delete its `formats::load_in`, move the pin).
