---
id: "3355"
title: "Parked: Desktop toys and several windows on the winit backend: transparent, undecorated, click-through and always-on-top windows moved by the game, and a frame's surfaces in windows of their own"
type: feature
milestone: "Engine: in-house platform"
model: opus-5.5 / gpt-6.1-sol
effort: high
status: blocked
blocked_by: ["3326", "3328"]
nick_input: none
completed:
---

# 3355 — Desktop toys and several windows

> **Parked** (Nick, 2026-11-22, `docs/decisions.md`): this builds on the winit backend (3326) and its window facts (3328), which wait for the MVP with the rest of the in-house platform layer (2026-10-19). Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

The Bevy coverage audit of 2026-11-28 (`docs/ROADMAP.md`, *Bevy's game
coverage*) found two kinds of desktop game Shuyi's window can't make:

- **Desktop toys and companions**: desk pets, idle games docked at the
  screen's edge, overlays. They need a window with a see-through
  background, no decorations, a level above other windows, clicks that
  fall through where nothing is drawn, and a drag the game starts.
  Bevy's `desk_toy` showcase does all of it
  (`examples/showcase/desk_toy.rs`, `examples/window/transparent_window.rs`,
  `window_drag_move.rs`, `examples/ui/window_fallthrough.rs`;
  `crates/bevy_window/src/window.rs`: `transparent`, `decorations`,
  `window_level`, `CursorOptions::hit_test`).
- **Several windows**: a second-screen map or a game master's screen,
  hot-seat play with information hidden from the other player, a tool
  beside the game on another monitor
  (`examples/window/multiple_windows.rs`). 3328's **Out** lists
  "several windows" and "moving the window from the game"; 3023's
  surfaces live inside one window.

Both are platform work: only `shuyi-app` knows windows, and a screen
asks `ctx.platform` whether the target has them (ADR-0060), never a
`cfg!`.

## Nick input

None.

## Scope

**In:**
- **Window styles:** the game's project file gains a window style
  (`Normal`, or `Toy { level: Top | Bottom | Normal, taskbar: bool }`),
  and the clear colour may be transparent.
- **Click-through:** over no hit in the frame's hit list (ADR-0134), the
  window lets the click through to the desktop; over a hit, it takes
  it.
- **A drag the game starts:** a screen request asks `shuyi-app` to start
  a drag-move of the window (from a `UiAction`, never a key).
- **Several windows:** 3023's surface layouts gain `Windows`: each
  surface opens in its own winit window with its own pointer, focus and
  remembered size (3044's record per surface). Closing a secondary
  window hides its surface. A target without windows falls back to
  `Stacked`.
- **Platform facts:** the target profile says whether each is
  supported; `shuyi check --target` refuses a style the target lacks
  (the web, consoles, the Playdate).
- **A desk-toy example** whose state is a tick or frame-clock animation
  and whose look is a skin; frame-png draws it on a checkerboard.

**Out (do not do):**
- the macroquad backend (it can't do these);
- per-pixel shaped windows on systems without a compositor;
- several windows on the web.

**Estimated size:** about 1,400 lines (code 650, tests 450, docs 300).

**Split:** past 1,000 lines, split off several windows (the fourth
bullet) into a new ticket.

## Implementation steps

1. Read 3326's and 3328's code, ADR-0060, ADR-0134, 3023 and 3044.
2. Add the window style to the project file and apply it in
   `shuyi-app`'s winit path.
3. Add click-through from the hit list and the drag request.
4. Add the platform facts and the check.
5. Add the `Windows` layout and per-surface state.
6. Write the desk-toy example and the docs page.

## Acceptance criteria

- [ ] On Windows, a `Toy` window with a transparent clear shows the
  desktop behind unpainted pixels (the capture mode of 2018 reads alpha
  0 there).
- [ ] A click over no hit reaches the window below (a manual check
  recorded in the completion notes, with the OS named).
- [ ] `shuyi check --target web` refuses a `Toy` style with its code.
- [ ] A two-surface `Windows` layout opens two windows; closing the
  second hides its surface and the game goes on.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] The skill pack (`skills/`) is updated for a changed schema, template, format or problem code, or the completion notes have a `Skills:` line saying why not (`cargo xtask docs-rules`, 3534).
- [ ] Guides in `docs/guides/` updated, if this adds a direction, a module or a move (ADR-0009 §5).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: style parsing, the click-through decision from a hit list, the
  check.
- Snapshot / integration: the desk toy's frame-png; a Harness run of two
  surfaces.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
