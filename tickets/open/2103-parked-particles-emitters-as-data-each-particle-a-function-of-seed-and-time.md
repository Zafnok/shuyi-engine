---
id: "2103"
title: "Parked: Particles: emitters as data, each particle a function of the seed and the time, painted by the skin"
type: feature
milestone: "Engine: animation"
model: sonnet-5
effort: high
status: blocked
blocked_by: ["1119", "1123"]
nick_input: none
completed:
---

# 2103 — Particles: emitters as data, painted by the skin

> **Parked** (Nick, 2026-10-14: "document the parked (i.e. after MVP/parity) tickets for networking, styles for later"): the game doesn't need particles, so this waits until after the engine reaches parity with the game (the MVP in `docs/ROADMAP.md`) and after 1119 and 1123. Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

Found by the 2026-10-20 comparison with Bevy 0.20 and Godot 4.6: both
ship particles (Godot's `GPUParticles2D` and `CPUParticles2D`; Bevy
through the `bevy_hanabi` crate), and no Shuyi ticket builds them.

The rule is already decided. [ADR-0004](../../docs/adr/0004-assets-standard-files-in-the-engines-own-data-out-and-a-deterministic-movement-kit.md)
§3: "an emitter's particles are a function of its seed and the time,
never stored one by one". ADR-0005's skill table lists particles in the
`shuyi-animate` skill beside clips. So a tick simulation (1119) stores
one small record per emitter, replays and rollback (1147) stay exact,
and `frame-png` can draw any moment without running the steps before it.

This ticket builds on:

- 1119: `Fx`, the 256-step angle and its sine table, and steps;
- 1123: `AnimationClip` and `frame_at`, so a particle can be a clip.

Graphics are a skin (ADR-0002 §4, `AGENTS.md` rule 9). Logic says where
the particles are, and the skin decides how they look: a glyph skin
draws a character in a cell, a sprite skin draws a sprite on a world
layer (1120).

## Nick input

None.

## Scope

**In:**
- **`shuyi_core::particles`**, the maths, pure and in `Fx` (no floats):
  - `EmitterDef`, plain data:
    - `burst: u16`, particles born at the start;
    - `rate: Fx`, particles a second after that (0 for a burst only);
    - `duration_ms: Option<u32>`, how long it emits (`None` = forever);
    - `life_ms: (u32, u32)`, the range a particle lives;
    - `angle: (Angle, Angle)` and `speed: (Fx, Fx)` (pixels a second),
      the ranges it starts with;
    - `accel: (Fx, Fx)`, pixels a second squared (gravity, wind);
    - `max_alive: u16`, a cap;
    - `look: Vec<LookKey>`, what the skin draws over the particle's
      life: each `(permille_of_life, look index)`.
  - `Emitter { origin: (Fx, Fx), seed: u64, start_step: u32 }`: all a
    simulation stores. It is `Copy` and serde, like 1119's records.
  - `particles_at(def, emitter, step, steps_per_second) -> impl
    Iterator<Item = Particle>`. Particle `i` is born at
    `i / rate` seconds (after the burst), lives the life its own random
    numbers pick, and is at `origin + v·t + ½·a·t²` at age `t`. Its
    random numbers come from a stateless mix of `(seed, i)`
    (SplitMix64's finalizer, written in `rng.rs` beside `SimRng`), never
    from `SimRng`'s stream, so particle `i` never depends on the
    others or on which steps were computed before.
  - It walks only the indices born in `[now − longest life, now]`, so
    the cost is the particles alive, not every particle ever born.
  - `Particle { x: Fx, y: Fx, age_permille: u16, look: u8, index: u32 }`.
- **The format** (ADR-0007, ADR-0019): `EmitterDef` is an asset kind
  (`AssetKind::NAME = "emitter"`), and `shuyi_assets::particles::EmitterRon`
  reads it from RON, registered in `Formats::register_standard`, with
  its id, extension, size limit and a property test that random bytes
  never panic it. Errors name the file and line:
  - a range whose minimum is above its maximum;
  - look keys not in rising order, or a look index past the skin's list;
  - `rate × longest life` above `max_alive` (a warning: the cap drops
    particles).
- **The view and the skins** (`shuyi-ui`):
  - `ParticlesView`: the particles of the emitters on screen, in
    console pixels, with their look index;
  - a glyph skin helper, `particles::glyph::paint(&ParticlesView,
    looks: &[(char, role)], &mut GlyphBuffer)`, that draws each particle
    as a character in the cell it is in, on the `Over` layer;
  - a sprite skin, `particles::sprite`, that draws each one as a frame
    of a clip (1123), on a world layer. **Either order with 1120:** if
    1120 is done, add it here; if not, add a line to 1120's scope
    asking for it, and leave the sprite skin to 1120.
  - snapshot lines for particles (`frame/snapshot.rs`), with positions
    rounded to pixels, and the format's doc comment updated.
- **An example:** the tick example from 1119 (the dot) leaves a trail of
  sparks while it moves. A test pins the snapshot at three steps.

**Out (do not do):**
- particles colliding with tiles or boxes (that needs state per
  particle; a follow-up ticket after 1122 if a game asks);
- a moving emitter whose old particles stay where they were born (that
  needs the emitter's past positions); particles here move with the
  emitter's `origin`, and a burst at a point is a new emitter;
- attractors, sub-emitters, trails as strips (2703 draws strips);
- GPU particles and the wgpu renderer (3034);
- an editor for emitters (2506–2508).

## Implementation steps

1. Read ADR-0004 §3, 1119's `Fx` and `Angle`, and 1123's `anim`.
2. Write the mix function and its tests in `shuyi_core::rng`.
3. Write `EmitterDef`, `Emitter` and `particles_at`, with property tests.
4. Write `EmitterRon` and register it; add fixtures (ours, CC0).
5. Write the view, the glyph skin and, if 1120 is done, the sprite skin.
6. Add the example's trail, its snapshots, and the skill table's line
   in `docs/` if the `shuyi-animate` skill exists (1134); otherwise
   1134 covers it from ADR-0005.
7. Run the gates.

## Acceptance criteria

- [ ] `particles_at` for a step gives the same particles whether or not
      earlier steps were computed (a property test).
- [ ] The example's replay hash is the same with and without the trail
      drawn (particles never change the simulation).
- [ ] No more than `max_alive` particles are ever returned.
- [ ] Random bytes never panic `EmitterRon`; bad fixtures give errors
      naming the file and line.
- [ ] The example's snapshots at three steps are committed and reviewed
      with `frame-png`.
- [ ] All gates in the `run-gates` skill pass, including the `no_std`
      build of `shuyi-core`.

## Tests required

- Unit: a burst of 3 at step 0; a rate of 10 a second over 1 s; a
  particle dying exactly at its life's end; gravity bends the path.
- Property: same inputs ⇒ same particles; particles never appear before
  their birth; the count never passes the cap; the mix function's output
  for `(seed, i)` doesn't depend on any other index.
- Snapshot / integration: the example's trail at three steps; a
  determinism fixture holding the hash of the particles at ten steps.

## Completion notes

*(Filled in by the session that completes the ticket.)*
