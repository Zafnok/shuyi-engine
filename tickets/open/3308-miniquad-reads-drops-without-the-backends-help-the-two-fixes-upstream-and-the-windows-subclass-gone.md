---
id: "3308"
title: "miniquad reads drops without the backend's help: the two fixes upstream, and the Windows subclass gone"
type: infra
milestone: "Engine: demos"
model: sonnet-5 / gpt-6.1-terra
effort: low
status: todo
blocked_by: ["3307"]
nick_input: none
completed:
---

# 3308 — The drop fixes upstream

## Context

**From 3307 (done):** `shuyi-app` answers `WM_DROPFILES` itself on
Windows, through a subclass of miniquad's window
([ADR-0104](../../docs/adr/0104-drops-on-windows-read-by-the-backend-itself-through-a-subclass-of-miniquads-window.md)),
and the vendored `crates/cli/web/mq_js_bundle.js` carries a local patch
that sizes a dropped file's name in UTF-8 bytes. Both work around bugs of
miniquad 0.4.11:

- `src/native/windows.rs`, `WM_DROPFILES`: `std::fs::read(&path).unwrap()`
  panics on a folder or an unreadable file, and the handler never calls
  `event_handler.files_dropped_event()`, so macroquad never hears a drop
  on Windows. Its path buffer is `MAX_PATH` long, and it never calls
  `DragFinish`.
- `js/gl.js`, `window.ondrop`: the name is sized by `file.name.length`
  (UTF-16 units) and written as UTF-8 into that many bytes, so a name
  beyond ASCII comes through cut short (`niño ドゥーム.txt` arrives as
  `niño ドゥ\0`, checked by hand in 3307).

3307 didn't open the upstream PRs: a PR on another project is sent from
Nick's GitHub account, so it waits on his yes.

## Nick input

None for the code. Opening a PR on `github.com/not-fl3/miniquad` is sent
from Nick's account; the session asks him before sending each.

## Scope

**In:**
- A PR to miniquad for each bug, with the fix 3307 made here:
  - Windows: skip a path `std::fs::read` fails on, call
    `files_dropped_event` after the loop, `DragFinish` the drop;
  - web: `new TextEncoder().encode(file.name)` for the name's bytes.
- When a miniquad release (and the macroquad bundle that matches it) has
  both: take out `crates/app/src/drops/windows.rs` and the `winapi`
  dependency, drop the local patch's note in `crates/cli/web/README.md`
  and its test `a_dropped_files_name_is_sized_in_utf8_bytes`, and write
  an ADR superseding ADR-0104.

**Out (do not do):**
- Anything of ADR-0103's event or limits.

## Implementation steps

1. Ask Nick before opening each upstream PR; open them; link them here.
2. Wait for a release with both; bump through the `dependency-upkeep`
   skill's `backend` group.
3. Take the workarounds out, and try both by hand as 3307's notes say.

## Acceptance criteria

- [ ] Both upstream PRs are open, linked here.
- [ ] After a miniquad release with both fixes, the subclass and the
      bundle's local patch are gone, and a folder dropped on Windows and
      a non-ASCII name dropped on the web still work.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- By hand, as 3307's completion notes say.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
