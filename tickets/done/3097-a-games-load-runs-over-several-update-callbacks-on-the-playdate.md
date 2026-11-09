---
id: "3097"
title: A game's load runs over several update callbacks on the Playdate
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: done
blocked_by: []
nick_input: none
completed: 2026-11-09
---

# 3097 — A game's load runs over several update callbacks on the Playdate

## Context

1. **The game ticket:** game#1170
   (`github.com/Zafnok/visions-of-shuyi/blob/main/tickets/open/1170-the-playdate-front-end-on-the-engines-backend.md`)
   moves the game's Playdate front end onto this engine's backend
   (`shuyi-playdate`). It is blocked on 2388 and 3096 today.
2. **The job:** start a game whose data takes longer than 10 seconds to
   load on the device, with the loading card up, without the Playdate OS
   ending it.
3. **No friction review:** a gap found by the retro of 2026-11-08 (game
   PRs #513–#580), reading the code.
   - The Playdate OS ends a game whose update callback hasn't returned
     for 10 seconds.
   - The game hit exactly that in game#1086: "run loop stalled for more
     than 10 seconds". Its device loads data about 600–1,300 times slower
     than the Simulator, and the game's data alone took 15,351 ms there.
   - The game fixed it in its own front end (game #537, game ADR-0111):
     `Boot`, a step machine run one step per update callback. Each step
     stops after one second and goes on in the next.
   - The engine's backend has no such thing. `Load<G>` is a one-shot
     `Box<dyn FnOnce(Setup) -> Result<Game<G>, String>>`
     (`crates/platform/src/app.rs`), and `shuyi-playdate`'s driver runs it
     inside one frame (`App::new(start)`, `crates/playdate/src/driver.rs`).
   - So game#1170's cutover would likely bring the stall back. That is
     an inference from the code, not a measurement.
4. **The generic shape:** a load in steps. The game gives its load as a
   series of steps; the backend runs as many as fit in a time slice per
   update callback, with the loading card on screen and a progress
   report, and starts the game when the last one returns.
5. **A game that isn't tactics:** a farming game that reads a large
   world and its saves at start, or any game with many dialogue files,
   on a slow handheld.

## Nick input

None.

## Scope

**In:**
- A stepwise load beside the one-shot one: for example a `Load::Steps`
  whose steps are `FnMut(&mut Loading) -> Progress`, or an iterator of
  boxed steps. `App` runs it on every backend. The PC and the web may run
  all the steps in one frame. The Playdate's driver runs steps until a
  time slice is used, about one second by default and set by the target
  profile, then returns from the callback.
- The load report (3090) gains the per-callback times, in the shape
  game#1086 added ("Start, ms per update callback: …").
- The one-shot `Load<G>` keeps working: either additive (ADR-0039), or
  with a `Breaking:` line and a migration block. Game#1170's PR moving
  the pin follows.
- A Harness test with a fake clock: a load of 30 steps of 100 ms each
  runs in three or more callbacks, none over the slice.

**Out (do not do):**
- Splitting the game's load into steps (game#1170 does that, from its
  `Boot`).
- Loading one chapter at a time (game#1068's subject).

## Implementation steps

1. Read the game's `crates/playdate/src/boot.rs` and game ADR-0111 (via
   `gh api repos/Zafnok/visions-of-shuyi/contents/…`) for the step
   shape that worked on the device.
2. Design the type in `crates/platform/src/app.rs`, and write an ADR
   (`write-adr`) since it is a public API shape.
3. Run the steps from `crates/playdate/src/driver.rs`, with the slice
   read from the target profile, which has a 10-second limit.
4. Add the report line in `crates/playdate/src/report.rs`.
5. Write the game's adopter note in the completion notes: what 1170 does
   with its `Boot` steps.

## Acceptance criteria

- [x] A Playdate driver test with a fake clock shows a 3-second load run
      over at least 3 callbacks, each under the slice.
- [x] A one-shot `Load<G>` game still starts unchanged (the existing
      driver tests).
- [x] The load report prints the per-callback times.
- [x] Game#1170's ticket names this ticket in its `blocked_by` (the
      game's PR from the retro adds it).
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the step runner's slice logic with a fake clock.
- Integration: the Playdate driver's tests with the fake SDK.

## Completion notes

Done as scoped (ADR-0095):

- **`shuyi-platform`:** `Start::in_steps(card, look, begin)` beside
  `Start::new`. `begin` takes the `Setup` and gives a `LoadStep<G>`, a
  boxed `FnMut() -> Progress<G>`; `Progress` is `More` or `Done` (built
  with `Progress::done(result)`, which boxes the game). `Start::load` is now
  a `Loader<G>`: `Once(Load<G>)` or `Steps(LoadInSteps<G>)`.
  `Platform::load_slice` (defaulted to `None`, all steps in one frame)
  gives a `LoadSlice`, a length and a `PerfClock`; `App` runs steps until
  it is used, at least one a frame, the card presented again between.
  `App::load_steps` counts them. A recorded or hashed run takes no slice,
  so a record plays back the same. `HeadlessPlatform::with_load_slice`
  for tests.
- **The slice is the profile's:** `budgets.load_slice_ms` in
  `TargetProfile` (at least 1); `targets/playdate.ron` says 1,000.
  `shuyi-playdate`'s backend reads it and times it with the SDK's
  microsecond timer, in every build.
- **The report:** `Start, ms per update callback: 1,000 (10 steps), …`
  after the load line; the time between two callbacks is
  `(between update callbacks)` in the reads.
- **Tests:** `app/tests/steps.rs` (the ticket's 30 steps of 100 ms on a
  slice of a second: three frames of a second each; a step longer than
  the slice; no slice; a failed load; the setup and `before_load`; the
  load's time; recorded and hashed runs; a one-shot load is one step), and
  in the driver's tests on the fake SDK
  `a_load_in_steps_runs_over_several_callbacks_each_within_the_slice` and
  `a_load_at_once_runs_in_one_callback_whatever_the_slice`, beside every
  existing driver test, unchanged.

Deviations: the Harness test the scope names is the `App` test on the
headless backend with a fake clock and the driver test on the fake SDK,
as the acceptance criteria and *Tests required* say: `Harness::start`
makes its own headless backend, which takes no slice, so a game's Harness
loads in one frame as on a PC. A step is the unit, as the ticket's
`FnMut(&mut Loading)` suggestion wasn't taken: the clock is read between
steps, and a step loops over parts by returning `More` after each
(ADR-0095, *Alternatives*).

**The game's adopter note (game#1170):** its `Boot` becomes the steps.
`Start::in_steps(card, look, |setup| { … })`: `begin` keeps the `Setup`
(the storage and the seed `Host` gave), holds `Boot::Data` in an
`Option`, and returns a closure that takes the `Boot`, runs one step and
puts the next back, giving `Progress::More`, until `Next::Started` gives
`Progress::done(Ok(game))` (or an error gives `Progress::done(Err(…))`).
The scenes' checks and the sounds return `More` after each part instead
of asking `Host::out_of_time`; the engine's slice is the profile's second.
`Host::mark` becomes `shuyi_playdate::report::mark`. The game's own
"Start, ms per update callback" line is the engine's report's now, with
step counts in place of `Boot`'s names. The game's PR that moves the pin
to this change is game#1170's.

Breaking: shuyi-platform: Start::load: a `Loader<G>` in place of a `Load<G>`; code that sets or reads the field wraps a one-shot load in `Loader::Once`.

```migration
since: 0.1.0
what: Start::load is a Loader, a load at once or in steps, in place of a Load.
old: "Start::load: Load<G>"
new: "Start::load: Loader<G>"
before: |
  start.load = Box::new(move |_| Err(why));
after: |
  start.load = Loader::Once(Box::new(move |_| Err(why)));
find: ["start.load = Box::new"]
```

Breaking: shuyi-assets: Budgets: a new field, `load_slice_ms`, how long one frame runs a load in steps for (`None` where a target has no limit on a frame).

```migration
since: 0.1.0
what: Budgets has a new field, load_slice_ms, how long one frame runs a load in steps for.
old: Budgets { frame_instructions, peak_heap_bytes, package_bytes, startup_ms }
new: Budgets { frame_instructions, peak_heap_bytes, package_bytes, startup_ms, load_slice_ms }
before: |
  Budgets { frame_instructions: None, peak_heap_bytes: None, package_bytes: None, startup_ms: Some(5_000) }
after: |
  Budgets { frame_instructions: None, peak_heap_bytes: None, package_bytes: None, startup_ms: Some(5_000), load_slice_ms: None }
```

Breaking: shuyi-assets: pack::VERSION and the target profile's golden value: a pack is of version 12 (a target profile's binary form has `load_slice_ms`); a pack baked before is refused, bake it again with `shuyi bake`.

```migration
since: 0.1.0
what: The pack format is version 12; packs baked before are refused.
old: "pack::VERSION == 11"
new: "pack::VERSION == 12"
before: |
  shuyi bake
after: |
  shuyi bake
```

Docs: `docs/platform-backends.md` (the first frames, `load_slice`),
`docs/targets/playdate.md` (*What a game adds* 6, the report's line),
`docs/targets/README.md` (`load_slice_ms`), and the book's *Targets and
export* chapter (*Size and speed*: a long load in steps).

Nick input: none.
