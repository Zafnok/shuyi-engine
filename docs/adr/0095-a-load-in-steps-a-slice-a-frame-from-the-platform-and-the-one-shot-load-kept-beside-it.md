# ADR-0095: A load in steps: a slice a frame from the platform, and the one-shot load kept beside it

- **Status:** Accepted
- **Date:** 2026-11-09
- **Related tickets:** 3097; game ticket 1086 and game ADR-0111 (the
  game's own start in steps), game ticket 1170 (the game's Playdate front
  end on `shuyi-playdate`)

## Context

- The Playdate OS ends a game whose update callback hasn't returned for
  10 seconds ("run loop stalled for more than 10 seconds"). The device
  loads data 600 to 1,300 times slower than the Simulator. The game's
  data alone took 15,351 ms there (game ticket 1086).
- The game fixed it in its own front end (game ADR-0111): `Boot`, a step
  machine run one step per update callback, whose steps made of many
  parts (the dialogue's scenes, the sounds) go on until the callback has
  run a second, then continue in the next.
- The engine had no such thing. `Start::load` was a `Load<G>`, a boxed
  `FnOnce(Setup) -> Result<Game<G>, String>`, and `App` ran it in the
  frame after the card's (ADR-0024). `shuyi-playdate`'s driver runs one
  `App::frame` per update callback, so a long load there is one long
  callback.
- `App` is the same on every backend. The PC and the web have no such
  limit: a long frame only shows the card longer.
- A run is recorded and played back frame by frame (ADR-0072), and a
  frame's hash says whether the game is loading. The game uses `Start`
  only through `Start::new`.

## Decision

1. **Two loaders.** `Start::load` is a `Loader<G>` (`#[non_exhaustive]`):
   `Loader::Once(Load<G>)`, the one-shot load, unchanged, which
   `Start::new` gives; and `Loader::Steps(LoadInSteps<G>)`, which
   `Start::in_steps(card, look, begin)` gives. `LoadInSteps<G>` is a boxed
   `FnOnce(Setup) -> LoadStep<G>`: it takes the `Setup` once and gives the
   steps. `LoadStep<G>` is a boxed `FnMut() -> Progress<G>`, one step a
   call; `Progress<G>` (`#[non_exhaustive]`) is `More` or
   `Done(Result<Box<Game<G>>, String>)`, the game boxed as clippy's
   `large_enum_variant` asks, which `Progress::done(result)` does for a
   step. `Start::before_load` wraps either.
   All in `crates/platform/src/app.rs`.
2. **The platform gives the slice.** `Platform::load_slice(&self) ->
   Option<LoadSlice>`, defaulted to `None`. A `LoadSlice` is a length in
   microseconds and a `PerfClock` to time it by. `None` runs every step
   in one frame (the PC, the web, the headless backend unless a test
   gives one with `HeadlessPlatform::with_load_slice`).
3. **`App` runs the steps.** From the second frame, each loading frame
   asks for the slice and runs steps until one says `Done` or the slice is
   used, and at least one a frame. The clock is read after each step, so a
   step is the unit: a step longer than the slice runs alone in its frame,
   and splitting it is the game's work. A frame that ends with steps left
   presents the card again. `App::load_steps` counts the steps run;
   `App::load_us` sums the frames' load times.
4. **A recorded or hashed run takes no slice.** Where a slice ends depends
   on the clock, which a playback doesn't repeat, so a run being recorded
   or hashed loads in one frame, as its playback on the headless backend
   does.
5. **The slice is the target's.** `TargetProfile`'s `budgets` gain
   `load_slice_ms: Option<u32>` (at least 1). `targets/playdate.ron` says
   1,000: a tenth of the system's limit, the game's `STEP_SECONDS`.
   `shuyi-playdate`'s backend reads it from its embedded profile and times
   it with the SDK's microsecond timer, in every build.
6. **The run report gives each callback.** `shuyi-playdate`'s run report
   adds the line `Start, ms per update callback: 1,000 (10 steps), …`
   once the game has loaded: each update callback that ran the load, its
   time and its steps (a one-shot load is one callback of one step). The
   time between two callbacks is a step of its own in the reads,
   `(between update callbacks)`, so it isn't the last file's.

## Consequences

- A game whose load passes 10 seconds on the Playdate gives its load as
  steps and starts there, the card on the screen; the same `Start` loads
  in one frame on the PC and the web.
- The game's `Boot` maps onto it: `begin` keeps the `Setup` (its storage
  and seed are what `Host` gave), and each call runs one `Boot` step, or
  one part of the scenes' checks or the sounds, which `Host::out_of_time`
  no longer needs to count.
- `Start::load`'s type changed from `Load<G>` to `Loader<G>`: a break for
  code that assigns or reads the field (`shuyi-app`'s own `start_as`). A
  game built with `Start::new` changes nothing.
- A target profile's binary form gains the field, so `pack::VERSION` is
  12 and a pack baked before is baked again.
- A load in steps owns what it hands from step to step: a step can't
  borrow from an earlier one.
- The slice is checked between steps only. A single step that stalls for
  10 seconds still ends the game; the report's per-callback line shows
  which callback did.

## Alternatives considered

- **The driver runs the steps, not `App`.** It would keep `App` as it
  was, but every backend with a limit (a console's watchdog) would copy
  the loop, and the PC, the web and the Harness would need a second way to
  start a game. `App` already runs the load behind the card.
- **A step that asks whether it is out of time** (`FnMut(&mut Loading)
  -> Progress`, the game's `Host::out_of_time`). It lets a step loop over
  many parts itself, but a step that never asks stalls anyway, and the
  same is said by returning `More` after each part, with the clock read in
  one place.
- **A trait for the steps.** A boxed closure matches `Load<G>` and is the
  least a small game writes; a game with a step machine wraps it in one
  closure.
- **The slice as a constant of `shuyi-playdate`.** The profile holds the
  target's numbers so they are written once (ADR-0008 §1); a console's
  backend reads its own the same way.
- **A load in steps added without changing `Start::load`** (a second,
  private field). Two loaders a `Start` could hold at once, with one
  silently ignored when a backend replaced the field.
