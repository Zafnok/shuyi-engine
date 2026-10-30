---
id: "2408"
title: "Parked: Rumble as output data: cues in a rumble file, the player's vibration setting, and backends for pads on desktop and in the browser"
type: feature
milestone: "Engine: input"
model: opus-5.5
effort: high
status: blocked
blocked_by: ["2511"]
nick_input: none
completed:
---

# 2408 — Rumble as output data

> **Parked** (Nick, 2026-10-14: "document the parked (i.e. after MVP/parity) tickets for networking, styles for later"; on 2026-10-20 he kept parked the gaps the game doesn't need): the game doesn't ask for it, so this waits until after the MVP (`docs/ROADMAP.md`). Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

Found by the 2026-10-20 comparison with Bevy 0.20, Godot 4.7, Unity 6
and Unreal 5.8. All four let a game rumble a pad, with a strong motor
and a weak one:

- Godot: `Input.start_joy_vibration(device, weak, strong, duration)`
  (<https://docs.godotengine.org/en/stable/classes/class_input.html#class-input-method-start-joy-vibration>);
- Unreal: Force Feedback Effect assets, played with an intensity;
- Bevy: `GamepadRumbleRequest` events, played by `bevy_gilrs`;
- Unity: `Gamepad.SetMotorSpeeds(low, high)` in its Input System, and
  console haptics.

In browsers, `GamepadHapticActuator.playEffect("dual-rumble", …)` does
the same where the browser has it
(<https://developer.mozilla.org/en-US/docs/Web/API/GamepadHapticActuator/playEffect>).

What Shuyi has today (checked 2026-10-20):

- `FrameOutput` (`crates/ui/src/game.rs`) carries the frame, the audio
  requests, the music commands, the volumes and fullscreen. Nothing for
  rumble.
- `Platform` (`crates/platform/src/platform.rs`) has seven methods, all
  required: `display`, `present`, `poll_input`, `system_events`,
  `storage`, `audio` and `text_input` (ADR-0024 §2). No slot for a
  pad's motors.
- `shuyi-app` reads pads with gilrs 0.11.2 (`PadInput` in
  `crates/app/src/pads.rs`, over `Source` in
  `crates/app/src/pads/native.rs`) and never calls its force feedback
  module, `gilrs::ff`. gilrs's README lists force feedback on Windows
  and Linux (evdev), not on macOS. On the web,
  `crates/cli/web/gamepad.js` only reads pads.
- The Harness logs each frame's audio (`Harness::audio_log`,
  `crates/platform/src/harness.rs`), and the contract's `run` answer
  lists sound cues (`Sound { frame, cue }`,
  `crates/cli/src/contract/session.rs`). Nothing for rumble.
- `Settings` (`crates/ui/src/settings.rs`) keeps the volumes in percent,
  and the options screen has a slider row for them (`RowValue::Volume`,
  `crates/ui/src/screens/options.rs`).
- `App` (`crates/platform/src/app.rs`) hears `Suspend`, `FocusLost` and
  `QuitRequested` (`on_system_event`). `shuyi-app` sends only
  `ToggleFullscreen` and `QuitRequested` today (`crates/app/src/backend.rs`).
- Planned: 2402 and 2403 (parked) give their pads `Motors` if this is
  done (their either-order clauses); 2903 (parked) lists it among what
  later work may want;
  2340, 2406 (seats) and 2407 (phones' browsers) point their rumble
  here.

Why it matters:

- **The game first.** Visions of Shuyi doesn't want rumble now. Its
  controls design (the game repository's
  [`docs/design/controls.md`](https://github.com/Zafnok/visions-of-shuyi/blob/main/docs/design/controls.md),
  Q7, settled in game#0032) says: "for now, no rumble, but we might add
  it later". That is why this is parked. If the game changes its answer,
  it asks through its `engine-change` skill, and this is unparked early.
- **Other 2D games.** An action game, a platformer or a shooter played
  on a pad or the Steam Deck rumbles on a hit, and its players expect to
  turn it down or off. The real-time work those games need (1119, 1127)
  is parked until the game launches, so this can wait with it.
- **The end goal** (2026-10-19, `docs/decisions.md`). Cues are data and
  output only, so an agent checks "a hit rumbles" headless, through the
  Harness and `shuyi run --json`, with no pad.

Rumble is output, like audio. A screen asks for a named cue. The frame
carries the request out. `shuyi-platform` turns it into motor levels on
the backend's clock, and the backend sets the motors. Nothing in a
simulation reads it, so state hashes and replays don't change
(`AGENTS.md` rule 5). It names no key (rule 8). The cue file is a
standard format (rule 10, ADR-0007, ADR-0019).

**Rumble never carries information alone.** Whatever a cue signals is
also seen or heard. The player can turn it off. A target with no motors
(the Playdate, a keyboard) loses nothing. The module docs and the ADR
say so, as the common game accessibility guidelines ask.

## Nick input

None. The game's own answer (above) is why it is parked. Nick unparks it
if he wants rumble in the game.

## Scope

**In:**

- **The kind and its format** (`crates/assets/src/rumble.rs`, `no_std`):
  - `RumbleCues { cues: BTreeMap<String, RumbleCue> }`, with `get(id)`;
  - `RumbleCue { steps: Vec<RumbleStep> }`;
  - `RumbleStep { ms: u16, strong: u8, weak: u8 }`: how long the step
    lasts, and each motor's strength in percent (0 to 100), like
    `SoundCue::volume`. The strong motor is the heavy, low one; the weak
    motor the light, high one;
  - kind name `"rumble cues"`. Each type derives `Debug`, `Clone`,
    `PartialEq`, `Serialize` and `Deserialize`, so the pack bakes it
    (the `Bakeable` blanket impl in `crates/assets/src/pack/bake.rs`);
  - the file lists the cues, each with its id. It is read into a
    private raw form (a `Vec` of cues), so a repeated id is an error,
    then into the map:

    ```ron
    // The game's rumble cues: each a few steps, each motor in percent.
    (
        cues: [
            (id: "bump", steps: [(ms: 60, strong: 0, weak: 50)]),
            (id: "hit", steps: [(ms: 80, strong: 80, weak: 40), (ms: 120, strong: 0, weak: 30)]),
        ],
    )
    ```

  - every problem is an error with its file and line (`ContentError`, as
    `names::from_source` gives them): an empty or repeated id; no steps,
    or more than `MAX_STEPS` (16, *tunable*); a step of 0 ms or more than
    `MAX_STEP_MS` (2000); a strength over 100; steps that add up to more
    than `MAX_CUE_MS` (5000, *tunable*: a longer rumble is asked for
    again); a cue whose steps are 0 on both motors;
  - `rumble::from_source(shown, text)` and the format `RumbleRon` (id
    `rumble`, extension `ron`, `TEXT_MAX_BYTES`), built like `NamesRon`
    (`crates/assets/src/names.rs`) and registered in
    `Formats::register_standard` (`crates/assets/src/format.rs`). It
    gets a row in `docs/formats.md`'s table of standard formats, and
    `rumble::RumbleCues` joins that page's list of the engine's other
    kinds. A game loads it by id:
    `formats.load::<RumbleCues>(&bundle, "rumble")`;
  - `Importers::standard()` (`crates/cli/src/contract/import.rs`)
    gets the kind, summed up as `count(r.cues.len(), "rumble cue",
    "rumble cues")`, as it sums up `Names`;
  - **from 2116 (done):** the format gets a row in
    `crates/assets/tests/golden/formats.ron` (`rumble: <format id>`):
    a `rows.format(…)` line in
    `crates/assets/src/format/golden_tests.rs`, and `RumbleCues` joins
    the covered kinds of its guard `missing_rows` (if 2515 hasn't moved
    the guard onto `Formats::kinds()` yet). `cargo xtask golden
    --bless` writes the row;
  - **either order with 2515:** if it is done, `RumbleRon` implements
    `TextSchema` (the raw form derives `JsonSchema` under the `schema`
    feature), has an example in `crates/assets/kinds/rumble.ron`, and
    `Importers::standard()` describes it. If not, 2515 covers it with
    the other standard RON formats.
- **Asking** (`crates/ui/src/rumble.rs`, `no_std`):
  - `RumbleRequest::Play { cue: String }` and `RumbleRequest::Stop`;
    `RumbleQueue` with `play(cue)`, `stop()`, `pending()` and `take()`;
  - `Ctx::rumble: RumbleQueue`, a public field beside `audio`:
    `ctx.rumble.play("hit")`;
  - `GameData::rumble_cues(&self) -> &RumbleCues`, defaulted to a
    `static` empty set, so no game has to change;
  - `Game::frame` moves the requests into a new
    `FrameOutput::rumble: &[RumbleRequest]`, and adds
    `FrameOutput::vibration: u8` (the setting). In debug builds a cue the
    game's cues lack panics, as an unknown sound cue does in
    `Game::collect_audio`; in release builds it is dropped.
- **The player's setting:**
  - `Settings::vibration: u8`, 0 (off) to `MAX_VOLUME` (100), default
    `DEFAULT_VIBRATION` (100, *tunable*). `Settings::restored` resets it,
    as it resets the volumes;
  - `OptionRow::vibration()`, the slider row (`RowValue::Volume`, whose
    help line already reads for any number), text key
    `options.row.vibration`. The toy's options screen shows it, with its
    English in `crates/ui/src/toy/en.ron`;
  - saved forms: the toy's (`crates/ui/src/toy.rs`), `templates/grid`
    (`crates/game/src/settings.rs`), the engine's
    `impl GameSettings for ()` (`crates/ui/src/settings.rs`, 2511, which
    the blank template uses) and `examples/hello-grid/src/settings.rs`
    save `vibration`, and a saved form without it reads the default
    (`#[serde(default)]`, as today). The templates aren't formatted by
    `cargo fmt --all`; CI's template job checks them;
  - **either order with 2110:** if it is done, each of these saved
    forms that has a `VERSION` (the engine's `()` settings, the grid
    template's) raises it, keeps its old fixtures, adds the new
    version's, and reads the old one through `Migrate::upgrade` with
    `vibration` at its default, as 2110's check asks. If not, 2110's
    version 1 holds the field.
- **The rules** (`crates/platform/src/rumble.rs`, `no_std`, integers, on
  the `Millis` that `App` gets from the backend; no clock is read here):
  - `MotorLevel { strong: u8, weak: u8 }`, in percent;
  - `RumblePlayer`, one track per motor:
    - `play(cue, now)`: the cue takes each motor it moves (any step
      above 0 on it) from whatever played there. **Newest wins per
      motor:** a motor the cue never moves keeps its track;
    - `stop()` ends both tracks;
    - `at(now) -> Option<(MotorLevel, u32)>`: the levels now and the
      milliseconds until the next change; `None` once both tracks have
      ended;
  - scaled by the setting: each strength × `vibration` / 100, rounded
    down. At 0 the motors are never called;
  - **`App`**, in each running frame after `audio().play`: a `Stop`
    request stops the player; each `Play` finds its cue in
    `ctx.game.rumble_cues()` and plays it. `App` keeps the levels it
    last sent and when their `for_ms` runs out, and sends the scaled
    levels:
    - `motors.set(level, for_ms)` when they are above 0 and differ from
      the last sent, or when the last `for_ms` has run out while a
      track still plays (two steps in a row at the same level);
    - `motors.stop()` when the motors may be moving (a `set` since the
      last `stop`) and the levels fall to 0 or `at` gives `None`. The
      stops below call it on the same condition, so at `vibration` 0
      the motors are never called;
  - **stops:** `Suspend`, `FocusLost`, `QuitRequested` and the game
    quitting stop the motors and the player in that frame. While
    suspended, or until `FocusGained`, cues asked for are dropped. A
    resume restarts nothing;
  - **pauses (from 2394, done):** a frame whose audio holds the
    `AudioRequest::PauseSounds` request stops the motors. That covers
    a pause menu, and 2316's debug pause, which pauses through the
    queue's `OUTSIDE_THE_STACK` owner. Cues asked for after it play, as
    a menu's own sounds do. 2394 left this wiring to this ticket;
  - **either order with 3042 and 3043:** they make `shuyi-app` send
    `FocusLost` from the window and `Suspend` from a hidden tab. The
    rule above needs nothing more from them, and it holds whatever
    background policy (3042) the game picks.
- **The platform slot** (`crates/platform/src/rumble.rs`,
  `crates/platform/src/platform.rs`):
  - `trait Motors { fn set(&mut self, level: MotorLevel, for_ms: u32);
    fn stop(&mut self); }`: hold `level` for `for_ms`, then let go
    unless set again;
  - `Platform::motors(&mut self) -> Option<&mut dyn Motors>`, a provided
    method that returns `None`. A backend with no motors (the Playdate's,
    3010) writes nothing, and no backend breaks;
  - `App::rumble()`: the last frame's requests, as `App::audio()` gives
    its audio;
  - `HeadlessPlatform` has recording motors (each `set` with its level
    and `for_ms`, and each `stop`, in order), and `without_motors()` for
    a backend with none. The Harness gains
    `rumble_log() -> Vec<(u32, &RumbleRequest)>` (each request with its
    frame, like `audio_log`) and `motor_log()` (what the motors were
    told, each with its frame, as `music_log` gives the music);
  - `docs/platform-backends.md`: the new step in "What one frame does",
    a section for `motors`, and a line in the checklist.
- **The backends** (`shuyi-app`; step 8 may move them to a ticket of
  their own):
  - **Which pad.** With several pads as one player, the pad pressed
    last. `Pads::last_pressed() -> Option<PadId>`
    (`crates/ui/src/input/pad.rs`): the pad holding the newest press
    `Pads::update` reported, found as `Pads::kind_holding` finds a pad
    (or, if 2406 removed `kind_holding`, from the newest `PadDown`'s
    pad id).
    `Pads` keeps it in a new field that `update` sets. Before any
    press, every connected pad.
  - **`PadInput` implements `Motors`** (`crates/app/src/pads.rs`): it
    picks the pad, then hands the levels to `native::Source` on native
    and to `web.rs`'s two new imports on the web.
  - **Native** (`crates/app/src/pads/native.rs`): `Source::set_motors`
    and `Source::stop_motors` use `gilrs::ff`.
    - gilrs gives no way to make a `GamepadId`, so each pad picked is
      the one of `gilrs.gamepads()` whose `usize::from(id)` is its
      `PadId`. A pad gets rumble only if `Gamepad::is_ff_supported()`.
    - `set_motors` drops the effect playing and builds one for the
      pads picked (`add_gamepad` for each) with `ff::EffectBuilder`:
      a `BaseEffect` of
      `BaseEffectType::Strong { magnitude }` and one of `Weak`, with
      `magnitude = level × 65535 / 100` and
      `scheduling: Replay { play_for: Ticks::from_ms(for_ms), ..Default::default() }`.
      The builder repeats forever unless told, so it also takes
      `.repeat(Repeat::For(Ticks::from_ms(for_ms)))`. Then `finish`,
      and `play`. `Source` keeps the `Effect`: dropping the handle ends
      it.
    - `stop_motors` stops the effect and drops it.
    - gilrs counts force feedback in ticks of 50 ms
      (`gilrs::ff::Ticks`), so on native a step lasts at least that
      long; the docs say so.
    - A gilrs error is logged once and that pad gets no more rumble,
      never a panic.
    - No new dependency: gilrs (MIT OR Apache-2.0, allowed by
      `deny.toml`) is already in `crates/app/Cargo.toml`, and
      `gilrs::ff` needs no feature.
  - **Our own pads, either order with 2402 and 2403 (parked):** if
    2402 is done, the `in-house-pads` source also implements
    `set_motors` and `stop_motors`, through a new
    `shuyi_pads::Pads::set_motors(id, strong: u16, weak: u16)` (0 and
    0 to stop), written from the documents as 2402's rumble clause
    says (ADR-0021 §2): XInput's `XInputSetState` for Xbox-style pads
    on Windows, let go by the source when `for_ms` runs out. The gilrs
    part above stays while 2403 isn't done. If 2403 is done too, gilrs
    is gone, so skip the gilrs part above: `Source::set_motors` and
    `stop_motors` go through `shuyi-pads`, the only native source, and
    Linux rumbles as 2403's rumble clause says, in place of
    `gilrs::ff`: an `FF_RUMBLE` effect uploaded with `EVIOCSFF`, played
    with an `EV_FF` event and removed with `EVIOCRMFF`, on a device
    opened read-write; none on macOS. If neither is done, nothing
    more: each adds its part when it lands, by its own clause.
  - **The web** (`crates/cli/web/gamepad.js`, `crates/app/src/pads/web.rs`):
    - `shuyi_pad_rumble(index, strong, weak, ms)` calls
      `vibrationActuator.playEffect("dual-rumble", { duration: ms,
      strongMagnitude: strong / 100, weakMagnitude: weak / 100 })` on
      the pad with the browser's `index` (its `PadId` on the web);
    - `shuyi_pad_rumble_stop(index)` calls `vibrationActuator.reset()`;
    - each checks that the actuator and its method exist and catches
      what they throw, so a browser without them does nothing;
    - `PLUGIN_VERSION` and the JS `VERSION` go to 4. The test
      `gamepad_js_provides_exactly_the_functions_the_app_imports`
      (`crates/cli/src/web.rs`) keeps the two lists of functions equal.
  - `Macroquad`'s `Platform::motors` (`crates/app/src/backend.rs`)
    returns its `PadInput` (the `pads` field) once the pads have
    started, and `None` before.
  - **Rumble per seat, either order with 2406 (parked, local
    multiplayer):** if 2406 is done, `RumbleRequest::Play` names a
    `Seat` (`Seat::FIRST` unless the screen says), each seat has its own
    `RumblePlayer`, `Motors::set` and `stop` take the seat, and a seat's
    cue goes to the last pad pressed in that seat. If 2406 isn't done,
    it does this when it lands.
- **The tool contract:**
  - `run`'s answer gets `rumble: Vec<Rumble>` (`Rumble { frame, cue }`,
    the `Play` requests) beside `sounds`
    (`crates/cli/src/contract/run.rs`), from `Session::rumble`, a
    provided method that returns none, so no other `Session` breaks.
    `Headless` (`crates/cli/src/contract/session.rs`) reads it from the
    Harness's `rumble_log`, and `FakeSession` (`fake.rs`) gives one. The
    text answer gets a `rumble:` line like `sounds:`;
  - raise `contract::VERSION`, regenerate `crates/cli/schemas/` and add
    the version's line to `versions.json` (ADR-0022 §5); mention rumble
    in `docs/contract.md`'s `run` row.
- **A short ADR** (the `write-adr` skill), extending ADR-0024 §2 and
  ADR-0007: rumble is output data like audio; the cue file is a standard
  kind; `Platform::motors` is a provided method; the rules live in
  `shuyi-platform`; rumble never carries information alone.
- **Either order with 2340 and 2342:** each adds settings and rows to the
  options screen. Whichever lands second follows the first's shape.
  Vibration is never one of 2342's sound groups.
- **Either order with 2351:** if the engine has built-in English for its
  own text keys, `options.row.vibration` goes there too.
- **What changes for the game:** additive. `Settings` and `Ctx` gain
  public fields, and neither can be built as a struct literal outside
  `shuyi-ui` (each has a private field). `GameData::rumble_cues`,
  `Platform::motors` and `Session::rumble` have defaults. `FrameOutput`
  gains two fields, and only `Game::frame` builds it. The game's next
  pin bump needs no code change. If the public API check (2010) calls
  any of this breaking, the PR's `Breaking:` line says so.

**Out (do not do):**

- adaptive triggers, HD haptics (DualSense; the Switch's, with 3014),
  motion sensors, lights and batteries;
- phone vibration: native phones are 3015's (parked). A phone's
  browser (`navigator.vibrate`, one motor) goes in the tickets 2407
  (parked) writes, which can play these cues through `Motors`;
- the DS Rumble Pak: 3024 (parked) leaves it out too, and a ticket
  when a DS game asks implements `Platform::motors` in `shuyi-nds`;
- Steam Input's own haptics (Steam only; game#0903 weighs Steam Input);
- rumble that fades with distance from a listener: 2363's positional
  sound (parked) has the listener; a follow-up if a game asks;
- saying per target or at run time whether a pad can rumble (a `rumble`
  field in `targets/*.ron`, a fact in 3053): nothing reads it yet, and
  the game picks which option rows it shows;
- a test pulse when the vibration row changes, and rumble from the
  engine's own menus and widgets;
- checking with `shuyi check` the cue names screens use in their code
  (the debug-build panic and the Harness tests catch them);
- rumble on our own pad code when this ticket lands first (2402, 2403,
  parked): each adds it to its source when it lands, by its rumble
  clause. If they are done first, the clause under the backends does it
  here.

## Implementation steps

1. Read ADR-0007, ADR-0019, ADR-0024, `docs/platform-backends.md`, the
   audio path (`crates/ui/src/audio.rs`; `Game::frame` and
   `collect_audio` in `crates/ui/src/game.rs`; `step` in
   `crates/platform/src/app.rs`; `crates/platform/src/audio.rs`) and the
   Harness. Check which of 2110, 2340, 2342, 2344, 2351, 2406, 2515
   and 3042 are done.
2. Write `shuyi_assets::rumble`, its format and its tests; register it;
   add the rows to `docs/formats.md`, the kind to
   `Importers::standard()` and the golden row (2116); add 2515's part
   if it is done.
3. Write `shuyi_ui::rumble`, the `Ctx` field, `GameData::rumble_cues`,
   the `FrameOutput` fields and the debug-build check.
4. Add the setting, the row, its text and the saved forms (with 2110's
   versions if it is done).
5. Write `shuyi_platform::rumble` (`MotorLevel`, `Motors`,
   `RumblePlayer`), `Platform::motors`, `App`'s part and the stops; then
   the headless motors and the Harness logs.
6. Add the contract's `rumble` entries and the schema version.
7. Write the ADR, and update `docs/platform-backends.md` and
   `docs/contract.md`.
8. Measure the diff. If it is past about 600 changed lines (not
   counting schemas, fixtures and snapshots), write the backends (the
   "The backends" part of the scope, with 2406's pad per seat) as one
   ticket of its own with the `write-ticket` skill, blocked by this one,
   and leave them out. Then go to step 11.
9. Add `Pads::last_pressed`, `PadInput`'s `Motors` and the gilrs
   backend. Try it with a pad on Windows or Linux.
10. Measure again. If it is now past about 600 changed lines, write the
    web half as a ticket of its own (the `write-ticket` skill) and leave
    it out. Otherwise add it, and try it in Chrome with a pad.
11. Run the gates.
12. In the completion notes: the pads and browsers tried, and, if a
    Steam Deck was at hand, whether its built-in controls rumbled through
    Steam's virtual pad; or the ticket the backends moved to.

## Acceptance criteria

- [ ] The fixture rumble file reads; a file breaking each rule gives
      that error with its line; random bytes and changed fixtures never
      panic; the fixture baked into a pack reads back equal.
- [ ] `Importers::standard()` sums up the fixture rumble file as its
      count of rumble cues (a unit test).
- [ ] Harness: a test screen asks for `hit`; `rumble_log` has it at its
      frame, and `motor_log` has each step's scaled levels at the first
      frame at or after the step starts, then a stop.
- [ ] Two steps in a row at the same level keep the motors on for both
      (a `set` is sent again when the first one's `for_ms` runs out).
- [ ] A cue that moves only the weak motor, asked for while one plays on
      the strong motor, leaves the strong motor's track playing (a unit
      test at chosen times).
- [ ] With `vibration` at 0 the motors are never called; at 50 every
      level is halved, rounded down.
- [ ] `Suspend`, `FocusLost`, `QuitRequested` and a quit each stop the
      motors in that frame.
- [ ] The same script played with `vibration` at 0 and at 100 gives the
      same snapshots and the same events.
- [ ] A backend whose `Platform::motors` is `None` runs a game that asks
      for rumble with no error.
- [ ] `shuyi run --json` lists `rumble` entries, and the schema test
      passes with the new version's line.
- [ ] The toy's options screen snapshot changes only by the new row,
      reviewed and committed.
- [ ] The toy, the grid template, the engine's `()` settings and
      hello-grid save `vibration` and read it back; a saved form without
      it reads the default.
- [ ] `shuyi-assets`, `shuyi-ui` and `shuyi-platform` still build for
      `thumbv7em-none-eabihf`.
- [ ] A pad on Windows or Linux rumbles for a cue (by hand, noted in the
      completion notes), or step 8 moved the backends to a ticket of
      their own, named in the completion notes.
- [ ] If 2402 is done: an Xbox pad rumbles for a cue with
      `in-house-pads` on (by hand, noted in the completion notes).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: one test per rule of the format, with its line; `RumbleQueue`
  (play, stop, `take` empties it); `RumblePlayer` at chosen times (one
  cue's steps in order, newest wins per motor, a cue on one motor,
  `stop`, `at` giving `None` once both tracks end); scaling at 0, 50 and
  100; `Pads::last_pressed` with two pads (if the backends stay in);
  each saved form read back; the `Importers::standard()` summary.
- Property: the format never panics on random bytes or on its fixtures
  with a few characters changed (the pattern of
  `crates/assets/src/format/builtin_tests.rs`); for random cues, settings
  and times, no level passes the cue's strength × `vibration` / 100, and
  every track has ended `MAX_CUE_MS` after it started.
- Snapshot / integration: Harness tests for `rumble_log`, `motor_log`,
  each stop, and `vibration` at 0 against 100; the `run` answer with
  `rumble` entries and the schema test; the pack round trip; the toy's
  options screen.
- Manual: a pad on Windows or Linux (if the backends stay in); Chrome
  with a pad (if the web half stays in); the Steam Deck if one is at
  hand.

## Completion notes

*(Filled in by the session that completes the ticket.)*
