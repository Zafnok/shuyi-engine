---
id: "3003"
title: "The platform boundary: a shuyi-platform crate with the Platform trait and App::frame, and shuyi-app as its macroquad backend"
type: feature
milestone: After MVP
model: opus-5.5
effort: xhigh
status: todo
blocked_by: ["game#1131"]
nick_input: none
completed:
---

# 3003 — The platform boundary

## Context

[ADR-0008](../../docs/adr/0008-export-targets-profiles-as-data-platform-backends-controls-that-fit-one-bit-and-budgets.md)
§2. Today `shuyi-app` (macroquad) owns both the devices and the run loop:
input mapping, the screen stack, the tick driver, and drawing. The
Playdate (3010) can't use macroquad, and closed console backends will
live in private repositories. So the run loop has to move into a crate
that every backend calls, and the devices have to sit behind a small
trait.

Each backend owns its loop:

- macroquad's async loop;
- the Playdate SDK's update callback;
- a console's main loop.

The engine provides `App::frame`, which the backend calls once per
frame with the time.

## Nick input

None. The game keeps depending on `shuyi-app`, and only its `main`
changes (a few lines).

## Scope

**In:**
- **A new crate, `shuyi-platform`**, between `shuyi-ui` and the
  backends:
  - the `Platform` trait, with `display`, `present`, `poll_input`,
    `system_events`, `storage`, `audio` and `text_input` (ADR-0008 §2
    gives the shape; fix the names here);
  - `Storage`, `Audio` and `TextInput`, and `SystemEvent` (`Suspend`,
    `Resume`, `FocusLost`, `FocusGained`, `QuitRequested`, `LowBattery`);
  - `App<G>`, which holds the screen stack, input state and tick driver
    moved from `shuyi-app`, with `App::frame(&mut self, p: &mut impl
    Platform, now: Millis) -> FrameOutcome` (`Continue` or `Quit`).
- **Suspend and resume.** On `Suspend`, `App` saves through the game's
  hook and stops stepping. On `Resume`, it resets the tick accumulator,
  so that a sleep doesn't become a burst of catch-up steps (ADR-0003 §2's
  cap of 4 still holds).
- **`shuyi-app` becomes the macroquad backend.** It implements
  `Platform` with macroquad, gilrs and the audio it already uses, and
  `shuyi_app::run(app)` drives `App::frame` from macroquad's loop on
  desktop and the web.
- **A headless backend.** `shuyi_platform::headless::HeadlessPlatform`
  records presented frames and plays scripted input. The Harness uses it
  instead of its own driver, so tests run the same `App::frame` the
  devices do.
- **Docs.** `docs/platform-backends.md` explains how to write a backend:
  the trait, what each method must do, the headless backend as a
  reference, and the rule that console backends stay private.
- **Rules.** ADR-0008 §2 already adds `shuyi-platform` to ADR-0002 §1's
  crates (ADR-0002 stays unedited; its index row says "extended by
  ADR-0008"). `AGENTS.md` rule 5 already says "platform backends". Check
  that both still match what was built.

**Out (do not do):**
- `no_std` (3005);
- the Playdate backend (3010);
- the 1-bit output (3006); `present` takes the RGBA frame for now;
- Steam's keyboard behind `TextInput` (the game's 0903).

## Implementation steps

1. Create `shuyi-platform` and move the run loop out of `shuyi-app`
   without changing behaviour. The existing Harness and snapshot tests
   must pass unchanged at each step.
2. Define the trait and `App::frame`. Implement it for macroquad in
   `shuyi-app` and for the headless backend.
3. Point the Harness at `HeadlessPlatform`.
4. Add the suspend and resume handling, with tests.
5. Update the examples' `main`s. Write the game's matching change as a
   game ticket and link it here.
6. Write `docs/platform-backends.md`.

## Acceptance criteria

- [ ] `shuyi-platform` doesn't depend on macroquad, gilrs or any audio
      crate (`cargo tree` check in CI).
- [ ] Every example runs on desktop and the web as before (the web
      build gate and a manual run).
- [ ] All existing Harness, snapshot and replay tests pass unchanged.
- [ ] A test: `Suspend`, then `Resume` 10 minutes later, steps the tick
      simulation at most once, not 36 000 times.
- [ ] The game's ticket for its `main` change exists and is linked.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `App::frame` with the headless backend: input, then actions,
  then a step, then a presented frame.
- Integration: every example's Harness tests through
  `HeadlessPlatform`; suspend and resume.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
