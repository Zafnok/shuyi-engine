---
id: "3270"
title: "Eased transitions between a widget's states in the look file, and ease::Chase for a value that trails a moving target, both drawn from time"
type: feature
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: ["3235", "2340"]
nick_input: none
completed:
---

# 3270 — Eased widget states and ease::Chase

## Context

`shuyi_core::ease` (2109, done) eases between fixed ends over a known
time. 3235 gives screens one look file, and its **Out** leaves "eased
transitions between a widget's states (normal, focused, disabled)".
The Bevy coverage audit of 2026-11-28 (`docs/ROADMAP.md`, *Bevy's game
coverage*) found two common looks that need more:

- **widgets that animate between states**: a button that eases its
  colour and grows a pixel on focus, items that slide in (Bevy's
  `examples/animation/animated_ui.rs`, `color_animation.rs`);
- **a value that chases a moving target**: a health bar that trails the
  real value, a cursor that glides to the focused widget, a follower
  that lags. A tween can't do it because the target keeps moving.
  Bevy's `smooth_nudge` does it frame-rate independently
  (`crates/bevy_math/src/common_traits.rs`;
  `examples/movement/smooth_follow.rs`).

Both are presentation: drawn from time alone, so a frame-png of any
moment is exact, and reduced motion (2340) makes each a cut.

## Nick input

None.

## Scope

**In:**
- **`ease::Chase { half_life_ms: u32 }`** in `shuyi_core::ease`, with
  `step(current, target, dt_ms) -> T` for any `Lerp` type. The fraction
  left after `dt` is `2^(-dt/half_life)` in integers: whole halvings by a
  shift and the fraction from a 256-entry table with linear
  interpolation (about 1 KB, small enough for the GBA and Playdate
  targets), so every machine gets the same value; a `dt` of 2 equals two
  steps of 1 within one unit; it lands on the target exactly when
  within one unit, never creeping forever. An `f32` twin in
  `shuyi_ui::ease` for skins.
- **Transitions in the look file:** per role, `transition: (ease:
  OutCubic, ms: 120)`. A widget's paint reads the time its state
  changed from the screen's own clock and eases colour (`Rgb` `Lerp`),
  a pixel offset and an inset with 2109's `Tween`. Only (previous
  state, changed at) is stored.
- **Reduced motion:** with 2340's setting on, both cut to the end.

**Out (do not do):**
- keyframed tracks of any value (2364, parked);
- springs and overshoot (2109's docs decline them);
- the camera's own smoothing (2362, parked; it may reuse `Chase`).

## Implementation steps

1. Read `crates/core/src/ease.rs`, 3235's look file and 2340's setting.
2. Write `Chase` and its table, then the `f32` twin.
3. Add `transition` to the look file and ease the widget paint.
4. Honour reduced motion.
5. Write the docs page: a trailing health bar and a button that eases
   on focus.

## Acceptance criteria

- [ ] After one half-life the gap is half, within one unit.
- [ ] `step(dt=2)` equals two `step(dt=1)` within one unit, for random
  values (property test).
- [ ] A chase reaches its target exactly and stays there.
- [ ] A frame-png 60 ms into a 120 ms focus transition equals its
  golden picture.
- [ ] With reduced motion on, the first frame after a state change
  shows the end state.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] The skill pack (`skills/`) is updated for a changed schema, template, format or problem code, or the completion notes have a `Skills:` line saying why not (`cargo xtask docs-rules`, 3534).
- [ ] Guides in `docs/guides/` updated, if this adds a direction, a module or a move (ADR-0009 §5).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the table, `Chase`, the look file's parsing.
- Property: the step-split law; monotone approach.
- Snapshot / integration: frame-png goldens mid-transition; a golden
  row of `Chase` on wasm32.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
