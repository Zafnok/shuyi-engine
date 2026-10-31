---
id: "2363"
title: "Parked: Positional 2D sound: a world position on the request, a listener the screen sets, and gain and pan by distance"
type: feature
milestone: "Engine: real time"
model: implementation
effort: high
status: blocked
blocked_by: ["2109"]
nick_input: none
completed:
---

# 2363 — Positional 2D sound: gain and pan by distance from a listener

> **Parked** (Nick, 2026-10-20: "unless these real-time work tickets overlap with visions of shuyi they should be parked until visions of shuyi is complete"): this is real-time work, so it waits until Visions of Shuyi launches, unless the game needs it first (the game asks, through its `engine-change` skill, for sounds on a battle map bigger than the screen to fade and pan by where they happen; the grid listener here works over today's tile camera, so this ticket is then unparked whole, with nothing parked under it). Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

Found by the 2026-10-20 comparison with Bevy 0.20, Godot 4.7, Unity 6
and Unreal 5.8, and the 2D engines GameMaker, Defold and Construct 3.
Each of them can play a sound at a place in the world, quieter the
further it is from a listener and panned to the side it is on:

- Godot: `AudioStreamPlayer2D`, with a maximum distance, an attenuation
  curve and a panning strength, and `AudioListener2D`, which takes the
  camera's place as the listener
  (docs.godotengine.org/en/stable/classes/class_audiostreamplayer2d.html);
- Bevy: `SpatialListener` and the `SpatialScale` resource, with spatial
  playback settings on a sound;
- Unity: `AudioSource.spatialBlend` and rolloff curves. Unreal: sound
  attenuation settings;
- GameMaker: emitters, a listener and falloff models. Construct 3: its
  Audio object plays at a position and has a listener position. Defold
  plays a sound with a pan, and the game works out the pan itself.

What Shuyi has today:

- `AudioRequest::PlaySound { cue, volume }` (`crates/ui/src/audio.rs`):
  a sound has a volume and no place. `AudioQueue::play_sound_at(cue,
  volume)` sets a volume; its "at" is not a place. A game can fake
  distance by working out the volume itself. It can't pan.
- `SoundRequest::Sound { cue, volume }` (`crates/platform/src/audio.rs`):
  the same at the platform boundary, built by
  `SoundRequest::from_request`.
- `shuyi-app`'s player (`crates/app/src/audio.rs`): its `Backend` trait
  is `pub(crate)`, and `Backend::play(sound, volume, looped)` has no
  pan. Its device, `Macroquad`, is macroquad 0.4.16 over quad-snd 0.2.8,
  whose `PlaySoundParams` holds only `looped` and `volume`. It can't
  pan.
- `SoundCue { files, volume, credit }` in the audio manifest
  (`crates/assets/src/audio.rs`): no falloff. The pack pins the
  manifest's binary form in a snapshot (`crates/assets/src/pack/tests.rs`;
  `pack::VERSION` is 2 today).
- `shuyi_ui::grid::Camera` (`crates/ui/src/grid/camera.rs`): the tile
  camera, the top-left tile of the view. With the view's size in tiles
  (`GridSkin::view_tiles`, `crates/ui/src/grid/skin.rs`) it says what is
  on screen, so it can be a listener.
- The contract's `run` lists each sound as `Sound { frame, cue }`
  (`crates/cli/src/contract/session.rs`), with no volume.
- [ADR-0004](../../docs/adr/0004-assets-standard-files-in-the-engines-own-data-out-and-a-deterministic-movement-kit.md)
  §6 promised that positional sound, "pan and volume from the distance
  to the camera", is added to the cue request when a tick example needs
  it. No other ticket carries it: 3030's mixer gives each voice a left
  and a right volume (`Params::balance`) and leaves panning by position
  to this ticket.

Planned around it:

- 1120 (parked): the world camera of free-moving games;
- 2109 (After MVP): integer ease curves in `shuyi_core::ease` (`ONE`
  is 65536, and `Ease` names the curves), which this ticket's falloff
  uses. It is the one ticket this waits for;
- 2342 (sound groups), 2344 (looped sounds with handles) and 2345 (cue
  rules as data), all After MVP. 2345's rules run in `App`
  (`shuyi_platform::audio::cues::Cues`) on what `Game::collect_audio`
  hands out;
- 2218 (After MVP): writers of the engine's text formats, the audio
  manifest's among them;
- 2010 and 2013 (both done): a `Breaking:` line on a PR that breaks
  the API, and ADR-0039's rules for public types that can grow;
- 3029 (After MVP): the player's rules shared by every backend, and the
  Playdate's audio, whose SDK sample player has a left and a right
  volume;
- 2902 and 3030 (parked): the sound research and an integer mixer with
  stereo output.

Why it matters:

- **The game first.** Visions of Shuyi's battle maps can be bigger than
  the screen. Today a sound from a corner of the map that is off screen
  plays as loud as one under the cursor. The game hasn't asked for this;
  if it does, this ticket is unparked (the note above), and it needs
  nothing parked to work on a grid.
- **Other 2D games.** A top-down town (1125), a platformer (1127) or a
  real-time strategy game (2716) wants a door, a fire or a fight heard
  from where it is: quiet when far, on the left when it is on the left.
- **The end goal** (2026-10-19, an agent makes a small game in few
  lines): a sound at a thing's place is one call, and how it fades is
  data in the manifest, not code each game writes.

## Nick input

None.

## Scope

**In:**
- **`shuyi_ui::audio::spatial`** (`crates/ui/src/audio/spatial.rs`, a
  module of `crates/ui/src/audio.rs`), `no_std`, in integers only:
  - `SoundAt { x: i32, y: i32 }`: a place in **world units**, which the
    game picks. A free-moving world uses its world pixels. A grid uses a
    whole number of units per tile (below). The falloff distances in the
    manifest are in the same units;
  - `Listener { at: SoundAt, half_width: u32 }`: where the player hears
    from, and half the width of the view in the same units, so a sound
    at the edge of the screen pans all the way;
  - `Mix { gain: i32, pan: i32 }`, in 1/65536 (2109's
    `shuyi_core::ease::ONE`): `gain` from 1 to `ONE`, `pan` from `-ONE`
    (left) to `ONE` (right);
  - `spatial_mix(listener: &Listener, at: SoundAt, falloff: &Falloff)
    -> Option<Mix>`, in this order:
    - the distance is `isqrt(dx² + dy²)`, in `u128` (`u128::isqrt`), so
      no position overflows;
    - at or past `silent`, the answer is `None`: the sound is never
      sent;
    - otherwise, at or within `full`, the gain is `ONE`. So a falloff
      built in code with `full` at or over `silent` plays at `ONE`
      closer than `silent`, and nothing divides by 0;
    - otherwise the gain is `ONE - falloff.curve.ease(p)`, where `p =
      (distance - full) * ONE / (silent - full)`. A gain that comes out
      0 (a curve that reaches `ONE` before `p` does) is `None` too, so
      no silent sound is sent;
    - the pan comes from `dx` alone: `dx * ONE / half_width` in `i64`,
      clamped to `±ONE`, then times `falloff.pan` and divided by 100.
      It is 0 when `half_width` is 0. Rust's division rounds toward
      zero, so a sound mirrored left to right gets the opposite pan
      exactly;
  - `pan_gains(pan: i32) -> (i32, i32)`: the left and right gains in
    1/65536, by the balance law, with `pan` clamped to `±ONE` first:
    both `ONE` at the centre, `left = min(ONE, ONE - pan)`, `right =
    min(ONE, ONE + pan)`. Every backend that pans uses it, so a pan
    sounds the same on all of them;
  - `SoundAt::tile(pos: Pos, units_per_tile: i32)`: the centre of a grid
    tile (`shuyi_core::Pos`), `pos.x * units_per_tile + units_per_tile
    / 2` across and the same down.
- **Falloff as cue data** (`crates/assets/src/audio.rs`):
  - `SoundCue` gains `falloff: Option<Falloff>`, with `Falloff { full:
    u32, silent: u32, curve: Ease, pan: u8 }` (`Debug`, `Clone`, `Copy`,
    `PartialEq`, `Eq`, serde, `deny_unknown_fields`). `Ease` is 2109's,
    written by its names. `RawSound` reads it with `#[serde(default)]`,
    so every manifest written before reads unchanged:

    ```ron
    (id: "door", files: ["sfx/door.wav"], volume: 80, credit: Own,
     falloff: Some((full: 64, silent: 480, curve: Out(Quad), pan: 100))),
    ```

  - the manifest check (`Check::sound`) reports each of these at the
    cue's line: `silent` of 0, `full` over `silent`, `pan` over 100, and
    a curve that isn't known never to rise. The allowed curves are
    `Linear`, `Smooth`, `Steps(_)`, and `In`, `Out`, `InOut` or `OutIn`
    of `Sine`, `Quad`, `Cubic`, `Quart`, `Quint`, `Expo` or `Circ`.
    Everything else is reported: `Back`, `Elastic` and `Bounce`, which
    overshoot or bounce, and any curve added to `Ease` later until it is
    listed here, because a sound must never get louder as it moves
    away. The check matches what is allowed and reports the rest with
    `_`, so it compiles whether or not `Ease` and `Family` are
    `#[non_exhaustive]`;
  - **From 2024 (done):** callers outside `crates/assets` build a
    `SoundCue` with `SoundCue::new` (a `const fn`; the type is
    `#[non_exhaustive]`): `new` sets `falloff: None` and stays `const`.
    The other literals outside `Check::sound` are tests in
    `crates/assets` (the test of `new` in `audio.rs`, and
    `credits.rs`): they get `falloff: None`;
  - the manifest's binary form changes, so `pack::VERSION`
    (`crates/assets/src/pack.rs`) becomes the next number and the
    pinned snapshot in `crates/assets/src/pack/tests.rs` is accepted
    again. The manifest in those tests gets a cue with a falloff, so the
    pack's round trip covers it;
  - `docs/formats.md`'s paragraph on the audio manifest says a sound
    cue may carry a falloff;
  - **either order with 2218:** if it is done, its `AudioManifestRon`
    writer writes `falloff` for a cue that has one, and its round trip
    covers such a cue. If not, 2218 does it.
- **The requests** (`crates/ui/src/audio.rs`):
  - `AudioQueue::play_sound_from(cue, at: SoundAt)` pushes a new
    `AudioRequest::PlaySoundFrom { cue, at }`. `play_sound_at(cue,
    volume)` keeps its name and meaning;
  - `AudioQueue::set_listener(listener: Listener)` and `listener() ->
    Option<Listener>`. A listener lasts one frame: `Game::collect_audio`
    reads it, then `take` clears it. The screen that shows the world
    sets it in every update that may play a positioned sound, so no
    listener is ever left over from a screen that has closed;
  - `AudioRequest::PlaySound` gains `pan: f32`, from -1 (left) to 1
    (right), 0 at the centre. `play_sound`, `play_sound_at` and the menu
    sounds send 0;
  - `AudioRequest::cue` returns the cue of `PlaySoundFrom`, and
    `MusicState::request` ignores it as it ignores `PlaySound`.
- **Resolved in `Game`** (`Game::collect_audio`, `crates/ui/src/game.rs`):
  - each `PlaySoundFrom` becomes `PlaySound { cue, volume: gain / ONE,
    pan: pan / ONE }` from the cue's falloff and the frame's listener,
    or is dropped when `spatial_mix` gives `None`. Both divisions are
    exact in `f32`;
  - a cue with no falloff, or a frame with no listener, plays the sound
    the way `play_sound` would: volume 1, pan 0;
  - `is_known` accepts a `PlaySoundFrom` of a known sound cue (in debug
    builds an unknown one panics, as for `PlaySound`);
  - so the platform, the Harness (`audio_log`, `sounds`) and the
    contract see only `PlaySound`, with its volume and pan.
- **2345's cue rules. Either order with 2345:** they run in `App`
  (`shuyi_platform::audio::cues::Cues`) on what `Game::collect_audio`
  hands out, so a sound dropped here never reaches them and counts
  toward no gap, cap or priority. If 2345 is done, `Cues::apply` builds
  `SoundRequest::Sound` itself, so it copies `pan` from `PlaySound`, and
  a Harness test shows that a sound past `silent` starts no
  `min_gap_ms` gap. If not, 2345 does both (its own clause says so).
- **The platform boundary:** `SoundRequest::Sound` gains `pan: f32`,
  copied from `PlaySound`. `SoundRequest::from_request` maps a
  `PlaySoundFrom` (which `Game` never lets through) to a centred sound at
  volume 1, so the match stays total.
- **The player. Either order with 3029:** if 3029 is done, this goes in
  the shared backend trait it made, by the rule its ADR sets for a
  feature some backends lack (if that rule is a provided method with an
  `Unsupported` default and one warning, the pan follows it in place of
  `can_pan`). If 3029 is not done, it goes in `shuyi-app`'s `Backend`
  (`crates/app/src/audio.rs`), and 3029 moves it with the rest:
  - `Backend::play` takes a pan (0 for music and voices), and
    `Backend::can_pan(&self) -> bool` defaults to `false`;
  - a backend that can't pan plays the gain alone, centred. The player
    warns once per run, through `take_warnings`, the first time a sound
    with a pan other than 0 reaches such a backend: "this sound device
    can't pan: positioned sounds play centred";
  - the `Macroquad` device can't pan (quad-snd has no pan);
  - if 3029 is done, its Playdate backend can pan: it sets the sample
    player's left and right volumes from `pan_gains`, tested with
    3029's fake SDK;
  - **either order with 3027 (parked, GBA audio):** if 3027 is done,
    its GBA backend can pan too: each mixer voice's pan comes from
    `pan_gains`, tested with 3027's host mixer. If not, 3027 fills in
    the pan when it lands, as its own clause says;
  - **either order with 3031 (parked, in-house sound):** if 3031 is
    done, its `in-house-sound` backend can pan too: it starts each
    voice with 3030's `Params::balance` set from `pan_gains` divided by
    16 (1/65536 to 1/4096), with a test on 3031's null output that a
    full-right pan sounds only on the right. If not, 3031 does it, as
    its own clause says;
  - **either order with 3017 (parked, target capabilities):** if 3017
    is done, add `Pan` to the audio `features` of each profile whose
    backend now pans (`targets/playdate.ron` if 3029 is done,
    `targets/gba.ron` if 3027 is done). If not, nothing to do: 3017
    fills each profile's `features` from what its backend plays.
- **Listeners:**
  - `grid::Camera::listener(&self, view: (i32, i32), units_per_tile:
    i32) -> Listener` (`crates/ui/src/grid/camera.rs`): `at` is the
    centre of the view, `origin * units_per_tile + view * units_per_tile
    / 2` on each axis, and `half_width` is `view.0 * units_per_tile /
    2`. `view` is the skin's `view_tiles`, as `Camera::follow` takes it.
    With an even `units_per_tile` the mix is symmetric on screen for any
    view, odd or even; the docs suggest 16. The gain by distance doesn't
    change with the skin. The pan follows what is on screen, which does;
  - **either order with 1120:** if 1120 is done, add
    `Camera::listener` for its world camera: the centre of the view in
    whole world pixels and half its width at the camera's zoom. If not,
    1120 adds it.
- **Public types follow ADR-0039:** `SoundAt`, `Listener` and `Mix`
  are plain values with public fields, `Falloff` is data read from a
  file, and the new `AudioRequest` variant follows its rule for enums.
- **The contract:** `run`'s `Sound` (`crates/cli/src/contract/session.rs`)
  gains `volume` and `pan`, read from the `PlaySound`, and loses `Eq`
  with them. Raise `VERSION` (`crates/cli/src/contract.rs`) to the next
  number, regenerate the schemas with `cargo shuyi schema --out
  crates/cli/schemas`, add the new fingerprint to
  `crates/cli/schemas/versions.json`, and update the fake session
  (`crates/cli/src/contract/fake.rs`). `docs/contract.md`'s `run` row
  says the sounds come with their volume and pan.
- **The toy game** (`crates/ui/src/toy.rs`) gets a sound cue `door`
  with a falloff, for the tests.
- **An ADR** (the `write-adr` skill, the next free number): positioned
  sound. Places are in world units the game picks; falloff is cue data;
  the mix is in integers in `shuyi-ui`, resolved by `Game` once per
  frame, so tests and the contract see the result; the balance law; a
  backend that can't pan plays the gain alone. It keeps ADR-0004 §6's
  promise. ADR-0004's `Status:` line gains "its §6 promise of positional
  sound is kept by ADR-NNNN". In `docs/adr/README.md`, ADR-0004's row
  changes and the new ADR's row is added.
- **For the game:** this changes the API. `AudioRequest::PlaySound` and
  `SoundRequest::Sound` gain `pan`, `AudioRequest` gains a variant,
  `SoundCue` gains a field, and `run`'s `Sound` gains two. `Backend` is
  `pub(crate)` in `shuyi-app`, so its new parameter breaks no game
  unless 3029 made the trait public. The PR's `Breaking:` line (2010)
  says so, and the game's PR that moves the pin follows it. The game's
  own manifest reads unchanged; a pack baked before is refused and
  baked again.

**Out (do not do):**
- a looped sound that moves with a thing (a waterfall, an engine's hum):
  it needs 2344's handles. A follow-up ticket after 2344, written when a
  game asks;
- several listeners at once (split screen: 2387, a listener per view
  of 2386's): one listener a frame here;
- Doppler, HRTF, occlusion, reverb zones and filters by distance
  (occlusion and reverb traced through the level: 2911 researches them;
  proximity voice over this falloff: 2910);
- 3D sound: none of the 3D tickets (3101–3109) has it. A follow-up
  there if a 3D game asks;
- stereo on quad-snd, or replacing it (2902, 3030–3032). The mixer's
  per-voice left and right volumes are 3030's;
- music and voices by position: they stay centred;
- showing where a sound came from, in captions (2381, blocked
  by 2366 and this ticket) or in the debug view (2315's channels);
- volumes by group (2342): a positional gain only multiplies with them;
- the cue rules themselves: gaps (2345), copy caps and priority
  (2378).

## Implementation steps

1. Read `crates/ui/src/audio.rs`, `Game::collect_audio` and `is_known`
   in `crates/ui/src/game.rs`, `crates/assets/src/audio.rs`,
   `crates/assets/src/pack/tests.rs`, `crates/platform/src/audio.rs`,
   the player (`crates/app/src/audio.rs`, or where 3029 moved it),
   `crates/ui/tests/it/audio.rs`, 2109's `shuyi_core::ease` and
   ADR-0039 (2013's). If they are done, read 2345's `Cues`, 2218's
   `AudioManifestRon` writer and 1120's `Camera`.
2. Add `Falloff` to the manifest with its checks and tests. Update
   `SoundCue::new` and literals, `docs/formats.md`, the pack tests' cue,
   `pack::VERSION` and its snapshot, and 2218's writer if 2218 is done.
3. Write `spatial.rs`: the types, `spatial_mix`, `pan_gains` and
   `SoundAt::tile`, with the unit, property and exhaustive tests.
4. Add `play_sound_from`, the listener, `PlaySoundFrom` and `pan`, and
   the resolution in `Game`. Move the engine's own tests that build
   `PlaySound` to the new field. Add the toy's `door` cue and the
   Harness test.
5. Add `pan` to `SoundRequest::Sound`, and to 2345's `Cues` if it is
   done. Then add it to the player's backend with `can_pan` and the
   warning, to the Playdate backend if 3029 is done, to the GBA's if
   3027 is done and to the in-house one if 3031 is done, and `Pan` to
   their profiles if 3017 is done.
6. Add the grid listener, and 1120's if 1120 is done.
7. Add `volume` and `pan` to the contract's `Sound`, raise `VERSION`,
   regenerate the schemas and update `docs/contract.md`.
8. Write the ADR and run the gates. Expect about 600 changed lines, not
   counting the `pan: 0.0` added to existing tests, the schemas and the
   pack snapshot. If 3029 or 1120 is done and the diff goes well past
   that, leave the Playdate backend's pan or the world camera's
   listener to a follow-up ticket (the `write-ticket` skill), and say so
   in the completion notes.

## Acceptance criteria

- [ ] For random listeners, places and falloffs with an allowed curve:
      the gain never rises as a sound moves away from the listener along
      a straight line; a sound mirrored left to right gets the opposite
      pan and the same gain; mirrored up and down, the same mix; closer
      than `silent`, the gain is `ONE` at or within `full`; the answer
      is `None` at or past `silent`; a `Some` has a gain from 1 to `ONE`
      and a pan from `-ONE` to `ONE` (property tests).
- [ ] Every curve the manifest check allows is non-decreasing at every
      `p` from 0 to `ONE` (an exhaustive test).
- [ ] `pan_gains` gives `(ONE, ONE)` at 0, `(0, ONE)` at `ONE`, `(ONE,
      0)` at `-ONE`, and the same as `±ONE` past them. Neither gain
      rises as the pan moves away from its side.
- [ ] Over the grid camera with 16 units per tile, in views 35 and 34
      tiles wide, sounds on the leftmost and rightmost visible tiles get
      pans of the same size and opposite sign.
- [ ] A Harness test in `crates/ui/tests/it/audio.rs`: a screen there
      (as `Beeper` is) sets a listener and plays the toy's `door` under
      the listener, at the right edge of the view, and past `silent`.
      The audio log has `PlaySound` with volume 1 and pan 0, then one
      with a lower volume and a pan above 0, and nothing for the third.
- [ ] A frame with no listener, and a cue with no falloff, play the
      sound centred at volume 1.
- [ ] Each bad falloff (`silent` of 0, `full` over `silent`, `pan` over
      100, `In(Back)`, `Out(Elastic)`, `InOut(Bounce)`) is reported at
      its cue's line. A manifest with no falloff reads as before. The
      pack's round trip reads the cue with a falloff back equal, under
      the next `pack::VERSION`.
- [ ] With the player's fake backend, a positioned sound's pan reaches
      `play` on a backend that can pan; on one that can't, the volume
      does and the warning comes once in a run of several such sounds.
- [ ] `shuyi run`'s answer lists each sound's volume and pan, under the
      raised contract version, and the schemas test passes.
- [ ] If 2345 is done: `Cues` passes a positioned sound's pan into
      `SoundRequest::Sound`, and a sound past `silent` starts no gap.
- [ ] If 3029 is done: the Playdate backend's fake SDK gets the left and
      right volumes `pan_gains` gives.
- [ ] If 3027 or 3031 is done: its backend gets the balance
      `pan_gains` gives, and a full-right pan sounds only on the right.
      If 3017 is done: each profile whose backend pans lists `Pan`.
- [ ] `shuyi-ui` and `shuyi-assets` still build for
      `thumbv7em-none-eabihf`.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `spatial_mix` at `full`, at `silent`, with `full` equal to
  `silent` and over it, with `half_width` 0, with a curve whose gain
  reaches 0 before `silent`, and at extreme positions (`i32::MIN`
  against `i32::MAX`); `pan_gains`, past `±ONE` too; `SoundAt::tile`;
  `grid::Camera::listener`; each falloff error; the resolution in
  `Game` (dropped, centred without a listener, centred without a
  falloff); the player's pan and the warning with the fake backend.
- Property: monotonic gain, symmetry, the ranges, and that a resolution
  of the same requests is the same twice.
- Exhaustive: every allowed curve is non-decreasing at every `p`.
- Snapshot / integration: the Harness test with the toy game; the pack
  round trip and its snapshot; the contract schemas.

## Completion notes

*(Filled in by the session that completes the ticket.)*
