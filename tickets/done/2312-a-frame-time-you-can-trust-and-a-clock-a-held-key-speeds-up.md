---
id: "2312"
title: "A frame time that is never negative or not a number, a clock a held action speeds up and a press skips, and the text box's Confirm step"
type: feature
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: done
blocked_by: []
nick_input: none
completed: 2026-10-19
---

# 2312 — A frame time you can trust, and a clock the player can hurry

## Context

- **The game ticket:** game#1166
  ([`1166-frame-time-and-skippable-clocks-from-the-engine.md`](https://github.com/Zafnok/visions-of-shuyi/blob/main/tickets/open/1166-frame-time-and-skippable-clocks-from-the-engine.md))
  waits for this one. Found by the game's first engine-friction sweep,
  game#1153.
- **The job:** "play something timed that the player can hurry by
  holding a key and finish with a press".
- **Friction:** found in eight of the game's screens (its timed pages,
  1.2×; its conversation screen, 1.6×; its title and others), outcome
  2: the pieces exist but cost code at every use. Ranked fourth of the
  sweep's findings.
  - `FrameInput::dt` (`crates/ui/src/input/frame.rs`) is a public
    `f32` with no promise. The game cleans it before use 13 times
    (`if dt.is_finite() { dt.max(0.0) } else { 0.0 }`), and the engine
    does it itself in `grid/cursor.rs`, `debug/camera.rs`, `game.rs`
    and `audio.rs`.
  - "A held key makes time pass faster; a press finishes the step, or
    goes to the next one" is written three times in the game (timed
    pages, a fight's playback, the computer's turn), 20 to 60 lines
    each.
  - The text box (`dialogue/text_box.rs`) has `reveal`, `next_page`,
    `hold` and `type_on`, but every screen that shows one writes the
    same "Confirm reveals the page, else turns it, else the box is
    done" (about 30 lines in the game's conversation screen).
- **The generic shape:**
  - `FrameInput` promises `dt` is finite and at least 0 (cleaned in
    `FrameInput::new` and every other constructor; the field becomes
    private with a `dt()` getter, or stays public with the promise
    documented and a debug assertion: the session picks and says why);
  - `HurryClock`: `tick(dt, held) -> f32` (the seconds that pass this
    frame: `dt`, times a factor after the key has been held a moment),
    and `skip()`; the text box's fast forward uses it;
  - `TextBox::confirm() -> Step` with `Revealed`, `NextPage` and
    `Done`.
- **Outside tactics:** a visual novel's text box; a farming game's
  end-of-day tally that counts up and can be hurried; a platformer's
  level-clear score.

## Nick input

None.

## Scope

**In:**
- The `dt` promise, and the engine's own cleanups removed.
- `HurryClock` (name as the session likes) in `crates/ui`, documented,
  with the hold delay and factor as fields with today's text box
  values as defaults.
- `TextBox::confirm`.

**Out (do not do):**
- Fixed-step simulation time (1119).
- Any game's screens.

**For the game:** if `dt` becomes private, the game's reads of
`input.dt` become `input.dt()`; otherwise additive. The game's PR that
moves its pin follows this one (game#1166).

## Implementation steps

1. Clean `dt` where `FrameInput` is built; remove the engine's own
   checks.
2. `HurryClock`; move the text box's fast forward onto it with no
   change in behavior.
3. `TextBox::confirm`.

## Acceptance criteria

- [x] Unit: `FrameInput::new` with `NaN`, `-1.0` and `INFINITY` gives
      `dt` 0; with `0.016` gives `0.016`.
- [x] Unit: `HurryClock` passes `dt` unheld, `dt` times the factor
      after the delay, and the part of a frame that crosses the delay
      at each speed.
- [x] Unit: `confirm` on a page being typed reveals it; on a revealed
      page with another after it turns; on the last page says `Done`.
- [x] The text box's existing tests pass unchanged.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit, as above.

## Completion notes

- **`dt`:** `FrameInput::dt` is now private, read with `dt()`, and is
  always finite and at least 0: `FrameInput::new` (its only constructor;
  the `with_*` builders keep it) cleans it with the new public
  `shuyi_ui::input::frame_seconds`. Private, not a documented public
  field, because a public field can be written after construction and the
  promise would be only a debug assertion; the getter makes it hold
  always. `Game::frame` also cleans its `dt` once at the top, so the
  clock and the music get the same seconds as the screens. The checks on
  `input.dt` are gone from the debug camera, the toy's grid field, the
  example's stage and the Key bindings screen. Public functions that take
  a raw `f32` themselves (`Cursor::tick`, `MusicState::update`, the
  harness's music player) keep their documented guard, now the one
  shared `frame_seconds` instead of each its own copy.
- **`HurryClock`** (`shuyi_ui::hurry`, re-exported at the crate root):
  `tick(dt, held) -> f32`, split into `hold` and `passing` for the text
  box's existing two-call API; `release`, `skip`, `is_skipped`, `reset`;
  public `delay` and `factor` fields defaulting to `HURRY_DELAY` 0.3 and
  `HURRY_FACTOR` 6 (today's text box values). The skip is a flag the
  owner reads and finishes its step on: the clock doesn't know where the
  step ends. The text box's fast forward runs on it;
  `FAST_FORWARD` and `FAST_FORWARD_DELAY` stay, as the clock's defaults.
- **`TextBox::confirm() -> Step`** (`Revealed`, `NextPage`, `Done`;
  `Step` re-exported from `shuyi_ui::dialogue`). The toy's and the
  example's talk screens use it.
- **One test changed:** the Key bindings screen's test said "a NaN frame
  time ends the moved-row highlight". A NaN can no longer reach a screen
  (it is 0 seconds), so the test now says a NaN frame passes no time, and
  the screen's NaN check went. The text box's tests are unchanged.
- **For the game (game#1166):** its reads of `input.dt` become
  `input.dt()`, and its 13 cleanups of it can go. The game PR that moves
  the pin is game#1166's; this engine PR merges first.
- No follow-up tickets. Nick input: none.
