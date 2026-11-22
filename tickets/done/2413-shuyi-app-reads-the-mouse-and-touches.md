---
id: "2413"
title: "shuyi-app reads the mouse and touches: the window's pointer in console pixels, the buttons, the wheel in notches, and a tap with no hover, split out of 2405"
type: feature
milestone: MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: done
blocked_by: ["2405"]
nick_input: none
completed: 2026-11-22
---

# 2413 — shuyi-app reads the mouse and touches

## Context

2405 brought the pointer to the engine: `RawInputEvent`'s pointer
variants (`crates/ui/src/game.rs`), the hit areas a painter records
(`crates/ui/src/frame/hit.rs`), the `Pointer` that folds them into each
frame's `PointerInput` (`crates/ui/src/input/pointer.rs`), the menu's
`point`, `Grid::tile_at_px`, and the Harness's `point`, `click`, `wheel`,
`leave_pointer` and `click_text` (`crates/platform/src/harness/pointer.rs`).
Its rules are ADR-0134. The game's screens and tests can take the
pointer, but no real backend sends it yet: the desktop build and the web
build (`cargo shuyi web`) still can't be clicked.

2405's split valve ("past 1,400 lines, `shuyi-app`'s reading of the
mouse goes to a new ticket") put its items 2 and 3 and its step 7 here,
so the engine's side landed with the Harness first.

**Why it matters.** Until this lands, the web build and the website's
playable examples (2801, 2803) take no click, though their menus and
Hello, Grid's title do in tests.

## Nick input

None. Nick decided on 2026-10-20 that this half of 1121 comes after the
MVP, not parked ([`docs/decisions.md`](../../docs/decisions.md)).

## Scope

**In:**

1. **The placement's inverse** (`crates/ui/src/frame/console.rs`):
   `Layout::frame_px(self, spec, window: (f32, f32)) -> Option<(i32,
   i32)>`, the console pixel at a window pixel (rounded down), `None`
   in the letterbox. **Either order with 2333:** if 2333 is done, the
   inverse is `Placement`'s, its `Fit` fraction included, and
   `pointer.rs` uses the mode the frame was drawn in; if not, this
   inverts today's `layout`, and 2333 moves it onto `Placement`.
2. **`shuyi-app` reads the mouse** in a new `crates/app/src/pointer.rs`,
   called from `Macroquad::poll_input` (`crates/app/src/backend.rs`)
   after the keys and pads, sending the events in the order ADR-0134
   and `Platform::poll_input`'s doc comment give:
   - **Where:** `mouse_position()` is in logical pixels (macroquad
     0.4.16 divides by the DPI factor, `src/input.rs`, line 51). Times
     `screen_dpi_scale()` it is in the physical pixels `Renderer::draw`
     places the frame in (`crates/app/src/render.rs`). The inverse of
     that placement (item 1) gives console pixels.
     - The backend keeps the `ConsoleSpec` of the last frame it drew
       (`draw_game`). Before the first game frame there are no pointer
       events.
     - `PointerMoved` only when the console pixel changes, and
       `PointerLeft` once when the pointer goes into the letterbox.
       macroquad reports no pointer leaving the window (miniquad's
       Windows backend has its mouse-leave code commented out), so off
       the window the pointer keeps its last place. That is accepted:
       hover follows motion only (ADR-0134).
   - **Buttons:** `is_mouse_button_pressed` and
     `is_mouse_button_released` for `MouseButton::Left`. A press outside
     the frame is dropped. Every press sent gets its release, wherever
     the release happens, so nothing stays held.
   - **Order within one poll:** `PointerMoved` or `PointerLeft` for the
     place, then `PointerDown`, then `PointerUp`, then `Wheel`. A press
     and a release in one frame give `PointerDown`, then `PointerUp`.
   - **Touches:** macroquad turns touches into left-button events by
     default (`simulate_mouse_with_touch`, `src/lib.rs`, lines 323 and
     655 to 666), on native and the web, and a button event moves
     `mouse_position` to the touch. So a tap arrives as a press and a
     release at the touch's place. A finger has no hover: after a
     release that came from a touch (`touches()` was not empty at its
     press), the reader sends `PointerLeft`, last in that poll.
   - **The wheel:** `mouse_wheel().1` is a raw delta in a unit each
     platform picks:
     - Windows: `WM_MOUSEWHEEL`'s delta, 120 a notch (miniquad 0.4.11,
       `src/native/windows.rs`, line 567);
     - Linux: X11, miniquad's default backend (`LinuxBackend::X11Only`,
       which `shuyi-app` keeps), 1 a notch (`src/native/linux_x11.rs`,
       lines 100 to 103);
     - macOS: a wheel's line delta times 10, so about 10 a notch, and a
       trackpad's pixel deltas (`src/native/macos.rs`, lines 597 to
       600);
     - the web: `-deltaY` (`crates/cli/web/mq_js_bundle.js`), about 100
       a notch for a mouse and small steps for a trackpad.

     An accumulator turns the deltas into whole notches by the
     platform's unit (`cfg` on the target), keeping the remainder, so a
     trackpad scrolls too. Measure the web in Chrome and Firefox on the
     day and write the numbers in a comment. macroquad keeps only the
     last wheel event of a frame (`src/lib.rs`, line 565); that is
     accepted.
   - The mapping, the accumulator and the touch rule are pure functions
     in `pointer.rs`, tested without a window. The macroquad calls are
     a thin shell around them.
3. `docs/platform-backends.md`: `shuyi-app`'s row says it sends the
   pointer.
4. No new dependency.

**Out (do not do):**
- Everything 2405's Out names: mouse buttons other than the primary, the
  pointer in `TickInput` (1121), swipes and several touches (2407), the
  system cursor (3314).
- The Playdate and the console backends: they have no pointer and send
  none.

**Estimated size:** about 700 lines (code 300, tests 300, docs 100).

## Implementation steps

1. Read ADR-0134, 2405's completion notes, `crates/app/src/backend.rs`,
   `keys.rs`, `pads.rs` and `render.rs`, and `frame/console.rs`. Check
   whether 2333 is done.
2. Write `Layout::frame_px` with its property test.
3. Write `crates/app/src/pointer.rs`: the mapping, the accumulator and
   the touch rule as pure functions, then the shell, and call it from
   `Macroquad::poll_input`.
4. Try the desktop build and the web build by hand (`cargo shuyi web` in
   `examples/hello-grid`): hover and click on the title menu. Measure
   the web's wheel deltas with a print you remove before the PR. The PR
   says what was tried where.
5. The backend doc's row. Run the gates.

## Acceptance criteria

- [x] `Layout::frame_px` inverts the placement for every console pixel
      at scales 1 to 4, and gives `None` in the letterbox.
- [x] `shuyi-app`'s pure parts are tested: the mapping inside the frame,
      in the letterbox and at a DPI factor of 1.75; the wheel's
      accumulator for each unit; a press and a release in one frame; a
      touch's release followed by `PointerLeft`.
- [x] Hello, Grid's title menu takes hover and a click in the desktop
      build and in the web build, tried by hand and said in the PR.
- [x] The book's chapter is updated (2802), or the completion notes have
      a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `shuyi-app`'s mapping, accumulator and touch rule.
- Property: `Layout::frame_px` inverts the placement for every console
  pixel, at scales 1 to 4.
- By hand: the desktop and web builds of Hello, Grid.

## Completion notes

What was done:

- `Layout::frame_px(self, spec, window)` (`crates/ui/src/frame/console.rs`):
  the console pixel at a window pixel, rounded down, `None` in the
  letterbox or for a place that is no number. 2333 is still open, so it
  inverts today's `layout`; 2333's clause now says it moves onto
  `Placement`. Tested for every window pixel of every console pixel of a
  small console at scales 1 to 4, by a property test over console and
  cell sizes, margins and places, and at the letterbox's edges.
- `crates/app/src/pointer.rs`: `console_px` (the mouse's logical pixels
  times the DPI factor, through the placement's inverse), `Notches` (the
  wheel's accumulator, one notch a platform's unit, chosen by `cfg`, the
  rest kept), and `PointerReader::events` (the place when its console
  pixel changes, `PointerLeft` once into the letterbox, the press inside
  the frame, its release wherever, the wheel, then `PointerLeft` after a
  finger's release). `read` is the shell over macroquad's calls.
  `Macroquad` keeps the last drawn frame's `ConsoleSpec` beside the
  reader, and `poll_input` adds the pointer's events after the keys and
  pads, before the live link's.
- `render::placement` is the one function `Renderer::draw` and the
  pointer both place the frame with, so they can't drift.
- `docs/platform-backends.md`: `shuyi-app`'s row and the pointer rule say
  it sends the pointer; the book's *The mouse and taps*
  (`site/content/learn/book/actions.md`) says the desktop and web builds
  take it.

Deviations:

- One rule beyond the ticket: a press whose release macroquad never
  reported (the button is up, no release came, as after a focus change)
  gets its `PointerUp` on the next poll, so the next press isn't dropped
  as a second press. A release and a new press in one frame send the old
  press's `PointerUp` first.
- `Reading` counts the touches (`touches().len()`) instead of a flag:
  clippy refuses a fourth bool in one struct.
- The web's wheel was measured in Chromium only (the desktop app's
  browser pane, Chromium 156 on Windows: three notches of its wheel input
  gave one `deltaY` of 300 in pixels, `deltaMode` 0, so 100 a notch). No
  Firefox was at hand; the comment says its numbers are not measured. No
  ticket: the unit is a constant in one place, and a Firefox notch of
  100 to 120 gives one notch, with an extra one at most every five or so
  from the kept rest, which a menu's scrolling bears. No print was added
  to the code: the deltas were read with a `wheel` listener on the page.

Tried by hand, on main's tip 23ba0a6b with this branch's changes, on
Windows 11 at a DPI factor of 1.75:

- the web build (`cargo shuyi web` in `examples/hello-grid`, served
  locally, in the desktop app's browser pane, frames run with
  `wasm_exports.frame()` as the pane was hidden): a `mousemove` over
  Options moved the title menu's focus to it; a press and release over
  Profiles opened Profiles; a touch's start and end over Options (a
  `TouchEvent`, with no move first) opened Options;
- the desktop build (`target/debug/hello-grid.exe`, a 784 × 784 window):
  `WM_MOUSEMOVE` posted over Profiles moved the focus to it, and
  `WM_LBUTTONDOWN` and `WM_LBUTTONUP` over Options opened Options.
  Posted messages go through miniquad's own window procedure, the path a
  real mouse takes, without moving the real cursor.

Behaviour: shuyi-app: the desktop and web builds send `PointerMoved`, `PointerLeft`, `PointerDown`, `PointerUp` and `Wheel`, so the engine's menus take hover, clicks and taps, and the grid cursor follows the pointer, where before only the Harness sent them.

Book: `site/content/learn/book/actions.md`, *The mouse and taps*, says the desktop and web builds take the mouse, and that a tap is a click.

Game: nothing to adopt, the game's code only builds key and pad events
(`gh search code --repo Zafnok/visions-of-shuyi 'RawInputEvent'`, and
`'pointer'`: no screen of its own matches on a pointer event); its menus
are the engine's, which take the pointer at its next pin bump.

No break: `Layout::frame_px` is new, and nothing public changed.
