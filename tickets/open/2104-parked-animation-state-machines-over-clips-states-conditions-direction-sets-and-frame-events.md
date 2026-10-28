---
id: "2104"
title: "Parked: Animation state machines over clips: states, conditions on parameters, exits when a clip ends, direction sets and frame events, as data"
type: feature
milestone: "Engine: animation"
model: sonnet-5
effort: high
status: blocked
blocked_by: ["1119", "2231", "2335"]
nick_input: none
completed:
---

# 2104 — Animation state machines over clips

> **Parked** (Nick, 2026-10-14: "document the parked (i.e. after MVP/parity) tickets for networking, styles for later"): the game doesn't need it, so this waits until after the engine reaches parity with the game (the MVP in `docs/ROADMAP.md`) and after 1119, 2231 and 2335. Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

Found by the 2026-10-20 comparison with Bevy 0.20 and Godot 4.6. 2231
gives clips: `AnimationClip` and `frame_at(clip, elapsed_ms)`. Which
clip plays is left to each game. Every character game then writes the
same code: idle until moving, walk while moving, jump on a press, back
to idle when the attack clip ends, and the walk clip that faces the
way the character moves.

Godot does this with `AnimationTree` (a `StateMachine` and a
`BlendSpace2D`), Unity with its Animator, Bevy with `AnimationGraph`.
For frame-by-frame sprites, "blending" two clips means nothing. What
sprite games use instead is picking the clip by a direction: a
`BlendSpace2D` whose points are clips, read without mixing. Here that is
a **direction set**. Mixing poses is for bones (2105).

The rule from [ADR-0004](../../docs/adr/0004-assets-standard-files-in-the-engines-own-data-out-and-a-deterministic-movement-kit.md)
§3 holds: the frame shown is a pure function of the data and the time.
A tick simulation (1119) stores a small `AnimState` per thing, so
replays are exact.

No code here uses 1119: `AnimState` and `step` take a plain step
number, which a game counts from `FrameInput::ticks()`, and the toy
needs no tick simulation. 1119 stays in `blocked_by` only because the
Parked note names it: whoever unparks this ticket before 1119 is done
drops it from both.

## Nick input

None.

## Scope

**In:**
- **`shuyi_assets::anim::graph`**, beside 2231's `anim`:
  - `AnimGraph`, an asset kind (`AssetKind::NAME = "anim_graph"`), read
    by `AnimGraphRon` (ADR-0007, ADR-0019), registered in
    `Formats::register_standard`, with a property test that random
    bytes never panic it;
  - **parameters**, named, each `Bool`, `Int` (`i32`), `Trigger` (used
    up by the transition that reads it) or `Dir` (2335's `Angle`, or
    none while still). The game sets them each step.
  - **states**, named, each one of:
    - `Clip(clip id)`;
    - `Dirs { param, clips: [(Angle, clip id)] }`: the clip whose angle
      is nearest the `Dir` parameter, the lower index on a tie, and the
      last pick kept while the parameter is none. Changing clip inside
      the state keeps the elapsed time, so a walk doesn't restart when
      it turns.
  - **transitions**, each `from` a state or `Any`, `to` a state, and
    `when`, a list of conditions that must all hold:
    `param == value`, `param > n`, `param < n`, `trigger param`, and
    `ended` (the clip finished, read from 2231's `AnimationClip::ended`;
    only for `Once` and `Times(n)` clips).
    They are tried in the order written. At most one fires per step.
  - **events**, per clip: `(frame index, name)`, such as a footstep on
    frames 2 and 6. Events between two steps follow one rule,
    `shuyi_core::timer::crossed` (2364): a mark at `m` is crossed when
    `prev < m <= now`, a backward jump or a seek crosses nothing, and a
    range longer than one loop crosses each mark once. Entering a state
    is the first call (`prev` is none), so an event on the first frame
    fires then. **Either order with 2364:** if it is done, use
    `crossed`; if not, write `crossed` in `shuyi_core::timer` with the
    rule and tests in 2364's Scope (creating the module if 2115 hasn't),
    and 2364 uses it.
    **Either order with 2105:** if 2105 is done and 2364 isn't, 2105
    wrote `crossed` in `shuyi_core::timer`: use it, and don't write it
    again. If not, nothing more to do (2105 uses the `crossed` this
    ticket writes).
  - checks, with file and line: an unknown clip, parameter or state;
    `ended` on a looping clip (an error); a state nothing reaches (a
    warning); a parameter of the wrong type in a condition.
- **The runner**, pure:
  - `AnimState { state: StateId, entered_step: u32, dir_pick: u8 }`,
    `Copy` and serde: all a simulation stores;
  - `AnimGraph::step(&self, state, &Params, step) -> Stepped { state,
    events: Vec<&str>, used_triggers }`. Events are every frame entered
    since the previous step, by `crossed`, in order, even when a step
    is longer than a frame;
  - `AnimGraph::frame(&self, state, step) -> FrameRef`, through 2231's
    `frame_at`;
  - a state's time is `ms_from_steps(step - entered_step)` ms (the
    subtraction saturating), 2115's one rule from steps to ms in
    `shuyi_core::timer`, for `frame_at`, `ended` and events alike. No
    `ms_per_step` is passed: a whole one (16 or 17) drifts from 60
    steps a second. **Either order with 2115:** if 2115 is done, use its
    `ms_from_steps`; if not, write `STEPS_PER_SECOND` (moved from
    `shuyi_platform::tick`, which re-exports it) and `ms_from_steps` in
    `shuyi_core::timer` with the rule and tests in 2115's Scope, unless
    2103 already did, and 2115 keeps them.
- **The toy and the example:** a test graph on 2231's fixture set (its
  own clips over the fixture's picture, since that set holds only
  `splash`, `waves` and `wind`) with idle, walk in four directions, and
  a one-shot action that returns to idle when it ends.
- **The debug readout:** if 2316 is done, a helper that a screen's
  readout calls to show `state name, clip, frame`. If not, skip it:
  2316 adds it when it is worked (its scope says so).
- **The graph panel:** **either order with 2508** (parked): if 2508 is
  done, its panel also shows `*.anim_graph.ron`, read only: states as
  boxes and transitions as wires, each with its conditions. If not,
  nothing here; 2508 does it when it is worked (its scope says so).

**Out (do not do):**
- mixing two poses or cross-fading (bones, 2105), and driving 2105's
  bone clips by this graph's states with a cross-fade on a transition
  (2134, parked);
- layers (upper and lower body on separate graphs): a follow-up if a
  game needs one;
- tweens on sprites (`shuyi_core::ease::Tween`, 2109);
- editing the graph as boxes and wires: the graph panel part above only
  shows it (a follow-up if a game asks).

## Implementation steps

1. Read ADR-0004 §3, 2231's `anim` module, 2335's `Angle` and 2364's
   rule for marks between two times. Check whether 2364, 2105 and 2115
   are done.
2. Write the types and the checks, with fixtures.
3. Write `step` and `frame`, with unit and property tests. If neither
   2364 nor 2105 is done, write `crossed` first, with its tests.
4. Add the test graph and its snapshot tests.
5. Run the gates.

## Acceptance criteria

- [ ] A recorded list of parameters played twice gives the same states
      and events (a property test over random parameter lists).
- [ ] `ended` fires on the step a `Once` clip finishes, never earlier.
- [ ] A trigger fires one transition and is then used up.
- [ ] A direction set turning from right to down switches clip without
      restarting the elapsed time.
- [ ] Events fire once per frame entered, in order, at any step length;
      a step longer than one loop fires each event once (`crossed`).
- [ ] Bad fixtures give errors with file and line; random bytes never
      panic the format.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: each condition; `Any` transitions; transition order; the
  direction pick at ties and when still; events across a long step
  and on entering a state; `crossed`'s tests from 2364 if this ticket
  writes it.
- Property: replay of random parameter lists; `step` is pure (same
  input, same output).
- Snapshot / integration: the test graph through a scripted walk,
  frames listed per step.

## Completion notes

*(Filled in by the session that completes the ticket.)*
