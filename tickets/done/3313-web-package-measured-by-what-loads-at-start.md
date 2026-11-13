---
id: "3313"
title: "shuyi bench measures a web package by what the page loads before the game starts"
type: feature
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: done
blocked_by: []
nick_input: none
completed: 2026-11-12
---

# 3313 — A web package measured by what loads before the game starts

## Context

1. **The game ticket:** `game#1249`
   ([github.com/Zafnok/visions-of-shuyi/blob/main/tickets/open/1249-the-web-package-within-its-size-budget.md](https://github.com/Zafnok/visions-of-shuyi/blob/main/tickets/open/1249-the-web-package-within-its-size-budget.md)).
2. **The job:** say how long a player waits before a web game starts,
   not how big its upload is.
3. **No friction review:** a measure that counts the wrong thing.
4. **The generic shape:** `shuyi bench` (ticket 3008, ADR-0081) gates
   each target's package against its profile's `package_bytes`. For the
   web it takes the whole zip (`export-report.json`'s first size). The
   web profile's budget, "25 MB download (prov.)" beside a start in 5 s at
   25 Mbit/s ([`docs/targets/web.md`](../../docs/targets/web.md)), stands
   for what a player waits for before the game starts. The web build
   ships `web.music` and `web.voice` as loose files beside the page, and
   the engine fetches each track and clip when it is first played
   (`load_music` and the voice loader run in a coroutine). itch.io and
   GitHub Pages serve an uploaded HTML5 zip as separate files, so those
   folders are never waited for. A game with 75 MB of music measured
   80 MB against the 25 MB budget while its page downloads 10.6 MB
   before the first frame.
5. **A game that isn't tactics:** a platformer with a soundtrack of
   forty tracks and voiced cut scenes ships the same folders beside its
   page and waits for none of them.

## Nick input

None.

## Scope

**In:**
- `bench`'s web package number: the bytes of the export's folder
  (`<base>/`, what the zip holds) outside the folders `web.music` and
  `web.voice` name, the files the page loads before the game's first
  frame. The whole zip's size is kept beside it in `bench-report.json`
  (`whole_bytes`) and in the table, so the upload stays visible.
- When the export's folder isn't there, the whole zip is measured and
  a limit line says so.
- A package over its budget says what was measured.
- `docs/targets/web.md` and `docs/targets/README.md` say what the web's
  `package_bytes` counts.

**Out (do not do):**
- Changing the export report (the contract): `bench` reads the folder
  the export left.
- Any other target's measure: their packages load whole.
- Raising any budget.

## Implementation steps

1. `bench::plan` keeps the package paths of `web.music` and `web.voice`.
2. `bench::packages` sums a web export's folder outside them; `Package`
   gains `whole_bytes`.
3. The report's table and the over-budget line show both.
4. Docs.

## Acceptance criteria

- [x] A web export whose music folder is over the budget and whose other
      files are under it is within its budget, its whole zip reported.
- [x] A web export without its folder is measured whole, with a limit.
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `packages` on a web export with music and voice folders, and
  without its folder; the Markdown table and the over line with
  `whole_bytes`.

## Completion notes

- `bench::plan` keeps `web.music` and `web.voice` as package paths
  (`Plan::later`); `bench::packages` measures a web export by its
  folder's files outside them (`at_start`) and keeps the zip's size in
  `Package::whole_bytes` (`bench-report.json`, omitted when `None`).
- Without the export's folder (or with a report naming none) the whole
  zip is measured, with a limit line.
- The table's package rows have a "Size (gated)" and a "Whole package"
  column and a line on what the web's gated size is; an over-budget
  line names the package "(what loads at start)".
- The measure sums the files' bytes as they are, not compressed: the
  page downloads them one by one.
- Docs: `docs/targets/web.md`, `docs/targets/README.md`.
- Book: `targets-and-export.md`'s "Size and speed" says what the web's
  package size counts.
- No break: `bench` is internal to `shuyi-cli`, and the export report
  (the contract) is unchanged.
- Behaviour: `shuyi bench`'s web package number drops the music and
  voice folders; its table gains a column.
- Game: game#1249 moves the pin and drops the web package from its known-over list.
