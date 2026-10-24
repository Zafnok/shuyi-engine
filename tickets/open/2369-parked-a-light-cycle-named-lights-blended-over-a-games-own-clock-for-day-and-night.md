---
id: "2369"
title: "Parked: A light cycle: named lights blended over the game's own clock, for day and night, read from a cycle file and painted by the grid skin"
type: feature
milestone: "Engine: 2D effects"
model: sonnet-5
effort: medium
status: blocked
blocked_by: []
nick_input: none
completed:
---

# 2369 — A light cycle: named lights blended over the game's clock

> **Parked** (Nick, 2026-10-22: "some random thoughts I have are supporting weather system, day/night cycle, and proximity audio and voice chat"): Visions of Shuyi doesn't ask for a day and night, so this waits until after the MVP like the rest of the 2D effects, unless the game asks for it first through its `engine-change` skill. Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

Nick asked on 2026-10-22 whether the engine supports a day and night
cycle (`docs/decisions.md`, "weather, day and night, voice chat and ray
tracing get tickets"). No ticket builds one. Most engines leave it to
the game, built from a light and a curve: Godot's `CanvasModulate`
animated by an `AnimationPlayer` or a `Gradient`, Unity's URP 2D
Global Light with a script, GameMaker's layer colour over a timer.

What Shuyi has today:

- **Named lights.** `crates/assets/src/lighting.rs`: `Light { grey,
  tint }` in thousandths (integers, so a light is a plain value a frame
  item carries and compares), `Light::apply` (the one place that knows
  the sum), and `Lighting`, every light by name, read from a lights
  file by `LightingRon` (ADR-0007). A lights file must have the
  default light (`noon` unless the game says, `LightingRon::new`).
- **A map is in one light.** `MapLook::light`
  (`crates/assets/src/grid_map.rs`) names it; the grid scene carries it
  (`crates/ui/src/grid/scene.rs`, `look.light`, printed as
  `light dusk` in the scene text when it isn't the usual one); the
  ground painter looks it up (`crates/ui/src/grid/sprite/ground.rs`,
  `looks.lights.get(&scene.look.light)`) and paints every ground
  sprite `Paint::Shaded(light)`.
- **Nothing blends two lights.** A game can switch from `noon` to
  `dusk`, but not go between them.

The generic shape: **a cycle of named lights over a length the game
gives**, so a day goes noon, dusk, night, dawn and back, and the frame
at any moment is a blend of two lights, worked out from the time alone.
The time is the game's own clock, in steps (tick games, 2115's
`STEPS_PER_SECOND`) or turns (a tactics game whose day passes one turn
at a time), never the wall clock (`AGENTS.md` rule 5). Examples that
aren't tactics: a farming game's day, a survival game's night, a
platformer stage that gets dark as the player climbs.

What the day *does* stays the game's (`AGENTS.md` rule 4): when shops
close, what comes out at night, whether the player sleeps. This ticket
only makes it look like the time it is.

## Nick input

None.

## Scope

**In:**
- **`Light::mix(self, to: Light, permille: u16) -> Light`** in
  `crates/assets/src/lighting.rs`: each of `grey` and the three `tint`
  numbers goes `permille` thousandths of the way from `self` to `to`,
  rounded to the nearest (half away from zero), in integers. `0` gives
  `self`, `1000` gives `to`, more than 1000 counts as 1000.
- **`LightCycle`**, a new asset kind (`light_cycle`) in a new module
  `crates/assets/src/light_cycle.rs`, read by `LightCycleRon`
  (`.cycle.ron`, ADR-0007, ADR-0019):
  ```ron
  // A day: each stop is a light at a point of the cycle, in thousandths.
  (
      stops: [
          (at: 0,   light: "night"),
          (at: 220, light: "dawn"),
          (at: 300, light: "noon"),
          (at: 700, light: "noon"),
          (at: 780, light: "dusk"),
          (at: 860, light: "night"),
      ],
  )
  ```
  - `stops`, at least one, with `at` from 0 to 999 and strictly rising;
    the last stop blends into the first again (the cycle wraps);
  - checked against the game's `Lighting` like a voice manifest against
    the dialogue (`voice::from_source`, ADR-0019): a stop that names no
    light is an error naming the stop, the light and the lights there
    are (`Lighting::names`). Every problem is reported, as
    `lighting::from_source` does;
  - the file's size cap is `TEXT_MAX_BYTES`, like the lights file.
- **`LightCycle::at(&self, now: u64, length: u64) -> LightBlend`**, the
  point of the cycle `now` is in, for a cycle `length` long (steps or
  turns, the game says which; a `length` of 0 counts as 1), in exact
  integer maths (`now % length`, scaled to millionths of the cycle so a
  long day still moves smoothly):
  ```rust
  pub struct LightBlend { pub from: String, pub to: String, pub permille: u16 }
  ```
  `LightBlend` is plain data, serde, `Eq`. A `LightBlend` whose two
  lights are the same (between `noon` at 300 and `noon` at 700) has
  `permille: 0`. It isn't called `Blend`: that name is 2337's
  `shuyi_ui::frame::Blend`, the blend mode a sprite or an overlay
  carries.
- **The grid scene carries a blend.** `look.light` stays the map's
  light. A new `look.light_blend: Option<LightBlend>` in
  `crates/ui/src/grid/scene.rs`, set by the screen from
  `LightCycle::at`, wins over it when it is `Some`. The scene text
  prints `light dusk>night 400` for a blend (and nothing when there is
  none, so every snapshot today is unchanged). The ground painter
  (`ground.rs`) paints `Paint::Shaded(from.mix(to, permille))`, looking
  both lights up in `Looks::lights`; a light the lights file doesn't
  have is painted plain, as a missing map light is today.
- **Docs**: the format's row in `docs/formats.md`, and a short "a day
  and a night" section in the guide the docs map names for skins (the
  `document` skill says where).
  - **Either order with 2370:** if 2370 is done, the "a day and a
    night" section shows an ambience over the light cycle. If not,
    nothing to do (2370's "weather" section shows the two together).

**Either order with 2109:** if 2109 is done, a stop may name the
`Ease` its blend into the next stop uses (`ease: InOut(Sine)`, default
`Linear`), applied to `permille` before `Light::mix`. If not, every
blend is straight, and 2109 adds `ease` to the stops when it lands
(its clause on 2369 says so).

**Either order with 2328:** if 2328 is done, a lit world layer (1120,
2328) takes `look.light_blend` too, and when it is `Some` the mixed
light (`Light::mix`) is the layer's ambient in place of its `ambient`,
so point lights show at night. If not, nothing to do (2328 adds it, its
clause says so).

**Out (do not do):**
- the game's clock and what time of day means to its rules (the game's;
  2115 gives it `Countdown` and `Every`);
- a sky, a sun or a moon that moves, stars, shadows that turn with the
  sun: art the game draws, or 2328's lights;
- weather (2370 brings its look together);
- blends on frames other than the grid scene's ground, but for a lit
  world layer's ambient (the clause with 2328 above). Any other world
  layer of free-moving games (1120) takes the same `look.light_blend`
  in the ticket that first needs it, as 1120's **Out** says;
- floats anywhere in the cycle: `Light::apply` keeps its floats for the
  pixel sum only, as today.

## Implementation steps

1. Add `Light::mix` and its unit tests.
2. Write `crates/assets/src/light_cycle.rs`: `LightCycle`,
   `LightBlend`, `from_source(file, source, &Lighting)`,
   `LightCycle::at`, and `LightCycleRon` registered in
   `crates/assets/src/format.rs` beside `LightingRon`. Follow how
   `voice::from_source` checks one file against another.
3. Add the format to the pack's kinds (`crates/assets/src/pack`) and
   update the pack's snapshot test if its kinds list is snapshotted.
4. Add `look.light_blend` to the grid scene, its scene text, and the
   ground painter's use of it.
5. Add the determinism fixture for the new format (ADR-0007 §5, as the
   other text formats have in `crates/assets/src/format/builtin_tests.rs`).
6. Write the docs, then run the gates.

## Acceptance criteria

- [ ] `Light::mix(a, b, 0) == a`, `Light::mix(a, b, 1000) == b`,
      `Light::mix(a, b, 2000) == b`, and halfway between `noon` and
      `night` is each number's rounded midpoint.
- [ ] A cycle file with stops out of order, an `at` of 1000 or more,
      no stops, or a light the lights file lacks fails to load, and the
      errors name each problem.
- [ ] `LightCycle::at` gives the stop's own light at each stop's `at`,
      blends across the wrap from the last stop to the first, and gives
      the same `LightBlend` for `now` and `now + length`.
- [ ] A grid scene with `look.light_blend: Some(..)` paints its ground
      `Paint::Shaded` with the mixed light, and its scene text says
      `light dusk>night 400`. With `None`, every existing scene and
      snapshot is unchanged.
- [ ] `docs/formats.md` lists the `light_cycle` kind.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `Light::mix` at 0, 1000, past 1000 and halfway, with rounding;
  `LightCycle::at` at stops, between them, across the wrap, with a
  `length` of 0 and 1, and at `now` near `u64::MAX`.
- Property: for any two lights and `permille`, each number of
  `Light::mix` lies between the two lights' numbers;
  `LightCycle::at(now, len) == LightCycle::at(now + len, len)`.
- Snapshot / integration: a grid scene's text with a blend; the ground
  painter's sprites under a blend (read from the frame's sprite items,
  as `ground.rs`'s tests read `Paint::Shaded(dusk)`); the format's
  round trip through the registry and the pack.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
