---
id: "3086"
title: The seed of a desktop launch comes from a clock that starts near zero, so launches share few seeds
type: bug
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
---

# 3086 — The seed of a desktop launch comes from a clock that starts near zero

## Context

Found while working 2519. `Setup::seed` is documented as "a seed from the clock,
different each launch" (`crates/platform/src/app.rs`, `App::load`: `seed:
mix(now.0)`), and Hello, Grid seeds its stages' luck from it.

Since 3067, `shuyi_app::run` hands `App::frame` the loop's one clock,
`pacing::seconds()` (`crates/app/src/pacing.rs`). On native that is an `Instant`
from its first read, so the load's frame is a few milliseconds after 0 on every
launch: the first frame is at 0 ms, and the load runs one frame later. `mix`
spreads what it is given, but it is given only a handful of values, so launches
on one machine draw from a few seeds and often repeat. The web reads `Date.now()`
and is fine.

A session record must keep playing the same: the seed reaches the game from the
platform, so ADR-0072's rule holds (a value from the platform gets a field in the
record and its playback).

## Nick input

None.

## Scope

**In:**

- The seed of a desktop launch differs from launch to launch, while the frames'
  times stay on the monotonic clock (the pacer needs a clock that can't jump).
  Likely shape: a backend gives `App` the launch's entropy through the `Platform`
  trait (a defaulted `launch_seed`), `shuyi-app` answers it from the system's
  time, `App::load` mixes it in, and `SessionRecord` gains an optional `seed`
  field that `HeadlessPlatform::from_session` answers with. An optional field
  leaves `FORMAT_VERSION` alone (ADR-0072).
- The docs of `Setup::seed` and `docs/platform-backends.md`.

**Out:**

- Anything about the frames' clock itself (3067 chose it).
- A game's own use of the seed.

## Implementation steps

1. Read `App::load`, `pacing::seconds`, `Recorder::handed_over`, and
   `HeadlessPlatform::from_session`.
2. Add the platform's seed, with the headless backend keeping today's value from
   the frame's time so existing records and goldens play unchanged.
3. Record it and play it back.
4. Bless Hello, Grid's golden session if its bytes move (`cargo xtask golden
   --bless`).

## Acceptance criteria

- [ ] Two desktop launches of Hello, Grid with the same frame timing get different
      `Setup::seed`s (a unit test over the pure part, with a fake time).
- [ ] A record made with a platform seed plays back with no difference, and its
      game sees the same seed (a test on the toy, as `crates/platform/src/session/tests.rs`'s
      `Seen` checks).
- [ ] A record with no `seed` field still plays.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the seed's pure part. Integration: the session round trip above.

## Completion notes

*(Filled in by the session that completes the ticket.)*
