---
id: "2340"
title: "Frame shake and flash, and the player's settings for reduced motion, shake strength and flashing"
type: feature
milestone: After MVP
model: sonnet-5
effort: high
status: todo
blocked_by: ["2339"]
nick_input: none
completed:
---

# 2340 — Frame shake and flash, and the player's motion, shake and flashing settings

## Context

Found by the 2026-10-20 comparison with Bevy 0.20, Godot 4.7, Unity 6
and Unreal 5.8. Unity's Cinemachine Impulse shakes cameras when
something happens in the game
(<https://docs.unity3d.com/Packages/com.unity.cinemachine@3.1/manual/CinemachineImpulse.html>),
and Unreal has camera shakes. Godot and Bevy leave shake and flash to
the game's code. None of the four gives the player a setting for less
motion or no flashing. The web has `prefers-reduced-motion`, and WCAG
2.3.1 asks that nothing flash more than three times in any one second
(<https://www.w3.org/WAI/WCAG22/Understanding/three-flashes-or-below-threshold.html>).
Outside the benchmarks, Phaser's and HaxeFlixel's cameras have shake
and flash built in.

What Shuyi has today (checked 2026-10-21):

- No frame offset. `GlyphBuffer` (`crates/ui/src/frame/glyph_buffer.rs`)
  places cells on a grid and items on console pixels, and every painter
  draws it at its place.
- No flash of the whole frame. An `Overlay` has no opacity. 2339 adds
  the frame's veil: one colour over the whole frame at an alpha, drawn
  last by every painter, the one-bit renderer in a Bayer order.
- `Settings` (`crates/ui/src/settings.rs`): text speed, fullscreen, the
  theme, two volumes and the layout. Nothing about motion or flashing.
  Each game saves them in its own form (`GameSettings::to_ron`).
- `IdleMotion` (`crates/ui/src/ctx.rs:163-171`) is a switch the platform
  turns on (`Setup::ctx`, `crates/platform/src/app.rs:141`) and tests
  leave off. It isn't the player's. Game ADR-0059 expected "a later
  reduce motion option" to set it, but the platform writes that field
  and tests rely on it being off.
- Planned and parked: 2326's `Settings::flashing` and its `Flash` pass
  wait for the wgpu renderer (3034); 1120's camera shake waits for 1119;
  2722 has a visual novel's own shake and flash; 2362 scales 1120's
  shake by "the player's settings from 2340"; 2359 holds a map's clock
  while idle motion is off.

Why it matters:

- **The game first.** The game shakes and flashes in its own code today,
  so parity needs nothing from here, and no game ticket asks. What it
  gains is a reduced-motion and a flashing setting its players can set,
  and one place to honour them. Moving its own code onto these is the
  game's choice after a pin bump.
- **Other 2D games.** A shake on a hit and a flash on a critical are one
  call each, on glyph, sprite and one-bit frames alike. Players who need
  less motion or no flashing get one setting for every game on the
  engine. The photosensitivity setting shouldn't wait for wgpu (2326).
- **The end goal** (2026-10-19, `docs/decisions.md`): one call, and
  `frame-png` shows it, on the Playdate too.

Graphics are a skin ([ADR-0002](../../docs/adr/0002-inherited-rules-layers-determinism-skins-actions-tests-licenses.md)
§4, `AGENTS.md` rule 9). Shake and flash change what is painted, never
what happened. A screen asks for them as it asks for a sound.

## Nick input

None.

## Scope

**In:**
- **The player's settings** (`crates/ui/src/settings.rs`), each put
  back by `Settings::restored`:
  - `motion: Motion`, `Full` (the default) or `Reduced`, with `ALL`,
    `key()` (`options.value.full`, `options.value.reduced`) and serde,
    as `TextSpeed` has;
  - `shake_strength: u8`, 0 (none) to 100 (the default): how strongly
    the frame shakes;
  - `flashing: bool`, on by default.
- **`Ctx::shows_idle_motion() -> bool`** (`crates/ui/src/ctx.rs`): idle
  motion on and `motion` `Full`. The docs of `IdleMotion` and of the
  `idle_motion` field say a screen's idle decoration reads this, and why
  the setting can't write the field (the platform sets it, and tests
  rely on it being off).
- **Option rows** (`crates/ui/src/screens/options.rs`):
  - `OptionRow::motion()`, a `RowValue::Choice` over `Motion::ALL`, as
    `OptionRow::text_speed` is. **Either order with 2314:** if done,
    built with `OptionRow::choice`, and `Motion` implements
    `ChoiceSetting`;
  - `OptionRow::shake_strength()`, a `RowValue::Volume` slider, 0 to
    100;
  - `OptionRow::flashing()`, a `RowValue::Toggle`.
  - The toy's options screen (`toy::option_rows`, `crates/ui/src/toy.rs`)
    shows the three after `sound_volume`, and `ROWS` in
    `crates/ui/src/screens/options/tests.rs` gains them.
  - Their text (`options.row.motion`, `options.row.shake_strength`,
    `options.row.flashing`, `options.value.full`,
    `options.value.reduced`) goes in `crates/ui/src/toy/en.ron`.
    **Either order with 2351:** if the engine has built-in English, the
    English goes there.
- **Saved forms:** every `GameSettings` in the repository saves the
  three fields, and an old saved form without them reads the defaults
  (`#[serde(default)]`, as today):
  - the toy's (`crates/ui/src/toy.rs`);
  - the grid template's (`templates/grid/crates/game/src/settings.rs`);
  - the engine's `impl GameSettings for ()` (`crates/ui/src/settings.rs`,
    from 2511, which the blank template uses);
  - `examples/hello-grid/src/settings.rs`.

  The templates aren't formatted by `cargo fmt --all`; CI's template
  job checks them.
- **`shuyi_ui::frame::shake`** (`crates/ui/src/frame/shake.rs`,
  `no_std`, integers only):
  - `Shake { trauma: u16, max_px: u8 }`, what a screen asks for:
    `Shake::new(trauma)` (thousandths, 0 to 1000) with `max_px` at
    `DEFAULT_SHAKE_PX` (4, *tunable*), and `with_max_px(px)`.
  - `FrameShake`, the shake playing: its trauma, the largest `max_px`
    asked for since the trauma was last 0, and the time in ms (`u64`).
    - `add(shake)` adds the trauma, capped at 1000;
    - `advance(now_ms: u64)` lowers it by `SHAKE_DECAY_PER_S` (1000,
      *tunable*: a full shake lasts one second) for each second since
      the last call, in whole ms, never below 0;
    - `offset() -> (i32, i32)`: an amplitude of
      `max_px × trauma² / 1 000 000` whole console pixels. On axis `a`
      (0 for x, 1 for y) the offset is that amplitude times `n`,
      divided by 1000 and rounded toward zero, where `n` is
      `splitmix64(SHAKE_SEED ^ (a << 32) ^ (now_ms / 33)) % 2001`, less
      1000. So the direction changes about 30 times a second.
      `splitmix64` is `audio::splitmix64` (`crates/ui/src/audio.rs`;
      once 2114 is done it is a re-export of `shuyi_core::rng::mix64`,
      the same function).
  - The same asks at the same times give the same offsets, in a replay,
    a test or `frame-png`.
- **`shuyi_ui::frame::flash`** (`crates/ui/src/frame/flash.rs`,
  `no_std`, integers only): `Flash { color: Rgb, strength: u8, ms: u16 }`,
  with `Flash::new(color, ms)` at strength 255 and
  `with_strength(strength)`. `Flash::alpha(self, elapsed_ms: u64) -> u8`
  is `strength × (ms − elapsed) / ms`, and 0 from `ms` on. A flash of
  0 ms shows nothing.
- **The frame:** `GlyphBuffer::set_shake(dx, dy)` and
  `shake() -> (i32, i32)`, in whole console pixels. Filling the whole
  buffer with a solid cell sets it back to `(0, 0)`, as it removes the
  backdrop and 2339's veil. A snapshot line `--- shake: dx,dy ---`
  (`crates/ui/src/frame/snapshot.rs`), after 2339's veil and transition
  lines, only when the shake isn't `(0, 0)`; the module doc lists it.
- **Every painter draws the frame moved by the shake:**
  - `shuyi-app` (`Renderer::draw`, `crates/app/src/render.rs:117`): the
    console's clear rectangle stays where it is; the layers and the
    backdrop's window move by the offset times the scale. What moves
    past the console's place is drawn over the black around it;
  - `frame-png` (`Painter::render`, `crates/cli/src/frame_png.rs:437`):
    the picture keeps its size. The layers and the backdrop's window
    move, what moves past an edge is cut off, and the edge left bare
    stays the clear colour;
  - the one-bit renderer (`onebit.rs` and `onebit/reference.rs`, kept
    equal): drawn at a `Place` moved from `Place::ORIGIN` by whole
    pixels, the edge left bare in the white ground. `render_changed`
    redraws the whole screen when the shake changed;
  - **Either order with 2332:** if `shuyi_ui::frame::raster` exists, the
    move goes there too; if not, 2332 moves it with the rest of
    `frame_png.rs`.
- **Flash, on 2339's veil:** while a flash shows, `Game` sets the veil
  to its colour at `Flash::alpha`, in place of any veil 2339's `compose`
  set. A newer flash replaces the one showing.
  - The one-bit renderer, and its reference, draw a veil whose colour is
    `Ink::Invert`'s code (`Ink::code`) by flipping the pixels it covers
    where `shows(alpha, x, y)`. So a flash in that colour shows whatever
    is under it, as an `Invert` rectangle does today.
- **Photosensitivity, whatever the game asks** (in `Game`):
  - at most three flashes start in any one second; a flash asked for
    sooner is dropped (WCAG 2.3.1);
  - with the player's `flashing` off, a flash's strength is at most
    `GENTLE_FLASH` (64, *tunable*: a quarter).
- **Asking:** `Ctx::shake(&mut self, Shake)` and `Ctx::flash(&mut self,
  Flash)`, beside 2339's `Ctx::transition_look`. `Game` takes them each
  frame and plays them on `ctx.clock_s` in whole ms. After the stack is
  drawn (and 2339's `compose`), it sets the buffer's shake and, while a
  flash shows, its veil.
- **The rules in `Game`:**
  - idle motion off (every Harness): shakes and flashes asked for are
    dropped, as 2339 drops looks;
  - `Motion::Reduced`: no shake. 2339's `Wipe` and `Dissolve` play as a
    `Fade` of the same length, through their `through` colour or else
    the colour the frame is cleared to (`GameData::blank_cell`'s
    background). `Fade` and `Cut` play as asked;
  - `shake_strength` scales every shake's trauma
    (`trauma × shake_strength / 100`).
- **Either order with 2359 (parked):** if done, move the places it
  changed (the toy field's `Field::update` and the doc of
  `MapScene::clock_ms`) from `ctx.idle_motion` onto
  `Ctx::shows_idle_motion()`; if not, 2359 uses it.
- **Either order with 2110:** if done, each saved form here that has a
  `VERSION` (the engine's `()` settings, the grid template's) raises it,
  keeps its old fixtures, adds `v<N+1>_*.ron` written by this build,
  and reads the old version through `Migrate::upgrade` with the new
  fields at their defaults, as its guard asks. If not, 2110's version 1
  holds these fields.
- **Either order with 2404 (parked):** both add settings to `Settings`
  and the saved forms. Whichever lands second follows the first's shape.
- **Either order with 2366 (parked):** it adds `Settings::sound_captions`
  and saves it in the toy's and hello-grid's forms. Whichever lands
  second follows the first's shape.
- **Either order with 2408 (parked):** it adds `Settings::vibration`
  and its row, and saves it in the same forms. Whichever lands second
  follows the first's shape.
- **Public types (ADR-0038):** the new public types follow it (`Shake`
  and `Flash` as config with `new` and `with_` methods, `Motion` as a
  setting's choices).
- **What changes for the game:** additive. `Settings` gains three
  public fields; no game can build it as a struct literal (`layout` is
  `pub(crate)`). A game that offers the rows saves the fields in its own
  form, in its PR that moves the pin; one that doesn't keeps the
  defaults. Say so in the PR.

**Out (do not do):**
- a world-layer camera's shake (1120, 2362), and shaking the world but
  not the UI;
- a sprite's own hit flash or shake (`Paint::Solid` today, 2337's
  `Paint::Tinted`, 2104's frame events);
- GPU screen passes (2326, parked). When it is unparked it reads this
  ticket's `Settings::flashing` and adds only its own `screen_effects`;
- reading the system's reduced-motion preference at first start
  (`prefers-reduced-motion` on the web, the desktop's own setting): a
  follow-up through 3053's platform facts;
- rumble (2408);
- captions (2366);
- checking a game's own painted frames for flashes (an analysis like
  the Harding test): research, if a game asks;
- moving the game's own shake, flashes and idle decoration onto these
  (the game's choice).

## Implementation steps

1. Read 2339's ADR and code, `settings.rs`, `options.rs`, `toy.rs`,
   `ctx.rs`, `game.rs`, `onebit.rs` and the two painters; 2110,
   2314, 2332, 2351, 2359 and 2511 if done.
2. Add `Motion`, `shake_strength` and `flashing` to `Settings`, the
   saved forms, the three rows, their text, the toy's rows and
   `Ctx::shows_idle_motion`, with tests.
3. Write `shake.rs` and `flash.rs`, with tests.
4. Add the frame's shake, its snapshot line and the moved frame in every
   painter.
5. Add `Ctx::shake` and `Ctx::flash`, and play them in `Game`: the
   veil, the invert ink on one-bit, the two photosensitivity rules, the
   idle-motion rule, `Motion::Reduced` and `shake_strength`.
6. Add the line to 2339's ADR: the shake is a whole-frame field like the
   veil, look only, drawn by every painter.
7. Add the picture snapshots, and run the gates.

This ticket is near the size limit. If the diff passes about 600
changed lines (snapshots aside) before step 5, stop after step 4, write
the rest as a new After MVP ticket blocked by this one (the
`write-ticket` skill) with the acceptance criteria it takes, and say so
in the Completion notes.

## Acceptance criteria

- [ ] With idle motion off, every existing test passes but the toy's
      list of option rows (`ROWS`), which gains the three rows on
      purpose. No existing snapshot changes. A shake or flash asked for
      leaves no offset and no veil.
- [ ] `FrameShake::offset` gives the same offsets for the same asks at
      the same times, never more than `max_px` on an axis, and `(0, 0)`
      once the trauma is 0 (property test).
- [ ] One full shake's trauma reaches 0 after 1000 ms.
- [ ] A fourth flash within one second is dropped, and with `flashing`
      off no veil alpha passes 64 (property test over random asks and
      times).
- [ ] Under `Motion::Reduced` nothing shakes, and a `Wipe` or `Dissolve`
      draws the same frames as a `Fade` of the same length.
- [ ] `shake_strength` at 0 gives no offset; at 50 it halves the trauma.
- [ ] The toy, the grid template, the engine's `()` settings and
      hello-grid save the three fields and read them back; a saved form
      without them reads the defaults.
- [ ] The one-bit renderer and its reference give equal frames with a
      shake and with an invert veil.
- [ ] Picture snapshots (`insta::assert_binary_snapshot!`, as
      `examples/hello-grid/tests/it/snapshot.rs` does) of the toy's
      frame shaken and flashing, and of a one-bit invert flash, are
      reviewed and committed.
- [ ] 2339's ADR says the shake is a whole-frame field like the veil.
- [ ] `shuyi-ui` still builds for `thumbv7em-none-eabihf`.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `add`, `advance` and `offset` at chosen times; `Flash::alpha`
  over its time; the three-per-second rule at its edges; the gentle cap;
  the Reduced mapping of each look; `Ctx::shows_idle_motion` for the
  four cases; each row stepping and toggling; each saved form, and an
  old one without the fields; `Settings::restored` putting the three
  back.
- Property: the shake and flash properties above.
- Snapshot / integration: a Harness test with idle motion on that asks
  for a shake and a flash and reads the `shake` and `veil` lines; one
  look test that reads pixels of the moved frame (`AGENTS.md` rule 9);
  the picture snapshots.

## Completion notes

*(Filled in by the session that completes the ticket.)*
