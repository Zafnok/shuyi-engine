---
id: "2370"
title: "Parked: Ambiences for weather: particles across the view, a tinted overlay and a looped sound as one named preset, switched on a step with a fade"
type: feature
milestone: "Engine: 2D effects"
model: sonnet-5 / gpt-6.1-terra
effort: high
status: blocked
blocked_by: ["2103", "2337", "2344"]
nick_input: none
completed:
---

# 2370 — Ambiences for weather: particles, an overlay and a loop as one preset

> **Parked** (Nick, 2026-10-22: "some random thoughts I have are supporting weather system, day/night cycle, and proximity audio and voice chat"): Visions of Shuyi doesn't ask for weather, and particles (2103) are real-time work that waits until the game launches. Nobody starts it until Nick unparks it, or the game asks for weather through its `engine-change` skill, which unparks 2103 with it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

Nick asked on 2026-10-22 whether the engine supports a weather system
(`docs/decisions.md`, "weather, day and night, voice chat and ray
tracing get tickets"). Weather is mostly the game's: which kinds there
are, when they change, and what they do to the rules (rain that weakens
fire, snow that slows a unit) are game rules (`AGENTS.md` rule 4). What
every game with weather writes the same is its **look and sound**: rain
or snow across the screen, the screen a little darker or greyer, a
loop of rain or wind, and a fade from one to the next. No engine this
one was compared with ships weather as a system either: Godot, Unity
and GameMaker games build it from particles, a full-screen tint and an
audio loop.

The pieces are all planned, none together:

- **Particles** (2103, parked): `EmitterDef` as data,
  `Emitter { origin, seed, start_step }`, and `particles_at`, each
  particle a function of the seed and the step. An emitter has a point
  origin only, so rain across the whole view needs an area to be born
  in.
- **A tinted overlay** (2337): `Overlay.opacity` and `Overlay.blend`
  (`Blend::Multiply` darkens, `Blend::Add` lightens), drawn the same by
  every renderer and by `frame-png`.
- **Looped sounds with handles** (2344): `AudioQueue::start_sound(cue)
  -> SoundId` and `stop_sound(id, fade_ms)`, owned by the screen that
  started them.
- **A light** (2369, parked): a light cycle for the time of day.
- **Named random streams** (2114): its own example is `"weather"`, so
  the game's draws for the weather never move its other draws.

The generic shape: **an ambience**, a named preset of particles, an
overlay and a looped sound, that a screen shows from a small plain
value the game keeps in its state: which ambience, which one before
it, and the step (or turn) it changed on. Examples that aren't
tactics: a farming game's rain on the fields, a horror game's fog, a
platformer's snowy level, a visual novel's rainy scene.

## Nick input

None.

## Scope

**In:**
- **Emitters born in an area**, added to 2103's `EmitterDef` in
  `shuyi_core::particles`: `spawn: Spawn`, `Point` (the default, what
  2103 built, so every emitter file stays valid), `Line { length: Fx }`
  along x from `origin`, or `Rect { w: Fx, h: Fx }` from `origin`.
  Particle `i`'s birth place comes from its own random numbers
  (`mix_at(seed, i)`, 2114), as its angle and speed do. The emitter file
  reads `spawn` (ADR-0007: the format's docs and its fuzz-style test
  cover it).
- **Emitters that follow the view**: a particle layer whose origin is
  in view pixels, not world pixels, so rain stays over the screen while
  the camera moves. Particles that leave the view wrap to its other
  side (`x mod view width`), so a slow drift never empties the screen.
- **`Ambiences`**, a new asset kind (`ambience`) read by `AmbienceRon`
  (`.ambience.ron`) in `crates/assets/src/ambience.rs`, every ambience
  by name:
  ```ron
  {
      "clear": (),
      "rain": (
          particles: [(emitter: "rain_drops", seed: 7)],
          overlay: (colour: "rain_grey", opacity: 64, blend: Multiply),
          sound: "rain_loop",
      ),
      "fog": (overlay: (colour: "fog_white", opacity: 96, blend: Alpha)),
  }
  ```
  - each part is optional; `()` is an ambience with nothing in it;
  - checked against the game's other files (ADR-0019): an emitter the
    game has no emitter file for, a colour its palette lacks, or a cue
    its audio manifest lacks (or whose cue isn't `looped`) is an error
    naming the ambience and the part. Every problem is reported.
- **`AmbienceState`** in `shuyi_ui` (plain data, serde, `Eq`, the value
  the game keeps): `now: String`, `before: Option<String>`,
  `since: u64` (the step or turn it changed on). `AmbienceState::new(name)`
  and `switch(&mut self, to, at: u64)`. The engine never decides when it
  switches: the game's rules do, with its own random stream if it
  wants one.
- **The view and the skin**: `AmbienceView::build(&Ambiences,
  &AmbienceState, now: u64, fade: u64)`, where `fade` is the switch's
  length in the same unit:
  - the overlay's opacity goes from the old ambience's to the new
    one's in a straight line over `fade` (and from or to 0 when one
    side has no overlay); the colour is the new one's, or the old one's
    while only it has one;
  - the old ambience's emitters stop emitting at `since` (their
    particles live out their lives), and the new ones start at `since`,
    so a switch is a function of `since` and `now` alone;
  - the frame gets the particle layers over the world and the overlay
    over them, under the UI. A glyph skin draws the particles as 2103's
    glyph helper does, and the overlay as 2337's overlay.
- **The sound**: `AmbienceSound`, kept by the screen, compares the
  state's `now` with the ambience it last started, and on a change
  calls `stop_sound(old, fade_ms)` and `start_sound(new)` on its
  `AudioQueue` (2344). `fade_ms` is a parameter the screen passes.
- **The scene and the contract**: the scene text prints the ambience
  line (`ambience rain since 120`, and `from clear` while fading), so a
  test of what happened reads which ambience is on, never the pixels
  (`AGENTS.md` rule 9).
- **Docs**: the format's row in `docs/formats.md`, and a "weather"
  section in the guide the docs map names for skins, which says plainly
  that the rules of weather are the game's.

**Either order with 2369:** if 2369 is done, the weather section of the
guide shows an ambience over a light cycle. If not, nothing to do
(2369's guide section shows the two together, by its own clause).

**Out (do not do):**
- any rule of weather: what it changes, when it changes, its chances
  (the game's; 2114's named streams give it its own draws);
- a weather "system" with forecasts, seasons or regions;
- splashes where drops land, puddles, wet sprites, lightning flashes
  timed with thunder: art and rules a game adds over this;
- wind that bends grass or trees, or particles that collide (2103's
  Out keeps collision for a follow-up after 1122);
- screen passes on the wgpu renderer such as blur, rain on the lens or
  heat haze (2326, 2327);
- sound by position: rain plays everywhere (2363 is positional sound);
- an example game: an example with weather is written when an example
  wants it (1125's walkable town is the likeliest).

## Implementation steps

1. Read 2103's, 2337's and 2344's code as built, and ADR-0019's checks
   of one file against another.
2. Add `Spawn` to `EmitterDef` and the emitter format, with its tests.
3. Add the view-following particle layer and its wrap.
4. Write `crates/assets/src/ambience.rs`: `Ambiences`, `AmbienceRon`,
   `from_source` with its checks, registered in
   `crates/assets/src/format.rs`; add the kind to the pack.
5. Write `AmbienceState`, `AmbienceView` and `AmbienceSound` in
   `shuyi-ui`, and the scene line.
6. Add the determinism fixture for the new format, the docs, and run
   the gates.

## Acceptance criteria

- [ ] An emitter with `spawn: Rect` puts every particle's birth place
      inside the rectangle, and the same seed and step give the same
      particles on every run.
- [ ] Every emitter file written for 2103 still loads unchanged
      (`spawn` defaults to `Point`).
- [ ] An ambience file naming a missing emitter, colour or cue, or a
      cue that isn't looped, fails to load with one error per problem.
- [ ] Halfway through a fade from an ambience with an overlay at
      opacity 64 to one with none, the overlay's opacity is 32.
- [ ] After a switch, the old ambience's emitters make no new particle
      past `since`, and the new ones' first particles are born at
      `since`.
- [ ] Switching ambience stops the old loop with the given fade and
      starts the new one, once, read from the `AudioQueue`'s requests.
- [ ] The scene text says which ambience is on and which it fades from.
- [ ] `docs/formats.md` lists the `ambience` kind.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `Spawn` for each shape; the view wrap; the overlay fade at 0,
  half and the end; `AmbienceState::switch`; `AmbienceSound` on a
  change, on no change, and on a switch back before the fade ends.
- Property: an ambience view at `now` is the same whatever steps were
  built before it; random bytes never panic `AmbienceRon`.
- Snapshot / integration: a scene's ambience line through a switch; a
  `frame-png` picture of rain over a grid at one step (a look test, so
  it reads pixels); the format's round trip through the registry and
  the pack.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
