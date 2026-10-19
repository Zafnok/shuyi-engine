---
id: "2312"
title: "A frame time that is never negative or not a number, a clock a held action speeds up and a press skips, and the text box's Confirm step"
type: feature
milestone: After MVP
model: sonnet-5
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
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

- [ ] Unit: `FrameInput::new` with `NaN`, `-1.0` and `INFINITY` gives
      `dt` 0; with `0.016` gives `0.016`.
- [ ] Unit: `HurryClock` passes `dt` unheld, `dt` times the factor
      after the delay, and the part of a frame that crosses the delay
      at each speed.
- [ ] Unit: `confirm` on a page being typed reveals it; on a revealed
      page with another after it turns; on the last page says `Done`.
- [ ] The text box's existing tests pass unchanged.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit, as above.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
