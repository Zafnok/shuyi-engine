---
id: "2233"
title: "The bundle's folder loader file by file, as the registry's is, and one helper that turns a file-by-file map into the whole map or every error"
type: feature
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2233 — The bundle's folder loader file by file

## Context

A request from the game.

1. **The game ticket:** game#1186, content folders with one rule for a
   broken file
   (`github.com/Zafnok/visions-of-shuyi/blob/main/tickets/open/1186-content-folders-one-rule-for-a-broken-file.md`).
   It is not a blocker of this one.
2. **The job in one sentence:** load every file of a folder with a parse
   function and get each file's result, so a broken file doesn't hide
   the others, or, when the caller wants it, the whole map or every
   error, from either of the engine's two folder loaders.
3. **The friction numbers:** game 1169's row (estimate 30, measured 71,
   2.4×, accepted): "the chapter and battler loaders check the files
   that parsed together … even when another file broke, which
   `load_each` (all or nothing) can't give, so the game keeps a 17-line
   `bundle::parsed` … A third such loader would make it an engine variant
   that keeps what parsed." Two and a half hours later 2219 gave that
   variant, but only on the registry (`Formats::load_each`), and its Out
   list left `Bundle::load_each` as it was. The game's retro of its last
   50 PRs (2026-10-22) found the result: two loaders named `load_each`
   with opposite rules for a broken file, and the game adapting each to
   the other (`bundle::parsed` one way, three hand fold-backs the other).
4. **The generic shape:** `Bundle::load_each` gets a file-by-file
   sibling, returning each file's `Result` by stem, as
   `Formats::load_each` does; and one function turns a file-by-file map
   into `Ok(whole map)` or `Err(every error, in path order)`, usable on
   either loader's output. `Bundle::load_each` itself stays as it is
   (all or nothing), now written on the two.
5. **Examples from games that aren't tactics:** a farming game loading
   its crops folder, whose crops reference each other: it parses each
   file, then checks the references over every crop that parsed, and
   reports a broken file and a dangling reference in one run.

## Nick input

None.

## Scope

**In:**
- `crates/assets/src/bundle.rs`: `Bundle::load_each_file(dir, ext,
  parse) -> BTreeMap<String, Result<T, Vec<E>>>` (name to taste; it says
  "each file"), a file that isn't UTF-8 being that file's `Err` with a
  `NotText`. `Bundle::load_each` keeps its signature and behaviour, on
  top of it.
- A free function (or a method on a small type) `all_or_errors(map) ->
  Result<BTreeMap<K, T>, Vec<E>>`, in the assets crate, documented on
  both loaders.
- `Formats::load_each`'s docs point to the helper for callers that want
  all or nothing.
- Doc examples and tests.

**Out (do not do):**
- Changing `Formats::load_each` (2219) or `Bundle::load_each`'s results.
- Reading subfolders (neither loader does).
- The game's use of it (game#1186).

## Implementation steps

1. **Read** `bundle.rs` (`load_each`, `LoadFile`, `NotText`, its tests
   and doc examples), `format.rs` (`Formats::load_each`), 2208 and 2219
   in `tickets/done/`.
2. Write `load_each_file`, then rewrite `load_each` as
   `all_or_errors(load_each_file(…))` with its tests unchanged.
3. The helper, with tests on both loaders' outputs.
4. Docs.

## Acceptance criteria

- [ ] `Bundle::load_each`'s existing tests and doc examples pass
      unchanged.
- [ ] `load_each_file` on a folder with one good file, one broken file
      and one non-UTF-8 file gives one `Ok` and two `Err`s.
- [ ] `all_or_errors` on that map gives both errors in path order; on an
      all-good map, the whole map.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `load_each_file`, `all_or_errors`, and `load_each` on top of them.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
