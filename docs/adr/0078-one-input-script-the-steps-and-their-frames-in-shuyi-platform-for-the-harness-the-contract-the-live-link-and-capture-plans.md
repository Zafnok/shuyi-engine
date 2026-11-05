# ADR-0078: One input script: the steps and their frames in shuyi-platform, for the Harness, the contract, the live link and capture plans

- **Status:** Accepted; amends ADR-0047 §7 (the link's steps and waits)
  and ADR-0052 (a plan's input steps)
- **Date:** 2026-11-04
- **Related tickets:** 2566 (this), 2521 (the live link, ADR-0047), 2018
  (capture plans, ADR-0052), 2540 (the crank step), 2524 (the agent
  eval, which builds on it), 2353, 2356 (later kinds of step)

## Context

Three types said "a script of input steps", and a fourth place played
them by hand:

- `shuyi_cli::frame_png::{Script, Step}`: keys, pad, a wait in seconds and
  the crank, played by the Harness (`run`, `scene`, `frame-png`, MCP's
  `step`, every game's tool program). No step pressed an action by name,
  so a headless agent named keys (`AGENTS.md` rule 8 asks for actions).
- `shuyi_platform::link::Step` (ADR-0047 §7): keys, pad, an action, a wait
  in frames. Its `plan` built the frames of a key press a second time.
- `shuyi_cli::contract::attach::AttachStep`: the bridge between the two,
  turning seconds into frames to the nearest.
- `shuyi_platform::capture::Step` (ADR-0052): keys, pad, a wait in seconds
  and `capture`, with its own copy of the frames.

Each new kind of step (the crank, a language, a debug command) was
written two to four times, and the two waits had different units and
limits.

## Decision

1. **One step type:** `shuyi_platform::script::Step` (`#[non_exhaustive]`):
   `Keys`, `Pad`, `Act`, `Wait(f32)` in seconds and `Crank(f32)`, with the
   serde form the contract had (`{"keys": "Enter Up"}`, `{"act":
   "Confirm"}`, `{"wait": 0.5}`) and `Script { steps }`. `Step::check`
   refuses names `Chord::parse` and `Button::parse` don't know, an empty
   action name, a wait that isn't finite or is negative, and a crank that
   isn't finite. `shuyi_cli::frame_png` re-exports `Script` and `Step`.
2. **One plan:** `script::frames(steps, keymap, pad)` gives
   `Planned::Events` (one frame) or `Planned::Idle(n)`, each frame 1/60 s.
   `Harness::keys`, `Harness::pad`, `Script::play` (the `harness`
   feature), the link's server and `capture::Plan::frames` are built on
   it, so a key press's `RawInputEvent`s are built in one place. An error
   names the step (`step 3: act: …`).
3. **One rule for a wait:** `script::wait_frames(seconds)`: the seconds
   at 60 frames a second, rounded up, a product within a thousandth of a
   whole number counting as that number; a wait above 0 is at least one
   frame and a wait of 0 is none. `Harness::wait`, which a game's tests
   call, keeps its seconds and its shorter last frame.
4. **The longest wait is the player's to check**, not the step's:
   `harness::MAX_WAIT` (its 2000 frames, about 33 s), `link::MAX_WAIT`
   (1 800 s), a capture plan's `capture::MAX_WAIT` (600 s), each checked
   with `script::check_waits`.
5. **One lookup of an action by name:** `script::press_for` (moved from
   `link`): `ActionSet::from_name`, then `Keymap::primary`, else
   `Keymap::primary_button`. The Harness presses an action as the link
   does; one the game can't press panics there, as an unknown key does.
   The contract asks a session for the actions it can press
   (`Session::actions`, `script::pressable`) and refuses a script naming
   another before anything plays (`run`, `scene`, MCP's `step`: `SH0001`).
6. **Amends ADR-0047 §7:** a `Play` request's steps are `script::Step`s;
   its waits are seconds on the wire, played as the build's frames by
   `wait_frames`; the link refuses a crank ("not over the live link
   yet"). `link::VERSION` is 3. `attach`'s `--frames N` after the steps
   is sent as waits that play exactly N frames (whole seconds, then the
   rest).
7. **Amends ADR-0052:** a plan's steps are `capture::PlanStep`s,
   `Input(script::Step)` or `Capture(name)`, read from the same RON
   (`act("Confirm")` too; a crank is refused). `Plan::frames` takes the
   game's keymap; a backend's capture mode plans a plan that presses an
   action once the game has loaded.
8. **Features:** `script` is always built (serde is already a dependency
   of the crate, for session records); `JsonSchema` is behind
   `shuyi-platform`'s `schema` feature, which `shuyi-cli` turns on.

## Consequences

- A new kind of step is added once, to `script::Step`, with its play in
  `Script::play` and its refusal (or frames) in `script::frames`.
- An agent playing a headless game presses actions by name, as one
  attached to a running build does.
- A script's wait that isn't whole frames, or is 0, plays a different
  number of frames than before (rounded up; none for 0), so a
  `frame-png` picture of such a script can change. Whole-frame waits play
  as before.
- `frame_png::Step`, `Script::parse_step` (now `frame_png::parse_step`),
  `link::Step`, `AttachStep`, `capture::Step`, `capture::FRAME_DT` and
  `capture::wait_frames` change or go: an API break for games (their
  exhaustive `match` on a step gains an arm).

## Alternatives considered

- **Keep the link's waits in frames.** Exact, but the two ends would keep
  converting, and a script written for `run` couldn't be sent as is.
- **Leave `Act` to the link and the contract.** The Harness would still
  not press an action, so `run` and `attach` would stay different tools.
- **A `script` feature gating serde.** serde is a non-optional dependency
  of `shuyi-platform` already; the feature would gate nothing.
